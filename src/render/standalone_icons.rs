// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
impl Scene {
    pub(super) fn standalone_icon(
        &mut self,
        layer: &Layer,
        fonts: &EguiMeasurer<'_>,
        rasters: &mut rasters::Rasters<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let p = &layer.properties;
        let b = c.bounds;
        let label = c.kind == Kind::IconLabel;
        let size = crate::icons::standalone_size(p, label);
        let icon = crate::icons::setting(p);
        let options = paint_options(p);
        let color = options.ink.to_array();
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let parsed = RichText::parse(&c.text, false);
        let text_size = fonts
            .rich_galley_with(&parsed, style, None, false, options)
            .size;
        let text_height = text_size.y.max(c.font_size);
        let position = match p.get("label_position") {
            Some(Value::Choice(s)) => s.as_str(),
            _ => "bottom",
        };
        let (ix, iy, tx, ty, tw, align) = if !label {
            (0., 0., 0., 0., 0., Alignment::Left)
        } else if position == "bottom" {
            ((b.w - size) / 2., 0., 0., size + 2., b.w, Alignment::Center)
        } else if position == "left" {
            (
                b.w - size,
                (b.h - size) / 2.,
                0.,
                (b.h - text_height) / 2.,
                text_size.x.min(b.w - size - 3.),
                Alignment::Left,
            )
        } else {
            (
                0.,
                (b.h - size) / 2.,
                size + 3.,
                (b.h - text_height) / 2.,
                b.w - size - 3.,
                Alignment::Left,
            )
        };
        // These controls fit their artwork horizontally rather than clipping it to their box.
        if label && tw > 0. {
            self.text(
                &parsed,
                style,
                TextBox {
                    bounds: Bounds::new(b.x + tx, b.y + ty, tw, text_height),
                    align,
                    middle: false,
                    wrap: false,
                    soft_breaks: false,
                    truncate: true,
                    clip: layer.clip,
                },
                fonts,
                options,
            )?;
        }
        let border = matches!(p.get("border"),Some(Value::Choice(s)) if s=="solid")
            && ix >= 0.
            && ix + size <= b.w;
        let padding = if border { 4. } else { 0. };
        let inner = size - padding * 2.;
        if ix >= 0. && ix + padding + inner <= b.w {
            let bounds = Bounds::new(b.x + ix + padding, b.y + iy + padding, inner, inner);
            if let Some(icon) = icon {
                self.items.push(Self::icon_item(
                    icon.source,
                    bounds,
                    color,
                    icon.turns,
                    layer.clip,
                    rasters,
                ));
            } else {
                self.icon_outline(bounds, color, layer.clip);
            }
        }
        if border {
            self.icon_outline(
                Bounds::new(b.x + ix, b.y + iy, size, size),
                color,
                layer.clip,
            );
        }
        Ok(())
    }
    /// Original rounded outline, scaled with the icon square.
    fn icon_outline(&mut self, b: Bounds, color: [u8; 4], clip: Option<Bounds>) {
        let stroke = b.w / 24.;
        let inset = stroke;
        let radius = b.w / 12.;
        let mut points = Vec::with_capacity(37);
        for (cx, cy, start) in [
            (b.x + b.w - inset - radius, b.y + inset + radius, -90.),
            (b.x + b.w - inset - radius, b.y + b.h - inset - radius, 0.),
            (b.x + inset + radius, b.y + b.h - inset - radius, 90.),
            (b.x + inset + radius, b.y + inset + radius, 180.),
        ] {
            for step in 0..=8 {
                let angle = (start + step as f32 * 90. / 8.).to_radians();
                points.push([cx + radius * angle.cos(), cy + radius * angle.sin()]);
            }
        }
        points.push(points[0]);
        self.primitive(
            Primitive::Line {
                points,
                color: [color[0], color[1], color[2]],
                width: stroke,
            },
            clip,
        );
    }
}
