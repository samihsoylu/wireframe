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

#[test]
fn pdf_options_block_document_shortcuts_until_the_dialog_is_closed() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 50., 50.);
    let before = app.editor.document.clone();
    app.pdf_options = Some(wireframe::pdf::Options::default());
    dispatch(&ctx, &mut app, vec![key(Key::Z, false), key(Key::N, false)]);
    assert_eq!(app.editor.document, before);
    assert!(app.pending.is_none());
    app.pdf_options = None;
    dispatch(&ctx, &mut app, vec![key(Key::Z, false)]);
    assert_ne!(app.editor.document, before);
}

#[test]
fn save_between_field_edits_preserves_the_snapshot_and_later_input_order() {
    for (later, asynchronous, late) in [
        (egui::Event::Text("after".into()), false, false),
        (key(Key::Z, false), false, false),
        (egui::Event::Text("after".into()), true, false),
        (key(Key::Z, false), true, false),
        (egui::Event::Text("after".into()), true, true),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.file = Some(dir.path().join("test.wireframe"));
        app.editor.document.title.clear();
        if asynchronous {
            ProjectStore
                .save(app.file.as_ref().unwrap(), &app.editor.document)
                .unwrap();
            app.enable_persistence(&ctx, dir.path().join("local"));
        }
        let mut run = |events| {
            let mut raw = egui::RawInput {
                events,
                ..Default::default()
            };
            app.order_text_input(&ctx, &mut raw);
            let mut output = ctx.run_ui(raw, |ui| {
                // Exercise a disabled busy frame even if this small test save
                // has already finished on the worker before we poll it.
                if app.save_busy() {
                    ui.disable();
                }
                app.poll_saves(ui.ctx());
                app.shortcuts(ui.ctx());
                ui.add(TextInput::singleline(&mut app.editor.document.title).autofocus(true));
            });
            output.textures_delta.clear();
        };
        run(vec![]);
        if late {
            run(vec![egui::Event::Text("before".into()), key(Key::S, false)]);
            run(vec![]); // Dispatch Save with no later input queued yet.
            run(vec![later.clone()]); // New input arrives during the busy frame.
        } else {
            run(vec![
                egui::Event::Text("before".into()),
                key(Key::S, false),
                later.clone(),
            ]);
        }
        for _ in 0..50 {
            run(vec![]);
            if asynchronous {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        assert_eq!(
            ProjectStore.load(app.file.as_ref().unwrap()).unwrap().title,
            "before"
        );
        assert_eq!(
            app.editor.document.title,
            if matches!(later, egui::Event::Text(_)) {
                "beforeafter"
            } else {
                ""
            }
        );
        assert!(app.deferred_text_input.is_empty());
    }
}

#[test]
fn text_after_tab_reaches_the_next_field_even_when_focus_wraps() {
    for reverse in [false, true] {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        let mut values = [String::new(), String::new()];
        let mut run = |events| {
            let mut raw = egui::RawInput {
                events,
                ..Default::default()
            };
            app.order_text_input(&ctx, &mut raw);
            let mut output = ctx.run_ui(raw, |ui| {
                for index in if reverse { [1, 0] } else { [0, 1] } {
                    ui.add(
                        TextInput::singleline(&mut values[index])
                            .id_salt(index)
                            .autofocus(index == 0),
                    );
                }
            });
            output.textures_delta.clear();
        };
        run(vec![]);
        run(vec![
            egui::Event::Text("before".into()),
            egui::Event::Key {
                key: Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
            egui::Event::Text("after".into()),
        ]);
        run(vec![]);
        assert_eq!(values, ["before", "after"], "reverse={reverse}");
    }
}

#[test]
fn clicking_save_after_typing_captures_the_updated_field() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.file = Some(dir.path().join("test.wireframe"));
    app.editor.document.title.clear();
    let mut run = |events| {
        let mut raw = egui::RawInput {
            events,
            ..Default::default()
        };
        app.order_text_input(&ctx, &mut raw);
        let mut rect = Rect::NOTHING;
        let mut output = ctx.run_ui(raw, |ui| {
            let response = ui.button("Save");
            rect = response.rect;
            if response.clicked() {
                app.save(false);
            }
            ui.add(TextInput::singleline(&mut app.editor.document.title).autofocus(true));
        });
        output.textures_delta.clear();
        rect
    };
    let pos = run(vec![]).center();
    run(vec![
        egui::Event::Text("complete".into()),
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ]);
    run(vec![]);
    assert_eq!(
        ProjectStore.load(app.file.as_ref().unwrap()).unwrap().title,
        "complete"
    );
}
