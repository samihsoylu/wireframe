// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::Value, model::Kind, workspace::Editor};

#[test]
fn nested_conversion_preserves_siblings_styles_order_and_replay() {
    let mut editor = Editor::default();
    editor.add(Kind::Label, 400., 300.);
    let label = *editor.selection.iter().next().unwrap();
    editor
        .set_control_property(label, "text", Value::Text("Café".into()))
        .unwrap();
    editor
        .set_control_property(label, "bold", Value::Bool(true))
        .unwrap();
    editor.add(Kind::Button, 700., 300.);
    editor.select_all();
    editor.group();
    let group = *editor.selection.iter().next().unwrap();
    assert!(editor.enter(group));
    editor.select(label, false);
    let bounds = editor.selected_views()[0].bounds;
    let before = editor.document.clone();
    let id = editor.transform_text_control(label).unwrap().unwrap();
    let after = editor.document.clone();
    assert_ne!(id, label);
    assert_eq!(editor.current().controls.len(), 2);
    let view = &editor.selected_views()[0];
    assert_eq!(view.kind, Kind::Paragraph);
    assert_eq!((view.bounds.x, view.bounds.y), (bounds.x, bounds.y));
    assert_eq!(view.text, "Café");
    assert!(editor.document.encode().is_ok());
    let (_, content) = editor.document.surface(editor.surface_id()).unwrap();
    let paths = content.paint_paths().unwrap();
    assert!(paths.iter().any(|p| p == &[group, id]));
    assert!(!paths.iter().any(|p| p.contains(&label)));
    let wireframe::document::NodeContent::Group { children } = &content.nodes[0].content else {
        panic!("group lost")
    };
    assert_eq!(children[0].properties.get("bold"), Some(&Value::Bool(true)));
    editor.undo();
    assert_eq!(editor.document, before);
    editor.redo();
    assert_eq!(editor.document, after);
}

#[test]
fn instance_conversion_is_rejected_without_changing_source_or_history() {
    let mut editor = Editor::default();
    editor.add(Kind::Label, 200., 150.);
    let label = *editor.selection.iter().next().unwrap();
    editor.create_component("Caption".into()).unwrap();
    let instance = *editor.selection.iter().next().unwrap();
    assert!(editor.enter(instance));
    editor.select(label, false);
    let before = editor.document.clone();
    let undo = editor.undo_name().map(str::to_owned);
    assert!(!editor.can_transform_text(label));
    assert_eq!(editor.transform_text_control(label).unwrap(), None);
    assert_eq!(editor.document, before);
    assert_eq!(editor.undo_name(), undo.as_deref());
}

#[test]
fn exhausted_identity_space_does_not_create_a_partial_conversion() {
    let mut editor = Editor::default();
    editor.add(Kind::TextInput, 200., 150.);
    let id = *editor.selection.iter().next().unwrap();
    editor.document.next_id = u64::MAX - 1;
    let before = editor.document.clone();
    let undo = editor.undo_name().map(str::to_owned);
    assert!(editor.transform_text_control(id).is_err());
    assert_eq!(editor.document, before);
    assert_eq!(editor.undo_name(), undo.as_deref());
}
