// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Tab labels, intrinsic sizing and placement shared by layout and rendering.
use crate::{
    document::{Node, NodeContent, Properties, Value},
    layout::{TextMeasurer, TextStyle},
    model::Bounds,
    rich_text::RichText,
};
pub const HORIZONTAL: &str = "One, Two, Three, Four";
pub const VERTICAL: &str = "First Tab\nSecond Tab\nThird Tab\nFourth Tab";
pub fn labels(text: &str, vertical: bool) -> Vec<String> {
    if vertical {
        text.split('\n').map(str::to_owned).collect()
    } else {
        crate::navigation::split(text)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    Top,
    Bottom,
    Left,
    Right,
}
impl Position {
    pub fn vertical(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
    pub fn from_properties(p: &Properties, vertical: bool) -> Self {
        match (vertical, p.get("tab_position")) {
            (true, Some(Value::Choice(s))) if s == "right" => Self::Right,
            (true, _) => Self::Left,
            (false, Some(Value::Choice(s))) if s == "bottom" => Self::Bottom,
            _ => Self::Top,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Spacing {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub overlap: f32,
    pub inset_left: f32,
    pub inset_right: f32,
    pub join: f32,
    pub fill_inset: f32,
    pub scroll: [f32; 3],
}
impl Spacing {
    pub fn new(vertical: bool, sketch: bool) -> Self {
        match (vertical, sketch) {
            (false, false) => Self {
                left: 6.,
                right: 6.,
                top: 7.,
                bottom: 7.,
                overlap: 1.,
                inset_left: 20.,
                inset_right: 24.,
                join: 1.,
                fill_inset: 1.,
                scroll: [0., 0., 0.],
            },
            (false, true) => Self {
                left: 6.,
                right: 14.,
                top: 7.,
                bottom: 7.,
                overlap: 8.,
                inset_left: 20.,
                inset_right: 21.,
                join: 2.,
                fill_inset: 2.,
                scroll: [4., 1., 2.],
            },
            (true, false) => Self {
                left: 6.,
                right: 4.,
                top: 8.,
                bottom: 8.,
                overlap: 1.,
                inset_left: 0.,
                inset_right: 0.,
                join: 1.,
                fill_inset: 1.,
                scroll: [0., 0., 0.],
            },
            (true, true) => Self {
                left: 6.,
                right: 4.,
                top: 9.,
                bottom: 8.,
                overlap: 2.,
                inset_left: 0.,
                inset_right: 0.,
                join: 2.,
                fill_inset: 2.,
                scroll: [3., 1., 2.],
            },
        }
    }
}
pub struct Tabs {
    pub texts: Vec<RichText>,
    pub vertical: bool,
}
pub struct Metrics {
    pub width: f32,
    pub height: f32,
    pub text_sizes: Vec<(f32, f32)>,
    pub tab_sizes: Vec<(f32, f32)>,
    pub strip: (f32, f32),
    pub spacing: Spacing,
}
pub struct Placement {
    pub tabs: Vec<Bounds>,
    pub visible: Vec<bool>,
    pub content: Bounds,
}
impl Tabs {
    pub fn parse(text: &str, vertical: bool) -> Result<Self, String> {
        if text.len() > 100_000 {
            return Err("Tab text exceeds the text limit".into());
        }
        let labels = labels(text, vertical);
        if labels.len() > 10_001 {
            return Err("Tabs exceed 10,001 labels".into());
        }
        Ok(Self {
            texts: labels.iter().map(|s| RichText::parse(s, false)).collect(),
            vertical,
        })
    }
    pub fn measure(&self, fonts: &impl TextMeasurer, style: TextStyle, sketch: bool) -> Metrics {
        let s = Spacing::new(self.vertical, sketch);
        let text_sizes: Vec<_> = self
            .texts
            .iter()
            .map(|t| {
                let m = fonts.measure_rich(t, style, None, true);
                (m.width, m.height)
            })
            .collect();
        let max_w = text_sizes.iter().map(|m| m.0).fold(0., f32::max);
        let max_h = text_sizes.iter().map(|m| m.1).fold(0., f32::max);
        let tab_sizes: Vec<_> = text_sizes
            .iter()
            .map(|&(w, h)| {
                (
                    (if self.vertical { max_w } else { w }) + s.left + s.right,
                    (if self.vertical { h } else { max_h }) + s.top + s.bottom,
                )
            })
            .collect();
        let overlap = self.texts.len().saturating_sub(1) as f32 * s.overlap;
        let strip = if self.vertical {
            (
                max_w + s.left + s.right,
                tab_sizes.iter().map(|m| m.1).sum::<f32>() - overlap,
            )
        } else {
            (
                tab_sizes.iter().map(|m| m.0).sum::<f32>() - overlap,
                max_h + s.top + s.bottom,
            )
        };
        let (width, height) = if self.vertical {
            (
                200.,
                (62. + tab_sizes.iter().map(|m| m.1).sum::<f32>()).max(100.),
            )
        } else {
            ((s.inset_left + strip.0 + s.inset_right).max(150.), 100.)
        };
        Metrics {
            width,
            height,
            text_sizes,
            tab_sizes,
            strip,
            spacing: s,
        }
    }
}
impl Metrics {
    pub fn place(&self, b: Bounds, position: Position, align: &str) -> Placement {
        let s = self.spacing;
        let vertical = position.vertical();
        let mut x = match position {
            Position::Left => 0.,
            Position::Right => b.w - self.strip.0,
            _ => match align {
                "center" => s.inset_left + (b.w - s.inset_left - s.inset_right - self.strip.0) / 2.,
                "right" => b.w - s.inset_left - s.inset_right - self.strip.0,
                _ => s.inset_left,
            },
        };
        let mut y = if vertical {
            20.
        } else if position == Position::Bottom {
            b.h - self.strip.1
        } else {
            0.
        };
        let mut tabs = vec![];
        let mut visible = vec![];
        for &(w, h) in &self.tab_sizes {
            tabs.push(Bounds::new(b.x + x, b.y + y, w, h));
            visible.push(if vertical { y + h <= b.h } else { x + w <= b.w });
            if vertical {
                y += h - s.overlap;
            } else {
                x += w - s.overlap;
            }
        }
        let content = match position {
            Position::Top => Bounds::new(
                b.x,
                b.y + self.strip.1 - s.join,
                b.w,
                (b.h - self.strip.1 + s.join).max(0.),
            ),
            Position::Bottom => Bounds::new(b.x, b.y, b.w, (b.h - self.strip.1 + s.join).max(0.)),
            Position::Left => Bounds::new(
                b.x + self.strip.0 - s.join,
                b.y,
                (b.w - self.strip.0 + s.join).max(0.),
                b.h,
            ),
            Position::Right => Bounds::new(b.x, b.y, (b.w - self.strip.0 + s.join).max(0.), b.h),
        };
        Placement {
            tabs,
            visible,
            content,
        }
    }
}
pub fn apply_text(node: &mut Node, text: &str, baseline: &Properties) -> Result<(), String> {
    let vertical = matches!(&node.content,NodeContent::Control{kind,..} if kind=="vertical_tabs");
    Tabs::parse(text, vertical)?;
    let old = match baseline.get("text") {
        Some(Value::Text(s)) => s.as_str(),
        _ => "",
    };
    let before = labels(old, vertical);
    let after = labels(text, vertical);
    let selected = match baseline.get("selected_index") {
        Some(Value::Number(n)) => *n as i32,
        _ => -1,
    };
    let next = if selected < 0 {
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
    if let Some(Value::Links(links)) = baseline.get("row_links") {
        let a = before.iter().map(String::as_str).collect::<Vec<_>>();
        let b = after.iter().map(String::as_str).collect::<Vec<_>>();
        node.properties.insert(
            "row_links".into(),
            Value::Links(crate::list::remap_labels(&a, &b, links)),
        );
    }
    node.properties
        .insert("text".into(), Value::Text(text.into()));
    node.properties
        .insert("selected_index".into(), Value::Number(next as f64));
    Ok(())
}
