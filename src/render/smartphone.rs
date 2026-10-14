// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::phone::{circle, contour, section_axis};
use super::*;
use crate::{
    layout::TextMeasurer,
    smartphone::{DetailKind, Options, StatusKind},
};

impl Scene {
    pub(super) fn smartphone(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let b = layer.control.bounds;
        let clip = clip_with(layer.clip, b);
        let options = Options::from_properties(&layer.properties);
        let clock = layer
            .clock
            .unwrap_or_else(crate::tablet::Clock::now)
            .label_24();
        let layout = options.layout(b.w, b.h, sketch, |size| {
            fonts.measure(&clock, crate::app_bar::text_style(size), None)
        });
        let offset = |r: Bounds| Bounds::new(b.x + r.x, b.y + r.y, r.w, r.h);
        for fill in &layout.fills {
            if fill.bounds.w > 0. && fill.bounds.h > 0. {
                self.primitive(
                    Primitive::Polygon {
                        points: contour(offset(fill.bounds), 0., 0., false),
                        fill: fill.color,
                    },
                    clip,
                );
            }
        }
        for item in &layout.status_items {
            let area = offset(item.bounds);
            if area.w <= 0. || area.h <= 0. {
                continue;
            }
            match item.kind {
                StatusKind::Time => self.text(
                    &RichText::plain(&clock),
                    crate::app_bar::text_style(item.size),
                    TextBox {
                        bounds: area,
                        align: Alignment::Left,
                        middle: false,
                        wrap: false,
                        soft_breaks: false,
                        truncate: false,
                        clip,
                    },
                    fonts,
                    PaintOptions {
                        ink: Color32::WHITE,
                        ..Default::default()
                    },
                )?,
                StatusKind::Battery => {
                    let pt = |x: f32, y: f32| [area.x + x * area.w, area.y + y * area.h];
                    self.phone_line(
                        vec![
                            pt(0.25, 0.15),
                            pt(0.75, 0.15),
                            pt(0.75, 1.),
                            pt(0.25, 1.),
                            pt(0.25, 0.15),
                        ],
                        [255; 3],
                        (area.w / 15.).max(0.2),
                        clip,
                    );
                    self.phone_line(
                        vec![pt(0.4, 0.03), pt(0.6, 0.03)],
                        [255; 3],
                        (area.w / 15.).max(0.2),
                        clip,
                    );
                    self.primitive(
                        Primitive::Polygon {
                            points: vec![
                                pt(0.38, 0.28),
                                pt(0.62, 0.28),
                                pt(0.62, 0.87),
                                pt(0.38, 0.87),
                            ],
                            fill: [255; 3],
                        },
                        clip,
                    );
                }
                StatusKind::Signal | StatusKind::Wifi => self.phone_status_icon(
                    if item.kind == StatusKind::Signal {
                        crate::phone::StatusKind::Signal
                    } else {
                        crate::phone::StatusKind::Wifi
                    },
                    area,
                    [255; 3],
                    clip,
                ),
            }
        }
        // Original white frame, with a real transparent opening. Nine-section
        // deformation keeps the corner radii and end caps stable while stretching.
        let cuts = Options::sections(sketch);
        let transform = |p: [f32; 2]| {
            let (tw, th) = if options.landscape {
                (layout.frame.h, layout.frame.w)
            } else {
                (layout.frame.w, layout.frame.h)
            };
            let x = if options.stretch {
                section_axis(p[0], 356., tw, cuts[1], cuts[2])
            } else {
                p[0] * layout.zoom
            };
            let y = if options.stretch {
                section_axis(p[1], 725., th, cuts[0], cuts[3])
            } else {
                p[1] * layout.zoom
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
        let authored = |r: Bounds, radius: f32| {
            let mut points = contour(r, radius, radius, false)
                .into_iter()
                .enumerate()
                .map(|(i, mut p)| {
                    if sketch {
                        p[0] += (i as f32 * 0.33).sin() * 0.65;
                        p[1] += (i as f32 * 0.21).sin() * 0.65;
                    }
                    transform(p)
                })
                .collect::<Vec<_>>();
            *points.last_mut().unwrap() = points[0];
            points
        };
        let top = if sketch { 83. } else { 79. };
        let outer = authored(Bounds::new(1.5, 1.5, 353., 722.), 34.);
        let inner = authored(Bounds::new(19., top, 318., 725. - top - 80.), 0.);
        let n = outer.len() as u32;
        let ink = if sketch { [1; 3] } else { [103; 3] };
        let weight = if sketch { 3. } else { 1.8 } * if options.stretch { 1. } else { layout.zoom };
        self.primitive(
            Primitive::Mesh {
                points: outer.iter().chain(&inner).copied().collect(),
                indices: (0..n - 1)
                    .flat_map(|i| [i, i + 1, n + i + 1, i, n + i + 1, n + i])
                    .collect(),
                fill: [255; 3],
            },
            clip,
        );
        self.phone_line(outer, ink, weight, clip);
        self.phone_line(inner, ink, weight, clip);
        let mut details = layout.details;
        if !options.stretch {
            // Fitted hardware has slimmer outlines than the separately drawn
            // stretch hardware. Its coordinates belong to the portrait artwork.
            for (kind, r) in [
                (DetailKind::Bottom, Bounds::new(113., 695., 131., 8.)),
                (DetailKind::Speaker, Bounds::new(68., 29., 177., 10.)),
            ] {
                let bounds = if options.landscape {
                    Bounds::new(
                        layout.frame.x + r.y * layout.zoom,
                        layout.frame.y + (356. - r.x - r.w) * layout.zoom,
                        r.h * layout.zoom,
                        r.w * layout.zoom,
                    )
                } else {
                    Bounds::new(
                        layout.frame.x + r.x * layout.zoom,
                        layout.frame.y + r.y * layout.zoom,
                        r.w * layout.zoom,
                        r.h * layout.zoom,
                    )
                };
                details.push(crate::smartphone::Detail { kind, bounds });
            }
        }
        for detail in details {
            let r = offset(detail.bounds);
            let ink = if sketch { [1; 3] } else { [102; 3] };
            if detail.kind == DetailKind::Bottom {
                self.phone_line(
                    contour(r, r.w.min(r.h) / 2., r.w.min(r.h) / 2., false),
                    ink,
                    weight,
                    clip,
                );
            } else {
                // The speaker assembly includes a camera dot and a long capsule.
                let (camera, speaker) = if options.landscape {
                    (
                        Bounds::new(r.x, r.y + r.h - r.w, r.w, r.w),
                        Bounds::new(r.x, r.y, r.w, r.h * 0.73),
                    )
                } else {
                    (
                        Bounds::new(r.x, r.y, r.h, r.h),
                        Bounds::new(r.x + r.w * 0.27, r.y, r.w * 0.73, r.h),
                    )
                };
                self.phone_line(circle(camera), ink, weight, clip);
                let radius = speaker.w.min(speaker.h) / 2.;
                self.phone_line(contour(speaker, radius, radius, false), ink, weight, clip);
            }
        }
        Ok(())
    }
}
