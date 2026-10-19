// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

fn frame(ctx: &egui::Context, app: &mut WireframeApp, events: Vec<egui::Event>) {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 800.))),
            time: Some(ctx.cumulative_frame_nr() as f64 * 0.05),
            events,
            ..Default::default()
        },
        |ui| {
            app.canvas(ui);
            app.icon_dialog_ui(ui.ctx());
        },
    );
    output.textures_delta.clear();
}
fn key(key: Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}
fn double_click(ctx: &egui::Context, app: &mut WireframeApp, pos: Pos2, modifiers: Modifiers) {
    for _ in 0..2 {
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
                        modifiers,
                    },
                ],
            );
        }
    }
}
#[test]
fn canvas_double_click_dispatches_plain_and_modified_label_icons() {
    for (kind, modifiers, icon) in [
        (Kind::Icon, Modifiers::NONE, true),
        (Kind::IconLabel, Modifiers::COMMAND, true),
        (Kind::IconLabel, Modifiers::NONE, false),
        (Kind::Button, Modifiers::NONE, false),
    ] {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(kind, 100., 100.);
        frame(&ctx, &mut app, vec![]);
        let control = app.editor.current().controls[0].clone();
        let pos = app.screen([
            control.bounds.x + control.bounds.w / 2.,
            control.bounds.y + control.bounds.h / 2.,
        ]);
        double_click(&ctx, &mut app, pos, modifiers);
        assert_eq!(app.icon_dialog.is_some(), icon, "{kind:?}");
        assert_eq!(app.text_edit.is_some(), !icon, "{kind:?}");
    }
}
#[test]
fn icon_selection_is_provisional_then_one_history_step_with_size() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Icon, 50., 50.);
    let id = app.editor.current().controls[0].id;
    let before = app.editor.document.clone();
    app.begin_icon_dialog(id);
    let mut draft = app.icon_dialog.take().unwrap();
    draft.chosen = Some(IconSource::Catalog("circle-plus-solid".into()));
    draft.size = 3;
    assert_eq!(app.editor.document, before);
    app.apply_icon_dialog(&draft).unwrap();
    let after = app.editor.document.clone();
    let (_, values) = app.editor.control_properties(id).unwrap();
    assert_eq!(values.get("icon_size"), Some(&Value::Choice("48".into())));
    assert_eq!(
        values.get("icon"),
        Some(&Value::Icon(IconSource::Catalog(
            "circle-plus-solid".into()
        )))
    );
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    app.editor.redo();
    assert_eq!(app.editor.document, after);
}
#[test]
fn escape_leaves_bytes_and_redo_unchanged() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Icon, 50., 50.);
    let id = app.editor.current().controls[0].id;
    app.editor.add(Kind::Button, 100., 100.);
    app.editor.undo();
    let before = app.editor.document.clone();
    let redo = app.editor.redo_name().map(str::to_owned);
    app.begin_icon_dialog(id);
    let draft = app.icon_dialog.as_mut().unwrap();
    draft.chosen = Some(IconSource::Catalog("circle-plus-solid".into()));
    draft.size = 5;
    frame(&ctx, &mut app, vec![]);
    frame(&ctx, &mut app, vec![key(Key::Escape)]);
    assert!(app.icon_dialog.is_none());
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.redo_name(), redo.as_deref());
}
#[test]
fn stale_project_or_view_cannot_apply_dialog_to_reused_id() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Icon, 50., 50.);
    let id = app.editor.current().controls[0].id;
    app.begin_icon_dialog(id);
    let mut draft = app.icon_dialog.take().unwrap();
    draft.chosen = Some(IconSource::Catalog("circle-plus-solid".into()));
    let document = app.editor.document.clone();
    app.editor = Editor::new(document.clone());
    assert!(app.apply_icon_dialog(&draft).is_err());
    assert_eq!(app.editor.document, document);
    app.begin_icon_dialog(id);
    let draft = app.icon_dialog.take().unwrap();
    app.editor.context.push(id);
    assert!(!draft.matches(&app.editor));
}
#[test]
fn category_metadata_is_sorted_resolves_and_filters_styles() {
    assert!(wireframe::icons::categories().len() > 50);
    let category = wireframe::icons::categories()
        .iter()
        .find(|c| c.id == "accessibility")
        .unwrap();
    assert!(category.contains(wireframe::icons::get("circle-info-solid").unwrap()));
    assert!(!category.contains(wireframe::icons::get("pizza-slice-solid").unwrap()));
    for category in wireframe::icons::categories() {
        assert!(
            wireframe::icons::all()
                .iter()
                .any(|icon| category.contains(icon)),
            "{}",
            category.id
        );
    }
}

