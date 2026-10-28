// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use eframe::App;

fn key(key: Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    }
}
fn frame(
    app: &mut WireframeApp,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    close: bool,
) -> egui::FullOutput {
    let focused = events
        .iter()
        .rev()
        .find_map(|e| match e {
            egui::Event::WindowFocused(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(true);
    let mut raw = egui::RawInput {
        focused,
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200., 850.))),
        events,
        ..Default::default()
    };
    if close {
        raw.viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
    }
    app.raw_input_hook(ctx, &mut raw);
    let mut out = ctx.run_ui(raw, |ui| app.ui(ui, &mut eframe::Frame::_new_kittest()));
    out.textures_delta.clear();
    out
}
fn setup() -> (egui::Context, WireframeApp) {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 40., 50.);
    frame(&mut app, &ctx, vec![], false);
    let out = frame(&mut app, &ctx, vec![], false);
    let bounds = out
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find_map(|(_, n)| {
            (n.value() == Some("Button") && n.role() == egui::accesskit::Role::MultilineTextInput)
                .then(|| n.bounds())
                .flatten()
        })
        .expect("Inspector field");
    let pos = egui::pos2(
        (bounds.x0 + bounds.x1) as f32 / 2.,
        (bounds.y0 + bounds.y1) as f32 / 2.,
    );
    for pressed in [true, false] {
        frame(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
            false,
        );
    }
    assert!(wireframe::text_entry::is_focused(&ctx));
    app.editor.commit();
    app.editor.mark_saved();
    (ctx, app)
}
fn cancelled(out: &egui::FullOutput) -> bool {
    out.viewport_output
        .get(&egui::ViewportId::ROOT)
        .unwrap()
        .commands
        .contains(&egui::ViewportCommand::CancelClose)
}

#[test]
fn window_close_commits_the_canvas_text_session_before_unsaved_review() {
    let (ctx, mut app) = setup();
    let id = app.editor.current().controls[0].id;
    app.begin_text_edit(id);
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![], false);
    }
    let before = app.editor.document.clone();
    let out = frame(
        &mut app,
        &ctx,
        vec![egui::Event::Paste("Final draft".into())],
        true,
    );
    assert!(cancelled(&out));
    assert!(app.text_edit.is_none());
    assert_eq!(app.editor.current().controls[0].text, "Final draft");
    assert!(matches!(app.pending, Some(Pending::Quit)));
    assert!(!app.quit_allowed);
    app.editor.undo();
    assert_eq!(app.editor.document, before);
}

#[test]
fn window_close_after_inspector_input_must_not_discard_the_new_edit() {
    for deactivate in [false, true] {
        let (ctx, mut app) = setup();
        let mut events = vec![key(Key::A), egui::Event::Text("Keep on close".into())];
        if deactivate {
            events.push(egui::Event::WindowFocused(false));
        }
        let out = frame(&mut app, &ctx, events, true);
        assert_eq!(app.editor.current().controls[0].text, "Keep on close");
        assert!(
            cancelled(&out),
            "The window must stay open for newly unsaved text"
        );
        assert!(matches!(app.pending, Some(Pending::Quit)));
    }
}

#[test]
fn close_waits_for_text_after_a_save_and_cancel_does_not_close_later() {
    let (ctx, mut app) = setup();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Manual.wireframe");
    app.file = Some(path.clone());
    ProjectStore.save(&path, &app.editor.document).unwrap();
    let out = frame(
        &mut app,
        &ctx,
        vec![
            key(Key::A),
            egui::Event::Text("Before".into()),
            key(Key::S),
            egui::Event::Text(" after".into()),
        ],
        true,
    );
    assert!(cancelled(&out));
    assert!(app.window_close_pending && app.pending.is_none() && !app.quit_allowed);
    for _ in 0..5 {
        frame(&mut app, &ctx, vec![], false);
    }
    assert_eq!(app.editor.current().controls[0].text, "Before after");
    let saved = Editor::new(ProjectStore.load(&path).unwrap());
    assert_eq!(saved.current().controls[0].text, "Before");
    assert!(matches!(app.pending, Some(Pending::Quit)));
    app.pending = None; // The user cancels the unsaved confirmation.
    for _ in 0..2 {
        frame(&mut app, &ctx, vec![], false);
    }
    assert!(!app.quit_allowed && !app.window_close_pending && app.pending.is_none());
    assert_eq!(app.editor.current().controls[0].text, "Before after");
}

