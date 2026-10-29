// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use more_controls::{review, view};

pub(super) struct Request {
    document: Project,
    session: String,
    selected: Vec<Id>,
    collapsed: HashSet<Id>,
}

pub(super) struct Review {
    copy: Clipboard,
    decisions: review::Review,
    incoming: Editor,
    selected: Vec<Id>,
    collapsed: HashSet<Id>,
    session: String,
    origin: String,
}

impl WireframeApp {
    pub(super) fn request_page_paste(&mut self, ctx: &egui::Context, selected: &[Id]) {
        if !self.finish_page_notes() {
            return;
        }
        self.end_pointer_gestures();
        self.page_paste_request = Some(Request {
            document: self.editor.document.clone(),
            session: self.editor.clipboard_session().into(),
            selected: selected.to_vec(),
            collapsed: self.navigator.collapsed.clone(),
        });
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
    }

    pub(super) fn pasted_page_text(&mut self, text: &str) {
        let request = self.page_paste_request.take();
        if request.as_ref().is_some_and(|r| {
            r.document != self.editor.document || r.session != self.editor.clipboard_session()
        }) {
            self.error = Some("The page paste target changed. Paste again.".into());
            return;
        }
        let Some(json) = text
            .strip_prefix("wireframe-nodes-v3:")
            .or_else(|| text.strip_prefix("wireframe-nodes-v2:"))
        else {
            return;
        };
        if json.len() > clipboard::MAX_JSON_BYTES {
            self.error = Some("Clipboard content is too large".into());
            return;
        }
        match serde_json::from_str::<Clipboard>(json) {
            Ok(copy) if copy.pages.is_some() => self.paste_page_resources_at(&copy, request),
            Ok(_) => self.status = "Select the canvas to paste controls".into(),
            Err(error) => self.error = Some(format!("Cannot paste wireframes: {error}")),
        }
    }
    pub(super) fn copy_page_resources(&mut self, ctx: &egui::Context, ids: &[Id], cut: bool) {
        if !self.finish_page_notes() {
            return;
        }
        self.end_pointer_gestures();
        let result = self
            .editor
            .copy_pages(ids, &self.clipboard_origin())
            .and_then(|copy| {
                if cut {
                    self.request_cut(ctx, copy, clipboard::CutScope::Pages(ids.to_vec()))
                } else {
                    self.publish_clipboard(ctx, copy)?;
                    self.status = format!("Copied {} wireframes", ids.len());
                    Ok(())
                }
            });
        if let Err(error) = result {
            self.error = Some(error);
        }
    }

    pub(super) fn paste_page_resources(&mut self, copy: &Clipboard) {
        self.paste_page_resources_at(copy, None);
    }
    fn paste_page_resources_at(&mut self, copy: &Clipboard, request: Option<Request>) {
        if self.canvas_input_blocked() || !self.finish_page_notes() {
            return;
        }
        self.end_pointer_gestures();
        let tree = wireframe::page_tree::PageTree::new(&self.editor.document);
        self.navigator.sync(
            &tree,
            self.editor.page,
            self.editor.duplicated_page_selection(),
        );
        let selected: Vec<_> = tree
            .rows
            .iter()
            .filter(|r| self.navigator.selected.contains(&r.id))
            .map(|r| r.id)
            .collect();
        let (selected, collapsed) = request.map_or_else(
            || (selected, self.navigator.collapsed.clone()),
            |r| (r.selected, r.collapsed),
        );
        let origin = self.clipboard_origin();
        let result = if copy.pages_from_same_project(&self.editor, &origin) {
            self.editor
                .prepare_same_project_page_paste(copy, &origin, &selected, &collapsed)
                .and_then(|p| self.editor.apply_page_paste(p))
                .map(|ids| {
                    self.status = format!("Pasted {} wireframes", ids.len());
                })
        } else {
            self.editor
                .plan_page_clipboard_import(copy)
                .and_then(|plan| {
                    let mut decisions = review::Review::from_plan(plan);
                    if decisions.prepared.is_some() {
                        let prepared = decisions.take_ready(&self.editor)?;
                        let prepared = self.editor.position_page_clipboard_import(
                            copy, prepared, &selected, &collapsed,
                        )?;
                        let ids = self.editor.apply_page_paste(prepared)?;
                        self.status = format!("Pasted {} wireframes", ids.len());
                    } else {
                        self.page_paste = Some(Review {
                            copy: copy.clone(),
                            decisions,
                            incoming: Editor::new(copy.page_payload()?.project().clone()),
                            selected,
                            collapsed,
                            session: self.editor.clipboard_session().into(),
                            origin,
                        });
                    }
                    Ok(())
                })
        };
        if let Err(error) = result {
            self.error = Some(error);
        }
    }

    pub(super) fn page_paste_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut paste) = self.page_paste.take() else {
            return;
        };
        let valid = paste.session == self.editor.clipboard_session()
            && paste.origin == self.clipboard_origin();
        if !valid {
            self.error = Some("The destination changed. Paste the pages again.".into());
            return;
        }
        let mut action = view::Action::None;
        let response = egui::Modal::new(egui::Id::new("paste-wireframes")).show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 64.).clamp(280., 1000.));
            let height = (ctx.content_rect().height() - 100.).max(160.);
            egui::ScrollArea::vertical()
                // Modal areas remember their previous size. Let the comparison
                // grow after a short summary instead of clipping its actions.
                // Auto-shrink still keeps short reviews compact.
                .min_scrolled_height(height)
                .max_height(height)
                .show(ui, |ui| {
                    view::review_content(
                        ui,
                        &mut paste.decisions,
                        &self.editor,
                        &paste.incoming,
                        "Paste wireframes",
                        "Paste wireframes",
                        &mut action,
                    );
                });
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                action = view::Action::CancelReview;
            }
        });
        if response.should_close() || matches!(action, view::Action::CancelReview) {
            return;
        }
        match action {
            view::Action::Choose(choice) => paste.decisions.choose(choice),
            view::Action::Review => match self.editor.plan_page_clipboard_import(&paste.copy) {
                Ok(plan) => paste.decisions = review::Review::from_plan(plan),
                Err(error) => paste.decisions.error = Some(error),
            },
            view::Action::Apply => {
                let result = paste
                    .decisions
                    .take_ready(&self.editor)
                    .and_then(|prepared| {
                        self.editor.position_page_clipboard_import(
                            &paste.copy,
                            prepared,
                            &paste.selected,
                            &paste.collapsed,
                        )
                    })
                    .and_then(|prepared| self.editor.apply_page_paste(prepared));
                match result {
                    Ok(ids) => {
                        self.status = format!("Pasted {} wireframes", ids.len());
                        return;
                    }
                    Err(error) => paste.decisions.error = Some(error),
                }
            }
            _ => {}
        }
        self.page_paste = Some(paste);
    }
}

#[cfg(test)]
mod tests;
