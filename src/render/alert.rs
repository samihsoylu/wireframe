// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
impl Scene {
    pub(super) fn alert(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let p = &layer.properties;
        let b = c.bounds;
        let clip = clip_with(layer.clip, b);
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let alert = crate::alert::Alert::parse(&c.text)?;
        let metrics = alert.measure(fonts, style);
        let inline = matches!(p.get("button_style"),Some(Value::Choice(s))if s=="inline");
        let layout = alert.layout(&metrics, fonts, style, b.w, b.h, inline);
        let ink = if sketch { [0; 3] } else { [51; 3] };
        let separator = if sketch { [0; 3] } else { [204; 3] };
        let border = !matches!(p.get("border"),Some(Value::Choice(s))if s=="none");
        let radius: f32 = if sketch { 18. } else { 4. };
        self.primitive(
            Primitive::Rect {
                b,
                fill: [255; 3],
                stroke: false,
                radius,
            },
            clip,
        );
        if border {
            // Original circular-arc outline, shared across canvas and exports.
            let thickness = if sketch { 2. } else { 1. };
            let half = thickness / 2.;
            let frame = Bounds::new(
                b.x + half,
                b.y + half,
                (b.w - thickness).max(0.),
                (b.h - thickness).max(0.),
            );
            let r = radius.min(frame.w / 2.).min(frame.h / 2.);
            let mut points = Vec::new();
            for (cx, cy, start) in [
                (frame.x + r, frame.y + r, std::f32::consts::PI),
                (
                    frame.x + frame.w - r,
                    frame.y + r,
                    1.5 * std::f32::consts::PI,
                ),
                (frame.x + frame.w - r, frame.y + frame.h - r, 0.),
                (
                    frame.x + r,
                    frame.y + frame.h - r,
                    std::f32::consts::FRAC_PI_2,
                ),
            ] {
                for step in 0..=12 {
                    let a = start + step as f32 * std::f32::consts::FRAC_PI_2 / 12.;
                    points.push([cx + r * a.cos(), cy + r * a.sin()]);
                }
            }
            points.push(points[0]);
            self.primitive(
                Primitive::Line {
                    points,
                    color: ink,
                    width: thickness,
                },
                clip,
            );
        }
        if let Some(y) = layout.divider_y {
            self.primitive(
                Primitive::Line {
                    points: vec![[b.x + 1., b.y + y], [b.x + b.w - 1., b.y + y]],
                    color: separator,
                    width: if sketch { 2. } else { 1. },
                },
                clip,
            );
            if layout.split {
                self.primitive(
                    Primitive::Line {
                        points: vec![[b.x + b.w / 2., b.y + y], [b.x + b.w / 2., b.y + b.h]],
                        color: separator,
                        width: if sketch { 2. } else { 1. },
                    },
                    clip,
                );
            }
        }
        let align = match p.get("align") {
            Some(Value::Choice(s)) if s == "left" => Alignment::Left,
            Some(Value::Choice(s)) if s == "right" => Alignment::Right,
            _ => Alignment::Center,
        };
        let offset = |v: Bounds| Bounds::new(b.x + v.x, b.y + v.y, v.w, v.h);
        for (text, area, text_style, wrap) in [
            Some((
                &alert.title,
                layout.title,
                TextStyle {
                    size: style.size + 2.,
                    ..style
                },
                false,
            )),
            alert
                .message
                .as_ref()
                .map(|t| (t, layout.message, style, true)),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(area) = area
                && area.w > 0.
            {
                let mut options = paint_options(p);
                options.ink = Color32::from_rgb(ink[0], ink[1], ink[2]);
                options.link = Color32::from_rgb(8, 83, 148);
                let prior = self.links.len();
                self.text(
                    text,
                    text_style,
                    TextBox {
                        bounds: offset(area),
                        align,
                        middle: false,
                        wrap,
                        soft_breaks: true,
                        truncate: !wrap,
                        clip,
                    },
                    fonts,
                    options,
                )?;
                self.links.truncate(prior);
            }
        }
        for button in &layout.buttons {
            let mut options = paint_options(p);
            options.underline = inline;
            options.link = Color32::from_rgb(8, 83, 148);
            if button.text.w > 0. {
                let prior = self.links.len();
                self.text(
                    &alert.buttons[button.index].text,
                    style,
                    TextBox {
                        bounds: offset(button.text),
                        align: Alignment::Center,
                        middle: false,
                        wrap: false,
                        soft_breaks: true,
                        truncate: true,
                        clip,
                    },
                    fonts,
                    options,
                )?;
                self.links.truncate(prior);
            }
            if let Some(area) = button.link
                && let Some(Value::Links(links)) = p.get("row_links")
                && let Some(Some(link)) = links.get(button.index)
            {
                let area = offset(area);
                let area = clip.map_or(area, |c| intersect(area, c));
                if area.w > 0. && area.h > 0. {
                    self.links.push(LinkRegion {
                        hint: area,
                        target: link.target(),
                        bounds: area,
                    });
                }
            }
        }
        Ok(())
    }
}
