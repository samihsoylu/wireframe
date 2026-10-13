// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::WireframeApp;
use std::{
    io::ErrorKind,
    time::{Duration, Instant},
};

impl WireframeApp {
    pub(super) fn clear_recovery(&mut self) {
        if let Some(path) = &self.recovery
            && let Err(error) = std::fs::remove_file(path)
            && error.kind() != ErrorKind::NotFound
        {
            self.status = format!("Recovery copy could not be removed: {error}");
            return;
        }
        self.recovery_doc = None;
    }

    /// Reconcile the recovery snapshot with the current document, including Undo.
    /// Shutdown bypasses the write interval but respects an explicit discard.
    pub(super) fn sync_recovery(&mut self, shutdown: bool) {
        if self.recovery.is_none() || self.recovery_offer.is_some() || self.quit_allowed {
            return;
        }
        if !self.editor.dirty() {
            if self.recovery_doc.is_some() {
                self.clear_recovery();
            }
            return;
        }
        if !shutdown && self.last_recovery.elapsed() < Duration::from_secs(5) {
            return;
        }
        self.last_recovery = Instant::now();
        if self.recovery_doc.as_ref() == Some(&self.editor.document) {
            return;
        }
        let path = self.recovery.as_ref().expect("checked recovery path");
        match self.store.save(path, &self.editor.document) {
            Ok(()) => self.recovery_doc = Some(self.editor.document.clone()),
            Err(error) => self.status = format!("Recovery could not be saved: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Pending;
    use eframe::{App, egui};
    use wireframe::{
        document::Project,
        model::Kind,
        storage::{ProjectRepository, ProjectStore},
    };

    #[test]
    fn undo_to_saved_clears_recovery_and_redo_is_recoverable_on_exit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recovery.wireframe");
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, Some(path.clone()));
        app.editor.add(Kind::Checkbox, 10., 20.);
        app.file = Some(dir.path().join("saved.wireframe"));
        assert!(app.save(false));
        let saved = app.editor.document.clone();
        app.editor.add(Kind::Label, 30., 40.);
        let edited = app.editor.document.clone();
        app.last_recovery = Instant::now() - Duration::from_secs(6);
        app.sync_recovery(false);
        assert_eq!(ProjectStore.load(&path).unwrap(), edited);
        app.editor.undo();
        assert_eq!(app.editor.document, saved);
        app.sync_recovery(false);
        assert!(!path.exists());
        assert!(app.recovery_doc.is_none());
        app.editor.redo();
        app.sync_recovery(false);
        assert!(!path.exists(), "write interval is still in effect");
        app.on_exit(None);
        assert_eq!(ProjectStore.load(&path).unwrap(), edited);
        assert_eq!(
            ProjectStore.load(app.file.as_ref().unwrap()).unwrap(),
            saved
        );
        drop(app);
        let reopened = WireframeApp::workspace(&ctx, Some(path));
        assert_eq!(reopened.recovery_offer, Some(edited));
    }

    #[test]
    fn pending_offer_survives_opening_another_project_and_shutdown() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recovery.wireframe");
        let offered = Project {
            title: "Unsaved work".into(),
            ..Project::default()
        };
        ProjectStore.save(&path, &offered).unwrap();
        let other = dir.path().join("other.wireframe");
        ProjectStore.save(&other, &Project::default()).unwrap();
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, Some(path.clone()));
        assert!(app.load(other));
        app.editor.add(Kind::Label, 30., 40.);
        app.last_recovery = Instant::now() - Duration::from_secs(6);
        app.sync_recovery(false);
        app.on_exit(None);
        assert_eq!(app.recovery_offer, Some(offered.clone()));
        assert_eq!(ProjectStore.load(&path).unwrap(), offered);
    }

    #[test]
    fn explicit_discard_is_not_recreated_at_shutdown() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recovery.wireframe");
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, Some(path.clone()));
        app.editor.add(Kind::Label, 30., 40.);
        app.sync_recovery(true);
        assert!(path.exists());
        app.perform(Pending::Quit, &ctx);
        app.on_exit(None);
        assert!(!path.exists());
        drop(app);
        assert!(
            WireframeApp::workspace(&ctx, Some(path))
                .recovery_offer
                .is_none()
        );
    }

    #[test]
    fn failed_recovery_write_keeps_previous_snapshot_and_retries() {
        struct FailedWrite;
        impl ProjectRepository for FailedWrite {
            fn load(&self, _: &std::path::Path) -> Result<Project, String> {
                Err("Unavailable".into())
            }
            fn save(&self, _: &std::path::Path, _: &Project) -> Result<(), String> {
                Err("Disk full".into())
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recovery.wireframe");
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, Some(path.clone()));
        app.editor.add(Kind::Label, 30., 40.);
        app.sync_recovery(true);
        let before = std::fs::read(&path).unwrap();
        app.editor.add(Kind::Checkbox, 50., 60.);
        app.store = Box::new(FailedWrite);
        app.sync_recovery(true);
        assert!(app.status.contains("Disk full"));
        assert_ne!(app.recovery_doc.as_ref(), Some(&app.editor.document));
        assert_eq!(std::fs::read(&path).unwrap(), before);
        app.store = Box::new(ProjectStore);
        app.on_exit(None);
        assert_eq!(ProjectStore.load(&path).unwrap(), app.editor.document);
    }
}
