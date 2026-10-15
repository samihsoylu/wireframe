// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Geometry of a seven-panel image carousel diagram.
use crate::model::Bounds;
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
pub struct Layout {
    pub frame: Bounds,
    pub image: Bounds,
    pub scrollbar: Option<Bounds>,
}
pub fn layout(width: f32, height: f32, scrollbar: bool) -> Layout {
    let bar = (scrollbar && height > 16.).then(|| {
        let w = (width * 0.75).round();
        Bounds::new(((width - w) / 2.).round(), height - 16., w, 16.)
    });
    let h = if bar.is_some() { height - 16. } else { height };
    Layout {
        frame: Bounds::new(0., 0., width, h),
        image: Bounds::new(width * 0.345, h * 0.11, width * 0.31, h * 0.78),
        scrollbar: bar,
    }
}
