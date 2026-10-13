// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::{Inline, RichText, Style};
use crate::layout::{EguiMeasurer, TextStyle};
use eframe::egui::{self, Color32, FontId, Stroke};
use std::{ops::Range, sync::Arc};
#[derive(Clone, Debug)]
pub struct LinkArea {
    pub target: String,
    pub rect: egui::Rect,
}
pub struct PaintedText {
    pub galley: Arc<egui::Galley>,
    pub links: Vec<LinkArea>,
}
impl EguiMeasurer<'_> {
    pub fn rich_galley(
        &self,
        text: &RichText,
        base: TextStyle,
        width: Option<f32>,
        soft_breaks: bool,
    ) -> PaintedText {
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = width.unwrap_or(f32::INFINITY);
        let mut links: Vec<(Range<usize>, String)> = vec![];
        for item in &text.items {
            let empty = Style::default();
            let (value, style) = match item {
                Inline::Span { text, style } => (text.as_str(), style),
                Inline::Break(super::Break::Soft) if !soft_breaks => continue,
                Inline::Break(_) => ("\n", &empty),
                Inline::Bullet => ("• ", &empty),
            };
            let size = style
                .size
                .filter(|s| (1..=512).contains(s))
                .map_or(base.size, |s| s as f32);
            let color = if style.disabled {
                Color32::from_gray(153)
            } else if let Some(color) = style.color.as_deref().and_then(parse_color) {
                color
            } else if style.link {
                Color32::from_rgb(0, 102, 204)
            } else {
                Color32::BLACK
            };
            let format = egui::text::TextFormat {
                font_id: FontId::new(
                    size,
                    if base.bold || style.bold {
                        self.bold.clone()
                    } else {
                        self.regular.clone()
                    },
                ),
                color,
                italics: base.italic || style.italic,
                underline: if style.underline || style.link {
                    Stroke::new(1., color)
                } else {
                    Stroke::NONE
                },
                strikethrough: if style.strike {
                    Stroke::new(1., color)
                } else {
                    Stroke::NONE
                },
                ..Default::default()
            };
            let start = job.text.len();
            job.append(value, 0., format);
            if let Some(target) = &style.target {
                links.push((start..job.text.len(), target.clone()));
            }
        }
        let galley = self.context.fonts_mut(|f| f.layout_job(job));
        let mut areas: Vec<LinkArea> = vec![];
        let mut byte = 0;
        let mut link_index = 0;
        for row in &galley.rows {
            let mut row_areas: Vec<(usize, egui::Rect)> = vec![];
            for glyph in &row.glyphs {
                while links
                    .get(link_index)
                    .is_some_and(|(range, _)| range.end <= byte)
                {
                    link_index += 1;
                }
                if links
                    .get(link_index)
                    .is_some_and(|(range, _)| range.contains(&byte))
                {
                    let index = link_index;
                    let rect = glyph.logical_rect().translate(row.pos.to_vec2());
                    if let Some((last, area)) = row_areas.last_mut()
                        && *last == index
                    {
                        *area = area.union(rect);
                    } else {
                        row_areas.push((index, rect));
                    }
                }
                byte += glyph.chr.len_utf8();
            }
            for (index, rect) in row_areas {
                areas.push(LinkArea {
                    target: links[index].1.clone(),
                    rect,
                });
            }
            if row.ends_with_newline {
                byte += 1;
            }
        }
        PaintedText {
            galley,
            links: areas,
        }
    }
}
fn parse_color(value: &str) -> Option<Color32> {
    let value = value.trim().to_ascii_lowercase();
    let hex = value.trim_start_matches('#');
    if hex.len() == 6 && hex.bytes().all(|c| c.is_ascii_hexdigit()) {
        let n = u32::from_str_radix(hex, 16).ok()?;
        return Some(Color32::from_rgb((n >> 16) as u8, (n >> 8) as u8, n as u8));
    }
    if hex.len() == 3 && hex.bytes().all(|c| c.is_ascii_hexdigit()) {
        let n = u16::from_str_radix(hex, 16).ok()?;
        return Some(Color32::from_rgb(
            ((n >> 8) & 15) as u8 * 17,
            ((n >> 4) & 15) as u8 * 17,
            (n & 15) as u8 * 17,
        ));
    }
    Some(match value.as_str() {
        "red" => Color32::RED,
        "green" => Color32::from_rgb(0, 128, 0),
        "blue" => Color32::BLUE,
        "black" => Color32::BLACK,
        "white" => Color32::WHITE,
        "gray" | "grey" => Color32::GRAY,
        "yellow" => Color32::YELLOW,
        "orange" => Color32::from_rgb(255, 165, 0),
        "purple" => Color32::from_rgb(128, 0, 128),
        _ => return None,
    })
}
