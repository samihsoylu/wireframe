// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::WireframeApp;
use std::{
    io::ErrorKind,
    time::{Duration, Instant},
};

impl WireframeApp {
    pub(super) fn clear_recovery(&mut self) {
        if self.recovery_offer.is_some() {
            return;
        }
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
        if self.persistence.is_some()
            || self.recovery.is_none()
            || self.recovery_offer.is_some()
            || self.quit_allowed
        {
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
    fn unreadable_recovery_is_preserved_before_new_work_can_checkpoint() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recovery.wireframe");
        let broken = b"{unfinished project bytes";
        std::fs::write(&path, broken).unwrap();
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, Some(path.clone()));
        assert!(
            app.error.is_some(),
            "startup must disclose unreadable recovery"
        );
        let archived: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|v| v.unwrap().path())
            .filter(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("unreadable-recovery-")
            })
            .collect();
        assert_eq!(archived.len(), 1);
        assert_eq!(std::fs::read(&archived[0]).unwrap(), broken);
        app.editor.add(Kind::Label, 30., 40.);
        let edited = app.editor.document.clone();
        app.on_exit(None);
        assert_eq!(ProjectStore.load(&path).unwrap(), edited);
        assert_eq!(std::fs::read(&archived[0]).unwrap(), broken);
        drop(app);
        let reopened = WireframeApp::workspace(&ctx, Some(path));
        assert_eq!(reopened.recovery_offer, Some(edited));
        assert!(reopened.error.is_none());
    }

    #[test]
    fn failed_preservation_disables_recovery_without_blocking_manual_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recovery.wireframe");
        std::fs::create_dir(&path).unwrap();
        let original = path.join("do-not-replace");
        std::fs::write(&original, b"kept").unwrap();
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, Some(path.clone()));
        assert!(app.recovery.is_none());
        assert!(
            app.error
                .as_ref()
                .unwrap()
                .contains("Save your work manually")
        );
        app.editor.add(Kind::Label, 30., 40.);
        let manual = dir.path().join("manual.wireframe");
        app.file = Some(manual.clone());
        assert!(app.save(false));
        app.editor.add(Kind::Button, 50., 60.);
        app.sync_recovery(true);
        app.perform(Pending::New, &ctx);
        app.on_exit(None);
        assert_eq!(std::fs::read(&original).unwrap(), b"kept");
        assert_eq!(
            ProjectStore.load(&manual).unwrap().pages[0]
                .content
                .nodes
                .len(),
            1
        );
    }

    #[test]
    fn repeated_unreadable_checkpoints_keep_distinct_originals() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recovery.wireframe");
        let ctx = egui::Context::default();
        for bytes in [b"invalid one".as_slice(), b"invalid two".as_slice()] {
            std::fs::write(&path, bytes).unwrap();
            let app = WireframeApp::workspace(&ctx, Some(path.clone()));
            assert!(app.error.is_some());
            drop(app);
        }
        let mut contents: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|v| v.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "wireframe"))
            .map(|p| std::fs::read(p).unwrap())
            .collect();
        contents.sort();
        assert_eq!(
            contents,
            vec![b"invalid one".to_vec(), b"invalid two".to_vec()]
        );
    }

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
