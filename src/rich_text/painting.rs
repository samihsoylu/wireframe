// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::{Inline, RichText, Style, source_lines};
use crate::layout::{EguiMeasurer, TextStyle};
use eframe::egui::{self, Color32, FontId, Stroke};
use std::{ops::Range, sync::Arc};
use unicode_segmentation::UnicodeSegmentation;
#[derive(Clone, Debug)]
pub struct LinkArea {
    pub target: String,
    pub rect: egui::Rect,
    pub row: usize,
}
#[derive(Clone, Copy)]
pub struct PaintOptions {
    pub ink: Color32,
    pub link: Color32,
    pub underline: bool,
    pub disabled_ink: Color32,
}
impl Default for PaintOptions {
    fn default() -> Self {
        Self {
            ink: Color32::BLACK,
            link: Color32::from_rgb(8, 83, 148),
            underline: false,
            disabled_ink: Color32::from_gray(153),
        }
    }
}
pub struct PaintedText {
    /// Font shaping supplies horizontal advances; canvas line boxes are independent.
    pub galley: Arc<egui::Galley>,
    pub rows: Vec<TextRow>,
    pub runs: Vec<Vec<TextRun>>,
    pub size: egui::Vec2,
    pub links: Vec<LinkArea>,
    pub error: Option<String>,
}
#[derive(Clone, Copy, Debug)]
pub struct TextRow {
    pub top: f32,
    pub height: f32,
    pub width: f32,
    pub baseline: f32,
}
/// A complete styled source run. Positioned ink can omit an overflowing trailing
/// separator while `text` retains the exact source bytes and formatting.
#[derive(Clone, Debug)]
pub struct TextRun {
    pub text: String,
    pub format: egui::text::TextFormat,
    pub x: f32,
    pub width: f32,
    pub face: String,
    pub face_scale: f32,
    pub positioned: Option<Arc<crate::text_engine::ShapedText>>,
}
impl TextRun {
    pub fn truncate(&mut self, context: &egui::Context, available: f32) -> Result<(), String> {
        if self.width <= available {
            return Ok(());
        }
        let measure = |text: &str| -> Result<f32, String> {
            if self.positioned.is_some() {
                return Ok(
                    crate::text_engine::shape_cached(context, &self.format.font_id, text)?
                        .ok_or("The selected canvas font is unavailable")?
                        .width as f32,
                );
            }
            let mut job = egui::text::LayoutJob::default();
            job.append(text, 0., self.format.clone());
            Ok(source_lines::layout(context, job).size().x)
        };
        let boundaries: Vec<_> = self
            .text
            .grapheme_indices(true)
            .map(|(i, _)| i)
            .chain(std::iter::once(self.text.len()))
            .collect();
        let mut lo = 0;
        let mut hi = boundaries.len();
        while lo < hi {
            let mid = (lo + hi) / 2;
            let width = measure(&self.text[..boundaries[mid]])?;
            if width <= available {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        self.text.truncate(boundaries[lo.saturating_sub(1)]);
        self.width = measure(&self.text)?;
        if self.positioned.is_some() {
            self.positioned =
                crate::text_engine::shape_cached(context, &self.format.font_id, &self.text)?;
        }
        Ok(())
    }
}
impl EguiMeasurer<'_> {
    pub fn rich_galley(
        &self,
        text: &RichText,
        base: TextStyle,
        width: Option<f32>,
        soft_breaks: bool,
    ) -> PaintedText {
        self.rich_galley_with(text, base, width, soft_breaks, PaintOptions::default())
    }
    pub fn rich_galley_with(
        &self,
        text: &RichText,
        base: TextStyle,
        width: Option<f32>,
        soft_breaks: bool,
        options: PaintOptions,
    ) -> PaintedText {
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = width.unwrap_or(f32::INFINITY);
        let mut links: Vec<(Range<usize>, String)> = vec![];
        let empty = Style::default();
        for item in &text.items {
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
                options.disabled_ink
            } else if let Some(color) = style
                .color
                .as_deref()
                .map(|value| parse_color(value).unwrap_or(Color32::BLACK))
            {
                color
            } else if style.link {
                options.link
            } else {
                options.ink
            };
            let sketch = self.regular == crate::fonts::family(false, false);
            let bold = base.bold || style.bold;
            let italic = base.italic || style.italic;
            let format = egui::text::TextFormat {
                font_id: FontId::new(
                    size,
                    if sketch {
                        crate::fonts::family(bold, italic)
                    } else if bold {
                        self.bold.clone()
                    } else {
                        self.regular.clone()
                    },
                ),
                color,
                italics: italic && !sketch,
                underline: if style.underline || style.link || options.underline {
                    Stroke::new((size * 0.065).max(1.), color)
                } else {
                    Stroke::NONE
                },
                strikethrough: if style.strike {
                    Stroke::new((size * 0.065).max(1.), color)
                } else {
                    Stroke::NONE
                },
                ..Default::default()
            };
            let start = job.text.len();
            // Formatting boundaries are shaping boundaries, even if appearances match.
            source_lines::append_section(&mut job, value, format);
            if let Some(target) = &style.target {
                links.push((start..job.text.len(), target.clone()));
            }
        }
        let positioned = self.regular == crate::fonts::family(false, false);
        let failed = |error: String| PaintedText {
            galley: self
                .context
                .fonts_mut(|f| f.layout_job(egui::text::LayoutJob::default())),
            rows: Vec::new(),
            runs: Vec::new(),
            links: Vec::new(),
            size: egui::vec2(f32::NAN, f32::NAN),
            error: Some(error),
        };
        if positioned
            && (job.text.len() > crate::text_engine::MAX_TEXT_BYTES || job.sections.len() > 4096)
        {
            return failed("Text exceeds the layout limit (64 KB or 4096 spans)".into());
        }
        let galley = self.context.fonts_mut(|f| f.layout_job(job));
        if positioned {
            // The UI galley cache rounds wrap widths. Canvas shaping must retain
            // the requested width, including fractional document coordinates.
            return match super::positioned::layout(
                self.context,
                galley.clone(),
                base,
                &links,
                width.unwrap_or(f32::INFINITY),
            ) {
                Ok(painted) => painted,
                Err(problem) => failed(problem),
            };
        }
        let mut rows = Vec::new();
        let mut runs = Vec::new();
        let mut areas = Vec::new();
        let mut top = 0.;
        let mut width: f32 = 0.;
        for (range, wrapped_width) in source_lines::ranges(self.context, &galley) {
            let line = source_lines::layout(
                self.context,
                source_lines::slice(&galley.job, range.clone()),
            );
            let height = line
                .job
                .sections
                .iter()
                .map(|s| s.format.font_id.size)
                .reduce(f32::max)
                .unwrap_or(base.size);
            let row_width = wrapped_width.unwrap_or_else(|| line.size().x).ceil();
            let mut row_runs = Vec::new();
            for section in &line.job.sections {
                let start = section.byte_range.start.0;
                let end = section.byte_range.end.0;
                for (part, face, face_scale) in crate::fonts::source_faces(
                    self.context,
                    &section.format.font_id.family,
                    &line.job.text[start..end],
                ) {
                    let a = start + part.start;
                    let b = start + part.end;
                    let x = source_lines::caret(self.context, &line, a);
                    row_runs.push(TextRun {
                        text: line.job.text[a..b].to_owned(),
                        format: section.format.clone(),
                        x,
                        width: (source_lines::caret(self.context, &line, b) - x).max(0.),
                        face,
                        face_scale,
                        positioned: None,
                    });
                }
            }
            for (link, target) in &links {
                let start = range.start.max(link.start);
                let end = range.end.min(link.end);
                if start < end {
                    let left = source_lines::caret(self.context, &line, start - range.start);
                    let right = source_lines::caret(self.context, &line, end - range.start);
                    areas.push(LinkArea {
                        target: target.clone(),
                        rect: egui::Rect::from_min_size(
                            egui::pos2(left, top),
                            egui::vec2((right - left).max(0.), height),
                        ),
                        row: rows.len(),
                    });
                }
            }
            rows.push(TextRow {
                top,
                height,
                width: row_width,
                baseline: top + height / 1.27,
            });
            runs.push(row_runs);
            width = width.max(row_width);
            top += height * 1.2;
        }
        let size = egui::vec2(
            width,
            rows.last()
                .map_or(base.size, |r| (r.top + r.height).round()),
        );
        PaintedText {
            galley,
            rows,
            runs,
            size,
            links: areas,
            error: None,
        }
    }
}
/// Resolve a markup color. Invalid values are rendered as black by callers.
pub fn parse_color(value: &str) -> Option<Color32> {
    let value = value.trim().to_ascii_lowercase();
    let packed = match value.as_str() {
        "black" => Some(0x000000),
        "white" => Some(0xffffff),
        _ => named_color(&value),
    };
    if let Some(rgb) = packed {
        return Some(Color32::from_rgb(
            (rgb >> 16) as u8,
            (rgb >> 8) as u8,
            rgb as u8,
        ));
    }
    let hex = value.strip_prefix('#').unwrap_or(&value);
    if !matches!(hex.len(), 3 | 6) || !hex.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let n = u32::from_str_radix(hex, 16).ok()?;
    if hex.len() == 3 {
        Some(Color32::from_rgb(
            ((n >> 8) & 15) as u8 * 17,
            ((n >> 4) & 15) as u8 * 17,
            (n & 15) as u8 * 17,
        ))
    } else {
        Some(Color32::from_rgb((n >> 16) as u8, (n >> 8) as u8, n as u8))
    }
}

