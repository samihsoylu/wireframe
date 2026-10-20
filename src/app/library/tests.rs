// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

#[test]
fn library_keeps_canvas_origin_fixed_for_empty_and_overflowing_categories() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    for width in [920., 940., 1360., 1920.] {
        for category in [CatalogCategory::All, CatalogCategory::Assets] {
            app.categories = vec![category];
            let mut height = 0.;
            for _ in 0..3 {
                ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 700.))),
                        ..Default::default()
                    },
                    |ui| {
                        let before = ui.available_rect_before_wrap().top();
                        app.library(ui);
                        height = ui.available_rect_before_wrap().top() - before;
                    },
                )
                .drop_without_applying_deltas();
            }
            assert!(
                (height - 120.).abs() < 0.1,
                "{category:?} at {width}: {height}"
            );
        }
    }
}

fn fixture(ctx: &egui::Context) -> (WireframeApp, Id) {
    let mut app = WireframeApp::workspace(ctx, None);
    app.editor.add(Kind::Button, 0., 0.);
    let id = app.editor.create_component("Action sample".into()).unwrap();
    app.editor.select_all();
    app.editor.delete();
    (app, id)
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
fn quick_frame(ctx: &egui::Context, app: &mut WireframeApp, events: Vec<egui::Event>) {
    let output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
            events,
            ..Default::default()
        },
        |ui| app.quick_add_ui(ui),
    );
    output.drop_without_applying_deltas();
}

#[test]
fn quick_add_horizontal_rule_inserts_control_before_matching_icons() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    let before = app.editor.document.clone();
    app.quick = "horizontal rule".into();
    app.quick_focus = true;
    quick_frame(&ctx, &mut app, vec![]);
    quick_frame(&ctx, &mut app, vec![key(Key::Enter)]);
    assert_eq!(app.editor.nodes().len(), 1);
    let id = *app.editor.selection.iter().next().unwrap();
    assert_eq!(
        app.editor.control_properties(id).unwrap().0.id,
        "horizontal_rule"
    );
    assert_eq!(app.editor.selected_views()[0].bounds.w, 100.);
    assert_eq!(app.editor.selected_views()[0].bounds.h, 10.);
    let inserted = app.editor.document.clone();
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    app.editor.redo();
    assert_eq!(app.editor.document, inserted);
}

#[test]
fn category_shift_combines_toggles_and_keeps_all_exclusive_without_document_history() {
    let ctx = egui::Context::default();
    let (mut app, _) = fixture(&ctx);
    let before = app.editor.document.clone();
    let undo = app.editor.undo_name().map(str::to_owned);
    app.select_catalog_category(CatalogCategory::Components, true);
    assert_eq!(app.categories, vec![CatalogCategory::Components]);
    app.select_catalog_category(CatalogCategory::Buttons, true);
    assert_eq!(
        app.categories,
        vec![CatalogCategory::Components, CatalogCategory::Buttons]
    );
    app.select_catalog_category(CatalogCategory::Components, true);
    assert_eq!(app.categories, vec![CatalogCategory::Buttons]);
    app.select_catalog_category(CatalogCategory::Buttons, true);
    assert_eq!(app.categories, vec![CatalogCategory::All]);
    app.select_catalog_category(CatalogCategory::Components, false);
    app.select_catalog_category(CatalogCategory::All, true);
    assert_eq!(app.categories, vec![CatalogCategory::All]);
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.undo_name(), undo.as_deref());
}

#[test]
fn quick_add_component_return_uses_default_placement_and_one_undo() {
    let ctx = egui::Context::default();
    let (mut app, id) = fixture(&ctx);
    app.pan = Vec2::new(-30., -40.);
    app.zoom = 2.;
    let before = app.editor.document.clone();
    app.quick = "Action sample".into();
    app.quick_focus = true;
    quick_frame(&ctx, &mut app, vec![]);
    quick_frame(&ctx, &mut app, vec![key(Key::Enter)]);
    assert!(app.quick.is_empty());
    assert!(app.error.is_none());
    assert_eq!(app.editor.nodes().len(), 1);
    let node = &app.editor.nodes()[0];
    assert_eq!(app.editor.instance_definition(node.id), Some(id));
    assert_eq!(node.origin, wireframe::document::Point::new(40., 45.));
    let after = app.editor.document.clone();
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    app.editor.redo();
    assert_eq!(app.editor.document, after);
    assert_eq!(
        wireframe::document::decode(&after.encode().unwrap()).unwrap(),
        after
    );
}

