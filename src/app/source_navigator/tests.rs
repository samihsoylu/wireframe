// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
fn setup() -> (egui::Context, WireframeApp, Id, Id) {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    let library = app.editor.create_library("Contact controls").unwrap();
    let source = app
        .editor
        .create_empty_source(
            library,
            "Contact button",
            wireframe::document::Point { x: 6., y: 6. },
        )
        .unwrap();
    (ctx, app, library, source)
}

#[test]
fn retained_library_and_sources_disappear_and_restore_in_navigator() {
    let (ctx, mut app, library, source) = setup();
    app.editor.edit_library_source(source).unwrap();
    let nodes = frame(&ctx, &mut app, vec![]);
    assert!(nodes.contains_key("Contact controls"));
    assert!(nodes.contains_key("Contact button"));
    app.editor.trash_libraries(&[library]).unwrap();
    let nodes = frame(&ctx, &mut app, vec![]);
    assert!(!nodes.contains_key("Contact controls"));
    assert!(!nodes.contains_key("Contact button"));
    assert!(
        app.resource_selection(ResourceRow::Source(source))
            .is_empty()
    );
    assert!(app.source_navigation.selected.is_empty());
    app.editor.restore_libraries(&[library]).unwrap();
    app.editor.edit_library_source(source).unwrap();
    let nodes = frame(&ctx, &mut app, vec![]);
    assert!(nodes.contains_key("Contact controls"));
    assert!(nodes.contains_key("Contact button"));
}
fn frame(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    events: Vec<egui::Event>,
) -> HashMap<String, Rect> {
    frame_nodes(ctx, app, events)
        .into_iter()
        .map(|(name, (rect, _))| (name, rect))
        .collect()
}
fn frame_nodes(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    events: Vec<egui::Event>,
) -> HashMap<String, (Rect, bool)> {
    let mut events = events;
    if let Some(modifiers) = events.iter().rev().find_map(|event| match event {
        egui::Event::PointerButton { modifiers, .. } | egui::Event::Key { modifiers, .. } => {
            Some(*modifiers)
        }
        _ => None,
    }) {
        events.insert(0, egui::Event::ModifiersChanged(modifiers));
    }
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 800.))),
            events,
            ..Default::default()
        },
        |ui| {
            app.prepare_page_notes(ui.ctx());
            app.shortcuts(ui.ctx());
            app.toolbar(ui);
            app.sidebar(ui);
            app.inspector(ui);
            app.page_rename_dialog(ui.ctx());
        },
    );
    let nodes = output
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
    output.textures_delta.clear();
    nodes
}
fn confirm_cut(ctx: &egui::Context, app: &mut WireframeApp, result: Result<(), String>) {
    let request = app
        .pending_cut_request()
        .expect("Cut should request native publication");
    frame(
        ctx,
        app,
        vec![egui::Event::ClipboardWriteResult { request, result }],
    );
}
fn focus_source(ctx: &egui::Context, app: &mut WireframeApp, source: Id) {
    frame(ctx, app, vec![]);
    let id = *app
        .source_navigation
        .focus
        .iter()
        .find(|(_, target)| **target == ResourceRow::Source(source))
        .unwrap()
        .0;
    ctx.memory_mut(|m| m.request_focus(id));
}
#[test]
fn source_rows_replace_page_rows_and_resource_duplicate_preserves_page() {
    let (ctx, mut app, library, source) = setup();
    let page = app.editor.page_data().clone();
    frame(&ctx, &mut app, vec![]);
    let nodes = frame(&ctx, &mut app, vec![]);
    assert!(nodes.contains_key("Contact controls"));
    assert!(nodes.contains_key("Contact button"));
    assert!(nodes.contains_key("Back to Wireframes"));
    assert!(nodes.contains_key("New component library"));
    assert!(nodes.contains_key("Click here to edit notes."));
    assert!(!nodes.contains_key("New wireframe"));
    // Stale focus from a previously visible page row cannot route page actions.
    let stale = egui::Id::new("old-page-row");
    app.navigator.focus_ids.insert(stale);
    ctx.memory_mut(|m| m.request_focus(stale));
    assert_eq!(app.page_command_enabled(&ctx, menus::Command::Delete), None);
    focus_source(&ctx, &mut app, source);
    assert!(app.command_enabled(&ctx, menus::Command::Copy));
    assert!(!app.command_enabled(&ctx, menus::Command::Group));
    assert!(app.command_enabled(&ctx, menus::Command::Duplicate));
    app.run_command(&ctx, menus::Command::Duplicate);
    assert_ne!(app.editor.definition, Some(source));
    assert_eq!(app.editor.library, Some(library));
    assert_eq!(app.editor.document.definitions.len(), 2);
    assert_eq!(app.editor.document.libraries.len(), 1);
    assert_eq!(
        app.editor.document.definitions[1].name,
        "Contact button copy"
    );
    assert_eq!(app.editor.page_data(), &page);
    let duplicate = app.editor.definition.unwrap();
    frame(&ctx, &mut app, vec![]);
    let focus = ctx.memory(|m| m.focused()).unwrap();
    assert_eq!(
        app.source_navigation.focus.get(&focus),
        Some(&ResourceRow::Source(duplicate))
    );
    app.run_command(&ctx, menus::Command::Delete);
    assert!(app.editor.document.component_source(source).is_some());
    assert!(app.editor.document.component_source(duplicate).is_none());
    app.editor.undo();
    assert!(app.editor.document.component_source(duplicate).is_some());
    app.editor.undo();
    assert_eq!(app.editor.document.definitions.len(), 1);
    assert_eq!(app.editor.page_data(), &page);
}
#[test]
fn source_delete_is_explicit_and_undo_restores_identity() {
    let (ctx, mut app, _, source) = setup();
    let before = app.editor.document.clone();
    focus_source(&ctx, &mut app, source);
    app.run_command(&ctx, menus::Command::Delete);
    assert!(app.editor.document.component_source(source).is_none());
    assert_eq!(app.editor.document.pages, before.pages);
    app.editor.undo();
    assert!(app.editor.document == before);
}
#[test]
fn library_resource_commands_do_not_fall_through_to_canvas() {
    let (ctx, mut app, library, source) = setup();
    app.editor.edit_library(library).unwrap();
    let placement = app.editor.document.component_source(source).unwrap().1.node;
    app.editor.select(placement, false);
    frame(&ctx, &mut app, vec![]);
    let id = *app
        .source_navigation
        .focus
        .iter()
        .find(|(_, target)| matches!(target, ResourceRow::Library(_)))
        .unwrap()
        .0;
    ctx.memory_mut(|m| m.request_focus(id));
    for command in [menus::Command::Delete, menus::Command::Duplicate] {
        assert!(!app.command_enabled(&ctx, command));
        app.run_command(&ctx, command);
    }
    let before = app.editor.document.clone();
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::Paste("Wrong destination".into())],
    );
    assert!(app.editor.document == before);
}
#[test]
fn source_rename_and_library_creation_return_preserve_page() {
    let (ctx, mut app, library, source) = setup();
    let page = app.editor.page_data().clone();
    for (target, is_library, name) in [
        (source, false, "Renamed action"),
        (library, true, "Renamed library"),
    ] {
        app.begin_source_rename(target, is_library);
        frame(&ctx, &mut app, vec![]);
        let nodes = frame(&ctx, &mut app, vec![]);
        assert!(nodes.contains_key("Cancel"));
        frame(&ctx, &mut app, vec![egui::Event::Paste(name.into())]);
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::Key {
                key: Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
        assert!(app.page_rename.is_none());
        assert_eq!(app.editor.document.surface(target).unwrap().0, name);
    }
    app.new_component_library();
    assert_ne!(app.editor.library, Some(library));
    assert_eq!(app.editor.document.libraries.len(), 2);
    app.back_to_wireframes();
    assert!(!app.editor.in_source_workspace());
    assert_eq!(app.editor.page_data(), &page);
    assert!(app.editor.selection.is_empty());
}

#[test]
fn long_source_names_keep_navigator_width_stable() {
    let (ctx, mut app, library, source) = setup();
    app.editor
        .rename_library(library, &"Shared controls ".repeat(40))
        .unwrap();
    app.editor
        .rename_definition(source, "Contact action ".repeat(40))
        .unwrap();
    let mut edges = vec![];
    for _ in 0..8 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(940., 700.))),
                ..Default::default()
            },
            |ui| {
                app.sidebar(ui);
                edges.push(ui.available_rect_before_wrap().left());
            },
        );
        output.textures_delta.clear();
    }
    assert!(edges.iter().all(|edge| *edge <= 301.), "{edges:?}");
    assert!((edges[7] - edges[6]).abs() < 0.1, "{edges:?}");
}

