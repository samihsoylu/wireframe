// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Scene {
    pub(super) fn multiline_button(
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
            let bounds = clip.unwrap_or(b);
            if bounds.w > 0. && bounds.h > 0. {
                self.links.push(LinkRegion {
                    target: link.target(),
                    bounds,
                });
            }
        }
        let opacity = number(p, "background_opacity", 1.);
        let radius = if sketch { 26_f32 } else { 6_f32 };
        self.primitive(
            Primitive::Rect {
                b,
                fill: c.fill,
                stroke: false,
                radius,
            },
            clip,
        );
        self.items.last_mut().expect("Button fill").opacity = opacity;
        let stroke = if sketch { 2.5_f32 } else { 1_f32 };
        let inset = (stroke / 2.).min(b.w / 2.).min(b.h / 2.);
        let radius = radius
            .min((b.w / 2. - inset).max(0.))
            .min((b.h / 2. - inset).max(0.));
        // Original rounded outline sampled independently of fonts and artwork.
        let mut points = Vec::with_capacity(37);
        for (x, y, start) in [
            (b.x + b.w - inset - radius, b.y + inset + radius, -90.),
            (b.x + b.w - inset - radius, b.y + b.h - inset - radius, 0.),
            (b.x + inset + radius, b.y + b.h - inset - radius, 90.),
            (b.x + inset + radius, b.y + inset + radius, 180.),
        ] {
            for i in 0..=8 {
                let a = (start + i as f32 * 90. / 8.).to_radians();
                points.push([x + radius * a.cos(), y + radius * a.sin()]);
            }
        }
        points.push(points[0]);
        let normal = if sketch { [0; 3] } else { [51; 3] };
        self.primitive(
            Primitive::Line {
                points,
                color: normal,
                width: stroke,
            },
            clip,
        );
        let brightness =
            u32::from(c.fill[0]) * 299 + u32::from(c.fill[1]) * 587 + u32::from(c.fill[2]) * 114;
        let white = brightness < 153_000 && opacity >= 0.75;
        let ink = if white { [255; 3] } else { normal };
        let mut options = paint_options(p);
        options.ink = Color32::from_rgb(ink[0], ink[1], ink[2]);
        options.link = if white {
            Color32::WHITE
        } else {
            Color32::from_rgb(8, 83, 148)
        };
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let (title, secondary, second_style) = crate::layout::multiline_text(&c.text, style);
        let first = fonts
            .rich_galley_with(&title, style, None, false, options)
            .size;
        let secondary_width = secondary.as_ref().map_or(0., |s| {
            fonts
                .rich_galley_with(s, second_style, None, true, options)
                .size
                .x
        });
        let natural = first.x.max(secondary_width);
        let padding = if sketch { 14. } else { 5. };
        let available = (b.w - padding * 2.).max(0.);
        let icon = crate::icons::setting(p);
        let size = icon.as_ref().map_or(0., |i| i.size);
        let combined = (natural + if icon.is_some() { size + 3. } else { 0. }).min(available);
        let group_x = padding + (available - combined) / 2.;
        let icon_visible = icon.is_some()
            && b.h >= size
            && b.w >= size
            && (c.text.is_empty() || b.w >= size + natural + padding * 2. + 3.);
        if icon_visible && let Some(icon) = &icon {
            let x = if c.text.is_empty() && available < size {
                (b.w - size) / 2.
            } else {
                group_x
            };
            self.items.push(Self::icon_item(
                icon.source,
                Bounds::new(b.x + x, b.y + (b.h - size) / 2., size, size),
                options.ink.to_array(),
                icon.turns,
                clip,
                rasters,
            ));
        }
        let offset = if icon_visible { size + 3. } else { 0. };
        let text_width = (available - offset).min(natural);
        if text_width > 0. && first.y <= b.h {
            let second_height = secondary.as_ref().map_or(0., |s| {
                fonts
                    .rich_galley_with(s, second_style, Some(text_width), true, options)
                    .size
                    .y
            });
            let total = first.y
                + if secondary.is_some() {
                    8. + second_height
                } else {
                    0.
                };
            let y = b.y + (b.h - total) / 2.;
            let x = b.x + group_x + offset;
            let align = if icon.is_some() {
                Alignment::Left
            } else {
                Alignment::Center
            };
            self.text(
                &title,
                style,
                TextBox {
                    bounds: Bounds::new(x, y, text_width, first.y),
                    align,
                    middle: false,
                    wrap: false,
                    soft_breaks: false,
                    truncate: true,
                    clip,
                },
                fonts,
                options,
            )?;
            if let Some(secondary) = secondary {
                self.text(
                    &secondary,
                    second_style,
                    TextBox {
                        bounds: Bounds::new(x, y + first.y + 8., text_width, second_height),
                        align,
                        middle: false,
                        wrap: true,
                        soft_breaks: true,
                        truncate: false,
                        clip,
                    },
                    fonts,
                    options,
                )?;
            }
        }
        Ok(())
    }
}
