// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
impl Scene {
    fn diagram_line(
        &mut self,
        mut points: Vec<[f32; 2]>,
        color: [u8; 3],
        width: f32,
        clip: Option<Bounds>,
        closed: bool,
    ) {
        if closed && let Some(&p) = points.first() {
            points.push(p);
        }
        self.primitive(
            Primitive::Line {
                points,
                color,
                width,
            },
            clip,
        );
    }

    pub(super) fn diagram(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let p = &layer.properties;
        let b = c.bounds;
        let clip = clip_with(layer.clip, b);
        let offset = |r: Bounds| Bounds::new(b.x + r.x, b.y + r.y, r.w, r.h);
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let mut options = paint_options(p);
        options.ink = if sketch {
            Color32::BLACK
        } else {
            Color32::from_rgb(51, 51, 51)
        };
        options.link = color32(link_color(p));
        if c.kind == Kind::Popover {
            let side = crate::popover::Side::parse(match p.get("direction") {
                Some(Value::Choice(v)) => v,
                _ => "top",
            });
            let l = crate::popover::layout(
                b.w,
                b.h,
                side,
                number(p, "arrow_position", 2.) as u8,
                sketch,
            );
            let points: Vec<_> = l.outline.iter().map(|v| [v[0] + b.x, v[1] + b.y]).collect();
            self.primitive(
                Primitive::Mesh {
                    indices: crate::popover::triangles(&l.outline),
                    points: points.clone(),
                    fill: c.fill,
                },
                clip,
            );
            if l.draw_box {
                self.diagram_line(points, [0; 3], l.stroke, clip, true);
            } else if l.draw_arrow {
                self.diagram_line(
                    l.arrow_outline
                        .iter()
                        .map(|v| [v[0] + b.x, v[1] + b.y])
                        .collect(),
                    [0; 3],
                    l.stroke,
                    clip,
                    false,
                );
            }
            if !c.text.is_empty()
                && let Some(label) = l.label
            {
                if crate::field_set::light_text(c.fill, 1.) {
                    options.ink = Color32::WHITE;
                    options.link = Color32::WHITE;
                }
                self.text(
                    &RichText::parse(&c.text, false),
                    style,
                    TextBox {
                        bounds: offset(label),
                        align: if c.text.contains('\n') {
                            Alignment::Left
                        } else {
                            Alignment::Center
                        },
                        middle: false,
                        wrap: true,
                        soft_breaks: true,
                        truncate: false,
                        clip,
                    },
                    fonts,
                    options,
                )?;
            }
        } else {
            let tree = crate::site_map::Tree::parse(&c.text);
            let horizontal = matches!(p.get("direction"),Some(Value::Choice(v))if v=="horizontal");
            let l = tree.layout(fonts, style, horizontal);
            let ink = if sketch { [0; 3] } else { [51; 3] };
            for (i, r) in l.nodes.iter().enumerate() {
                let bounds = offset(*r);
                self.date_rect(bounds, [255; 3], clip);
                self.rectangle_frame(bounds, ink, if sketch { 2. } else { 1. }, clip);
                let error = RichText::parse(crate::site_map::ERROR, false);
                let text = if l.error {
                    &error
                } else {
                    &tree.nodes[i].label
                };
                let before = self.links.len();
                let mut opt = options;
                if l.error {
                    opt.ink = Color32::from_rgb(210, 30, 30);
                }
                self.text(
                    text,
                    style,
                    TextBox {
                        bounds: if l.error {
                            Bounds::new(
                                bounds.x + 10.,
                                bounds.y + 10.,
                                bounds.w - 20.,
                                bounds.h - 20.,
                            )
                        } else {
                            bounds
                        },
                        align: if l.error {
                            Alignment::Left
                        } else {
                            Alignment::Center
                        },
                        middle: !l.error,
                        wrap: false,
                        soft_breaks: true,
                        truncate: false,
                        clip,
                    },
                    fonts,
                    opt,
                )?;
                self.links.truncate(before);
                if !l.error
                    && let Some(Value::Links(links)) = p.get("row_links")
                    && let Some(Some(link)) = links.get(i)
                {
                    let r = offset(l.links[i]);
                    let bounds = clip.map_or(r, |v| intersect(r, v));
                    if bounds.w > 0. && bounds.h > 0. {
                        self.links.push(LinkRegion {
                            hint: bounds,
                            bounds,
                            target: link.target(),
                        });
                    }
                }
            }
            for edge in l.edges {
                self.diagram_line(
                    edge.into_iter().map(|v| [v[0] + b.x, v[1] + b.y]).collect(),
                    ink,
                    if sketch { 2. } else { 1. },
                    clip,
                    false,
                );
            }
        }
        Ok(())
    }
}
