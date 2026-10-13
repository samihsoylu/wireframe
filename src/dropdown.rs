// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Source-preserving dropdown text semantics and shared measurement.
use crate::{
    layout::{TextMeasurer, TextStyle},
    rich_text::RichText,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    None,
    Dot,
    Check,
}
#[derive(Clone, Debug)]
pub struct Row {
    pub text: String,
    pub mark: Mark,
    pub arrow: bool,
    pub separator: bool,
    pub selected: bool,
}
pub struct Menu {
    pub title: String,
    pub rows: Vec<Row>,
    pub has_marks: bool,
}
pub struct Metrics {
    pub width: f32,
    pub height: f32,
    pub header: f32,
    pub rows: Vec<f32>,
}
pub fn selectable_labels(source: &str) -> Vec<&str> {
    source
        .split('\n')
        .enumerate()
        .filter(|(i, s)| *i == 0 || !s.trim().is_empty())
        .map(|(_, s)| s)
        .collect()
}
impl Menu {
    pub fn parse(source: &str, selection: usize) -> Result<Self, String> {
        let lines: Vec<_> = source.split('\n').take(10_002).collect();
        if lines.len() > 10_001 {
            return Err("Dropdown exceeds 10,000 menu rows".into());
        }
        let selected = lines
            .iter()
            .enumerate()
            .filter(|(i, s)| *i == 0 || !s.trim().is_empty())
            .nth(selection)
            .map(|(i, _)| i);
        let title = selected.map_or("", |i| lines[i]);
        let title = title.strip_prefix("o ").unwrap_or(title);
        let title = title.strip_prefix("x ").unwrap_or(title);
        let title = title.strip_suffix(" >").unwrap_or(title).to_owned();
        let mut has_marks = lines
            .iter()
            .any(|s| s.starts_with("o ") || s.starts_with("x "));
        let rows = lines
            .iter()
            .enumerate()
            .skip(1)
            .map(|(i, line)| {
                let separator = line.chars().all(|c| c == ' ' || c == '-')
                    || line.chars().all(|c| c == ' ' || c == '=');
                let (mark, text) = if let Some(t) = line.strip_prefix("o ") {
                    (Mark::Dot, t)
                } else if let Some(t) = line.strip_prefix("x ") {
                    (Mark::Check, t)
                } else {
                    (Mark::None, *line)
                };
                has_marks |= mark != Mark::None;
                let arrow = text
                    .rsplit_once(' ')
                    .is_some_and(|(_, s)| !s.is_empty() && s.chars().all(|c| c == '>'));
                let text = if arrow {
                    text.rsplit_once(' ').unwrap().0
                } else {
                    text
                };
                Row {
                    text: text.replacen('\u{2028}', "", 1),
                    mark,
                    arrow,
                    separator,
                    selected: selected == Some(i),
                }
            })
            .collect();
        Ok(Self {
            title,
            rows,
            has_marks,
        })
    }
    pub fn measure(
        &self,
        fonts: &impl TextMeasurer,
        style: TextStyle,
        sketch: bool,
        closed: bool,
    ) -> Metrics {
        let title = fonts.measure_rich(&RichText::parse(&self.title, false), style, None, false);
        let header = if title.height == 0. {
            style.size
        } else {
            title.height
        } + if sketch { 14. } else { 11. };
        let rows: Vec<f32> = self
            .rows
            .iter()
            .map(|r| {
                if r.separator {
                    if sketch { 5. } else { 1. }
                } else {
                    fonts
                        .measure_rich(&RichText::parse(&r.text, false), style, None, false)
                        .height
                        + 3.
                }
            })
            .collect();
        let height = header
            + if closed || rows.is_empty() {
                0.
            } else {
                rows.iter().sum::<f32>() + if sketch { 10. } else { 9. }
            };
        Metrics {
            width: title.width + 30.,
            height,
            header,
            rows,
        }
    }
}