#[test]
fn automatic_close_flushes_input_after_an_in_flight_save_before_allowing_exit() {
    let (ctx, mut app) = setup();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Automatic.wireframe");
    app.file = Some(path.clone());
    ProjectStore.save(&path, &app.editor.document).unwrap();
    app.enable_persistence(&ctx, dir.path().join("local"));
    let out = frame(
        &mut app,
        &ctx,
        vec![
            key(Key::A),
            egui::Event::Text("Before".into()),
            key(Key::S),
            egui::Event::Text(" after".into()),
        ],
        true,
    );
    assert!(cancelled(&out));
    let deadline = std::time::Instant::now() + Duration::from_secs(12);
    while !app.quit_allowed {
        frame(&mut app, &ctx, vec![], false);
        assert!(
            std::time::Instant::now() < deadline,
            "Close did not finish: {:?}",
            app.error
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(app.error.is_none(), "{:?}", app.error);
    assert_eq!(app.editor.current().controls[0].text, "Before after");
    assert!(ProjectStore.load(&path).unwrap() == app.editor.document);
    assert!(app.deferred_text_input.is_empty());
    app.on_exit(None);
}

#[test]
fn clean_window_close_is_acknowledged_and_does_not_cancel_its_second_request() {
    let (ctx, mut app) = setup();
    let out = frame(&mut app, &ctx, vec![], true);
    assert!(app.quit_allowed && app.pending.is_none());
    assert!(
        out.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .contains(&egui::ViewportCommand::Close)
    );
    let out = frame(&mut app, &ctx, vec![], true);
    assert!(!cancelled(&out));
}

#[test]
fn minimized_logic_only_close_preserves_input_until_the_window_can_prompt() {
    let (ctx, mut app) = setup();
    let original = app.editor.document.clone();
    let mut raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200., 850.))),
        events: vec![
            key(Key::A),
            egui::Event::Text("Retained while minimized".into()),
        ],
        ..Default::default()
    };
    let viewport = raw.viewports.get_mut(&egui::ViewportId::ROOT).unwrap();
    viewport.minimized = Some(true);
    viewport.events.push(egui::ViewportEvent::Close);
    app.raw_input_hook(&ctx, &mut raw);
    let out = ctx.run_logic(&raw, |ctx| {
        app.logic(ctx, &mut eframe::Frame::_new_kittest())
    });
    let commands = &out.viewport_commands[&egui::ViewportId::ROOT];
    assert!(commands.contains(&egui::ViewportCommand::CancelClose));
    assert!(commands.contains(&egui::ViewportCommand::Minimized(false)));
    assert!(app.window_close_pending && !app.quit_allowed);
    assert!(app.editor.document == original);
    raw.viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .minimized = Some(false);
    let mut out = ctx.run_ui(raw, |ui| app.ui(ui, &mut eframe::Frame::_new_kittest()));
    out.textures_delta.clear();
    assert_eq!(
        app.editor.current().controls[0].text,
        "Retained while minimized"
    );
    assert!(matches!(app.pending, Some(Pending::Quit)));
    assert!(!app.quit_allowed);
}

#[test]
fn escape_cancels_native_close_review_without_losing_the_committed_draft() {
    let (ctx, mut app) = setup();
    let original = app.editor.document.clone();
    let id = app.editor.current().controls[0].id;
    app.begin_text_edit(id);
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![], false);
    }
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Paste("Retain this draft".into())],
        true,
    );
    assert!(matches!(app.pending, Some(Pending::Quit)));
    frame(&mut app, &ctx, vec![], false);
    let mut escape = key(Key::Escape);
    if let egui::Event::Key { modifiers, .. } = &mut escape {
        *modifiers = Modifiers::NONE;
    }
    frame(&mut app, &ctx, vec![escape], false);
    assert!(app.pending.is_none(), "Escape must cancel the close review");
    assert!(!app.quit_allowed && !app.window_close_pending);
    assert_eq!(app.editor.current().controls[0].text, "Retain this draft");
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![], false);
    }
    assert!(!app.quit_allowed && app.pending.is_none());
    app.editor.undo();
    assert_eq!(app.editor.document, original);
}