#[test]
fn owning_placement_exposes_source_editing_not_instance_reset_or_detach() {
    let (ctx, mut app, library, source) = setup();
    app.editor.edit_library(library).unwrap();
    let placement = app.editor.document.component_source(source).unwrap().1.node;
    app.editor.select(placement, false);
    frame(&ctx, &mut app, vec![]);
    let nodes = frame(&ctx, &mut app, vec![]);
    assert!(nodes.contains_key("Edit component source"));
    assert!(!nodes.contains_key("Edit this instance"));
    assert!(!nodes.contains_key("Reset instance overrides"));
    assert!(!nodes.contains_key("Detach component"));
}

fn focus_library(ctx: &egui::Context, app: &mut WireframeApp) {
    frame(ctx, app, vec![]);
    let id = *app
        .source_navigation
        .focus
        .iter()
        .find(|(_, target)| matches!(target, ResourceRow::Library(_)))
        .unwrap()
        .0;
    ctx.memory_mut(|m| m.request_focus(id));
}
#[test]
fn source_clipboard_keyboard_routes_destinations_and_repeated_offsets() {
    use wireframe::document::{NodeContent, Point};
    let (ctx, mut app, library, source) = setup();
    app.editor.add(Kind::Button, 0., 0.);
    focus_source(&ctx, &mut app, source);
    frame(&ctx, &mut app, vec![egui::Event::Copy]);
    assert!(app.clipboard.source.is_some());
    let text = format!(
        "wireframe-nodes-v2:{}",
        serde_json::to_string(&app.clipboard).unwrap()
    );
    app.editor.edit_library(library).unwrap();
    focus_library(&ctx, &mut app);
    assert!(app.command_enabled(&ctx, menus::Command::Paste));
    frame(&ctx, &mut app, vec![egui::Event::Paste(text.clone())]);
    assert!(app.error.is_none(), "{:?}", app.error);
    assert!(app.editor.error.is_none(), "{:?}", app.editor.error);
    assert_eq!(app.editor.document.definitions.len(), 2);
    assert_eq!(
        app.editor.document.definitions.last().unwrap().name,
        "Contact button (1)"
    );
    assert_eq!(
        app.editor.document.libraries[0]
            .content
            .nodes
            .last()
            .unwrap()
            .origin,
        Point::new(16., 16.)
    );
    let pasted_source = app.editor.document.definitions.last().unwrap().id;
    assert_eq!(app.editor.definition, Some(pasted_source));
    assert!(app.editor.selection.is_empty());
    frame(&ctx, &mut app, vec![]);
    assert!(app.command_enabled(&ctx, menus::Command::Copy));
    assert_eq!(
        ctx.memory(|m| m.focused()).and_then(|id| app
            .source_navigation
            .focus
            .get(&id)
            .copied()
            .and_then(ResourceRow::source)),
        Some(pasted_source)
    );
    app.editor.edit_library_source(source).unwrap();
    focus_source(&ctx, &mut app, source);
    frame(&ctx, &mut app, vec![egui::Event::Paste(text.clone())]);
    assert!(matches!(
        app.editor.nodes().last().unwrap().content,
        NodeContent::Group { .. }
    ));
    assert_eq!(
        app.editor.nodes().last().unwrap().origin,
        Point::new(20., 20.)
    );
    app.back_to_wireframes();
    frame(&ctx, &mut app, vec![egui::Event::Paste(text)]);
    assert!(matches!(
        app.editor.nodes().last().unwrap().content,
        NodeContent::Group { .. }
    ));
    assert_eq!(
        app.editor.nodes().last().unwrap().origin,
        Point::new(6., 6.)
    );
}
#[test]
fn source_cut_reconnects_through_app_and_undo_keeps_document() {
    let (ctx, mut app, library, source) = setup();
    app.editor.add(Kind::Button, 0., 0.);
    app.back_to_wireframes();
    app.editor.insert_component(source, 100., 100.).unwrap();
    app.editor.edit_library_source(source).unwrap();
    let before = app.editor.document.clone();
    focus_source(&ctx, &mut app, source);
    frame(&ctx, &mut app, vec![egui::Event::Cut]);
    assert_eq!(app.editor.document, before);
    confirm_cut(&ctx, &mut app, Ok(()));
    assert!(app.editor.document.component_source(source).is_none());
    let cut = app.editor.document.clone();
    app.editor.edit_library(library).unwrap();
    focus_library(&ctx, &mut app);
    app.run_command(&ctx, menus::Command::PasteControls);
    assert!(app.editor.error.is_none(), "{:?}", app.editor.error);
    let new = app.editor.document.definitions.last().unwrap().id;
    assert_ne!(new, source);
    let wireframe::document::NodeContent::Instance { definition, .. } =
        app.editor.document.pages[0].content.nodes[0].content
    else {
        panic!()
    };
    assert_eq!(definition, new);
    assert_eq!(
        app.editor.document.libraries[0]
            .content
            .nodes
            .last()
            .unwrap()
            .origin,
        wireframe::document::Point::new(6., 6.)
    );
    app.editor.undo();
    assert_eq!(app.editor.document, cut);
    app.editor.undo();
    assert_eq!(app.editor.document, before);
}
#[test]
fn clipboard_origin_distinguishes_sessions_and_uses_canonical_named_files() {
    let (_, mut app, _, _) = setup();
    let (_, mut other, _, _) = setup();
    assert_ne!(app.clipboard_origin(), other.clipboard_origin());
    let origin = app.clipboard_origin();
    app.editor.add_page();
    app.editor.undo();
    assert_eq!(app.clipboard_origin(), origin);
    let file = tempfile::NamedTempFile::new().unwrap();
    app.file = Some(file.path().into());
    other.file = app.file.clone();
    assert_eq!(app.clipboard_origin(), other.clipboard_origin());
    assert!(
        !app.clipboard_origin()
            .contains(file.path().to_str().unwrap())
    );
    let other_file = tempfile::NamedTempFile::new().unwrap();
    other.file = Some(other_file.path().into());
    assert_ne!(app.clipboard_origin(), other.clipboard_origin());
}
#[test]
fn source_paste_keeps_top_left_visible_and_rejects_failure_without_offset_advance() {
    let (ctx, mut app, library, source) = setup();
    app.editor.add(Kind::Button, 0., 0.);
    app.copy_source_resource(&ctx, source, false).unwrap();
    app.editor.edit_library(library).unwrap();
    app.canvas_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(200., 200.));
    app.pan = Vec2::new(-500., -300.);
    let copy = app.clipboard.clone();
    app.paste_clipboard(&copy);
    assert!(app.error.is_none(), "{:?}", app.error);
    assert!(app.editor.error.is_none(), "{:?}", app.editor.error);
    let point = app.editor.document.libraries[0]
        .content
        .nodes
        .last()
        .unwrap()
        .origin;
    assert_eq!(point, wireframe::document::Point::new(500., 300.));
    app.editor.edit_library(library).unwrap();
    app.canvas_rect = Rect::NOTHING;
    app.reset_paste_offset(10.);
    let mut bad = copy.clone();
    bad.source.as_mut().unwrap().node += 1;
    let before = app.editor.document.clone();
    app.paste_clipboard(&bad);
    assert!(app.editor.error.take().is_some());
    assert_eq!(app.editor.document, before);
    app.paste_clipboard(&copy);
    assert_eq!(
        app.editor.document.libraries[0]
            .content
            .nodes
            .last()
            .unwrap()
            .origin,
        wireframe::document::Point::new(16., 16.)
    );
}

