// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use crate::{
    fonts::ProjectFont,
    model::Bounds,
    render::Glyph,
    text_engine::{FontFace, FontSource, Ink},
};
use krilla::{
    color::rgb,
    geom::{Point, Transform},
    num::NormalizedF32,
    paint::Fill,
    surface::Surface,
    tagging::{ContentTag, SpanTag},
    text::{Font, GlyphId, KrillaGlyph, Tag},
};
use std::{collections::HashMap, sync::Arc};
#[derive(Default)]
pub(super) struct Fonts {
    // Retain the Arc alongside its pointer key so addresses cannot be reused.
    native: HashMap<usize, (Arc<FontFace>, Font)>,
    // Page snapshots retain their project fonts for the entire encode call.
    // Reuse the resolver instead of reloading every face for every text span.
    legacy_options: HashMap<usize, resvg::usvg::Options<'static>>,
}
impl Fonts {
    pub(super) fn legacy(
        &mut self,
        surface: &mut Surface<'_>,
        fragment: &str,
        project: Option<&ProjectFont>,
    ) -> Result<(), String> {
        let key = project.map_or(0, |font| font as *const ProjectFont as usize);
        let options = self.legacy_options.entry(key).or_insert_with(|| {
            let mut options = resvg::usvg::Options::default();
            for font in eframe::egui::FontDefinitions::default().font_data.values() {
                options.fontdb_mut().load_font_data(font.font.to_vec());
            }
            crate::fonts::install_export(options.fontdb_mut(), project);
            options.fontdb_mut().set_sans_serif_family("Ubuntu");
            options
        });
        legacy(surface, fragment, options)
    }
    pub(super) fn draw(
        &mut self,
        surface: &mut Surface<'_>,
        g: &Glyph,
        project: Option<&ProjectFont>,
    ) -> Result<(), String> {
        if [g.x, g.baseline, g.size, g.face_scale]
            .iter()
            .any(|v| !v.is_finite())
            || g.size <= 0.
            || g.face_scale <= 0.
        {
            return Err("Invalid PDF text geometry".into());
        }
        // Positioned SVG ink applies text alpha to the complete run, including
        // overlapping marks and bitmap glyphs, rather than to each glyph alone.
        if g.positioned.is_some() {
            surface.push_opacity(NormalizedF32::new(f32::from(g.color[3]) / 255.).unwrap());
            let result = self.draw_inner(surface, g, project);
            surface.pop();
            result
        } else {
            self.draw_inner(surface, g, project)
        }
    }

