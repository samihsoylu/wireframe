// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

fn setup() -> (egui::Context, WireframeApp, Clipboard, Id) {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let app = WireframeApp::workspace(&ctx, None);
    let mut source = Editor::default();
    let library = source.create_library("Contact controls").unwrap();
    source
        .create_empty_source(
            library,
            "Contact button",
            wireframe::document::Point::new(6., 6.),
        )
        .unwrap();
    let copy = source.copy_libraries(&[library], "foreign").unwrap();
    (ctx, app, copy, library)
}
fn changed(copy: &Clipboard) -> Clipboard {
    let mut copy = copy.clone();
    copy.resources.as_mut().unwrap().libraries[0].sources[0].notes = "Updated source notes".into();
    copy
}
fn frame(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    events: Vec<egui::Event>,
) -> std::collections::HashMap<String, Rect> {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 800.))),
            events,
            ..Default::default()
        },
        |ui| {
            app.shortcuts(ui.ctx());
            app.library_import_dialog(ui.ctx());
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
                Rect::from_min_max(
                    egui::pos2(b.x0 as f32, b.y0 as f32),
                    egui::pos2(b.x1 as f32, b.y1 as f32),
                ),
            ))
        })
        .collect();
    output.textures_delta.clear();
    nodes
}
fn click(ctx: &egui::Context, app: &mut WireframeApp, label: &str) {
    frame(ctx, app, vec![]);
    let nodes = frame(ctx, app, vec![]);
    let pos = nodes[label].center();
    frame(
        ctx,
        app,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    frame(
        ctx,
        app,
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    );
}
#[test]
fn resource_paste_preserves_canvas_target_and_repeats_without_history() {
    let (_, mut app, copy, _) = setup();
    let before = app.editor.document.clone();
    let page = app.editor.page;
    app.pasted(&format!(
        "wireframe-nodes-v2:{}",
        serde_json::to_string(&copy).unwrap()
    ));
    assert!(app.library_import.is_none());
    assert!(app.error.is_none());
    assert_eq!(app.editor.page, page);
    assert!(app.editor.library.is_none());
    assert_eq!(app.editor.document.pages, before.pages);
    assert!(app.status.contains("1 added, 0 replaced, 0 reused"));
    let after = app.editor.document.clone();
    let history = app.editor.undo_name().map(str::to_owned);
    app.paste_clipboard(&copy);
    assert_eq!(app.editor.document, after);
    assert_eq!(app.editor.undo_name(), history.as_deref());
    assert!(app.status.contains("0 added, 0 replaced, 1 reused"));
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    app.editor.redo();
    assert_eq!(app.editor.document, after);
}
#[test]
fn dialog_buttons_cancel_reuse_replace_and_keep_both_atomically() {
    for label in [
        "Cancel Paste Libraries",
        "Use Existing",
        "Use New",
        "Keep Both",
    ] {
        let (ctx, mut app, copy, _) = setup();
        app.paste_clipboard(&copy);
        let before = app.editor.document.clone();
        let history = app.editor.undo_name().map(str::to_owned);
        app.paste_clipboard(&changed(&copy));
        assert!(app.library_import.is_some());
        assert_eq!(app.editor.document, before);
        assert!(!app.command_enabled(&ctx, menus::Command::NewPage));
        app.run_command(&ctx, menus::Command::NewPage);
        assert_eq!(app.editor.document, before);
        click(&ctx, &mut app, label);
        assert!(app.library_import.is_none(), "{label}");
        assert!(app.error.is_none(), "{label}: {:?}", app.error);
        if matches!(label, "Cancel Paste Libraries" | "Use Existing") {
            assert_eq!(app.editor.document, before);
            assert_eq!(app.editor.undo_name(), history.as_deref());
        } else {
            let after = app.editor.document.clone();
            assert_eq!(
                after.libraries.len(),
                before.libraries.len() + usize::from(label == "Keep Both")
            );
            assert!(
                after
                    .libraries
                    .iter()
                    .flat_map(|l| &l.sources)
                    .any(|s| s.notes == "Updated source notes")
            );
            app.editor.undo();
            assert_eq!(app.editor.document, before);
            app.editor.redo();
            assert_eq!(app.editor.document, after);
        }
    }
}
#[test]
fn stale_choices_replan_without_mutating_changed_destination() {
    let (_, mut app, copy, _) = setup();
    app.paste_clipboard(&copy);
    app.paste_clipboard(&changed(&copy));
    app.editor.document.title = "Changed while reviewing".into();
    let before = app.editor.document.clone();
    app.choose_library_import(Choice::UseNew);
    assert_eq!(app.editor.document, before);
    let review = app.library_import.as_ref().unwrap();
    assert!(review.choices.is_empty());
    assert!(review.error.as_ref().unwrap().contains("project changed"));
    assert!(
        review
            .plan
            .matches_destination(&before, &app.clipboard_origin())
    );
}
#[test]
fn escape_cancels_and_second_paste_cannot_replace_review() {
    let (ctx, mut app, copy, _) = setup();
    app.paste_clipboard(&copy);
    let changed = changed(&copy);
    app.paste_clipboard(&changed);
    let before = app.editor.document.clone();
    app.paste_clipboard(&copy);
    assert_eq!(
        serde_json::to_value(&app.library_import.as_ref().unwrap().clipboard).unwrap(),
        serde_json::to_value(&changed).unwrap()
    );
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
    assert!(app.library_import.is_none());
    assert_eq!(app.editor.document, before);
}

#[test]
fn multiple_conflicts_wait_for_all_choices_and_support_apply_remaining() {
    for all in [false, true] {
        let (ctx, mut app, _, _) = setup();
        let mut source = Editor::default();
        let mut libraries = vec![];
        for name in ["Contact", "Checkout"] {
            let library = source.create_library(name).unwrap();
            source
                .create_empty_source(library, name, wireframe::document::Point::default())
                .unwrap();
            libraries.push(library);
        }
        let copy = source.copy_libraries(&libraries, "foreign").unwrap();
        app.paste_clipboard(&copy);
        let before = app.editor.document.clone();
        let mut update = copy.clone();
        for library in &mut update.resources.as_mut().unwrap().libraries {
            library.sources[0].notes = "Updated".into();
        }
        app.paste_clipboard(&update);
        if all {
            click(&ctx, &mut app, "Apply to remaining 1 conflict");
        }
        click(&ctx, &mut app, "Keep Both");
        if !all {
            assert_eq!(app.editor.document, before);
            assert!(app.library_import.is_some());
            click(&ctx, &mut app, "Use Existing");
        }
        assert!(app.library_import.is_none());
        assert_eq!(
            app.editor.document.libraries.len(),
            before.libraries.len() + if all { 2 } else { 1 }
        );
        let after = app.editor.document.clone();
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        app.editor.redo();
        assert_eq!(app.editor.document, after);
    }
}

#[test]
fn unresolved_edit_impact_requires_explicit_choice_and_can_be_avoided() {
    use wireframe::document::{Node, NodeContent, Override, Patch, Point, Properties, Size, Value};
    for preserve in [false, true] {
        let (ctx, mut app, mut copy, _) = setup();
        copy.resources.as_mut().unwrap().definitions[0]
            .content
            .nodes = vec![Node::control(
            100,
            "button",
            Point::default(),
            Size::default(),
        )];
        app.paste_clipboard(&copy);
        let definition = app.editor.document.libraries[0].sources[0].definition;
        let id = app.editor.document.allocate_id().unwrap();
        let mut instance = Node::group(id, Point::default(), vec![]);
        instance.content = NodeContent::Instance {
            definition,
            overrides: vec![Override {
                path: vec![100],
                patch: Patch {
                    properties: Properties::from([(
                        "text".into(),
                        Value::Text("Local edit".into()),
                    )]),
                    ..Patch::default()
                },
            }],
        };
        app.editor.document.pages[0].content.nodes.push(instance);
        let before = app.editor.document.clone();
        copy.resources.as_mut().unwrap().definitions[0]
            .content
            .nodes
            .clear();
        app.paste_clipboard(&copy);
        click(&ctx, &mut app, "Use New");
        assert_eq!(app.editor.document, before);
        assert!(
            app.library_import
                .as_ref()
                .unwrap()
                .prepared
                .as_ref()
                .unwrap()
                .newly_orphaned_overrides()
                > 0
        );
        let nodes = frame(&ctx, &mut app, vec![]);
        assert!(nodes.contains_key("Import with changes"));
        if preserve {
            click(&ctx, &mut app, "Review choices");
            click(&ctx, &mut app, "Keep Both");
            assert!(
                app.editor
                    .document
                    .definitions
                    .iter()
                    .find(|d| d.id == definition)
                    .unwrap()
                    .content
                    .nodes
                    .iter()
                    .any(|n| n.id == 100)
            );
        } else {
            click(&ctx, &mut app, "Import with changes");
            assert!(
                app.editor
                    .document
                    .definitions
                    .iter()
                    .find(|d| d.id == definition)
                    .unwrap()
                    .content
                    .nodes
                    .is_empty()
            );
        }
        assert!(app.library_import.is_none());
        assert_eq!(app.editor.document.pages, before.pages);
        let after = app.editor.document.clone();
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        app.editor.redo();
        assert_eq!(app.editor.document, after);
    }
}

#[test]
fn malformed_resource_payload_does_not_enter_review_or_change_history() {
    let (_, mut app, mut copy, _) = setup();
    let before = app.editor.document.clone();
    copy.resources.as_mut().unwrap().requested.push(99999);
    app.paste_clipboard(&copy);
    assert!(app.error.is_some());
    assert!(app.library_import.is_none());
    assert_eq!(app.editor.document, before);
    assert!(app.editor.undo_name().is_none());
}

fn scene_text(scene: &wireframe::render::Scene) -> String {
    scene
        .items
        .iter()
        .filter_map(|item| match &item.drawing {
            wireframe::render::Drawing::Glyph(glyph) => Some(glyph.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}
#[test]
fn conflict_previews_render_distinct_contents_and_refresh_after_destination_changes() {
    use wireframe::document::{Node, Point, Size, Value};
    let (ctx, mut app, mut copy, _) = setup();
    let mut button = Node::control(100, "button", Point::new(-70., 20.), Size::fixed(180., 40.));
    button
        .properties
        .insert("text".into(), Value::Text("Incoming action".into()));
    copy.resources.as_mut().unwrap().definitions[0]
        .content
        .nodes = vec![button];
    app.paste_clipboard(&copy);
    app.editor.document.definitions[0].content.nodes[0]
        .properties
        .insert("text".into(), Value::Text("Existing action".into()));
    let before = app.editor.document.clone();
    app.paste_clipboard(&copy);
    frame(&ctx, &mut app, vec![]);
    let review = app.library_import.as_ref().unwrap();
    let pair = &review.preview.as_ref().unwrap().1;
    assert!(scene_text(pair.incoming.as_ref().unwrap()).contains("Incoming action"));
    assert!(scene_text(pair.existing.as_ref().unwrap()).contains("Existing action"));
    assert_eq!(
        pair.incoming.as_ref().unwrap().font,
        app.editor.document.font
    );
    assert_eq!(app.editor.document, before);
    let incoming_project = &review
        .preview_editor
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .document;
    assert_eq!(incoming_project.settings, before.settings);
    incoming_project.validate().unwrap();
    app.editor.document.definitions[0].content.nodes[0]
        .properties
        .insert("text".into(), Value::Text("Revised existing action".into()));
    frame(&ctx, &mut app, vec![]);
    assert!(app.library_import.as_ref().unwrap().error.is_some());
    click(&ctx, &mut app, "Review choices");
    frame(&ctx, &mut app, vec![]);
    let pair = &app
        .library_import
        .as_ref()
        .unwrap()
        .preview
        .as_ref()
        .unwrap()
        .1;
    assert!(scene_text(pair.existing.as_ref().unwrap()).contains("Revised existing action"));
    assert!(scene_text(pair.incoming.as_ref().unwrap()).contains("Incoming action"));
}

#[test]
fn paired_preview_cards_and_choices_fit_narrow_and_wide_windows() {
    for size in [Vec2::new(640., 480.), Vec2::new(1100., 800.)] {
        let (ctx, mut app, copy, _) = setup();
        app.paste_clipboard(&copy);
        app.paste_clipboard(&changed(&copy));
        let mut rects = std::collections::HashMap::new();
        for _ in 0..3 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                    ..Default::default()
                },
                |ui| app.library_import_dialog(ui.ctx()),
            );
            rects = output
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .filter_map(|(_, node)| {
                    let b = node.bounds()?;
                    Some((
                        node.label()?.to_owned(),
                        Rect::from_min_max(
                            egui::pos2(b.x0 as f32, b.y0 as f32),
                            egui::pos2(b.x1 as f32, b.y1 as f32),
                        ),
                    ))
                })
                .collect();
            output.textures_delta.clear();
        }
        for label in [
            "Existing library preview",
            "New library preview",
            "Use New",
            "Use Existing",
            "Keep Both",
            "Cancel Paste Libraries",
        ] {
            let rect = rects[label];
            assert!(
                Rect::from_min_size(Pos2::ZERO, size).contains_rect(rect),
                "{label}: {rect:?} outside {size:?}"
            );
        }
        assert!(rects["Existing library preview"].max.x < rects["New library preview"].min.x);
        let pair = &app
            .library_import
            .as_ref()
            .unwrap()
            .preview
            .as_ref()
            .unwrap()
            .1;
        assert!(pair.existing.is_ok());
        assert!(pair.incoming.is_ok());
    }
}

#[test]
fn preview_graph_is_validated_and_does_not_rewrite_original_clipboard_links() {
    use wireframe::document::{Link, Node, Point, Size, Value};
    let (_, _, mut copy, library) = setup();
    let mut label = Node::control(100, "label", Point::default(), Size::default());
    label
        .properties
        .insert("link".into(), Value::Link(Link::Page(999)));
    copy.resources.as_mut().unwrap().definitions[0]
        .content
        .nodes = vec![label];
    let before = serde_json::to_value(&copy).unwrap();
    let project = copy.library_resources().unwrap().preview_project().unwrap();
    project.validate().unwrap();
    assert_eq!(project.libraries[0].id, library);
    assert_eq!(serde_json::to_value(&copy).unwrap(), before);
    assert_ne!(
        project.definitions[0].content.nodes[0]
            .properties
            .get("link"),
        Some(&Value::Link(Link::Page(999)))
    );
    copy.resources.as_mut().unwrap().requested.push(999);
    assert!(copy.resources.as_ref().unwrap().preview_project().is_err());
}

#[test]
fn genuinely_empty_library_preview_is_distinct_from_existing_artwork() {
    use wireframe::document::{Node, Point, Size};
    let (ctx, mut app, _, _) = setup();
    let mut source = Editor::default();
    let library = source.create_library("Empty incoming library").unwrap();
    let copy = source.copy_libraries(&[library], "foreign").unwrap();
    app.paste_clipboard(&copy);
    let id = app.editor.document.allocate_id().unwrap();
    app.editor.document.libraries[0]
        .content
        .nodes
        .push(Node::control(
            id,
            "button",
            Point::default(),
            Size::default(),
        ));
    app.paste_clipboard(&copy);
    frame(&ctx, &mut app, vec![]);
    let pair = &app
        .library_import
        .as_ref()
        .unwrap()
        .preview
        .as_ref()
        .unwrap()
        .1;
    assert!(pair.incoming.as_ref().unwrap().items.is_empty());
    assert!(!pair.existing.as_ref().unwrap().items.is_empty());
}
