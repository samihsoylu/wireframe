// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::app::source_navigator::ResourceRow;
use eframe::App;
use std::collections::HashSet;
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

// Capture every public history state, returning to the original tip. These
// fixtures intentionally start without a redo tail.
fn local_save_history(editor: &mut Editor) -> Vec<serde_json::Value> {
    assert!(editor.redo_name().is_none());
    let mut states = vec![local_save_editor_state(editor)];
    while editor.undo_name().is_some() {
        editor.undo();
        states.push(local_save_editor_state(editor));
    }
    for _ in 1..states.len() {
        editor.redo();
    }
    states
}
fn local_save_editor_state(editor: &Editor) -> serde_json::Value {
    let mut selection: Vec<_> = editor.selection.iter().copied().collect();
    selection.sort_unstable();
    serde_json::json!({
        "document": editor.document,
        "page": editor.page,
        "definition": editor.definition,
        "library": editor.library,
        "context": editor.context,
        "selection": selection,
        "versions": editor.version_selections(),
        "undo": editor.undo_name(),
        "redo": editor.redo_name(),
    })
}
fn assert_local_save_stopped(app: &WireframeApp) {
    assert!(app.persistence.as_ref().unwrap().failed);
    assert_local_save_stopped_feedback(app);
}
fn assert_local_save_stopped_feedback(app: &WireframeApp) {
    assert_eq!(
        app.status_text(),
        "Saving stopped · keep this document open"
    );
    let error = app.error.as_deref().unwrap();
    let (primary, details) = error.split_once("\n\nDetails: ").unwrap();
    assert_eq!(
        primary,
        "Saving stopped. Keep this document open. Use File → Save as… to try saving a separate copy. If saving still fails, check available disk space and access."
    );
    assert!(!details.is_empty());
    for technical in ["worker", "queue", "transaction"] {
        assert!(!primary.contains(technical));
    }
}

#[test]
fn first_local_record_failure_keeps_graph_history_and_save_as_recovery_guidance() {
    let (dir, ctx, mut app) = fixture();
    // A synthetic graph with official and alternate content, two pages, and a
    // library/source. No local record exists until the controlled edit below.
    app.editor.add(Kind::Button, 10., 20.);
    app.editor.add_page();
    let page = app.editor.page;
    app.editor.rename_page(page, "Details").unwrap();
    let library = app.editor.create_library("Shared").unwrap();
    app.editor
        .create_empty_source(library, "Card", Default::default())
        .unwrap();
    app.editor.switch_page(page);
    let alternate = app.editor.create_version("Phone").unwrap();
    let before_edit = app.editor.document.clone();
    let earlier_path = dir.path().join("Earlier.wireframe");
    ProjectStore.save(&earlier_path, &before_edit).unwrap();
    let earlier_bytes = std::fs::read(&earlier_path).unwrap();
    let root = dir.path().join("working-copies");
    let aside = dir.path().join("owned-working-copies-aside");
    assert!(app.persistence.as_ref().unwrap().session.is_none());
    assert!(app.persistence.as_ref().unwrap().id.is_none());
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
    std::fs::rename(&root, &aside).unwrap();
    std::fs::write(&root, b"owned local-save fault sentinel").unwrap();
    app.editor.add(Kind::Label, 30., 40.); // Exactly one edit after the fault.
    let expected = app.editor.document.clone();
    let history = local_save_history(&mut app.editor);
    let state = local_save_editor_state(&app.editor);
    let dirty = app.editor.dirty();
    assert!(dirty);
    assert_eq!(app.editor.page, page);
    assert_eq!(app.editor.active_alternate(page), Some(alternate));
    assert!(!app.editor.selection.is_empty());
    app.pending = Some(Pending::Quit);
    app.observe_project();
    assert_eq!(app.editor.document, expected);
    assert_eq!(local_save_editor_state(&app.editor), state);
    assert_eq!(local_save_history(&mut app.editor), history);
    assert_eq!(app.editor.dirty(), dirty);
    assert!(matches!(app.pending, Some(Pending::Quit)));
    assert!(!app.quit_allowed);
    assert!(!app.save_busy());
    assert!(app.persistence.as_ref().unwrap().failed);
    assert!(app.error.is_some());
    eprintln!("FIRST-FAULT status={:?}; error={:?}", app.status, app.error);
    eprintln!(
        "FULL-ORACLE {}",
        serde_json::json!({"state": state, "history": history, "dirty": dirty, "pre_edit": before_edit})
    );
    assert_local_save_stopped(&app);
    let stopped_status = app.status.clone();
    app.error = None; // Dismissal leaves the persistent stopped status.
    app.observe_project();
    app.poll_saves(&ctx);
    assert_eq!(app.status, stopped_status);
    assert!(app.error.is_none());
    let rescue = dir.path().join("Separate.wireframe");
    assert!(!app.queue_save(rescue.clone(), true));
    eprintln!(
        "BLOCKED-RETRY status={:?}; error={:?}",
        app.status, app.error
    );
    assert_local_save_stopped(&app);
    assert!(!rescue.exists());
    assert_eq!(local_save_editor_state(&app.editor), state);
    assert_eq!(local_save_history(&mut app.editor), history);
    assert_eq!(app.editor.dirty(), dirty);
    assert!(matches!(app.pending, Some(Pending::Quit)));
    assert!(!app.save_busy());
    assert!(!app.quit_allowed);
    std::fs::remove_file(&root).unwrap();
    std::fs::rename(&aside, &root).unwrap();
    app.pending = None; // Explicit Cancel before the normal Save As rescue.
    app.error = None;
    assert!(app.queue_save(rescue.clone(), true));
    drain(&mut app, &ctx);
    assert!(app.error.is_none(), "{:?}", app.error);
    assert!(!app.persistence.as_ref().unwrap().failed);
    assert_eq!(app.status_text(), "Saved to your computer");
    assert_eq!(ProjectStore.load(&rescue).unwrap(), expected);
    assert_eq!(local_save_editor_state(&app.editor), state);
    assert_eq!(local_save_history(&mut app.editor), history);
    assert!(!app.editor.dirty());
    app.editor.undo();
    assert_eq!(app.editor.document, before_edit);
    assert!(app.editor.dirty());
    app.editor.redo();
    assert_eq!(app.editor.document, expected);
    assert!(!app.editor.dirty());
    assert_eq!(std::fs::read(&earlier_path).unwrap(), earlier_bytes);
    app.on_exit(None);
    // A fresh model from saved bytes, not a claim of native process coverage.
    let cold = Editor::new(ProjectStore.load(&rescue).unwrap());
    assert_eq!(cold.document, expected);
    assert!(!cold.dirty());
    assert!(cold.undo_name().is_none());
    assert!(cold.redo_name().is_none());
    eprintln!(
        "RECOVERED: full graph, complete history, one Undo/Redo, earlier bytes, cold model verified"
    );
}

