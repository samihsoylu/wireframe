// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Scene {
    pub(super) fn shape_control(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
        rasters: &mut rasters::Rasters<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let p = &layer.properties;
        let b = c.bounds;
        let shape = match p.get("shape") {
            Some(Value::Choice(s)) => s.as_str(),
            _ => "ellipse",
        };
        let geometry =
            crate::shapes::geometry(shape, number(p, "shape_rotation", 0.), b.w, b.h, sketch);
        let clip = clip_with(layer.clip, b);
        let points: Vec<_> = geometry
            .outline
            .iter()
            .map(|v| [b.x + v[0], b.y + v[1]])
            .collect();
        let alpha = number(p, "background_opacity", 1.);
        self.items.push(Item {
            opacity: 1.,
            clip,
            drawing: Drawing::BeginOpacity(alpha),
        });
        self.primitive(
            Primitive::Mesh {
                indices: crate::popover::triangles(&points),
                points: points.clone(),
                fill: c.fill,
            },
            clip,
        );
        if geometry.thickness > 0. {
            let color = rgb(p, "border_color", if sketch { [0; 3] } else { [51; 3] });
            if geometry.dashed {
                self.primitive(
                    super::outlines::dash_mesh(&points, geometry.thickness, color),
                    clip,
                );
            } else {
                let mut closed = points;
                closed.push(closed[0]);
                self.primitive(
                    Primitive::Line {
                        points: closed,
                        color,
                        width: geometry.thickness,
                    },
                    clip,
                );
            }
        }
        self.items.push(Item {
            opacity: 1.,
            clip,
            drawing: Drawing::EndOpacity,
        });
        let masked = geometry.tear.is_some();
        if let Some((edge, keep_above)) = geometry.tear {
            self.items.push(Item {
                opacity: 1.,
                clip,
                drawing: Drawing::BeginMask(mask::BottomMask {
                    bounds: b,
                    keep_above,
                    edge: edge.into_iter().map(|v| [b.x + v[0], b.y + v[1]]).collect(),
                }),
            });
        }
        let options = background_text_options(
            p,
            c.fill,
            alpha,
            if sketch { [0; 3] } else { [51; 3] },
            false,
        );
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
        let icon_size = icon.as_ref().map_or(0., |i| i.size);
        let text_width = if c.text.is_empty() {
            0.
        } else {
            (b.w - 16. - icon_size).max(0.).min(measured.x)
        };
        let combined = text_width + if icon.is_some() { icon_size + 3. } else { 0. };
        let alignment = match p.get("align") {
            Some(Value::Choice(s)) if s == "left" => Alignment::Left,
            Some(Value::Choice(s)) if s == "right" => Alignment::Right,
            _ => Alignment::Center,
        };
        let mut x = match alignment {
            Alignment::Left => 8.,
            Alignment::Center => (b.w - combined) / 2.,
            Alignment::Right => b.w - 8. - combined,
        }
        .max(0.)
        .round();
        if let Some(icon) = icon {
            self.items.push(Self::icon_item(
                icon.source,
                Bounds::new(
                    b.x + x,
                    b.y + ((b.h - icon_size) / 2.).round(),
                    icon_size,
                    icon_size,
                ),
                options.ink.to_array(),
                icon.turns,
                clip,
                rasters,
            ));
            x += icon_size + 3.;
        }
        let has_icon = icon_size > 0.;
        let width = if !has_icon && c.text.encode_utf16().count() == 1 && text_width < b.w {
            x = 0.;
            b.w
        } else {
            text_width
        };
        if !c.text.is_empty() && measured.y < b.h && width > 0. {
            self.text(
                &parsed,
                style,
                TextBox {
                    bounds: Bounds::new(b.x + x, b.y, width, b.h),
                    align: if has_icon { Alignment::Left } else { alignment },
                    middle: true,
                    wrap: false,
                    soft_breaks: true,
                    truncate: false,
                    clip,
                },
                fonts,
                options,
            )?;
        }
        if masked {
            self.items.push(Item {
                opacity: 1.,
                clip,
                drawing: Drawing::EndMask,
            });
        }
        Ok(())
    }
}
