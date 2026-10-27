// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

fn translated(b: Bounds, origin: Bounds) -> Bounds {
    Bounds::new(b.x + origin.x, b.y + origin.y, b.w, b.h)
}
fn ink(sketch: bool) -> [u8; 3] {
    if sketch { [0; 3] } else { [51; 3] }
}
impl Scene {
    pub(super) fn date_rect(&mut self, b: Bounds, color: [u8; 3], clip: Option<Bounds>) {
        self.primitive(
            Primitive::Rect {
                b,
                fill: color,
                stroke: false,
                radius: 0.,
            },
            clip,
        );
    }
    pub(super) fn date_outline(
        &mut self,
        b: Bounds,
        color: [u8; 3],
        width: f32,
        clip: Option<Bounds>,
    ) {
        let i = (width / 2.).min(b.w / 2.).min(b.h / 2.);
        self.primitive(
            Primitive::Line {
                points: vec![
                    [b.x + i, b.y + i],
                    [b.x + b.w - i, b.y + i],
                    [b.x + b.w - i, b.y + b.h - i],
                    [b.x + i, b.y + b.h - i],
                    [b.x + i, b.y + i],
                ],
                color,
                width,
            },
            clip,
        );
    }
    pub(super) fn calendar_control(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let b = layer.control.bounds;
        let clip = clip_with(layer.clip, b);
        let layout = crate::date_controls::calendar(
            layer.date.unwrap_or_else(crate::date_controls::Date::today),
            b.w,
            b.h,
            layer.control.font_size,
            sketch,
            fonts,
        );
        let color = ink(sketch);
        self.date_rect(b, [255; 3], clip);
        self.date_outline(b, color, layout.border, clip);
        if let Some(circle) = layout.selection {
            let circle = translated(circle, b);
            let mut points = Vec::with_capacity(49);
            for n in 0..=48 {
                let a = n as f32 * std::f32::consts::TAU / 48.;
                points.push([
                    circle.x + circle.w / 2. + circle.w / 2. * a.cos(),
                    circle.y + circle.h / 2. + circle.h / 2. * a.sin(),
                ]);
            }
            self.primitive(
                Primitive::Polygon {
                    points: points.clone(),
                    fill: selection_color(&layer.properties),
                },
                clip,
            );
            self.primitive(
                Primitive::Line {
                    points,
                    color: [204; 3],
                    width: 1.,
                },
                clip,
            );
        }
        let style = TextStyle {
            size: layout.font_size,
            bold: false,
            italic: false,
        };
        for label in layout.weekdays.iter().chain(&layout.days) {
            self.text(
                &RichText::plain(&label.text),
                style,
                TextBox {
                    bounds: translated(label.bounds, b),
                    align: Alignment::Left,
                    middle: false,
                    wrap: false,
                    soft_breaks: false,
                    truncate: false,
                    clip,
                },
                fonts,
                PaintOptions {
                    ink: Color32::from_rgb(
                        if label.muted { 221 } else { color[0] },
                        if label.muted { 221 } else { color[1] },
                        if label.muted { 221 } else { color[2] },
                    ),
                    ..Default::default()
                },
            )?;
        }
        for rule in layout.rules {
            self.date_rect(translated(rule, b), color, clip);
        }
        if let Some(label) = layout.heading {
            self.text(
                &RichText::plain(&label.text),
                style,
                TextBox {
                    bounds: translated(label.bounds, b),
                    align: Alignment::Center,
                    middle: true,
                    wrap: false,
                    soft_breaks: false,
                    truncate: false,
                    clip,
                },
                fonts,
                PaintOptions {
                    ink: Color32::from_rgb(color[0], color[1], color[2]),
                    ..Default::default()
                },
            )?;
        }
        for (n, arrow) in layout.arrows.into_iter().enumerate() {
            let a = translated(arrow, b);
            let (tip, edge) = if n == 0 {
                (a.x, a.x + a.w)
            } else {
                (a.x + a.w, a.x)
            };
            self.primitive(
                Primitive::Polygon {
                    points: vec![[tip, a.y + a.h / 2.], [edge, a.y], [edge, a.y + a.h]],
                    fill: color,
                },
                clip,
            );
        }
        Ok(())
    }
    pub(super) fn date_chooser(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let b = c.bounds;
        let p = &layer.properties;
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let text = RichText::parse(&c.text, false);
        let layout = crate::date_controls::chooser(&text, style, Some(b.w), sketch, fonts);
        let disabled = matches!(p.get("state"),Some(Value::Choice(s)) if s=="disabled");
        let focused = matches!(p.get("state"),Some(Value::Choice(s)) if s=="focused");
        let icon_color = if disabled {
            if sketch { [128, 128, 139] } else { [207; 3] }
        } else {
            ink(sketch)
        };
        let border = if focused {
            selection_color(&layer.properties)
        } else if disabled {
            icon_color
        } else {
            rgb(p, "border_color", ink(sketch))
        };
        let clip = layer.clip;
        if let (Some(field), Some(area)) = (layout.field, layout.text) {
            let field = translated(field, b);
            self.date_rect(field, [255; 3], clip);
            self.date_outline(field, border, if sketch { 2. } else { 1. }, clip);
            let area = translated(area, b);
            let prior = self.links.len();
            self.text(
                &text,
                style,
                TextBox {
                    bounds: area,
                    align: Alignment::Left,
                    middle: false,
                    wrap: false,
                    soft_breaks: true,
                    truncate: true,
                    clip: clip_with(clip, area),
                },
                fonts,
                PaintOptions {
                    ink: if disabled {
                        Color32::from_rgb(128, 128, 139)
                    } else {
                        let i = ink(sketch);
                        Color32::from_rgb(i[0], i[1], i[2])
                    },
                    ..paint_options(p)
                },
            )?;
            self.links.truncate(prior);
        }
        self.items.push(Item {
            opacity: 1.,
            clip,
            drawing: Drawing::Icon {
                bounds: translated(layout.icon, b),
                id: "calendar-days-regular".into(),
                turns: 0,
                color: [icon_color[0], icon_color[1], icon_color[2], 255],
            },
        });
        Ok(())
    }
}
