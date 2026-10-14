// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Single-line rows, sizing and selected-row identity for list controls.
use crate::{
    document::{Node, Properties, Value},
    layout::{TextMeasurer, TextStyle},
    rich_text::RichText,
};

pub struct List {
    pub rows: Vec<RichText>,
}
pub struct Metrics {
    pub width: f32,
    pub height: f32,
    pub row: f32,
    pub top: f32,
    pub header: f32,
}
pub fn labels(source: &str, header: bool) -> Vec<&str> {
    source.split('\n').skip(usize::from(header)).collect()
}
pub fn selection_after_edit(old: &str, new: &str, selected: i32, header: bool) -> i32 {
    if selected < 0 {
        return -1;
    }
    let offset = usize::from(header);
    let old: Vec<_> = old.split('\n').collect();
    let new: Vec<_> = new.split('\n').collect();
    if let Some(label) = old.get(selected as usize + offset)
        && let Some(index) = new.iter().position(|line| line == label)
    {
        return index as i32 - offset as i32;
    }
    if old.len() == new.len() { selected } else { -1 }
}
pub fn apply_text(node: &mut Node, source: &str, baseline: &Properties) -> Result<(), String> {
    if source.len() > 100_000 {
        return Err("List text exceeds the text limit".into());
    }
    List::parse(source)?;
    let old = match baseline.get("text") {
        Some(Value::Text(s)) => s.as_str(),
        _ => "",
    };
    let selected = match baseline.get("selected_index") {
        Some(Value::Number(n)) => *n as i32,
        _ => -1,
    };
    let header = baseline.get("header") == Some(&Value::Bool(true));
    let next = selection_after_edit(old, source, selected, header);
    node.properties
        .insert("text".into(), Value::Text(source.into()));
    // Keep explicit None when editing a component whose source selects a row.
    node.properties
        .insert("selected_index".into(), Value::Number(next as f64));
    Ok(())
}
impl List {
    pub fn parse(source: &str) -> Result<Self, String> {
        let lines: Vec<_> = source.split('\n').take(10_002).collect();
        if lines.len() > 10_001 {
            return Err("List exceeds 10,000 data rows".into());
        }
        Ok(Self {
            rows: lines
                .into_iter()
                .map(|s| RichText::parse(s, false))
                .collect(),
        })
    }
    pub fn measure(
        &self,
        fonts: &impl TextMeasurer,
        style: TextStyle,
        sketch: bool,
        header: bool,
        row_height: f32,
    ) -> Metrics {
        let top = if sketch { 3. } else { 2. };
        let row = row_height.max(style.size + 8.);
        let measured: Vec<_> = self
            .rows
            .iter()
            .map(|r| fonts.measure_rich(r, style, None, false))
            .collect();
        let first = measured
            .first()
            .map_or(style.size, |m| m.height.max(style.size));
        let height = if header {
            top + 3. + first + 6. + (self.rows.len() as f32 - 2.) * row
        } else {
            top + 3. + self.rows.len() as f32 * row
        };
        Metrics {
            width: (measured.iter().map(|m| m.width).fold(0., f32::max) + 16.).max(100.),
            height: height.max(126.),
            row,
            top,
            header: if header { top + 6. + first } else { 0. },
        }
    }
}
