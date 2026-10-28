// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

fn frame(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    mut events: Vec<egui::Event>,
    modifiers: Modifiers,
    focused: bool,
) {
    events.insert(0, egui::Event::ModifiersChanged(modifiers));
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 850.))),
            events,
            focused,
            ..Default::default()
        },
        |ui| {
            app.shortcuts(ui.ctx());
            // Match production ordering: the selected-control inspector paints
            // before the canvas processes release or window deactivation.
            app.inspector(ui);
            app.canvas(ui);
        },
    )
    .drop_without_applying_deltas();
}

fn pointer(pos: Pos2, pressed: bool, modifiers: Modifiers) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers,
    }
}

#[test]
fn inspector_keeps_single_control_move_open_until_final_release_position() {
    for kind in [Kind::TextInput, Kind::CircleButton] {
        for duplicate in [false, true] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(kind, 50., 50.);
            let id = app.editor.nodes()[0].id;
            let before = app.editor.document.clone();
            frame(&ctx, &mut app, vec![], Modifiers::NONE, true);
            let start = app.screen([70., 60.]);
            let middle = start + Vec2::new(60., 30.);
            let end = start + Vec2::new(90., 60.);
            let modifiers = Modifiers {
                alt: duplicate,
                command: true,
                ..Modifiers::NONE
            };
            frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::PointerMoved(start),
                    pointer(start, true, modifiers),
                ],
                modifiers,
                true,
            );
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(middle)],
                modifiers,
                true,
            );
            assert!(app.drag.is_some(), "{kind:?}, duplicate={duplicate}");
            frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::PointerMoved(end),
                    pointer(end, false, modifiers),
                ],
                modifiers,
                true,
            );
            assert!(
                app.error.is_none(),
                "{kind:?}, duplicate={duplicate}: {:?}",
                app.error
            );
            assert!(app.drag.is_none());
            let nodes = app.editor.nodes();
            assert_eq!(nodes.len(), if duplicate { 2 } else { 1 });
            let moved = nodes.last().unwrap();
            assert_eq!(moved.origin, wireframe::document::Point::new(140., 110.));
            if duplicate {
                assert_eq!(nodes[0], before.pages[0].content.nodes[0]);
                assert_ne!(moved.id, id);
            }
            let after = app.editor.document.clone();
            app.editor.undo();
            assert_eq!(app.editor.document, before);
            app.editor.redo();
            assert_eq!(app.editor.document, after);
        }
    }
}

#[test]
fn inspector_does_not_save_move_preview_before_window_focus_cancellation() {
    for duplicate in [false, true] {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::TextInput, 50., 50.);
        app.editor.add(Kind::Checkbox, 300., 50.);
        let redo = app.editor.document.clone();
        app.editor.undo();
        let before = app.editor.document.clone();
        let undo_name = app.editor.undo_name().map(str::to_owned);
        frame(&ctx, &mut app, vec![], Modifiers::NONE, true);
        let start = app.screen([70., 60.]);
        let end = start + Vec2::new(90., 60.);
        let modifiers = Modifiers {
            alt: duplicate,
            command: true,
            ..Modifiers::NONE
        };
        frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(start),
                pointer(start, true, modifiers),
            ],
            modifiers,
            true,
        );
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(end)],
            modifiers,
            true,
        );
        assert_ne!(app.editor.document, before);
        // Native deactivation can deliver its button release in the same frame.
        frame(
            &ctx,
            &mut app,
            vec![pointer(end, false, modifiers)],
            modifiers,
            false,
        );
        assert!(app.error.is_none(), "{:?}", app.error);
        assert!(app.drag.is_none());
        assert!(
            app.editor.document == before,
            "duplicate={duplicate}: cancelled move changed the project"
        );
        assert_eq!(app.editor.undo_name(), undo_name.as_deref());
        app.editor.redo();
        assert_eq!(app.editor.document, redo);
    }
}

#[test]
fn interrupted_move_stops_without_restarting_or_repeating_the_error() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::TextInput, 50., 50.);
    frame(&ctx, &mut app, vec![], Modifiers::NONE, true);
    let start = app.screen([70., 60.]);
    let middle = start + Vec2::new(60., 30.);
    let end = start + Vec2::new(90., 60.);
    frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerMoved(start),
            pointer(start, true, Modifiers::COMMAND),
        ],
        Modifiers::COMMAND,
        true,
    );
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(middle)],
        Modifiers::COMMAND,
        true,
    );
    // Simulate an edit boundary ending the model edit while input still owns
    // a stale drag. Later input must not rebase or extend the finished move.
    app.editor.commit();
    let finished = app.editor.document.clone();
    let undo = app.editor.undo_name().map(str::to_owned);
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(end)],
        Modifiers::COMMAND,
        true,
    );
    assert_eq!(
        app.error.as_deref(),
        Some("The move was interrupted. Try dragging again.")
    );
    assert!(app.drag.is_none());
    assert!(app.alignment_guides.is_empty());
    assert!(app.editor.document == finished);
    assert_eq!(app.editor.undo_name(), undo.as_deref());
    app.error = None;
    frame(
        &ctx,
        &mut app,
        vec![pointer(end, false, Modifiers::COMMAND)],
        Modifiers::COMMAND,
        true,
    );
    assert!(app.error.is_none());
    assert!(app.drag.is_none());
    assert!(app.editor.document == finished);
}
