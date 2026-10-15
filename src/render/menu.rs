// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
impl Scene {
    pub(super) fn menu(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
        rasters: &mut rasters::Rasters<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let p = &layer.properties;
        let b = c.bounds;
        let bar = c.kind == Kind::MenuBar;
        let clip = clip_with(layer.clip, b);
        let normal = if sketch { [0; 3] } else { [51; 3] };
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let selected = number(p, "selected_index", -1.) as i32;
        self.primitive(
            Primitive::Rect {
                b,
                fill: [255; 3],
                stroke: false,
                radius: 0.,
            },
            clip,
        );
        if !bar || !matches!(p.get("border"), Some(Value::Choice(s)) if s == "none") {
            self.rectangle_frame(b, normal, if sketch { 3. } else { 1. }, clip);
        }
        let options_for = |index: usize| {
            let is_selected = selected == index as i32;
            let mut options = background_text_options(
                p,
                if is_selected {
                    selection_color(p)
                } else {
                    [255; 3]
                },
                1.,
                if is_selected && !bar { [0; 3] } else { normal },
                is_selected && !bar,
            );
            options.disabled_ink = Color32::from_rgb(128, 128, 139);
            options
        };
        if bar {
            let parsed = crate::menu::MenuBar::parse(&c.text)?;
            let icon = crate::icons::setting(p);
            let size = icon.as_ref().map(|i| i.size);
            let metrics = parsed.measure(fonts, style, sketch, size);
            if let Some(icon) = icon {
                self.items.push(Self::icon_item(
                    icon.source,
                    Bounds::new(
                        b.x + 18.,
                        b.y + (b.h - icon.size) / 2.,
                        icon.size,
                        icon.size,
                    ),
                    [normal[0], normal[1], normal[2], 255],
                    icon.turns,
                    clip,
                    rasters,
                ));
            }
            let source_count = crate::menu::labels(&c.text, true).len();
            for item in metrics.items(b.w, size) {
                let area = Bounds::new(b.x + item.x, b.y, item.width, b.h);
                if selected == item.index as i32 {
                    self.menu_selection(area, clip, selection_color(p));
                }
                if !item.visible {
                    continue;
                }
                let prior = self.links.len();
                self.text(
                    &parsed.texts[item.index],
                    style,
                    TextBox {
                        bounds: Bounds::new(
                            area.x + 9.,
                            b.y + 2.,
                            (item.width - 18.).max(0.),
                            (b.h - 3.).max(0.),
                        ),
                        align: Alignment::Left,
                        middle: true,
                        wrap: false,
                        soft_breaks: true,
                        truncate: false,
                        clip,
                    },
                    fonts,
                    options_for(item.index),
                )?;
                self.links.truncate(prior);
                // Visible entries retain their own source destinations, even when earlier labels do not fit.
                if item.index < source_count {
                    self.menu_link(
                        p,
                        item.index,
                        Bounds::new(area.x, b.y + 2., area.w, (b.h - 3.).max(0.)),
                        clip,
                    );
                }
            }
        } else {
            let parsed = crate::menu::Menu::parse(&c.text)?;
            let metrics = parsed.measure(fonts, style, sketch);
            let inset = if sketch { 5. } else { 1. };
            for (index, (row, m)) in parsed.rows.iter().zip(&metrics.rows).enumerate() {
                let area =
                    Bounds::new(b.x + inset, b.y + m.y, (b.w - 2. * inset).max(0.), m.height);
                if selected == index as i32 {
                    self.menu_selection(area, clip, selection_color(p));
                }
                if row.separator {
                    self.primitive(
                        Primitive::Line {
                            points: vec![
                                [area.x, area.y + area.h / 2.],
                                [area.x + area.w, area.y + area.h / 2.],
                            ],
                            color: [0; 3],
                            width: if sketch { 2. } else { 1. },
                        },
                        clip,
                    );
                } else {
                    let available = (b.w - 25.).max(0.);
                    let right = if row.arrow {
                        12.
                    } else {
                        m.shortcut_width.min(available)
                    };
                    let prior = self.links.len();
                    if let Some(shortcut) = &row.shortcut {
                        self.text(
                            shortcut,
                            style,
                            TextBox {
                                bounds: Bounds::new(b.x + 20., area.y, available, area.h),
                                align: Alignment::Right,
                                middle: true,
                                wrap: false,
                                soft_breaks: true,
                                truncate: true,
                                clip,
                            },
                            fonts,
                            options_for(index),
                        )?;
                    }
                    self.text(
                        &row.label,
                        style,
                        TextBox {
                            bounds: Bounds::new(
                                b.x + 20.,
                                area.y,
                                (available - right).max(0.),
                                area.h,
                            ),
                            align: Alignment::Left,
                            middle: true,
                            wrap: false,
                            soft_breaks: true,
                            truncate: true,
                            clip,
                        },
                        fonts,
                        options_for(index),
                    )?;
                    self.links.truncate(prior);
                    if let Some(mark) = row.mark {
                        self.menu_symbol(
                            match mark {
                                crate::menu::Mark::Check => "check-solid",
                                crate::menu::Mark::Dot => "circle-solid",
                            },
                            if mark == crate::menu::Mark::Dot {
                                Bounds::new(b.x + inset + 2., area.y + 5., 8., 8.)
                            } else {
                                Bounds::new(b.x + inset, area.y + 3., 12., 12.)
                            },
                            clip,
                            if selected == index as i32 {
                                contrast(selection_color(p), [0; 3])
                            } else {
                                [0; 3]
                            },
                        );
                    }
                    if row.arrow {
                        self.menu_symbol(
                            "angle-right-solid",
                            Bounds::new(b.x + b.w - 16., area.y + 3., 12., 12.),
                            clip,
                            if selected == index as i32 {
                                contrast(selection_color(p), [0; 3])
                            } else {
                                [0; 3]
                            },
                        );
                    }
                }
                self.menu_link(p, index, Bounds::new(b.x, area.y, b.w, area.h), clip);
            }
        }
        Ok(())
    }
    fn menu_selection(&mut self, b: Bounds, clip: Option<Bounds>, color: [u8; 3]) {
        if b.w > 0. && b.h > 0. {
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
    }
    fn menu_symbol(&mut self, id: &str, bounds: Bounds, clip: Option<Bounds>, color: [u8; 3]) {
        self.items.push(Item {
            drawing: Drawing::Icon {
                bounds,
                id: id.into(),
                turns: 0,
                color: [color[0], color[1], color[2], 255],
            },
            clip,
            opacity: 1.,
        });
    }
    pub(super) fn menu_link(
        &mut self,
        properties: &Properties,
        index: usize,
        area: Bounds,
        clip: Option<Bounds>,
    ) {
        if let Some(Value::Links(links)) = properties.get("row_links")
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
}
