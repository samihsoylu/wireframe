// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Markup placement rules, independent of text measurement and drawing backends.
use crate::model::{Bounds, Kind};
use serde::{Deserialize, Serialize};
pub const DEFAULT_TEXT: &str = "A paragraph of text.\nA second row of text.";
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    HorizontalBrace,
    VerticalBrace,
    RedCross,
    ScratchOut,
}
impl Family {
    pub fn for_kind(kind: Kind) -> Option<Self> {
        match kind {
            Kind::HorizontalBrace => Some(Self::HorizontalBrace),
            Kind::VerticalBrace => Some(Self::VerticalBrace),
            Kind::RedCross => Some(Self::RedCross),
            Kind::ScratchOut => Some(Self::ScratchOut),
            _ => None,
        }
    }
    pub fn for_id(id: &str) -> Option<Self> {
        match id {
            "horizontal_brace" => Some(Self::HorizontalBrace),
            "vertical_brace" => Some(Self::VerticalBrace),
            "red_cross" => Some(Self::RedCross),
            "scratch_out" => Some(Self::ScratchOut),
            _ => None,
        }
    }
    pub fn default_size(self) -> (f32, f32) {
        match self {
            Self::HorizontalBrace => (200., 80.),
            Self::VerticalBrace => (180., 140.),
            Self::RedCross => (240., 104.),
            Self::ScratchOut => (205., 107.),
        }
    }
    pub fn brace(self) -> bool {
        matches!(self, Self::HorizontalBrace | Self::VerticalBrace)
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Layout {
    pub frame: Bounds,
    pub art: Bounds,
    pub label: Option<Bounds>,
    pub reverse: bool,
    pub horizontal: bool,
}
pub fn layout(family: Family, w: f32, h: f32, direction: &str) -> Layout {
    let horizontal = family == Family::HorizontalBrace;
    let reverse = if horizontal {
        direction == "bottom"
    } else {
        direction == "right"
    };
    let mut out = Layout {
        frame: Bounds::new(0., 0., w, h),
        art: Bounds::new(0., 0., w, h),
        label: None,
        reverse,
        horizontal,
    };
    if family.brace() {
        out.art = if horizontal {
            Bounds::new(0., if reverse { h - 15. } else { 0. }, w, 15.)
        } else {
            Bounds::new(if reverse { w - 15. } else { 0. }, 0., 15., h)
        };
        let label = if horizontal {
            Bounds::new(0., if reverse { 0. } else { 20. }, w, h - 20.)
        } else {
            Bounds::new(if reverse { 0. } else { 20. }, 0., w - 20., h)
        };
        if label.w > 0. && label.h > 0. {
            out.label = Some(label);
        }
    }
    out
}
pub fn cross_color(sketch: bool) -> [u8; 3] {
    if sketch { [119, 24, 34] } else { [207, 43, 40] }
}
