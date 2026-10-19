// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Scene {
    pub(super) fn tree(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let p = &layer.properties;
        let b = c.bounds;
        let clip = clip_with(layer.clip, b);
        let tree = crate::tree::Tree::parse(&c.text)?;
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let metrics = tree.measure(fonts, style);
        let border = !matches!(p.get("border"), Some(Value::Choice(s)) if s=="none");
        let disabled = matches!(p.get("state"), Some(Value::Choice(s)) if s=="disabled");
        let scrollbar = boolean(p, "vertical_scrollbar");
        let opacity = number(p, "background_opacity", 1.);
        let selected = number(p, "selected_index", -1.) as i32;
        let normal = if sketch { [0; 3] } else { [51; 3] };
        let gray = [128, 128, 139];
        self.primitive(
            Primitive::Rect {
                b,
                fill: c.fill,
                stroke: false,
                radius: 0.,
            },
            clip,
        );
        self.items.last_mut().unwrap().opacity = opacity;
        for (index, (row, m)) in tree.rows.iter().zip(&metrics.rows).enumerate() {
            let y = b.y + if border { 6. } else { 0. } + m.y;
            let x = b.x + if border { 8. } else { 0. } + row.depth as f32 * 25.;
            let background = if selected == index as i32 {
                selection_color(p)
            } else {
                c.fill
            };
            if selected == index as i32 {
                self.primitive(
                    Primitive::Rect {
                        b: Bounds::new(b.x, y, b.w, m.height),
                        fill: background,
                        stroke: false,
                        radius: 0.,
                    },
                    clip,
                );
            }
            let dark = opacity >= 0.75
                && u32::from(background[0]) * 299
                    + u32::from(background[1]) * 587
                    + u32::from(background[2]) * 114
                    < 153_000;
            let ink = if disabled {
                gray
            } else if dark {
                [255; 3]
            } else {
                normal
            };
            let mut options = background_text_options(p, background, opacity, normal, false);
            options.disabled_ink = Color32::from_rgb(gray[0], gray[1], gray[2]);
            let prior = self.links.len();
            self.text(
                &row.text,
                style,
                TextBox {
                    bounds: Bounds::new(x + 26., y + 6., m.text_width, m.text_height),
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
            if let Some(id) = crate::tree::icon(row.marker.as_deref()) {
                self.items.push(Item {
                    drawing: Drawing::Icon {
                        bounds: Bounds::new(x, y + (m.height - 20.) / 2. + 1., 20., 20.),
                        id: id.into(),
                        turns: 0,
                        color: [ink[0], ink[1], ink[2], 255],
                    },
                    clip,
                    opacity: 1.,
                });
            }
            if let Some(Value::Links(links)) = p.get("row_links")
                && let Some(Some(link)) = links.get(index)
            {
                let bounds = Bounds::new(
                    b.x,
                    y,
                    (b.w - if scrollbar { 16. } else { 0. }).max(0.),
                    m.height,
                );
                let bounds = clip.map_or(bounds, |c| intersect(bounds, c));
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
        if scrollbar {
            let (top, bottom, right) = if sketch { (4., 2., 2.) } else { (0., 0., 0.) };
            self.panel_scrollbar(
                Bounds::new(b.x + b.w - right - 16., b.y + top, 16., b.h - top - bottom),
                number(p, "value", 0.),
                disabled,
                sketch,
                clip,
            );
        }
        if border {
            self.rectangle_frame(
                b,
                if disabled { gray } else { normal },
                if sketch { 3. } else { 1. },
                clip,
            );
        }
        Ok(())
    }
}
