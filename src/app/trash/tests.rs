// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::collections::HashMap;

fn frame(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    size: Vec2,
    events: Vec<egui::Event>,
) -> HashMap<String, Rect> {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            events,
            ..Default::default()
        },
        |ui| {
            if app.trash.is_some() {
                ui.disable();
            }
            app.shortcuts(ui.ctx());
            app.toolbar(ui);
            app.trash_dialog(ui.ctx());
        },
    );
    // A confirmation and its disabled background can expose the same caption.
    // Prefer the enabled foreground control when building click targets.
    let mut nodes: Vec<_> = output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .collect();
    nodes.sort_by_key(|(_, node)| !node.is_disabled());
    let labels = nodes
        .iter()
        .filter_map(|(_, n)| Some((n.label()?.to_owned(), n.bounds()?)))
        .map(|(s, b)| {
            (
                s,
                Rect::from_min_max(
                    egui::pos2(b.x0 as f32, b.y0 as f32),
                    egui::pos2(b.x1 as f32, b.y1 as f32),
                ),
            )
        })
        .collect();
    output.textures_delta.clear();
    labels
}
fn key(key: Key, modifiers: Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}
fn click(ctx: &egui::Context, app: &mut WireframeApp, label: &str) {
    let size = Vec2::new(1100., 850.);
    let mut labels = HashMap::new();
    for _ in 0..3 {
        labels = frame(ctx, app, size, vec![]);
    }
    let pos = labels
        .get(label)
        .unwrap_or_else(|| panic!("{label}: {:?}", labels.keys()))
        .center();
    for pressed in [true, false] {
        frame(
            ctx,
            app,
            size,
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
fn setup() -> (egui::Context, WireframeApp, Id) {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.rename_page(1, "Landing").unwrap();
    app.editor.add(Kind::Button, 50., 60.);
    app.editor.add_page();
    let second = app.editor.page;
    app.editor.rename_page(second, "Contact").unwrap();
    app.editor.trash_pages(&[1, second]).unwrap();
    (ctx, app, second)
}

fn focused_label(ctx: &egui::Context, app: &mut WireframeApp, size: Vec2) -> String {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            ..Default::default()
        },
        |ui| app.trash_dialog(ui.ctx()),
    );
    let update = output.platform_output.accesskit_update.as_ref().unwrap();
    let label = update
        .nodes
        .iter()
        .find(|(id, _)| *id == update.focus)
        .and_then(|(_, node)| node.label())
        .unwrap_or("<no labeled focus>")
        .to_owned();
    output.textures_delta.clear();
    label
}

#[test]
fn overflow_arrows_keep_native_focus_on_the_selected_card() {
    for size in [Vec2::new(920., 620.), Vec2::new(1360., 932.)] {
        let (ctx, mut app, _) = setup();
        let mut pages = Vec::new();
        for index in 0..30 {
            app.editor.add_page();
            let page = app.editor.page;
            app.editor
                .rename_page(page, &format!("Overflow {index:02}"))
                .unwrap();
            pages.push(page);
        }
        app.editor.trash_pages(&pages).unwrap();
        let before = app.editor.document.clone();
        let resources: Vec<_> = app
            .editor
            .retained_resources()
            .map(|(id, name)| (id, format!("{name} — Wireframe")))
            .collect();
        app.open_trash();
        for _ in 0..3 {
            frame(&ctx, &mut app, size, vec![]);
        }
        assert_eq!(focused_label(&ctx, &mut app, size), resources[0].1);
        for (key, modifiers, index, anchor) in [
            (Key::ArrowDown, Modifiers::NONE, 3, 3),
            (Key::ArrowDown, Modifiers::NONE, 6, 6),
            (Key::ArrowDown, Modifiers::NONE, 9, 9),
            (Key::ArrowRight, Modifiers::NONE, 10, 10),
            (Key::ArrowDown, Modifiers::SHIFT, 13, 10),
            (Key::ArrowLeft, Modifiers::SHIFT, 12, 10),
            (Key::ArrowUp, Modifiers::NONE, 9, 9),
            (Key::ArrowLeft, Modifiers::NONE, 8, 8),
        ] {
            frame(&ctx, &mut app, size, vec![self::key(key, modifiers)]);
            assert_eq!(
                focused_label(&ctx, &mut app, size),
                resources[index].1,
                "{key:?} at {size:?}"
            );
            let draft = app.trash.as_ref().unwrap();
            assert_eq!(draft.focused, resources[index].0);
            assert_eq!(
                draft.selected,
                resources[anchor.min(index)..=anchor.max(index)]
                    .iter()
                    .map(|(id, _)| *id)
                    .collect()
            );
            assert_eq!(app.editor.document, before);
        }
    }
}

#[test]
fn card_arrow_ownership_preserves_footer_tab_and_escape() {
    let (ctx, mut app, _) = setup();
    let size = Vec2::new(920., 620.);
    let before = app.editor.document.clone();
    app.open_trash();
    for _ in 0..3 {
        frame(&ctx, &mut app, size, vec![]);
    }
    frame(
        &ctx,
        &mut app,
        size,
        vec![key(Key::ArrowRight, Modifiers::NONE)],
    );
    assert_eq!(focused_label(&ctx, &mut app, size), "Contact — Wireframe");
    for label in ["Empty Trash", "Delete Permanently", "Restore", "Close"] {
        frame(&ctx, &mut app, size, vec![key(Key::Tab, Modifiers::NONE)]);
        assert_eq!(focused_label(&ctx, &mut app, size), label);
    }
    frame(&ctx, &mut app, size, vec![key(Key::Tab, Modifiers::SHIFT)]);
    assert_eq!(focused_label(&ctx, &mut app, size), "Restore");
    frame(
        &ctx,
        &mut app,
        size,
        vec![key(Key::Escape, Modifiers::NONE)],
    );
    assert!(app.trash.is_none());
    assert_eq!(app.editor.document, before);
}

#[test]
fn recovery_keyboard_selection_restore_and_modal_command_boundary() {
    let (ctx, mut app, second) = setup();
    let blank = app.editor.page;
    click(&ctx, &mut app, "Trash");
    let before = app.editor.document.clone();
    assert!(app.trash.is_some());
    assert!(app.canvas_input_blocked());
    assert!(!app.command_enabled(&ctx, menus::Command::Save));
    for k in [Key::N, Key::Z, Key::D] {
        frame(
            &ctx,
            &mut app,
            Vec2::new(1100., 850.),
            vec![key(k, Modifiers::COMMAND)],
        );
        assert_eq!(app.editor.document, before);
    }
    frame(
        &ctx,
        &mut app,
        Vec2::new(1100., 850.),
        vec![key(Key::ArrowRight, Modifiers::SHIFT)],
    );
    assert_eq!(
        app.trash.as_ref().unwrap().selected,
        HashSet::from([RetainedResource::Page(1), RetainedResource::Page(second)])
    );
    frame(
        &ctx,
        &mut app,
        Vec2::new(1100., 850.),
        vec![key(Key::ArrowLeft, Modifiers::SHIFT)],
    );
    assert_eq!(
        app.trash.as_ref().unwrap().selected,
        HashSet::from([RetainedResource::Page(1)])
    );
    frame(
        &ctx,
        &mut app,
        Vec2::new(1100., 850.),
        vec![key(Key::A, Modifiers::COMMAND)],
    );
    assert_eq!(app.trash.as_ref().unwrap().selected.len(), 2);
    click(&ctx, &mut app, "Restore");
    assert!(app.trash.is_none());
    assert_eq!(app.editor.document.live_pages().count(), 3);
    assert_eq!(app.editor.page, blank);
    assert_eq!(app.editor.undo_name(), Some("Restore from Trash"));
    app.editor.undo();
    assert_eq!(app.editor.document, before);
}

#[test]
fn recovery_cards_and_footer_fit_supported_windows_and_close_without_edits() {
    let (ctx, mut app, _) = setup();
    let before = app.editor.document.clone();
    app.open_trash();
    for size in [
        Vec2::new(860., 650.),
        Vec2::new(1100., 850.),
        Vec2::new(1360., 932.),
    ] {
        let mut nodes = HashMap::new();
        for _ in 0..3 {
            nodes = frame(&ctx, &mut app, size, vec![]);
        }
        for label in [
            "Landing — Wireframe",
            "Contact — Wireframe",
            "Close",
            "Restore",
            "Delete Permanently",
            "Empty Trash",
        ] {
            let rect = nodes
                .get(label)
                .unwrap_or_else(|| panic!("Missing {label}"));
            assert!(
                rect.left() >= 0.
                    && rect.right() <= size.x
                    && rect.top() >= 0.
                    && rect.bottom() <= size.y,
                "{label}: {rect:?} in {size:?}"
            );
        }
        assert!(!nodes["Landing — Wireframe"].intersects(nodes["Contact — Wireframe"]));
        assert!(nodes["Close"].top() > nodes["Landing — Wireframe"].bottom());
        assert!(nodes["Close"].top() - nodes["Landing — Wireframe"].bottom() > 150.);
        assert!(nodes["Close"].right() < nodes["Restore"].left());
        assert!(nodes["Restore"].right() < nodes["Delete Permanently"].left());
        assert!(nodes["Delete Permanently"].right() < nodes["Empty Trash"].left());
    }
    frame(
        &ctx,
        &mut app,
        Vec2::new(1100., 850.),
        vec![key(Key::Escape, Modifiers::NONE)],
    );
    assert!(app.trash.is_none());
    assert_eq!(app.editor.document, before);
    app.open_trash();
    click(&ctx, &mut app, "Close");
    assert!(app.trash.is_none());
    assert_eq!(app.editor.document, before);
}

#[test]
fn partial_restore_selects_next_retained_page_and_keeps_dialog_open() {
    let (ctx, mut app, second) = setup();
    let current = app.editor.page;
    app.open_trash();
    click(&ctx, &mut app, "Restore");
    assert!(app.editor.document.live_page(1).is_some());
    assert!(app.editor.document.live_page(second).is_none());
    assert_eq!(
        app.trash.as_ref().unwrap().selected,
        HashSet::from([RetainedResource::Page(second)])
    );
    assert_eq!(app.editor.page, current);
    click(&ctx, &mut app, "Restore");
    assert!(app.trash.is_none());
    assert!(app.editor.document.live_page(second).is_some());
    assert_eq!(app.editor.page, current);
}

fn add_library(app: &mut WireframeApp) -> Id {
    app.editor.switch_page(app.editor.page);
    app.editor.add(Kind::Button, 40., 50.);
    let source = app.editor.create_component("Action".into()).unwrap();
    let library = app.editor.document.component_source(source).unwrap().0.id;
    app.editor.rename_library(library, "Landing").unwrap();
    app.move_library_to_trash(library);
    assert!(app.error.is_none());
    library
}

#[test]
fn library_only_trash_button_restores_and_undo_keeps_missing_instances() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    let library = add_library(&mut app);
    let before = app.editor.document.clone();
    click(&ctx, &mut app, "Trash");
    assert_eq!(
        app.trash.as_ref().unwrap().selected,
        HashSet::from([RetainedResource::Library(library)])
    );
    let labels = frame(&ctx, &mut app, Vec2::new(1100., 850.), vec![]);
    assert!(labels.contains_key("Landing — Component library"));
    assert!(labels.contains_key("Delete Permanently"));
    assert!(labels.contains_key("Empty Trash"));
    click(&ctx, &mut app, "Restore");
    assert!(app.trash.is_none());
    assert_eq!(app.editor.document.live_libraries().count(), 1);
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    click(&ctx, &mut app, "Trash");
    click(&ctx, &mut app, "Close");
    assert_eq!(app.editor.document, before);
}

