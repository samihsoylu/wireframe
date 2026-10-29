// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
mod discovery;
use super::{Pending, WireframeApp};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use wireframe::{
    project_session::{ProjectSession, RequestId, SessionEvent},
    storage::{self, ProjectStore},
    working_copy::{CopySummary, NamedStatus, WorkingCopies, WorkingCopy},
    workspace::Editor,
};

const SAVING_STOPPED_STATUS: &str = "Saving stopped · keep this document open";
const SAVING_STOPPED_GUIDANCE: &str = "Saving stopped. Keep this document open. Use File → Save as… to try saving a separate copy. If saving still fails, check available disk space and access.";

#[derive(Serialize, Deserialize)]
struct Settings {
    automatic: bool,
}
struct SaveAction {
    request: RequestId,
    after: Option<Pending>,
}
pub(super) struct Persistence {
    copies: WorkingCopies,
    settings: PathBuf,
    automatic: bool,
    ctx: egui::Context,
    session: Option<ProjectSession>,
    id: Option<String>,
    busy: Option<SaveAction>,
    offers: Vec<CopySummary>,
    discovery: Option<discovery::Scan>,
    discovery_errors: Vec<String>,
    chooser: bool,
    failed: bool,
}
pub(super) struct PreparedOpen {
    pub copy: WorkingCopy,
    pub file: Option<PathBuf>,
    pub editor: Editor,
}
impl PreparedOpen {
    pub fn read(copies: &WorkingCopies, path: &std::path::Path) -> Result<Self, String> {
        let mut copy = copies.open_named(path).map_err(|e| e.to_string())?;
        let mut file = copy.named_path().map_err(|e| e.to_string())?;
        match copy.named_status().map_err(|e| e.to_string())? {
            NamedStatus::Changed(_) => copy.reload_named(copies).map_err(|e| e.to_string())?,
            NamedStatus::Missing | NamedStatus::Unreadable(_) => {
                copy = copies
                    .create_unnamed(copy.project())
                    .map_err(|e| e.to_string())?;
                file = None;
            }
            NamedStatus::Unchanged => {}
        }
        let saved = file.as_ref().map(|p| ProjectStore.load(p)).transpose()?;
        let mut editor = super::startup::prepare_editor(copy.project().clone())?;
        if let Some(named) = &saved {
            editor.mark_saved_snapshot(named);
        } else {
            editor.mark_unsaved();
        }
        Ok(Self { copy, file, editor })
    }
}
pub(super) struct OpenSource {
    copies: WorkingCopies,
    session: Option<ProjectSession>,
    id: Option<String>,
    document: wireframe::document::Project,
}
impl OpenSource {
    pub fn finish(mut self, discard: bool) -> Result<(), String> {
        if let Some(mut session) = self.session.take() {
            session.observe(&self.document)?;
            if discard {
                session.discard()
            } else {
                session.finish()
            }
        } else if discard && let Some(id) = self.id {
            self.copies
                .resume(&id)
                .and_then(|copy| copy.discard())
                .map_err(|e| e.to_string())
        } else {
            Ok(())
        }
    }
}
impl WireframeApp {
    pub(super) fn version_preferences_key(&self) -> Option<String> {
        if let Some(persistence) = &self.persistence {
            persistence.id.clone()
        } else {
            self.file.as_ref().map(|path| {
                format!(
                    "file:{}",
                    path.canonicalize()
                        .unwrap_or_else(|_| path.clone())
                        .display()
                )
            })
        }
    }