#[test]
fn rejected_large_clipboard_keeps_cut_controls_and_history() {
    use wireframe::document::{Asset, IconSource, Node, Point, Size, Value};
    let (ctx, mut app, _, _) = setup();
    app.back_to_wireframes();
    let mut project = app.editor.document.clone();
    project.font = None;
    project.next_id = 1000;
    project.assets.push(Asset {
        attribution: None,
        trashed: false,
        id: 900,
        name: "Large icon".into(),
        media_type: "image/svg+xml".into(),
        bytes: format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"20\" height=\"20\">{}</svg>",
            " ".repeat(clipboard::MAX_JSON_BYTES / 3 + 1)
        )
        .into_bytes(),
    });
    let mut node = Node::control(100, "button", Point::default(), Size::default());
    node.properties
        .insert("icon".into(), Value::Icon(IconSource::Asset(900)));
    project.pages[0].content.nodes = vec![node];
    project.validate().unwrap();
    app.editor = Editor::new(project);
    app.editor.select_all();
    assert!(app.command_enabled(&ctx, menus::Command::Cut));
    let before = app.editor.document.clone();
    let undo = app.editor.undo_name().map(str::to_owned);
    app.run_command(&ctx, menus::Command::Cut);
    assert_eq!(app.error.as_deref(), Some("Clipboard content is too large"));
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.undo_name(), undo.as_deref());
}

