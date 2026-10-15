// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
fn key(key: Key, shift: bool) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: if shift {
            Modifiers::COMMAND | Modifiers::SHIFT
        } else {
            Modifiers::COMMAND
        },
    }
}
fn dispatch(ctx: &egui::Context, app: &mut WireframeApp, events: Vec<egui::Event>) {
    let mut output = ctx.run_ui(
        egui::RawInput {
            events,
            ..Default::default()
        },
        |ui| app.shortcuts(ui.ctx()),
    );
    output.textures_delta.clear();
}
#[test]
fn undo_then_save_in_one_frame_persists_the_undone_state() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.file = Some(dir.path().join("test.wireframe"));
    app.editor.add(Kind::Video, 50., 50.);
    let original = app.editor.document.clone();
    app.editor
        .edit("Change style", |e| e.document.settings.sketch = false);
    dispatch(&ctx, &mut app, vec![key(Key::Z, false), key(Key::S, false)]);
    assert_eq!(app.editor.document, original);
    assert_eq!(
        ProjectStore.load(app.file.as_ref().unwrap()).unwrap(),
        original
    );
    assert!(!app.editor.dirty());
}
#[test]
fn save_then_undo_keeps_the_saved_snapshot_and_marks_the_editor_dirty() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.file = Some(dir.path().join("test.wireframe"));
    app.editor.add(Kind::Video, 50., 50.);
    let original = app.editor.document.clone();
    app.editor
        .edit("Change style", |e| e.document.settings.sketch = false);
    let edited = app.editor.document.clone();
    dispatch(&ctx, &mut app, vec![key(Key::S, false), key(Key::Z, false)]);
    assert_eq!(app.editor.document, original);
    assert_eq!(
        ProjectStore.load(app.file.as_ref().unwrap()).unwrap(),
        edited
    );
    assert!(app.editor.dirty());
}
#[test]
fn repeated_history_events_and_redo_save_preserve_event_order() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.file = Some(dir.path().join("test.wireframe"));
    app.editor.add(Kind::Video, 50., 50.);
    let one = app.editor.document.clone();
    app.editor.add(Kind::Webcam, 400., 50.);
    let two = app.editor.document.clone();
    app.editor.add(Kind::Playback, 50., 300.);
    dispatch(
        &ctx,
        &mut app,
        vec![key(Key::Z, false), key(Key::Z, false), key(Key::S, false)],
    );
    assert_eq!(app.editor.document, one);
    assert_eq!(ProjectStore.load(app.file.as_ref().unwrap()).unwrap(), one);
    dispatch(&ctx, &mut app, vec![key(Key::Z, true), key(Key::S, false)]);
    assert_eq!(app.editor.document, two);
    assert_eq!(ProjectStore.load(app.file.as_ref().unwrap()).unwrap(), two);
}
#[test]
fn new_project_confirmation_blocks_later_save_in_the_same_frame() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    let path = dir.path().join("test.wireframe");
    app.file = Some(path.clone());
    app.editor.add(Kind::Video, 50., 50.);
    let expected = app.editor.document.clone();
    dispatch(&ctx, &mut app, vec![key(Key::N, false), key(Key::S, false)]);
    assert!(app.pending.is_some());
    assert_eq!(app.editor.document, expected);
    assert!(!path.exists());
}
#[test]
fn new_page_shortcut_does_not_trigger_new_project() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Video, 50., 50.);
    let first = app.editor.document.pages[0].clone();
    dispatch(&ctx, &mut app, vec![key(Key::N, true)]);
    assert!(app.pending.is_none());
    assert_eq!(app.editor.document.pages.len(), 2);
    assert_eq!(app.editor.document.pages[0], first);
}
#[test]
fn movement_repeats_and_save_use_the_ordered_input_for_each_platform_modifier() {
    for command in [
        Modifiers::COMMAND,
        Modifiers::CTRL | Modifiers::COMMAND,
        Modifiers::MAC_CMD | Modifiers::COMMAND,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.file = Some(dir.path().join("test.wireframe"));
        app.editor.add(Kind::Video, 50., 50.);
        let arrow = |pressed, repeat| egui::Event::Key {
            key: Key::ArrowRight,
            physical_key: None,
            pressed,
            repeat,
            modifiers: Modifiers::NONE,
        };
        let save = || egui::Event::Key {
            key: Key::S,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: command,
        };
        dispatch(
            &ctx,
            &mut app,
            vec![
                arrow(true, false),
                arrow(true, true),
                arrow(false, false),
                save(),
                arrow(true, true),
            ],
        );
        assert_eq!(app.editor.selected_views()[0].bounds.x, 53.);
        let saved = ProjectStore.load(app.file.as_ref().unwrap()).unwrap();
        assert_eq!(saved.pages[0].content.nodes[0].origin.x, 52.);
        assert!(app.editor.dirty());
    }
}
#[test]
fn duplicate_copy_and_delete_follow_the_input_order() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Video, 50., 50.);
    let original = *app.editor.selection.iter().next().unwrap();
    dispatch(
        &ctx,
        &mut app,
        vec![key(Key::D, false), egui::Event::Cut, key(Key::Z, false)],
    );
    assert_eq!(app.editor.current().controls.len(), 2);
    assert_eq!(app.clipboard.len(), 1);
    assert!(
        app.editor
            .current()
            .controls
            .iter()
            .any(|c| c.id == original)
    );
}