#[test]
fn quick_add_disabled_source_cannot_submit_and_same_name_source_stays_distinct() {
    let ctx = egui::Context::default();
    let (mut app, id) = fixture(&ctx);
    app.editor.edit_library_source(id).unwrap();
    let before = app.editor.document.clone();
    app.quick = "Action sample".into();
    app.quick_focus = true;
    quick_frame(&ctx, &mut app, vec![]);
    quick_frame(&ctx, &mut app, vec![key(Key::Enter)]);
    assert_eq!(app.editor.document, before);
    app.editor.switch_page(app.editor.page);
    app.editor.add(Kind::Label, 0., 0.);
    let second = app.editor.create_component("Action sample".into()).unwrap();
    app.editor.select_all();
    app.editor.delete();
    app.quick = "Action sample".into();
    app.quick_focus = true;
    quick_frame(&ctx, &mut app, vec![]);
    quick_frame(&ctx, &mut app, vec![key(Key::ArrowDown)]);
    quick_frame(&ctx, &mut app, vec![key(Key::Enter)]);
    let root = *app.editor.selection.iter().next().unwrap();
    assert_eq!(app.editor.instance_definition(root), Some(second));
}

#[test]
fn default_component_placement_avoids_previous_instance_and_rejects_stale_source() {
    let ctx = egui::Context::default();
    let (mut app, id) = fixture(&ctx);
    for _ in 0..2 {
        app.add_catalog_item(CatalogItem::Component(id), None);
    }
    let roots = app.editor.nodes();
    assert_eq!(roots.len(), 2);
    assert_ne!(roots[0].origin, roots[1].origin);
    app.editor.delete_definitions(&[id]).unwrap();
    let before = app.editor.document.clone();
    let undo = app.editor.undo_name().map(str::to_owned);
    app.add_catalog_item(CatalogItem::Component(id), None);
    assert!(app.error.is_some());
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.undo_name(), undo.as_deref());
}

#[test]
fn categories_survive_project_reopen_and_legacy_preferences_default_to_all() {
    let ctx = egui::Context::default();
    let (mut app, _) = fixture(&ctx);
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first.wireframe");
    let second = dir.path().join("second.wireframe");
    std::fs::write(&first, app.editor.document.encode().unwrap()).unwrap();
    std::fs::write(&second, app.editor.document.encode().unwrap()).unwrap();
    app.load_version_preferences(dir.path());
    assert!(app.load(first.clone()));
    app.categories = vec![CatalogCategory::Components, CatalogCategory::Buttons];
    assert!(app.load(second.clone()));
    assert_eq!(app.categories, default_categories());
    app.categories = vec![
        CatalogCategory::Android,
        CatalogCategory::Ios,
        CatalogCategory::Navigation,
        CatalogCategory::Charts,
    ];
    app.remember_version_view();
    let mut fresh = WireframeApp::workspace(&ctx, None);
    fresh.load_version_preferences(dir.path());
    assert!(fresh.load(first));
    assert_eq!(
        fresh.categories,
        vec![CatalogCategory::Components, CatalogCategory::Buttons]
    );
    assert!(fresh.load(second));
    assert_eq!(
        fresh.categories,
        vec![
            CatalogCategory::Android,
            CatalogCategory::Ios,
            CatalogCategory::Navigation,
            CatalogCategory::Charts
        ]
    );
    fresh.perform(Pending::New, &ctx);
    assert_eq!(fresh.categories, default_categories());
    let path = dir.path().join("version-view-settings.json");
    let mut old: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for view in old.as_object_mut().unwrap().values_mut() {
        view.as_object_mut().unwrap().remove("categories");
    }
    std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    fresh.load_version_preferences(dir.path());
    assert!(fresh.error.is_none());
    fresh.categories = vec![CatalogCategory::Components];
    assert!(fresh.load(dir.path().join("first.wireframe")));
    assert_eq!(fresh.categories, default_categories());
}

