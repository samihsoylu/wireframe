// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Source-defined menus, measured rows and stable item destinations.
use crate::{
    document::{Node, NodeContent, Properties, Value},
    layout::{TextMeasurer, TextStyle},
    rich_text::RichText,
};
pub const MENU: &str = "Open,CTRL+O\nOpen Recent >\n---\no Option One\nOption Two\n=\nx Toggle Item\n-Disabled Item-\nExit,CTRL+Q";
pub const BAR: &str = "File, Edit, View, Help";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Check,
    Dot,
}
#[derive(Debug)]
pub struct Row {
    pub label: RichText,
    pub shortcut: Option<RichText>,
    pub mark: Option<Mark>,
    pub arrow: bool,
    pub separator: bool,
}
pub struct Menu {
    pub rows: Vec<Row>,
}
pub struct RowMetrics {
    pub y: f32,
    pub height: f32,
    pub shortcut_width: f32,
}
pub struct MenuMetrics {
    pub height: f32,
    pub rows: Vec<RowMetrics>,
}
pub struct MenuBar {
    pub texts: Vec<RichText>,
}
pub struct BarMetrics {
    pub width: f32,
    pub height: f32,
    pub widths: Vec<f32>,
}
#[derive(Debug)]
pub struct BarItem {
    pub index: usize,
    pub x: f32,
    pub width: f32,
    pub visible: bool,
}
fn check_limit(source: &str) -> Result<(), String> {
    if source.len() > 100_000 {
        Err("Menu exceeds the text limit".into())
    } else {
        Ok(())
    }
}
pub fn labels(source: &str, bar: bool) -> Vec<String> {
    if bar {
        crate::navigation::split(source)
    } else {
        source.split('\n').map(str::to_owned).collect()
    }
}
pub fn parse_row(source: &str) -> Row {
    let separator = source.chars().all(|c| c == ' ' || c == '-')
        || source.chars().all(|c| c == ' ' || c == '=');
    let source = source.replacen('\u{2028}', "", 1);
    let (source, mark) = if let Some(s) = source.strip_prefix("x ") {
        (s, Some(Mark::Check))
    } else if let Some(s) = source.strip_prefix("o ") {
        (s, Some(Mark::Dot))
    } else {
        (source.as_str(), None)
    };
    let arrow_split = source
        .rsplit_once(' ')
        .filter(|(_, tail)| !tail.is_empty() && tail.chars().all(|c| c == '>'))
        .or_else(|| {
            (!source.is_empty() && source.chars().all(|c| c == '>')).then_some(("", source))
        });
    let mut unescaped = String::new();
    let mut comma = None;
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&',') {
            chars.next();
            unescaped.push(',');
        } else {
            if c == ',' {
                comma = Some(unescaped.len());
            }
            unescaped.push(c);
        }
    }
    let (label, shortcut) = if let Some((label, _)) = arrow_split {
        (label.replace("\\,", ","), None)
    } else if let Some(i) = comma {
        (
            unescaped[..i].trim_end_matches([' ', ',']).to_owned(),
            Some(RichText::parse(
                unescaped[i + 1..].trim_end_matches([' ', ',']),
                false,
            )),
        )
    } else {
        (unescaped, None)
    };
    Row {
        label: RichText::parse(&label, false),
        shortcut,
        mark,
        arrow: arrow_split.is_some(),
        separator,
    }
}
impl Menu {
    pub fn parse(source: &str) -> Result<Self, String> {
        check_limit(source)?;
        if source.split('\n').take(10_001).count() > 10_000 {
            return Err("Menu exceeds 10,000 rows".into());
        }
        Ok(Self {
            rows: source.split('\n').map(parse_row).collect(),
        })
    }
    pub fn measure(
        &self,
        fonts: &impl TextMeasurer,
        style: TextStyle,
        sketch: bool,
    ) -> MenuMetrics {
        let padding = if sketch { 5. } else { 1. };
        let mut y = padding;
        let rows = self
            .rows
            .iter()
            .map(|row| {
                let advance = if row.separator {
                    if sketch { 5. } else { 1. }
                } else {
                    fonts.measure_rich(&row.label, style, None, true).height + 8.
                };
                let result = RowMetrics {
                    y,
                    height: if row.separator && sketch {
                        10.
                    } else {
                        advance
                    },
                    shortcut_width: row
                        .shortcut
                        .as_ref()
                        .map_or(0., |t| fonts.measure_rich(t, style, None, true).width),
                };
                y += advance;
                result
            })
            .collect();
        MenuMetrics {
            height: y + padding,
            rows,
        }
    }
}
impl MenuBar {
    pub fn parse(source: &str) -> Result<Self, String> {
        check_limit(source)?;
        let mut parts = labels(source, true);
        if parts.len() > 10_000 {
            return Err("Menu bar exceeds 10,000 items".into());
        }
        if parts.len() == 1 {
            parts.push("...".into());
        }
        Ok(Self {
            texts: parts.iter().map(|s| RichText::parse(s, false)).collect(),
        })
    }
    pub fn measure(
        &self,
        fonts: &impl TextMeasurer,
        style: TextStyle,
        sketch: bool,
        icon: Option<f32>,
    ) -> BarMetrics {
        let measured: Vec<_> = self
            .texts
            .iter()
            .map(|s| fonts.measure_rich(s, style, None, true))
            .collect();
        BarMetrics {
            width: 18.
                + measured.iter().map(|m| m.width + 18.).sum::<f32>()
                + icon.map_or(0., |s| s + 18.),
            height: measured
                .iter()
                .map(|m| m.height)
                .fold(if sketch { 29. } else { 21. }, f32::max)
                .max(icon.unwrap_or(0.))
                + 3.,
            widths: measured.iter().map(|m| m.width).collect(),
        }
    }
}
impl BarMetrics {
    /// A skipped label consumes its leading inset; later labels may still fit.
    pub fn items(&self, width: f32, icon: Option<f32>) -> Vec<BarItem> {
        let mut x = 9. + icon.map_or(0., |s| s + 18.);
        self.widths
            .iter()
            .enumerate()
            .map(|(index, &w)| {
                let start = x;
                x += 9.;
                let visible = w <= width - x - 9.;
                if visible {
                    x += w + 9.;
                }
                BarItem {
                    index,
                    x: start,
                    width: w + 18.,
                    visible,
                }
            })
            .collect()
    }
}
pub fn apply_text(node: &mut Node, source: &str, baseline: &Properties) -> Result<(), String> {
    let bar = matches!(&node.content, NodeContent::Control {kind, ..} if kind == "menu_bar");
    if bar {
        MenuBar::parse(source)?;
    } else {
        Menu::parse(source)?;
    }
    let old = match baseline.get("text") {
        Some(Value::Text(s)) => s.as_str(),
        _ => "",
    };
    let before = labels(old, bar);
    let after = labels(source, bar);
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
    // Track duplicate occurrences rather than silently selecting the first copy.
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
