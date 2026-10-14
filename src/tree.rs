// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Source-defined hierarchy rows, measurement and destination ownership.
use crate::{
    document::{Node, Properties, Value},
    layout::{TextMeasurer, TextStyle},
    rich_text::structured::{self, TreeRow},
};

pub const DEFAULT: &str =
    "F Project\n f Screens\n..- Overview\n..- Details\n f Assets\n..- Logo\n- Notes";

pub struct Tree {
    pub rows: Vec<TreeRow>,
}
pub struct RowMetrics {
    pub y: f32,
    pub height: f32,
    pub text_width: f32,
    pub text_height: f32,
}
pub struct Metrics {
    pub height: f32,
    pub rows: Vec<RowMetrics>,
}
pub fn labels(source: &str) -> Vec<String> {
    structured::tree(source)
        .into_iter()
        .map(|r| r.source)
        .collect()
}
impl Tree {
    pub fn parse(source: &str) -> Result<Self, String> {
        if source.len() > 100_000 || source.split('\n').take(10_001).count() > 10_000 {
            return Err("Tree exceeds the text or row limit".into());
        }
        Ok(Self {
            rows: structured::tree(source),
        })
    }
    pub fn measure(&self, fonts: &impl TextMeasurer, style: TextStyle) -> Metrics {
        let mut y = 0.;
        let rows = self
            .rows
            .iter()
            .map(|row| {
                let text = fonts.measure_rich(&row.text, style, None, true);
                let result = RowMetrics {
                    y,
                    height: text.height + 8.,
                    text_width: text.width,
                    text_height: text.height,
                };
                y += result.height;
                result
            })
            .collect();
        Metrics {
            height: y + 12.,
            rows,
        }
    }
}
pub fn icon(marker: Option<&str>) -> Option<&'static str> {
    Some(match marker? {
        "folder" => "folder-solid",
        "open_folder" => "folder-open-regular",
        "right" => "caret-right-solid",
        "left" => "caret-left-solid",
        "down" => "caret-down-solid",
        "file" => "file-lines-regular",
        "unchecked" => "square-regular",
        "expand" => "square-plus-regular",
        "collapse" => "square-minus-regular",
        "checked" => "square-check-regular",
        _ => return None,
    })
}
pub fn apply_text(node: &mut Node, source: &str, baseline: &Properties) -> Result<(), String> {
    Tree::parse(source)?;
    let old = match baseline.get("text") {
        Some(Value::Text(s)) => s.as_str(),
        _ => "",
    };
    let before = labels(old);
    let after = labels(source);
    let before: Vec<_> = before.iter().map(String::as_str).collect();
    let after: Vec<_> = after.iter().map(String::as_str).collect();
    if let Some(Value::Links(links)) = baseline.get("row_links") {
        node.properties.insert(
            "row_links".into(),
            Value::Links(crate::list::remap_labels(&before, &after, links)),
        );
    }
    let selected = match baseline.get("selected_index") {
        Some(Value::Number(n)) => *n as i32,
        _ => -1,
    };
    let next = if let Some(label) = before.get(selected as usize) {
        let occurrence = before[..selected as usize]
            .iter()
            .filter(|s| *s == label)
            .count();
        after
            .iter()
            .enumerate()
            .filter(|(_, s)| *s == label)
            .nth(occurrence)
            .map(|(i, _)| i as i32)
            .unwrap_or(if before.len() == after.len() {
                selected
            } else {
                -1
            })
    } else if before.len() == after.len() {
        selected
    } else {
        -1
    };
    node.properties
        .insert("selected_index".into(), Value::Number(next as f64));
    node.properties
        .insert("text".into(), Value::Text(source.into()));
    Ok(())
}
