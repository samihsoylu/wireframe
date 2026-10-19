// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use eframe::App;
use std::collections::BTreeMap;

fn frame(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    events: Vec<egui::Event>,
) -> BTreeMap<String, (Rect, bool)> {
    let mut input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200., 850.))),
        events,
        ..Default::default()
    };
    app.raw_input_hook(ctx, &mut input);
    let out = ctx.run_ui(input, |ui| app.ui(ui, &mut eframe::Frame::_new_kittest()));
    let nodes = out
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter_map(|(_, n)| {
            let b = n.bounds()?;
            Some((
                n.label()?.to_owned(),
                (
                    Rect::from_min_max(
                        egui::pos2(b.x0 as f32, b.y0 as f32),
                        egui::pos2(b.x1 as f32, b.y1 as f32),
                    ),
                    n.is_disabled(),
                ),
            ))
        })
        .collect();
    out.drop_without_applying_deltas();
    nodes
}

fn click(ctx: &egui::Context, app: &mut WireframeApp, label: &str) {
    let nodes = frame(ctx, app, vec![]);
    let (rect, disabled) = nodes
        .get(label)
        .unwrap_or_else(|| panic!("Missing {label}"));
    assert!(!disabled, "Disabled {label}");
    for pressed in [true, false] {
        frame(
            ctx,
            app,
            vec![
                egui::Event::PointerMoved(rect.center()),
                egui::Event::PointerButton {
                    pos: rect.center(),
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
}

#[test]
fn in_window_duplicate_targets_the_focused_page_or_source_and_undoes_exactly() {
    for source in [false, true] {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = WireframeApp::workspace(&ctx, None);
        let label = if source {
            let library = app.editor.create_library("Menu sources").unwrap();
            let id = app
                .editor
                .create_empty_source(library, "Focused source", Default::default())
                .unwrap();
            app.editor.edit_library_source(id).unwrap();
            "Focused source"
        } else {
            app.editor
                .rename_page(app.editor.page, "Focused page")
                .unwrap();
            app.thumbnails = false;
            "Focused page"
        };
        app.editor.add(Kind::Button, 60., 70.);
        for _ in 0..3 {
            frame(&ctx, &mut app, vec![]);
        }
        click(&ctx, &mut app, label);
        let before = app.editor.document.clone();
        click(&ctx, &mut app, "Edit");
        click(&ctx, &mut app, "Duplicate");
        let after = app.editor.document.clone();
        if source {
            assert_eq!(after.definitions.len(), before.definitions.len() + 1);
            assert_eq!(after.pages, before.pages);
        } else {
            assert_eq!(after.pages.len(), before.pages.len() + 1);
            assert_eq!(after.pages[0], before.pages[0]);
        }
        click(&ctx, &mut app, "Edit");
        // Undo labels include the operation name; inspect the current rendered menu.
        let nodes = frame(&ctx, &mut app, vec![]);
        let undo = nodes
            .keys()
            .find(|name| name.starts_with("Undo"))
            .unwrap()
            .clone();
        click(&ctx, &mut app, &undo);
        assert_eq!(app.editor.document, before);
    }
}

#[test]
fn captured_menu_navigation_rejects_changed_surface_project_and_hidden_panel() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    let owner = egui::Id::new("old-page");
    app.menus.navigator_owner = Some(NavigatorMenuOwner {
        id: owner,
        session: app.editor.clipboard_session().into(),
        page: app.editor.page,
        surface: None,
    });
    app.menus.drawing_popup = true;
    assert_eq!(app.command_navigation_focus(&ctx), Some(owner));
    app.show_navigator = false;
    assert_eq!(app.command_navigation_focus(&ctx), None);
    app.show_navigator = true;
    let page = app.editor.page;
    app.editor.add_page();
    assert_eq!(app.command_navigation_focus(&ctx), None);
    app.editor.switch_page(page);
    let library = app.editor.create_library("Different surface").unwrap();
    app.editor.edit_library(library).unwrap();
    assert_eq!(app.command_navigation_focus(&ctx), None);
    app.editor.switch_page(page);
    app.editor = Editor::new(Default::default());
    assert_eq!(app.command_navigation_focus(&ctx), None);
}