#[test]
fn multiple_source_rows_copy_cut_and_library_paste_keep_scope_and_one_history_entry() {
    let (ctx, mut app, library, first) = setup();
    app.editor.add(Kind::Button, 0., 0.);
    app.editor.duplicate_source(first).unwrap();
    let second = app.editor.definition.unwrap();
    app.select_source_row(first, egui::Modifiers::NONE).unwrap();
    app.select_source_row(second, egui::Modifiers::SHIFT)
        .unwrap();
    focus_source(&ctx, &mut app, second);
    assert_eq!(app.source_resource_selection(second), vec![first, second]);
    let before = app.editor.document.clone();
    frame(&ctx, &mut app, vec![egui::Event::Copy]);
    assert_eq!(app.clipboard.sources.len(), 2);
    assert_eq!(app.editor.document, before);
    frame(&ctx, &mut app, vec![egui::Event::Cut]);
    assert_eq!(app.editor.document, before);
    confirm_cut(&ctx, &mut app, Ok(()));
    assert!(app.editor.document.definitions.is_empty());
    let cut = app.editor.document.clone();
    app.editor.edit_library(library).unwrap();
    focus_library(&ctx, &mut app);
    app.run_command(&ctx, menus::Command::PasteControls);
    frame(&ctx, &mut app, vec![]);
    assert_eq!(app.editor.definition, None);
    assert_eq!(app.editor.selection.len(), 2);
    assert!(!app.source_navigation_focused(&ctx));
    assert!(app.command_enabled(&ctx, menus::Command::Copy));
    frame(&ctx, &mut app, vec![egui::Event::Copy]);
    assert_eq!(app.clipboard.sources.len(), 2);
    assert!(
        app.clipboard
            .nodes
            .iter()
            .all(|n| matches!(n.content, wireframe::document::NodeContent::Group { .. }))
    );
    app.editor.undo();
    assert_eq!(app.editor.document, cut);
    app.editor.undo();
    assert_eq!(app.editor.document, before);
}

#[test]
fn source_range_toggle_and_explicit_context_target_are_distinct() {
    let (_, mut app, library, first) = setup();
    let second = app
        .editor
        .create_empty_source(library, "Second", Default::default())
        .unwrap();
    let third = app
        .editor
        .create_empty_source(library, "Third", Default::default())
        .unwrap();
    app.select_source_row(first, egui::Modifiers::NONE).unwrap();
    app.select_source_row(third, egui::Modifiers::SHIFT)
        .unwrap();
    assert_eq!(
        app.source_resource_selection(second),
        vec![first, second, third]
    );
    app.select_source_row(second, egui::Modifiers::COMMAND)
        .unwrap();
    assert_eq!(app.source_resource_selection(first), vec![first, third]);
    assert_eq!(app.source_resource_selection(second), vec![second]);
    app.select_source_row(third, egui::Modifiers::NONE).unwrap();
    assert_eq!(app.source_resource_selection(third), vec![third]);
    app.back_to_wireframes();
    assert!(app.source_navigation.selected.is_empty());
}

#[test]
fn multi_source_paste_viewport_uses_all_roots_without_changing_relative_positions() {
    use wireframe::document::Point;
    let (ctx, mut app, library, first) = setup();
    app.editor.add(Kind::Button, 0., 0.);
    app.editor.duplicate_source(first).unwrap();
    let second = app.editor.definition.unwrap();
    app.editor.document.libraries[0].content.nodes[0].origin = Point::new(800., 0.);
    app.editor.document.libraries[0].content.nodes[1].origin = Point::new(0., 0.);
    let copy = app
        .editor
        .copy_sources(&[first, second], &app.clipboard_origin())
        .unwrap();
    app.editor.edit_library(library).unwrap();
    focus_library(&ctx, &mut app);
    app.canvas_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(200., 200.));
    app.pan = Vec2::new(-500., -300.);
    app.paste_clipboard(&copy);
    assert!(app.error.is_none(), "{:?}", app.error);
    assert!(app.editor.error.is_none(), "{:?}", app.editor.error);
    let pasted = &app.editor.document.libraries[0].content.nodes[2..];
    assert_eq!(pasted[0].origin, Point::new(1300., 300.));
    assert_eq!(pasted[1].origin, Point::new(500., 300.));
    assert_eq!(app.editor.selection.len(), 2);
}

