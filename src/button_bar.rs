// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Comma-separated segment text, automatic metrics and visible columns.
use crate::{
    document::{Node, Properties, Value},
    layout::{TextMeasurer, TextStyle},
    rich_text::RichText,
};
pub struct ButtonBar {
    pub texts: Vec<RichText>,
}
pub struct Metrics {
    pub width: f32,
    pub height: f32,
    pub columns: Vec<f32>,
}
pub fn labels(text: &str) -> Vec<&str> {
    text.split(',').map(str::trim).collect()
}
impl ButtonBar {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut parts = text
            .split(',')
            .take(10_002)
            .map(|s| s.strip_prefix(' ').unwrap_or(s))
            .collect::<Vec<_>>();
        if parts.len() > 10_001 {
            return Err("Button bar exceeds 10,001 segments".into());
        }
        if parts.len() == 1 {
            parts.push("...");
        }
        Ok(Self {
            texts: parts
                .into_iter()
                .map(|s| RichText::parse(s, false))
                .collect(),
        })
    }
    pub fn measure(&self, fonts: &impl TextMeasurer, style: TextStyle, sketch: bool) -> Metrics {
        let measured = self
            .texts
            .iter()
            .map(|s| fonts.measure_rich(s, style, None, true))
            .collect::<Vec<_>>();
        let columns = measured
            .iter()
            .enumerate()
            .map(|(i, m)| {
                m.width
                    + if i == 0 { 14. } else { 12. }
                    + if i + 1 == measured.len() { 14. } else { 12. }
            })
            .collect::<Vec<_>>();
        Metrics {
            width: columns.iter().sum(),
            height: measured.iter().map(|m| m.height).fold(0., f32::max)
                + if sketch { 14. } else { 8. },
            columns,
        }
    }
}
impl Metrics {
    pub fn visible_columns(&self, width: f32) -> Vec<(f32, f32)> {
        let mut result = Vec::new();
        let mut x = 0.;
        for (i, &minimum) in self.columns.iter().enumerate() {
            let proposed = minimum.max(width * minimum / self.width).round();
            let size = if i + 1 == self.columns.len() && x + proposed > width {
                (width - x).max(minimum)
            } else {
                proposed
            };
            if x + size > width {
                break;
            }
            result.push((x, size));
            x += size;
        }
        result
    }
}
pub fn apply_text(node: &mut Node, text: &str, baseline: &Properties) -> Result<(), String> {
    if text.len() > 100_000 {
        return Err("Button bar text exceeds the text limit".into());
    }
    ButtonBar::parse(text)?;
    let old = match baseline.get("text") {
        Some(Value::Text(s)) => s.as_str(),
        _ => "",
    };
    let before = labels(old);
    let after = labels(text);
    let selected = match baseline.get("selected_index") {
        Some(Value::Number(n)) => *n as i32,
        _ => 0,
    };
    let next = if selected < 0 {
        -1
    } else if let Some(index) = before
        .get(selected as usize)
        .and_then(|label| after.iter().position(|s| s == label))
    {
        index as i32
    } else if before.len() == after.len() {
        selected
    } else {
        -1
    };
    if let Some(Value::Links(links)) = baseline.get("row_links") {
        let links = if text.is_empty() {
            vec![]
        } else {
            crate::list::remap_labels(&before, &after, links)
        };
        node.properties
            .insert("row_links".into(), Value::Links(links));
    }
    node.properties
        .insert("text".into(), Value::Text(text.into()));
    node.properties
        .insert("selected_index".into(), Value::Number(next as f64));
    Ok(())
}
