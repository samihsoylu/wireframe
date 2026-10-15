// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
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
    chooser: bool,
    failed: bool,
}
impl WireframeApp {
    pub(super) fn enable_persistence(&mut self, ctx: &egui::Context, root: PathBuf) {
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
            return Err("Saving stopped. Keep this document open and use Save As to retain a separate copy.".into());
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
    pub(super) fn recovery_chooser_open(&self) -> bool {
        self.persistence.as_ref().is_some_and(|p| p.chooser)
    }
    pub(super) fn automatic_named(&self) -> bool {
        self.file.is_some() && self.persistence.as_ref().is_some_and(|p| p.automatic)
    }
    pub(super) fn observe_project(&mut self) {
        if self.quit_allowed || self.persistence.is_none() {
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
            self.error = Some(e);
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
                        self.error = Some(e);
                        return false;
                    }
                }
                Err(e) => {
                    self.error = Some(e.to_string());
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
                    self.error = Some(e);
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
                self.error = Some(e);
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
        if let Some(p) = &mut self.persistence {
            p.id = None;
            p.failed = false;
        }
    }
    pub(super) fn load_persistent(&mut self, path: PathBuf) -> bool {
        if !self.finish_session(false) {
            return false;
        }
        let result = (|| {
            let copies = self.persistence.as_ref().unwrap().copies.clone();
            let mut copy = copies.open_named(&path).map_err(|e| e.to_string())?;
            let mut file = copy.named_path().map_err(|e| e.to_string())?;
            match copy.named_status().map_err(|e| e.to_string())? {
                NamedStatus::Changed(_) => copy.reload_named(&copies).map_err(|e| e.to_string())?,
                NamedStatus::Missing | NamedStatus::Unreadable(_) => {
                    copy = copies
                        .create_unnamed(copy.project())
                        .map_err(|e| e.to_string())?;
                    file = None;
                }
                NamedStatus::Unchanged => {}
            }
            let mut candidate = Editor::new(copy.project().clone());
            if let Some(path) = &file {
                let named = ProjectStore.load(path)?;
                candidate.mark_saved_snapshot(&named);
            } else {
                candidate.mark_unsaved();
            }
            for page in &candidate.document.pages {
                candidate.layout_page(page.id)?;
            }
            self.start_copy(copy)?;
            self.editor = candidate;
            self.file = file;
            self.pan = egui::Vec2::ZERO;
            self.zoom = 1.;
            self.status = if self.editor.dirty() {
                "Recovered local work"
            } else {
                "Project opened"
            }
            .into();
            Ok::<_, String>(())
        })();
        if let Err(e) = result {
            self.error = Some(e);
            return false;
        }
        let show = self.persistence.as_ref().unwrap().chooser;
        self.refresh_recoveries(show);
        true
    }
    pub(super) fn discard_then(&mut self, pending: Pending, ctx: &egui::Context) {
        // Validate a requested file before discarding the visible document.
        if let Pending::Open(path) = &pending
            && let Err(e) = ProjectStore.load(path)
        {
            self.error = Some(e);
            self.pending = Some(pending);
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
    pub(super) fn persistence_menu(&mut self, ui: &mut egui::Ui) {
        if let Some(p) = &self.persistence {
            let mut enabled = p.automatic;
            if ui
                .checkbox(&mut enabled, "Automatically save named projects")
                .changed()
            {
                self.set_save_mode(enabled);
            }
            if ui.button("Recover work…").clicked() {
                self.refresh_recoveries(true);
                self.persistence.as_mut().unwrap().chooser = true;
                ui.close();
            }
            ui.separator();
        }
    }
    fn refresh_recoveries(&mut self, show: bool) {
        let Some(p) = &mut self.persistence else {
            return;
        };
        p.offers.clear();
        match p.copies.list().and_then(|mut entries| {
            entries.extend(p.copies.list_archives()?);
            Ok(entries)
        }) {
            Ok(entries) => {
                for entry in entries {
                    if p.id.as_deref() == Some(&entry.id) {
                        continue;
                    }
                    match entry.summary {
                        Ok(s)
                            if s.has_local_changes
                                || s.named_path.as_ref().is_some_and(|f| !f.exists()) =>
                        {
                            p.offers.push(s)
                        }
                        Ok(_) => {}
                        Err(e) => {
                            self.error = Some(format!(
                                "A recovery record was retained but couldn't be read: {e}"
                            ))
                        }
                    }
                }
            }
            Err(e) => self.error = Some(e.to_string()),
        }
        p.chooser = show && !p.offers.is_empty();
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
            for page in &editor.document.pages {
                editor.layout_page(page.id)?;
            }
            self.start_copy(copy)?;
            self.editor = editor;
            self.file = file;
            self.pan = egui::Vec2::ZERO;
            self.zoom = 1.;
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
        self.editor = Editor::new(doc);
        self.editor.mark_unsaved();
        self.file = None;
        self.recovery_offer = None;
        if self.persistence.is_some() {
            self.clear_recovery();
        }
    }
    pub(super) fn persistence_dialogs(&mut self, ctx: &egui::Context) {
        if self.save_busy() {
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
            egui::Modal::new(egui::Id::new("working-copies")).show(ctx, |ui| {
                ui.heading("Recover local work");
                ui.label("Choose a project to resume. Other copies stay available under Recover work in the File menu.");
                if self.persistence.as_ref().unwrap().offers.is_empty() {
                    ui.label("No other local work is waiting to be recovered.");
                }
                egui::ScrollArea::vertical().max_height(320.).show(ui, |ui| {
                    for offer in &self.persistence.as_ref().unwrap().offers {
                        ui.push_id(&offer.id, |ui| {
                            ui.horizontal(|ui| {
                                if ui.button("Restore").clicked() { selected = Some(offer.id.clone()); }
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
            if close || selected.is_some() {
                self.persistence.as_mut().unwrap().chooser = false;
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
