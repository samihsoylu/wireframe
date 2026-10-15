// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Original procedural markup artwork, shared by the native canvas and exports.
use super::*;
use crate::markup::{self, Family};
use std::f32::consts::PI;

/// A continuous ribbon with shared edges. Each surface is painted once, so
/// translucent strokes keep their opacity at joins in every backend.
fn ribbon(
    samples: &[[f32; 2]],
    half_width: impl Fn(usize) -> f32,
    map: impl Fn([f32; 2]) -> [f32; 2],
    color: [u8; 3],
) -> Primitive {
    let mut points = Vec::with_capacity(samples.len() * 2);
    let mut indices = Vec::with_capacity((samples.len() - 1) * 6);
    for (i, p) in samples.iter().enumerate() {
        let a = samples[i.saturating_sub(1)];
        let b = samples[(i + 1).min(samples.len() - 1)];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let scale = half_width(i) / dx.hypot(dy).max(0.0001);
        points.push(map([p[0] - dy * scale, p[1] + dx * scale]));
        points.push(map([p[0] + dy * scale, p[1] - dx * scale]));
        if i > 0 {
            let j = (2 * i) as u32;
            indices.extend([j - 2, j - 1, j, j - 1, j + 1, j]);
        }
    }
    Primitive::Mesh {
        points,
        indices,
        fill: color,
    }
}
fn brace_curve(sketch: bool) -> Vec<[f32; 2]> {
    // Four smooth cubic segments form an original symmetric brace. The slight
    // style variation is deterministic, so saved documents export consistently.
    let mut points = Vec::new();
    let lift = if sketch { 1.2 } else { 0. };
    for [a, b, c, d] in [
        [[0., 2.], [28., 17.], [112., -6. + lift], [156., 4.]],
        [[156., 4.], [176., 5.], [187., 8.], [194., 14.]],
        [[194., 14.], [201., 8.], [212., 5.], [232., 4.]],
        [[232., 4.], [276., -6. - lift], [360., 17.], [388., 2.]],
    ] {
        for i in 0..=32 {
            if !points.is_empty() && i == 0 {
                continue;
            }
            let t = i as f32 / 32.;
            let u = 1. - t;
            points.push([0, 1].map(|axis| {
                u * u * u * a[axis]
                    + 3. * u * u * t * b[axis]
                    + 3. * u * t * t * c[axis]
                    + t * t * t * d[axis]
            }));
        }
    }
    points
}
impl Scene {
    pub(super) fn markup(
        &mut self,
        layer: &Layer,
        family: Family,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let b = c.bounds;
        let p = &layer.properties;
        let direction = match p.get("direction") {
            Some(Value::Choice(v)) => v.as_str(),
            _ => "",
        };
        let l = markup::layout(family, b.w, b.h, direction);
        let clip = clip_with(layer.clip, b);
        let ink = rgb(p, "text_color", if sketch { [0; 3] } else { [51; 3] });
        if family.brace() {
            let samples = brace_curve(sketch);
            let count = samples.len();
            self.primitive(
                ribbon(
                    &samples,
                    |i| {
                        let t = i as f32 / (count - 1) as f32;
                        // f32 sin(PI) can be slightly negative at the tapered
                        // endpoint; keep the ribbon coordinates finite.
                        (PI * t).sin().max(0.).sqrt() * if sketch { 1.6 } else { 1.25 }
                    },
                    |[along, across]| {
                        let (x, y) = if l.horizontal {
                            (along / 388. * b.w, across)
                        } else {
                            (across, along / 388. * b.h)
                        };
                        if l.reverse {
                            [b.x + b.w - x, b.y + b.h - y]
                        } else {
                            [b.x + x, b.y + y]
                        }
                    },
                    ink,
                ),
                clip,
            );
            if let Some(label) = l.label {
                let title = match p.get("_page_title") {
                    Some(Value::Text(t)) => t.as_str(),
                    _ => self.title.as_str(),
                };
                let text = crate::annotations::Family::Callout.parse(&c.text, title);
                let style = TextStyle {
                    size: c.font_size,
                    bold: c.bold,
                    italic: boolean(p, "italic"),
                };
                let align = if l.horizontal {
                    Alignment::Center
                } else if l.reverse {
                    Alignment::Right
                } else {
                    Alignment::Left
                };
                self.text_rows_anchored(
                    &text,
                    style,
                    TextBox {
                        bounds: Bounds::new(b.x + label.x, b.y + label.y, label.w, label.h),
                        align,
                        middle: !l.horizontal,
                        wrap: true,
                        soft_breaks: true,
                        truncate: true,
                        clip,
                    },
                    fonts,
                    paint_options(p),
                    true,
                    l.horizontal && l.reverse,
                )?;
            }
        } else if family == Family::RedCross {
            let ink = markup::cross_color(sketch);
            for flip in [false, true] {
                let samples: Vec<_> = (0..=256)
                    .map(|i| {
                        let t = i as f32 / 256.;
                        [
                            6. + 228. * t,
                            6. + 92. * t
                                + if sketch {
                                    (t * PI * 2.).sin() * 1.4
                                } else {
                                    0.
                                },
                        ]
                    })
                    .collect();
                self.primitive(
                    ribbon(
                        &samples,
                        |i| {
                            // Rounded ends, constructed before independent axis scaling.
                            let edge = ((i as f32).min((256 - i) as f32) * 228.0_f32.hypot(92.)
                                / 256.
                                / 5.5)
                                .min(1.);
                            5.5 * (1. - (1. - edge).powi(2)).sqrt()
                        },
                        |[x, y]| {
                            [
                                b.x + x / 240. * b.w,
                                b.y + (if flip { 104. - y } else { y }) / 104. * b.h,
                            ]
                        },
                        ink,
                    ),
                    clip,
                );
            }
        } else {
            // Parallel strokes joined by semicircles form one non-overlapping
            // ribbon. Its bend radius exceeds its width, including in Sketch.
            let mut samples = Vec::new();
            let rotate = |u: f32, v: f32| {
                let v = v * (0.92 + 0.10 * (u * 0.073).sin()) + 2.0 * (u * 0.12).sin();
                [0.87 * u - 0.49 * v, 0.49 * u + 0.87 * v]
            };
            for lane in 0..17 {
                let sign = if lane % 2 == 0 { 1. } else { -1. };
                let u = lane as f32 * 8.;
                for step in 0..=32 {
                    if lane > 0 && step == 0 {
                        continue;
                    }
                    let t = step as f32 / 32.;
                    samples.push([u, sign * (-40. + 80. * t)]);
                }
                if lane < 16 {
                    for step in 1..=24 {
                        let a = PI * step as f32 / 24.;
                        samples.push([u + 4. - 4. * a.cos(), sign * (40. + 4. * a.sin())]);
                    }
                }
            }
            let count = samples.len();
            let primitive = ribbon(
                &samples,
                |i| {
                    let edge = (i as f32).min((count - 1 - i) as f32).min(2.) / 2.;
                    let width = if sketch {
                        3.05 + (i as f32 * 0.021).sin() * 0.12
                    } else {
                        3.1
                    };
                    width * (1. - (1. - edge).powi(2)).sqrt()
                },
                |[u, v]| rotate(u, v),
                c.fill,
            );
            let Primitive::Mesh {
                mut points,
                indices,
                fill,
            } = primitive
            else {
                unreachable!()
            };
            let min = [0, 1].map(|a| points.iter().map(|p| p[a]).fold(f32::INFINITY, f32::min));
            let max = [0, 1].map(|a| {
                points
                    .iter()
                    .map(|p| p[a])
                    .fold(f32::NEG_INFINITY, f32::max)
            });
            for point in &mut points {
                *point = [
                    b.x + (point[0] - min[0]) / (max[0] - min[0]) * b.w,
                    b.y + (point[1] - min[1]) / (max[1] - min[1]) * b.h,
                ];
            }
            self.primitive(
                Primitive::Mesh {
                    points,
                    indices,
                    fill,
                },
                clip,
            );
            self.items.last_mut().unwrap().opacity = number(p, "background_opacity", 1.);
        }
        Ok(())
    }
}