#[test]
fn multi_source_command_duplicate_is_disabled_but_context_target_is_single() {
    let (ctx, mut app, library, first) = setup();
    app.editor.add(Kind::Button, 0., 0.);
    let second = app
        .editor
        .create_empty_source(library, "Second", Default::default())
        .unwrap();
    app.select_source_row(first, egui::Modifiers::NONE).unwrap();
    app.select_source_row(second, egui::Modifiers::SHIFT)
        .unwrap();
    focus_source(&ctx, &mut app, second);
    let before = app.editor.document.clone();
    assert!(!app.command_enabled(&ctx, menus::Command::Duplicate));
    app.run_command(&ctx, menus::Command::Duplicate);
    assert_eq!(app.editor.document, before);

    // Exercise the row menu: Rename is disabled, while Duplicate has one explicit target.
    let row_id = *app
        .source_navigation
        .focus
        .iter()
        .find(|(_, target)| **target == ResourceRow::Source(first))
        .unwrap()
        .0;
    let pos = ctx.read_response(row_id).unwrap().rect.center();
    let click = |app: &mut WireframeApp, pos, button| {
        for pressed in [true, false] {
            frame(
                &ctx,
                app,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
    };
    click(&mut app, pos, egui::PointerButton::Secondary);
    frame(&ctx, &mut app, vec![]);
    let menu = frame_nodes(&ctx, &mut app, vec![]);
    assert!(menu["Rename…"].1);
    assert!(!menu["Duplicate"].1);
    click(
        &mut app,
        menu["Duplicate"].0.center(),
        egui::PointerButton::Primary,
    );
    frame(&ctx, &mut app, vec![]);
    let copied = app.editor.definition.unwrap();
    assert_eq!(app.editor.document.definitions.len(), 3);
    assert_eq!(app.source_resource_selection(copied), vec![copied]);
    let copy = app
        .editor
        .document
        .definitions
        .iter()
        .find(|d| d.id == copied)
        .unwrap();
    assert_eq!(copy.name, "Contact button copy");
    assert_eq!(copy.content.nodes.len(), 1);
    assert_eq!(app.editor.document.definitions[0], before.definitions[0]);
    assert_eq!(app.editor.document.definitions[1], before.definitions[1]);
    app.editor.undo();
    assert_eq!(app.editor.document, before);
}

#[test]
fn library_row_copy_uses_resources_and_paste_preserves_active_library() {
    let (ctx, mut app, library, source) = setup();
    app.editor.edit_library(library).unwrap();
    frame(&ctx, &mut app, vec![]);
    let id = *app
        .source_navigation
        .focus
        .iter()
        .find(|(_, target)| matches!(target, ResourceRow::Library(_)))
        .unwrap()
        .0;
    ctx.memory_mut(|m| m.request_focus(id));
    assert!(app.command_enabled(&ctx, menus::Command::Copy));
    assert!(app.command_enabled(&ctx, menus::Command::Cut));
    let before = app.editor.document.clone();
    app.run_command(&ctx, menus::Command::Copy);
    assert_eq!(
        app.clipboard.library_resources().unwrap().requested,
        vec![library]
    );
    assert_eq!(app.editor.document, before);
    app.run_command(&ctx, menus::Command::PasteControls);
    assert_eq!(app.editor.library, Some(library));
    assert!(app.editor.definition.is_none());
    assert_eq!(app.editor.document.libraries.len(), 2);
    assert_eq!(app.editor.document.pages, before.pages);
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    app.editor.edit_library_source(source).unwrap();
    focus_source(&ctx, &mut app, source);
    app.run_command(&ctx, menus::Command::Copy);
    assert!(app.clipboard.has_sources());
    assert!(app.clipboard.resources.is_none());
}

#[test]
fn library_copy_targets_the_focused_row_even_when_another_canvas_is_active() {
    let (ctx, mut app, first, _) = setup();
    let second = app.editor.create_library("Checkout controls").unwrap();
    app.editor.edit_library(first).unwrap();
    frame(&ctx, &mut app, vec![]);
    let id = *app
        .source_navigation
        .focus
        .iter()
        .find(|(_, target)| **target == ResourceRow::Library(second))
        .unwrap()
        .0;
    ctx.memory_mut(|m| m.request_focus(id));
    app.run_command(&ctx, menus::Command::Copy);
    assert_eq!(
        app.clipboard.library_resources().unwrap().requested,
        vec![second]
    );
    assert_eq!(app.editor.library, Some(first));
}

#[test]
fn consecutive_duplicate_and_copy_use_new_source_before_next_frame() {
    let (ctx, mut app, _, source) = setup();
    focus_source(&ctx, &mut app, source);
    app.run_command(&ctx, menus::Command::Duplicate);
    let copied = app.editor.definition.unwrap();
    assert_ne!(copied, source);
    app.run_command(&ctx, menus::Command::Copy);
    assert_eq!(app.clipboard.source.as_ref().unwrap().definition, copied);
}

#[test]
fn cross_library_source_copy_includes_complete_owners_and_paste_collapses_selection() {
    let (ctx, mut app, first_library, first) = setup();
    let first_extra = app
        .editor
        .create_empty_source(first_library, "Unselected first source", Default::default())
        .unwrap();
    let second_library = app.editor.create_library("Checkout controls").unwrap();
    let second = app
        .editor
        .create_empty_source(second_library, "Checkout button", Default::default())
        .unwrap();
    let second_extra = app
        .editor
        .create_empty_source(
            second_library,
            "Unselected second source",
            Default::default(),
        )
        .unwrap();
    app.select_source_row(first, Modifiers::NONE).unwrap();
    frame(&ctx, &mut app, vec![]);
    frame(&ctx, &mut app, vec![]);
    let row_id = *app
        .source_navigation
        .focus
        .iter()
        .find(|(_, row)| **row == ResourceRow::Source(second))
        .unwrap()
        .0;
    let pos = ctx.read_response(row_id).unwrap().rect.center();
    frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::COMMAND,
            },
        ],
    );
    frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::COMMAND,
            },
            // Native input may release the key before this click frame runs.
            egui::Event::ModifiersChanged(Modifiers::NONE),
        ],
    );
    assert_eq!(
        app.editor.definition,
        Some(first),
        "selected={:?}, surface={:?}, modifiers={:?}",
        app.source_navigation.selected,
        app.source_navigation.surface,
        ctx.input(|i| i.modifiers)
    );
    assert_eq!(app.source_resource_selection(second), vec![first, second]);
    assert!(!app.command_enabled(&ctx, menus::Command::Duplicate));
    assert!(app.command_enabled(&ctx, menus::Command::Cut));
    assert!(!app.command_enabled(&ctx, menus::Command::Delete));
    app.run_command(&ctx, menus::Command::Copy);
    let payload = app.clipboard.library_resources().unwrap();
    assert_eq!(payload.requested, vec![first_library, second_library]);
    assert_eq!(payload.definitions.len(), 4);
    assert!(payload.definitions.iter().any(|d| d.id == first_extra));
    assert!(payload.definitions.iter().any(|d| d.id == second_extra));
    let before = app.editor.document.clone();
    app.run_command(&ctx, menus::Command::Cut);
    app.run_command(&ctx, menus::Command::Delete);
    assert_eq!(app.editor.document, before);
    app.run_command(&ctx, menus::Command::PasteControls);
    assert_eq!(app.editor.definition, Some(first));
    assert_eq!(app.editor.library, Some(first_library));
    assert_eq!(app.editor.document.libraries.len(), 4);
    assert_eq!(app.editor.document.definitions.len(), 8);
    assert_eq!(app.source_resource_selection(first), vec![first]);
    let after = app.editor.document.clone();
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    app.editor.redo();
    assert_eq!(app.editor.document, after);
}

#[test]
fn mixed_library_source_copy_deduplicates_owners_and_context_outside_selection_is_explicit() {
    let (ctx, mut app, first_library, first) = setup();
    let extra = app
        .editor
        .create_empty_source(first_library, "Extra", Default::default())
        .unwrap();
    let second_library = app.editor.create_library("Checkout").unwrap();
    let second = app
        .editor
        .create_empty_source(second_library, "Checkout source", Default::default())
        .unwrap();
    app.select_resource_row(ResourceRow::Library(first_library), Modifiers::NONE)
        .unwrap();
    app.select_source_row(first, Modifiers::COMMAND).unwrap();
    app.copy_source_resource(&ctx, first, false).unwrap();
    assert_eq!(
        app.clipboard.library_resources().unwrap().requested,
        vec![first_library]
    );
    app.select_source_row(second, Modifiers::COMMAND).unwrap();
    let selected = app.source_navigation.selected.clone();
    app.copy_selected_resource(&ctx, ResourceRow::Library(first_library), false)
        .unwrap();
    assert_eq!(
        app.clipboard.library_resources().unwrap().requested,
        vec![first_library, second_library]
    );
    app.copy_source_resource(&ctx, extra, false).unwrap();
    assert!(app.clipboard.resources.is_none());
    assert_eq!(app.clipboard.source.as_ref().unwrap().definition, extra);
    assert_eq!(app.source_navigation.selected, selected);
    let before = app.editor.document.clone();
    let clipboard_before = serde_json::to_value(&app.clipboard).unwrap();
    assert!(app.copy_source_resource(&ctx, second, true).is_ok());
    confirm_cut(&ctx, &mut app, Err("Clipboard unavailable".into()));
    assert_eq!(
        serde_json::to_value(&app.clipboard).unwrap(),
        clipboard_before
    );
    assert_eq!(app.editor.document, before);
}

