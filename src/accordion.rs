// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Source rows, selection and geometry for an accordion diagram.
use crate::{
    document::{Node, Properties, Value},
    layout::{TextMeasurer, TextStyle},
    model::Bounds,
    rich_text::RichText,
};

pub const DEFAULT: &str = "Overview\nPages\n- Landing\n- Settings\nAssets\nNotes";
pub const FIRST_CHILD_ERROR: &str = "ERROR: the 1st row must be top level  ";

#[derive(Clone, Debug)]
pub struct Row {
    pub source: String,
    pub text: RichText,
    pub parent: Option<usize>,
    pub children: usize,
    pub selected: bool,
    pub visible: bool,
}
#[derive(Clone, Debug)]
pub struct Accordion {
    pub rows: Vec<Row>,
    pub selected: Option<usize>,
}
#[derive(Clone, Copy, Debug)]
pub struct Appearance {
    pub left: f32,
    pub right: f32,
    pub overlap: f32,
    pub allowance: f32,
    pub scrollbar_right: f32,
}
impl Appearance {
    pub fn new(sketch: bool) -> Self {
        if sketch {
            Self {
                left: 6.,
                right: 3.,
                overlap: 3.,
                allowance: 78.,
                scrollbar_right: 2.,
            }
        } else {
            Self {
                left: 4.,
                right: 4.,
                overlap: 1.,
                allowance: 80.,
                scrollbar_right: 0.,
            }
        }
    }
}
pub struct Metrics {
    pub width: f32,
    pub height: f32,
    pub row_heights: Vec<f32>,
}
#[derive(Clone, Debug)]
pub struct Placement {
    pub index: usize,
    pub bounds: Bounds,
    pub link: Bounds,
}
pub struct Layout {
    pub rows: Vec<Placement>,
    pub scrollbar: Option<Bounds>,
}
impl Accordion {
    /// Raw grammar, without the control's empty-text default substitution.
    pub fn parse_source(source: &str, selected: i32) -> Result<Self, String> {
        if source.len() > 100_000 || source.split('\n').take(10_001).count() > 10_000 {
            return Err("Accordion exceeds the text or row limit".into());
        }
        let mut rows: Vec<Row> = Vec::new();
        let mut parent: Option<usize> = None;
        for (index, line) in source.split('\n').enumerate() {
            let child = line.starts_with("- ") || line.starts_with('\t');
            let label = line.strip_prefix("- ").unwrap_or(line);
            let label = label.strip_prefix('\t').unwrap_or(label);
            let label = if child && index == 0 {
                FIRST_CHILD_ERROR
            } else {
                label
            };
            let owner = if child && index > 0 { parent } else { None };
            if let Some(owner) = owner {
                rows[owner].children += 1;
            } else {
                parent = Some(index);
            }
            rows.push(Row {
                source: label.into(),
                text: RichText::parse(label, false),
                parent: owner,
                children: 0,
                selected: false,
                visible: owner.is_none(),
            });
        }
        let selected = usize::try_from(selected).ok().filter(|i| *i < rows.len());
        if let Some(index) = selected {
            let root = rows[index].parent.unwrap_or(index);
            rows[root].selected = true;
            rows[index].selected = true;
            for row in &mut rows {
                row.visible |= row.parent == Some(root);
            }
        }
        Ok(Self { rows, selected })
    }
    pub fn parse(source: &str, selected: i32) -> Result<Self, String> {
        Self::parse_source(if source.is_empty() { DEFAULT } else { source }, selected)
    }
    pub fn last_child(&self) -> usize {
        self.selected.map_or(self.rows.len() - 1, |i| {
            let root = self.rows[i].parent.unwrap_or(i);
            root + self.rows[root].children
        })
    }
    pub fn measure(&self, fonts: &impl TextMeasurer, style: TextStyle, sketch: bool) -> Metrics {
        let appearance = Appearance::new(sketch);
        let text: Vec<_> = self
            .rows
            .iter()
            .map(|r| fonts.measure_rich(&r.text, style, None, true))
            .collect();
        let width =
            text.iter().map(|m| m.width).fold(0., f32::max) + appearance.left + appearance.right;
        let visible = self.rows.iter().filter(|r| r.visible).count() as f32;
        Metrics {
            width: width.max(150.),
            height: (appearance.allowance + visible * (text[0].height + 17. - appearance.overlap))
                .max(186.),
            row_heights: text.iter().map(|m| m.height + 17.).collect(),
        }
    }
    pub fn layout(
        &self,
        metrics: &Metrics,
        width: f32,
        height: f32,
        sketch: bool,
        scrollbar: bool,
    ) -> Layout {
        let a = Appearance::new(sketch);
        let mut rows = Vec::new();
        let mut y = 0.;
        let mut remaining = height;
        let mut previous_height = metrics.row_heights[0];
        let mut scroll_y = None;
        let end = self.last_child();
        for index in 0..=end {
            let row = &self.rows[index];
            if !row.visible || previous_height - a.overlap > remaining {
                continue;
            }
            let h = metrics.row_heights[index];
            previous_height = h;
            let beside_scroll = scrollbar && scroll_y.is_some_and(|top| y + h >= top);
            rows.push(Placement {
                index,
                bounds: Bounds::new(0., y, width, h.min(remaining)),
                link: Bounds::new(
                    0.,
                    y,
                    (width - if beside_scroll { 16. } else { 0. }).max(0.),
                    (h - if beside_scroll { 3. } else { 1. }).max(0.),
                ),
            });
            y += h - a.overlap;
            remaining -= h - a.overlap;
            if scroll_y.is_none() && row.selected && row.parent.is_none() && remaining != 0. {
                scroll_y = Some(y + a.overlap - 1.);
            }
        }
        let lower_count = self.rows[end + 1..].iter().filter(|r| r.visible).count();
        let needed = if lower_count == 0 {
            0.
        } else {
            lower_count as f32 * metrics.row_heights[end + 1] - (lower_count - 1) as f32 * a.overlap
        };
        let scrollbar = scroll_y.filter(|_| scrollbar).and_then(|top| {
            let reserved = if remaining >= needed {
                needed - 1.
            } else if remaining > previous_height {
                remaining - 1.
            } else {
                0.
            };
            let h = height - top - reserved;
            (h > previous_height).then(|| {
                Bounds::new(
                    width - 16. - a.scrollbar_right,
                    top,
                    16.,
                    h - (top + h - height).max(0.),
                )
            })
        });
        if lower_count > 0 {
            if remaining > needed {
                y = height - needed;
            }
            previous_height = metrics.row_heights[end + 1];
            for index in end + 1..self.rows.len() {
                if remaining < previous_height - a.overlap {
                    break;
                }
                if !self.rows[index].visible {
                    continue;
                }
                let h = metrics.row_heights[index];
                previous_height = h;
                rows.push(Placement {
                    index,
                    bounds: Bounds::new(0., y, width, h.min(remaining)),
                    link: Bounds::new(0., y, width, h),
                });
                y += h - a.overlap;
                remaining -= h - a.overlap;
            }
        }
        Layout { rows, scrollbar }
    }
}
pub fn labels(source: &str) -> Vec<String> {
    Accordion::parse(source, -1)
        .map(|a| a.rows.into_iter().map(|r| r.source).collect())
        .unwrap_or_default()
}
pub fn apply_text(node: &mut Node, source: &str, baseline: &Properties) -> Result<(), String> {
    Accordion::parse(source, -1)?;
    let old = match baseline.get("text") {
        Some(Value::Text(s)) => s.as_str(),
        _ => DEFAULT,
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
        _ => 0,
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