fn query_input_frame(ctx: &egui::Context, app: &mut WireframeApp, events: Vec<egui::Event>) {
    let mut events = events;
    for _ in 0..24 {
        let mut raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
            events: std::mem::take(&mut events),
            ..Default::default()
        };
        app.order_text_input(ctx, &mut raw);
        ctx.run_ui(raw, |ui| {
            app.shortcuts(ui.ctx());
            app.quick_add_ui(ui);
        })
        .drop_without_applying_deltas();
        if app.deferred_text_input.is_empty() {
            return;
        }
    }
    panic!("query input did not drain");
}
fn history_key(redo: bool) -> egui::Event {
    let mut event = key(Key::Z);
    if let egui::Event::Key { modifiers, .. } = &mut event {
        *modifiers = if redo {
            Modifiers::COMMAND | Modifiers::SHIFT
        } else {
            Modifiers::COMMAND
        };
    }
    event
}

#[test]
fn quick_add_exhausted_history_undoes_and_redoes_insert_without_losing_focus() {
    let ctx = egui::Context::default();
    let (mut app, _) = fixture(&ctx);
    let before = app.editor.document.clone();
    app.quick_focus = true;
    query_input_frame(&ctx, &mut app, vec![]);
    query_input_frame(
        &ctx,
        &mut app,
        vec![egui::Event::Paste("Action sample".into()), key(Key::Enter)],
    );
    let inserted = app.editor.document.clone();
    assert_ne!(inserted, before);
    assert_eq!(ctx.memory(|m| m.focused()), app.quick_input_id);
    query_input_frame(&ctx, &mut app, vec![history_key(false)]);
    assert_eq!(app.editor.document, before);
    assert_eq!(ctx.memory(|m| m.focused()), app.quick_input_id);
    query_input_frame(&ctx, &mut app, vec![history_key(true)]);
    assert_eq!(app.editor.document, inserted);
    assert!(app.quick.is_empty());
}

#[test]
fn quick_add_query_history_precedes_document_history_even_in_coalesced_input() {
    let ctx = egui::Context::default();
    let (mut app, _) = fixture(&ctx);
    let before = app.editor.document.clone();
    app.quick_focus = true;
    query_input_frame(&ctx, &mut app, vec![]);
    query_input_frame(
        &ctx,
        &mut app,
        vec![egui::Event::Paste("Action sample".into()), key(Key::Enter)],
    );
    let inserted = app.editor.document.clone();
    query_input_frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::Paste("Circle".into()),
            history_key(false),
            history_key(true),
        ],
    );
    assert_eq!(app.quick, "Circle");
    assert_eq!(app.editor.document, inserted);
    query_input_frame(&ctx, &mut app, vec![history_key(false), history_key(false)]);
    assert!(app.quick.is_empty());
    assert_eq!(app.editor.document, before);
    query_input_frame(&ctx, &mut app, vec![history_key(true)]);
    assert_eq!(app.editor.document, inserted);
    assert!(
        app.quick.is_empty(),
        "document history ends the query redo branch"
    );
}

#[test]
fn quick_add_escape_clears_then_unfocuses_without_leaving_source() {
    let ctx = egui::Context::default();
    let (mut app, id) = fixture(&ctx);
    app.editor.edit_library_source(id).unwrap();
    let before = app.editor.document.clone();
    let surface = app.editor.surface_id();
    app.quick_focus = true;
    query_input_frame(&ctx, &mut app, vec![]);
    query_input_frame(
        &ctx,
        &mut app,
        vec![egui::Event::Paste("Circle".into()), key(Key::Escape)],
    );
    assert!(app.quick.is_empty());
    assert_eq!(ctx.memory(|m| m.focused()), app.quick_input_id);
    query_input_frame(&ctx, &mut app, vec![key(Key::Escape)]);
    assert_ne!(ctx.memory(|m| m.focused()), app.quick_input_id);
    assert_eq!(app.editor.surface_id(), surface);
    assert_eq!(app.editor.document, before);
}

