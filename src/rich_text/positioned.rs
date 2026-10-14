// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Source-based line layout over independently shaped formatting spans.
use super::{
    painting::{LinkArea, PaintedText, TextRow, TextRun},
    source_lines,
};
use crate::{layout::TextStyle, text_engine};
use eframe::egui::{self, text::LayoutJob};
use std::{ops::Range, sync::Arc};
use unicode_segmentation::UnicodeSegmentation;

fn shape(
    ctx: &egui::Context,
    job: &LayoutJob,
    range: Range<usize>,
) -> Result<Vec<TextRun>, String> {
    let line = source_lines::slice(job, range);
    let mut runs = Vec::new();
    let mut x = 0.;
    for section in &line.sections {
        let text = &line.text[section.byte_range.start.0..section.byte_range.end.0];
        let shaped = text_engine::shape_cached(ctx, &section.format.font_id, text)?
            .ok_or("The selected canvas font is unavailable")?;
        let width = shaped.width;
        runs.push(TextRun {
            text: text.into(),
            format: section.format.clone(),
            x: x as f32,
            width: width as f32,
            face: section.format.font_id.family.to_string(),
            face_scale: 1.,
            positioned: Some(shaped),
        });
        x += width;
    }
    Ok(runs)
}
fn width(ctx: &egui::Context, job: &LayoutJob, range: Range<usize>) -> Result<f64, String> {
    Ok(shape(ctx, job, range)?
        .iter()
        .map(|r| r.positioned.as_ref().unwrap().width)
        .sum())
}
fn ranges(ctx: &egui::Context, job: &LayoutJob) -> Result<Vec<Range<usize>>, String> {
    let mut rows = Vec::new();
    let mut offset = 0;
    for paragraph in job.text.split('\n') {
        if paragraph.is_empty() || !job.wrap.max_width.is_finite() {
            rows.push(offset..offset + paragraph.len());
        } else {
            let boundaries: Vec<_> = paragraph
                .grapheme_indices(true)
                .map(|(i, _)| i)
                .chain(std::iter::once(paragraph.len()))
                .collect();
            let mut first = 0;
            while first + 1 < boundaries.len() {
                let start = boundaries[first];
                let fits = |end: usize| -> Result<bool, String> {
                    Ok(width(ctx, job, offset + start..offset + boundaries[end])?
                        <= f64::from(job.wrap.max_width.max(0.)))
                };
                // Bound shaping work to this line, including very narrow long paragraphs.
                let mut lo = first;
                let mut hi = first + 1;
                while fits(hi)? {
                    lo = hi;
                    if hi + 1 == boundaries.len() {
                        break;
                    }
                    hi = (first + (hi - first) * 2).min(boundaries.len() - 1);
                }
                while lo + 1 < hi {
                    let mid = (lo + hi) / 2;
                    if fits(mid)? {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                let fit = boundaries[lo.max(first + 1)];
                let end = if fit < paragraph.len() {
                    paragraph[start..fit]
                        .split_word_bound_indices()
                        .filter_map(|(i, word)| {
                            word.chars()
                                .all(char::is_whitespace)
                                .then_some(start + i + word.len())
                        })
                        .next_back()
                        .unwrap_or(fit)
                } else {
                    fit
                };
                rows.push(offset + start..offset + end);
                first = boundaries
                    .binary_search(&end)
                    .map_err(|_| "Invalid line boundary")?;
            }
        }
        offset += paragraph.len() + 1;
    }
    Ok(rows)
}
pub(super) fn layout(
    ctx: &egui::Context,
    galley: Arc<egui::Galley>,
    base: TextStyle,
    links: &[(Range<usize>, String)],
) -> Result<PaintedText, String> {
    let job = &galley.job;
    // Bound the entire layout as well as individual spans to avoid adversarial fragmentation.
    if job.text.len() > text_engine::MAX_TEXT_BYTES || job.sections.len() > 4096 {
        return Err("Text exceeds the layout limit (64 KB or 4096 spans)".into());
    }
    let mut rows = Vec::new();
    let mut runs = Vec::new();
    let mut areas = Vec::new();
    let mut top = 0.;
    let mut widest: f32 = 0.;
    for range in ranges(ctx, job)? {
        let row_runs = shape(ctx, job, range.clone())?;
        let height = row_runs
            .iter()
            .map(|r| r.format.font_id.size)
            .reduce(f32::max)
            .unwrap_or(base.size);
        let row_width = row_runs
            .iter()
            .map(|r| r.positioned.as_ref().unwrap().width)
            .sum::<f64>()
            .ceil() as f32;
        let mut source = range.start;
        for run in &row_runs {
            let run_range = source..source + run.text.len();
            // A target belongs to a complete source formatting span. Line wrapping
            // may split it; a target never splits a shaped run or a bidi cluster.
            if let Some((_, target)) = links
                .iter()
                .find(|(link, _)| link.start <= run_range.start && link.end >= run_range.end)
            {
                areas.push(LinkArea {
                    target: target.clone(),
                    row: rows.len(),
                    rect: egui::Rect::from_min_size(
                        egui::pos2(run.x, top),
                        egui::vec2(run.width, height),
                    ),
                });
            }
            source = run_range.end;
        }
        rows.push(TextRow {
            top,
            height,
            width: row_width,
            baseline: top + height / 1.27,
        });
        runs.push(row_runs);
        widest = widest.max(row_width);
        top += height * 1.2;
    }
    let size = egui::vec2(
        widest,
        rows.last()
            .map_or(base.size, |r| (r.top + r.height).round()),
    );
    Ok(PaintedText {
        galley,
        rows,
        runs,
        size,
        links: areas,
        error: None,
    })
}
