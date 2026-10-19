// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! One memory budget for image controls and custom icon variants in a scene.
use crate::{
    document::{Asset, Id},
    images::Decoded,
};
use std::{collections::HashMap, sync::Arc};
#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub(super) enum Style {
    Image { turns: u8, sketch: bool },
    Icon([u8; 3], u8),
}
pub(super) struct Rasters<'a> {
    assets: &'a [Asset],
    cache: HashMap<(Id, Style), Option<Arc<Decoded>>>,
    bytes: usize,
}
impl<'a> Rasters<'a> {
    pub fn new(assets: &'a [Asset]) -> Self {
        Self {
            assets,
            cache: HashMap::new(),
            bytes: 0,
        }
    }
    pub fn get(&mut self, id: Id, style: Style) -> Option<Arc<Decoded>> {
        self.cache
            .entry((id, style))
            .or_insert_with(|| {
                let asset = self.assets.iter().find(|a| a.id == id && !a.trashed)?;
                let (w, h) = crate::images::dimensions(&asset.bytes).ok()?;
                let pad = if matches!(style, Style::Image { sketch: true, .. }) {
                    4
                } else {
                    0
                };
                let bytes = (w as usize + pad) * (h as usize + pad) * 4;
                if self.bytes + bytes > 128 * 1024 * 1024 {
                    return None;
                }
                let decoded = match style {
                    Style::Image { turns, sketch } => {
                        crate::images::decoded_styled(&asset.bytes, turns, sketch)
                    }
                    Style::Icon(color, turns) => {
                        crate::images::decoded_tinted_rotated(&asset.bytes, color, turns)
                    }
                }
                .ok()?;
                self.bytes += bytes;
                Some(Arc::new(decoded))
            })
            .clone()
    }
}