#[test]
fn mixed_equal_names_range_and_select_all_restore_atomically() {
    let (ctx, mut app, second) = setup();
    let library = add_library(&mut app);
    let before = app.editor.document.clone();
    click(&ctx, &mut app, "Trash");
    let labels = frame(&ctx, &mut app, Vec2::new(1100., 850.), vec![]);
    assert!(labels.contains_key("Landing — Wireframe"));
    assert!(labels.contains_key("Landing — Component library"));
    for _ in 0..2 {
        frame(
            &ctx,
            &mut app,
            Vec2::new(1100., 850.),
            vec![key(Key::ArrowRight, Modifiers::SHIFT)],
        );
    }
    assert_eq!(
        app.trash.as_ref().unwrap().selected,
        HashSet::from([
            RetainedResource::Page(1),
            RetainedResource::Page(second),
            RetainedResource::Library(library)
        ])
    );
    click(&ctx, &mut app, "Landing — Component library");
    assert_eq!(
        app.trash.as_ref().unwrap().selected,
        HashSet::from([RetainedResource::Library(library)])
    );
    frame(
        &ctx,
        &mut app,
        Vec2::new(1100., 850.),
        vec![key(Key::A, Modifiers::COMMAND)],
    );
    click(&ctx, &mut app, "Restore");
    assert!(app.trash.is_none());
    assert_eq!(app.editor.retained_resources().count(), 0);
    let after = app.editor.document.clone();
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    app.editor.redo();
    assert_eq!(app.editor.document, after);
}

