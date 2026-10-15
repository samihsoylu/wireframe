// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::tabs::{Position, Tabs};
impl Scene {
    pub(super) fn tabs(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let p = &layer.properties;
        let b = c.bounds;
        let vertical = c.kind == Kind::VerticalTabs;
        let tabs = Tabs::parse(&c.text, vertical)?;
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let m = tabs.measure(fonts, style, sketch);
        let spacing = m.spacing;
        let position = Position::from_properties(p, vertical);
        let align = match p.get("align") {
            Some(Value::Choice(s)) => s.as_str(),
            _ => "left",
        };
        let placement = m.place(b, position, align);
        let clip = clip_with(layer.clip, b);
        let selected = number(p, "selected_index", -1.) as i32;
        let selected_area = usize::try_from(selected).ok().and_then(|i| {
            placement
                .tabs
                .get(i)
                .copied()
                .filter(|_| placement.visible[i])
        });
        let mask = selected_area
            .filter(|r| r.x >= b.x && r.x + r.w <= b.x + b.w && r.y >= b.y)
            .map(|r| {
                let (top, bottom, left, right) = match (position, sketch) {
                    (Position::Top, false) => (0., -1., 1., 1.),
                    (Position::Bottom, false) => (-1., 0., 1., 1.),
                    (Position::Left, false) => (1., 1., 0., -1.),
                    (Position::Right, false) => (1., 1., -1., 0.),
                    (Position::Top, true) => (0., -3., 2., 2.),
                    (Position::Bottom, true) => (-3., 0., 3., 6.),
                    (Position::Left, true) => (2., 2., 0., -1.),
                    (Position::Right, true) => (2., 2., -1., 0.),
                };
                Bounds::new(
                    r.x + left,
                    r.y + top,
                    (r.w - left - right).max(0.),
                    (r.h - top - bottom).max(0.),
                )
            });
        let ink = if sketch { [0; 3] } else { [51; 3] };
        let stroke = if sketch { 2. } else { 1. };
        let opacity = number(p, "background_opacity", 1.);
        // Paint selected tabs last, opening the body border at their join.
        for selected_pass in [false, true] {
            if selected_pass {
                let body = placement.content;
                if !matches!(p.get("border"),Some(Value::Choice(s)) if s=="none") {
                    let inset = spacing.fill_inset;
                    if body.w > inset * 2. && body.h > inset * 2. {
                        self.primitive(
                            Primitive::Rect {
                                b: Bounds::new(
                                    body.x + inset,
                                    body.y + inset,
                                    body.w - inset * 2.,
                                    body.h - inset * 2.,
                                ),
                                fill: c.fill,
                                stroke: false,
                                radius: 0.,
                            },
                            clip,
                        );
                        self.items.last_mut().unwrap().opacity = opacity;
                    }
                }
                if boolean(p, "vertical_scrollbar") {
                    self.vertical_scrollbar(
                        Bounds::new(
                            body.x + body.w - spacing.scroll[2] - 16.,
                            body.y + spacing.scroll[0],
                            16.,
                            body.h - spacing.scroll[0] - spacing.scroll[1],
                        ),
                        number(p, "value", 0.),
                        false,
                        sketch,
                        clip,
                    );
                }
                let border_clips = mask.map_or_else(
                    || vec![clip.unwrap_or(b)],
                    |hole| subtract(clip.unwrap_or(b), hole),
                );
                for area in border_clips {
                    if matches!(p.get("border"),Some(Value::Choice(s)) if s=="none") {
                        let points = match position {
                            Position::Top => vec![
                                [body.x, body.y + spacing.join],
                                [body.x + body.w, body.y + spacing.join],
                            ],
                            Position::Bottom => vec![
                                [body.x, body.y + body.h - spacing.join],
                                [body.x + body.w, body.y + body.h - spacing.join],
                            ],
                            Position::Left => vec![
                                [body.x + spacing.join, body.y],
                                [body.x + spacing.join, body.y + body.h],
                            ],
                            Position::Right => vec![
                                [body.x + body.w, body.y],
                                [body.x + body.w, body.y + body.h],
                            ],
                        };
                        self.primitive(
                            Primitive::Line {
                                points,
                                color: ink,
                                width: stroke,
                            },
                            Some(area),
                        );
                    } else if body.w > 0. && body.h > 0. {
                        self.rectangle_frame(body, ink, stroke, Some(area));
                    }
                }
            }
            for (i, area) in placement.tabs.iter().copied().enumerate() {
                let chosen = i as i32 == selected;
                if chosen != selected_pass || !placement.visible[i] {
                    continue;
                }
                let clips = if sketch && !vertical && !chosen {
                    mask.map_or_else(
                        || vec![clip.unwrap_or(b)],
                        |hole| subtract(clip.unwrap_or(b), hole),
                    )
                } else {
                    vec![clip.unwrap_or(b)]
                };
                for tab_clip in clips {
                    let points = tab_outline(area, position, sketch, stroke / 2.);
                    self.primitive(
                        Primitive::Polygon {
                            points: points.clone(),
                            fill: if chosen {
                                c.fill
                            } else if sketch {
                                [204; 3]
                            } else {
                                [221; 3]
                            },
                        },
                        Some(tab_clip),
                    );
                    self.items.last_mut().unwrap().opacity = if chosen { opacity } else { 1. };
                    let mut outline = points;
                    if !chosen {
                        outline.push(outline[0]);
                    }
                    self.primitive(
                        Primitive::Line {
                            points: outline,
                            color: ink,
                            width: stroke,
                        },
                        Some(tab_clip),
                    );
                    let mut options = paint_options(p);
                    let dark = u32::from(c.fill[0]) * 299
                        + u32::from(c.fill[1]) * 587
                        + u32::from(c.fill[2]) * 114
                        < 153_000;
                    let white = chosen && dark && opacity >= 0.75;
                    options.ink = if white {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(ink[0], ink[1], ink[2])
                    };
                    options.link = if white {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(8, 83, 148)
                    };
                    let offset = if sketch && chosen {
                        match position {
                            Position::Top => 2.,
                            Position::Bottom => -2.,
                            _ => 0.,
                        }
                    } else {
                        0.
                    };
                    let links = self.links.len();
                    self.text(
                        &tabs.texts[i],
                        style,
                        TextBox {
                            bounds: Bounds::new(
                                area.x + spacing.left,
                                area.y + spacing.top + offset,
                                m.text_sizes[i].0.max(1.),
                                m.text_sizes[i].1,
                            ),
                            align: Alignment::Left,
                            middle: false,
                            wrap: false,
                            soft_breaks: true,
                            truncate: false,
                            clip: Some(tab_clip),
                        },
                        fonts,
                        options,
                    )?;
                    self.links.truncate(links);
                }
                if let Some(Value::Links(links)) = p.get("row_links")
                    && let Some(Some(link)) = links.get(i)
                {
                    let hit = Bounds::new(
                        area.x,
                        area.y,
                        (area.w - spacing.overlap).min(b.w).max(0.),
                        area.h.min(b.h),
                    );
                    let hit = intersect(hit, clip.unwrap_or(b));
                    if hit.w > 0. && hit.h > 0. {
                        self.links.push(LinkRegion {
                            hint: hit,
                            target: link.target(),
                            bounds: hit,
                        });
                    }
                }
            }
        }
        Ok(())
    }
}
/// Original outlines ordered so the last-to-first edge joins the content panel.
fn tab_outline(b: Bounds, position: Position, sketch: bool, inset: f32) -> Vec<[f32; 2]> {
    let x = b.x + inset;
    let y = b.y + inset;
    let r = b.x + b.w - inset;
    let d = b.y + b.h - inset;
    let slant = if sketch && !position.vertical() {
        6_f32.min(b.w / 3.)
    } else {
        0.
    };
    match position {
        Position::Top => vec![[x, d], [x, y], [r - slant, y], [r, d]],
        Position::Bottom => vec![[r, y], [r - slant, d], [x, d], [x, y]],
        Position::Left => vec![[r, d], [x, d], [x, y], [r, y]],
        Position::Right => vec![[x, y], [r, y], [r, d], [x, d]],
    }
}