use std::sync::{Condvar, Mutex};
struct SaveFocusGate(Arc<(Mutex<bool>, Condvar)>);
impl SaveFocusGate {
    fn release(&self) {
        *self.0.0.lock().unwrap() = true;
        self.0.1.notify_all();
    }
}
impl Drop for SaveFocusGate {
    fn drop(&mut self) {
        self.release();
    }
}
#[test]
fn injected_disconnect_keeps_clean_or_dirty_state_and_releases_busy_close() {
    // Controlled application unit evidence: the test wake callback panics
    // before the actor can process queued commands. This is not a native
    // worker-crash reproduction or a filesystem-failure test.
    for dirty in [false, true] {
        for busy in [false, true] {
            let (dir, ctx, mut app) = fixture();
            app.set_save_mode(false);
            app.editor.add(Kind::Button, 10., 20.);
            let path = dir.path().join("Already-saved.wireframe");
            ProjectStore.save(&path, &app.editor.document).unwrap();
            let saved_bytes = std::fs::read(&path).unwrap();
            app.file = Some(path.clone());
            app.editor.mark_saved();
            let gate = SaveFocusGate(Arc::new((Mutex::new(false), Condvar::new())));
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
                        let mut released = wait.0.lock().unwrap();
                        while !*released {
                            released = wait.1.wait(released).unwrap();
                        }
                        drop(released);
                        panic!("injected test-only disconnect after initial checkpoint");
                    }),
                )
                .unwrap(),
            );
            if dirty {
                app.editor.add(Kind::Label, 30., 40.);
            }
            let history = local_save_history(&mut app.editor);
            let state = local_save_editor_state(&app.editor);
            app.pending = Some(Pending::Quit);
            if busy {
                assert!(app.queue_save(path.clone(), false));
                assert!(app.pending.is_none());
                assert_eq!(app.status, "Saving…");
            }
            gate.release();
            let deadline = Instant::now() + Duration::from_secs(12);
            while !app.persistence.as_ref().unwrap().failed {
                app.poll_saves(&ctx);
                assert!(Instant::now() < deadline, "injected disconnect timed out");
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(!app.save_busy());
            assert!(matches!(app.pending, Some(Pending::Quit)));
            assert!(!app.quit_allowed);
            assert_eq!(app.file.as_ref(), Some(&path));
            assert_eq!(local_save_editor_state(&app.editor), state);
            assert_eq!(local_save_history(&mut app.editor), history);
            assert_eq!(app.editor.dirty(), dirty);
            assert_eq!(std::fs::read(&path).unwrap(), saved_bytes);
            assert_local_save_stopped(&app);
            assert!(
                app.error
                    .as_deref()
                    .unwrap()
                    .ends_with("Details: Save worker stopped; keep your current document open")
            );
            eprintln!(
                "INJECTED-DISCONNECT dirty={dirty} busy={busy}: graph/history/saved bytes/pending Quit preserved; {:?}",
                app.error
            );
            app.error = None;
            app.observe_project();
            app.poll_saves(&ctx);
            assert_eq!(app.status, "Saving stopped · keep this document open");
            assert_eq!(app.editor.dirty(), dirty);
            assert!(app.error.is_none());
            if dirty {
                // A normal catalog edit replaces the transient status. The
                // actual toolbar must still show terminal persistence state.
                app.insert_catalog_at(
                    wireframe::workspace::CatalogItem::Control(Kind::Label),
                    egui::pos2(60., 70.),
                );
                assert_eq!(app.status, "Added Text Label");
                app.observe_project();
                ctx.enable_accesskit();
                let mut labels = Vec::new();
                for _ in 0..2 {
                    let output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(1200., 850.),
                            )),
                            ..Default::default()
                        },
                        |ui| app.toolbar(ui),
                    );
                    labels = output
                        .platform_output
                        .accesskit_update
                        .as_ref()
                        .unwrap()
                        .nodes
                        .iter()
                        .filter_map(|(_, node)| {
                            node.label().or_else(|| node.value()).map(str::to_owned)
                        })
                        .collect();
                    output.drop_without_applying_deltas();
                }
                assert!(
                    labels
                        .iter()
                        .any(|label| label == "Saving stopped · keep this document open"),
                    "toolbar labels: {labels:?}"
                );
                assert!(!labels.iter().any(|label| label == "Added Text Label"));
                assert!(app.persistence.as_ref().unwrap().failed);
                app.editor.undo();
                assert_eq!(
                    local_save_editor_state(&app.editor)["document"],
                    state["document"]
                );
                app.editor.redo();
                eprintln!(
                    "INJECTED-DISCONNECT: stopped toolbar status survives dismissal and normal catalog edit/status"
                );
            }
            let finish_state = local_save_editor_state(&app.editor);
            let finish_history = local_save_history(&mut app.editor);
            let finish_dirty = app.editor.dirty();
            assert!(!app.finish_session(false));
            // Preserve the existing take/failed=false/return=false lifecycle.
            assert!(app.persistence.as_ref().unwrap().session.is_none());
            assert!(!app.persistence.as_ref().unwrap().failed);
            assert!(!app.save_busy());
            assert_local_save_stopped_feedback(&app);
            assert_eq!(local_save_editor_state(&app.editor), finish_state);
            assert_eq!(local_save_history(&mut app.editor), finish_history);
            assert_eq!(app.editor.dirty(), finish_dirty);
            assert_eq!(std::fs::read(&path).unwrap(), saved_bytes);
            assert!(matches!(app.pending, Some(Pending::Quit)));
            assert!(!app.quit_allowed);
            eprintln!(
                "INJECTED-DISCONNECT finish error: graph/history/dirty/bytes/pending preserved, session taken and failed cleared as before; {:?}",
                app.error
            );
        }
    }
}

