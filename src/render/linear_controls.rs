// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
impl Scene {
    pub(super) fn linear_control(&mut self, layer: &Layer, sketch: bool) {
        let b = layer.control.bounds;
        let axis = crate::linear_controls::Axis::for_kind(layer.control.kind).unwrap();
        let vertical = axis.vertical;
        let length = if vertical { b.h } else { b.w };
        let scroll = matches!(
            layer.control.kind,
            Kind::HorizontalScrollbar | Kind::VerticalScrollbar
        );
        let value = match layer.properties.get("value") {
            Some(Value::Number(n)) => *n as f32,
            _ => 0.,
        };
        let layout = if scroll {
            crate::linear_controls::scrollbar(vertical, length, value, sketch)
        } else {
            crate::linear_controls::splitter(vertical, length, sketch)
        };
        let clip = clip_with(layer.clip, b);
        let translate = |r: Bounds| Bounds::new(b.x + r.x, b.y + r.y, r.w, r.h);
        let point = |along: f32, cross: f32| {
            if vertical {
                [b.x + cross, b.y + along]
            } else {
                [b.x + along, b.y + cross]
            }
        };
        let ink = layout.ink;
        if let Some(bg) = layout.background.filter(|r| r.w > 0. && r.h > 0.) {
            self.primitive(
                Primitive::Rect {
                    b: translate(bg),
                    fill: [255; 3],
                    stroke: false,
                    radius: if sketch { 3. } else { 0. },
                },
                clip,
            );
        }
        if scroll {
            let width = if sketch { 1.5 } else { 0.7 };
            let inset = width / 2.;
            self.primitive(
                Primitive::Line {
                    points: vec![
                        point(inset, inset),
                        point(length - inset, inset),
                        point(length - inset, 16. - inset),
                        point(inset, 16. - inset),
                        point(inset, inset),
                    ],
                    color: ink,
                    width,
                },
                clip,
            );
            // Short saved bars compress their arrow compartments without adding a thumb.
            let end = 12_f32.min(length / 2.);
            if sketch {
                for along in [end, length - end] {
                    self.primitive(
                        Primitive::Line {
                            points: vec![point(along, 0.), point(along, 16.)],
                            color: ink,
                            width,
                        },
                        clip,
                    );
                }
            }
            for (along, direction) in [(end / 2., -1.), (length - end / 2., 1.)] {
                let depth = if sketch { 3. } else { 2.5 };
                self.primitive(
                    Primitive::Polygon {
                        points: vec![
                            point(along - direction * depth, 4.),
                            point(along - direction * depth, 12.),
                            point(along + direction * depth, 8.),
                        ],
                        fill: ink,
                    },
                    clip,
                );
            }
            if let Some(thumb) = layout.thumb {
                self.primitive(
                    Primitive::Rect {
                        b: translate(thumb),
                        fill: ink,
                        stroke: false,
                        radius: if sketch { 2. } else { 4. },
                    },
                    clip,
                );
            }
        } else {
            for cross in [2., 10.] {
                let points = if sketch {
                    vec![
                        point(0., cross),
                        point(length * 0.42, cross - 0.3),
                        point(length, cross + 0.1),
                    ]
                } else {
                    vec![point(0., cross), point(length, cross)]
                };
                self.primitive(
                    Primitive::Line {
                        points,
                        color: ink,
                        width: if sketch { 1.5 } else { 0.7 },
                    },
                    clip,
                );
            }
            let thumb = layout.thumb.unwrap();
            for i in 0..3 {
                let along = if vertical { thumb.y } else { thumb.x } + 1. + i as f32 * 6.;
                let cross = if vertical { thumb.x } else { thumb.y } + 1.;
                let p = point(along, cross);
                self.primitive(
                    Primitive::Rect {
                        b: Bounds::new(p[0], p[1], 2., 2.),
                        fill: ink,
                        stroke: false,
                        radius: if sketch { 0.5 } else { 0. },
                    },
                    clip,
                );
            }
        }
    }
}
