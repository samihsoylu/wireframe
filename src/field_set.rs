// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Label and border geometry for a form section.
use crate::{layout::TextMetrics, model::Bounds, rich_text::RichText};
pub const DEFAULT: &str = "Group Name";
pub fn parse(source: &str) -> Result<RichText, String> {
    if source.len() > 100_000 {
        return Err("Field set label exceeds the text limit".into());
    }
    Ok(RichText::parse(source, false))
}
pub fn intrinsic(label: TextMetrics) -> (f32, f32) {
    ((label.width + 30.).max(200.), 170.)
}
#[derive(Debug)]
pub struct Layout {
    pub border: Bounds,
    pub label: Bounds,
    pub cutout: Bounds,
}
pub fn layout(width: f32, height: f32, label: TextMetrics) -> Layout {
    let top = label.height / 2.;
    Layout {
        border: Bounds::new(0., top, width, (height - top).max(1.)),
        label: Bounds::new(15., 0., label.width, label.height),
        cutout: Bounds::new(12., 0., label.width + 6., label.height),
    }
}
pub fn light_text(fill: [u8; 3], opacity: f32) -> bool {
    opacity >= 0.75
        && u32::from(fill[0]) * 299 + u32::from(fill[1]) * 587 + u32::from(fill[2]) * 114 < 153_000
}
