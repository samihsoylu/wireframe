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
/// A complete styled source run, shaped together when painted or exported.
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
            } else if let Some(color) = style.color.as_deref().and_then(parse_color) {
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
            return match super::positioned::layout(self.context, galley.clone(), base, &links) {
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
pub(crate) fn parse_color(value: &str) -> Option<Color32> {
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