    fn draw_inner(
        &mut self,
        surface: &mut Surface<'_>,
        g: &Glyph,
        project: Option<&ProjectFont>,
    ) -> Result<(), String> {
        surface.set_stroke(None);
        surface.set_fill(Some(Fill {
            paint: rgb::Color::new(g.color[0], g.color[1], g.color[2]).into(),
            opacity: NormalizedF32::ONE,
            ..Default::default()
        }));
        if let Some(shaped) = &g.positioned {
            for run in &shaped.runs {
                let embedded = if let FontSource::Embedded(face) = &run.font_source {
                    let key = Arc::as_ptr(face) as usize;
                    if let Some((_, font)) = self.native.get(&key) {
                        Some(font.clone())
                    } else if let Ok(data) = face.data() {
                        let coords: Vec<_> = face
                            .variations
                            .iter()
                            .map(|(t, v)| (Tag::new(t), *v))
                            .collect();
                        let font =
                            Font::new_variable(Arc::new(data.to_vec()).into(), face.index, &coords);
                        if let Some(font) = &font {
                            self.native.insert(key, (face.clone(), font.clone()));
                        }
                        font
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(font) = embedded {
                    let size = run.font_size;
                    let mut pen = [0., 0.];
                    let glyphs: Vec<_> = run
                        .glyphs
                        .iter()
                        .map(|glyph| {
                            let out = KrillaGlyph {
                                glyph_id: GlyphId::new(u32::from(glyph.id)),
                                text_range: glyph.source.clone(),
                                x_advance: (glyph.advance[0] / size) as f32,
                                y_advance: (glyph.advance[1] / size) as f32,
                                x_offset: ((glyph.position[0] - pen[0]) / size) as f32,
                                y_offset: ((glyph.position[1] - pen[1]) / size) as f32,
                                location: None,
                            };
                            pen[0] += glyph.advance[0];
                            pen[1] += glyph.advance[1];
                            out
                        })
                        .collect();
                    surface.draw_glyphs(
                        Point::from_xy(g.x, g.baseline),
                        &glyphs,
                        font,
                        &shaped.source,
                        size as f32,
                        false,
                    );
                } else {
                    surface.start_tagged(ContentTag::Span(
                        SpanTag::empty().with_actual_text(Some(&shaped.source[run.source.clone()])),
                    ));
                    let result = (|| {
                        for glyph in &run.glyphs {
                            let x = g.x + glyph.position[0] as f32;
                            let y = g.baseline + glyph.position[1] as f32;
                            match &*glyph.ink {
                                Ink::Outline(path) => {
                                    surface.push_transform(&Transform::from_translate(x, y));
                                    let result = super::vectors::markup(
                                        surface,
                                        &format!(
                                            "<path d=\"{}\" fill=\"{}\" fill-opacity=\"{}\"/>",
                                            crate::scene::escape(path),
                                            crate::scene::color([
                                                g.color[0], g.color[1], g.color[2]
                                            ]),
                                            1.0
                                        ),
                                    );
                                    surface.pop();
                                    result?;
                                }
                                Ink::Bitmap {
                                    png,
                                    bounds: [left, top, w, h],
                                } => {
                                    surface.push_opacity(NormalizedF32::new(1.0).unwrap());
                                    let result = super::vectors::image(
                                        surface,
                                        Bounds::new(
                                            x + *left as f32,
                                            y + *top as f32,
                                            *w as f32,
                                            *h as f32,
                                        ),
                                        png,
                                    );
                                    surface.pop();
                                    result?;
                                }
                                Ink::Empty => {}
                            }
                        }
                        Ok::<_, String>(())
                    })();
                    surface.end_tagged();
                    result?;
                }
            }
        } else {
            let primary = g.face.starts_with("canvas-");
            let family = if primary {
                crate::fonts::export_family(project).to_owned()
            } else {
                crate::fonts::export_fallback_family(&g.face)
            };
            self.legacy(
                surface,
                &format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\" font-weight=\"{}\" font-style=\"{}\" fill=\"{}\" fill-opacity=\"{}\" xml:space=\"preserve\">{}</text>",
                    g.x,
                    g.baseline,
                    crate::scene::escape(&family),
                    g.size * g.face_scale,
                    if primary && g.bold { 700 } else { 400 },
                    if primary && g.italic {
                        "italic"
                    } else {
                        "normal"
                    },
                    crate::scene::color([g.color[0], g.color[1], g.color[2]]),
                    f32::from(g.color[3]) / 255.,
                    crate::scene::escape(&g.text)
                ),
                project,
            )?;
        }
        Ok(())
    }
}

/// Non-native runs and generic placeholders use the same font resolver and
/// positioned glyphs as SVG export, including its bidi/fallback layout.
fn legacy(
    surface: &mut Surface<'_>,
    fragment: &str,
    options: &resvg::usvg::Options<'_>,
) -> Result<(), String> {
    use resvg::usvg;
    let source = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\">{fragment}</svg>"
    );
    let tree = usvg::Tree::from_str(&source, options)
        .map_err(|e| format!("Cannot lay out PDF text: {e}"))?;
    fn draw(
        surface: &mut Surface<'_>,
        group: &usvg::Group,
        tree: &usvg::Tree,
    ) -> Result<(), String> {
        for node in group.children() {
            match node {
                usvg::Node::Group(g) => draw(surface, g, tree)?,
                usvg::Node::Text(text) => {
                    for span in text.layouted() {
                        if !span.visible {
                            continue;
                        }
                        let mut fonts = HashMap::new();
                        let mut coords: Vec<_> = span
                            .variations
                            .iter()
                            .map(|v| (Tag::new(&v.tag), v.value))
                            .collect();
                        if span.font_optical_sizing == usvg::FontOpticalSizing::Auto
                            && !span.variations.iter().any(|v| v.tag == *b"opsz")
                        {
                            coords.push((Tag::new(b"opsz"), span.font_size.get()));
                        }
                        if let Some(fill) = &span.fill {
                            let usvg::Paint::Color(c) = fill.paint() else {
                                return Err("Unsupported text paint".into());
                            };
                            surface.set_fill(Some(Fill {
                                paint: rgb::Color::new(c.red, c.green, c.blue).into(),
                                opacity: NormalizedF32::new(fill.opacity().get()).unwrap(),
                                ..Default::default()
                            }));
                        }
                        surface.set_stroke(None);
                        for glyph in &span.positioned_glyphs {
                            let font = if let Some(font) = fonts.get(&glyph.font) {
                                Font::clone(font)
                            } else {
                                let font = tree
                                    .fontdb()
                                    .with_face_data(glyph.font, |bytes, index| {
                                        Font::new_variable(
                                            Arc::new(bytes.to_vec()).into(),
                                            index,
                                            &coords,
                                        )
                                    })
                                    .flatten()
                                    .ok_or("Missing resolved PDF font")?;
                                fonts.insert(glyph.font, font.clone());
                                font
                            };
                            let t = glyph.transform();
                            surface.push_transform(&Transform::from_row(
                                t.sx, t.ky, t.kx, t.sy, t.tx, t.ty,
                            ));
                            let size = font.units_per_em();
                            let g = KrillaGlyph {
                                glyph_id: GlyphId::new(glyph.id.0),
                                text_range: 0..glyph.text.len(),
                                x_advance: 0.,
                                y_advance: 0.,
                                x_offset: 0.,
                                y_offset: 0.,
                                location: None,
                            };
                            surface.draw_glyphs(
                                Point::from_xy(0., 0.),
                                &[g],
                                font,
                                &glyph.text,
                                size,
                                false,
                            );
                            surface.pop();
                        }
                    }
                }
                _ => return Err("Unexpected drawing in PDF text layout".into()),
            }
        }
        Ok(())
    }
    draw(surface, tree.root(), &tree)
}