#[test]
fn modifier_release_in_same_frame_toggles_library_card_and_partial_restore_selects_it() {
    let (ctx, mut app, _) = setup();
    let library = add_library(&mut app);
    app.open_trash();
    let mut labels = HashMap::new();
    for _ in 0..3 {
        labels = frame(&ctx, &mut app, Vec2::new(1100., 850.), vec![]);
    }
    let pos = labels["Landing — Component library"].center();
    frame(
        &ctx,
        &mut app,
        Vec2::new(1100., 850.),
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::COMMAND,
            },
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::COMMAND,
            },
            egui::Event::ModifiersChanged(Modifiers::NONE),
        ],
    );
    assert_eq!(
        app.trash.as_ref().unwrap().selected,
        HashSet::from([
            RetainedResource::Page(1),
            RetainedResource::Library(library)
        ])
    );
    // Restore both pages, then focus the remaining library card.
    click(&ctx, &mut app, "Landing — Wireframe");
    frame(
        &ctx,
        &mut app,
        Vec2::new(1100., 850.),
        vec![key(Key::ArrowRight, Modifiers::SHIFT)],
    );
    click(&ctx, &mut app, "Restore");
    assert_eq!(
        app.trash.as_ref().unwrap().selected,
        HashSet::from([RetainedResource::Library(library)])
    );
    click(&ctx, &mut app, "Restore");
    assert!(app.trash.is_none());
}