#[test]
fn picker_stays_within_viewport_across_search_frames() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Icon, 50., 50.);
    app.begin_icon_dialog(app.editor.current().controls[0].id);
    for index in 0..12 {
        if index == 4 {
            app.icon_dialog.as_mut().unwrap().query = "star".into();
        }
        frame(&ctx, &mut app, vec![]);
        let rect = ctx
            .memory(|m| m.area_rect(egui::Id::new("icon-library-dialog")))
            .unwrap();
        assert!(rect.height() < 760., "frame {index}: {rect:?}");
        assert!(rect.width() < 960., "frame {index}: {rect:?}");
    }
}

#[test]
fn shared_row_dialog_is_provisional_skips_short_lists_and_keeps_other_icons() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::List, 50., 50.);
    let first = *app.editor.selection.iter().next().unwrap();
    app.editor.add(Kind::List, 300., 50.);
    let second = *app.editor.selection.iter().next().unwrap();
    app.editor
        .set_control_property(first, "text", Value::Text("One\nTwo\nThree".into()))
        .unwrap();
    app.editor
        .set_control_property(second, "text", Value::Text("A\nB".into()))
        .unwrap();
    app.editor
        .set_control_property(
            first,
            "row_icons",
            Value::Icons(vec![Some(IconSource::Catalog("star-solid".into()))]),
        )
        .unwrap();
    app.editor.commit();
    let before = app.editor.document.clone();
    app.begin_row_icon_dialog(vec![first, second], 2);
    let mut draft = app.icon_dialog.take().unwrap();
    assert_eq!(draft.targets, vec![first]);
    draft.chosen = Some(IconSource::Catalog("backward-step-solid".into()));
    assert_eq!(app.editor.document, before);
    app.apply_icon_dialog(&draft).unwrap();
    let after = app.editor.document.clone();
    assert_eq!(
        app.editor.list_row_icons(&[first]).unwrap()[0].icon,
        Some(IconSource::Catalog("star-solid".into()))
    );
    assert_eq!(
        after.pages[0].content.nodes[1],
        before.pages[0].content.nodes[1]
    );
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    let redo = app.editor.redo_name().map(str::to_owned);
    app.begin_row_icon_dialog(vec![first, second], 1);
    app.icon_dialog.as_mut().unwrap().chosen = Some(IconSource::Catalog("star-solid".into()));
    frame(&ctx, &mut app, vec![]);
    frame(&ctx, &mut app, vec![key(Key::Escape)]);
    assert!(app.icon_dialog.is_none());
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.redo_name(), redo.as_deref());
    app.editor.redo();
    assert_eq!(app.editor.document, after);
}

#[test]
fn shared_app_bar_dialog_shares_collection_preserves_menu_and_cancels_without_history() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::AppBar, 50., 50.);
    let first = *app.editor.selection.iter().next().unwrap();
    app.editor.add(Kind::AppBar, 50., 200.);
    let second = *app.editor.selection.iter().next().unwrap();
    let icon = |s: &str| Some(IconSource::Catalog(s.into()));
    app.editor
        .set_control_property(
            first,
            "icons",
            Value::Icons(vec![icon("star-solid"), icon("heart-solid")]),
        )
        .unwrap();
    app.editor
        .set_control_property(second, "icons", Value::Icons(vec![icon("circle-solid")]))
        .unwrap();
    app.editor
        .set_control_property(
            second,
            "row_links",
            Value::Links(vec![
                None,
                None,
                Some(wireframe::document::Link::Back),
                Some(wireframe::document::Link::Page(app.editor.page)),
            ]),
        )
        .unwrap();
    app.editor.commit();
    let before = app.editor.document.clone();
    app.begin_app_bar_icon_dialog(vec![first, second], 1);
    assert_eq!(app.editor.document, before);
    frame(&ctx, &mut app, vec![]);
    frame(&ctx, &mut app, vec![key(Key::Escape)]);
    assert!(app.icon_dialog.is_none());
    assert_eq!(app.editor.document, before);
    app.begin_app_bar_icon_dialog(vec![first, second], 1);
    let draft = app.icon_dialog.take().unwrap();
    app.apply_icon_dialog(&draft).unwrap();
    assert_eq!(
        app.editor.control_properties(first).unwrap().1["icons"],
        app.editor.control_properties(second).unwrap().1["icons"]
    );
    assert_eq!(
        app.editor.control_properties(second).unwrap().1["row_links"],
        Value::Links(vec![
            None,
            None,
            None,
            None,
            Some(wireframe::document::Link::Page(app.editor.page))
        ])
    );
    let after = app.editor.document.clone();
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    app.editor.redo();
    assert_eq!(app.editor.document, after);
    app.begin_app_bar_icon_dialog(vec![first, second], 2);
    let mut draft = app.icon_dialog.take().unwrap();
    draft.chosen = icon("plus-solid");
    app.editor
        .set_control_property(first, "text", Value::Text("Changed while open".into()))
        .unwrap();
    let stale = app.editor.document.clone();
    assert!(app.apply_icon_dialog(&draft).is_err());
    assert_eq!(app.editor.document, stale);
}
