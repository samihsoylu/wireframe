// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::pickers::{Ink, Mode};
impl Scene {
    pub(super) fn picker(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let b = c.bounds;
        let p = &layer.properties;
        let mode = if c.kind == Kind::TimePicker {
            Mode::Time
        } else if matches!(p.get("date_mode"),Some(Value::Choice(s)) if s=="year") {
            Mode::Year
        } else {
            Mode::Month
        };
        let layout = crate::pickers::layout(
            crate::pickers::Options {
                mode,
                width: b.w,
                height: b.h,
                sketch,
                project_font: c.font_size,
            },
            layer.date.unwrap_or_else(crate::date_controls::Date::today),
            layer.clock.unwrap_or_else(crate::tablet::Clock::now),
            fonts,
        );
        let clip = clip_with(layer.clip, b);
        let ink = if sketch { [0; 3] } else { [51; 3] };
        let selected = if sketch {
            [149, 188, 242]
        } else {
            [151, 189, 240]
        };
        let offset = |a: Bounds| Bounds::new(a.x + b.x, a.y + b.y, a.w, a.h);
        self.date_rect(b, [255; 3], clip);
        self.date_outline(b, ink, layout.border, clip);
        if let Some(panel) = layout.heading {
            self.date_rect(offset(panel), [221; 3], clip);
        }
        for (circle, color) in [(layout.face, [238; 3]), (layout.selection, selected)] {
            if let Some(circle) = circle {
                if color == selected
                    && let Some(hand) = layout.hand
                {
                    self.primitive(
                        Primitive::Line {
                            points: hand.map(|[x, y]| [x + b.x, y + b.y]).to_vec(),
                            color: selected,
                            width: 3.,
                        },
                        clip,
                    );
                }
                let circle = offset(circle);
                let points = (0..=64)
                    .map(|i| {
                        let a = i as f32 * std::f32::consts::TAU / 64.;
                        [
                            circle.x + circle.w / 2. + circle.w / 2. * a.cos(),
                            circle.y + circle.h / 2. + circle.h / 2. * a.sin(),
                        ]
                    })
                    .collect();
                self.primitive(
                    Primitive::Polygon {
                        points,
                        fill: color,
                    },
                    clip,
                );
            }
        }
        for (i, a) in layout.arrows.iter().enumerate() {
            let a = offset(*a);
            let center = a.x + a.w / 2.;
            let edge = if i == 0 {
                center + a.w / 6.
            } else {
                center - a.w / 6.
            };
            let tip = if i == 0 {
                center - a.w / 6.
            } else {
                center + a.w / 6.
            };
            self.primitive(
                Primitive::Line {
                    points: vec![
                        [edge, a.y + a.h / 6.],
                        [tip, a.y + a.h / 2.],
                        [edge, a.y + a.h * 5. / 6.],
                    ],
                    color: ink,
                    width: (a.h / 12.).max(0.1),
                },
                clip,
            );
        }
        for label in layout.labels {
            let color = match label.ink {
                Ink::Normal => ink,
                Ink::Selected => {
                    let c =
                        crate::app_bar::foreground([selected[0], selected[1], selected[2], 255]);
                    [c[0], c[1], c[2]]
                }
                Ink::Active => [0; 3],
                Ink::Inactive => [153; 3],
                Ink::Link => [8, 83, 148],
            };
            let area = offset(label.bounds);
            if !label.text.is_empty() && label.size > 0. {
                let prior = self.links.len();
                self.text(
                    &RichText::plain(&label.text),
                    TextStyle {
                        size: label.size,
                        bold: label.bold,
                        italic: false,
                    },
                    TextBox {
                        bounds: area,
                        align: Alignment::Left,
                        middle: false,
                        wrap: false,
                        soft_breaks: true,
                        truncate: false,
                        clip,
                    },
                    fonts,
                    PaintOptions {
                        ink: Color32::from_rgb(color[0], color[1], color[2]),
                        underline: label.button.is_some(),
                        ..Default::default()
                    },
                )?;
                self.links.truncate(prior);
            }
            if let Some(index) = label.button
                && let Some(Value::Links(links)) = p.get("row_links")
                && let Some(Some(target)) = links.get(index)
            {
                let area = clip.map_or(area, |c| intersect(area, c));
                if area.w > 0. && area.h > 0. {
                    self.links.push(LinkRegion {
                        hint: area,
                        target: target.target(),
                        bounds: area,
                    });
                }
            }
        }
        Ok(())
    }
}
