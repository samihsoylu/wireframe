// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! PDF encoding of an already positioned document snapshot. No UI or file writes.
mod export;
mod plan;
mod semantic;
mod text;
pub use export::ExportJob;
pub use plan::{Entry, Options, Orientation, Paper, Selection, entries, plan};
mod vectors;
use crate::{
    model::Bounds,
    render::{Drawing, Scene},
};
use krilla::{
    Document,
    action::{Action, LinkAction},
    annotation::{LinkAnnotation, Target},
    destination::XyzDestination,
    geom::{PathBuilder, Point, Rect, Transform},
    metadata::Metadata,
    num::NormalizedF32,
    page::PageSettings,
    paint::FillRule,
};

#[derive(Clone, Debug)]
pub enum Destination {
    Web(String),
    Page(usize),
}
#[derive(Clone, Debug)]
pub struct Link {
    pub bounds: Bounds,
    pub destination: Destination,
}
#[derive(Clone, Debug)]
pub struct Page {
    pub scene: Scene,
    /// Headers, notes and hints in page coordinates, independent of artwork scaling.
    pub decoration: Option<Scene>,
    pub size: [f32; 2],
    /// Position of the scene's top-left content bound on the PDF page.
    pub origin: [f32; 2],
    pub scale: f32,
    /// Resolved destinations; bounds remain in scene coordinates.
    pub links: Vec<Link>,
}

pub fn encode(title: &str, pages: &[Page]) -> Result<Vec<u8>, String> {
    if pages.is_empty() || pages.len() > 1000 {
        return Err("PDF requires 1 to 1000 pages".into());
    }
    let mut doc = Document::new();
    let mut notices = String::new();
    for p in pages {
        let notice = crate::fonts::notices(p.scene.font.as_deref());
        if !notices.contains(notice) {
            notices.push_str(notice);
            notices.push('\n');
        }
    }
    if let Some(attribution) = crate::icons::attribution(pages.iter().flat_map(|page| {
        page.scene.items.iter().filter_map(|item| {
            if let Drawing::Icon { id, .. } = &item.drawing {
                Some(id.as_str())
            } else {
                None
            }
        })
    })) {
        notices.push_str(&attribution);
    }
    doc.set_metadata(
        Metadata::new()
            .title(title.into())
            .creator("Wireframe".into())
            .description(notices),
    );
    let mut fonts = text::Fonts::default();
    for (page_index, input) in pages.iter().enumerate() {
        if input.size.iter().any(|v| !v.is_finite() || *v <= 0.)
            || input.origin.iter().any(|v| !v.is_finite())
            || !input.scale.is_finite()
            || input.scale <= 0.
            || !valid_bounds(input.scene.bounds)
        {
            return Err("Invalid PDF page geometry".into());
        }
        let mut page = doc.start_page_with(
            PageSettings::from_wh(input.size[0], input.size[1]).ok_or("Invalid PDF page size")?,
        );
        let mut surface = page.surface();
        let tx = input.origin[0] - input.scene.bounds.x * input.scale;
        let ty = input.origin[1] - input.scene.bounds.y * input.scale;
        if !tx.is_finite() || !ty.is_finite() {
            return Err("PDF transform exceeds limits".into());
        }
        surface.push_transform(&Transform::from_row(
            input.scale,
            0.,
            0.,
            input.scale,
            tx,
            ty,
        ));
        let result = draw_scene(&mut surface, &input.scene, &mut fonts)
            .map_err(|e| format!("Page {} ({}): {e}", page_index + 1, input.scene.title));
        surface.pop();
        result?;
        if let Some(decoration) = &input.decoration {
            draw_scene(&mut surface, decoration, &mut fonts)?;
        }
        surface.finish();
        for link in &input.links {
            let target = match &link.destination {
                Destination::Web(url)
                    if url.starts_with("https://") || url.starts_with("http://") =>
                {
                    Target::Action(Action::Link(LinkAction::new(url.clone())))
                }
                Destination::Page(index) if *index < pages.len() => {
                    Target::Destination(XyzDestination::new(*index, Point::from_xy(0., 0.)).into())
                }
                _ => return Err("Invalid PDF link destination".into()),
            };
            let b = link.bounds;
            let b = Bounds::new(
                tx + b.x * input.scale,
                ty + b.y * input.scale,
                b.w * input.scale,
                b.h * input.scale,
            );
            if !valid_bounds(b) {
                return Err("Invalid PDF link bounds".into());
            }
            if b.w > 0. && b.h > 0. {
                page.add_annotation(LinkAnnotation::new(rect(b)?, target).into());
            }
        }
        page.finish();
    }
    doc.finish()
        .map_err(|e| format!("Cannot encode PDF: {e:?}"))
}
fn valid_bounds(b: Bounds) -> bool {
    [b.x, b.y, b.w, b.h].iter().all(|v| v.is_finite()) && b.w >= 0. && b.h >= 0.
}
fn rect(b: Bounds) -> Result<Rect, String> {
    Rect::from_xywh(b.x, b.y, b.w, b.h).ok_or_else(|| "Invalid PDF rectangle".into())
}

