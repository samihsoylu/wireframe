// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::text_controls;
use std::f32::consts::TAU;

/// An original continuous pen stroke. Smooth changes in slant and amplitude avoid
/// disconnected glyph-like stamps; every backend receives the same path.
fn scribble(b: Bounds, seed: usize) -> Primitive {
    let loops = (b.w / b.h.max(1.) * 1.15).max(1.);
    let count = (loops * 24.).ceil().clamp(24., 12000.) as usize;
    let phase = seed as f32 * 0.71;
    let points = (0..=count)
        .map(|i| {
            let t = i as f32 / count as f32;
            let a = t * loops * TAU;
            let wave = a.sin();
            let envelope = 0.8 + 0.2 * (a * 0.13 + phase).sin();
            [
                b.x + t * b.w + wave * b.h * 0.23,
                b.y + b.h * (0.5 + 0.3 * wave * envelope + 0.035 * (a * 0.37 + phase).cos()),
            ]
        })
        .collect();
    Primitive::Line {
        points,
        color: [0; 3],
        width: b.h * 0.36,
    }
}
impl Scene {
    pub(super) fn text_control(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let b = c.bounds;
        let p = &layer.properties;
        let clip = clip_with(layer.clip, b);
        let offset = |r: Bounds| Bounds::new(b.x + r.x, b.y + r.y, r.w, r.h);
        match c.kind {
            Kind::TextBlock | Kind::TextLine => {
                let boxes = if c.kind == Kind::TextLine {
                    vec![Bounds::new(
                        0.,
                        0.,
                        if sketch {
                            text_controls::line_width(b.w, b.h)
                        } else {
                            b.w
                        },
                        b.h,
                    )]
                } else if sketch {
                    text_controls::block_words(b.w, b.h, number(p, "_project_font_size", 13.))
                } else {
                    text_controls::block_rows(b.w, b.h, number(p, "_project_font_size", 13.))
                };
                for (index, r) in boxes.into_iter().enumerate() {
                    if sketch {
                        self.primitive(scribble(offset(r), index), clip);
                    } else {
                        self.primitive(
                            Primitive::Rect {
                                b: offset(r),
                                fill: [51; 3],
                                stroke: false,
                                radius: 0.,
                            },
                            clip,
                        );
                        self.items.last_mut().unwrap().opacity = 0.5;
                    }
                }
            }
            Kind::Heading | Kind::Subtitle => {
                let title = match p.get("_page_title") {
                    Some(Value::Text(s)) => s.as_str(),
                    _ => self.title.as_str(),
                };
                let text = text_controls::title_text(&c.text, c.kind == Kind::Subtitle, title);
                self.text(
                    &text,
                    TextStyle {
                        size: c.font_size,
                        bold: c.bold,
                        italic: boolean(p, "italic"),
                    },
                    TextBox {
                        bounds: offset(text_controls::title_box(
                            b.w,
                            b.h,
                            c.kind == Kind::Subtitle,
                        )),
                        align: alignment(p),
                        middle: false,
                        wrap: false,
                        soft_breaks: true,
                        truncate: true,
                        clip: layer.clip,
                    },
                    fonts,
                    paint_options(p),
                )?;
            }
            Kind::TagCloud => {
                let style = TextStyle {
                    size: c.font_size,
                    bold: false,
                    italic: false,
                };
                for label in text_controls::cloud_layout(&c.text, c.id, b.w, b.h, fonts, style) {
                    if label.visible {
                        self.text(
                            &label.text,
                            TextStyle {
                                size: label.font,
                                ..style
                            },
                            TextBox {
                                bounds: offset(label.bounds),
                                align: Alignment::Left,
                                middle: false,
                                wrap: false,
                                soft_breaks: true,
                                truncate: true,
                                clip,
                            },
                            fonts,
                            paint_options(p),
                        )?;
                    }
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}