#[test]
fn visible_range_across_headers_excludes_collapsed_children_and_toggle_keeps_active_member() {
    let (_, mut app, first_library, first) = setup();
    let hidden = app
        .editor
        .create_empty_source(first_library, "Hidden", Default::default())
        .unwrap();
    let second_library = app.editor.create_library("Checkout").unwrap();
    let second = app
        .editor
        .create_empty_source(second_library, "Second", Default::default())
        .unwrap();
    app.select_resource_row(ResourceRow::Library(first_library), Modifiers::NONE)
        .unwrap();
    app.source_navigation.visible = vec![
        ResourceRow::Library(first_library),
        ResourceRow::Library(second_library),
        ResourceRow::Source(second),
    ];
    app.select_source_row(second, Modifiers::SHIFT).unwrap();
    assert_eq!(
        app.source_navigation.selected,
        HashSet::from([
            ResourceRow::Library(first_library),
            ResourceRow::Library(second_library),
            ResourceRow::Source(second)
        ])
    );
    assert!(
        !app.source_navigation
            .selected
            .contains(&ResourceRow::Source(first))
    );
    assert!(
        !app.source_navigation
            .selected
            .contains(&ResourceRow::Source(hidden))
    );
    assert_eq!(app.editor.library, Some(first_library));
    app.select_resource_row(ResourceRow::Library(first_library), Modifiers::COMMAND)
        .unwrap();
    assert_eq!(app.editor.library, Some(second_library));
    assert!(app.editor.definition.is_none());
    app.select_resource_row(ResourceRow::Library(second_library), Modifiers::COMMAND)
        .unwrap();
    assert_eq!(app.editor.definition, Some(second));
    app.select_source_row(second, Modifiers::COMMAND).unwrap();
    assert!(app.source_navigation.selected.is_empty());
    assert_eq!(app.editor.library, Some(second_library));
    app.sync_resource_selection();
    assert!(app.source_navigation.selected.is_empty());
}

#[test]
fn shift_range_to_library_uses_pointer_snapshot_after_modifier_release() {
    let (ctx, mut app, first_library, first) = setup();
    let second_library = app.editor.create_library("Checkout controls").unwrap();
    app.select_source_row(first, Modifiers::NONE).unwrap();
    frame(&ctx, &mut app, vec![]);
    frame(&ctx, &mut app, vec![]);
    let row_id = *app
        .source_navigation
        .focus
        .iter()
        .find(|(_, row)| **row == ResourceRow::Library(second_library))
        .unwrap()
        .0;
    let pos = ctx.read_response(row_id).unwrap().rect.center();
    for pressed in [true, false] {
        frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::SHIFT,
                },
                egui::Event::ModifiersChanged(Modifiers::NONE),
            ],
        );
    }
    assert_eq!(app.editor.definition, Some(first));
    assert_eq!(
        app.resource_selection(ResourceRow::Library(second_library)),
        vec![
            ResourceRow::Source(first),
            ResourceRow::Library(second_library)
        ]
    );
    app.run_command(&ctx, menus::Command::Copy);
    assert_eq!(
        app.clipboard.library_resources().unwrap().requested,
        vec![first_library, second_library]
    );
}