fn named_color(name: &str) -> Option<u32> {
    // Each hue has its base color followed by variants 2 through 6.
    let end = name.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    let (hue, variant) = name.split_at(end);
    let grayscale = matches!(hue, "gray" | "grey");
    let index = match (grayscale, variant) {
        (true, "1") | (false, "") => 0,
        (_, "2") => 1,
        (_, "3") => 2,
        (_, "4") => 3,
        (_, "5") => 4,
        (_, "6") => 5,
        _ => return None,
    };
    Some(
        match hue {
            "gray" | "grey" => [0x333333, 0x666666, 0x999999, 0xcccccc, 0xdddddd, 0xeeeeee],
            "red" => [0xcf2a27, 0xea9999, 0xe06666, 0xcc0000, 0x990000, 0x660000],
            "orange" => [0xff9900, 0xf9cb9c, 0xf6b26b, 0xe69138, 0xb45f06, 0x783f04],
            "yellow" => [0xffff00, 0xffe599, 0xffd966, 0xf1c232, 0xbf9000, 0x7f6000],
            "green" => [0x009e0f, 0xb6d7a8, 0x93c47d, 0x6aa84f, 0x38761d, 0x274e13],
            "cyan" => [0x00ffff, 0xa2c4c9, 0x76a5af, 0x45818e, 0x134f5c, 0x0c343d],
            "blue" => [0x2b78e4, 0x9fc5f8, 0x6fa8dc, 0x597eaa, 0x085394, 0x073763],
            "purple" => [0x9900ff, 0xb4a7d6, 0x8e7cc3, 0x674ea7, 0x351c75, 0x20124d],
            "pink" => [0xff00ff, 0xd5a6bd, 0xc27ba0, 0xa64d79, 0x741b47, 0x4c1130],
            _ => return None,
        }[index],
    )
}
