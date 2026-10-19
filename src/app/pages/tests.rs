// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::collections::HashMap;

fn key(key: Key, modifiers: Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}
fn frame(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    events: Vec<egui::Event>,
) -> HashMap<String, Pos2> {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 850.))),
            events,
            ..Default::default()
        },
        |ui| {
            app.shortcuts(ui.ctx());
            app.page_actions_menu(ui, 1);
            app.page_rename_dialog(ui.ctx());
        },
    );
    let labels = output
        .platform_output
        .accesskit_update
        .as_ref()
        .map(|update| {
            update
                .nodes
                .iter()
                .filter_map(|(_, n)| Some((n.label()?.to_owned(), n.bounds()?)))
                .map(|(label, b)| {
                    (
                        label,
                        Pos2::new(((b.x0 + b.x1) / 2.) as f32, ((b.y0 + b.y1) / 2.) as f32),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    output.textures_delta.clear();
    labels
}
fn click(ctx: &egui::Context, app: &mut WireframeApp, label: &str) {
    let mut labels = HashMap::new();
    for _ in 0..3 {
        labels = frame(ctx, app, vec![]);
    }
    let pos = *labels
        .get(label)
        .unwrap_or_else(|| panic!("Missing {label}: {:?}", labels.keys()));
    for pressed in [true, false] {
        frame(
            ctx,
            app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
}
#[test]
fn rename_selects_existing_text_blocks_canvas_commands_and_commits_once() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 50., 60.);
    let before = app.editor.document.clone();
    click(&ctx, &mut app, "Rename…");
    for _ in 0..3 {
        frame(&ctx, &mut app, vec![]);
    }
    assert!(app.canvas_input_blocked());
    assert!(!app.command_enabled(&ctx, menus::Command::Save));
    frame(&ctx, &mut app, vec![key(Key::D, Modifiers::COMMAND)]);
    assert_eq!(app.editor.document, before);
    frame(&ctx, &mut app, vec![egui::Event::Paste("Contact".into())]);
    assert_eq!(app.page_rename.as_ref().unwrap().name, "Contact");
    frame(&ctx, &mut app, vec![key(Key::Enter, Modifiers::NONE)]);
    assert!(app.page_rename.is_none());
    assert_eq!(app.editor.page_data().name, "Contact");
    assert_eq!(app.editor.undo_name(), Some("Rename wireframe"));
    app.editor.undo();
    assert_eq!(app.editor.document, before);
}

#[test]
fn context_actions_use_target_page_and_cancel_does_not_edit() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add_page();
    let before = app.editor.document.clone();
    click(&ctx, &mut app, "Rename…");
    frame(&ctx, &mut app, vec![egui::Event::Paste("Discarded".into())]);
    frame(&ctx, &mut app, vec![key(Key::Escape, Modifiers::NONE)]);
    assert!(app.page_rename.is_none());
    assert_eq!(app.editor.document, before);
    click(&ctx, &mut app, "Create alternate version");
    assert_eq!(app.editor.page, 1);
    assert_eq!(app.editor.document.pages[0].alternates.len(), 1);
    assert!(app.editor.document.pages[1].alternates.is_empty());
    click(&ctx, &mut app, "Duplicate");
    assert_eq!(app.editor.document.pages.len(), 3);
    assert_eq!(app.editor.page_data().name, "Wireframe 1 copy");
    assert!(app.editor.active_alternate(app.editor.page).is_some());
}

#[test]
fn context_trash_retains_last_page_and_creates_a_live_blank() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 50., 60.);
    let before = app.editor.document.clone();
    click(&ctx, &mut app, "Move to Trash");
    assert!(app.editor.document.pages[0].trashed);
    assert_eq!(
        app.editor.document.pages[0].content,
        before.pages[0].content
    );
    assert_eq!(app.editor.document.live_pages().count(), 1);
    assert_ne!(app.editor.page, 1);
    app.editor.undo();
    assert_eq!(app.editor.document, before);
}

#[test]
fn keyboard_rename_returns_navigation_after_cancel_or_commit() {
    use eframe::App;
    fn full_frame(
        ctx: &egui::Context,
        app: &mut WireframeApp,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200., 850.))),
            events,
            ..Default::default()
        };
        app.raw_input_hook(ctx, &mut input);
        ctx.run_ui(input, |ui| app.ui(ui, &mut eframe::Frame::_new_kittest()))
    }
    for (thumbnails, commit) in [(false, false), (true, false), (false, true), (true, true)] {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.thumbnails = thumbnails;
        let first = app.editor.page;
        app.editor.add_page();
        let second = app.editor.page;
        app.editor.switch_page(first);
        let before = app.editor.document.clone();
        let mut point = Pos2::ZERO;
        for _ in 0..3 {
            let output = full_frame(&ctx, &mut app, vec![]);
            let node = &output
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .find(|(_, n)| n.label() == Some("Wireframe 1"))
                .unwrap()
                .1;
            let b = node.bounds().unwrap();
            point = Pos2::new(((b.x0 + b.x1) / 2.) as f32, ((b.y0 + b.y1) / 2.) as f32);
            output.drop_without_applying_deltas();
        }
        for pressed in [true, false] {
            full_frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Modifiers::NONE,
                    },
                ],
            )
            .drop_without_applying_deltas();
        }
        assert!(app.navigator.focused(&ctx));
        full_frame(&ctx, &mut app, vec![key(Key::F2, Modifiers::NONE)])
            .drop_without_applying_deltas();
        full_frame(
            &ctx,
            &mut app,
            vec![egui::Event::Paste("Contact 世界".into())],
        )
        .drop_without_applying_deltas();
        assert_eq!(app.page_rename.as_ref().unwrap().name, "Contact 世界");
        full_frame(
            &ctx,
            &mut app,
            vec![key(
                if commit { Key::Enter } else { Key::Escape },
                Modifiers::NONE,
            )],
        )
        .drop_without_applying_deltas();
        assert!(app.page_rename.is_none());
        full_frame(&ctx, &mut app, vec![key(Key::ArrowDown, Modifiers::NONE)])
            .drop_without_applying_deltas();
        assert_eq!(
            app.editor.page, second,
            "The first arrow after rename must navigate"
        );
        assert!(app.navigator.focused(&ctx));
        if commit {
            assert_eq!(
                app.editor.document.live_page(first).unwrap().name,
                "Contact 世界"
            );
            full_frame(&ctx, &mut app, vec![key(Key::Z, Modifiers::COMMAND)])
                .drop_without_applying_deltas();
        }
        assert_eq!(app.editor.document, before);
    }
}
