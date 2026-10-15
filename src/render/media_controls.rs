// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Original vector drawings for media placeholders.
use super::*;
use std::f32::consts::{PI, TAU};
struct Art<'a> {
    scene: &'a mut Scene,
    origin: [f32; 2],
    scale: [f32; 2],
    clip: Option<Bounds>,
}
impl Art<'_> {
    fn point(&self, p: [f32; 2]) -> [f32; 2] {
        [
            self.origin[0] + p[0] * self.scale[0],
            self.origin[1] + p[1] * self.scale[1],
        ]
    }
    fn polygon(&mut self, p: Vec<[f32; 2]>, fill: [u8; 3]) {
        let points = p.into_iter().map(|p| self.point(p)).collect();
        self.scene
            .primitive(Primitive::Polygon { points, fill }, self.clip);
    }
    // Stroke each segment as a filled quad before the independent-axis transform.
    fn line(&mut self, p: Vec<[f32; 2]>, color: [u8; 3], width: f32) {
        for edge in p.windows(2) {
            let [a, b] = [edge[0], edge[1]];
            let d = [b[0] - a[0], b[1] - a[1]];
            let length = d[0].hypot(d[1]);
            if length < 0.00001 {
                continue;
            }
            let n = [-d[1] * width / (2. * length), d[0] * width / (2. * length)];
            self.polygon(
                vec![
                    [a[0] + n[0], a[1] + n[1]],
                    [b[0] + n[0], b[1] + n[1]],
                    [b[0] - n[0], b[1] - n[1]],
                    [a[0] - n[0], a[1] - n[1]],
                ],
                color,
            );
        }
    }
    fn ellipse(&mut self, b: Bounds, fill: [u8; 3], stroke: Option<([u8; 3], f32)>) {
        let points = |inset: f32| {
            (0..64)
                .map(|i| {
                    let t = i as f32 * TAU / 64.;
                    [
                        b.x + b.w / 2. + (b.w / 2. - inset) * t.cos(),
                        b.y + b.h / 2. + (b.h / 2. - inset) * t.sin(),
                    ]
                })
                .collect::<Vec<_>>()
        };
        self.polygon(points(0.), stroke.map_or(fill, |s| s.0));
        if let Some((_, width)) = stroke {
            self.polygon(points(width), fill);
        }
    }
}
impl Scene {
    pub(super) fn media_control(&mut self, layer: &Layer, sketch: bool) {
        let c = &layer.control;
        let b = c.bounds;
        let l = crate::media_controls::layout(c.kind, b.w, b.h, sketch);
        let mut a = Art {
            scene: self,
            origin: [b.x, b.y],
            scale: l.artwork_scale,
            clip: clip_with(layer.clip, b),
        };
        let ink = l.ink;
        let white = [255; 3];
        match c.kind {
            Kind::VolumeSlider => {
                // The speaker and knob form one fixed group; only the trailing line extends.
                let end = (l.frame.w - l.volume_right) / l.artwork_scale[0];
                a.line(
                    if sketch {
                        vec![[20., 9.], [34., 8.7], [52., 8.2], [end, 7.]]
                    } else {
                        vec![[20., 9.], [end, 9.]]
                    },
                    ink,
                    if sketch { 1.8 } else { 1. },
                );
                let speaker = vec![
                    [1., 5.],
                    [4.5, 5.],
                    [9., 1.],
                    [9., 15.],
                    [4.5, 11.],
                    [1., 11.],
                ];
                let speaker_fill = if sketch { white } else { ink };
                a.polygon(
                    vec![[1., 5.], [4.5, 5.], [4.5, 11.], [1., 11.]],
                    speaker_fill,
                );
                a.polygon(
                    vec![[4.5, 5.], [9., 1.], [9., 15.], [4.5, 11.]],
                    speaker_fill,
                );
                if sketch {
                    let mut outline = speaker;
                    outline.push(outline[0]);
                    a.line(outline, ink, 1.5);
                    a.line(vec![[4.5, 5.], [4.5, 11.]], ink, 1.2);
                }
                for (radius, width) in [(3.8, 1.1), (7., 1.)] {
                    let points = (0..18)
                        .map(|i| {
                            let t = -0.75 + i as f32 * 1.5 / 17.;
                            [9.5 + radius * t.cos(), 8. + radius * t.sin()]
                        })
                        .collect();
                    a.line(points, ink, width);
                }
                a.ellipse(
                    Bounds::new(39., 3., 11., 11.),
                    white,
                    Some((ink, if sketch { 1.6 } else { 1. })),
                );
                if sketch {
                    a.ellipse(Bounds::new(43., 7., 3., 3.), ink, None);
                }
            }
            Kind::Playback => {
                for (cx, r) in [(15., 13.), (55., 18.), (95., 13.)] {
                    a.ellipse(
                        Bounds::new(cx - r, 18. - r, 2. * r, 2. * r),
                        l.background,
                        sketch.then_some((ink, 1.6)),
                    );
                    let triangles: Vec<Vec<[f32; 2]>> = if cx == 55. {
                        vec![vec![[50., 10.], [50., 26.], [63., 18.]]]
                    } else {
                        let direction = if cx < 55. { -1. } else { 1. };
                        [-3.5, 3.5]
                            .into_iter()
                            .map(|dx| {
                                vec![
                                    [cx + dx - direction * 2.5, 13.],
                                    [cx + dx - direction * 2.5, 23.],
                                    [cx + dx + direction * 3., 18.],
                                ]
                            })
                            .collect()
                    };
                    for mut p in triangles {
                        if sketch {
                            p.push(p[0]);
                            a.line(p, ink, 1.4);
                        } else {
                            a.polygon(p, ink);
                        }
                    }
                }
            }
            Kind::Webcam => {
                a.polygon(
                    vec![[0., 0.], [177., 0.], [177., 146.], [0., 146.]],
                    l.background,
                );
                if sketch {
                    // Shoulder arc and face drawn independently, so the head covers the joining line.
                    let mut shoulders = (0..49)
                        .map(|i| {
                            let t = PI + i as f32 * PI / 48.;
                            [88.5 + 65. * t.cos(), 145. + 58. * t.sin()]
                        })
                        .collect::<Vec<_>>();
                    a.line(std::mem::take(&mut shoulders), ink, 2.4);
                    a.ellipse(Bounds::new(47., 21., 72., 77.), white, Some((ink, 2.3)));
                    for x in [68., 98.] {
                        a.ellipse(Bounds::new(x, 42., 5., 5.), ink, None);
                    }
                    let smile = (0..30)
                        .map(|i| {
                            let t = 0.15 + i as f32 * (PI - 0.3) / 29.;
                            [85. + 23. * t.cos(), 71. + 18. * t.sin()]
                        })
                        .collect();
                    a.line(smile, ink, 2.3);
                    a.line(
                        vec![[1., 1.], [176., 1.5], [175.5, 145.], [1.5, 144.5], [1., 1.]],
                        ink,
                        2.3,
                    );
                } else {
                    a.ellipse(Bounds::new(48., 13., 81., 101.), white, None);
                    // Convex pieces keep native canvas tessellation consistent with exports.
                    a.polygon(
                        vec![[74., 94.], [104., 94.], [104., 118.], [74., 118.]],
                        white,
                    );
                    a.polygon(
                        vec![
                            [74., 118.],
                            [104., 118.],
                            [159., 137.],
                            [165., 145.],
                            [12., 145.],
                            [18., 137.],
                        ],
                        white,
                    );
                }
            }
            _ => unreachable!(),
        }
    }
}