#[test]
fn quick_add_composition_escape_stays_local_before_clearing_query() {
    let ctx = egui::Context::default();
    let (mut app, _) = fixture(&ctx);
    app.quick_focus = true;
    query_input_frame(&ctx, &mut app, vec![]);
    query_input_frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::Paste("Circle".into()),
            egui::Event::Ime(egui::ImeEvent::Preedit {
                text: "あ".into(),
                active_range_chars: None,
            }),
        ],
    );
    let before = app.editor.document.clone();
    query_input_frame(&ctx, &mut app, vec![key(Key::Escape)]);
    assert_eq!(app.quick, "Circle");
    assert_eq!(app.editor.document, before);
    assert_eq!(ctx.memory(|m| m.focused()), app.quick_input_id);
    query_input_frame(&ctx, &mut app, vec![key(Key::Escape)]);
    assert!(app.quick.is_empty());
}

#[test]
fn quick_add_menu_history_uses_the_same_local_and_document_owners() {
    let ctx = egui::Context::default();
    let (mut app, _) = fixture(&ctx);
    let before = app.editor.document.clone();
    app.quick_focus = true;
    query_input_frame(&ctx, &mut app, vec![]);
    query_input_frame(
        &ctx,
        &mut app,
        vec![egui::Event::Paste("Action sample".into()), key(Key::Enter)],
    );
    let inserted = app.editor.document.clone();
    let undo = egui::Event::AppCommand(menus::Command::Undo as u64);
    let redo = egui::Event::AppCommand(menus::Command::Redo as u64);
    query_input_frame(
        &ctx,
        &mut app,
        vec![egui::Event::Paste("Circle".into()), undo.clone()],
    );
    assert!(app.quick.is_empty());
    assert_eq!(app.editor.document, inserted);
    query_input_frame(&ctx, &mut app, vec![undo, redo]);
    assert_eq!(app.editor.document, inserted);
    query_input_frame(
        &ctx,
        &mut app,
        vec![egui::Event::AppCommand(menus::Command::Undo as u64)],
    );
    assert_eq!(app.editor.document, before);
}

#[test]
fn quick_add_empty_return_edits_one_control_and_cancel_restores_query_focus() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 100., 100.);
    let before = app.editor.document.clone();
    let id = app.editor.selected_views()[0].id;
    app.quick_focus = true;
    query_input_frame(&ctx, &mut app, vec![]);
    query_input_frame(&ctx, &mut app, vec![key(Key::Enter)]);
    assert_eq!(app.text_edit.as_ref().map(|(id, _)| *id), Some(id));
    assert_eq!(app.editor.document, before);
    ctx.run_ui(egui::RawInput::default(), |ui| app.dialogs(ui.ctx()))
        .drop_without_applying_deltas();
    ctx.run_ui(
        egui::RawInput {
            events: vec![key(Key::Escape)],
            ..Default::default()
        },
        |ui| app.dialogs(ui.ctx()),
    )
    .drop_without_applying_deltas();
    assert!(app.text_edit.is_none());
    query_input_frame(&ctx, &mut app, vec![]);
    assert_eq!(ctx.memory(|m| m.focused()), app.quick_input_id);
    assert_eq!(app.editor.document, before);
}

#[test]
fn quick_add_empty_return_preserves_noneditable_and_multiple_selection_contexts() {
    for case in ["empty", "multiple", "group", "media"] {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        if case == "media" {
            app.editor.add(Kind::Video, 0., 0.);
        } else if case != "empty" {
            app.editor.add(Kind::Button, 0., 0.);
            app.editor.add(Kind::Label, 100., 100.);
            app.editor.select_all();
            if case == "group" {
                app.editor.group();
            }
        }
        let before = app.editor.document.clone();
        let selection = app.editor.selection.clone();
        let context = app.editor.context.clone();
        app.quick_focus = true;
        query_input_frame(&ctx, &mut app, vec![]);
        query_input_frame(&ctx, &mut app, vec![key(Key::Enter)]);
        assert!(app.text_edit.is_none(), "{case}");
        assert_eq!(app.editor.document, before, "{case}");
        assert_eq!(app.editor.selection, selection, "{case}");
        assert_eq!(app.editor.context, context, "{case}");
        assert_eq!(ctx.memory(|m| m.focused()), app.quick_input_id, "{case}");
    }
}

