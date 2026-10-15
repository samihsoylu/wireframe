// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Original chart contours and procedural sectors; no external artwork is embedded.
use super::*;
use crate::charts::{self, Family};
use std::f32::consts::TAU;
fn circle(center: [f32; 2], radius: f32, start: f32, sweep: f32, steps: usize) -> Vec<[f32; 2]> {
    (0..=steps)
        .map(|i| {
            let a = start + sweep * i as f32 / steps as f32;
            [center[0] + radius * a.cos(), center[1] + radius * a.sin()]
        })
        .collect()
}
impl Scene {
    pub(super) fn chart(&mut self, layer: &Layer, family: Family, sketch: bool) {
        let b = layer.control.bounds;
        let l = charts::layout(family, b.w, b.h, sketch);
        let clip = clip_with(layer.clip, b);
        let offset = |r: Bounds| Bounds::new(b.x + r.x, b.y + r.y, r.w, r.h);
        let translate = |points: Vec<[f32; 2]>| {
            points
                .into_iter()
                .map(|[x, y]| [b.x + x, b.y + y])
                .collect()
        };
        let line = |s: charts::Stroke| Primitive::Line {
            points: translate(s.points),
            color: s.color,
            width: s.width,
        };
        if matches!(family, Family::Column | Family::Bar) {
            // Filled strips preserve flat axis ends, including frames narrower than the stroke.
            let t = l.axes[0].width;
            for r in [
                Bounds::new(0., 0., t, b.h),
                Bounds::new(0., b.h - t, b.w, t),
            ] {
                self.primitive(
                    Primitive::Rect {
                        b: offset(r),
                        fill: l.ink,
                        stroke: false,
                        radius: 0.,
                    },
                    clip,
                );
            }
            for bar in l.bars {
                {
                    let fill = bar.fill;
                    self.primitive(
                        Primitive::Rect {
                            b: offset(fill),
                            fill: bar.color,
                            stroke: false,
                            radius: 0.,
                        },
                        clip,
                    );
                }
                let r = bar.frame;
                if sketch {
                    // A single independent ring avoids internal antialias seams. Small bars
                    // compress the edge widths on each axis without expanding the frame.
                    let tx = 2.6_f32.min(r.w / 2.);
                    let ty = 2.6_f32.min(r.h / 2.);
                    let outer = [
                        [0., 0.6],
                        [0.35, 0.],
                        [1., 0.5],
                        [1., 1.],
                        [0.65, 1.],
                        [0., 0.97],
                    ];
                    let points: Vec<_> = outer
                        .iter()
                        .map(|[x, y]| {
                            [
                                r.x + x * r.w,
                                r.y + if *y < 0.9 {
                                    y * ty
                                } else {
                                    r.h - (1. - y) * ty
                                },
                            ]
                        })
                        .chain(outer.iter().map(|[x, y]| {
                            [
                                r.x + tx + x * (r.w - 2. * tx),
                                r.y + ty
                                    + if *y < 0.9 {
                                        y * ty * 0.1
                                    } else {
                                        r.h - 2. * ty - (1. - y) * ty
                                    },
                            ]
                        }))
                        .collect();
                    let mut indices = vec![];
                    for i in 0..6u32 {
                        let j = (i + 1) % 6;
                        indices.extend([i, j, j + 6, i, j + 6, i + 6]);
                    }
                    self.primitive(
                        Primitive::Mesh {
                            points: translate(points),
                            indices,
                            fill: l.ink,
                        },
                        clip,
                    );
                } else {
                    self.rectangle_frame(offset(r), l.ink, 1., clip);
                }
            }
        } else if family == Family::Line {
            for s in l.series.into_iter().chain(l.axes) {
                self.primitive(line(s), clip);
            }
        } else if let Some(p) = l.pie {
            let inner = p.background;
            let center = [inner.x + inner.w / 2., inner.y + inner.h / 2.];
            let radius = inner.w / 2.;
            self.primitive(
                Primitive::Polygon {
                    points: translate(circle(center, radius, 0., TAU, 128)),
                    fill: [230; 3],
                },
                clip,
            );
            let mut wedge = vec![center];
            wedge.extend(circle(
                center,
                radius,
                120.0_f32.to_radians(),
                150.0_f32.to_radians(),
                64,
            ));
            self.primitive(
                Primitive::Polygon {
                    points: translate(wedge),
                    fill: [167; 3],
                },
                clip,
            );
            self.primitive(
                Primitive::Line {
                    points: translate(p.divider),
                    color: l.ink,
                    width: p.width,
                },
                clip,
            );
            self.primitive(
                Primitive::Line {
                    points: translate(circle(p.center, p.radius, 0., TAU, 128)),
                    color: l.ink,
                    width: p.width,
                },
                clip,
            );
        }
    }
}
