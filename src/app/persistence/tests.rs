// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use eframe::App;
use std::time::{Duration, Instant};
use wireframe::{document::Project, model::Kind};

fn fixture() -> (tempfile::TempDir, egui::Context, WireframeApp) {
    let dir = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, Some(dir.path().join("recovery.wireframe")));
    app.enable_persistence(&ctx, dir.path().to_owned());
    (dir, ctx, app)
}
fn drain(app: &mut WireframeApp, ctx: &egui::Context) {
    let deadline = Instant::now() + Duration::from_secs(12);
    while app.save_busy() {
        app.poll_saves(ctx);
        assert!(Instant::now() < deadline, "save timed out: {:?}", app.error);
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn named(app: &mut WireframeApp, ctx: &egui::Context, path: &std::path::Path) {
    app.editor.add(Kind::CircleButton, 40., 50.);
    assert!(app.queue_save(path.to_owned(), true));
    drain(app, ctx);
    assert!(app.error.is_none(), "{:?}", app.error);
    assert!(!app.editor.dirty());
}

#[test]
fn named_save_restores_navigator_focus_before_the_next_outdent_and_arrow() {
    use std::sync::{Condvar, Mutex};
    struct Gate(Arc<(Mutex<bool>, Condvar)>);
    impl Gate {
        fn release(&self) {
            *self.0.0.lock().unwrap() = true;
            self.0.1.notify_all();
        }
    }
    impl Drop for Gate {
        fn drop(&mut self) {
            self.release();
        }
    }
    fn frame(
        ctx: &egui::Context,
        app: &mut WireframeApp,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200., 850.),
            )),
            events,
            ..Default::default()
        };
        app.raw_input_hook(ctx, &mut input);
        ctx.run_ui(input, |ui| app.ui(ui, &mut eframe::Frame::_new_kittest()))
    }
    fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers,
        }
    }
    for (thumbnails, key_on_completion, busy_frames) in [
        (false, false, 3),
        (true, false, 3),
        (false, true, 3),
        (true, true, 3),
        (false, true, 0),
        (true, true, 0),
    ] {
        let (dir, ctx, mut app) = fixture();
        ctx.enable_accesskit();
        app.thumbnails = thumbnails;
        let first = app.editor.page;
        app.editor.add_page();
        let second = app.editor.page;
        app.editor.rename_page(second, "Second").unwrap();
        app.editor.add_page();
        let third = app.editor.page;
        app.editor.switch_page(second);
        let path = dir.path().join("Navigation.wireframe");
        ProjectStore.save(&path, &app.editor.document).unwrap();
        app.file = Some(path.clone());
        app.editor.mark_saved();
        let gate = Gate(Arc::new((Mutex::new(false), Condvar::new())));
        let wait = gate.0.clone();
        let p = app.persistence.as_mut().unwrap();
        let copy = p.copies.open_named(&path).unwrap();
        p.id = Some(copy.id().to_owned());
        p.session = Some(
            ProjectSession::start(
                p.copies.clone(),
                copy,
                false,
                Arc::new(move || {
                    let (lock, ready) = &*wait;
                    let mut released = lock.lock().unwrap();
                    while !*released {
                        released = ready.wait(released).unwrap();
                    }
                }),
            )
            .unwrap(),
        );
        let mut point = egui::Pos2::ZERO;
        for _ in 0..3 {
            let out = frame(&ctx, &mut app, vec![]);
            let node = &out
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .find(|(_, n)| n.label() == Some("Second"))
                .unwrap()
                .1;
            let b = node.bounds().unwrap();
            point = egui::pos2(((b.x0 + b.x1) / 2.) as f32, ((b.y0 + b.y1) / 2.) as f32);
            out.drop_without_applying_deltas();
        }
        for pressed in [true, false] {
            frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            )
            .drop_without_applying_deltas();
        }
        assert!(app.navigator.focused(&ctx));
        frame(
            &ctx,
            &mut app,
            vec![key(egui::Key::Tab, egui::Modifiers::NONE)],
        )
        .drop_without_applying_deltas();
        assert_eq!(
            app.editor.document.live_page(second).unwrap().parent,
            Some(first)
        );
        let saved = app.editor.document.clone();
        frame(
            &ctx,
            &mut app,
            vec![key(egui::Key::S, egui::Modifiers::COMMAND)],
        )
        .drop_without_applying_deltas();
        assert!(app.save_busy());
        for _ in 0..busy_frames {
            frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
        }
        gate.release();
        if key_on_completion {
            // Deliver the next key on the first UI pass after completion, while
            // egui still remembers the previous pass's saving modal.
            drain(&mut app, &ctx);
        } else {
            let deadline = Instant::now() + Duration::from_secs(12);
            while app.save_busy() {
                frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
                assert!(Instant::now() < deadline, "Save timed out");
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(
                app.navigator.focused(&ctx),
                "Save must retain page keyboard ownership"
            );
        }
        assert!(app.error.is_none(), "{:?}", app.error);
        assert_eq!(ProjectStore.load(&path).unwrap(), saved);
        frame(
            &ctx,
            &mut app,
            vec![key(egui::Key::Tab, egui::Modifiers::SHIFT)],
        )
        .drop_without_applying_deltas();
        assert_eq!(app.editor.document.live_page(second).unwrap().parent, None);
        frame(
            &ctx,
            &mut app,
            vec![key(egui::Key::ArrowDown, egui::Modifiers::NONE)],
        )
        .drop_without_applying_deltas();
        assert_eq!(app.editor.page, third);
        frame(
            &ctx,
            &mut app,
            vec![key(egui::Key::Z, egui::Modifiers::COMMAND)],
        )
        .drop_without_applying_deltas();
        assert_eq!(app.editor.document, saved);
        app.on_exit(None);
    }
}
#[test]
fn save_focus_does_not_override_a_dialog_new_owner_or_replaced_session() {
    let (_dir, ctx, mut app) = fixture();
    let previous = egui::Id::new("previous-page");
    let next = egui::Id::new("new-owner");
    app.save_navigator_focus = Some(previous);
    app.error = Some("Save failed".into());
    app.preserve_save_navigator_focus(&ctx);
    assert_eq!(ctx.memory(|m| m.focused()), None);
    assert_eq!(app.navigator.focus_to, None);
    assert_eq!(app.save_navigator_focus, None);
    app.error = None;
    app.save_navigator_focus = Some(previous);
    ctx.memory_mut(|m| m.request_focus(next));
    app.preserve_save_navigator_focus(&ctx);
    assert_eq!(ctx.memory(|m| m.focused()), Some(next));
    assert_eq!(app.navigator.focus_to, None);
    app.save_navigator_focus = Some(previous);
    app.reset_session_identity();
    assert_eq!(app.save_navigator_focus, None);
}
#[test]
fn ui_named_save_automatic_edit_and_close_flush_use_the_worker() {
    let (dir, ctx, mut app) = fixture();
    let path = dir.path().join("Project.wireframe");
    named(&mut app, &ctx, &path);
    app.editor.add(Kind::Label, 120., 130.);
    app.observe_project();
    let deadline = Instant::now() + Duration::from_secs(12);
    while app.editor.dirty() {
        app.poll_saves(&ctx);
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(ProjectStore.load(&path).unwrap(), app.editor.document);
    app.editor.add(Kind::Button, 200., 210.);
    let latest = app.editor.document.clone();
    app.request(Pending::Quit, &ctx);
    assert!(!app.quit_allowed);
    assert!(app.save_busy());
    drain(&mut app, &ctx);
    assert!(app.quit_allowed);
    assert_eq!(ProjectStore.load(&path).unwrap(), latest);
    app.on_exit(None);
}
#[test]
fn older_ui_ack_keeps_later_edits_dirty_and_undo_history_intact() {
    let (dir, ctx, mut app) = fixture();
    let path = dir.path().join("Project.wireframe");
    app.set_save_mode(false);
    app.editor.add(Kind::Button, 10., 20.);
    let captured = app.editor.document.clone();
    assert!(app.queue_save(path.clone(), true));
    app.editor.add(Kind::Label, 30., 40.);
    let latest = app.editor.document.clone();
    app.observe_project();
    drain(&mut app, &ctx);
    assert!(app.editor.dirty());
    assert_eq!(ProjectStore.load(&path).unwrap(), captured);
    assert_eq!(app.editor.document, latest);
    app.editor.undo();
    assert!(!app.editor.dirty());
    app.editor.redo();
    assert!(app.editor.dirty());
    app.on_exit(None);
}
#[test]
fn manual_quit_retains_local_edits_and_mode_then_reopen_recovers() {
    let (dir, ctx, mut app) = fixture();
    let path = dir.path().join("Project.wireframe");
    named(&mut app, &ctx, &path);
    let saved = app.editor.document.clone();
    app.set_save_mode(false);
    app.editor.add(Kind::Label, 120., 130.);
    let latest = app.editor.document.clone();
    app.on_exit(None);
    assert_eq!(ProjectStore.load(&path).unwrap(), saved);
    drop(app);
    let mut reopened = WireframeApp::workspace(&ctx, Some(dir.path().join("recovery.wireframe")));
    reopened.enable_persistence(&ctx, dir.path().to_owned());
    discovery::tests::drain_recoveries(&mut reopened);
    assert!(!reopened.persistence.as_ref().unwrap().automatic);
    assert_eq!(reopened.persistence.as_ref().unwrap().offers.len(), 1);
    assert!(reopened.load(path));
    assert_eq!(reopened.editor.document, latest);
    assert!(reopened.editor.dirty());
}
#[test]
fn deleted_clean_file_blocks_close_and_explicit_same_path_save_as_repairs_it() {
    let (dir, ctx, mut app) = fixture();
    let path = dir.path().join("Project.wireframe");
    named(&mut app, &ctx, &path);
    std::fs::remove_file(&path).unwrap();
    app.request(Pending::Quit, &ctx);
    drain(&mut app, &ctx);
    assert!(!app.quit_allowed);
    assert!(matches!(app.pending, Some(Pending::Quit)));
    assert!(app.error.take().unwrap().contains("missing"));
    assert!(app.queue_save(path.clone(), true));
    drain(&mut app, &ctx);
    assert!(app.quit_allowed);
    assert_eq!(ProjectStore.load(&path).unwrap(), app.editor.document);
}
#[test]
fn failed_save_as_keeps_source_and_pending_action() {
    let (dir, ctx, mut app) = fixture();
    app.editor.add(Kind::Button, 10., 20.);
    let before = app.editor.document.clone();
    app.pending = Some(Pending::New);
    assert!(app.queue_save(dir.path().join("missing/Project.wireframe"), true));
    drain(&mut app, &ctx);
    assert!(matches!(app.pending, Some(Pending::New)));
    assert!(app.file.is_none());
    assert_eq!(app.editor.document, before);
    app.pending = None; // Cancel leaves the edited document in place.
    app.on_exit(None);
    let p = app.persistence.as_ref().unwrap();
    assert_eq!(
        p.copies.resume(p.id.as_ref().unwrap()).unwrap().project(),
        &before
    );
}
#[test]
fn explicit_discard_removes_local_work_without_shutdown_recreation() {
    let (_dir, ctx, mut app) = fixture();
    app.editor.add(Kind::Button, 10., 20.);
    app.observe_project();
    let copies = app.persistence.as_ref().unwrap().copies.clone();
    app.discard_then(Pending::Quit, &ctx);
    assert!(app.quit_allowed);
    app.on_exit(None);
    assert!(copies.list().unwrap().is_empty());
}
#[test]
fn legacy_restore_migrates_before_removing_old_copy_and_preserves_an_empty_project() {
    let dir = tempfile::tempdir().unwrap();
    let legacy = dir.path().join("recovery.wireframe");
    let project = Project::default();
    ProjectStore.save(&legacy, &project).unwrap();
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, Some(legacy.clone()));
    app.enable_persistence(&ctx, dir.path().to_owned());
    app.restore_legacy();
    assert!(app.recovery_offer.is_none());
    assert!(!legacy.exists());
    assert!(app.editor.dirty());
    app.on_exit(None);
    let p = app.persistence.as_ref().unwrap();
    assert_eq!(
        p.copies.resume(p.id.as_ref().unwrap()).unwrap().project(),
        &project
    );
}
#[test]
fn named_open_preserves_conflicting_local_work_for_recovery() {
    let (dir, ctx, mut app) = fixture();
    let path = dir.path().join("Project.wireframe");
    named(&mut app, &ctx, &path);
    app.set_save_mode(false);
    app.editor.add(Kind::Label, 120., 130.);
    let local = app.editor.document.clone();
    app.on_exit(None);
    let external = Project {
        title: "Outside version".into(),
        ..Project::default()
    };
    ProjectStore.save(&path, &external).unwrap();
    assert!(app.load(path.clone()));
    assert_eq!(app.editor.document, external);
    app.refresh_recoveries(true);
    discovery::tests::drain_recoveries(&mut app);
    let p = app.persistence.as_ref().unwrap();
    assert_eq!(p.offers.len(), 1);
    assert_eq!(p.copies.resume(&p.offers[0].id).unwrap().project(), &local);
    assert_eq!(ProjectStore.load(&path).unwrap(), external);
}
#[test]
fn failed_open_retains_current_document_and_can_resume_its_owner() {
    let (dir, ctx, mut app) = fixture();
    let path = dir.path().join("Project.wireframe");
    named(&mut app, &ctx, &path);
    app.set_save_mode(false);
    app.editor.add(Kind::Label, 120., 130.);
    let local = app.editor.document.clone();
    assert!(!app.load(dir.path().join("missing.wireframe")));
    assert_eq!(app.editor.document, local);
    app.error = None;
    app.observe_project();
    assert!(app.persistence.as_ref().unwrap().session.is_some());
    assert_eq!(app.file, Some(path.canonicalize().unwrap()));
}
#[test]
fn restored_external_conflict_becomes_unnamed_without_overwriting_named_file() {
    let (dir, ctx, mut app) = fixture();
    let path = dir.path().join("Project.wireframe");
    named(&mut app, &ctx, &path);
    app.set_save_mode(false);
    app.editor.add(Kind::Label, 120., 130.);
    let local = app.editor.document.clone();
    app.on_exit(None);
    let id = app.persistence.as_ref().unwrap().id.clone().unwrap();
    let external = Project {
        title: "Outside version".into(),
        ..Project::default()
    };
    ProjectStore.save(&path, &external).unwrap();
    assert!(app.restore_copy(id));
    assert!(app.file.is_none());
    assert!(app.editor.dirty());
    assert_eq!(app.editor.document, local);
    app.on_exit(None);
    assert_eq!(ProjectStore.load(&path).unwrap(), external);
}
#[test]
fn malformed_settings_are_retained_and_automatic_mode_is_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let settings = dir.path().join("save-settings.json");
    std::fs::write(&settings, b"damaged settings").unwrap();
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.enable_persistence(&ctx, dir.path().to_owned());
    assert!(!app.persistence.as_ref().unwrap().automatic);
    assert!(app.error.is_some());
    assert_eq!(std::fs::read(settings).unwrap(), b"damaged settings");
}

