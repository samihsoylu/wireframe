// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

use super::*;

fn key(key: Key, modifiers: Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

fn dialog_frame(ctx: &egui::Context, app: &mut WireframeApp, events: Vec<egui::Event>) {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
            events,
            ..Default::default()
        },
        |ui| app.dialogs(ui.ctx()),
    );
    output.textures_delta.clear();
}

#[test]
fn text_modal_accepts_immediate_replacement_and_cancel_reopens_selected() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Heading, 50., 50.);
    let original = app.editor.current().controls[0].clone();
    dialog_frame(&ctx, &mut app, vec![]);
    for replacement in ["Northstar Studio", "Second attempt"] {
        app.begin_text_edit(original.id);
        for _ in 0..3 {
            dialog_frame(&ctx, &mut app, vec![]);
        }
        dialog_frame(&ctx, &mut app, vec![egui::Event::Paste(replacement.into())]);
        assert_eq!(app.text_edit.as_ref().unwrap().1, replacement);
        assert_eq!(app.editor.current().controls[0].text, original.text);
        dialog_frame(&ctx, &mut app, vec![key(Key::Escape, Modifiers::NONE)]);
        assert!(app.text_edit.is_none());
        assert_eq!(app.editor.current().controls[0].text, original.text);
    }
}

#[test]
fn quick_add_title_alias_inserts_a_heading() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.quick_focus = true;
    app.quick = "Title".into();
    for events in [vec![], vec![key(Key::Enter, Modifiers::NONE)]] {
        let mut output = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| app.quick_add_ui(ui),
        );
        output.textures_delta.clear();
    }
    assert_eq!(app.editor.current().controls[0].kind, Kind::Heading);
}

#[test]
fn repeated_insertion_scans_origins_down_the_viewport() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.canvas_rect = Rect::from_min_size(Pos2::new(200., 100.), Vec2::new(700., 600.));
    app.pan = Vec2::ZERO;
    app.zoom = 1.;
    app.add(Kind::Heading);
    app.add(Kind::Button);
    let controls = app.editor.current().controls;
    assert_eq!((controls[0].bounds.x, controls[0].bounds.y), (50., 50.));
    assert_eq!((controls[1].bounds.x, controls[1].bounds.y), (50., 100.));
}

#[test]
fn insertion_respects_zoom_pan_wrapping_and_active_group() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.canvas_rect = Rect::from_min_size(Pos2::new(200., 100.), Vec2::new(700., 100.));
    app.pan = Vec2::new(-150., -250.);
    app.zoom = 2.;
    for expected in [(100., 150.), (100., 175.), (125., 150.)] {
        app.add(Kind::Button);
        let c = app.editor.current().controls.last().unwrap().clone();
        assert_eq!((c.bounds.x, c.bounds.y), expected);
    }
    app.editor.selection = app.editor.current().controls.iter().map(|c| c.id).collect();
    app.editor.group();
    let group = app.editor.current().controls[0].id;
    assert!(app.editor.enter(group));
    app.add(Kind::Button);
    let c = app.editor.current().controls.last().unwrap().clone();
    assert_eq!((c.bounds.x, c.bounds.y), (125., 175.));
    app.editor.undo();
    assert_eq!(app.editor.current().controls.len(), 3);
    app.editor.redo();
    let c = app.editor.current().controls.last().unwrap().clone();
    assert_eq!((c.bounds.x, c.bounds.y), (125., 175.));
}

#[test]
fn insertion_clamps_measured_size_and_terminates_at_occupied_boundary() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.pan = Vec2::splat(-20_000.);
    app.add(Kind::Button);
    let first = app.editor.current().controls[0].bounds;
    assert_eq!((first.x + first.w, first.y + first.h), (10_000., 10_000.));
    app.add(Kind::Button);
    assert_eq!(app.editor.current().controls[1].bounds, first);
    app.pan = Vec2::splat(500.);
    app.add(Kind::Heading);
    let third = app.editor.current().controls[2].bounds;
    assert_eq!((third.x, third.y), (0., 0.));
}

#[test]
fn text_modal_commits_single_line_and_preserves_multiline_newlines_and_undo() {
    for kind in [Kind::Heading, Kind::Button, Kind::TextArea] {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(kind, 50., 50.);
        let original = app.editor.current().controls[0].clone();
        app.begin_text_edit(original.id);
        for _ in 0..3 {
            dialog_frame(&ctx, &mut app, vec![]);
        }
        dialog_frame(&ctx, &mut app, vec![egui::Event::Paste("First".into())]);
        dialog_frame(&ctx, &mut app, vec![key(Key::Enter, Modifiers::NONE)]);
        let expected = if kind == Kind::TextArea {
            assert_eq!(app.text_edit.as_ref().unwrap().1, "First\n");
            dialog_frame(&ctx, &mut app, vec![egui::Event::Text("Second".into())]);
            dialog_frame(&ctx, &mut app, vec![key(Key::Z, Modifiers::COMMAND)]);
            assert_eq!(app.text_edit.as_ref().unwrap().1, "First\n");
            dialog_frame(&ctx, &mut app, vec![egui::Event::Text("Final".into())]);
            dialog_frame(&ctx, &mut app, vec![key(Key::Enter, Modifiers::COMMAND)]);
            "First\nFinal"
        } else {
            "First"
        };
        assert!(app.text_edit.is_none(), "{kind:?} did not commit");
        assert_eq!(app.editor.current().controls[0].text, expected);
        app.editor.undo();
        assert_eq!(app.editor.current().controls[0].text, original.text);
        app.editor.redo();
        assert_eq!(app.editor.current().controls[0].text, expected);
    }
}

#[test]
fn text_modal_does_not_submit_or_close_during_ime_confirmation() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Heading, 50., 50.);
    let id = app.editor.current().controls[0].id;
    app.begin_text_edit(id);
    for _ in 0..3 {
        dialog_frame(&ctx, &mut app, vec![]);
    }
    dialog_frame(
        &ctx,
        &mut app,
        vec![egui::Event::Ime(egui::ImeEvent::Preedit {
            text: "かな".into(),
            active_range_chars: None,
        })],
    );
    dialog_frame(&ctx, &mut app, vec![key(Key::Enter, Modifiers::NONE)]);
    assert!(app.text_edit.is_some());
    dialog_frame(
        &ctx,
        &mut app,
        vec![egui::Event::Ime(egui::ImeEvent::Commit("仮名".into()))],
    );
    assert_eq!(app.text_edit.as_ref().unwrap().1, "仮名");
    dialog_frame(&ctx, &mut app, vec![key(Key::Enter, Modifiers::NONE)]);
    assert!(app.text_edit.is_none());
    assert_eq!(app.editor.current().controls[0].text, "仮名");
}
