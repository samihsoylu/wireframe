// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::static_layouts as layouts;
impl Scene {
    pub(super) fn static_layout(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let b = c.bounds;
        let p = &layer.properties;
        let clip = clip_with(layer.clip, b);
        let translate = |r: Bounds| Bounds::new(b.x + r.x, b.y + r.y, r.w, r.h);
        let ink = if sketch { [0; 3] } else { [51; 3] };
        match c.kind {
            Kind::FormattingToolbar => {
                self.date_rect(b, [255; 3], clip);
                self.date_outline(b, ink, if sketch { 2. } else { 1. }, clip);
                let layout = layouts::toolbar(b.w);
                for (index, rect) in layout.icons {
                    self.items.push(Item {
                        drawing: Drawing::Icon {
                            bounds: translate(rect),
                            id: layouts::TOOLBAR_ICONS[index].into(),
                            color: [ink[0], ink[1], ink[2], 255],
                            turns: 0,
                        },
                        clip,
                        opacity: 1.,
                    });
                }
                for r in layout.separators {
                    self.date_rect(translate(r), ink, clip);
                }
                if let Some(r) = layout.combo {
                    self.date_rect(translate(r), [255; 3], clip);
                    self.date_rect(
                        translate(Bounds::new(r.x + 20., r.y + 1., 30., 19.)),
                        if sketch { [255; 3] } else { [221; 3] },
                        clip,
                    );
                    self.date_outline(translate(r), ink, if sketch { 2. } else { 1. }, clip);
                    self.text(
                        &RichText::parse("style", false),
                        TextStyle {
                            size: 10.,
                            bold: true,
                            italic: true,
                        },
                        TextBox {
                            bounds: translate(Bounds::new(
                                r.x + if sketch { 6. } else { 4. },
                                r.y,
                                if sketch { 29. } else { 31. },
                                21.,
                            )),
                            align: Alignment::Left,
                            middle: true,
                            wrap: false,
                            soft_breaks: false,
                            truncate: true,
                            clip,
                        },
                        fonts,
                        PaintOptions {
                            ink: Color32::BLACK,
                            underline: true,
                            ..Default::default()
                        },
                    )?;
                    let x = b.x + r.x + 36.;
                    let y = b.y + r.y + 7.;
                    self.primitive(
                        Primitive::Polygon {
                            points: vec![[x, y], [x + 10., y], [x + 5., y + 7.]],
                            fill: ink,
                        },
                        clip,
                    );
                }
            }
            Kind::StreetMap => {
                let scale = |p: [f32; 2]| [b.x + p[0] / 252. * b.w, b.y + p[1] / 222. * b.h];
                let [blocks, park, road] = layouts::map_palette(sketch);
                if sketch {
                    self.date_rect(b, [255; 3], clip);
                }
                static BLOCKS: std::sync::LazyLock<Vec<Vec<[f32; 2]>>> =
                    std::sync::LazyLock::new(layouts::map_blocks);
                // One surface prevents antialiasing seams between partition pieces.
                let mut points = vec![];
                let mut indices = vec![];
                for polygon in BLOCKS.iter() {
                    let base = points.len() as u32;
                    points.extend(polygon.iter().copied().map(scale));
                    for i in 1..polygon.len() - 1 {
                        indices.extend([base, base + i as u32, base + i as u32 + 1]);
                    }
                }
                self.primitive(
                    Primitive::Mesh {
                        points,
                        indices,
                        fill: blocks,
                    },
                    clip,
                );
                self.primitive(
                    Primitive::Polygon {
                        points: [[8., 130.], [151., 106.], [160., 146.], [11., 159.]]
                            .into_iter()
                            .map(scale)
                            .collect(),
                        fill: park,
                    },
                    clip,
                );
                // Build the highlighted road in map coordinates before stretching.
                let route = if sketch {
                    [[-2., 161.], [241., 141.], [217., -2.]]
                } else {
                    [[-2., 162.], [241., 142.], [223., 35.]]
                };
                for pair in route.windows(2) {
                    let [a, z] = [pair[0], pair[1]];
                    let dx = z[0] - a[0];
                    let dy = z[1] - a[1];
                    let length = f32::hypot(dx, dy);
                    let nx = -dy / length * 3.;
                    let ny = dx / length * 3.;
                    self.primitive(
                        Primitive::Polygon {
                            points: [
                                [a[0] + nx, a[1] + ny],
                                [z[0] + nx, z[1] + ny],
                                [z[0] - nx, z[1] - ny],
                                [a[0] - nx, a[1] - ny],
                            ]
                            .into_iter()
                            .map(scale)
                            .collect(),
                            fill: road,
                        },
                        clip,
                    );
                }
            }
            Kind::ModalOverlay => {
                self.date_rect(b, [0; 3], clip);
                self.items.last_mut().unwrap().opacity = 0.4;
                if let Some(Value::Link(link)) = p.get("link") {
                    let bounds = clip.map_or(b, |v| intersect(b, v));
                    if bounds.w > 0. && bounds.h > 0. {
                        self.links.push(LinkRegion {
                            bounds,
                            target: link.target(),
                        });
                    }
                }
            }
            Kind::VerticalRule => {
                let color = rgb(p, "border_color", ink);
                let start = self.items.len();
                let stroke = match p.get("stroke") {
                    Some(Value::Choice(v)) => v.as_str(),
                    _ => "solid",
                };
                if stroke == "solid" {
                    if sketch {
                        self.primitive(
                            Primitive::Mesh {
                                indices: vec![
                                    0, 1, 9, 1, 2, 9, 2, 8, 9, 2, 3, 8, 3, 7, 8, 3, 4, 7, 4, 6, 7,
                                    4, 5, 6,
                                ],
                                points: layouts::rule_solid(b.h)
                                    .into_iter()
                                    .map(|[x, y]| [b.x + x - 0.5, b.y + y])
                                    .collect(),
                                fill: color,
                            },
                            clip,
                        );
                    } else {
                        self.date_rect(Bounds::new(b.x + 4.5, b.y, 1., b.h), color, clip);
                    }
                } else {
                    let rule = layouts::rule(b.h, stroke == "dotted", sketch);
                    for [a, z] in rule.segments {
                        self.primitive(
                            Primitive::Line {
                                points: vec![[b.x + 5., b.y + a], [b.x + 5., b.y + z]],
                                color,
                                width: rule.thickness,
                            },
                            clip,
                        );
                    }
                }
                let alpha = number(p, "background_opacity", 1.);
                for item in &mut self.items[start..] {
                    item.opacity = alpha;
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}
