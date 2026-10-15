// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
impl Scene {
    pub(super) fn navigation(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let p = &layer.properties;
        let b = c.bounds;
        let crumbs = c.kind == Kind::Breadcrumbs;
        let navigation = crate::navigation::Navigation::parse(&c.text, crumbs)?;
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let metrics = navigation.measure(fonts, style);
        let selected = number(p, "selected_index", -1.) as i32;
        let normal = if sketch { [0; 3] } else { [51; 3] };
        let separator = if crumbs {
            normal
        } else {
            rgb(p, "border_color", [0; 3])
        };
        let mut x = b.x;
        for (index, (text, &(width, height))) in
            navigation.texts.iter().zip(&metrics.items).enumerate()
        {
            let last = index + 1 == navigation.texts.len();
            let ink = if crumbs {
                if last { normal } else { link_color(p) }
            } else if selected == index as i32 {
                separator
            } else {
                rgb(p, "text_color", link_color(p))
            };
            let mut options = paint_options(p);
            options.ink = Color32::from_rgb(ink[0], ink[1], ink[2]);
            options.link = options.ink;
            options.underline = if crumbs {
                !last
            } else {
                selected != index as i32 && boolean(p, "underline")
            };
            let area = if crumbs {
                Bounds::new(x, b.y, width, b.h)
            } else {
                Bounds::new(x, b.y + 4., width, height)
            };
            let previous = self.links.len();
            self.text(
                text,
                style,
                TextBox {
                    bounds: area,
                    align: Alignment::Left,
                    middle: crumbs,
                    wrap: false,
                    soft_breaks: true,
                    truncate: false,
                    clip: layer.clip,
                },
                fonts,
                options,
            )?;
            // Item destinations own navigation; inline markup only decorates labels.
            self.links.truncate(previous);
            if let Some(Value::Links(links)) = p.get("row_links")
                && let Some(Some(link)) = links.get(index)
            {
                let hit = if crumbs {
                    Bounds::new(x, b.y, width + 8., b.h)
                } else {
                    area
                };
                let bounds = layer.clip.map_or(hit, |clip| intersect(hit, clip));
                if bounds.w > 0. && bounds.h > 0. {
                    self.links.push(LinkRegion {
                        hint: bounds,
                        target: link.target(),
                        bounds,
                    });
                }
            }
            if !last {
                if crumbs {
                    self.items.push(Item {
                        drawing: Drawing::Icon {
                            bounds: Bounds::new(x + width + 4., b.y + (b.h - 10.) / 2., 10., 10.),
                            id: "angle-right-solid".into(),
                            turns: 0,
                            color: [normal[0], normal[1], normal[2], 255],
                        },
                        opacity: 1.,
                        clip: layer.clip,
                    });
                } else {
                    self.primitive(
                        Primitive::Line {
                            points: vec![[x + width + 9., b.y], [x + width + 9., b.y + b.h]],
                            color: separator,
                            width: 2.,
                        },
                        layer.clip,
                    );
                }
            }
            x += width + if crumbs { 16. } else { 18. };
        }
        Ok(())
    }
}
