// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

fn frame(ctx: &egui::Context, app: &mut WireframeApp, events: Vec<egui::Event>) {
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 800.))),
            time: Some(ctx.cumulative_frame_nr() as f64 * 0.05),
            events,
            ..Default::default()
        },
        |ui| app.canvas(ui),
    )
    .drop_without_applying_deltas();
}
fn click(ctx: &egui::Context, app: &mut WireframeApp, point: Pos2) {
    for pressed in [true, false] {
        frame(
            ctx,
            app,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
}

#[test]
fn image_double_click_queues_import_without_editing_text_or_document() {
    for kind in [Kind::Image, Kind::CoverFlow] {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(kind, 50., 50.);
        let original = app.editor.document.clone();
        let control = app.editor.current().controls[0].clone();
        frame(&ctx, &mut app, vec![]);
        let pos = app.screen([
            control.bounds.x + control.bounds.w / 2.,
            control.bounds.y + control.bounds.h / 2.,
        ]);
        click(&ctx, &mut app, pos);
        click(&ctx, &mut app, pos);
        assert_eq!(app.image_source_request, Some(control.id), "{kind:?}");
        assert!(app.text_edit.is_none());
        assert!(app.icon_dialog.is_none());
        assert_eq!(app.editor.document, original);
    }
}

#[test]
fn group_gap_entry_and_empty_double_click_change_only_one_context_level() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 50., 50.);
    app.editor.add(Kind::Button, 50., 250.);
    app.editor.selection = app.editor.current().controls.iter().map(|c| c.id).collect();
    app.editor.group();
    let inner = app.editor.current().controls[0].id;
    app.editor.add(Kind::Button, 400., 50.);
    app.editor.selection = app.editor.current().controls.iter().map(|c| c.id).collect();
    app.editor.group();
    let outer = app.editor.current().controls[0].id;
    assert!(app.editor.enter(outer));
    let original = app.editor.document.clone();
    let undo = app.editor.undo_name().map(str::to_owned);
    frame(&ctx, &mut app, vec![]);
    let gap = app.screen([70., 150.]);
    click(&ctx, &mut app, gap);
    click(&ctx, &mut app, gap);
    assert_eq!(app.editor.context, [outer, inner]);
    assert!(app.editor.selection.is_empty());
    // Start a separate click sequence after the framework triple-click interval.
    for _ in 0..13 {
        frame(&ctx, &mut app, vec![]);
    }
    let blank = app.screen([750., 550.]);
    click(&ctx, &mut app, blank);
    assert_eq!(app.editor.context, [outer, inner]);
    click(&ctx, &mut app, blank);
    assert_eq!(app.editor.context, [outer]);
    assert!(app.editor.selection.is_empty());
    assert_eq!(app.editor.document, original);
    assert_eq!(app.editor.undo_name(), undo.as_deref());
}

#[test]
fn empty_double_click_leaves_source_for_its_library_without_editing_or_history() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 50., 50.);
    app.editor.add(Kind::Button, 50., 250.);
    app.editor.select_all();
    app.editor.group();
    let definition = app
        .editor
        .create_component("Nested controls".into())
        .unwrap();
    app.editor.edit_library_source(definition).unwrap();
    let library = app.editor.library.unwrap();
    let ungrouped = app.editor.document.clone();
    app.editor.select_all();
    app.editor.group();
    let group = *app.editor.selection.iter().next().unwrap();
    assert!(app.editor.enter(group));
    let document = app.editor.document.clone();
    let undo = app.editor.undo_name().map(str::to_owned);
    frame(&ctx, &mut app, vec![]);
    let blank = app.screen([700., 550.]);
    click(&ctx, &mut app, blank);
    assert_eq!(app.editor.context, [group]);
    click(&ctx, &mut app, blank);
    assert!(app.editor.context.is_empty());
    assert_eq!(app.editor.definition, Some(definition));
    assert!(app.editor.selection.is_empty());
    for _ in 0..13 {
        frame(&ctx, &mut app, vec![]);
    }
    click(&ctx, &mut app, blank);
    click(&ctx, &mut app, blank);
    assert_eq!(
        app.editor.definition, None,
        "blank double-click leaves source root"
    );
    assert_eq!(app.editor.library, Some(library));
    assert!(app.editor.context.is_empty());
    assert!(app.editor.selection.is_empty());
    assert_eq!(app.editor.document, document);
    assert_eq!(app.editor.undo_name(), undo.as_deref());
    // The library overview is the canvas root; another blank double-click stays.
    for _ in 0..13 {
        frame(&ctx, &mut app, vec![]);
    }
    click(&ctx, &mut app, blank);
    click(&ctx, &mut app, blank);
    assert_eq!(app.editor.definition, None);
    assert_eq!(app.editor.library, Some(library));
    assert_eq!(app.editor.document, document);
    assert_eq!(app.editor.undo_name(), undo.as_deref());
    app.editor.undo();
    assert_eq!(
        app.editor.document, ungrouped,
        "one Undo removes only Group"
    );
    app.editor.redo();
    assert_eq!(app.editor.document, document, "one Redo restores Group");
}
