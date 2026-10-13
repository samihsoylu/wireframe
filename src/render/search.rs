// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

// Original vector outlines, shared by canvas and exports.
fn rounded_outline(b: Bounds, radius: f32) -> Vec<[f32; 2]> {
    let r = radius.min(b.w / 2.).min(b.h / 2.).max(0.);
    let mut points = Vec::with_capacity(37);
    for (cx, cy, start) in [
        (b.x + b.w - r, b.y + r, -90.),
        (b.x + b.w - r, b.y + b.h - r, 0.),
        (b.x + r, b.y + b.h - r, 90.),
        (b.x + r, b.y + r, 180.),
    ] {
        for step in 0..=8 {
            let angle = (start + step as f32 * 90. / 8.).to_radians();
            points.push([cx + r * angle.cos(), cy + r * angle.sin()]);
        }
    }
    points.push(points[0]);
    points
}

impl Scene {
    pub(super) fn search_control(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let b = c.bounds;
        let clip = clip_with(layer.clip, b);
        let disabled =
            matches!(layer.properties.get("state"), Some(Value::Choice(s)) if s == "disabled");
        let focused =
            matches!(layer.properties.get("state"), Some(Value::Choice(s)) if s == "focused");
        let square =
            matches!(layer.properties.get("shape"), Some(Value::Choice(s)) if s == "rectangle");
        let ink = if disabled { [128, 128, 139] } else { [0, 0, 0] };
        let border = if focused {
            if sketch {
                [149, 188, 242]
            } else {
                [151, 189, 240]
            }
        } else {
            ink
        };
        let radius = if square { 0. } else { b.h / 2. };
        let stroke = if sketch { 2. } else { 1. };
        self.primitive(
            Primitive::Rect {
                b: Bounds::new(b.x + 2., b.y + 2., (b.w - 4.).max(0.), (b.h - 4.).max(0.)),
                fill: [255; 3],
                stroke: false,
                radius: if square { 0. } else { 10. },
            },
            clip,
        );
        let search_icon = boolean(&layer.properties, "search_icon");
        let left = if search_icon { 22. } else { 6. };
        let placement = crate::layout::text_placement("search", sketch);
        let parsed = RichText::parse(&c.text, false);
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(&layer.properties, "italic"),
        };
        let options = paint_options(&layer.properties);
        let measured = fonts
            .rich_galley_with(&parsed, style, None, false, options)
            .size;
        self.text(
            &parsed,
            style,
            TextBox {
                bounds: Bounds::new(
                    b.x + left,
                    b.y + placement.top,
                    b.w,
                    (b.h - placement.top).max(1.),
                ),
                align: Alignment::Left,
                middle: false,
                wrap: false,
                soft_breaks: false,
                truncate: false,
                clip,
            },
            fonts,
            options,
        )?;
        let icon_y = b.y + (b.h - 12.) / 2.;
        if search_icon {
            let x = b.x + 6.;
            self.primitive(
                Primitive::Line {
                    points: rounded_outline(Bounds::new(x + 0.8, icon_y + 0.8, 7.5, 7.5), 3.75),
                    color: ink,
                    width: 1.6,
                },
                clip,
            );
            self.primitive(
                Primitive::Line {
                    points: vec![[x + 7., icon_y + 7.], [x + 11.5, icon_y + 11.5]],
                    color: ink,
                    width: 1.8,
                },
                clip,
            );
        }
        if boolean(&layer.properties, "microphone_icon") && b.w - left - measured.x > 18. {
            let x = b.x + b.w - 18.;
            self.primitive(
                Primitive::Rect {
                    b: Bounds::new(x + 4., icon_y, 4., 7.5),
                    fill: ink,
                    stroke: false,
                    radius: 2.,
                },
                clip,
            );
            self.primitive(
                Primitive::Line {
                    points: vec![
                        [x + 2., icon_y + 5.],
                        [x + 2., icon_y + 7.],
                        [x + 3., icon_y + 9.],
                        [x + 6., icon_y + 10.],
                        [x + 9., icon_y + 9.],
                        [x + 10., icon_y + 7.],
                        [x + 10., icon_y + 5.],
                    ],
                    color: ink,
                    width: 1.2,
                },
                clip,
            );
            self.primitive(
                Primitive::Line {
                    points: vec![
                        [x + 6., icon_y + 10.],
                        [x + 6., icon_y + 12.],
                        [x + 3., icon_y + 12.],
                        [x + 9., icon_y + 12.],
                    ],
                    color: ink,
                    width: 1.2,
                },
                clip,
            );
        }
        self.primitive(
            Primitive::Line {
                points: rounded_outline(
                    Bounds::new(
                        b.x + stroke / 2.,
                        b.y + stroke / 2.,
                        (b.w - stroke).max(0.),
                        (b.h - stroke).max(0.),
                    ),
                    radius,
                ),
                color: border,
                width: stroke,
            },
            clip,
        );
        Ok(())
    }
}
