// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::layout::TextMeasurer;
impl Scene {
    pub(super) fn list_control(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
        rasters: &mut rasters::Rasters<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let b = c.bounds;
        let p = &layer.properties;
        let list = crate::list::List::parse(&c.text)?;
        let header = boolean(p, "header");
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let has_icons = crate::list::has_icons(p);
        let m = list.measure(
            fonts,
            style,
            sketch,
            header,
            number(p, "row_height", 21.),
            has_icons,
        );
        let clip = clip_with(layer.clip, b);
        let fill = rgb(p, "fill", [255; 3]);
        let alternate = rgb(p, "alternate_fill", [255; 3]);
        let selected = number(p, "selected_index", -1.) as i32;
        let accent = if sketch {
            [149, 188, 242]
        } else {
            [151, 189, 240]
        };
        let row_color = |i: usize| {
            if selected >= 0 && i as i32 == selected {
                accent
            } else if i.is_multiple_of(2) {
                fill
            } else {
                alternate
            }
        };
        let mut link_bounds = Vec::new();
        if header {
            link_bounds.push(Bounds::new(b.x, b.y, b.w, m.header));
        }
        let start = if header { m.header } else { m.top };
        self.primitive(
            Primitive::Rect {
                b: Bounds::new(b.x, b.y, b.w, start),
                fill: if header { [204; 3] } else { row_color(0) },
                stroke: false,
                radius: 0.,
            },
            clip,
        );
        let mut y = b.y + start;
        let mut index = 0;
        while y < b.y + b.h {
            self.primitive(
                Primitive::Rect {
                    b: Bounds::new(b.x, y, b.w, m.row.min(b.y + b.h - y)),
                    fill: row_color(index),
                    stroke: false,
                    radius: 0.,
                },
                clip,
            );
            link_bounds.push(Bounds::new(b.x, y, b.w, m.row.min(b.y + b.h - y)));
            index += 1;
            y += m.row;
        }
        let ink = if sketch { [0; 3] } else { [51; 3] };
        let thickness = if sketch { 3. } else { 1. };
        if header {
            self.rectangle_frame(Bounds::new(b.x, b.y, b.w, m.header), ink, thickness, clip);
        }
        let border = !matches!(p.get("border"),Some(Value::Choice(s)) if s=="none");
        if border {
            self.rectangle_frame(b, ink, thickness, clip);
        }
        let mut icon_items = Vec::new();
        y = b.y + m.top;
        for (i, row) in list.rows.iter().enumerate() {
            let is_header = header && i == 0;
            let height = fonts.measure_rich(row, style, None, false).height;
            let text_y = y + if is_header { 3. } else { (m.row - height) / 2. };
            if text_y >= b.y + b.h {
                break;
            }
            let mut options = paint_options(p);
            if !is_header {
                let background = row_color(i - usize::from(header));
                let dark = u32::from(background[0]) * 299
                    + u32::from(background[1]) * 587
                    + u32::from(background[2]) * 114
                    < 153_000;
                if dark {
                    options.ink = Color32::WHITE;
                    options.link = Color32::WHITE;
                }
            }
            if let Some(Value::Icons(icons)) = p.get("row_icons")
                && let Some(Some(source)) = icons.get(i)
            {
                let bounds = Bounds::new(b.x + 8., text_y, c.font_size, c.font_size);
                let color = options.ink.to_array();
                icon_items.push(Self::icon_item(source, bounds, color, 0, clip, rasters));
            }
            let previous_links = self.links.len();
            self.text(
                row,
                style,
                TextBox {
                    bounds: Bounds::new(
                        b.x + if has_icons { c.font_size + 16. } else { 8. },
                        text_y,
                        (b.w - 8. - if sketch { 2. } else { 0. }).max(0.),
                        height,
                    ),
                    align: Alignment::Left,
                    middle: false,
                    wrap: false,
                    soft_breaks: false,
                    truncate: true,
                    clip,
                },
                fonts,
                options,
            )?;
            // List rows own links separately from inline text decoration.
            self.links.truncate(previous_links);
            y += if is_header { m.header - m.top } else { m.row };
        }
        if boolean(p, "vertical_scrollbar") && b.w >= 16. {
            let padding = if border && sketch { 2. } else { 0. };
            let top = if header { m.header - 1. } else { padding };
            self.vertical_scrollbar(
                Bounds::new(
                    b.x + b.w - padding - 16.,
                    b.y + top,
                    16.,
                    b.h - top - padding,
                ),
                number(p, "value", 0.),
                false,
                sketch,
                clip,
            );
        }
        self.items.extend(icon_items);
        if let Some(Value::Links(links)) = p.get("row_links") {
            for (bounds, target) in link_bounds.into_iter().zip(links) {
                if let Some(target) = target {
                    let bounds = clip.map_or(bounds, |c| intersect(bounds, c));
                    if bounds.w > 0. && bounds.h > 0. {
                        self.links.push(LinkRegion {
                            target: target.target(),
                            bounds,
                        });
                    }
                }
            }
        }
        Ok(())
    }
}
