// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Intrinsic comma-separated navigation labels and stable item metadata.
use crate::{
    document::{Node, NodeContent, Properties, Value},
    layout::{TextMeasurer, TextStyle},
    rich_text::RichText,
};
pub const BREADCRUMBS: &str = "Home, Products, Xyz, Features";
pub const LINK_BAR: &str = "Home, Products, Company, Blog";
/// Preserve escaped commas and remove one space immediately after a delimiter.
pub fn split(text: &str) -> Vec<String> {
    let mut parts = vec![String::new()];
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&',') {
            chars.next();
            parts.last_mut().unwrap().push(',');
        } else if c == ',' {
            parts.push(String::new());
            if chars.peek() == Some(&' ') {
                chars.next();
            }
        } else {
            parts.last_mut().unwrap().push(c);
        }
    }
    parts
}
pub fn labels(text: &str, breadcrumbs: bool) -> Vec<String> {
    split(text)
        .into_iter()
        .map(|s| {
            if breadcrumbs {
                RichText::parse(&s, false).display_text(true)
            } else {
                s
            }
        })
        .collect()
}
pub struct Navigation {
    pub texts: Vec<RichText>,
    pub breadcrumbs: bool,
}
pub struct Metrics {
    pub width: f32,
    pub height: f32,
    pub items: Vec<(f32, f32)>,
}
impl Navigation {
    pub fn parse(text: &str, breadcrumbs: bool) -> Result<Self, String> {
        if text.len() > 100_000 {
            return Err("Navigation text exceeds the text limit".into());
        }
        let text = if text.is_empty() && !breadcrumbs {
            LINK_BAR
        } else {
            text
        };
        let mut parts = split(text);
        if parts.len() > 10_001 {
            return Err("Navigation exceeds 10,001 items".into());
        }
        if breadcrumbs && parts.len() == 1 {
            parts.push("...".into());
        }
        Ok(Self {
            texts: parts
                .iter()
                .map(|s| RichText::parse(if breadcrumbs { s } else { s.trim() }, false))
                .collect(),
            breadcrumbs,
        })
    }
    pub fn measure(&self, fonts: &impl TextMeasurer, style: TextStyle) -> Metrics {
        let items = self
            .texts
            .iter()
            .map(|s| {
                let m = fonts.measure_rich(s, style, None, true);
                (m.width, m.height)
            })
            .collect::<Vec<_>>();
        Metrics {
            // A whitespace-only item must remain a valid, editable control.
            width: (items.iter().map(|m| m.0).sum::<f32>()
                + (items.len().saturating_sub(1) as f32)
                    * if self.breadcrumbs { 16. } else { 18. })
            .max(1.),
            height: items.iter().map(|m| m.1).fold(0., f32::max) + 8.,
            items,
        }
    }
}
pub fn apply_text(node: &mut Node, text: &str, baseline: &Properties) -> Result<(), String> {
    let breadcrumbs = matches!(&node.content,NodeContent::Control {kind,..} if kind=="breadcrumbs");
    Navigation::parse(text, breadcrumbs)?;
    let old = match baseline.get("text") {
        Some(Value::Text(s)) => s.as_str(),
        _ => "",
    };
    let before = labels(old, breadcrumbs);
    let after = labels(text, breadcrumbs);
    if let Some(Value::Links(links)) = baseline.get("row_links") {
        let before = before.iter().map(String::as_str).collect::<Vec<_>>();
        let after = after.iter().map(String::as_str).collect::<Vec<_>>();
        let mapped = if text.is_empty() {
            vec![]
        } else {
            crate::list::remap_labels(&before, &after, links)
        };
        node.properties
            .insert("row_links".into(), Value::Links(mapped));
    }
    if !breadcrumbs {
        let selected = match baseline.get("selected_index") {
            Some(Value::Number(n)) => *n as i32,
            _ => -1,
        };
        let selected = if text.is_empty() || selected < 0 {
            -1
        } else if let Some(i) = before
            .get(selected as usize)
            .and_then(|s| after.iter().position(|n| s == n))
        {
            i as i32
        } else if before.len() == after.len() {
            selected
        } else {
            -1
        };
        node.properties
            .insert("selected_index".into(), Value::Number(selected as f64));
    }
    if !breadcrumbs && text.is_empty() {
        node.properties
            .insert("text".into(), Value::Text(LINK_BAR.into()));
    } else {
        node.properties
            .insert("text".into(), Value::Text(text.into()));
    }
    Ok(())
}