#[test]
fn permanent_selected_delete_requires_confirmation_and_cancel_preserves_history() {
    let (ctx, mut app, second) = setup();
    click(&ctx, &mut app, "Trash");
    let before = app.editor.document.clone();
    let undo = app.editor.undo_name().map(str::to_owned);
    click(&ctx, &mut app, "Delete Permanently");
    assert_eq!(
        app.trash
            .as_ref()
            .unwrap()
            .confirmation
            .as_ref()
            .unwrap()
            .resources,
        vec![RetainedResource::Page(1)]
    );
    assert_eq!(app.editor.document, before);
    click(&ctx, &mut app, "Cancel");
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.undo_name(), undo.as_deref());
    click(&ctx, &mut app, "Delete Permanently");
    click(&ctx, &mut app, "Delete");
    assert!(app.editor.document.pages.iter().all(|p| p.id != 1));
    assert!(
        app.editor
            .document
            .pages
            .iter()
            .find(|p| p.id == second)
            .unwrap()
            .trashed
    );
    assert_eq!(
        app.trash.as_ref().unwrap().selected,
        HashSet::from([RetainedResource::Page(second)])
    );
    let after = app.editor.document.clone();
    app.editor.undo(); // Mixed Trash restores only the remaining page.
    assert!(app.editor.document.live_page(second).is_some());
    assert!(app.editor.document.pages.iter().all(|p| p.id != 1));
    app.editor.redo();
    assert_eq!(app.editor.document, after);
}

#[test]
fn empty_trash_confirms_all_pages_and_libraries_and_preserves_live_canvas() {
    let (ctx, mut app, _) = setup();
    add_library(&mut app);
    let live = app.editor.document.first_live_page().clone();
    let before = app.editor.document.clone();
    click(&ctx, &mut app, "Trash");
    click(&ctx, &mut app, "Empty Trash");
    let confirmation = app.trash.as_ref().unwrap().confirmation.as_ref().unwrap();
    assert!(confirmation.all);
    assert_eq!(confirmation.resources.len(), 3);
    assert_eq!(app.editor.document, before);
    click(&ctx, &mut app, "Empty Trash");
    assert!(
        app.trash.is_none(),
        "remaining={} confirmation={} error={:?}",
        app.editor.retained_resources().count(),
        app.trash.as_ref().is_some_and(|d| d.confirmation.is_some()),
        app.trash.as_ref().and_then(|d| d.error.as_ref())
    );
    assert_eq!(app.editor.document.pages, vec![live]);
    assert!(app.editor.document.libraries.is_empty());
    assert!(app.editor.document.definitions.is_empty());
    assert_eq!(app.editor.retained_resources().count(), 0);
    assert_eq!(
        wireframe::document::decode(&app.editor.document.encode().unwrap()).unwrap(),
        app.editor.document
    );
}