fn draw_scene(
    surface: &mut krilla::surface::Surface<'_>,
    scene: &Scene,
    fonts: &mut text::Fonts,
) -> Result<(), String> {
    let mut scopes = Vec::new();
    let result = (|| {
        for item in &scene.items {
            match &item.drawing {
                Drawing::BeginOpacity(opacity) => {
                    surface.push_opacity(
                        NormalizedF32::new(*opacity).ok_or("Invalid control opacity")?,
                    );
                    scopes.push(false);
                    continue;
                }
                Drawing::EndOpacity => {
                    if scopes.last() != Some(&false) {
                        return Err("Unbalanced PDF opacity".into());
                    }
                    surface.pop();
                    scopes.pop();
                    continue;
                }
                Drawing::BeginMask(mask) => {
                    let points = mask.polygon();
                    if points.iter().flatten().any(|v| !v.is_finite()) {
                        return Err("Invalid PDF mask coordinates".into());
                    }
                    let mut path = PathBuilder::new();
                    if let Some(p) = points.first() {
                        path.move_to(p[0], p[1]);
                    }
                    for p in points.iter().skip(1) {
                        path.line_to(p[0], p[1]);
                    }
                    path.close();
                    surface.push_clip_path(
                        &path.finish().ok_or("Invalid PDF mask")?,
                        &FillRule::NonZero,
                    );
                    scopes.push(true);
                    continue;
                }
                Drawing::EndMask => {
                    if scopes.last() != Some(&true) {
                        return Err("Unbalanced PDF mask".into());
                    }
                    surface.pop();
                    scopes.pop();
                    continue;
                }
                _ => {}
            }
            let opacity = NormalizedF32::new(item.opacity).ok_or("Invalid drawing opacity")?;
            if item.opacity == 0. {
                continue;
            }
            if let Some(b) = item.clip {
                if !valid_bounds(b) {
                    return Err("Invalid PDF clipping rectangle".into());
                }
                if b.w == 0. || b.h == 0. {
                    continue;
                }
                let mut path = PathBuilder::new();
                path.push_rect(rect(b)?);
                surface.push_clip_path(
                    &path.finish().ok_or("Invalid PDF clip")?,
                    &FillRule::NonZero,
                );
            }
            surface.push_opacity(opacity);
            let result = draw_item(surface, &item.drawing, scene, fonts);
            surface.pop();
            if item.clip.is_some() {
                surface.pop();
            }
            result?;
        }
        if !scopes.is_empty() {
            Err("Unbalanced PDF mask".into())
        } else {
            Ok(())
        }
    })();
    while scopes.pop().is_some() {
        surface.pop();
    }
    result
}
fn draw_item(
    surface: &mut krilla::surface::Surface<'_>,
    drawing: &Drawing,
    scene: &Scene,
    fonts: &mut text::Fonts,
) -> Result<(), String> {
    match drawing {
        Drawing::Primitive(
            p @ crate::scene::Primitive::Text {
                text: _,
                x,
                y,
                size,
                bold,
                ..
            },
        ) => {
            if [*x, *y, *size].iter().any(|v| !v.is_finite()) || *size <= 0. {
                return Err("Invalid primitive text geometry".into());
            }
            // The native legacy painter uses ordinary-face overdraw. Keep one
            // searchable copy; outline only the offset ink so extraction does
            // not repeat the caption.
            let mut plain = p.clone();
            if let crate::scene::Primitive::Text { bold, .. } = &mut plain {
                *bold = false;
            }
            fonts.legacy(
                surface,
                &crate::scene::primitive_svg(plain.clone()),
                scene.font.as_deref(),
                false,
            )?;
            if *bold {
                if let crate::scene::Primitive::Text { x, .. } = &mut plain {
                    *x += crate::scene::LEGACY_BOLD_OFFSET;
                }
                fonts.legacy(
                    surface,
                    &crate::scene::primitive_svg(plain),
                    scene.font.as_deref(),
                    true,
                )?;
            }
            Ok(())
        }

        Drawing::Primitive(p) => vectors::primitive(surface, p),
        Drawing::Icon {
            bounds,
            id,
            turns,
            color,
        } => {
            if !valid_bounds(*bounds) {
                return Err("Invalid icon bounds".into());
            }
            surface.push_transform(&Transform::from_rotate_at(
                -90. * f32::from(*turns),
                bounds.x + bounds.w / 2.,
                bounds.y + bounds.h / 2.,
            ));
            let result = if let Some(icon) = crate::icons::get(id) {
                vectors::markup(surface, &icon.svg(*bounds, *color))
            } else {
                vectors::markup(
                    surface,
                    &format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"{}\"/>",
                        bounds.x,
                        bounds.y,
                        bounds.w,
                        bounds.h,
                        crate::scene::color([color[0], color[1], color[2]])
                    ),
                )
            };
            surface.pop();
            result
        }
        Drawing::Image { bounds, data } => vectors::image(surface, *bounds, &data.png),
        Drawing::Glyph(g) => {
            if !g.rotation.is_finite() {
                return Err("Invalid text rotation".into());
            }
            if g.rotation == 0. {
                return fonts.draw(surface, g, scene.font.as_deref());
            }
            surface.push_transform(&Transform::from_rotate_at(
                g.rotation.to_degrees(),
                g.x,
                g.baseline,
            ));
            let result = fonts.draw(surface, g, scene.font.as_deref());
            surface.pop();
            result
        }
        Drawing::BeginMask(_)
        | Drawing::EndMask
        | Drawing::BeginOpacity(_)
        | Drawing::EndOpacity => unreachable!(),
    }
}
