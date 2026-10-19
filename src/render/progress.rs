// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
impl Scene {
    pub(super) fn progress_control(&mut self, layer: &Layer, sketch: bool) {
        let b = layer.control.bounds;
        let p = &layer.properties;
        let thin = matches!(p.get("progress_style"), Some(Value::Choice(s)) if s == "thin");
        let indeterminate = boolean(p, "indeterminate");
        let value = match p.get("value") {
            Some(Value::Number(n)) => (*n as f32).clamp(0., 1.),
            _ => 0.5,
        };
        let ink = layer.control.fill;
        let clip = clip_with(layer.clip, b);
        let track = if thin {
            Bounds::new(b.x, b.y + 3., b.w, (b.h - 6.).max(0.))
        } else {
            b
        };
        self.primitive(
            Primitive::Rect {
                b: track,
                fill: if thin {
                    ink.map(|c| (c as f32 * 0.45 + 255. * 0.55).round() as u8)
                } else {
                    [255; 3]
                },
                stroke: false,
                radius: 0.,
            },
            clip,
        );
        let fill = if thin {
            Bounds::new(
                b.x + if indeterminate { b.w * 0.25 } else { 0. },
                b.y + 3.,
                b.w * if indeterminate { 0.5 } else { value },
                (b.h - 6.).max(0.),
            )
        } else {
            Bounds::new(
                b.x + 3.,
                b.y + 3.,
                (b.w * value - 6.).max(0.),
                (b.h - 6.).max(0.),
            )
        };
        if indeterminate && !thin {
            let inside = Bounds::new(b.x + 3., b.y + 3., (b.w - 6.).max(0.), (b.h - 6.).max(0.));
            let stripe_clip = clip_with(clip, inside);
            let mut x = inside.x;
            while x < inside.x + inside.w && inside.h > 0. {
                // Original diagonal stripe artwork repeats in a fixed-width tile.
                self.primitive(
                    Primitive::Polygon {
                        points: vec![
                            [x + 12.5, inside.y],
                            [x + 25., inside.y],
                            [x + 12.5, inside.y + inside.h],
                            [x, inside.y + inside.h],
                        ],
                        fill: ink,
                    },
                    stripe_clip,
                );
                x += 25.;
            }
        } else if fill.w > 0. && fill.h > 0. {
            self.primitive(
                Primitive::Rect {
                    b: fill,
                    fill: ink,
                    stroke: false,
                    radius: 0.,
                },
                clip,
            );
        }
        if !thin {
            let width = if sketch { 2. } else { 1. };
            let inset = (width / 2_f32).min(b.w / 2.).min(b.h / 2.);
            self.primitive(
                Primitive::Line {
                    points: vec![
                        [b.x + inset, b.y + inset],
                        [b.x + b.w - inset, b.y + inset],
                        [b.x + b.w - inset, b.y + b.h - inset],
                        [b.x + inset, b.y + b.h - inset],
                        [b.x + inset, b.y + inset],
                    ],
                    color: [0; 3],
                    width,
                },
                clip,
            );
        }
    }
}