#[test]
fn confirmation_escape_and_document_drift_never_delete_a_changed_scope() {
    let (ctx, mut app, second) = setup();
    click(&ctx, &mut app, "Trash");
    click(&ctx, &mut app, "Empty Trash");
    frame(
        &ctx,
        &mut app,
        Vec2::new(1100., 850.),
        vec![key(Key::Escape, Modifiers::NONE)],
    );
    assert!(app.trash.as_ref().unwrap().confirmation.is_none());
    assert_eq!(app.editor.retained_resources().count(), 2);
    click(&ctx, &mut app, "Empty Trash");
    app.editor.restore_pages(&[second]).unwrap();
    let changed = app.editor.document.clone();
    click(&ctx, &mut app, "Empty Trash");
    assert_eq!(app.editor.document, changed);
    assert!(app.trash.as_ref().unwrap().confirmation.is_some());
    click(&ctx, &mut app, "Cancel");
    assert!(app.trash.as_ref().unwrap().confirmation.is_none());
    assert_eq!(app.editor.document, changed);
    assert!(app.editor.document.live_page(second).is_some());
}

fn image_setup() -> (egui::Context, WireframeApp, Id) {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    let mut bytes = std::io::Cursor::new(vec![]);
    image::RgbaImage::from_pixel(12, 8, image::Rgba([50, 90, 140, 255]))
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let id = app.editor.document.allocate_id().unwrap();
    app.editor.document.assets.push(wireframe::document::Asset {
        attribution: None,
        id,
        trashed: false,
        name: "Photo".into(),
        media_type: "image/png".into(),
        bytes: bytes.into_inner(),
    });
    app.editor.insert_catalog_asset(id, 20., 30.).unwrap();
    (ctx, app, id)
}
#[test]
fn image_trash_card_restores_with_one_history_step_and_does_not_change_source() {
    let (ctx, mut app, id) = image_setup();
    let before = app.editor.document.clone();
    app.move_asset_to_trash(id);
    let retained = app.editor.document.clone();
    click(&ctx, &mut app, "Trash");
    let labels = frame(&ctx, &mut app, Vec2::new(1100., 850.), vec![]);
    assert!(labels.contains_key("Photo — Image"));
    assert_eq!(app.editor.document, retained);
    click(&ctx, &mut app, "Restore");
    assert_eq!(app.editor.document, before);
    assert!(app.trash.is_none());
    app.editor.undo();
    assert_eq!(app.editor.document, retained);
    app.editor.redo();
    assert_eq!(app.editor.document, before);
}
#[test]
fn image_permanent_confirmation_cancel_and_accept_preserve_the_correct_scope() {
    let (ctx, mut app, id) = image_setup();
    app.move_asset_to_trash(id);
    click(&ctx, &mut app, "Trash");
    let before = app.editor.document.clone();
    click(&ctx, &mut app, "Delete Permanently");
    click(&ctx, &mut app, "Cancel");
    assert_eq!(app.editor.document, before);
    click(&ctx, &mut app, "Delete Permanently");
    click(&ctx, &mut app, "Delete");
    assert!(app.editor.document.assets.is_empty());
    assert!(app.editor.document.removed_assets.contains(&id));
    assert!(app.trash.is_none());
    for _ in 0..4 {
        app.editor.undo();
        assert!(app.editor.document.assets.is_empty());
    }
}
#[test]
fn empty_trash_includes_images_and_surviving_page_content_keeps_its_reference() {
    let (ctx, mut app, id) = image_setup();
    let surviving_content = app.editor.document.pages[0].content.clone();
    app.editor.add_page();
    let page = app.editor.page;
    app.editor.trash_pages(&[page]).unwrap();
    app.move_asset_to_trash(id);
    click(&ctx, &mut app, "Trash");
    click(&ctx, &mut app, "Empty Trash");
    let confirmation = app.trash.as_ref().unwrap().confirmation.as_ref().unwrap();
    assert_eq!(confirmation.resources.len(), 2);
    assert!(
        confirmation
            .resources
            .contains(&RetainedResource::Asset(id))
    );
    click(&ctx, &mut app, "Empty Trash");
    assert!(app.trash.is_none());
    assert_eq!(app.editor.document.pages[0].content, surviving_content);
    assert!(app.editor.document.assets.is_empty());
    assert!(app.editor.document.removed_assets.contains(&id));
}
