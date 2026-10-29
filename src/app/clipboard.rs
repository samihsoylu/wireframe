// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::document::{Id as ResourceId, Point};
mod cut;
mod transport;
pub(super) use transport::MAX_JSON_BYTES;
use transport::clipboard_text;
#[cfg(test)]
mod capacity_tests;
pub(super) use cut::{CutScope, PendingCut};

#[derive(Default)]
pub(super) struct PasteState {
    target: Option<(String, ResourceId, Vec<ResourceId>)>,
    offset: Point,
}

impl WireframeApp {
    pub(super) fn clipboard_origin(&self) -> String {
        if let Some(path) = &self.file {
            use sha2::Digest;
            let path = path.canonicalize().unwrap_or_else(|_| path.clone());
            let digest = sha2::Sha256::digest(path.as_os_str().as_encoded_bytes());
            format!(
                "file:{}",
                digest
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            )
        } else {
            format!("session:{}", self.editor.clipboard_session())
        }
    }
    pub(super) fn sync_paste_target(&mut self) {
        let resource = self.editor.library.unwrap_or_else(|| {
            self.editor
                .active_alternate(self.editor.page)
                .unwrap_or(self.editor.page)
        });
        let target = (
            self.clipboard_origin(),
            resource,
            self.editor.context.clone(),
        );
        if self.clipboard_paste.target.as_ref() != Some(&target) {
            self.clipboard_paste = PasteState {
                target: Some(target),
                offset: Point::default(),
            };
        }
    }
    pub(super) fn reset_paste_offset(&mut self, value: f32) {
        self.sync_paste_target();
        self.clipboard_paste.offset = Point::new(value, value);
    }
    pub(super) fn publish_clipboard(
        &mut self,
        ctx: &egui::Context,
        copy: Clipboard,
    ) -> Result<(), String> {
        let text = clipboard_text(&copy)?;
        self.pending_cut = None;
        ctx.copy_text(text);
        self.clipboard = copy;
        self.reset_paste_offset(10.);
        Ok(())
    }
    pub(super) fn copy_source_resource(
        &mut self,
        ctx: &egui::Context,
        source: ResourceId,
        cut: bool,
    ) -> Result<(), String> {
        self.copy_selected_resource(ctx, source_navigator::ResourceRow::Source(source), cut)
    }
    pub(super) fn copy_source_resources(
        &mut self,
        ctx: &egui::Context,
        sources: &[ResourceId],
        cut: bool,
    ) -> Result<(), String> {
        let copy = self
            .editor
            .copy_sources(sources, &self.clipboard_origin())?;
        if cut {
            return self.request_cut(ctx, copy, CutScope::Sources(sources.to_vec()));
        }
        self.publish_clipboard(ctx, copy)?;
        self.status = if sources.len() == 1 {
            "Copied component source".into()
        } else {
            format!("Copied {} component sources", sources.len())
        };
        Ok(())
    }
    pub(super) fn paste_clipboard(&mut self, copy: &Clipboard) {
        if copy.pages.is_some() {
            self.paste_page_resources(copy);
            return;
        }
        if copy.resources.is_some() {
            self.start_library_import(copy);
            return;
        }
        if !copy.has_sources() {
            self.editor.paste(copy, 20.);
            return;
        }
        self.sync_paste_target();
        let mut offset = self.clipboard_paste.offset;
        if self.canvas_rect.is_positive() && self.canvas_rect.is_finite() {
            let mut preview = Editor::new(Project {
                settings: self.editor.document.settings.clone(),
                font: self.editor.document.font.clone(),
                ..Project::default()
            });
            // Reuse import validation/remapping for layout; clipboard resource
            // IDs and page links need not belong to this temporary document.
            preview.paste_from(copy, 0., None);
            let bounds = preview.error.take().map_or(Ok(()), Err).and_then(|_| {
                preview
                    .layout_page(preview.page)?
                    .nodes
                    .iter()
                    .map(|n| n.bounds)
                    .reduce(|a, b| a.union(b))
                    .ok_or("Clipboard is empty".into())
            });
            let bounds = match bounds {
                Ok(b) => b,
                Err(error) => {
                    self.error = Some(error);
                    return;
                }
            };
            let mut visible = -self.pan / self.zoom;
            if let Some(definition) = self.editor.definition
                && let Some((library, source)) = self.editor.document.component_source(definition)
                && let Some(owner) = library.content.nodes.iter().find(|n| n.id == source.node)
            {
                visible += Vec2::new(owner.origin.x, owner.origin.y);
            }
            let size = self.canvas_rect.size() / self.zoom;
            let keep_visible = |position: f32, extent: f32, delta: f32, start: f32, length: f32| {
                let point = (position + delta).max(start).min(start + length - 40.);
                point.min(10_000. - extent) - position
            };
            offset.x = keep_visible(bounds.x, bounds.width, offset.x, visible.x, size.x);
            offset.y = keep_visible(bounds.y, bounds.height, offset.y, visible.y, size.y);
        }
        let previous_definition = self.editor.definition;
        let next_id = self.editor.document.next_id;
        let origin = self.clipboard_origin();
        self.editor.paste_from_at(copy, offset, Some(&origin));
        if self.editor.document.next_id != next_id {
            self.source_navigation.release_focus = !self.editor.selection.is_empty();
            self.clipboard_paste.offset = offset.translated(Point::new(10., 10.));
            if self.editor.definition != previous_definition {
                self.source_navigation.focus_to = self.editor.definition;
            }
        }
    }
    pub(super) fn copy(&mut self, ctx: &egui::Context) -> bool {
        let result = self
            .editor
            .copy_with_origin(&self.clipboard_origin())
            .and_then(|copy| self.publish_clipboard(ctx, copy));
        if let Err(error) = result {
            self.error = Some(error);
            return false;
        }
        self.status = format!("Copied {} controls", self.clipboard.len());
        true
    }
    pub(super) fn pasted(&mut self, s: &str) {
        if self.page_paste_request.is_some() {
            self.pasted_page_text(s);
            return;
        }
        if let Some(json) = s
            .strip_prefix("wireframe-nodes-v3:")
            .or_else(|| s.strip_prefix("wireframe-nodes-v2:"))
        {
            if json.len() > MAX_JSON_BYTES {
                self.error = Some("Clipboard content is too large".into());
                return;
            }
            match serde_json::from_str::<Clipboard>(json) {
                Ok(copy) => self.paste_clipboard(&copy),
                Err(e) => self.error = Some(format!("Cannot paste controls: {e}")),
            };
            return;
        }
        if let Some(json) = s.strip_prefix("wireframe-controls-v1:") {
            if json.len() > MAX_JSON_BYTES {
                self.error = Some("Clipboard content is too large".into());
                return;
            }
            if let Ok(controls) = serde_json::from_str::<Vec<Control>>(json) {
                if controls.len() <= 1000
                    && controls.iter().all(|c| {
                        c.bounds.valid()
                            && c.font_size.is_finite()
                            && (6.0..=120.0).contains(&c.font_size)
                            && c.text.len() <= 100_000
                    })
                {
                    self.editor.paste_legacy(&controls, 20.);
                } else {
                    self.error = Some("Clipboard contains invalid controls".into());
                }
            }
        }
    }
}
