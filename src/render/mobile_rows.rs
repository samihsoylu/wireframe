// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::mobile_menu::{Mark, Menu, caption_style};

fn rounded(b: Bounds, rx: f32, ry: f32) -> Vec<[f32; 2]> {
    let rx = rx.min(b.w / 2.).max(0.);
    let ry = ry.min(b.h / 2.).max(0.);
    let mut points = Vec::new();
    for (cx, cy, start) in [
        (b.x + b.w - rx, b.y + ry, -90.),
        (b.x + b.w - rx, b.y + b.h - ry, 0.),
        (b.x + rx, b.y + b.h - ry, 90.),
        (b.x + rx, b.y + ry, 180.),
    ] {
        for i in 0..=12 {
            let a = (start + i as f32 * 7.5).to_radians();
            points.push([cx + rx * a.cos(), cy + ry * a.sin()]);
        }
    }
    if let Some(first) = points.first().copied() {
        points.push(first)
    }
    points
}
impl Scene {
    pub(super) fn mobile_menu(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let b = c.bounds;
        let p = &layer.properties;
        let clip = clip_with(layer.clip, b);
        let menu = Menu::parse(&c.text)?;
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let layout = menu.layout(fonts, style, Some(b.w));
        let offset = |r: Bounds| Bounds::new(b.x + r.x, b.y + r.y, r.w, r.h);
        let radius = if sketch { 30. } else { 7. };
        self.primitive(
            Primitive::Polygon {
                points: rounded(b, radius, radius),
                fill: [255; 3],
            },
            clip,
        );
        if !matches!(p.get("border"),Some(Value::Choice(s)) if s=="none") {
            let r = if sketch { 18. } else { 4. };
            self.primitive(
                Primitive::Line {
                    points: rounded(
                        Bounds::new(b.x + 0.5, b.y + 0.5, (b.w - 1.).max(0.), (b.h - 1.).max(0.)),
                        r,
                        r,
                    ),
                    color: if sketch { [0; 3] } else { [51; 3] },
                    width: if sketch { 1.5 } else { 1. },
                },
                clip,
            );
        }
        let normal = if sketch {
            Color32::BLACK
        } else {
            Color32::from_gray(51)
        };
        let options = PaintOptions {
            ink: normal,
            ..paint_options(p)
        };
        for (index, (row, r)) in menu.rows.iter().zip(layout.rows).enumerate() {
            if let (Some(mark), Some(bounds)) = (row.trailing, r.trailing) {
                self.mobile_mark(mark, offset(bounds), sketch, clip);
            }
            let prior = self.links.len();
            if let (Some(text), Some(bounds)) = (&row.caption, r.caption) {
                self.text(
                    text,
                    caption_style(style),
                    TextBox {
                        bounds: offset(bounds),
                        align: Alignment::Right,
                        middle: true,
                        wrap: false,
                        soft_breaks: true,
                        truncate: true,
                        clip,
                    },
                    fonts,
                    PaintOptions {
                        ink: normal,
                        underline: false,
                        ..Default::default()
                    },
                )?;
            }
            if let (Some(mark), Some(bounds)) = (row.leading, r.leading) {
                self.mobile_mark(mark, offset(bounds), sketch, clip);
            }
            if let (Some(text), Some(bounds)) = (&row.main, r.main) {
                self.text(
                    text,
                    style,
                    TextBox {
                        bounds: offset(bounds),
                        align: Alignment::Left,
                        middle: true,
                        wrap: false,
                        soft_breaks: true,
                        truncate: true,
                        clip,
                    },
                    fonts,
                    options,
                )?;
            }
            self.links.truncate(prior);
            if index > 0 {
                let y = b.y + r.bounds.y + 0.5;
                self.primitive(
                    Primitive::Line {
                        points: vec![[b.x + 1., y], [b.x + b.w - 1., y]],
                        color: [0; 3],
                        width: 1.,
                    },
                    clip,
                );
            }
            self.menu_link(p, index, offset(r.bounds), clip);
        }
        Ok(())
    }
    pub(super) fn mobile_picker(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let b = layer.control.bounds;
        let clip = clip_with(layer.clip, b);
        let picker = crate::mobile_picker::Picker::parse(&layer.control.text)?;
        let base = layer.control.font_size;
        let layout = picker.layout(fonts, base);
        self.primitive(
            Primitive::Rect {
                b,
                fill: [255; 3],
                stroke: false,
                radius: 0.,
            },
            clip,
        );
        for (row, cells) in picker.rows.iter().enumerate() {
            let ink = crate::mobile_picker::ink(row, sketch);
            let prior = self.links.len();
            for (cell, bounds) in cells.iter().zip(&layout.cells[row]) {
                if let Some(text) = cell {
                    self.text(
                        text,
                        crate::mobile_picker::style(row, base),
                        TextBox {
                            bounds: Bounds::new(b.x + bounds.x, b.y + bounds.y, bounds.w, bounds.h),
                            align: Alignment::Center,
                            middle: false,
                            wrap: false,
                            soft_breaks: true,
                            truncate: true,
                            clip,
                        },
                        fonts,
                        PaintOptions {
                            ink: Color32::from_rgb(ink[0], ink[1], ink[2]),
                            ..Default::default()
                        },
                    )?;
                }
            }
            self.links.truncate(prior);
        }
        for y in [68., 90.] {
            let points = if sketch {
                vec![
                    [b.x, b.y + y - 0.3],
                    [b.x + b.w * 0.22, b.y + y + 0.35],
                    [b.x + b.w * 0.73, b.y + y - 0.2],
                    [b.x + b.w, b.y + y + 0.35],
                ]
            } else {
                vec![[b.x, b.y + y + 0.5], [b.x + b.w, b.y + y + 0.5]]
            };
            self.primitive(
                Primitive::Line {
                    points,
                    color: if sketch { [0; 3] } else { [221; 3] },
                    width: if sketch { 1.5 } else { 1. },
                },
                clip,
            );
        }
        Ok(())
    }
    fn mobile_mark(&mut self, mark: Mark, b: Bounds, sketch: bool, clip: Option<Bounds>) {
        let line = |points, color, width| Primitive::Line {
            points,
            color,
            width,
        };
        let pt = |x: f32, y: f32| [b.x + x * b.w, b.y + y * b.h];
        if matches!(mark, Mark::On | Mark::Off) {
            let on = mark == Mark::On;
            let green = [106, 169, 79];
            let fill = if on {
                green
            } else if sketch {
                [226; 3]
            } else {
                [255; 3]
            };
            let ink = if sketch {
                [0; 3]
            } else if on {
                green
            } else {
                [153; 3]
            };
            let radius = b.h / 2.;
            self.primitive(
                Primitive::Polygon {
                    points: rounded(b, radius, radius),
                    fill,
                },
                clip,
            );
            self.primitive(
                line(
                    rounded(
                        Bounds::new(b.x + 1., b.y + 1., b.w - 2., b.h - 2.),
                        radius - 1.,
                        radius - 1.,
                    ),
                    ink,
                    2.,
                ),
                clip,
            );
            let knob = Bounds::new(
                if on { b.x + b.w - b.h + 2. } else { b.x + 2. },
                b.y + 2.,
                b.h - 4.,
                b.h - 4.,
            );
            self.primitive(
                Primitive::Polygon {
                    points: super::phone::circle(knob),
                    fill: [255; 3],
                },
                clip,
            );
            self.primitive(
                line(
                    super::phone::circle(knob),
                    ink,
                    if sketch { 2. } else { 1. },
                ),
                clip,
            );
            return;
        }
        if mark == Mark::Bullet {
            self.primitive(
                Primitive::Polygon {
                    points: super::phone::circle(Bounds::new(
                        b.x + 2.,
                        b.y + 2.,
                        b.w - 4.,
                        b.h - 4.,
                    )),
                    fill: [0, 0, 255],
                },
                clip,
            );
            return;
        }
        let color = match mark {
            Mark::Add => [0, 154, 15],
            Mark::Remove => [206, 42, 39],
            Mark::CircleChevron => [37, 117, 218],
            Mark::Grip => [153; 3],
            _ => [0; 3],
        };
        if matches!(mark, Mark::Add | Mark::Remove | Mark::CircleChevron) {
            self.primitive(
                line(
                    super::phone::circle(Bounds::new(b.x + 1., b.y + 1., b.w - 2., b.h - 2.)),
                    color,
                    1.2,
                ),
                clip,
            );
        }
        let mut paths = Vec::new();
        match mark {
            Mark::Add | Mark::Remove => {
                paths.push(vec![pt(0.28, 0.5), pt(0.72, 0.5)]);
                if mark == Mark::Add {
                    paths.push(vec![pt(0.5, 0.28), pt(0.5, 0.72)]);
                }
            }
            Mark::Check => paths.push(vec![pt(0.1, 0.48), pt(0.38, 0.76), pt(0.91, 0.2)]),
            Mark::Chevron => paths.push(vec![pt(0.35, 0.12), pt(0.72, 0.5), pt(0.35, 0.88)]),
            Mark::CircleChevron => paths.push(vec![pt(0.42, 0.28), pt(0.65, 0.5), pt(0.42, 0.72)]),
            Mark::Grip => {
                for y in [0.24, 0.5, 0.76] {
                    paths.push(vec![pt(0.1, y), pt(0.9, y)])
                }
            }
            _ => {}
        }
        for p in paths {
            self.primitive(
                line(
                    p,
                    color,
                    if matches!(mark, Mark::Check | Mark::Chevron | Mark::Grip) {
                        2.
                    } else {
                        1.2
                    },
                ),
                clip,
            );
        }
    }
}