#[test]
fn save_as_archived_destination_is_visible_and_restores_as_a_separate_project() {
    let (dir, ctx, mut app) = fixture();
    let path = dir.path().join("Destination.wireframe");
    let old = Project {
        title: "Earlier destination".into(),
        ..Project::default()
    };
    ProjectStore.save(&path, &old).unwrap();
    let copies = app.persistence.as_ref().unwrap().copies.clone();
    drop(copies.open_named(&path).unwrap());
    named(&mut app, &ctx, &path);
    let written = ProjectStore.load(&path).unwrap();
    app.refresh_recoveries(true);
    discovery::tests::drain_recoveries(&mut app);
    let archive = app
        .persistence
        .as_ref()
        .unwrap()
        .offers
        .iter()
        .find(|o| o.id.starts_with("archive:"))
        .unwrap()
        .id
        .clone();
    assert!(app.restore_copy(archive.clone()));
    assert_eq!(app.editor.document, old);
    assert!(app.file.is_none());
    assert!(app.editor.dirty());
    assert_eq!(ProjectStore.load(&path).unwrap(), written);
    assert_eq!(copies.list_archives().unwrap().len(), 1);
    assert!(copies.restore_archive("../Destination.wireframe").is_err());
    assert!(
        copies
            .restore_archive("previous-working-../../other.json")
            .is_err()
    );
}

