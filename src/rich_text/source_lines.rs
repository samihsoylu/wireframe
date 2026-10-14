// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Map wrapping back to original UTF-8, never to reconstructed glyph characters.
use eframe::egui::{self, text::LayoutJob};
use std::{ops::Range, sync::Arc};
use unicode_segmentation::UnicodeSegmentation;

pub(super) fn append_section(job: &mut LayoutJob, text: &str, format: egui::text::TextFormat) {
    let start = job.text.len();
    job.text.push_str(text);
    let mut section = egui::text::LayoutSection {
        format,
        leading_space: 0.,
        byte_range: Default::default(),
    };
    section.byte_range.start.0 = start;
    section.byte_range.end.0 = job.text.len();
    job.sections.push(section);
}

pub(super) fn slice(job: &LayoutJob, range: Range<usize>) -> LayoutJob {
    let mut out = LayoutJob::default();
    for section in &job.sections {
        let start = range.start.max(section.byte_range.start.0);
        let end = range.end.min(section.byte_range.end.0);
        if start < end {
            append_section(&mut out, &job.text[start..end], section.format.clone());
        }
    }
    out
}

pub(super) fn layout(ctx: &egui::Context, job: LayoutJob) -> Arc<egui::Galley> {
    ctx.fonts_mut(|fonts| fonts.layout_job(job))
}

pub(super) fn ranges(
    ctx: &egui::Context,
    galley: &egui::Galley,
) -> Vec<(Range<usize>, Option<f32>)> {
    let mut out = Vec::new();
    let mut row_start = 0;
    let mut offset = 0;
    for paragraph in galley.job.text.split('\n') {
        let mut row_end = row_start;
        while row_end < galley.rows.len() {
            row_end += 1;
            if galley.rows[row_end - 1].ends_with_newline {
                break;
            }
        }
        let rows = &galley.rows[row_start..row_end];
        let chars: Vec<_> = paragraph
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(paragraph.len()))
            .collect();
        let boundaries: Vec<_> = paragraph
            .grapheme_indices(true)
            .map(|(i, _)| i)
            .chain(std::iter::once(paragraph.len()))
            .collect();
        let mut consumed = 0;
        let mut proposed = Vec::new();
        for row in rows {
            let start = consumed;
            consumed += row.glyphs.len();
            if let (Some(&a), Some(&b)) = (chars.get(start), chars.get(consumed)) {
                proposed.push(offset + a..offset + b);
            }
        }
        if rows.len() <= 1 {
            out.push((
                offset..offset + paragraph.len(),
                rows.first().map(|r| r.size.x),
            ));
        } else if consumed == chars.len() - 1
            && proposed.len() == rows.len()
            && proposed
                .iter()
                .all(|r| boundaries.binary_search(&(r.end - offset)).is_ok())
        {
            out.extend(
                proposed
                    .into_iter()
                    .zip(rows.iter().map(|r| Some(r.size.x))),
            );
        } else {
            // Some shaping backends expose more glyph records than source characters.
            // Re-wrap original graphemes in that case; source ownership stays exact.
            let mut first = 0;
            while first + 1 < boundaries.len() {
                let start = boundaries[first];
                let fits = |end| {
                    layout(
                        ctx,
                        slice(&galley.job, offset + start..offset + boundaries[end]),
                    )
                    .size()
                    .x <= galley.job.wrap.max_width
                };
                // Exponential search bounds work to the current line rather than
                // repeatedly shaping the entire remaining paragraph at narrow widths.
                let mut lo = first;
                let mut hi = first + 1;
                while fits(hi) {
                    lo = hi;
                    if hi + 1 == boundaries.len() {
                        break;
                    }
                    hi = (first + (hi - first) * 2).min(boundaries.len() - 1);
                }
                while lo + 1 < hi {
                    let mid = (lo + hi) / 2;
                    if fits(mid) {
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
                out.push((offset + start..offset + end, None));
                first = boundaries
                    .binary_search(&end)
                    .expect("word boundary is a grapheme boundary");
            }
        }
        offset += paragraph.len() + 1;
        row_start = row_end;
    }
    out
}

/// A source caret in a single unwrapped line. Glyph counts are used only when
/// their correspondence has been checked; otherwise shape the source prefix.
pub(super) fn caret(ctx: &egui::Context, line: &egui::Galley, byte: usize) -> f32 {
    let Some(row) = line.rows.first() else {
        return 0.;
    };
    if byte == line.job.text.len() {
        return row.size.x;
    }
    if line.job.text.chars().eq(row.glyphs.iter().map(|g| g.chr)) {
        let index = line.job.text[..byte].chars().count();
        row.glyphs.get(index).map_or_else(
            || row.glyphs.last().map_or(0., |g| g.pos.x + g.advance_width),
            |g| g.pos.x,
        )
    } else {
        layout(ctx, slice(&line.job, 0..byte)).size().x
    }
}
