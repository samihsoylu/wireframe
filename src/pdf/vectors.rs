// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Convert the product's small, internally generated vector fragments. Text is
//! handled separately to retain glyph positions and source mappings.
use crate::{model::Bounds, scene::Primitive};
use krilla::{
    color::rgb,
    geom::{Path, PathBuilder, Size, Transform},
    image::Image,
    num::NormalizedF32,
    paint::{Fill, FillRule, LineCap, LineJoin, Paint, Stroke},
    surface::Surface,
};
use resvg::{tiny_skia::PathSegment, usvg};
use std::sync::Arc;

pub(super) fn primitive(surface: &mut Surface<'_>, p: &Primitive) -> Result<(), String> {
    let valid = match p {
        Primitive::Mesh {
            points, indices, ..
        } => {
            indices.len() % 3 == 0
                && indices.iter().all(|i| (*i as usize) < points.len())
                && points.iter().flatten().all(|v| v.is_finite())
        }
        Primitive::Polygon { points, .. } => points.iter().flatten().all(|v| v.is_finite()),
        Primitive::Line { points, width, .. } => {
            width.is_finite() && *width >= 0. && points.iter().flatten().all(|v| v.is_finite())
        }
        Primitive::Rect { b, radius, .. } => {
            super::valid_bounds(*b) && radius.is_finite() && *radius >= 0.
        }
        Primitive::Ellipse { b, .. } => super::valid_bounds(*b),
        Primitive::StyledEllipse { b, width, .. } => {
            super::valid_bounds(*b) && width.is_finite() && *width >= 0.
        }
        Primitive::Text { .. } => false,
    };
    if !valid {
        return Err("Invalid PDF vector geometry".into());
    }
    markup(surface, &crate::scene::primitive_svg(p.clone()))
}
pub(super) fn markup(surface: &mut Surface<'_>, fragment: &str) -> Result<(), String> {
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\">{fragment}</svg>"
    );
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default())
        .map_err(|e| format!("Invalid vector: {e}"))?;
    group(surface, tree.root())
}
fn group(surface: &mut Surface<'_>, g: &usvg::Group) -> Result<(), String> {
    if g.clip_path().is_some() || g.mask().is_some() || !g.filters().is_empty() {
        return Err("Unsupported vector fragment effect".into());
    }
    let t = g.transform();
    surface.push_transform(&Transform::from_row(t.sx, t.ky, t.kx, t.sy, t.tx, t.ty));
    surface.push_opacity(NormalizedF32::new(g.opacity().get()).ok_or("Invalid vector opacity")?);
    let result = (|| {
        for node in g.children() {
            match node {
                usvg::Node::Group(g) => group(surface, g)?,
                usvg::Node::Path(p) if p.is_visible() => {
                    surface.set_fill(
                        p.fill()
                            .map(|f| {
                                Ok::<_, String>(Fill {
                                    paint: paint(f.paint())?,
                                    opacity: NormalizedF32::new(f.opacity().get())
                                        .ok_or("Invalid fill opacity")?,
                                    rule: match f.rule() {
                                        usvg::FillRule::NonZero => FillRule::NonZero,
                                        usvg::FillRule::EvenOdd => FillRule::EvenOdd,
                                    },
                                })
                            })
                            .transpose()?,
                    );
                    surface.set_stroke(
                        p.stroke()
                            .map(|s| {
                                Ok::<_, String>(Stroke {
                                    paint: paint(s.paint())?,
                                    width: s.width().get(),
                                    miter_limit: s.miterlimit().get(),
                                    line_cap: match s.linecap() {
                                        usvg::LineCap::Butt => LineCap::Butt,
                                        usvg::LineCap::Round => LineCap::Round,
                                        usvg::LineCap::Square => LineCap::Square,
                                    },
                                    line_join: match s.linejoin() {
                                        usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => {
                                            LineJoin::Miter
                                        }
                                        usvg::LineJoin::Round => LineJoin::Round,
                                        usvg::LineJoin::Bevel => LineJoin::Bevel,
                                    },
                                    opacity: NormalizedF32::new(s.opacity().get())
                                        .ok_or("Invalid stroke opacity")?,
                                    ..Default::default()
                                })
                            })
                            .transpose()?,
                    );
                    surface.draw_path(&path(p.data())?);
                }
                usvg::Node::Path(_) => {}
                _ => return Err("Non-vector content in PDF vector fragment".into()),
            }
        }
        Ok(())
    })();
    surface.pop();
    surface.pop();
    result
}
fn paint(p: &usvg::Paint) -> Result<Paint, String> {
    match p {
        usvg::Paint::Color(c) => Ok(rgb::Color::new(c.red, c.green, c.blue).into()),
        _ => Err("Unsupported vector paint".into()),
    }
}
fn path(p: &resvg::tiny_skia::Path) -> Result<Path, String> {
    let mut out = PathBuilder::new();
    for s in p.segments() {
        match s {
            PathSegment::MoveTo(p) => out.move_to(p.x, p.y),
            PathSegment::LineTo(p) => out.line_to(p.x, p.y),
            PathSegment::QuadTo(a, b) => out.quad_to(a.x, a.y, b.x, b.y),
            PathSegment::CubicTo(a, b, c) => out.cubic_to(a.x, a.y, b.x, b.y, c.x, c.y),
            PathSegment::Close => out.close(),
        }
    }
    out.finish().ok_or_else(|| "Invalid vector path".into())
}
pub(super) fn image(surface: &mut Surface<'_>, b: Bounds, png: &[u8]) -> Result<(), String> {
    if !super::valid_bounds(b) {
        return Err("Invalid PDF image bounds".into());
    }
    if b.w == 0. || b.h == 0. {
        return Ok(());
    }
    let image = Image::from_png(Arc::new(png.to_vec()).into(), true)?;
    let size = Size::from_wh(b.w, b.h).ok_or("Invalid image size")?;
    surface.push_transform(&Transform::from_translate(b.x, b.y));
    surface.draw_image(image, size);
    surface.pop();
    Ok(())
}
