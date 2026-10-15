// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use serde::{Deserialize, Serialize};

/// Opaque project colors. Per-control overrides remain in node properties.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Palette {
    pub selection: [u8; 3],
    pub link: [u8; 3],
}
impl Default for Palette {
    fn default() -> Self {
        Self {
            selection: [149, 188, 242],
            link: [8, 83, 148],
        }
    }
}
impl Palette {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
    pub fn selection_rgba(self) -> [u8; 4] {
        [self.selection[0], self.selection[1], self.selection[2], 255]
    }
    pub fn link_rgba(self) -> [u8; 4] {
        [self.link[0], self.link[1], self.link[2], 255]
    }
    pub fn contrasting_ink(background: [u8; 3], opacity: f32, normal: [u8; 3]) -> [u8; 3] {
        if opacity >= 0.75
            && u32::from(background[0]) * 299
                + u32::from(background[1]) * 587
                + u32::from(background[2]) * 114
                < 153_000
        {
            [255; 3]
        } else {
            normal
        }
    }
}