    pub(super) fn export_preferences_key(&mut self) -> Result<Option<String>, String> {
        if self.persistence.is_some() {
            self.ensure_session()?;
            Ok(self.persistence.as_ref().and_then(|p| p.id.clone()))
        } else {
            Ok(self.file.as_ref().map(|path| {
                format!(
                    "file:{}",
                    path.canonicalize()
                        .unwrap_or_else(|_| path.clone())
                        .display()
                )
            }))
        }
    }
    pub(super) fn enable_persistence(&mut self, ctx: &egui::Context, root: PathBuf) {
        self.load_version_preferences(&root);
        self.load_viewport_preferences(&root);
        self.load_panel_preferences(&root);
        if let Err(error) = self.presentation_view.preferences.load(&root) {
            self.error = Some(format!(
                "Presentation settings could not be read. The file is unchanged.\n{error}"
            ));
        }
        let result = (|| {
            let copies =
                WorkingCopies::new(root.join("working-copies")).map_err(|e| e.to_string())?;
            let settings = root.join("save-settings.json");
            let automatic = match std::fs::read(&settings) {
                Ok(bytes) => match serde_json::from_slice::<Settings>(&bytes) {
                    Ok(s) => s.automatic,
                    Err(e) => {
                        self.error = Some(format!(
                            "Save settings couldn't be read. Automatic saving is off until you choose it in File. The settings file is unchanged.\n{e}"
                        ));
                        false
                    }
                },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
                Err(e) => {
                    self.error = Some(format!(
                        "Save settings couldn't be read. Automatic saving is off.\n{e}"
                    ));
                    false
                }
            };
            Ok::<_, String>(Persistence {
                copies,
                settings,
                automatic,
                ctx: ctx.clone(),
                session: None,
                id: None,
                busy: None,
                offers: vec![],
                discovery: None,
                discovery_errors: vec![],
                chooser: false,
                failed: false,
            })
        })();
        match result {
            Ok(p) => {
                self.persistence = Some(p);
                self.refresh_recoveries(true);
            }
            Err(e) => {
                self.error = Some(format!(
                    "Per-project recovery is unavailable. Save your work manually.\n{e}"
                ))
            }
        }
    }
    fn start_copy(&mut self, copy: WorkingCopy) -> Result<(), String> {
        let p = self.persistence.as_mut().unwrap();
        let id = copy.id().to_owned();
        let ctx = p.ctx.clone();
        let session = ProjectSession::start(
            p.copies.clone(),
            copy,
            p.automatic,
            Arc::new(move || ctx.request_repaint()),
        )?;
        p.session = Some(session);
        p.id = Some(id);
        p.failed = false;
        Ok(())
    }
    fn ensure_session(&mut self) -> Result<(), String> {
        let p = self.persistence.as_ref().unwrap();
        if p.failed {
            return Err(SAVING_STOPPED_GUIDANCE.into());
        }
        if p.session.is_some() {
            return Ok(());
        }
        let copy = if let Some(id) = &p.id {
            p.copies.resume(id)
        } else if let Some(path) = &self.file {
            p.copies.open_named(path)
        } else {
            p.copies.create_unnamed(&self.editor.document)
        }
        .map_err(|e| e.to_string())?;
        self.start_copy(copy)
    }
    pub(super) fn save_busy(&self) -> bool {
        self.persistence.as_ref().is_some_and(|p| p.busy.is_some())
    }
    pub(super) fn preserve_save_navigator_focus(&mut self, ctx: &egui::Context) {
        if self.save_busy() {
            // Disabled rows relinquish focus while the worker saves the file.
            if self.save_navigator_focus.is_none()
                && (self.navigator.focused(ctx) || self.source_navigation_focused(ctx))
            {
                self.save_navigator_focus = ctx.memory(|m| m.focused());
            }
        } else if let Some(id) = self.save_navigator_focus.take()
            && !self.canvas_input_blocked()
            && ctx.memory(|m| m.focused()).is_none_or(|owner| owner == id)
        {
            // Restore before dispatching the next key. The previous save modal
            // still blocks widgets for this pass, so the row also requests focus
            // after construction, just like other navigator selection changes.
            if self.editor.in_source_workspace() {
                if !self.restore_source_navigator_focus(id) {
                    return;
                }
            } else {
                self.navigator.focus_to = Some(self.editor.page);
            }
            ctx.memory_mut(|m| m.request_focus(id));
        }
    }
    pub(super) fn recovery_chooser_open(&self) -> bool {
        self.persistence.as_ref().is_some_and(|p| p.chooser)
    }
    pub(super) fn automatic_named(&self) -> bool {
        self.file.is_some() && self.persistence.as_ref().is_some_and(|p| p.automatic)
    }
    pub(super) fn status_text(&self) -> &str {
        if self.persistence.as_ref().is_some_and(|p| p.failed) {
            SAVING_STOPPED_STATUS
        } else if let Some(status) = self.presentation_save_status() {
            status
        } else {
            &self.status
        }
    }
    fn show_stopped_saving(&mut self, error: impl std::fmt::Display) {
        self.status = SAVING_STOPPED_STATUS.into();
        self.error = Some(format!("{SAVING_STOPPED_GUIDANCE}\n\nDetails: {error}"));
    }
    pub(super) fn observe_project(&mut self) {
        if self.quit_allowed || self.persistence.is_none() || self.startup_open.is_some() {
            return;
        }
        if self.persistence.as_ref().unwrap().session.is_none() && !self.editor.dirty() {
            return;
        }
        if self.persistence.as_ref().unwrap().failed {
            return;
        }
        let result = self.ensure_session().and_then(|()| {
            self.persistence
                .as_mut()
                .unwrap()
                .session
                .as_mut()
                .unwrap()
                .observe(&self.editor.document)
        });
        if let Err(e) = result {
            self.persistence.as_mut().unwrap().failed = true;
            self.show_stopped_saving(e);
        }
    }
    /// Queue a save. A selected Save As path is the caller's overwrite consent.
    pub(super) fn queue_save(&mut self, path: PathBuf, choose: bool) -> bool {
        if self.save_busy() {
            return false;
        }
        self.end_pointer_gestures();
        // A failed worker may be replaced only by an explicit Save As. Preserve
        // its old record and create a fresh source from the visible document.
        if choose && self.persistence.as_ref().unwrap().failed {
            let p = self.persistence.as_mut().unwrap();
            if let Some(session) = p.session.take() {
                let _ = session.finish();
            }
            match p.copies.create_unnamed(&self.editor.document) {
                Ok(copy) => {
                    if let Err(e) = self.start_copy(copy) {
                        self.show_stopped_saving(e);
                        return false;
                    }
                }
                Err(e) => {
                    self.show_stopped_saving(e);
                    return false;
                }
            }
        }
        let result = self.ensure_session().and_then(|()| {
            let session = self.persistence.as_mut().unwrap().session.as_mut().unwrap();
            if choose || self.file.is_none() {
                session.save_as(path, &self.editor.document)
            } else {
                session.flush(&self.editor.document)
            }
        });
        match result {
            Ok(request) => {
                self.persistence.as_mut().unwrap().busy = Some(SaveAction {
                    request,
                    after: self.pending.take(),
                });
                self.status = "Saving…".into();
                true
            }
            Err(e) => {
                self.error = Some(e);
                false
            }
        }
    }
    pub(super) fn poll_saves(&mut self, ctx: &egui::Context) {
        loop {
            let Some(p) = &self.persistence else {
                return;
            };
            if p.failed {
                return;
            }
            let Some(session) = &p.session else {
                return;
            };
            let event = match session.try_event() {
                Ok(Some(e)) => e,
                Ok(None) => return,
                Err(e) => {
                    let p = self.persistence.as_mut().unwrap();
                    p.failed = true;
                    if let Some(action) = p.busy.take() {
                        self.pending = action.after;
                    }
                    self.show_stopped_saving(e);
                    return;
                }
            };
            match event {
                SessionEvent::Checkpointed { copy_id, snapshot } => {
                    self.persistence.as_mut().unwrap().id = Some(copy_id);
                    if self.recovery_offer.is_none() {
                        self.clear_recovery();
                    }
                    if !self.save_busy() && self.editor.dirty() {
                        self.status = if snapshot.as_ref() != &self.editor.document {
                            "Local copy saved · newer edits pending"
                        } else if self.automatic_named() {
                            "Local copy saved · saving file soon"
                        } else {
                            "Local copy saved · file has unsaved changes"
                        }
                        .into();
                    }
                }
                SessionEvent::Saved { snapshot, path } => {
                    self.editor.mark_saved_snapshot(&snapshot);
                    self.file = Some(path);
                    self.status = if self.editor.dirty() {
                        "Saved · newer edits pending"
                    } else {
                        "Saved to your computer"
                    }
                    .into();
                }
                SessionEvent::Problem(error) => {
                    self.status = "Saving needs attention · keep this document open".into();
                    self.error = Some(error);
                }
                SessionEvent::RequestFinished { request, result } => {
                    if self
                        .persistence
                        .as_ref()
                        .unwrap()
                        .busy
                        .as_ref()
                        .is_none_or(|a| a.request != request)
                    {
                        continue;
                    }
                    let action = self.persistence.as_mut().unwrap().busy.take().unwrap();
                    match result {
                        Ok(receipt) => {
                            self.persistence.as_mut().unwrap().id = Some(receipt.copy_id);
                            self.file = Some(receipt.path);
                            self.editor.mark_saved_snapshot(&receipt.snapshot);
                            // A clean flush can finish without a Saved event.
                            self.status = if self.editor.dirty() {
                                "Saved · newer edits pending"
                            } else {
                                "Saved to your computer"
                            }
                            .into();
                            if let Some(warning) = receipt.warning {
                                self.error = Some(warning);
                            }
                            if let Some(after) = action.after {
                                self.perform(after, ctx);
                                return;
                            }
                        }
                        Err(e) => {
                            self.pending = action.after;
                            self.status = "Save failed · document remains open".into();
                            self.error = Some(format!(
                                "{e}\n\nUse Save As to choose another file, or explicitly replace the current file."
                            ));
                        }
                    }
                }
            }
        }
    }
    /// Drain local work before replacing the editor. Keep the record until an
    /// explicit discard, including when an earlier save completes during Undo.
    pub(super) fn finish_session(&mut self, discard: bool) -> bool {
        self.remember_version_view();
        self.remember_viewports(true);
        if self.persistence.is_none() {
            return true;
        }
        let p = self.persistence.as_mut().unwrap();
        if let Some(mut session) = p.session.take() {
            let result = session.observe(&self.editor.document).and_then(|()| {
                if discard {
                    session.discard()
                } else {
                    session.finish()
                }
            });
            if let Err(e) = result {
                p.failed = false;
                self.show_stopped_saving(e);
                return false;
            }
        } else if discard && let Some(id) = &p.id {
            let result = p.copies.resume(id).and_then(|c| c.discard());
            if let Err(e) = result {
                self.error = Some(e.to_string());
                return false;
            }
        }
        if discard {
            p.id = None;
        }
        true
    }
    pub(super) fn reset_session_identity(&mut self) {
        self.screenshot_text = None;
        self.save_navigator_focus = None;
        self.review.reset();
        self.remember_version_view();
        self.remember_viewports(true);
        self.presentation_history.clear();
        self.versions.rename = None;
        self.page_rename = None;
        self.page_inspector.clear();
        self.trash = None;
        self.navigator = Default::default();
        self.thumbnails = true;
        if let Some(p) = &mut self.persistence {
            p.id = None;
            p.failed = false;
        }
    }
    #[cfg(test)]
    pub(super) fn load_persistent(&mut self, path: PathBuf) -> bool {
        if !self.finish_session(false) {
            return false;
        }
        let copies = self.persistence.as_ref().unwrap().copies.clone();
        let show = self.persistence.as_ref().unwrap().chooser;
        match PreparedOpen::read(&copies, &path)
            .and_then(|prepared| self.accept_prepared_open(prepared))
        {
            Ok(()) => {
                self.refresh_recoveries(show);
                true
            }
            Err(e) => {
                self.error = Some(e);
                false
            }
        }
    }
    pub(super) fn open_source_id(&self) -> Option<String> {
        self.persistence.as_ref().and_then(|p| p.id.clone())
    }
    pub(super) fn take_open_source(&mut self) -> Option<OpenSource> {
        self.persistence.as_mut().map(|p| OpenSource {
            copies: p.copies.clone(),
            session: p.session.take(),
            id: p.id.clone(),
            document: self.editor.document.clone(),
        })
    }
    pub(super) fn complete_open_source(&mut self, discarded: bool) {
        if let Some(p) = &mut self.persistence {
            if discarded {
                p.id = None;
            }
            // Until adoption installs a new owner, explicit discard has released
            // the old recovery record. If adoption fails, Save As must create a
            // fresh source from the still-visible document, even if its old file
            // vanished during handoff.
            p.failed = discarded;
        }
    }
    pub(super) fn startup_copies(&self) -> Option<WorkingCopies> {
        self.persistence.as_ref().map(|p| p.copies.clone())
    }
    pub(super) fn accept_prepared_open(&mut self, prepared: PreparedOpen) -> Result<(), String> {
        let preserved = prepared.copy.preserved_copy().is_some();
        self.start_copy(prepared.copy)?;
        self.review.reset();
        self.editor = prepared.editor;
        self.file = prepared.file;
        self.restore_version_view();
        self.status = if self.editor.dirty() {
            "Recovered local work"
        } else {
            "Project opened"
        }
        .into();
        if preserved {
            self.status
                .push_str(" · Earlier local work was preserved under File → Recover work");
        }
        // A startup worker may have opened an entry that was already offered.
        if let Some(p) = &mut self.persistence {
            p.offers.retain(|entry| Some(&entry.id) != p.id.as_ref());
            p.chooser &= !p.offers.is_empty();
        }
        Ok(())
    }
    pub(super) fn discard_then(&mut self, pending: Pending, ctx: &egui::Context) {
        if let Pending::Open(path) = pending {
            self.start_menu_open(path, true, ctx);
            return;
        }
        if self.finish_session(true) {
            self.perform(pending, ctx);
        } else {
            self.pending = Some(pending);
        }
    }
    pub(super) fn set_save_mode(&mut self, enabled: bool) {
        let p = self.persistence.as_mut().unwrap();
        let result = (|| {
            let bytes = serde_json::to_vec_pretty(&Settings { automatic: enabled })
                .map_err(|e| e.to_string())?;
            storage::write_atomic(&p.settings, &bytes)?;
            if let Some(session) = &mut p.session {
                session.set_automatic(enabled)?;
            }
            p.automatic = enabled;
            Ok::<_, String>(())
        })();
        if let Err(e) = result {
            self.error = Some(e);
        }
    }
    pub(super) fn automatic_save_setting(&self) -> Option<bool> {
        self.persistence.as_ref().map(|p| p.automatic)
    }
    pub(super) fn show_recoveries(&mut self) {
        self.refresh_recoveries(true);
        if let Some(p) = &mut self.persistence {
            p.chooser = true;
        }
    }
    pub(super) fn restore_copy(&mut self, id: String) -> bool {
        if !self.finish_session(false) {
            return false;
        }
        let result = (|| {
            let copies = self.persistence.as_ref().unwrap().copies.clone();
            let mut copy = if let Some(name) = id.strip_prefix("archive:") {
                copies.restore_archive(name)
            } else {
                copies.resume(&id)
            }
            .map_err(|e| e.to_string())?;
            let mut file = copy.named_path().map_err(|e| e.to_string())?;
            if file.is_some()
                && !matches!(
                    copy.named_status().map_err(|e| e.to_string())?,
                    NamedStatus::Unchanged
                )
            {
                copy = copies
                    .create_unnamed(copy.project())
                    .map_err(|e| e.to_string())?;
                file = None;
            }
            let mut editor = Editor::new(copy.project().clone());
            if let Some(path) = &file {
                editor.mark_saved_snapshot(&ProjectStore.load(path)?);
            } else {
                editor.mark_unsaved();
            }
            for page in editor.document.live_pages() {
                editor.layout_page(page.id)?;
            }
            self.start_copy(copy)?;
            self.review.reset();
            self.editor = editor;
            self.file = file;
            self.restore_version_view();
            self.status = "Recovered local work".into();
            Ok::<_, String>(())
        })();
        if let Err(e) = result {
            self.error = Some(e);
            return false;
        }
        true
    }
    pub(super) fn restore_legacy(&mut self) {
        let Some(doc) = self.recovery_offer.clone() else {
            return;
        };
        if self.persistence.is_some() {
            if !self.finish_session(false) {
                return;
            }
            let result = self
                .persistence
                .as_ref()
                .unwrap()
                .copies
                .create_unnamed(&doc)
                .map_err(|e| e.to_string())
                .and_then(|copy| self.start_copy(copy));
            if let Err(e) = result {
                self.error = Some(e);
                return;
            }
        }
        self.review.reset();
        self.editor = Editor::new(doc);
        self.editor.mark_unsaved();
        self.file = None;
        self.restore_version_view();
        self.recovery_offer = None;
        if self.persistence.is_some() {
            self.clear_recovery();
        }
    }
    pub(super) fn persistence_dialogs(&mut self, ctx: &egui::Context) {
        if self.save_busy() {
            // Capture in the same pass that opens the modal: a fast save may
            // finish before the next UI pass polls the worker.
            self.preserve_save_navigator_focus(ctx);
            egui::Modal::new(egui::Id::new("saving")).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Saving project…");
                });
            });
        }
        if self.recovery_chooser_open()
            && self.recovery_offer.is_none()
            && self.pending.is_none()
            && !self.save_busy()
        {
            let mut selected = None;
            let mut close = false;
            let response = egui::Modal::new(egui::Id::new("working-copies")).show(ctx, |ui| {
                ui.set_width(440.);
                ui.heading("Recover local work");
                ui.label("Choose a project to resume. Other copies stay available under Recover work in the File menu.");
                let p = self.persistence.as_ref().unwrap();
                if p.discovery.is_some() {
                    ui.horizontal(|ui| { ui.spinner(); ui.label("Looking for local work…"); });
                } else if p.offers.is_empty() && p.discovery_errors.is_empty() {
                    ui.label("No other local work is waiting to be recovered.");
                }
                egui::ScrollArea::vertical().max_height(320.).show(ui, |ui| {
                    for error in &p.discovery_errors { ui.colored_label(egui::Color32::DARK_RED, error); }
                    for offer in &self.persistence.as_ref().unwrap().offers {
                        ui.push_id(&offer.id, |ui| {
                            ui.horizontal(|ui| {
                                if ui.add_enabled(self.persistence.as_ref().unwrap().discovery.is_none(), egui::Button::new("Restore")).clicked() { selected = Some(offer.id.clone()); }
                                ui.vertical(|ui| {
                                    ui.label(&offer.title);
                                    ui.small(offer.named_path.as_ref().map_or_else(|| "Unsaved project".into(), |p| p.display().to_string()));
                                });
                            });
                        });
                    }
                });
                close = ui.button("Later").clicked();
            });
            // Escape defers recovery just like Later; it never restores or
            // removes a record, and an error above this picker owns its key.
            close |= response.is_top_modal
                && !response.any_popup_open
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            if close || selected.is_some() {
                self.dismiss_recoveries();
            }
            if let Some(id) = selected {
                self.request(Pending::Restore(id), ctx);
            }
        }
    }
    pub(super) fn shutdown_persistence(&mut self) {
        if self.persistence.is_none() {
            self.sync_recovery(true);
            return;
        }
        if !self.quit_allowed {
            self.observe_project();
        }
        // Ordinary window close has already awaited its save. Application quit
        // can bypass that viewport flow; drain a named flush here before joining.
        if !self.quit_allowed && self.automatic_named() {
            let p = self.persistence.as_mut().unwrap();
            if let Some(session) = &mut p.session
                && let Ok(request) = session.flush(&self.editor.document)
            {
                loop {
                    match session.try_event() {
                        Ok(Some(SessionEvent::RequestFinished {
                            request: done,
                            result,
                        })) if done == request => {
                            if let Err(e) = result {
                                self.status = e;
                            }
                            break;
                        }
                        Ok(Some(_)) => {}
                        Ok(None) => std::thread::sleep(std::time::Duration::from_millis(10)),
                        Err(e) => {
                            self.status = e;
                            break;
                        }
                    }
                }
            }
        }
        self.finish_session(false);
    }
}

#[cfg(test)]
mod tests;