fn save_focus_frame(
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
fn save_focus_key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers,
    }
}

#[test]
fn named_save_restores_navigator_focus_before_the_next_outdent_and_arrow() {
    page_save_focus(false);
}

#[test]
fn file_menu_save_restores_page_focus_before_the_next_outdent_and_arrow() {
    page_save_focus(true);
}

fn page_save_focus(menu: bool) {
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
        let gate = SaveFocusGate(Arc::new((Mutex::new(false), Condvar::new())));
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
            let out = save_focus_frame(&ctx, &mut app, vec![]);
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
            save_focus_frame(
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
        save_focus_frame(
            &ctx,
            &mut app,
            vec![save_focus_key(egui::Key::Tab, egui::Modifiers::NONE)],
        )
        .drop_without_applying_deltas();
        assert_eq!(
            app.editor.document.live_page(second).unwrap().parent,
            Some(first)
        );
        let saved = app.editor.document.clone();
        if menu {
            save_focus_click(&ctx, &mut app, "File");
            save_focus_click(&ctx, &mut app, "Save                   ⌘/Ctrl S");
        } else {
            save_focus_frame(
                &ctx,
                &mut app,
                vec![save_focus_key(egui::Key::S, egui::Modifiers::COMMAND)],
            )
            .drop_without_applying_deltas();
        }
        assert!(app.save_busy());
        for _ in 0..busy_frames {
            save_focus_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
        }
        gate.release();
        if key_on_completion {
            // Deliver the next key on the first UI pass after completion, while
            // egui still remembers the previous pass's saving modal.
            drain(&mut app, &ctx);
        } else {
            let deadline = Instant::now() + Duration::from_secs(12);
            while app.save_busy() {
                save_focus_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
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
        save_focus_frame(
            &ctx,
            &mut app,
            vec![save_focus_key(egui::Key::Tab, egui::Modifiers::SHIFT)],
        )
        .drop_without_applying_deltas();
        assert_eq!(app.editor.document.live_page(second).unwrap().parent, None);
        save_focus_frame(
            &ctx,
            &mut app,
            vec![save_focus_key(egui::Key::ArrowDown, egui::Modifiers::NONE)],
        )
        .drop_without_applying_deltas();
        assert_eq!(app.editor.page, third);
        save_focus_frame(
            &ctx,
            &mut app,
            vec![save_focus_key(egui::Key::Z, egui::Modifiers::COMMAND)],
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

#[test]
fn named_save_restores_source_range_focus_before_the_next_arrow() {
    source_range_save_focus(false);
}

#[test]
fn file_menu_save_restores_source_range_focus_before_the_next_arrow() {
    source_range_save_focus(true);
}

fn source_range_save_focus(menu: bool) {
    for busy_frames in [0, 3] {
        let (dir, ctx, mut app) = fixture();
        ctx.enable_accesskit();
        let library = app.editor.create_library("Sources").unwrap();
        let first = app
            .editor
            .create_empty_source(library, "First source", Default::default())
            .unwrap();
        let second = app
            .editor
            .create_empty_source(library, "Second source", Default::default())
            .unwrap();
        app.editor.edit_library_source(first).unwrap();
        let path = dir.path().join("Sources.wireframe");
        ProjectStore.save(&path, &app.editor.document).unwrap();
        app.file = Some(path.clone());
        app.editor.mark_saved();
        let saved = app.editor.document.clone();
        let history = app.editor.undo_name().map(str::to_owned);
        let gate = SaveFocusGate(Arc::new((Mutex::new(false), Condvar::new())));
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
            let out = save_focus_frame(&ctx, &mut app, vec![]);
            let node = &out
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .find(|(_, n)| n.label() == Some("First source"))
                .unwrap()
                .1;
            let b = node.bounds().unwrap();
            point = egui::pos2(((b.x0 + b.x1) / 2.) as f32, ((b.y0 + b.y1) / 2.) as f32);
            out.drop_without_applying_deltas();
        }
        for pressed in [true, false] {
            save_focus_frame(
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
        assert!(app.source_navigation_focused(&ctx));
        save_focus_frame(
            &ctx,
            &mut app,
            vec![save_focus_key(egui::Key::ArrowDown, egui::Modifiers::SHIFT)],
        )
        .drop_without_applying_deltas();
        assert_eq!(app.source_navigation.selected.len(), 2);
        assert_eq!(app.editor.definition, Some(first));
        if menu {
            save_focus_click(&ctx, &mut app, "File");
            save_focus_click(&ctx, &mut app, "Save                   ⌘/Ctrl S");
        } else {
            save_focus_frame(
                &ctx,
                &mut app,
                vec![save_focus_key(egui::Key::S, egui::Modifiers::COMMAND)],
            )
            .drop_without_applying_deltas();
        }
        assert!(app.save_busy());
        for _ in 0..busy_frames {
            save_focus_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
        }
        gate.release();
        drain(&mut app, &ctx);
        // The first post-save key contracts the range from the previously focused second row.
        save_focus_frame(
            &ctx,
            &mut app,
            vec![save_focus_key(egui::Key::ArrowUp, egui::Modifiers::SHIFT)],
        )
        .drop_without_applying_deltas();
        assert!(app.source_navigation_focused(&ctx));
        assert_eq!(
            app.source_navigation.selected,
            HashSet::from([ResourceRow::Source(first)])
        );
        save_focus_frame(
            &ctx,
            &mut app,
            vec![save_focus_key(egui::Key::ArrowDown, egui::Modifiers::NONE)],
        )
        .drop_without_applying_deltas();
        assert_eq!(app.editor.definition, Some(second));
        assert_eq!(app.editor.document, saved);
        assert_eq!(ProjectStore.load(&path).unwrap(), saved);
        assert_eq!(app.editor.undo_name(), history.as_deref());
        app.on_exit(None);
    }
}

fn save_focus_click(ctx: &egui::Context, app: &mut WireframeApp, label: &str) {
    let output = save_focus_frame(ctx, app, vec![]);
    let node = output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some(label))
        .unwrap_or_else(|| panic!("Missing {label}"));
    assert!(!node.1.is_disabled(), "Disabled {label}");
    let b = node.1.bounds().unwrap();
    let point = egui::pos2(((b.x0 + b.x1) / 2.) as f32, ((b.y0 + b.y1) / 2.) as f32);
    output.drop_without_applying_deltas();
    for pressed in [true, false] {
        save_focus_frame(
            ctx,
            app,
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
}

#[test]
fn preference_feedback_preserves_document_stopped_priority_with_both_real_failures() {
    let (dir, ctx, mut app) = fixture();
    app.set_save_mode(false);
    app.editor.add(Kind::Button, 10., 20.);
    let path = dir.path().join("Already-saved.wireframe");
    ProjectStore.save(&path, &app.editor.document).unwrap();
    let saved = std::fs::read(&path).unwrap();
    app.file = Some(path.clone());
    app.editor.mark_saved();
    // Reuse the existing controlled actor-disconnect setup. It is not an OS crash.
    let gate = SaveFocusGate(Arc::new((Mutex::new(false), Condvar::new())));
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
                let mut released = wait.0.lock().unwrap();
                while !*released {
                    released = wait.1.wait(released).unwrap();
                }
                drop(released);
                panic!("test-only document actor disconnect for preference priority");
            }),
        )
        .unwrap(),
    );
    app.start_presentation();
    app.remember_presentation();
    let prefs = dir.path().join("presentation-settings.json");
    std::fs::rename(&prefs, dir.path().join("preferences-baseline.json")).unwrap();
    std::fs::create_dir(&prefs).unwrap();
    app.presentation_view
        .session
        .toggle(wireframe::presentation::Aid::BigCursor);
    app.remember_presentation(); // A real independent preference persist failure.
    assert_eq!(
        app.status_text(),
        "Couldn't save presentation settings. Your choices still work until you close Wireframe."
    );
    gate.release();
    let deadline = Instant::now() + Duration::from_secs(12);
    while app.status_text() != "Saving stopped · keep this document open" {
        app.poll_saves(&ctx);
        assert!(
            Instant::now() < deadline,
            "document actor did not disconnect"
        );
        std::thread::yield_now();
    }
    assert_local_save_stopped(&app);
    let document = app.editor.document.clone();
    app.error = None; // Existing acknowledgement does not clear either persistence state.
    app.present = false;
    ctx.enable_accesskit();
    let mut labels = vec![];
    for _ in 0..3 {
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(920., 620.),
                )),
                ..Default::default()
            },
            |ui| {
                app.toolbar(ui);
                app.remember_presentation();
            },
        );
        labels = output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .filter_map(|(_, n)| n.label().or_else(|| n.value()).map(str::to_owned))
            .collect::<Vec<_>>();
        output.drop_without_applying_deltas();
    }
    assert!(
        labels
            .iter()
            .any(|s| s == "Saving stopped · keep this document open"),
        "Toolbar tree: {labels:?}"
    );
    assert!(
        !labels
            .iter()
            .any(|s| s.starts_with("Couldn't save presentation settings."))
    );
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    assert_eq!(app.editor.document, document);
    std::fs::remove_dir(&prefs).unwrap();
    app.remember_presentation(); // Successful preference receipt must not dismiss stopped document saving.
    assert_eq!(
        app.status_text(),
        "Saving stopped · keep this document open"
    );
    assert!(app.persistence.as_ref().unwrap().failed);
    app.on_exit(None);
}
