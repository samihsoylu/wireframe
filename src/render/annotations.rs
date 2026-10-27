// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Original annotation artwork; all views and exports consume these same paths.
use super::*;
use crate::{
    annotations::{self, Direction, Family},
    layout::TextMeasurer,
};
use std::f32::consts::{FRAC_PI_2, PI};
fn round_outline(b: Bounds, radius: f32) -> Vec<[f32; 2]> {
    let rx = radius.min(b.w / 2.).max(0.);
    let ry = radius.min(b.h / 2.).max(0.);
    let mut out = Vec::new();
    for (x, y, a) in [
        (b.x + rx, b.y + ry, PI),
        (b.x + b.w - rx, b.y + ry, 1.5 * PI),
        (b.x + b.w - rx, b.y + b.h - ry, 0.),
        (b.x + rx, b.y + b.h - ry, FRAC_PI_2),
    ] {
        for i in 0..=16 {
            let a = a + i as f32 * FRAC_PI_2 / 16.;
            out.push([x + rx * a.cos(), y + ry * a.sin()]);
        }
    }
    out
}
// Original continuous oval ribbon: a slightly tilted hand-drawn contour
// and a heavier lower rim. Shared edges avoid double opacity at joins.
fn sketch_callout(b: Bounds, seed: f32) -> (Vec<[f32; 2]>, Option<Primitive>) {
    const N: usize = 96;
    let mut outer = Vec::with_capacity(N);
    let mut inner = Vec::with_capacity(N);
    let unit = (b.w.min(b.h) / 39.).clamp(0., 1.);
    let rx = (b.w / 2. - 0.6 * unit).max(0.);
    let ry = (b.h / 2. - 0.6 * unit).max(0.);
    for i in 0..N {
        let t = i as f32 * 2. * PI / N as f32;
        let wobble = (t * 3. + seed * 0.71).sin() * 0.55 * unit;
        let tilt = t.sin() * 1.25 * unit;
        let point = [
            b.x + b.w / 2. + (rx + wobble) * t.cos() + tilt,
            b.y + b.h / 2. + (ry + wobble * 0.65) * t.sin(),
        ];
        let rim = (2.5 + seed * 0.07 + 2.6 * t.sin().max(0.)) * unit;
        outer.push(point);
        inner.push([
            point[0] - t.cos() * rim.min(rx),
            point[1] - t.sin() * rim.min(ry),
        ]);
    }
    let mut indices = Vec::with_capacity(N * 6);
    let points: Vec<_> = outer.into_iter().chain(inner.iter().copied()).collect();
    for i in 0..N {
        let next = (i + 1) % N;
        for mut triangle in [[i, next, N + i], [next, N + next, N + i]] {
            let [a, b, c] = triangle.map(|j| points[j]);
            if (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]) < 0. {
                triangle.swap(1, 2);
            }
            indices.extend(triangle.map(|j| j as u32));
        }
    }
    (
        inner,
        Some(Primitive::Mesh {
            points,
            indices,
            fill: [0; 3],
        }),
    )
}

