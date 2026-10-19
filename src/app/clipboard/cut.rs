// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

pub(in crate::app) enum CutScope {
    Controls,
    Sources(Vec<ResourceId>),
    Libraries(Vec<ResourceId>),
    Pages(Vec<ResourceId>),
}

#[derive(PartialEq)]
struct CutTarget {
    session: String,
    origin: String,
    page: ResourceId,
    alternate: Option<ResourceId>,
    library: Option<ResourceId>,
    definition: Option<ResourceId>,
    context: Vec<ResourceId>,
    selection: HashSet<ResourceId>,
    resource_selection: HashSet<source_navigator::ResourceRow>,
    page_selection: HashSet<ResourceId>,
}

pub(in crate::app) struct PendingCut {
    request: u64,
    document: Project,
    target: CutTarget,
    copy: Clipboard,
    scope: CutScope,
}

impl WireframeApp {
    pub(in crate::app) fn pending_cut_request(&self) -> Option<u64> {
        self.pending_cut.as_ref().map(|p| p.request)
    }

    fn cut_target(&self) -> CutTarget {
        CutTarget {
            session: self.editor.clipboard_session().into(),
            origin: self.clipboard_origin(),
            page: self.editor.page,
            alternate: self.editor.active_alternate(self.editor.page),
            library: self.editor.library,
            definition: self.editor.definition,
            context: self.editor.context.clone(),
            selection: self.editor.selection.clone(),
            resource_selection: self.source_navigation.selected.clone(),
            page_selection: self.navigator.selected.clone(),
        }
    }

    pub(in crate::app) fn request_cut(
        &mut self,
        ctx: &egui::Context,
        copy: Clipboard,
        scope: CutScope,
    ) -> Result<(), String> {
        let text = clipboard_text(&copy)?;
        self.end_pointer_gestures();
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let request = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // A newer request supersedes an earlier Cut. A late reply cannot remove
        // content on behalf of an old selection or a different document.
        self.pending_cut = Some(PendingCut {
            request,
            document: self.editor.document.clone(),
            target: self.cut_target(),
            copy,
            scope,
        });
        ctx.output_mut(|o| {
            o.commands
                .push(egui::OutputCommand::CopyTextWithResult { request, text })
        });
        self.status = "Copying to clipboard…".into();
        Ok(())
    }

    pub(in crate::app) fn finish_cut(
        &mut self,
        ctx: &egui::Context,
        request: u64,
        result: &Result<(), String>,
    ) {
        if self
            .pending_cut
            .as_ref()
            .is_none_or(|p| p.request != request)
        {
            return;
        }
        let pending = self.pending_cut.take().unwrap();
        if let Err(error) = result {
            self.error = Some(format!("Nothing was cut. {error}"));
            self.status = "Cut failed; content kept".into();
            return;
        }
        self.clipboard = pending.copy;
        self.reset_paste_offset(10.);
        if pending.document != self.editor.document
            || pending.target != self.cut_target()
            || self.canvas_input_blocked()
            || self.font_import.is_some()
            || self.present
            || self.image_crop.is_some()
            || self.library_drag.is_some()
            || self.library_press.is_some()
        {
            self.status = "Copied; the editing target changed, so nothing was cut".into();
            return;
        }
        let (result, status, resource) = match pending.scope {
            CutScope::Controls => {
                self.editor.delete();
                (
                    Ok(()),
                    format!("Cut {} controls", self.clipboard.len()),
                    false,
                )
            }
            CutScope::Sources(ids) => (
                self.editor.delete_definitions(&ids),
                format!("Cut {} component sources", ids.len()),
                true,
            ),
            CutScope::Libraries(ids) => (
                self.editor.cut_libraries(&ids),
                format!("Cut {} component libraries", ids.len()),
                true,
            ),
            CutScope::Pages(ids) => (
                self.editor.cut_pages(&ids),
                format!("Cut {} wireframes", ids.len()),
                false,
            ),
        };
        if let Err(error) = result {
            self.error = Some(error);
            self.status = "Copied; content could not be cut".into();
            return;
        }
        self.reset_paste_offset(0.);
        self.status = status;
        if resource {
            if let Some(id) = ctx.memory(|m| m.focused()) {
                ctx.memory_mut(|m| m.surrender_focus(id));
            }
            self.source_navigation = Default::default();
        }
    }

    pub(in crate::app) fn cut_controls(&mut self, ctx: &egui::Context) {
        let result = self
            .editor
            .copy_with_origin(&self.clipboard_origin())
            .and_then(|copy| self.request_cut(ctx, copy, CutScope::Controls));
        if let Err(error) = result {
            self.error = Some(error);
        }
    }
}

#[cfg(test)]
mod tests;