fn click_context_item(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    target: ResourceRow,
    label: &str,
) {
    frame(ctx, app, vec![]);
    frame(ctx, app, vec![]);
    let row = *app
        .source_navigation
        .focus
        .iter()
        .find(|(_, row)| **row == target)
        .unwrap()
        .0;
    let position = ctx.read_response(row).unwrap().rect.center();
    let click = |app: &mut WireframeApp, pos, button| {
        for pressed in [true, false] {
            frame(
                ctx,
                app,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
    };
    click(app, position, egui::PointerButton::Secondary);
    frame(ctx, app, vec![]);
    let menu = frame_nodes(ctx, app, vec![]);
    assert!(!menu[label].1, "{label} should be available");
    click(app, menu[label].0.center(), egui::PointerButton::Primary);
    frame(ctx, app, vec![]);
}

#[test]
fn library_context_trash_distinguishes_headings_mixed_and_explicit_targets() {
    for case in 0..5 {
        let (ctx, mut app, library, source) = setup();
        let other = app.editor.create_library("Other library").unwrap();
        let other_source = app
            .editor
            .create_empty_source(other, "Other source", Default::default())
            .unwrap();
        let third = app.editor.create_library("Unselected library").unwrap();
        app.select_resource_row(ResourceRow::Library(library), egui::Modifiers::NONE)
            .unwrap();
        match case {
            1 | 4 => app
                .select_resource_row(ResourceRow::Library(other), egui::Modifiers::COMMAND)
                .unwrap(),
            2 => app
                .select_source_row(other_source, egui::Modifiers::COMMAND)
                .unwrap(),
            3 => app
                .select_source_row(source, egui::Modifiers::COMMAND)
                .unwrap(),
            _ => (),
        }
        let target = if case == 4 { third } else { library };
        let before = app.editor.document.clone();
        let expected = if case == 1 {
            vec![library, other]
        } else {
            vec![target]
        };
        let label = if case == 1 {
            "Move 2 Libraries to Trash"
        } else {
            "Move Library to Trash"
        };
        click_context_item(&ctx, &mut app, ResourceRow::Library(target), label);
        let mut after = before.clone();
        for library in &mut after.libraries {
            if expected.contains(&library.id) {
                library.trashed = true;
            }
        }
        assert_eq!(app.editor.document, after, "case {case}");
        assert!(app.error.is_none());
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        app.editor.redo();
        assert_eq!(app.editor.document, after);
    }
}

#[test]
fn source_context_delete_excludes_headings_and_keeps_other_sources() {
    for case in 0..3 {
        let (ctx, mut app, library, source) = setup();
        let other = app.editor.create_library("Other library").unwrap();
        let selected = app
            .editor
            .create_empty_source(other, "Selected source", Default::default())
            .unwrap();
        let untouched = app
            .editor
            .create_empty_source(other, "Untouched source", Default::default())
            .unwrap();
        let first_row = if case == 0 {
            ResourceRow::Library(library)
        } else {
            ResourceRow::Source(source)
        };
        app.select_resource_row(first_row, egui::Modifiers::NONE)
            .unwrap();
        app.select_source_row(selected, egui::Modifiers::COMMAND)
            .unwrap();
        let target = if case == 2 { untouched } else { selected };
        let removed = if case == 1 {
            vec![source, selected]
        } else {
            vec![target]
        };
        let before = app.editor.document.clone();
        let label = if case == 1 {
            "Delete 2 component sources"
        } else {
            "Delete component source"
        };
        click_context_item(&ctx, &mut app, ResourceRow::Source(target), label);
        assert!(app.error.is_none());
        assert_eq!(app.editor.document.pages, before.pages);
        assert_eq!(app.editor.document.libraries.len(), before.libraries.len());
        assert!(
            app.editor
                .document
                .libraries
                .iter()
                .all(|library| !library.trashed)
        );
        for definition in &before.definitions {
            let after = app
                .editor
                .document
                .definitions
                .iter()
                .find(|d| d.id == definition.id);
            if removed.contains(&definition.id) {
                assert!(after.is_none());
            } else {
                assert_eq!(after, Some(definition));
            }
        }
        let after = app.editor.document.clone();
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        app.editor.redo();
        assert_eq!(app.editor.document, after);
    }
}

#[test]
fn source_transfer_drag_keeps_definitions_and_supports_coalesced_input() {
    for schedule in 0..3 {
        let (ctx, mut app, original, source) = setup();
        let target = app.editor.create_library("Destination controls").unwrap();
        app.editor
            .create_empty_source(
                target,
                "Other button",
                wireframe::document::Point::new(150., 20.),
            )
            .unwrap();
        let mut editor = Editor::new(app.editor.document.clone());
        editor.edit_library_source(source).unwrap();
        app.editor = editor;
        for _ in 0..3 {
            frame(&ctx, &mut app, vec![]);
        }
        let nodes = frame(&ctx, &mut app, vec![]);
        let start = nodes["Contact button"].center();
        let end = nodes["Other button"].center();
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            pressed,
            button: egui::PointerButton::Primary,
            modifiers: Modifiers::NONE,
        };
        let before = app.editor.document.clone();
        frame(&ctx, &mut app, vec![egui::Event::PointerMoved(start)]);
        if schedule == 0 {
            frame(&ctx, &mut app, vec![button(start, true)]);
            frame(&ctx, &mut app, vec![egui::Event::PointerMoved(end)]);
            frame(&ctx, &mut app, vec![button(end, false)]);
        } else if schedule == 1 {
            frame(
                &ctx,
                &mut app,
                vec![
                    button(start, true),
                    egui::Event::PointerMoved(end),
                    button(end, false),
                ],
            );
        } else {
            frame(&ctx, &mut app, vec![button(start, true)]);
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(end), button(end, false)],
            );
        }
        assert_eq!(app.editor.library, Some(target), "schedule {schedule}");
        assert_eq!(app.editor.definition, Some(source));
        assert_eq!(app.editor.document.definitions, before.definitions);
        assert_eq!(app.editor.document.pages, before.pages);
        assert!(app.source_navigation.drag.is_none());
        let after = app.editor.document.clone();
        app.editor.undo();
        assert!(app.editor.document == before);
        assert_eq!(app.editor.library, Some(original));
        app.editor.redo();
        assert!(app.editor.document == after);
    }
}

#[test]
fn source_drag_cancel_and_stale_context_preserve_history() {
    for case in 0..5 {
        let (ctx, mut app, _, source) = setup();
        let target = app.editor.create_library("Destination controls").unwrap();
        app.editor.edit_library_source(source).unwrap();
        let original = app.editor.document.component_source(source).unwrap().0.id;
        app.editor
            .move_sources_to_library(&[source], target)
            .unwrap();
        app.editor.undo();
        for _ in 0..3 {
            frame(&ctx, &mut app, vec![]);
        }
        let nodes = frame(&ctx, &mut app, vec![]);
        let start = nodes["Contact button"].center();
        let end = nodes["Destination controls"].center();
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            pressed,
            button: egui::PointerButton::Primary,
            modifiers: Modifiers::NONE,
        };
        let before = app.editor.document.clone();
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(start), button(start, true)],
        );
        match case {
            0 => {
                frame(
                    &ctx,
                    &mut app,
                    vec![egui::Event::Key {
                        key: Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::NONE,
                    }],
                );
            }
            1 => {
                app.editor.edit_library(original).unwrap();
            }
            2 => {
                app.error = Some("Blocking dialog".into());
                frame(&ctx, &mut app, vec![]);
                app.error = None;
            }
            3 => {
                app.show_navigator = false;
                frame(&ctx, &mut app, vec![]);
                app.show_navigator = true;
            }
            _ => {}
        }
        let end = if case == 4 {
            Pos2::new(600., 400.)
        } else {
            end
        };
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(end), button(end, false)],
        );
        assert!(app.editor.document == before, "case {case}");
        assert!(app.editor.redo_name().is_some(), "case {case}");
        assert!(app.source_navigation.drag.is_none());
    }
}

fn navigation_key(key: Key, modifiers: Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers,
    }
}

