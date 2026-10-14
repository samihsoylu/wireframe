// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Scene {
    pub(super) fn circle_button(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
        rasters: &mut rasters::Rasters<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let p = &layer.properties;
        let b = c.bounds;
        let diameter = b.w.min(b.h);
        let thickness = if sketch { 3. } else { 1. };
        let selection = [149, 188, 242];
        let normal = if sketch { [0; 3] } else { [51; 3] };
        let disabled = [128, 128, 139];
        let state = match p.get("state") {
            Some(Value::Choice(s)) => s.as_str(),
            _ => "normal",
        };
        let fill = if state == "selected" {
            selection
        } else {
            c.fill
        };
        let stroke = match state {
            "disabled" => disabled,
            "focused" => selection,
            _ => normal,
        };
        self.primitive(
            Primitive::StyledEllipse {
                b: Bounds::new(
                    b.x + (b.w - diameter + thickness) / 2.,
                    b.y + (b.h - diameter + thickness) / 2.,
                    (diameter - thickness).max(0.),
                    (diameter - thickness).max(0.),
                ),
                fill,
                stroke,
                width: if matches!(p.get("border"),Some(Value::Choice(s)) if s=="none") {
                    0.
                } else {
                    thickness
                },
            },
            layer.clip,
        );
        let brightness =
            u32::from(fill[0]) * 299 + u32::from(fill[1]) * 587 + u32::from(fill[2]) * 114;
        let ink = if state == "disabled" {
            disabled
        } else if brightness < 153_000 {
            [255; 3]
        } else {
            normal
        };
        let mut options = paint_options(p);
        options.ink = Color32::from_rgb(ink[0], ink[1], ink[2]);
        if state != "disabled" {
            options.link = if brightness < 153_000 {
                Color32::WHITE
            } else {
                Color32::from_rgb(8, 83, 148)
            };
        }
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let parsed = RichText::parse(&c.text, false);
        let measured = fonts
            .rich_galley_with(&parsed, style, None, true, options)
            .size;
        let icon = crate::icons::setting(p);
        let size = icon.as_ref().map_or(0., |i| i.size);
        let gap = if icon.is_some() { size + 2. } else { 0. };
        let position = match p.get("label_position") {
            Some(Value::Choice(s)) => s.as_str(),
            _ => "bottom",
        };
        let bottom = position == "bottom";
        let text_visible = !c.text.is_empty()
            && if bottom {
                measured.x < diameter && measured.y + gap < diameter
            } else {
                measured.y < diameter && measured.x + gap < diameter
            };
        let combined_w = measured.x + gap;
        let combined_h = measured.y + gap;
        if text_visible {
            let (x, y) = match position {
                "left" => ((b.w - combined_w) / 2., (b.h - measured.y) / 2.),
                "right" => ((b.w - measured.x + gap) / 2., (b.h - measured.y) / 2.),
                _ => ((b.w - measured.x) / 2., (b.h - combined_h) / 2. + size),
            };
            self.text(
                &parsed,
                style,
                TextBox {
                    bounds: Bounds::new(b.x + x, b.y + y, measured.x, measured.y),
                    align: Alignment::Left,
                    middle: false,
                    wrap: false,
                    soft_breaks: true,
                    truncate: false,
                    clip: layer.clip,
                },
                fonts,
                options,
            )?;
        }
        if let Some(icon) = icon
            && size < diameter
        {
            let (x, y) = if !text_visible {
                ((b.w - size) / 2., (b.h - size) / 2.)
            } else {
                match position {
                    "left" => ((b.w - size + measured.x + 2.) / 2., (b.h - size) / 2.),
                    "right" => ((b.w - combined_w) / 2., (b.h - size) / 2.),
                    _ => ((b.w - size) / 2., (b.h - combined_h) / 2.),
                }
            };
            self.items.push(Self::icon_item(
                icon.source,
                Bounds::new(b.x + x, b.y + y, size, size),
                options.ink.to_array(),
                icon.turns,
                layer.clip,
                rasters,
            ));
        }
        Ok(())
    }
}
