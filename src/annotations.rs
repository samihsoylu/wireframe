// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Annotation measurement, placement and contrast, independent of rendering widgets.
use crate::{
    layout::TextMetrics,
    model::{Bounds, Kind},
    rich_text::RichText,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    Callout,
    StickyNote,
    Tooltip,
}
impl Family {
    pub fn for_kind(kind: Kind) -> Option<Self> {
        match kind {
            Kind::Callout => Some(Self::Callout),
            Kind::StickyNote => Some(Self::StickyNote),
            Kind::Tooltip => Some(Self::Tooltip),
            _ => None,
        }
    }
    pub fn for_id(id: &str) -> Option<Self> {
        match id {
            "callout" => Some(Self::Callout),
            "sticky_note" => Some(Self::StickyNote),
            "tooltip" => Some(Self::Tooltip),
            _ => None,
        }
    }
    pub fn parse(self, source: &str, page_title: &str) -> RichText {
        if self == Self::Tooltip {
            return RichText::parse(source, false);
        }
        // Insert a name as literal content so punctuation cannot introduce formatting.
        let literal = page_title
            .replace('\\', "\\\\")
            .replace('_', "\\_")
            .replace('*', "\\*")
            .replace('[', "\\[");
        RichText::parse(&source.replace("{mockup-name}", &literal), true)
    }
    pub fn intrinsic(self, text: TextMetrics, sketch: bool) -> (f32, f32) {
        match self {
            Self::Callout => ((text.width + 26.).max(39.), (text.height + 15.).max(39.)),
            Self::StickyNote => (109., 123.),
            Self::Tooltip => (
                text.width + if sketch { 40. } else { 34. },
                text.height.max(33.),
            ),
        }
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
impl Direction {
    pub fn from_value(s: &str) -> Self {
        match s {
            "top_right" => Self::TopRight,
            "bottom_left" => Self::BottomLeft,
            "bottom_right" => Self::BottomRight,
            _ => Self::TopLeft,
        }
    }
    pub fn right(self) -> bool {
        matches!(self, Self::TopRight | Self::BottomRight)
    }
    pub fn bottom(self) -> bool {
        matches!(self, Self::BottomLeft | Self::BottomRight)
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Layout {
    pub frame: Bounds,
    pub label: Option<Bounds>,
    pub tape: Option<Bounds>,
    pub pointer: Option<Bounds>,
    pub mirror: [bool; 2],
}
pub fn layout(
    family: Family,
    w: f32,
    h: f32,
    sketch: bool,
    text: TextMetrics,
    left: bool,
    direction: Direction,
) -> Layout {
    let mut out = Layout {
        frame: Bounds::new(0., 0., w, h),
        label: None,
        tape: None,
        pointer: None,
        mirror: [false; 2],
    };
    match family {
        Family::Callout => {
            out.label = Some(Bounds::new(14., 9., (w - 26.).max(1.), (h - 15.).max(1.)));
        }
        Family::StickyNote => {
            let top = if sketch { 31. } else { 13. };
            let right = if sketch { 14. } else { 10. };
            if sketch {
                let tw = (w / 4.).max(47.);
                out.tape = Some(Bounds::new(((w - tw) / 2. + 0.5).floor(), 0., tw, 27.));
                out.frame = Bounds::new(0., 14., w.max(1.), (h - 14.).max(1.));
            }
            let tw = (w - 10. - right).min(text.width);
            let th = h - top - 10.;
            // Right alignment is inside the centered, content-sized box. Left alone is edge anchored.
            if tw > 0. && th > 0. {
                out.label = Some(Bounds::new(
                    if left { 10. } else { (w - tw) / 2. },
                    top,
                    tw,
                    th,
                ));
            }
        }
        Family::Tooltip => {
            let y = if direction.bottom() { 0. } else { 7. };
            out.frame = Bounds::new(0., y, w, h - 7.);
            let tw = w - if sketch { 30. } else { 24. };
            if tw > 0. {
                out.label = Some(Bounds::new(
                    12.,
                    y + if sketch { 2. } else { 1. },
                    tw,
                    h - 7.,
                ));
            }
            if w >= 32. && h >= 28. {
                out.pointer = Some(Bounds::new(
                    if direction.right() { w - 8. } else { 8. },
                    if direction.bottom() { h } else { 0. },
                    16.,
                    14.,
                ));
            }
            out.mirror = [direction.right(), direction.bottom()];
        }
    }
    out
}
pub fn light_text(family: Family, fill: [u8; 3], opacity: f32, sketch: bool) -> bool {
    let offset = if family == Family::StickyNote && sketch {
        90
    } else {
        0
    };
    let c = fill.map(|v| v.saturating_add(offset));
    crate::field_set::light_text(
        c,
        if family == Family::StickyNote {
            1.
        } else {
            opacity
        },
    )
}
