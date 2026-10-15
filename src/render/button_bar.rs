// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
impl Scene {
    pub(super) fn button_bar(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let p = &layer.properties;
        let b = c.bounds;
        let bar = crate::button_bar::ButtonBar::parse(&c.text)?;
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let metrics = bar.measure(fonts, style, sketch);
        let columns = metrics.visible_columns(b.w);
        let width = columns.iter().map(|(_, w)| w).sum::<f32>();
        let clip = clip_with(layer.clip, b);
        let normal = if sketch { [0; 3] } else { [51; 3] };
        let selected = number(p, "selected_index", 0.) as i32;
        if width > 2. && b.h > 2. {
            self.primitive(
                Primitive::Rect {
                    b: Bounds::new(b.x + 1., b.y + 1., width - 2., b.h - 2.),
                    fill: if sketch { [255; 3] } else { [221; 3] },
                    stroke: false,
                    radius: 0.,
                },
                clip,
            );
        }
        if selected >= 0
            && let Some(&(x, w)) = columns.get(selected as usize)
        {
            let left = if selected == 0 { 1. } else { 0. };
            let right = if selected as usize + 1 == columns.len() {
                if sketch { 4. } else { 1. }
            } else {
                0.
            };
            let bottom = if sketch { 2. } else { 1. };
            if w > left + right && b.h > 1. + bottom {
                self.primitive(
                    Primitive::Rect {
                        b: Bounds::new(
                            b.x + x + left,
                            b.y + 1.,
                            w - left - right,
                            b.h - 1. - bottom,
                        ),
                        fill: selection_color(p),
                        stroke: false,
                        radius: 0.,
                    },
                    clip,
                );
            }
        }
        let separator = if sketch { 2. } else { 1. };
        for (x, w) in columns.iter().take(columns.len().saturating_sub(1)) {
            let x = b.x + x + w - separator / 2.;
            self.primitive(
                Primitive::Line {
                    points: vec![[x, b.y + 1.], [x, b.y + b.h - 1.]],
                    color: normal,
                    width: separator,
                },
                clip,
            );
        }
        self.rectangle_frame(
            Bounds::new(b.x, b.y, width, b.h),
            normal,
            if sketch { 3. } else { 1. },
            clip,
        );
        for (index, (&(x, w), text)) in columns.iter().zip(&bar.texts).enumerate() {
            let area = Bounds::new(b.x + x, b.y, w, b.h);
            let background = if selected == index as i32 {
                selection_color(p)
            } else if sketch {
                [255; 3]
            } else {
                [221; 3]
            };
            let options = background_text_options(p, background, 1., normal, false);
            let previous = self.links.len();
            self.text(
                text,
                style,
                TextBox {
                    bounds: area,
                    align: Alignment::Center,
                    middle: true,
                    wrap: false,
                    soft_breaks: true,
                    truncate: false,
                    clip,
                },
                fonts,
                options,
            )?;
            // Segment links own the whole area; inline markup is decoration only.
            self.links.truncate(previous);
            if let Some(Value::Links(links)) = p.get("row_links")
                && let Some(Some(link)) = links.get(index)
            {
                let bounds = clip.map_or(area, |c| intersect(area, c));
                if bounds.w > 0. && bounds.h > 0. {
                    self.links.push(LinkRegion {
                        hint: bounds,
                        target: link.target(),
                        bounds,
                    });
                }
            }
        }
        Ok(())
    }
}
