// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
impl Scene {
    pub(super) fn accordion(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let p = &layer.properties;
        let b = c.bounds;
        let clip = clip_with(layer.clip, b);
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let parsed =
            crate::accordion::Accordion::parse(&c.text, number(p, "selected_index", 0.) as i32)?;
        let metrics = parsed.measure(fonts, style, sketch);
        let layout = parsed.layout(&metrics, b.w, b.h, sketch, boolean(p, "vertical_scrollbar"));
        let a = crate::accordion::Appearance::new(sketch);
        let ink = if sketch { [0; 3] } else { [51; 3] };
        let thickness = if sketch { 3. } else { 1. };
        self.primitive(
            Primitive::Rect {
                b,
                fill: [255; 3],
                stroke: false,
                radius: 0.,
            },
            clip,
        );
        self.rectangle_frame(b, ink, thickness, clip);
        for placement in layout.rows.iter().rev() {
            let row = &parsed.rows[placement.index];
            let local = placement.bounds;
            let area = Bounds::new(b.x + local.x, b.y + local.y, local.w, local.h);
            let fill = if row.selected {
                selection_color(p)
            } else {
                [255; 3]
            };
            if row.parent.is_none() {
                self.primitive(
                    Primitive::Rect {
                        b: area,
                        fill,
                        stroke: false,
                        radius: 0.,
                    },
                    clip,
                );
                self.rectangle_frame(area, ink, thickness, clip);
            } else if row.selected {
                let selected = Bounds::new(
                    area.x + 10.,
                    area.y + a.overlap,
                    (area.w - 10. - a.right).max(0.),
                    (area.h - 2. * a.overlap).max(0.),
                );
                if selected.w > 0. && selected.h > 0. {
                    self.primitive(
                        Primitive::Rect {
                            b: selected,
                            fill,
                            stroke: false,
                            radius: 0.,
                        },
                        clip,
                    );
                }
            }
            let inset = a.left + if row.parent.is_some() { 10. } else { 0. };
            if area.w > inset && area.h > 0. {
                let mut options = background_text_options(p, fill, 1., ink, false);
                options.disabled_ink = Color32::from_rgb(128, 128, 139);
                let prior = self.links.len();
                self.text(
                    &row.text,
                    style,
                    TextBox {
                        bounds: Bounds::new(area.x + inset, area.y, area.w - inset, area.h),
                        align: Alignment::Left,
                        middle: true,
                        wrap: false,
                        soft_breaks: true,
                        truncate: true,
                        clip,
                    },
                    fonts,
                    options,
                )?;
                self.links.truncate(prior);
            }
        }
        // Link order follows the source, independently of reverse paint order.
        if let Some(Value::Links(links)) = p.get("row_links") {
            for placement in &layout.rows {
                if let Some(Some(link)) = links.get(placement.index) {
                    let local = placement.link;
                    let area = Bounds::new(b.x + local.x, b.y + local.y, local.w, local.h);
                    let area = clip.map_or(area, |c| intersect(area, c));
                    if area.w > 0. && area.h > 0. {
                        self.links.push(LinkRegion {
                            quad: None,
                            hint: area,
                            target: link.target(),
                            bounds: area,
                        });
                    }
                }
            }
        }
        if let Some(scroll) = layout.scrollbar {
            self.panel_scrollbar(
                Bounds::new(b.x + scroll.x, b.y + scroll.y, scroll.w, scroll.h),
                number(p, "value", 0.),
                false,
                sketch,
                clip,
            );
        }
        Ok(())
    }
}
