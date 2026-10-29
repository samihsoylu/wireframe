// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Read-only entrypoints: extraction never imports an asset or accepts a draft.
use super::*;
use std::{io::Read, path::Path};

fn read_screenshot(path: &Path) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path).map_err(|error| format!(
        "Couldn't open this screenshot. Check that you can open the file, then try again.\n\nDetails: {error}"
    ))?;
    if file
        .metadata()
        .is_ok_and(|m| m.len() > wireframe_ocr::MAX_BYTES as u64)
    {
        return Err("Choose a screenshot smaller than 32 MB".into());
    }
    let mut bytes = Vec::new();
    file.take(wireframe_ocr::MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            format!("Couldn't read this screenshot. Try another image.\n\nDetails: {error}")
        })?;
    if bytes.len() > wireframe_ocr::MAX_BYTES {
        return Err("Choose a screenshot smaller than 32 MB".into());
    }
    Ok(bytes)
}

impl WireframeApp {
    pub(super) fn can_extract_screenshot(&self) -> bool {
        screenshot_text::available()
            && !self.canvas_input_blocked()
            && self.text_edit.is_none()
            && self.drag.is_none()
            && self.library_drag.is_none()
            && self.library_press.is_none()
    }
    pub(super) fn extract_asset_text(&mut self, ctx: &egui::Context, asset: Id) {
        if !self.can_extract_screenshot() {
            return;
        }
        if let Some(asset) = self.editor.document.live_asset(asset) {
            self.screenshot_text = Some(screenshot_text::Review::new(ctx, asset.bytes.clone()));
        }
    }
    pub(super) fn choose_screenshot_text(&mut self, ctx: &egui::Context) {
        if !self.can_extract_screenshot() {
            return;
        }
        if let Some(path) = self.menus.with_dialog(|| {
            rfd::FileDialog::new()
                .add_filter("Screenshots", &["png", "jpg", "jpeg", "gif"])
                .pick_file()
        }) {
            match read_screenshot(&path) {
                Ok(bytes) => self.screenshot_text = Some(screenshot_text::Review::new(ctx, bytes)),
                Err(error) => self.error = Some(error),
            }
        }
    }
    pub(super) fn screenshot_text_dialog(&mut self, ctx: &egui::Context) {
        if !screenshot_text::available() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if self.error.is_none()
            && self
                .screenshot_text
                .as_mut()
                .is_some_and(|review| review.show(ctx))
        {
            self.screenshot_text = None;
        }
    }
}

#[cfg(test)]
mod tests;
