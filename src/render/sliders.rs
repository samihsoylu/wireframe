// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::sliders::{Options, Thumb};
impl Scene {
    pub(super) fn slider(&mut self, layer: &Layer, sketch: bool) {
        let b = layer.control.bounds;
        let vertical = layer.control.kind == Kind::VerticalSlider;
        let o = Options::from_properties(
            &layer.properties,
            vertical,
            sketch,
            if vertical { b.h } else { b.w },
        );
        let l = crate::sliders::layout(o);
        let clip = clip_with(layer.clip, b);
        // Original rail artwork, kept inside the same per-control crop as the thumb.
        for (i, track) in l.tracks.iter().enumerate() {
            let color = if i == 0 {
                l.leading_color
            } else {
                l.rest_color
            };
            let length = if vertical { track.h } else { track.w };
            if length <= 0. {
                continue;
            }
            let cross = if vertical {
                if sketch { 5.6 } else { 6.5 }
            } else if sketch {
                6.5
            } else {
                6.45
            };
            let start = if vertical { track.y } else { track.x };
            let width = if sketch { 2. } else { 1. };
            let points = if sketch {
                vec![
                    [start, cross - 0.6],
                    [start + length * 0.35, cross - 0.9],
                    [start + length, cross - 0.75],
                    [start + length, cross + 1.2],
                    [start + length * 0.35, cross + 0.9],
                    [start, cross + 0.7],
                ]
            } else {
                vec![
                    [start, cross - width / 2.],
                    [start + length, cross - width / 2.],
                    [start + length, cross + width / 2.],
                    [start, cross + width / 2.],
                ]
            };
            self.primitive(
                Primitive::Polygon {
                    points: points
                        .into_iter()
                        .map(|[along, across]| {
                            if vertical {
                                [b.x + across, b.y + along]
                            } else {
                                [b.x + along, b.y + across]
                            }
                        })
                        .collect(),
                    fill: color,
                },
                clip,
            );
        }
        let t = l.thumb;
        let mut points = match o.thumb {
            Thumb::Circle => (0..=48)
                .map(|i| {
                    let a = i as f32 * std::f32::consts::TAU / 48.;
                    let radius = if sketch {
                        4.8 + 0.15 * (3. * a).sin()
                    } else {
                        5.5
                    };
                    [6. + radius * a.cos(), 6. + radius * a.sin()]
                })
                .collect::<Vec<_>>(),
            Thumb::Rectangle => {
                if sketch {
                    vec![
                        [3.6, 1.5],
                        [7.9, 1.4],
                        [8.2, 10.],
                        [7.6, 10.5],
                        [4.1, 10.3],
                        [3.5, 9.8],
                    ]
                } else {
                    vec![
                        [3.5, 1.],
                        [7.7, 1.],
                        [8., 1.3],
                        [8., 10.],
                        [7.7, 10.3],
                        [3.5, 10.3],
                        [3.2, 10.],
                        [3.2, 1.3],
                    ]
                }
            }
            Thumb::Pointy => {
                if sketch {
                    vec![
                        [1.5, 1.7],
                        [9.9, 1.4],
                        [10.5, 2.],
                        [10.3, 7.7],
                        [6., 10.8],
                        [5.4, 10.7],
                        [1.5, 7.6],
                    ]
                } else {
                    vec![
                        [1., 0.5],
                        [9.8, 0.5],
                        [10.2, 0.9],
                        [10.2, 5.5],
                        [9.9, 6.2],
                        [6.1, 10.1],
                        [5.5, 10.1],
                        [1.3, 6.1],
                        [1., 5.5],
                    ]
                }
            }
        };
        if vertical && o.thumb != Thumb::Circle {
            for p in &mut points {
                p.swap(0, 1);
            }
        }
        self.primitive(
            Primitive::Polygon {
                points: points
                    .into_iter()
                    .map(|[x, y]| [b.x + t.x + x, b.y + t.y + y])
                    .collect(),
                fill: l.rest_color,
            },
            clip,
        );
    }
}