#[test]
fn quick_add_empty_return_converts_placeholder_with_separate_undo_after_cancel() {
    for (kind, converted) in [
        (Kind::TextLine, Kind::Subtitle),
        (Kind::TextBlock, Kind::Paragraph),
    ] {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(kind, 100., 100.);
        let before = app.editor.document.clone();
        app.quick_focus = true;
        query_input_frame(&ctx, &mut app, vec![]);
        query_input_frame(&ctx, &mut app, vec![key(Key::Enter)]);
        assert!(app.text_edit.is_some());
        assert_eq!(app.editor.selected_views()[0].kind, converted);
        ctx.run_ui(
            egui::RawInput {
                events: vec![key(Key::Escape)],
                ..Default::default()
            },
            |ui| app.dialogs(ui.ctx()),
        )
        .drop_without_applying_deltas();
        assert!(app.text_edit.is_none());
        assert_eq!(app.editor.selected_views()[0].kind, converted);
        query_input_frame(&ctx, &mut app, vec![]);
        query_input_frame(&ctx, &mut app, vec![history_key(false)]);
        assert_eq!(app.editor.document, before);
    }
}

#[test]
fn quick_add_presets_insert_their_properties_and_undo_in_one_step() {
    for preset in wireframe::workspace::CatalogPreset::ALL {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        let before = app.editor.document.clone();
        app.quick_focus = true;
        query_input_frame(&ctx, &mut app, vec![]);
        query_input_frame(
            &ctx,
            &mut app,
            vec![egui::Event::Paste(preset.name().into()), key(Key::Enter)],
        );
        assert!(app.error.is_none(), "{preset:?}: {:?}", app.error);
        assert_eq!(app.editor.nodes().len(), 1, "{preset:?}");
        let mut expected = wireframe::workspace::Editor::new(before.clone());
        let point = app.editor.nodes()[0].origin;
        expected.add_preset(preset, point.x, point.y);
        assert_eq!(app.editor.document, expected.document, "{preset:?}");
        query_input_frame(&ctx, &mut app, vec![history_key(false)]);
        assert_eq!(app.editor.document, before, "{preset:?}");
    }
}

#[test]
fn quick_add_asset_uses_shared_image_natural_size_and_one_step_history() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    let mut bytes = std::io::Cursor::new(vec![]);
    image::RgbaImage::from_pixel(80, 120, image::Rgba([200, 80, 30, 255]))
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let id = app.editor.document.allocate_id().unwrap();
    app.editor.document.assets.push(wireframe::document::Asset {
        trashed: false,
        id,
        name: "Shared test image".into(),
        media_type: "image/png".into(),
        bytes: bytes.into_inner(),
    });
    let before = app.editor.document.clone();
    app.quick = "Shared test image".into();
    app.quick_focus = true;
    quick_frame(&ctx, &mut app, vec![]);
    quick_frame(&ctx, &mut app, vec![key(Key::Enter)]);
    assert!(app.quick.is_empty());
    assert!(app.error.is_none());
    assert_eq!(app.editor.nodes().len(), 1);
    assert_eq!(
        app.editor.nodes()[0].properties.get("image"),
        Some(&wireframe::document::Value::Asset(id))
    );
    assert_eq!(app.editor.document.assets, before.assets);
    assert_eq!(app.editor.undo_name(), Some("Add image"));
    app.editor.undo();
    assert_eq!(app.editor.document, before);
}

