// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

#[derive(Clone, Copy)]
pub(super) enum Mark {
    Empty,
    Selected,
    Mixed,
}
impl Scene {
    pub(super) fn choice_control(&mut self, layer: &Layer) {
        let c = &layer.control;
        let radio = c.kind == Kind::Radio;
        let size = if radio { 13. } else { 16. };
        let state = match layer.properties.get("state") {
            Some(Value::Choice(s)) => s.as_str(),
            _ => "normal",
        };
        let mark = if state.contains("indeterminate") {
            Mark::Mixed
        } else if state.contains("selected") {
            Mark::Selected
        } else {
            Mark::Empty
        };
        let b = Bounds::new(
            c.bounds.x,
            c.bounds.y + ((c.bounds.h - size) / 2.).round(),
            size,
            size,
        );
        self.choice_symbol(
            b,
            radio,
            mark,
            clip_with(layer.clip, c.bounds),
            paint_options(&layer.properties).ink,
        );
    }
    pub(super) fn choice_symbol(
        &mut self,
        b: Bounds,
        radio: bool,
        mark: Mark,
        clip: Option<Bounds>,
        color: Color32,
    ) {
        let color = [color.r(), color.g(), color.b()];
        let outline = if radio {
            (0..=32)
                .map(|i| {
                    let a = i as f32 * std::f32::consts::TAU / 32.;
                    [
                        b.x + b.w / 2. + a.cos() * b.w / 2.,
                        b.y + b.h / 2. + a.sin() * b.h / 2.,
                    ]
                })
                .collect()
        } else {
            vec![
                [b.x, b.y],
                [b.x + b.w, b.y],
                [b.x + b.w, b.y + b.h],
                [b.x, b.y + b.h],
                [b.x, b.y],
            ]
        };
        self.primitive(
            Primitive::Line {
                points: outline,
                color,
                width: 1.5,
            },
            clip,
        );
        match mark {
            Mark::Empty => {}
            Mark::Mixed => self.primitive(
                Primitive::Line {
                    points: vec![
                        [b.x + b.w * 0.25, b.y + b.h / 2.],
                        [b.x + b.w * 0.75, b.y + b.h / 2.],
                    ],
                    color,
                    width: 2.,
                },
                clip,
            ),
            Mark::Selected if radio => {
                // Filled center built from close horizontal strokes; no fixed-color legacy outline.
                let radius = b.w * 0.22;
                for i in -3..=3 {
                    let y = radius * i as f32 / 3.5;
                    let x = (radius * radius - y * y).sqrt();
                    self.primitive(
                        Primitive::Line {
                            points: vec![
                                [b.x + b.w / 2. - x, b.y + b.h / 2. + y],
                                [b.x + b.w / 2. + x, b.y + b.h / 2. + y],
                            ],
                            color,
                            width: 1.,
                        },
                        clip,
                    );
                }
            }
            Mark::Selected => self.primitive(
                Primitive::Line {
                    points: vec![
                        [b.x + b.w * 0.18, b.y + b.h * 0.5],
                        [b.x + b.w * 0.43, b.y + b.h * 0.75],
                        [b.x + b.w * 0.86, b.y + b.h * 0.2],
                    ],
                    color,
                    width: 2.,
                },
                clip,
            ),
        }
    }
}
