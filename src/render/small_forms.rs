// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::small_forms as forms;

// Build stroke surfaces before mapping their vertices. A fixed screen-space
// stroke would incorrectly fill a one-unit-wide compressed frame solid black.
fn ribbon(
    points: &[[f32; 2]],
    width: f32,
    transform: impl Fn([f32; 2]) -> [f32; 2],
    color: [u8; 3],
) -> Primitive {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for pair in points.windows(2) {
        let [a, b] = [pair[0], pair[1]];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let length = dx.hypot(dy);
        if length == 0. {
            continue;
        }
        let n = [-dy / length * width / 2., dx / length * width / 2.];
        let first = vertices.len() as u32;
        vertices.extend(
            [
                [a[0] + n[0], a[1] + n[1]],
                [b[0] + n[0], b[1] + n[1]],
                [b[0] - n[0], b[1] - n[1]],
                [a[0] - n[0], a[1] - n[1]],
            ]
            .map(&transform),
        );
        indices.extend([first, first + 1, first + 2, first, first + 2, first + 3]);
    }
    Primitive::Mesh {
        points: vertices,
        indices,
        fill: color,
    }
}
impl Scene {
    pub(super) fn small_form(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let b = c.bounds;
        let p = &layer.properties;
        let clip = clip_with(layer.clip, b);
        let offset = |r: Bounds| Bounds::new(b.x + r.x, b.y + r.y, r.w, r.h);
        match c.kind {
            Kind::NumericStepper => {
                let disabled = matches!(p.get("state"), Some(Value::Choice(s)) if s == "disabled");
                let colors = forms::stepper_colors(
                    sketch,
                    disabled,
                    rgb(p, "border_color", if sketch { [0; 3] } else { [51; 3] }),
                );
                self.date_rect(b, [255; 3], clip);
                self.date_rect(offset(forms::stepper_button(b.w, b.h)), colors.button, clip);
                let transform = |a: [f32; 2]| {
                    let r = forms::stepper_point(a, b.w, b.h, sketch);
                    [b.x + r[0], b.y + r[1]]
                };
                // Independent frame and arrow artwork; corners retain their weight
                // while the field and central arrow strip stretch with the text.
                let height = if sketch { 26. } else { 24. };
                let outline = if sketch {
                    vec![
                        [1.3, 1.2],
                        [22., 1.0],
                        [42.6, 1.4],
                        [43., 12.],
                        [42.4, 24.6],
                        [22., 25.],
                        [1.1, 24.5],
                        [1.3, 1.2],
                    ]
                } else {
                    vec![
                        [0.5, 0.5],
                        [43.5, 0.5],
                        [43.5, 23.5],
                        [0.5, 23.5],
                        [0.5, 0.5],
                    ]
                };
                self.primitive(
                    ribbon(
                        &outline,
                        if sketch { 2. } else { 1. },
                        transform,
                        colors.border,
                    ),
                    clip,
                );
                self.primitive(
                    ribbon(
                        &[[26., 1.], [26., height - 1.]],
                        if sketch { 2. } else { 1. },
                        transform,
                        colors.border,
                    ),
                    clip,
                );
                for triangle in if sketch {
                    [
                        [[30.5, 11.], [35.5, 4.5], [40.5, 11.]],
                        [[30.5, 15.], [40.5, 15.], [35.5, 22.]],
                    ]
                } else {
                    [
                        [[30.5, 10.], [35., 4.], [39.5, 10.]],
                        [[30.5, 14.], [39.5, 14.], [35., 20.]],
                    ]
                } {
                    self.primitive(
                        Primitive::Polygon {
                            points: triangle.into_iter().map(transform).collect(),
                            fill: colors.border,
                        },
                        clip,
                    );
                }
                if !c.text.is_empty() {
                    let prior = self.links.len();
                    self.text(
                        &RichText::parse(&c.text, false),
                        TextStyle {
                            size: c.font_size,
                            bold: c.bold,
                            italic: boolean(p, "italic"),
                        },
                        TextBox {
                            bounds: offset(forms::stepper_text(b.w, b.h)),
                            align: Alignment::Right,
                            middle: false,
                            wrap: false,
                            soft_breaks: true,
                            truncate: true,
                            clip,
                        },
                        fonts,
                        PaintOptions {
                            ink: Color32::from_rgb(colors.text[0], colors.text[1], colors.text[2]),
                            link: Color32::from_rgb(colors.link[0], colors.link[1], colors.link[2]),
                            underline: boolean(p, "underline"),
                            ..Default::default()
                        },
                    )?;
                    self.links.truncate(prior);
                }
            }
            Kind::ColorPicker => {
                self.date_rect(offset(forms::picker_swatch(sketch)), c.fill, clip);
                if sketch {
                    self.primitive(
                        Primitive::Line {
                            points: vec![
                                [b.x + 1.6, b.y + 1.5],
                                [b.x + 23.8, b.y + 1.8],
                                [b.x + 23.1, b.y + 22.5],
                                [b.x + 12., b.y + 22.8],
                                [b.x + 1.4, b.y + 23.1],
                                [b.x + 1.6, b.y + 1.5],
                            ],
                            color: [0; 3],
                            width: 2.,
                        },
                        clip,
                    );
                    self.primitive(
                        Primitive::Polygon {
                            points: vec![
                                [b.x + 13., b.y + 18.5],
                                [b.x + 22.5, b.y + 18.],
                                [b.x + 17., b.y + 27.],
                            ],
                            fill: [0; 3],
                        },
                        clip,
                    );
                } else {
                    self.date_outline(offset(Bounds::new(0., 1., 24., 24.)), [103; 3], 1., clip);
                    self.date_outline(offset(Bounds::new(1., 2., 22., 22.)), [255; 3], 1., clip);
                    self.date_rect(offset(Bounds::new(12., 15., 11., 9.)), [255; 3], clip);
                    self.primitive(
                        Primitive::Polygon {
                            points: vec![
                                [b.x + 13., b.y + 17.],
                                [b.x + 22., b.y + 17.],
                                [b.x + 17.5, b.y + 22.],
                            ],
                            fill: [103; 3],
                        },
                        clip,
                    );
                }
            }
            Kind::HelpButton => {
                let color = if sketch { [0; 3] } else { [103; 3] };
                if sketch {
                    self.primitive(
                        Primitive::Ellipse {
                            b: offset(Bounds::new(2., 2., 14., 14.)),
                            fill: [255; 3],
                        },
                        clip,
                    );
                }
                let circle = (0..=64)
                    .map(|i| {
                        let a = i as f32 * std::f32::consts::TAU / 64.;
                        let r = 7. + if sketch { 0.16 * (3. * a).sin() } else { 0. };
                        [b.x + 9. + r * a.cos(), b.y + 9. + r * a.sin()]
                    })
                    .collect();
                self.primitive(
                    Primitive::Line {
                        points: circle,
                        color,
                        width: 2.,
                    },
                    clip,
                );
                // Original question mark, drawn as a small curved hook and dot.
                let hook = (0..=32)
                    .map(|i| {
                        let t = i as f32 / 32.;
                        let a = std::f32::consts::PI * (1. - 1.6 * t);
                        [b.x + 9. + 2.05 * a.cos(), b.y + 6.8 - 1.8 * a.sin()]
                    })
                    .chain([[b.x + 9., b.y + 9.6], [b.x + 9., b.y + 10.2]])
                    .collect();
                self.primitive(
                    Primitive::Line {
                        points: hook,
                        color,
                        width: 1.65,
                    },
                    clip,
                );
                self.primitive(
                    Primitive::Ellipse {
                        b: offset(Bounds::new(8.1, 12., 1.8, 1.8)),
                        fill: color,
                    },
                    clip,
                );
                if let Some(Value::Link(link)) = p.get("link") {
                    let bounds = clip.map_or(b, |v| intersect(b, v));
                    if bounds.w > 0. && bounds.h > 0. {
                        self.links.push(LinkRegion {
                            hint: bounds,
                            target: link.target(),
                            bounds,
                        });
                    }
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}