#[test]
fn quick_add_individual_icon_retains_style_and_undo_in_one_step() {
    for (name, id) in [
        ("Star Regular", "star-regular"),
        ("Star Solid", "star-solid"),
    ] {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.categories = vec![CatalogCategory::Common];
        let before = app.editor.document.clone();
        app.quick_focus = true;
        query_input_frame(&ctx, &mut app, vec![]);
        query_input_frame(
            &ctx,
            &mut app,
            vec![egui::Event::Paste(name.into()), key(Key::Enter)],
        );
        assert!(app.error.is_none(), "{:?}", app.error);
        assert_eq!(app.editor.nodes().len(), 1);
        assert_eq!(
            app.editor.nodes()[0].properties.get("icon"),
            Some(&wireframe::document::Value::Icon(
                wireframe::document::IconSource::Catalog(id.into())
            ))
        );
        query_input_frame(&ctx, &mut app, vec![history_key(false)]);
        assert_eq!(app.editor.document, before);
    }
}

#[test]
fn icon_library_allocates_only_visible_accessible_tiles() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.categories = vec![CatalogCategory::Icons];
    let mut observed = vec![];
    for _ in 0..2 {
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
                ..Default::default()
            },
            |ui| app.library(ui),
        );
        let nodes = &output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes;
        let count = nodes.len();
        observed.extend(
            nodes
                .iter()
                .filter_map(|(_, n)| n.label().map(str::to_owned)),
        );
        output.drop_without_applying_deltas();
        assert!(
            count < 200,
            "offscreen catalog must not flood accessibility: {count}"
        );
    }
    assert!(
        observed.iter().any(|name| name == "0 Solid"),
        "{observed:?}"
    );
}

#[test]
fn asset_context_menu_moves_image_to_shared_trash_without_inserting_a_control() {
    fn labels(
        ctx: &egui::Context,
        app: &mut WireframeApp,
        events: Vec<egui::Event>,
    ) -> std::collections::HashMap<String, Rect> {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 850.))),
                time: Some(ctx.cumulative_frame_nr() as f64 * 0.25),
                events,
                ..Default::default()
            },
            |ui| app.library(ui),
        );
        let result = output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .filter_map(|(_, node)| {
                let (Some(label), Some(b)) = (node.label(), node.bounds()) else {
                    return None;
                };
                Some((
                    label.to_string(),
                    Rect::from_min_max(
                        egui::pos2(b.x0 as f32, b.y0 as f32),
                        egui::pos2(b.x1 as f32, b.y1 as f32),
                    ),
                ))
            })
            .collect();
        output.textures_delta.clear();
        result
    }
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    let id = app.editor.document.allocate_id().unwrap();
    let mut bytes = std::io::Cursor::new(vec![]);
    image::RgbaImage::from_pixel(12, 8, image::Rgba([40, 60, 90, 255]))
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    app.editor.document.assets.push(wireframe::document::Asset {
        id,
        trashed: false,
        name: "Photo".into(),
        media_type: "image/png".into(),
        bytes: bytes.into_inner(),
    });
    app.categories = vec![CatalogCategory::Assets];
    let before = app.editor.document.clone();
    let mut found = labels(&ctx, &mut app, vec![]);
    for _ in 0..2 {
        found = labels(&ctx, &mut app, vec![]);
    }
    let target = found["Photo"].center();
    for pressed in [true, false] {
        labels(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(target),
                egui::Event::PointerButton {
                    pos: target,
                    button: egui::PointerButton::Secondary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
    let mut found = labels(&ctx, &mut app, vec![]);
    for _ in 0..2 {
        found = labels(&ctx, &mut app, vec![]);
    }
    let action = found["Move Image to Trash"].center();
    for pressed in [true, false] {
        labels(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(action),
                egui::Event::PointerButton {
                    pos: action,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
    assert!(app.editor.document.assets[0].trashed);
    assert!(app.editor.nodes().is_empty());
    assert!(
        app.editor
            .retained_resources()
            .any(|(resource, _)| resource == wireframe::workspace::RetainedResource::Asset(id))
    );
    app.editor.undo();
    assert_eq!(app.editor.document, before);
}
