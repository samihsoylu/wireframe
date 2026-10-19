// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

fn frame(ctx: &egui::Context, app: &mut WireframeApp, panel: &str, events: Vec<egui::Event>) {
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200., 800.))),
            events,
            ..Default::default()
        },
        |ui| {
            match panel {
                "navigator" => app.sidebar(ui),
                "inspector" => app.inspector(ui),
                _ => app.review_panel(ui),
            }
            app.canvas(ui);
        },
    )
    .drop_without_applying_deltas();
}

#[test]
fn editor_panel_quick_drags_keep_released_width_and_document() {
    let mut failures = Vec::new();
    for panel in ["navigator", "inspector", "review-panel"] {
        for schedule in [0, 1, 2] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            // Artwork extends under both pane boundaries. A leaked press must
            // fail this test even when panel sizing itself appears correct.
            let mut project = Project::default();
            project.pages[0].content.nodes = vec![wireframe::document::Node::control(
                2,
                "rectangle",
                wireframe::document::Point::new(-100., -100.),
                wireframe::document::Size::fixed(2000., 1000.),
            )];
            project.next_id = 3;
            app.editor = Editor::new(project);
            app.pan = Vec2::ZERO;
            let document = app.editor.document.clone();
            let id = egui::Id::new(panel);
            for _ in 0..3 {
                frame(&ctx, &mut app, panel, vec![]);
            }
            let before = egui::containers::panel::PanelState::load(&ctx, id)
                .unwrap()
                .outer_rect;
            let left = panel == "navigator";
            let start = Pos2::new(if left { before.max.x } else { before.min.x }, 200.);
            let end = start + Vec2::new(if left { 50. } else { -50. }, 0.);
            let button = |pos, pressed| egui::Event::PointerButton {
                pos,
                pressed,
                button: egui::PointerButton::Primary,
                modifiers: Modifiers::NONE,
            };
            frame(
                &ctx,
                &mut app,
                panel,
                vec![egui::Event::PointerMoved(start)],
            );
            if schedule == 0 {
                frame(&ctx, &mut app, panel, vec![button(start, true)]);
                frame(&ctx, &mut app, panel, vec![egui::Event::PointerMoved(end)]);
                frame(&ctx, &mut app, panel, vec![button(end, false)]);
            } else if schedule == 1 {
                frame(
                    &ctx,
                    &mut app,
                    panel,
                    vec![
                        button(start, true),
                        egui::Event::PointerMoved(end),
                        button(end, false),
                    ],
                );
            } else {
                frame(&ctx, &mut app, panel, vec![button(start, true)]);
                frame(
                    &ctx,
                    &mut app,
                    panel,
                    vec![egui::Event::PointerMoved(end), button(end, false)],
                );
            }
            for _ in 0..3 {
                frame(&ctx, &mut app, panel, vec![]);
            }
            let after = egui::containers::panel::PanelState::load(&ctx, id)
                .unwrap()
                .outer_rect
                .width();
            if (after - before.width() - 50.).abs() > 1. {
                failures.push(format!(
                    "{panel} schedule{schedule}: {} -> {after}",
                    before.width()
                ));
            }
            assert!(
                app.editor.document == document,
                "{panel} schedule{schedule} changed artwork"
            );
            assert!(app.editor.undo_name().is_none());
            assert!(app.editor.selection.is_empty(), "{panel} selected artwork");
            // The next ordinary canvas gesture must still be delivered.
            let canvas_start = Pos2::new(600., 400.);
            let canvas_end = canvas_start + Vec2::new(35., 25.);
            frame(
                &ctx,
                &mut app,
                panel,
                vec![
                    egui::Event::PointerMoved(canvas_start),
                    button(canvas_start, true),
                ],
            );
            frame(
                &ctx,
                &mut app,
                panel,
                vec![egui::Event::PointerMoved(canvas_end)],
            );
            frame(&ctx, &mut app, panel, vec![button(canvas_end, false)]);
            assert!(
                app.editor.document != document,
                "{panel} swallowed canvas drag"
            );
            app.editor.undo();
            assert!(
                app.editor.document == document,
                "{panel} canvas undo differed"
            );
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("; "));
}

#[test]
fn hidden_or_blocked_editor_panel_does_not_reuse_a_stale_press() {
    for hidden in [false, true] {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        let id = egui::Id::new("inspector");
        for _ in 0..3 {
            frame(&ctx, &mut app, "inspector", vec![]);
        }
        let rect = egui::containers::panel::PanelState::load(&ctx, id)
            .unwrap()
            .outer_rect;
        let start = Pos2::new(rect.min.x, 200.);
        let end = start - Vec2::new(50., 0.);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            pressed,
            button: egui::PointerButton::Primary,
            modifiers: Modifiers::NONE,
        };
        frame(
            &ctx,
            &mut app,
            "inspector",
            vec![egui::Event::PointerMoved(start), button(start, true)],
        );
        if hidden {
            frame(&ctx, &mut app, "review-panel", vec![]);
        } else {
            app.error = Some("A pending error".into());
            frame(&ctx, &mut app, "inspector", vec![]);
            app.error = None;
        }
        frame(
            &ctx,
            &mut app,
            "inspector",
            vec![egui::Event::PointerMoved(end), button(end, false)],
        );
        frame(&ctx, &mut app, "inspector", vec![]);
        assert_eq!(
            egui::containers::panel::PanelState::load(&ctx, id)
                .unwrap()
                .outer_rect
                .width(),
            rect.width(),
            "hidden={hidden}"
        );
    }
}