fn tint(c: [u8; 3], amount: f32) -> [u8; 3] {
    c.map(|v| {
        (v as f32 + (255. - v as f32) * amount)
            .round()
            .clamp(0., 255.) as u8
    })
}
impl Scene {
    pub(super) fn annotation(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let b = c.bounds;
        let p = &layer.properties;
        let family = Family::for_kind(c.kind).expect("annotation");
        let title = match p.get("_page_title") {
            Some(Value::Text(t)) => t.as_str(),
            _ => self.title.as_str(),
        };
        let text = family.parse(&c.text, title);
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let metrics = fonts.measure_rich(&text, style, None, true);
        let align = alignment(p);
        let direction = Direction::from_value(match p.get("direction") {
            Some(Value::Choice(v)) => v,
            _ => "top_left",
        });
        let l = annotations::layout(
            family,
            b.w,
            b.h,
            sketch,
            metrics,
            matches!(align, Alignment::Left),
            direction,
        );
        let offset = |r: Bounds| Bounds::new(b.x + r.x, b.y + r.y, r.w, r.h);
        let clip = clip_with(layer.clip, b);
        let ink = if sketch { [0; 3] } else { [51; 3] };
        let opacity = number(p, "background_opacity", 1.);
        match family {
            Family::Callout => {
                let (surface, outline) = if sketch {
                    sketch_callout(b, (c.id % 9) as f32)
                } else {
                    (round_outline(b, 30.), None)
                };
                self.primitive(
                    Primitive::Polygon {
                        points: surface,
                        fill: c.fill,
                    },
                    clip,
                );
                self.items.last_mut().unwrap().opacity = opacity;
                if let Some(outline) = outline {
                    self.primitive(outline, clip);
                }
            }
            Family::StickyNote => {
                let paper = offset(l.frame);
                let base = if sketch { tint(c.fill, 0.17) } else { c.fill };
                // Edge-anchored corners keep paper detail readable when the middle stretches.
                let ex = paper.w.min(8.);
                let ey = paper.h.min(5.);
                self.primitive(
                    Primitive::Polygon {
                        points: vec![
                            [paper.x + ex * 0.12, paper.y],
                            [paper.x + paper.w, paper.y + ey * 0.2],
                            [paper.x + paper.w - ex * 0.18, paper.y + paper.h - ey * 0.22],
                            [paper.x + paper.w * 0.61, paper.y + paper.h],
                            [paper.x, paper.y + paper.h - ey * 0.35],
                        ],
                        fill: base,
                    },
                    layer.clip,
                );
                if sketch {
                    // Independent low-contrast paper facets, each convex for the native mesh path.
                    for (points, amount) in [
                        (
                            vec![[0.01, 0.02], [0.98, 0.03], [0.56, 0.22], [0.18, 0.18]],
                            0.14,
                        ),
                        (
                            vec![[0.02, 0.23], [0.20, 0.19], [0.51, 0.31], [0.08, 0.54]],
                            0.10,
                        ),
                        (
                            vec![[0.33, 0.33], [0.77, 0.15], [0.96, 0.17], [0.65, 0.40]],
                            0.09,
                        ),
                        (
                            vec![[0.05, 0.58], [0.40, 0.49], [0.60, 0.59], [0.08, 0.68]],
                            0.08,
                        ),
                        (vec![[0.13, 0.70], [0.51, 0.66], [0.36, 0.77]], 0.07),
                    ] {
                        self.primitive(
                            Primitive::Polygon {
                                points: points
                                    .into_iter()
                                    .map(|[x, y]| [paper.x + x * paper.w, paper.y + y * paper.h])
                                    .collect(),
                                fill: tint(base, amount),
                            },
                            layer.clip,
                        );
                    }
                } else {
                    self.primitive(
                        Primitive::Line {
                            points: vec![
                                [paper.x + ex, paper.y + ey * 0.65],
                                [paper.x + paper.w - ex, paper.y + ey * 0.65],
                            ],
                            color: tint(base, 0.10),
                            width: 0.7,
                        },
                        layer.clip,
                    );
                }
                if let Some(t) = l.tape {
                    let t = offset(t);
                    self.primitive(
                        Primitive::Polygon {
                            points: vec![
                                [t.x, t.y + 4.],
                                [t.x + t.w - 1., t.y + 1.],
                                [t.x + t.w, t.y + t.h - 3.],
                                [t.x + 2., t.y + t.h],
                            ],
                            fill: [215, 47, 36],
                        },
                        layer.clip,
                    );
                    self.primitive(
                        Primitive::Polygon {
                            points: vec![
                                [t.x, t.y + 4.],
                                [t.x + t.w - 1., t.y + 1.],
                                [t.x + t.w - 1., t.y + 9.],
                                [t.x + 1., t.y + 12.],
                            ],
                            fill: [232, 91, 79],
                        },
                        layer.clip,
                    );
                }
            }
            Family::Tooltip => {
                let body = offset(l.frame);
                let radius = if sketch { 12. } else { 1. };
                self.primitive(
                    Primitive::Polygon {
                        points: round_outline(body, radius),
                        fill: [255; 3],
                    },
                    clip,
                );
                let half = if sketch { 1. } else { 0.5 };
                let inset = Bounds::new(
                    body.x + half,
                    body.y + half,
                    (body.w - 2. * half).max(0.),
                    (body.h - 2. * half).max(0.),
                );
                let mut outline = round_outline(inset, radius);
                outline.push(outline[0]);
                self.primitive(
                    Primitive::Line {
                        points: outline,
                        color: if sketch { ink } else { [102; 3] },
                        width: half * 2.,
                    },
                    clip,
                );
                if let Some(t) = l.pointer {
                    let point = |x: f32, y: f32| {
                        [
                            b.x + t.x + if l.mirror[0] { -x } else { x },
                            b.y + t.y + if l.mirror[1] { -y } else { y },
                        ]
                    };
                    let points = if sketch {
                        vec![point(0., 14.), point(3., 1.), point(16., 14.)]
                    } else {
                        vec![point(0., 7.7), point(7., 1.), point(14., 7.7)]
                    };
                    self.primitive(
                        Primitive::Polygon {
                            points: points.clone(),
                            fill: [255; 3],
                        },
                        clip,
                    );
                    self.primitive(
                        Primitive::Line {
                            points,
                            color: if sketch { ink } else { [102; 3] },
                            width: if sketch { 2. } else { 1. },
                        },
                        clip,
                    );
                }
            }
        }
        if let Some(label) = l.label {
            let mut paint = paint_options(p);
            paint.ink = Color32::from_rgb(ink[0], ink[1], ink[2]);
            paint.link = color32(link_color(p));
            if family != Family::Tooltip && annotations::light_text(family, c.fill, opacity, sketch)
            {
                paint.ink = Color32::WHITE;
                paint.link = Color32::WHITE;
            }
            self.text_rows(
                &text,
                style,
                TextBox {
                    bounds: offset(label),
                    align: if family == Family::Callout {
                        Alignment::Left
                    } else {
                        align
                    },
                    middle: family != Family::StickyNote,
                    wrap: true,
                    soft_breaks: true,
                    truncate: true,
                    clip,
                },
                fonts,
                paint,
                true,
            )?;
        }
        Ok(())
    }
}
