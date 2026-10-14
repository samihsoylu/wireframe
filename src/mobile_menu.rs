// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Mobile menu content and measured row geometry, independent of drawing.
use crate::{
    document::{Node, Properties, Value},
    layout::{TextMeasurer, TextStyle},
    model::Bounds,
    rich_text::RichText,
};

pub const DEFAULT: &str = "Settings\n+ Add item, >\n- Remove item\nAccount, Personal\nv Selected item, (>)\n* Notification, =\n_ Icon space\n__ Picture space\nEnabled, ON\nDisabled, OFF\n\nMore options, >";
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Mark {
    Add,
    Remove,
    Check,
    Bullet,
    Space,
    LargeSpace,
    Chevron,
    CircleChevron,
    Grip,
    On,
    Off,
}
impl Mark {
    pub fn size(self) -> (f32, f32) {
        match self {
            Self::Bullet => (12., 12.),
            Self::LargeSpace => (48., 48.),
            Self::On | Self::Off => (49., 28.),
            _ => (16., 16.),
        }
    }
    pub fn drawn(self) -> bool {
        !matches!(self, Self::Space | Self::LargeSpace)
    }
}
#[derive(Debug)]
pub struct Row {
    pub main: Option<RichText>,
    pub caption: Option<RichText>,
    pub leading: Option<Mark>,
    pub trailing: Option<Mark>,
    pub separator: bool,
}
#[derive(Debug)]
pub struct Menu {
    pub rows: Vec<Row>,
}
#[derive(Debug, serde::Serialize)]
pub struct RowLayout {
    pub bounds: Bounds,
    pub main: Option<Bounds>,
    pub caption: Option<Bounds>,
    pub leading: Option<Bounds>,
    pub trailing: Option<Bounds>,
}
#[derive(Debug, serde::Serialize)]
pub struct Layout {
    pub width: f32,
    pub height: f32,
    pub rows: Vec<RowLayout>,
}
pub fn caption_style(style: TextStyle) -> TextStyle {
    TextStyle {
        size: style.size - 2.,
        bold: false,
        italic: false,
    }
}
fn parts(raw: &str) -> (String, Option<String>) {
    let mut left = String::new();
    let mut right = None::<String>;
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&',') {
            chars.next();
            right.as_mut().unwrap_or(&mut left).push(',');
        } else if c == ',' && right.is_none() {
            right = Some(String::new());
        } else {
            right.as_mut().unwrap_or(&mut left).push(c);
        }
    }
    (left, right)
}
impl Menu {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > 100_000 || text.split('\n').count() > 10_001 {
            return Err("Mobile menu exceeds the text limit".into());
        }
        let rows = text
            .split('\n')
            .map(|raw| {
                let mut content = raw;
                let mut leading = None;
                for (prefix, mark) in [
                    ("+ ", Mark::Add),
                    ("- ", Mark::Remove),
                    ("v ", Mark::Check),
                    ("* ", Mark::Bullet),
                    ("_ ", Mark::Space),
                    ("__ ", Mark::LargeSpace),
                ] {
                    if let Some(rest) = raw.strip_prefix(prefix) {
                        content = rest;
                        leading = Some(mark);
                        break;
                    }
                }
                let (main, mut caption) = parts(content);
                let mut trailing = None;
                if let Some(raw) = caption.as_mut() {
                    for (suffix, mark) in [
                        (" >", Mark::Chevron),
                        (" (>)", Mark::CircleChevron),
                        (" =", Mark::Grip),
                        (" ON", Mark::On),
                        (" OFF", Mark::Off),
                    ] {
                        if let Some(rest) = raw.strip_suffix(suffix) {
                            *raw = rest.trim_end_matches(' ').to_string();
                            trailing = Some(mark);
                            break;
                        }
                    }
                }
                let separator = main.chars().all(|c| c == '_');
                Row {
                    main: (!separator).then(|| RichText::parse(&main, false)),
                    caption: caption
                        .filter(|s| !s.is_empty())
                        .map(|s| RichText::parse(&s, false)),
                    leading,
                    trailing,
                    separator,
                }
            })
            .collect();
        Ok(Self { rows })
    }
    pub fn layout(
        &self,
        fonts: &impl TextMeasurer,
        style: TextStyle,
        width: Option<f32>,
    ) -> Layout {
        let width = width.unwrap_or(if style.size < 15. {
            215.
        } else {
            style.size * 14.5
        });
        let mut y = 5.;
        let mut rows = Vec::with_capacity(self.rows.len());
        for row in &self.rows {
            let main = row
                .main
                .as_ref()
                .map(|t| fonts.measure_rich(t, style, None, true));
            let caption = row
                .caption
                .as_ref()
                .map(|t| fonts.measure_rich(t, caption_style(style), None, true));
            let (lw, lh) = row.leading.map_or((0., 0.), Mark::size);
            let (rw, rh) = row.trailing.map_or((0., 0.), Mark::size);
            let height = if row.separator {
                29.
            } else {
                27_f32
                    .max(lh)
                    .max(rh)
                    .max(main.map_or(0., |m| m.height))
                    .max(caption.map_or(0., |m| m.height))
                    + 2.
            };
            let bounds = Bounds::new(0., y, width, height);
            let caption_width = caption.map_or(0., |m| m.width);
            let pad = if row.trailing.is_some() {
                (width - rw).clamp(0., 6.)
            } else if caption.is_some() {
                (width - caption_width).clamp(0., 6.)
            } else {
                6.
            };
            let mut available = (width - pad).max(1.);
            let trailing = row
                .trailing
                .filter(|m| m.drawn() && rw <= available)
                .map(|_| Bounds::new(available - rw, y + (height - rh) / 2., rw, rh));
            if row.trailing.is_some() {
                available = (available - rw - 4.).max(1.);
            }
            let caption = caption.map(|_| Bounds::new(0., y, available, height));
            // A trailing symbol shares the caption's available area with the main label.
            if caption.is_some() && row.trailing.is_none() {
                available = (available - caption_width).max(1.);
            }
            let mut x = 8.;
            available = (available - 8.).max(1.);
            let leading = row
                .leading
                .filter(|m| m.drawn() && lw <= width - 8.)
                .map(|_| Bounds::new(x, y + (height - lh) / 2., lw, lh));
            if row.leading.is_some() {
                let pad = lw
                    + 4.
                    + if row.leading == Some(Mark::Bullet) {
                        4.
                    } else {
                        0.
                    };
                x += pad;
                available = (available - pad).max(1.);
            }
            let main = row
                .main
                .as_ref()
                .filter(|_| available > 1.)
                .map(|_| Bounds::new(x, y, available, height));
            rows.push(RowLayout {
                bounds,
                main,
                caption,
                leading,
                trailing,
            });
            y += height;
        }
        Layout {
            width,
            height: y + 5.,
            rows,
        }
    }
}
pub fn apply_text(node: &mut Node, text: &str, baseline: &Properties) -> Result<(), String> {
    Menu::parse(text)?;
    let old = match baseline.get("text") {
        Some(Value::Text(s)) => s.as_str(),
        _ => "",
    };
    if let Some(Value::Links(links)) = baseline.get("row_links") {
        let old: Vec<_> = old.split('\n').collect();
        let new: Vec<_> = text.split('\n').collect();
        node.properties.insert(
            "row_links".into(),
            Value::Links(crate::list::remap_labels(&old, &new, links)),
        );
    }
    node.properties
        .insert("text".into(), Value::Text(text.into()));
    Ok(())
}
