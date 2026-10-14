// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::layout::TextMeasurer;
impl Scene {
    pub(super) fn field_set(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let b = c.bounds;
        let p = &layer.properties;
        let clip = clip_with(layer.clip, b);
        let text = crate::field_set::parse(&c.text)?;
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let metrics = fonts.measure_rich(&text, style, None, true);
        let layout = crate::field_set::layout(b.w, b.h, metrics);
        let offset = |r: Bounds| Bounds::new(b.x + r.x, b.y + r.y, r.w, r.h);
        let opacity = number(p, "background_opacity", 1.);
        self.primitive(
            Primitive::Rect {
                b,
                fill: c.fill,
                stroke: false,
                radius: 0.,
            },
            clip,
        );
        self.items.last_mut().expect("Field set fill").opacity = opacity;
        let ink = if sketch { [0; 3] } else { [51; 3] };
        // Disjoint clips keep the border hole transparent in every renderer.
        for visible in subtract(clip.unwrap_or(b), offset(layout.cutout)) {
            self.rectangle_frame(
                offset(layout.border),
                ink,
                if sketch { 3. } else { 1. },
                Some(visible),
            );
        }
        let mut paint = paint_options(p);
        paint.ink = Color32::from_rgb(ink[0], ink[1], ink[2]);
        if crate::field_set::light_text(c.fill, opacity) {
            paint.ink = Color32::WHITE;
            paint.link = Color32::WHITE;
        }
        self.text(
            &text,
            style,
            TextBox {
                bounds: offset(layout.label),
                align: Alignment::Left,
                middle: false,
                wrap: false,
                soft_breaks: true,
                truncate: false,
                clip,
            },
            fonts,
            paint,
        )?;
        Ok(())
    }
}
