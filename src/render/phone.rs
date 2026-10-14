// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::{
    layout::TextMeasurer,
    phone::{DetailKind, Model, Options, StatusKind},
};
use std::f32::consts::{FRAC_PI_2, PI};

// Original continuous contours. Corresponding samples form a transparent ring;
// no screen-colored patch is painted over content beneath the device.
pub(super) fn contour(b: Bounds, top: f32, bottom: f32, notch: bool) -> Vec<[f32; 2]> {
    let rt = top.min(b.w / 2.).min(b.h / 2.).max(0.);
    let rb = bottom.min(b.w / 2.).min(b.h / 2.).max(0.);
    let mut p = Vec::new();
    let smooth = |t: f32| t * t * (3. - 2. * t);
    for i in 0..=48 {
        let t = i as f32 / 48.;
        let x = b.x + rt + (b.w - 2. * rt) * t;
        let dip = if notch {
            let a = ((x - (b.x + b.w * 0.24)) / 10.).clamp(0., 1.);
            let z = ((b.x + b.w * 0.76 - x) / 10.).clamp(0., 1.);
            30. * smooth(a.min(z))
        } else {
            0.
        };
        p.push([x, b.y + dip]);
    }
    for i in 1..=12 {
        let a = -FRAC_PI_2 + i as f32 / 12. * FRAC_PI_2;
        p.push([b.x + b.w - rt + rt * a.cos(), b.y + rt + rt * a.sin()]);
    }
    p.push([b.x + b.w, b.y + b.h - rb]);
    for i in 1..=12 {
        let a = i as f32 / 12. * FRAC_PI_2;
        p.push([b.x + b.w - rb + rb * a.cos(), b.y + b.h - rb + rb * a.sin()]);
    }
    for i in 1..=48 {
        let t = i as f32 / 48.;
        p.push([b.x + b.w - rb - (b.w - 2. * rb) * t, b.y + b.h]);
    }
    for i in 1..=12 {
        let a = FRAC_PI_2 + i as f32 / 12. * FRAC_PI_2;
        p.push([b.x + rb + rb * a.cos(), b.y + b.h - rb + rb * a.sin()]);
    }
    p.push([b.x, b.y + rt]);
    for i in 1..=12 {
        let a = PI + i as f32 / 12. * FRAC_PI_2;
        p.push([b.x + rt + rt * a.cos(), b.y + rt + rt * a.sin()]);
    }
    p
}
pub(super) fn section_axis(p: f32, source: f32, target: f32, start: f32, end: f32) -> f32 {
    let compression = (target / (start + end)).min(1.);
    let a = start * compression;
    let z = end * compression;
    if p <= start {
        p * compression
    } else if p >= source - end {
        target - (source - p) * compression
    } else {
        a + (p - start) / (source - start - end) * (target - a - z).max(0.)
    }
}
pub(super) fn circle(b: Bounds) -> Vec<[f32; 2]> {
    (0..=40)
        .map(|i| {
            let a = i as f32 / 40. * 2. * PI;
            [
                b.x + b.w / 2. + b.w / 2. * a.cos(),
                b.y + b.h / 2. + b.h / 2. * a.sin(),
            ]
        })
        .collect()
}
impl Scene {
    pub(super) fn phone(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let b = c.bounds;
        let clip = clip_with(layer.clip, b);
        let options = Options::from_properties(&layer.properties);
        let layout = options.layout(b.w, b.h, sketch, |size| {
            let s = crate::app_bar::text_style(size);
            (
                fonts.measure("ABC", s, None),
                fonts.measure("09:52 AM", s, None),
            )
        });
        let offset = |r: Bounds| Bounds::new(b.x + r.x, b.y + r.y, r.w, r.h);
        for fill in &layout.fills {
            if fill.bounds.w <= 0. || fill.bounds.h <= 0. {
                continue;
            }
            self.primitive(
                Primitive::Polygon {
                    points: contour(
                        offset(fill.bounds),
                        if fill.round_top { fill.radius } else { 0. },
                        if fill.round_bottom { fill.radius } else { 0. },
                        false,
                    ),
                    fill: fill.color,
                },
                clip,
            );
        }
        for item in &layout.status_items {
            let area = offset(item.bounds);
            if area.w <= 0. || area.h <= 0. {
                continue;
            }
            match item.kind {
                StatusKind::Carrier | StatusKind::Time => {
                    let paint = PaintOptions {
                        ink: Color32::from_rgb(item.color[0], item.color[1], item.color[2]),
                        ..Default::default()
                    };
                    self.text(
                        &RichText::plain(if item.kind == StatusKind::Carrier {
                            "ABC"
                        } else {
                            "09:52 AM"
                        }),
                        crate::app_bar::text_style(item.size),
                        TextBox {
                            bounds: area,
                            align: if item.kind == StatusKind::Carrier {
                                Alignment::Left
                            } else {
                                Alignment::Center
                            },
                            middle: true,
                            wrap: false,
                            soft_breaks: false,
                            truncate: false,
                            clip,
                        },
                        fonts,
                        paint,
                    )?;
                }
                _ => self.phone_status_icon(item.kind, area, item.color, clip),
            }
        }
        let (native_w, native_h) = options.model.native();
        let native_w = if options.model == Model::Notched {
            434.
        } else {
            native_w
        };
        let cuts = options.model.sections(sketch);
        let transform = |p: [f32; 2]| {
            let (tw, th) = if options.landscape {
                (layout.frame.h, layout.frame.w)
            } else {
                (layout.frame.w, layout.frame.h)
            };
            let x = if options.stretch {
                section_axis(p[0], native_w, tw, cuts[1], cuts[2])
            } else {
                p[0] * tw / native_w
            };
            let y = if options.stretch {
                section_axis(p[1], native_h, th, cuts[0], cuts[3])
            } else {
                p[1] * th / native_h
            };
            if options.landscape {
                [
                    b.x + layout.frame.x + y,
                    b.y + layout.frame.y + layout.frame.h - x,
                ]
            } else {
                [b.x + layout.frame.x + x, b.y + layout.frame.y + y]
            }
        };
        let (insets, radius, inner_radius) = match options.model {
            Model::Classic | Model::Tall => ([24., 24., 83., 80.], 36., 2.),
            Model::Wide => ([29., 29., 90., 95.], 44., 1.),
            Model::Notched => ([24., 24., 22., 22.], 58., 36.),
        };
        let outer = Bounds::new(4., 2., native_w - 8., native_h - 4.);
        let inner = Bounds::new(
            insets[0],
            insets[2],
            native_w - insets[0] - insets[1],
            native_h - insets[2] - insets[3],
        );
        let authored = |r: Bounds, rad: f32, notch: bool| {
            let mut points = contour(r, rad, rad, notch)
                .into_iter()
                .enumerate()
                .map(|(i, mut p)| {
                    if sketch {
                        let t = i as f32;
                        p[0] += (t * 0.31).sin() * 0.65;
                        p[1] += (t * 0.19).sin() * 0.65;
                    }
                    transform(p)
                })
                .collect::<Vec<_>>();
            let first = points[0];
            *points.last_mut().unwrap() = first;
            points
        };
        let outer_points = authored(outer, radius, false);
        let inner_points = authored(inner, inner_radius, options.model == Model::Notched);
        let body = if sketch { [193, 192, 192] } else { [255; 3] };
        let ink = if sketch { [0; 3] } else { [102; 3] };
        let weight = if sketch { 4. } else { 1.5 };
        let scale = if options.stretch { 1. } else { layout.zoom };
        let n = outer_points.len() as u32;
        let points = outer_points.iter().chain(&inner_points).copied().collect();
        let indices = (0..n - 1)
            .flat_map(|i| [i, i + 1, n + i + 1, i, n + i + 1, n + i])
            .collect();
        self.primitive(
            Primitive::Mesh {
                points,
                indices,
                fill: body,
            },
            clip,
        );
        self.phone_line(outer_points, ink, weight * scale, clip);
        self.phone_line(inner_points, ink, weight * scale, clip);
        // Small side switches are authored with the silhouette and share its map.
        for (x, y, w, h) in [
            (1., native_h * 0.13, 4., native_h * 0.035),
            (1., native_h * 0.22, 4., native_h * 0.06),
            (1., native_h * 0.31, 4., native_h * 0.06),
            (native_w - 5., native_h * 0.26, 4., native_h * 0.075),
        ] {
            self.primitive(
                Primitive::Polygon {
                    points: contour(Bounds::new(x, y, w, h), 1., 1., false)
                        .into_iter()
                        .map(transform)
                        .collect(),
                    fill: ink,
                },
                clip,
            );
        }
        if options.stretch {
            for detail in &layout.details {
                self.phone_detail(
                    detail.kind,
                    offset(detail.visible_bounds()),
                    options.landscape,
                    options.model,
                    ink,
                    body,
                    weight,
                    clip,
                );
            }
        } else {
            let original = Options {
                landscape: false,
                stretch: true,
                ..options
            }
            .layout(native_w, native_h, sketch, |size| {
                (
                    crate::layout::TextMetrics {
                        width: 0.,
                        height: size,
                        rows: 1,
                    },
                    crate::layout::TextMetrics {
                        width: 0.,
                        height: size,
                        rows: 1,
                    },
                )
            });
            for detail in original.details {
                // Render in native coordinates first, then transform every point.
                let before = self.items.len();
                self.phone_detail(
                    detail.kind,
                    detail.bounds,
                    false,
                    options.model,
                    ink,
                    body,
                    weight,
                    clip,
                );
                for item in &mut self.items[before..] {
                    if let Drawing::Primitive(primitive) = &mut item.drawing {
                        match primitive {
                            Primitive::Polygon { points, .. } => {
                                for p in points {
                                    *p = transform(*p);
                                }
                            }
                            Primitive::Line { points, width, .. } => {
                                for p in points {
                                    *p = transform(*p);
                                }
                                *width *= scale;
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }
        }
        Ok(())
    }
    pub(super) fn phone_line(
        &mut self,
        points: Vec<[f32; 2]>,
        color: [u8; 3],
        width: f32,
        clip: Option<Bounds>,
    ) {
        self.primitive(
            Primitive::Line {
                points,
                color,
                width,
            },
            clip,
        );
    }
    #[allow(clippy::too_many_arguments)]
    fn phone_detail(
        &mut self,
        kind: DetailKind,
        b: Bounds,
        landscape: bool,
        model: Model,
        ink: [u8; 3],
        body: [u8; 3],
        width: f32,
        clip: Option<Bounds>,
    ) {
        let map = |x: f32, y: f32| {
            if landscape {
                [b.x + y * b.w, b.y + (1. - x) * b.h]
            } else {
                [b.x + x * b.w, b.y + y * b.h]
            }
        };
        let local = |r: Bounds, round: bool| {
            let points = if round {
                circle(r)
            } else {
                contour(r, 0.12, 0.12, false)
            };
            points
                .into_iter()
                .map(|p| map(p[0], p[1]))
                .collect::<Vec<_>>()
        };
        if kind == DetailKind::Home {
            let points = local(Bounds::new(0., 0., 1., 1.), true);
            self.primitive(
                Primitive::Polygon {
                    points: points.clone(),
                    fill: body,
                },
                clip,
            );
            self.phone_line(points, ink, width, clip);
            if model != Model::Wide {
                self.phone_line(
                    local(Bounds::new(0.3, 0.3, 0.4, 0.4), false),
                    ink,
                    width * 0.65,
                    clip,
                );
            }
        } else {
            let slot = if model == Model::Notched {
                Bounds::new(0., 0.15, 0.64, 0.5)
            } else {
                Bounds::new(0.25, 0.5, 0.65, 0.3)
            };
            self.phone_line(local(slot, false), ink, width * 0.75, clip);
            let camera = if model == Model::Notched {
                Bounds::new(0.83, 0., 0.13, 0.65)
            } else {
                Bounds::new(0.03, 0.1, 0.12, 0.3)
            };
            self.phone_line(local(camera, true), ink, width * 0.75, clip);
        }
    }
    pub(super) fn phone_status_icon(
        &mut self,
        kind: StatusKind,
        b: Bounds,
        ink: [u8; 3],
        clip: Option<Bounds>,
    ) {
        let pt = |x: f32, y: f32| [b.x + x * b.w, b.y + y * b.h];
        let width = (b.w / 15.).max(0.2);
        match kind {
            StatusKind::Signal => {
                for i in 0..5 {
                    let x = 0.08 + i as f32 * 0.19;
                    self.phone_line(
                        vec![pt(x, 0.95), pt(x, 0.85 - i as f32 * 0.18)],
                        ink,
                        width,
                        clip,
                    );
                }
            }
            StatusKind::Wifi => {
                for radius in [0.3, 0.57, 0.84] {
                    let points = (0..=16)
                        .map(|i| {
                            let a = -PI * 0.78 + i as f32 / 16. * PI * 0.56;
                            pt(
                                0.5 + radius * a.cos() * 0.65,
                                0.94 + radius * a.sin() * 0.85,
                            )
                        })
                        .collect();
                    self.phone_line(points, ink, width, clip);
                }
                self.phone_line(vec![pt(0.5, 0.88), pt(0.5, 0.95)], ink, width * 1.4, clip);
            }
            StatusKind::Battery => {
                self.phone_line(
                    vec![
                        pt(0., 0.25),
                        pt(0.85, 0.25),
                        pt(0.85, 0.75),
                        pt(0., 0.75),
                        pt(0., 0.25),
                    ],
                    ink,
                    width,
                    clip,
                );
                self.phone_line(vec![pt(0.97, 0.4), pt(0.97, 0.6)], ink, width, clip);
                self.primitive(
                    Primitive::Polygon {
                        points: vec![
                            pt(0.15, 0.38),
                            pt(0.62, 0.38),
                            pt(0.62, 0.62),
                            pt(0.15, 0.62),
                        ],
                        fill: ink,
                    },
                    clip,
                );
            }
            _ => unreachable!(),
        }
    }
}