#[test]
fn local_acknowledgement_does_not_claim_newer_edits_are_durable() {
    let (_dir, ctx, mut app) = fixture();
    app.editor.add(Kind::Button, 10., 20.);
    app.observe_project();
    // Hold the newer edit in the editor while the worker acknowledges its
    // previously captured local copy. No timing assumption about disk speed.
    app.editor.add(Kind::Label, 40., 50.);
    let deadline = Instant::now() + Duration::from_secs(5);
    while !app.status.contains("newer edits pending") {
        app.poll_saves(&ctx);
        assert!(Instant::now() < deadline, "{}", app.status);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(app.editor.dirty());
    app.on_exit(None);
    let p = app.persistence.as_ref().unwrap();
    assert_eq!(
        p.copies.resume(p.id.as_ref().unwrap()).unwrap().project(),
        &app.editor.document
    );
}

#[test]
fn clean_save_receipt_finishes_status_without_a_separate_saved_event() {
    let (dir, ctx, mut app) = fixture();
    let path = dir.path().join("Project.wireframe");
    named(&mut app, &ctx, &path);
    let saved = app.editor.document.clone();
    for newer_edit in [false, true] {
        assert!(app.queue_save(path.clone(), false));
        assert_eq!(app.status, "Saving…");
        if newer_edit {
            app.editor.add(Kind::Label, 200., 200.);
        }
        drain(&mut app, &ctx);
        assert_eq!(app.editor.dirty(), newer_edit);
        assert_eq!(ProjectStore.load(&path).unwrap(), saved);
        assert_eq!(
            app.status,
            if newer_edit {
                "Saved · newer edits pending"
            } else {
                "Saved to your computer"
            }
        );
    }
}
