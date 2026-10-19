// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
impl Scene {
    pub(super) fn choice_group(
        &mut self,
        layer: &Layer,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let radio = c.kind == Kind::RadioGroup;
        let group = crate::choice_group::ChoiceGroup::parse(&c.text, radio)?;
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(&layer.properties, "italic"),
        };
        let m = group.measure(fonts, style);
        let b = c.bounds;
        let clip = clip_with(layer.clip, b);
        for (index, metrics) in m.visible_rows(b.h) {
            let row = &group.rows[index];
            let area = Bounds::new(b.x, b.y + metrics.y, b.w, metrics.height);
            let mut options = paint_options(&layer.properties);
            let disabled = Color32::from_rgb(128, 128, 139);
            if row.disabled {
                options.ink = disabled;
                options.link = disabled;
            }
            if let Some(mark) = row.mark {
                let color = if row.disabled {
                    disabled
                } else {
                    row.symbol_color
                        .as_deref()
                        .map(|value| {
                            crate::rich_text::painting::parse_color(value).unwrap_or(Color32::BLACK)
                        })
                        .unwrap_or(options.ink)
                };
                let size = if radio { 13. } else { 16. };
                self.choice_symbol(
                    Bounds::new(
                        area.x,
                        area.y + choice::center_offset(area.h, size),
                        size,
                        size,
                    ),
                    radio,
                    match mark {
                        crate::choice_group::Mark::Empty => choice::Mark::Empty,
                        crate::choice_group::Mark::Selected => choice::Mark::Selected,
                        crate::choice_group::Mark::Mixed => choice::Mark::Mixed,
                    },
                    clip,
                    color,
                );
            }
            let inset = if row.mark.is_some() {
                20_f32.min(area.w)
            } else {
                0.
            };
            let y = if row.mark.is_some() {
                (area.h - metrics.text_height) / 2. + 1.
            } else {
                0.
            };
            let links_before = self.links.len();
            self.text(
                &row.text,
                style,
                TextBox {
                    bounds: Bounds::new(
                        area.x + inset,
                        area.y + y,
                        (area.w - inset).max(0.),
                        metrics.text_height,
                    ),
                    align: Alignment::Left,
                    middle: false,
                    wrap: false,
                    soft_breaks: true,
                    truncate: true,
                    clip,
                },
                fonts,
                options,
            )?;
            // Row destinations own the full visible row, including disabled-looking rows.
            self.links.truncate(links_before);
            if let Some(Value::Links(links)) = layer.properties.get("row_links")
                && let Some(Some(link)) = links.get(index)
            {
                let bounds = clip.map_or(area, |c| intersect(area, c));
                if bounds.w > 0. && bounds.h > 0. {
                    self.links.push(LinkRegion {
                        quad: None,
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
