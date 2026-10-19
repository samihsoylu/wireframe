// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

// One filled capsule keeps round caps consistent in native, raster and vector
// output, without darkening overlapping cap/stem pieces at partial opacity.
pub(super) fn round_segment(a: [f32; 2], b: [f32; 2], width: f32, color: [u8; 3]) -> Primitive {
    let angle = (b[1] - a[1]).atan2(b[0] - a[0]);
    let half_turn = std::f32::consts::PI;
    let points: Vec<_> = [(a, angle + half_turn / 2.), (b, angle - half_turn / 2.)]
        .into_iter()
        .flat_map(|(center, from)| {
            (0..=16).map(move |step| {
                let t = from + half_turn * step as f32 / 16.;
                [
                    center[0] + t.cos() * width / 2.,
                    center[1] + t.sin() * width / 2.,
                ]
            })
        })
        .collect();
    let indices = (1..points.len() as u32 - 1)
        .flat_map(|i| [0, i, i + 1])
        .collect();
    Primitive::Mesh {
        points,
        indices,
        fill: color,
    }
}

impl Scene {
    pub(super) fn horizontal_rule(&mut self, layer: &Layer, sketch: bool) -> Result<(), String> {
        let b = layer.control.bounds;
        let p = &layer.properties;
        let color = rgb(p, "border_color", if sketch { [0; 3] } else { [51; 3] });
        let alpha = number(p, "background_opacity", 1.);
        let legacy = crate::rules::legacy_frame(p);
        let clip = if legacy {
            layer.clip
        } else {
            clip_with(layer.clip, b)
        };
        let stroke = crate::rules::stroke(p);
        let map = |point: [f32; 2]| [b.x + point[0], b.y + point[1]];
        if matches!(stroke, "breakline_top" | "breakline_bottom") {
            // This white mask remains opaque when only the border fades out.
            let points = crate::rules::white_region(b.w, b.h, stroke == "breakline_top")
                .into_iter()
                .map(map)
                .collect::<Vec<_>>();
            self.primitive(
                Primitive::Mesh {
                    indices: crate::popover::triangles(&points),
                    points,
                    fill: [255; 3],
                },
                clip,
            );
        }
        let start = self.items.len();
        match stroke {
            "solid" if legacy => self.primitive(
                Primitive::Line {
                    points: vec![[b.x, b.y + b.h / 2.], [b.x + b.w, b.y + b.h / 2.]],
                    color,
                    width: 1.5,
                },
                clip,
            ),
            "solid" if sketch => {
                let points = crate::static_layouts::rule_solid(b.w)
                    .into_iter()
                    .map(|[x, y]| [b.x + y, b.y + b.h / 2. + x - 5.5])
                    .collect::<Vec<_>>();
                self.primitive(
                    Primitive::Mesh {
                        indices: crate::popover::triangles(&points),
                        points,
                        fill: color,
                    },
                    clip,
                );
            }
            "solid" => self.date_rect(Bounds::new(b.x, b.y + b.h / 2. - 0.5, b.w, 1.), color, clip),
            "dashed" | "dotted" => {
                let rule = crate::static_layouts::rule(b.w, stroke == "dotted", sketch);
                for [a, z] in rule.segments {
                    self.primitive(
                        round_segment(
                            [b.x + a, b.y + b.h / 2.],
                            [b.x + z, b.y + b.h / 2.],
                            rule.thickness,
                            color,
                        ),
                        clip,
                    );
                }
            }
            "breakline_top" | "breakline_bottom" => {
                let thickness = if sketch {
                    3.
                } else if b.w < 65. {
                    2.
                } else {
                    1.
                };
                self.primitive(
                    Primitive::Line {
                        points: crate::rules::edge(b.w, b.h / 2., thickness)
                            .into_iter()
                            .map(map)
                            .collect(),
                        color,
                        width: thickness,
                    },
                    clip,
                );
            }
            _ => unreachable!("validated rule stroke"),
        }
        for item in &mut self.items[start..] {
            item.opacity = alpha;
        }
        Ok(())
    }
}
