// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::dropdown::{Mark, Menu};

impl Scene {
    fn dropdown_frame(&mut self, b: Bounds, color: [u8; 3], width: f32, clip: Option<Bounds>) {
        let inset = (width / 2.).min(b.w / 2.).min(b.h / 2.);
        self.primitive(
            Primitive::Line {
                points: vec![
                    [b.x + inset, b.y + inset],
                    [b.x + b.w - inset, b.y + inset],
                    [b.x + b.w - inset, b.y + b.h - inset],
                    [b.x + inset, b.y + b.h - inset],
                    [b.x + inset, b.y + inset],
                ],
                color,
                width,
            },
            clip,
        );
    }
    pub(super) fn dropdown_control(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let b = c.bounds;
        let clip = clip_with(layer.clip, b);
        let selection = number(&layer.properties, "selected_index", 0.).max(0.) as usize;
        let menu = Menu::parse(&c.text, selection)?;
        let state = match layer.properties.get("state") {
            Some(Value::Choice(s)) => s.as_str(),
            _ => "normal",
        };
        let closed = state.ends_with("closed");
        let disabled = state.starts_with("disabled");
        let focused = state.starts_with("focused");
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(&layer.properties, "italic"),
        };
        let metrics = menu.measure(fonts, style, sketch, closed);
        let ink = if disabled {
            [128, 128, 139]
        } else {
            rgb(&layer.properties, "text_color", [0, 0, 0])
        };
        let border = if disabled {
            if sketch { [137; 3] } else { [204; 3] }
        } else {
            rgb(&layer.properties, "border_color", [0, 0, 0])
        };
        let accent = if sketch {
            [149, 188, 242]
        } else {
            [151, 189, 240]
        };
        let left = if sketch { 6. } else { 4. };
        let arrow_width = if sketch { 22. } else { 20. };
        let thickness = if sketch { 3. } else { 1. };
        let header = Bounds::new(b.x, b.y, b.w, metrics.header);
        self.primitive(
            Primitive::Rect {
                b,
                fill: if disabled && !sketch {
                    [221; 3]
                } else {
                    [255; 3]
                },
                stroke: false,
                radius: 0.,
            },
            clip,
        );
        self.text(
            &RichText::parse(&menu.title, false),
            style,
            TextBox {
                bounds: Bounds::new(b.x + left, b.y, (b.w - arrow_width).max(0.), metrics.header),
                align: Alignment::Left,
                middle: true,
                wrap: false,
                soft_breaks: false,
                truncate: true,
                clip,
            },
            fonts,
            paint_options(&layer.properties),
        )?;
        self.primitive(
            Primitive::Rect {
                b: Bounds::new(
                    b.x + (b.w - 20.).max(0.),
                    b.y + 1.,
                    b.w.min(19.),
                    (metrics.header - 2.).max(0.),
                ),
                fill: if sketch { [255; 3] } else { [221; 3] },
                stroke: false,
                radius: 0.,
            },
            clip,
        );
        self.dropdown_frame(
            header,
            if focused { accent } else { border },
            thickness,
            clip,
        );
        self.primitive(
            Primitive::Line {
                points: vec![
                    [b.x + b.w - 19., b.y],
                    [b.x + b.w - 19., b.y + metrics.header],
                ],
                color: if focused { accent } else { border },
                width: thickness,
            },
            clip,
        );
        let arrow_color = if disabled {
            [128, 128, 139]
        } else {
            rgb(&layer.properties, "border_color", [0, 0, 0])
        };
        let arrow_y = b.y + (metrics.header - 7.) / 2.;
        self.primitive(
            Primitive::Polygon {
                points: vec![
                    [b.x + b.w - 15., arrow_y],
                    [b.x + b.w - 5., arrow_y],
                    [b.x + b.w - 10., arrow_y + 7.],
                ],
                fill: arrow_color,
            },
            clip,
        );
        if closed || menu.rows.is_empty() {
            return Ok(());
        }
        let icon_left = if sketch { 5. } else { 1. };
        let right = icon_left;
        let mut y = b.y + metrics.header + if sketch { 4. } else { 2. };
        for (row, height) in menu.rows.iter().zip(metrics.rows.iter().copied()) {
            if row.separator {
                self.primitive(
                    Primitive::Line {
                        points: vec![[b.x + icon_left, y], [b.x + b.w - right, y]],
                        color: ink,
                        width: 1.,
                    },
                    clip,
                );
                y += height;
                continue;
            }
            if row.selected {
                self.primitive(
                    Primitive::Rect {
                        b: Bounds::new(
                            b.x + icon_left,
                            y,
                            (b.w - icon_left - right).max(0.),
                            height,
                        ),
                        fill: accent,
                        stroke: false,
                        radius: 0.,
                    },
                    clip,
                );
            }
            let label_left = if menu.has_marks { 20. } else { left };
            let mut options = paint_options(&layer.properties);
            if row.selected {
                options.link = Color32::BLACK;
            }
            self.text(
                &RichText::parse(&row.text, false),
                style,
                TextBox {
                    bounds: Bounds::new(
                        b.x + label_left,
                        y,
                        (b.w - 25. - if row.arrow { 12. } else { 0. }).max(0.),
                        height,
                    ),
                    align: Alignment::Left,
                    middle: true,
                    wrap: false,
                    soft_breaks: false,
                    truncate: true,
                    clip,
                },
                fonts,
                options,
            )?;
            let mark_ink = if row.selected { [0; 3] } else { ink };
            let mx = b.x + icon_left;
            match row.mark {
                Mark::Dot => self.primitive(
                    Primitive::Rect {
                        b: Bounds::new(mx + 2., y + 5., 7., 7.),
                        fill: mark_ink,
                        stroke: false,
                        radius: 3.5,
                    },
                    clip,
                ),
                Mark::Check => self.primitive(
                    Primitive::Line {
                        points: vec![[mx, y + 9.], [mx + 4., y + 12.], [mx + 11., y + 4.]],
                        color: mark_ink,
                        width: 1.6,
                    },
                    clip,
                ),
                Mark::None => {}
            }
            if row.arrow {
                let x = b.x + b.w - 12.;
                self.primitive(
                    Primitive::Line {
                        points: vec![[x, y + 4.], [x + 5., y + 8.], [x, y + 12.]],
                        color: mark_ink,
                        width: 1.5,
                    },
                    clip,
                );
            }
            y += height;
        }
        self.dropdown_frame(
            Bounds::new(
                b.x,
                b.y + metrics.header - 1.,
                b.w,
                (b.h - metrics.header - 1.).max(1.),
            ),
            border,
            thickness,
            clip,
        );
        Ok(())
    }
}
