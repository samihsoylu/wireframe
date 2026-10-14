// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::mobile_keyboard::{Icon, Legend, Options, Surface};

impl Scene {
    pub(super) fn mobile_keyboard(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let b = layer.control.bounds;
        let clip = clip_with(layer.clip, b);
        let layout = Options::from_properties(&layer.properties).layout(b.w, b.h);
        if layout.scale <= 0. {
            return Ok(());
        }
        let offset = |r: Bounds| Bounds::new(b.x + r.x, b.y + r.y, r.w, r.h);
        self.primitive(
            Primitive::Rect {
                b: offset(layout.frame),
                fill: [238; 3],
                stroke: false,
                radius: 0.,
            },
            clip,
        );
        for key in layout.keys {
            let area = offset(key.bounds);
            let (sw, sh) = key.surface.size(sketch);
            let ink = if sketch || key.surface == Surface::Letter {
                [0; 3]
            } else {
                [51; 3]
            };
            let fill = if key.surface.light() {
                [255; 3]
            } else {
                [204; 3]
            };
            let thickness = if sketch { 1.5 } else { 1. };
            let transform = |points: Vec<[f32; 2]>| {
                points
                    .into_iter()
                    .map(|[x, y]| [area.x + x * area.w / sw, area.y + y * area.h / sh])
                    .collect()
            };
            for (inset, color) in [(0., ink), (thickness, fill)] {
                self.primitive(
                    Primitive::Polygon {
                        points: transform(key_contour(sw, sh, inset, sketch)),
                        fill: color,
                    },
                    clip,
                );
            }
            match key.legend {
                Legend::Text(text) => self.text(
                    &RichText::plain(text),
                    crate::app_bar::text_style(key.font_size),
                    TextBox {
                        bounds: area,
                        align: Alignment::Center,
                        middle: true,
                        wrap: false,
                        soft_breaks: true,
                        truncate: false,
                        clip,
                    },
                    fonts,
                    PaintOptions {
                        ink: if sketch {
                            Color32::BLACK
                        } else {
                            Color32::from_gray(51)
                        },
                        ..Default::default()
                    },
                )?,
                Legend::Icon(icon) => {
                    let size = key.icon_size;
                    self.keyboard_icon(
                        icon,
                        Bounds::new(
                            area.x + (area.w - size) / 2.,
                            area.y + (area.h - size) / 2.,
                            size,
                            size,
                        ),
                        sketch,
                        clip,
                    );
                }
            }
        }
        Ok(())
    }
    fn keyboard_icon(&mut self, icon: Icon, b: Bounds, sketch: bool, clip: Option<Bounds>) {
        let ink = if sketch { [0; 3] } else { [102; 3] };
        if matches!(icon, Icon::Globe | Icon::Microphone) {
            self.items.push(Item {
                drawing: Drawing::Icon {
                    bounds: b,
                    id: if icon == Icon::Globe {
                        "earth-americas-solid"
                    } else {
                        "microphone-solid"
                    }
                    .into(),
                    color: [ink[0], ink[1], ink[2], 255],
                    turns: 0,
                },
                clip,
                opacity: 1.,
            });
            return;
        }
        let pt = |x: f32, y: f32| [b.x + x * b.w, b.y + y * b.h];
        let mut paths = Vec::new();
        match icon {
            Icon::Shift => paths.push(vec![
                pt(0.5, 0.08),
                pt(0.94, 0.52),
                pt(0.67, 0.52),
                pt(0.67, 0.92),
                pt(0.33, 0.92),
                pt(0.33, 0.52),
                pt(0.06, 0.52),
                pt(0.5, 0.08),
            ]),
            Icon::Backspace => {
                paths.push(vec![
                    pt(0.05, 0.5),
                    pt(0.29, 0.19),
                    pt(0.95, 0.19),
                    pt(0.95, 0.81),
                    pt(0.29, 0.81),
                    pt(0.05, 0.5),
                ]);
                paths.push(vec![pt(0.48, 0.36), pt(0.76, 0.64)]);
                paths.push(vec![pt(0.48, 0.64), pt(0.76, 0.36)]);
            }
            Icon::Dismiss => {
                paths.push(vec![
                    pt(0.04, 0.14),
                    pt(0.96, 0.14),
                    pt(0.96, 0.65),
                    pt(0.04, 0.65),
                    pt(0.04, 0.14),
                ]);
                for row in 0..3 {
                    for col in 0..8 {
                        let x = 0.14 + col as f32 * 0.10;
                        let y = 0.24 + row as f32 * 0.10;
                        paths.push(vec![pt(x, y), pt(x + 0.035, y)]);
                    }
                }
                paths.push(vec![pt(0.27, 0.56), pt(0.73, 0.56)]);
                paths.push(vec![pt(0.32, 0.79), pt(0.5, 0.92), pt(0.68, 0.79)]);
            }
            _ => unreachable!(),
        }
        let color = if icon == Icon::Dismiss && !sketch {
            [51; 3]
        } else {
            ink
        };
        for points in paths {
            self.primitive(
                Primitive::Line {
                    points,
                    color,
                    width: b.w * if icon == Icon::Dismiss { 0.035 } else { 0.09 },
                },
                clip,
            );
        }
    }
}

// Original rounded key contours. Sketch introduces a small smooth variation;
// the whole key, including border thickness, stretches with its target bounds.
fn key_contour(w: f32, h: f32, inset: f32, sketch: bool) -> Vec<[f32; 2]> {
    let radius = (if sketch { 4. } else { 5. } - inset).max(0.);
    let corners = [
        (
            w - inset - radius,
            inset + radius,
            -std::f32::consts::FRAC_PI_2,
        ),
        (w - inset - radius, h - inset - radius, 0.),
        (
            inset + radius,
            h - inset - radius,
            std::f32::consts::FRAC_PI_2,
        ),
        (inset + radius, inset + radius, std::f32::consts::PI),
    ];
    let mut points = Vec::new();
    for (cx, cy, start) in corners {
        for i in 0..=10 {
            let angle = start + i as f32 / 10. * std::f32::consts::FRAC_PI_2;
            let x = cx + radius * angle.cos();
            let y = cy + radius * angle.sin();
            let dx = if sketch {
                0.18 * (y / h * std::f32::consts::TAU).sin()
            } else {
                0.
            };
            let dy = if sketch {
                0.16 * (x / w * std::f32::consts::TAU).sin()
            } else {
                0.
            };
            points.push([x + dx, y + dy]);
        }
    }
    points
}
