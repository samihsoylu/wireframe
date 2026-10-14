// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Text-defined checkbox and radio rows with shared measurement and link identity.
use crate::{
    document::{Node, Properties, Value},
    layout::{TextMeasurer, TextStyle},
    rich_text::RichText,
};

pub const CHECKBOX: &str = "[ ] not selected\n[x] selected\n[-] indeterminate\n-[ ] disabled-\n-[x] disabled selected-\n-[-] disabled indeterminate-\nA row without a checkbox";
pub const RADIO: &str = "(o) option 1 (selected)\n( ) option 2\n(-) option 3 (indeterminate)\n-( ) option 4 (disabled)-\n-(o) option 5\\r(disabled and selected)-\n-(-) option 6\\r(disabled indeterminate)-\nA row without a radio button";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Empty,
    Selected,
    Mixed,
}
#[derive(Debug)]
pub struct Row {
    pub text: RichText,
    pub mark: Option<Mark>,
    pub disabled: bool,
    pub symbol_color: Option<String>,
}
pub struct ChoiceGroup {
    pub rows: Vec<Row>,
}
pub struct RowMetrics {
    pub y: f32,
    pub height: f32,
    pub text_height: f32,
}
pub struct Metrics {
    pub width: f32,
    pub height: f32,
    pub rows: Vec<RowMetrics>,
}

impl Metrics {
    pub fn visible_rows(&self, height: f32) -> impl Iterator<Item = (usize, &RowMetrics)> {
        self.rows
            .iter()
            .enumerate()
            .take_while(move |(_, r)| r.y + r.height <= height)
    }
}

/// A leading color wrapper may cover the marker alone or marker and label.
fn color_wrapper(source: &str) -> Option<(&str, &str, &str, &str, &str)> {
    let rest = source
        .strip_prefix("{color:")
        .or_else(|| source.strip_prefix("{colour:"))?;
    let (color, _) = rest.split_once('}')?;
    if color.is_empty() {
        return None;
    }
    let open_end = source.find('}')? + 1;
    let close = [source.rfind("{color}"), source.rfind("{colour}")]
        .into_iter()
        .flatten()
        .max()?;
    if close < open_end {
        return None;
    }
    let closing_len = if source[close..].starts_with("{colour}") {
        8
    } else {
        7
    };
    Some((
        color,
        &source[..open_end],
        &source[open_end..close],
        &source[close..close + closing_len],
        &source[close + closing_len..],
    ))
}

pub fn parse_row(source: &str, radio: bool) -> Row {
    let plain = || Row {
        text: RichText::parse(source, false),
        mark: None,
        disabled: false,
        symbol_color: None,
    };
    let wrapper = color_wrapper(source);
    let body = wrapper.map_or(source, |(_, _, body, _, _)| body);
    let (body, disabled) =
        if let Some(inner) = body.strip_prefix('-').and_then(|s| s.strip_suffix('-')) {
            (inner, true)
        } else {
            (body, false)
        };
    let (left, right) = if radio { ('(', ')') } else { ('[', ']') };
    let Some(rest) = body.strip_prefix(left) else {
        return plain();
    };
    let Some(end) = rest.find(right) else {
        return plain();
    };
    let mark = match &rest[..end] {
        "" | " " => Mark::Empty,
        "o" | "O" | "v" | "V" | "x" | "X" | "*" => Mark::Selected,
        "-" | "=" => Mark::Mixed,
        _ => return plain(),
    };
    let label = &rest[end + 1..];
    let text = if let Some((_, opening, _, closing, tail)) = wrapper {
        // Preserve the label's formatting independently of the symbol color.
        if label.is_empty() {
            tail.trim().to_owned()
        } else {
            format!(
                "{opening}{}{label}{closing}{tail}",
                if disabled { "-" } else { "" },
                label = label.trim()
            )
        }
    } else {
        label.trim().to_owned()
    };
    Row {
        text: RichText::parse(&text, false),
        mark: Some(mark),
        disabled,
        symbol_color: wrapper.map(|(color, _, _, _, _)| color.to_owned()),
    }
}

impl ChoiceGroup {
    pub fn parse(source: &str, radio: bool) -> Result<Self, String> {
        if source.len() > 100_000 || source.split('\n').take(10_001).count() > 10_000 {
            return Err("Choice group exceeds the text or row limit".into());
        }
        Ok(Self {
            rows: source
                .split('\n')
                .map(|line| parse_row(line, radio))
                .collect(),
        })
    }
    pub fn measure(&self, fonts: &impl TextMeasurer, style: TextStyle) -> Metrics {
        let mut width: f32 = 0.;
        let mut y = 2.;
        let mut rows = Vec::with_capacity(self.rows.len());
        for row in &self.rows {
            let text = fonts.measure_rich(&row.text, style, None, true);
            let height = if row.mark.is_some() {
                text.height.max(12.)
            } else {
                text.height
            };
            width = width.max(text.width + if row.mark.is_some() { 20. } else { 0. });
            rows.push(RowMetrics {
                y,
                height,
                text_height: text.height,
            });
            y += height + 9.;
        }
        Metrics {
            width,
            height: y - 9. + 2.,
            rows,
        }
    }
}

pub fn apply_text(node: &mut Node, source: &str, baseline: &Properties) -> Result<(), String> {
    // Row count and size limits do not depend on marker kind.
    ChoiceGroup::parse(source, false)?;
    let old = match baseline.get("text") {
        Some(Value::Text(s)) => s.as_str(),
        _ => "",
    };
    if let Some(Value::Links(links)) = baseline.get("row_links") {
        node.properties.insert(
            "row_links".into(),
            Value::Links(crate::list::remap_rows(old, source, links)),
        );
    }
    node.properties
        .insert("text".into(), Value::Text(source.into()));
    Ok(())
}
