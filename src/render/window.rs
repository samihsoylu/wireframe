// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::layout::TextMeasurer;
use crate::window::{Button, Options};
impl Scene {
    pub(super) fn window(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let b = c.bounds;
        let p = &layer.properties;
        let clip = clip_with(layer.clip, b);
        let title = crate::window::parse(&c.text)?;
        let text_style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let options = Options {
            header: number(p, "header_height", 26.),
            footer: number(p, "footer_height", 25.),
            minimize: boolean(p, "minimize"),
            maximize: boolean(p, "maximize"),
            close: boolean(p, "close"),
            resize_handle: boolean(p, "resize_handle"),
            scrollbar: boolean(p, "vertical_scrollbar"),
        };
        let layout = options.layout(
            b.w,
            b.h,
            fonts.measure_rich(&title, text_style, None, true).height,
            sketch,
        );
        let offset = |r: Bounds| Bounds::new(b.x + r.x, b.y + r.y, r.w, r.h);
        let ink = if sketch { [0; 3] } else { [51; 3] };
        let stroke = if sketch { 3. } else { 1. };
        self.primitive(
            Primitive::Rect {
                b,
                fill: [255; 3],
                stroke: false,
                radius: 0.,
            },
            clip,
        );
        for area in [
            Some(Bounds::new(0., 0., b.w, b.h)),
            Some(layout.header),
            layout.footer,
        ]
        .into_iter()
        .flatten()
        {
            self.rectangle_frame(offset(area), ink, stroke, clip);
        }
        if let Some(scroll) = layout.scrollbar {
            self.browser_scrollbar(offset(scroll), number(p, "value", 0.), sketch, clip);
        }
        let mut paint = paint_options(p);
        paint.ink = Color32::from_rgb(ink[0], ink[1], ink[2]);
        let links = self.links.len();
        self.text(
            &title,
            text_style,
            TextBox {
                bounds: offset(layout.title),
                align: Alignment::Left,
                middle: false,
                wrap: false,
                soft_breaks: true,
                truncate: layout.title.w > 0.,
                clip,
            },
            fonts,
            paint,
        )?;
        self.links.truncate(links);
        // Simple original vector chrome, independent of the icon library.
        for (kind, area) in layout.buttons {
            let r = offset(area);
            match kind {
                Button::Minimize => {
                    self.window_stroke(vec![[r.x, r.y + 11.], [r.x + 13., r.y + 11.]], 1.5, clip)
                }
                Button::Maximize => {
                    self.rectangle_frame(Bounds::new(r.x, r.y + 1., 13., 10.), [0; 3], 1.5, clip);
                    self.window_stroke(vec![[r.x, r.y + 3.], [r.x + 13., r.y + 3.]], 2., clip);
                }
                Button::Close => {
                    for (a, z) in [([3., 3.], [10., 10.]), ([10., 3.], [3., 10.])] {
                        self.window_stroke(
                            vec![[r.x + a[0], r.y + a[1]], [r.x + z[0], r.y + z[1]]],
                            1.6,
                            clip,
                        );
                    }
                }
            }
        }
        if let Some(area) = layout.resize_handle {
            let r = offset(area);
            for (a, z) in [([2., 10.], [11., 1.]), ([7., 10.], [12., 5.])] {
                self.window_stroke(
                    vec![[r.x + a[0], r.y + a[1]], [r.x + z[0], r.y + z[1]]],
                    if sketch { 2.5 } else { 1.5 },
                    clip,
                );
            }
        }
        Ok(())
    }
    fn window_stroke(&mut self, points: Vec<[f32; 2]>, width: f32, clip: Option<Bounds>) {
        self.primitive(
            Primitive::Line {
                points,
                color: [0; 3],
                width,
            },
            clip,
        );
    }
}
