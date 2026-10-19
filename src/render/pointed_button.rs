// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

// Original rounded polygon construction, shared by the fill and outline.
fn outline(b: Bounds, direction: &str, width: f32) -> Vec<[f32; 2]> {
    let inset = (width / 2.).min(b.w / 2.).min(b.h / 2.);
    let (x, y, w, h) = (
        b.x + inset,
        b.y + inset,
        (b.w - 2. * inset).max(0.),
        (b.h - 2. * inset).max(0.),
    );
    let tip = 18_f32.min(w / 2.);
    let corners = match direction {
        "left" => vec![
            [x + tip, y],
            [x + w, y],
            [x + w, y + h],
            [x + tip, y + h],
            [x, y + h / 2.],
        ],
        "right" => vec![
            [x, y],
            [x + w - tip, y],
            [x + w, y + h / 2.],
            [x + w - tip, y + h],
            [x, y + h],
        ],
        _ => vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]],
    };
    let mut result = Vec::new();
    for (i, &p) in corners.iter().enumerate() {
        let previous = corners[(i + corners.len() - 1) % corners.len()];
        let next = corners[(i + 1) % corners.len()];
        let approach = |q: [f32; 2]| {
            let length = (q[0] - p[0]).hypot(q[1] - p[1]);
            let ratio = if length > 0. {
                (4. / length).min(0.25)
            } else {
                0.
            };
            [p[0] + (q[0] - p[0]) * ratio, p[1] + (q[1] - p[1]) * ratio]
        };
        let a = approach(previous);
        let z = approach(next);
        for step in 0..=4 {
            let t = step as f32 / 4.;
            let u = 1. - t;
            result.push([
                u * u * a[0] + 2. * u * t * p[0] + t * t * z[0],
                u * u * a[1] + 2. * u * t * p[1] + t * t * z[1],
            ]);
        }
    }
    result.push(result[0]);
    result
}

impl Scene {
    pub(super) fn pointed_button(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
        rasters: &mut rasters::Rasters<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let p = &layer.properties;
        let b = c.bounds;
        let clip = clip_with(layer.clip, b);
        if let Some(Value::Link(link)) = p.get("link") {
            let visible = clip.unwrap_or(b);
            if visible.w > 0. && visible.h > 0. {
                self.links.push(LinkRegion {
                    quad: None,
                    hint: visible,
                    target: link.target(),
                    bounds: visible,
                });
            }
        }
        let direction = match p.get("direction") {
            Some(Value::Choice(s)) => s.as_str(),
            _ => "right",
        };
        let border = !matches!(p.get("border"),Some(Value::Choice(s)) if s=="none");
        if border {
            let width = if sketch { 2.5 } else { 1. };
            let points = outline(b, direction, width);
            self.primitive(
                Primitive::Polygon {
                    points: points.clone(),
                    fill: c.fill,
                },
                clip,
            );
            let disabled = matches!(p.get("state"),Some(Value::Choice(s)) if s=="disabled");
            let color = match (disabled, sketch) {
                (true, true) => [153; 3],
                (true, false) => [128, 128, 139],
                (false, true) => [0; 3],
                _ => [51; 3],
            };
            self.primitive(
                Primitive::Line {
                    points,
                    color,
                    width,
                },
                clip,
            );
        } else {
            self.primitive(
                Primitive::Rect {
                    b,
                    fill: c.fill,
                    stroke: false,
                    radius: if sketch { 8. } else { 5. },
                },
                clip,
            );
        }
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let mut parsed = RichText::parse(&c.text, false);
        // This family exposes one control link, never hidden inline hotspots.
        for item in &mut parsed.items {
            if let crate::rich_text::Inline::Span { style, .. } = item {
                style.link = false;
                style.target = None;
            }
        }
        let mut options = paint_options(p);
        forms::button_text_colors(layer, sketch, &mut options);
        let measured = fonts
            .rich_galley_with(&parsed, style, None, false, options)
            .size;
        let icon = crate::icons::setting(p);
        let size = icon.as_ref().map_or(0., |i| i.size);
        let gap = if icon.is_some() && measured.x > 0. {
            3.
        } else {
            0.
        };
        let menu = crate::icons::menu_size(boolean(p, "menu_icon"), c.font_size);
        let total = size + gap + measured.x.min(b.w) + if menu > 0. { menu + 3. } else { 0. };
        let (left, right) = if measured.x > b.w {
            (0., 0.)
        } else {
            match direction {
                "left" => (20., 10.),
                "right" => (10., 20.),
                _ => (10., 10.),
            }
        };
        let tx = (left + (b.w - left - right - total) / 2. + size + gap).max(0.);
        if measured.x > 0. {
            self.text(
                &parsed,
                style,
                TextBox {
                    bounds: Bounds::new(b.x + tx, b.y, measured.x.min(b.w), b.h),
                    align: Alignment::Center,
                    middle: true,
                    wrap: false,
                    soft_breaks: false,
                    truncate: true,
                    clip,
                },
                fonts,
                options,
            )?;
        }
        let color = options.ink.to_array();
        if let Some(icon) = icon {
            self.items.push(Self::icon_item(
                icon.source,
                Bounds::new(
                    b.x + (tx - gap - size).max(0.),
                    b.y + (b.h - size) / 2.,
                    size,
                    size,
                ),
                color,
                icon.turns,
                clip,
                rasters,
            ));
        }
        if menu > 0. {
            let x = b.x
                + if direction == "center" {
                    (b.w - right - menu).max(0.)
                } else {
                    b.w - right - menu
                };
            let y = b.y + (b.h - menu) / 2.;
            self.primitive(
                Primitive::Polygon {
                    points: vec![
                        [x, y + menu * 0.15],
                        [x + menu, y + menu * 0.15],
                        [x + menu / 2., y + menu * 0.85],
                    ],
                    fill: [color[0], color[1], color[2]],
                },
                clip,
            );
        }
        if !border && direction != "center" {
            let x = b.x + if direction == "left" { 3. } else { b.w - 15. };
            let y = b.y + (b.h - 12.) / 2.;
            let (tip, base) = if direction == "left" {
                (3., 8.)
            } else {
                (9., 4.)
            };
            self.primitive(
                Primitive::Line {
                    points: vec![[x + base, y + 2.], [x + tip, y + 6.], [x + base, y + 10.]],
                    color: [color[0], color[1], color[2]],
                    width: 2.5,
                },
                clip,
            );
        }
        Ok(())
    }
}