#[test]
fn source_keyboard_moves_extends_and_contracts_without_document_history() {
    let (ctx, mut app, library, first) = setup();
    let second = app
        .editor
        .create_empty_source(library, "Second", Default::default())
        .unwrap();
    app.select_source_row(first, Modifiers::NONE).unwrap();
    focus_source(&ctx, &mut app, first);
    let before = app.editor.document.clone();
    let history = app.editor.undo_name().map(str::to_owned);
    frame(
        &ctx,
        &mut app,
        vec![navigation_key(Key::ArrowDown, Modifiers::SHIFT)],
    );
    assert_eq!(
        app.source_navigation.selected,
        HashSet::from([ResourceRow::Source(first), ResourceRow::Source(second)])
    );
    assert_eq!(
        app.focused_resource_row(&ctx),
        Some(ResourceRow::Source(second))
    );
    frame(
        &ctx,
        &mut app,
        vec![navigation_key(Key::ArrowUp, Modifiers::SHIFT)],
    );
    assert_eq!(
        app.source_navigation.selected,
        HashSet::from([ResourceRow::Source(first)])
    );
    frame(
        &ctx,
        &mut app,
        vec![navigation_key(Key::ArrowDown, Modifiers::NONE)],
    );
    assert_eq!(app.editor.definition, Some(second));
    assert!(
        app.source_navigation_focused(&ctx),
        "focus {:?} rows {:?} pending {:?}",
        ctx.memory(|m| m.focused()),
        app.source_navigation.focus,
        app.source_navigation.keyboard_focus
    );
    // Multiple delivered keys traverse from the preceding key's destination.
    frame(
        &ctx,
        &mut app,
        vec![
            navigation_key(Key::ArrowUp, Modifiers::NONE),
            navigation_key(Key::ArrowUp, Modifiers::NONE),
        ],
    );
    assert_eq!(app.editor.definition, None);
    assert_eq!(app.editor.library, Some(library));
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.undo_name(), history.as_deref());
}

#[test]
fn source_keyboard_expansion_skips_closed_children_and_preserves_source_noops() {
    let (ctx, mut app, library, first) = setup();
    let other = app.editor.create_library("Other").unwrap();
    app.editor.edit_library(library).unwrap();
    app.collapse_resource_selection();
    frame(&ctx, &mut app, vec![]);
    let id = *app
        .source_navigation
        .focus
        .iter()
        .find(|(_, row)| **row == ResourceRow::Library(library))
        .unwrap()
        .0;
    ctx.memory_mut(|m| m.request_focus(id));
    let before = app.editor.document.clone();
    frame(
        &ctx,
        &mut app,
        vec![
            navigation_key(Key::ArrowLeft, Modifiers::NONE),
            navigation_key(Key::ArrowDown, Modifiers::NONE),
        ],
    );
    assert_eq!(app.editor.library, Some(other));
    assert!(
        !app.source_navigation
            .visible
            .contains(&ResourceRow::Source(first))
    );
    frame(
        &ctx,
        &mut app,
        vec![
            navigation_key(Key::ArrowUp, Modifiers::NONE),
            navigation_key(Key::ArrowRight, Modifiers::NONE),
            navigation_key(Key::ArrowDown, Modifiers::NONE),
        ],
    );
    assert_eq!(app.editor.definition, Some(first));
    frame(
        &ctx,
        &mut app,
        vec![
            navigation_key(Key::ArrowLeft, Modifiers::NONE),
            navigation_key(Key::ArrowRight, Modifiers::NONE),
            navigation_key(Key::Enter, Modifiers::NONE),
        ],
    );
    assert_eq!(app.editor.definition, Some(first));
    assert!(app.text_edit.is_none());
    assert!(app.page_rename.is_none());
    assert_eq!(app.editor.document, before);
}

#[test]
fn source_keyboard_handoff_routes_copy_and_respects_other_input_owners() {
    let (ctx, mut app, library, first) = setup();
    let second = app
        .editor
        .create_empty_source(library, "Second", Default::default())
        .unwrap();
    app.select_source_row(first, Modifiers::NONE).unwrap();
    focus_source(&ctx, &mut app, first);
    let before = app.editor.document.clone();
    // Clipboard routing in the same event batch uses the new row, before repaint.
    frame(
        &ctx,
        &mut app,
        vec![
            navigation_key(Key::ArrowDown, Modifiers::NONE),
            egui::Event::Copy,
        ],
    );
    assert_eq!(app.editor.definition, Some(second));
    assert_eq!(app.clipboard.source.as_ref().unwrap().definition, second);
    app.error = Some("Test modal boundary".into());
    frame(
        &ctx,
        &mut app,
        vec![navigation_key(Key::ArrowUp, Modifiers::NONE)],
    );
    assert_eq!(app.editor.definition, Some(second));
    app.error = None;
    frame(&ctx, &mut app, vec![]);
    let query = app.quick_input_id.unwrap();
    ctx.memory_mut(|m| m.request_focus(query));
    frame(
        &ctx,
        &mut app,
        vec![navigation_key(Key::ArrowUp, Modifiers::NONE)],
    );
    assert_eq!(app.editor.definition, Some(second));
    assert_eq!(app.editor.document, before);
}

#[test]
fn source_keyboard_pending_focus_yields_to_duplicate_and_copy_in_same_batch() {
    let (ctx, mut app, library, first) = setup();
    let second = app
        .editor
        .create_empty_source(library, "Second", Default::default())
        .unwrap();
    app.select_source_row(first, Modifiers::NONE).unwrap();
    focus_source(&ctx, &mut app, first);
    frame(
        &ctx,
        &mut app,
        vec![
            navigation_key(Key::ArrowDown, Modifiers::NONE),
            egui::Event::AppCommand(menus::Command::Duplicate as u64),
            egui::Event::Copy,
        ],
    );
    let duplicate = app.editor.definition.unwrap();
    assert_ne!(duplicate, second);
    assert_eq!(app.clipboard.source.as_ref().unwrap().definition, duplicate);
    assert_eq!(
        app.focused_resource_row(&ctx),
        Some(ResourceRow::Source(duplicate))
    );
    // A subsequent arrow must override the duplicate's pending focus.
    frame(
        &ctx,
        &mut app,
        vec![navigation_key(Key::ArrowUp, Modifiers::NONE)],
    );
    assert_ne!(app.editor.definition, Some(duplicate));
}
