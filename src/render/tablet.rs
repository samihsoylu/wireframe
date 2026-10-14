// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::phone::{circle, contour, section_axis};
use super::*;
use crate::{
    layout::TextMeasurer,
    tablet::{DetailKind, Options, StatusKind},
};

impl Scene {
    pub(super) fn tablet(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let b = layer.control.bounds;
        let clip = clip_with(layer.clip, b);
        let options = Options::from_properties(&layer.properties);
        let layout = options.layout(b.w, b.h, sketch, |size| {
            fonts.measure("Tab", crate::app_bar::text_style(size), None)
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
        let cuts = Options::sections(sketch);
        let transform = |p: [f32; 2]| {
            let (tw, th) = if options.landscape {
                (layout.frame.h, layout.frame.w)
            } else {
                (layout.frame.w, layout.frame.h)
            };
            let x = if options.stretch {
                section_axis(p[0], 616., tw, cuts[1], cuts[2])
            } else {
                p[0] * layout.zoom
            };
            let y = if options.stretch {
                section_axis(p[1], 916., th, cuts[0], cuts[3])
            } else {
                p[1] * layout.zoom
            };
            if options.landscape {
                [
                    b.x + layout.frame.x + layout.frame.w - y,
                    b.y + layout.frame.y + x,
                ]
            } else {
                [b.x + layout.frame.x + x, b.y + layout.frame.y + y]
            }
        };
        // Independently authored continuous contours share vertex counts so the
        // frame is one mesh with a real opening, without antialiased tile seams.
        let authored = |r: Bounds, radius: f32| {
            let mut points = contour(r, radius, radius, false)
                .into_iter()
                .enumerate()
                .map(|(i, mut p)| {
                    if sketch {
                        p[0] += (i as f32 * 0.31).sin() * 0.7;
                        p[1] += (i as f32 * 0.19).sin() * 0.7;
                    }
                    transform(p)
                })
                .collect::<Vec<_>>();
            let first = points[0];
            *points.last_mut().unwrap() = first;
            points
        };
        let outer = authored(Bounds::new(3., 3., 610., 910.), 12.);
        let inner = authored(Bounds::new(33., 93., 550., 730.), 1.);
        let n = outer.len() as u32;
        let body = if sketch { [69; 3] } else { [222, 221, 221] };
        let ink = if sketch { [0; 3] } else { [102, 103, 102] };
        let weight = if sketch { 4.5 } else { 1. } * if options.stretch { 1. } else { layout.zoom };
        self.primitive(
            Primitive::Mesh {
                points: outer.iter().chain(&inner).copied().collect(),
                indices: (0..n - 1)
                    .flat_map(|i| [i, i + 1, n + i + 1, i, n + i + 1, n + i])
                    .collect(),
                fill: body,
            },
            clip,
        );
        self.phone_line(outer, ink, weight * if sketch { 1.1 } else { 3. }, clip);
        self.phone_line(inner, ink, weight * if sketch { 1.1 } else { 0.75 }, clip);
        let clock = layer
            .clock
            .unwrap_or_else(crate::tablet::Clock::now)
            .label();
        for item in &layout.status_items {
            let area = offset(item.bounds);
            if area.w <= 0. || area.h <= 0. {
                continue;
            }
            match item.kind {
                StatusKind::Label | StatusKind::Time => self.text(
                    &RichText::plain(if item.kind == StatusKind::Label {
                        "Tab"
                    } else {
                        &clock
                    }),
                    crate::app_bar::text_style(item.size),
                    TextBox {
                        bounds: area,
                        align: if item.kind == StatusKind::Label {
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
                    PaintOptions {
                        ink: Color32::from_rgb(item.color[0], item.color[1], item.color[2]),
                        ..Default::default()
                    },
                )?,
                StatusKind::Wifi | StatusKind::Battery => self.phone_status_icon(
                    if item.kind == StatusKind::Wifi {
                        crate::phone::StatusKind::Wifi
                    } else {
                        crate::phone::StatusKind::Battery
                    },
                    area,
                    item.color,
                    clip,
                ),
            }
        }
        for detail in layout.details {
            let r = offset(detail.bounds);
            let inset = weight / 2.;
            let outline = Bounds::new(
                r.x + inset,
                r.y + inset,
                (r.w - weight).max(0.),
                (r.h - weight).max(0.),
            );
            let points = circle(outline);
            if detail.kind == DetailKind::Camera {
                if sketch {
                    self.primitive(
                        Primitive::Polygon {
                            points: circle(r),
                            fill: ink,
                        },
                        clip,
                    );
                } else {
                    self.phone_line(points, ink, weight, clip);
                }
            } else {
                self.primitive(
                    Primitive::Polygon {
                        points: points.clone(),
                        fill: body,
                    },
                    clip,
                );
                self.phone_line(points, ink, weight, clip);
                self.phone_line(
                    contour(
                        Bounds::new(r.x + r.w * 0.335, r.y + r.h * 0.335, r.w * 0.33, r.h * 0.33),
                        r.w * 0.05,
                        r.h * 0.05,
                        false,
                    ),
                    ink,
                    weight,
                    clip,
                );
            }
        }
        Ok(())
    }
}
