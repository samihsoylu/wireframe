// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::document::IconSource;
impl Scene {
    pub(super) fn icon_item(
        source: &IconSource,
        bounds: Bounds,
        color: [u8; 4],
        turns: u8,
        clip: Option<Bounds>,
        rasters: &mut rasters::Rasters<'_>,
    ) -> Item {
        let drawing = match source {
            IconSource::Catalog(id) => Drawing::Icon {
                bounds,
                id: id.clone(),
                color,
                turns,
            },
            IconSource::Asset(id) => {
                if let Some(data) = rasters.get(
                    *id,
                    rasters::Style::Icon([color[0], color[1], color[2]], turns),
                ) {
                    Drawing::Image { bounds, data }
                } else {
                    Drawing::Primitive(Primitive::Line {
                        points: vec![
                            [bounds.x, bounds.y],
                            [bounds.x + bounds.w, bounds.y],
                            [bounds.x + bounds.w, bounds.y + bounds.h],
                            [bounds.x, bounds.y + bounds.h],
                            [bounds.x, bounds.y],
                            [bounds.x + bounds.w, bounds.y + bounds.h],
                            [bounds.x + bounds.w, bounds.y],
                            [bounds.x, bounds.y + bounds.h],
                        ],
                        color: [color[0], color[1], color[2]],
                        width: 1.,
                    })
                }
            }
        };
        Item {
            drawing,
            clip,
            opacity: if matches!(source, IconSource::Asset(_)) {
                color[3] as f32 / 255.
            } else {
                1.
            },
        }
    }
    pub(super) fn button_control(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
        rasters: &mut rasters::Rasters<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let b = c.bounds;
        let p = &layer.properties;
        self.form_control(layer, sketch);
        let clip = clip_with(layer.clip, b);
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let parsed = RichText::parse(&c.text, false);
        let mut options = paint_options(p);
        forms::button_text_colors(layer, sketch, &mut options);
        let measured = fonts
            .rich_galley_with(&parsed, style, None, false, options)
            .size;
        let (left, right, gap) = if sketch { (3., 5., 4.) } else { (3., 3., 3.) };
        let menu = crate::icons::menu_size(boolean(p, "menu_icon"), c.font_size);
        let icon = crate::icons::setting(p);
        let size = icon.as_ref().map_or(0., |i| i.size);
        let available = (b.w - left - right - if menu > 0. { menu + gap } else { 0. }).max(0.);
        let combined = (measured.x + if icon.is_some() { size + gap } else { 0. }).min(available);
        let group_x = left + alignment(p).offset(available, combined);
        let visible = icon.is_some()
            && b.h >= size
            && (if c.text.is_empty() {
                b.w >= size
            } else {
                b.w >= size + measured.x + menu + left + right + gap
            })
            && (if menu > 0. {
                b.w - menu - 1.5 * gap >= size
            } else {
                b.w >= size
            });
        let color = options.ink.to_array();
        if visible && let Some(icon) = &icon {
            let x = if c.text.is_empty() && menu == 0. && b.w - left - right < size {
                (b.w - size) / 2.
            } else {
                group_x
            };
            self.items.push(Self::icon_item(
                icon.source,
                Bounds::new(b.x + x, b.y + (b.h - size) / 2., size, size),
                color,
                icon.turns,
                clip,
                rasters,
            ));
        }
        if menu > 0. && b.w > menu + 2. * gap + left {
            let x = b.x + b.w - 1.5 * gap - menu;
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
        let offset = if visible { size + gap } else { 0. };
        let width = (available - offset).min(measured.x);
        if width > 0. && measured.y <= b.h {
            self.text(
                &parsed,
                style,
                TextBox {
                    bounds: Bounds::new(b.x + group_x + offset, b.y, width, b.h),
                    align: if icon.is_some() {
                        Alignment::Left
                    } else {
                        alignment(p)
                    },
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
        Ok(())
    }
}
