// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
impl Scene {
    pub(super) fn app_bar(
        &mut self,
        layer: &Layer,
        fonts: &EguiMeasurer<'_>,
        rasters: &mut rasters::Rasters<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let p = &layer.properties;
        let b = c.bounds;
        let clip = clip_with(layer.clip, b);
        let bar = crate::app_bar::AppBar::parse(&c.text, p)?;
        let metrics = bar.measure(fonts, c.font_size);
        let layout = bar.layout(&metrics, b.w, b.h);
        let fill = match p.get("fill") {
            Some(Value::Color(c)) => *c,
            _ => [221, 221, 221, 255],
        };
        let color = crate::app_bar::foreground(fill);
        self.primitive(
            Primitive::Rect {
                b,
                fill: [fill[0], fill[1], fill[2]],
                stroke: false,
                radius: 0.,
            },
            clip,
        );
        let offset = |r: Bounds| Bounds::new(b.x + r.x, b.y + r.y, r.w, r.h);
        // Stable source slots survive clipping and disabled built-in icons.
        // Paint order keeps navigation and custom artwork above the title.
        let mut slots = vec![None; bar.icons.len() + 3];
        for placement in &layout.icons {
            slots[placement.index] = Some(placement);
        }
        for index in [1, usize::MAX, bar.icons.len() + 2, 0]
            .into_iter()
            .chain(2..bar.icons.len() + 2)
        {
            if index == usize::MAX {
                if let Some(area) = layout.title
                    && area.w > 0.
                {
                    let mut options = paint_options(p);
                    options.ink =
                        Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
                    options.underline = false;
                    let prior = self.links.len();
                    self.text(
                        &bar.title,
                        crate::app_bar::text_style(c.font_size),
                        TextBox {
                            bounds: offset(area),
                            align: Alignment::Left,
                            middle: false,
                            wrap: false,
                            soft_breaks: true,
                            truncate: false,
                            clip,
                        },
                        fonts,
                        options,
                    )?;
                    self.links.truncate(prior);
                }
                continue;
            }
            let Some(placement) = slots[index] else {
                continue;
            };
            if let Some(icon) = bar.icon(index) {
                self.items.push(Self::icon_item(
                    &icon,
                    offset(placement.bounds),
                    color,
                    0,
                    clip,
                    rasters,
                ));
            }
            if let Some(Value::Links(links)) = p.get("row_links")
                && let Some(Some(link)) = links.get(index)
            {
                let area = offset(placement.link);
                let area = clip.map_or(area, |c| intersect(area, c));
                if area.w > 0. && area.h > 0. {
                    self.links.push(LinkRegion {
                        target: link.target(),
                        bounds: area,
                    });
                }
            }
        }
        Ok(())
    }
}