#[test]
fn escape_dismisses_only_the_top_error_before_cancelling_unsaved_review() {
    let (ctx, mut app) = setup();
    app.editor.add(Kind::Label, 20., 30.);
    let changed = app.editor.document.clone();
    app.pending = Some(Pending::Quit);
    app.error = Some("The destination is unavailable. Your work remains open.".into());
    for _ in 0..3 {
        let out = frame(&mut app, &ctx, vec![], false);
        let nodes = &out.platform_output.accesskit_update.as_ref().unwrap().nodes;
        assert!(nodes.iter().any(|(_, n)| {
            n.label().or_else(|| n.value()) == Some("Couldn't complete that action")
        }));
        assert!(
            !nodes
                .iter()
                .any(|(_, n)| { n.label().or_else(|| n.value()) == Some("Save your changes?") }),
            "the save review must not obscure the error that needs acknowledgement"
        );
    }
    let escape = egui::Event::Key {
        key: Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    frame(&mut app, &ctx, vec![escape.clone()], false);
    assert!(app.error.is_none(), "Escape must dismiss the visible error");
    assert!(matches!(app.pending, Some(Pending::Quit)));
    assert_eq!(app.editor.document, changed);
    frame(&mut app, &ctx, vec![], false);
    frame(&mut app, &ctx, vec![escape], false);
    assert!(app.pending.is_none());
    assert!(!app.quit_allowed);
    assert_eq!(app.editor.document, changed);
}

#[test]
fn synchronous_save_error_never_competes_with_the_pending_close_review() {
    let (ctx, mut app) = setup();
    app.editor.add(Kind::Label, 20., 30.);
    let changed = app.editor.document.clone();
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("blocked.wireframe");
    std::fs::create_dir(&destination).unwrap();
    app.file = Some(destination);
    app.pending = Some(Pending::Quit);
    for _ in 0..2 {
        frame(&mut app, &ctx, vec![], false);
    }
    let out = frame(&mut app, &ctx, vec![], false);
    let bounds = out
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find_map(|(_, n)| (n.label() == Some("Save")).then(|| n.bounds()).flatten())
        .expect("pending close Save button");
    let position = egui::pos2(
        (bounds.x0 + bounds.x1) as f32 / 2.,
        (bounds.y0 + bounds.y1) as f32 / 2.,
    );
    let mut saw_error = false;
    for events in [
        vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
        vec![egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
        vec![],
        vec![],
        vec![],
    ] {
        let out = frame(&mut app, &ctx, events, false);
        let labels: Vec<_> = out
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .filter_map(|(_, n)| n.label().or_else(|| n.value()))
            .collect();
        let error_visible = labels.contains(&"Couldn't complete that action");
        saw_error |= error_visible;
        assert!(
            !(error_visible && labels.contains(&"Save your changes?")),
            "a failure created by Save must be presented separately from its review"
        );
        assert!(matches!(app.pending, Some(Pending::Quit)));
        assert!(!app.quit_allowed);
        assert_eq!(app.editor.document, changed);
    }
    assert!(app.error.is_some() && saw_error);
}

#[cfg(target_os = "macos")]
#[test]
fn appkit_quit_routes_pending_input_and_cancellation_through_window_close() {
    let (ctx, mut app) = setup();
    app.native_termination = Some(native_termination::NativeTermination::simulated(&ctx));
    app.native_termination.as_ref().unwrap().request_for_test();
    let out = frame(
        &mut app,
        &ctx,
        vec![
            key(Key::A),
            egui::Event::Text("Native quit draft 世界".into()),
        ],
        false,
    );
    assert!(cancelled(&out));
    assert_eq!(
        app.editor.current().controls[0].text,
        "Native quit draft 世界"
    );
    assert!(matches!(app.pending, Some(Pending::Quit)));
    assert!(!app.quit_allowed);
    app.pending = None; // Cancel the save review.
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![], false);
    }
    assert!(!app.native_termination.as_ref().unwrap().approved_for_test());
    assert!(!app.window_close_pending && !app.quit_allowed);
    // A new native request after cancellation must work, rather than stay latched.
    app.native_termination.as_ref().unwrap().request_for_test();
    frame(&mut app, &ctx, vec![], false);
    assert!(matches!(app.pending, Some(Pending::Quit)));
}

#[cfg(target_os = "macos")]
#[test]
fn appkit_clean_quit_approves_native_exit_after_the_editor_gate() {
    let (ctx, mut app) = setup();
    let original = app.editor.document.clone();
    app.native_termination = Some(native_termination::NativeTermination::simulated(&ctx));
    app.native_termination.as_ref().unwrap().request_for_test();
    let out = frame(&mut app, &ctx, vec![], false);
    assert!(app.quit_allowed);
    assert!(app.native_termination.as_ref().unwrap().approved_for_test());
    assert_eq!(app.editor.document, original);
    assert!(
        !out.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .contains(&egui::ViewportCommand::Close)
    );
}

#[cfg(target_os = "macos")]
#[test]
fn appkit_quit_does_not_approve_a_failed_or_cancelled_save() {
    let (ctx, mut app) = setup();
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("blocked.wireframe");
    std::fs::create_dir(&destination).unwrap();
    app.file = Some(destination); // A directory cannot be a project destination.
    app.native_termination = Some(native_termination::NativeTermination::simulated(&ctx));
    app.native_termination.as_ref().unwrap().request_for_test();
    frame(
        &mut app,
        &ctx,
        vec![
            key(Key::A),
            egui::Event::Text("Keep after save failure".into()),
        ],
        false,
    );
    assert!(matches!(app.pending, Some(Pending::Quit)));
    assert!(!app.save(false));
    assert!(app.error.is_some());
    app.settle_native_close();
    assert!(!app.quit_allowed && !app.native_termination.as_ref().unwrap().approved_for_test());
    app.pending = None;
    app.settle_native_close();
    assert_eq!(
        app.editor.current().controls[0].text,
        "Keep after save failure"
    );
    assert!(!app.native_termination.as_ref().unwrap().approved_for_test());
}
