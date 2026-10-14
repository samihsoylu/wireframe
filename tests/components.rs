// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::collections::HashSet;
use wireframe::{document::*, model::Kind, workspace::Editor};
fn texts(e: &Editor) -> Vec<String> {
    e.layers(1)
        .unwrap()
        .iter()
        .map(|l| l.control.text.clone())
        .collect()
}
#[test]
fn components_create_reuse_edit_detach_and_reopen_without_losing_overrides() {
    let mut e = Editor::default();
    e.add(Kind::Button, 50., 60.);
    let first = e.nodes()[0].id;
    e.set_control_property(first, "text", Value::Text("Source".into()))
        .unwrap();
    let before = e.rendered_scene(1).unwrap().svg();
    let definition = e.create_component("Reusable button".into()).unwrap();
    assert_eq!(before, e.rendered_scene(1).unwrap().svg());
    assert_eq!(e.nodes()[0].id, first);
    let child = e.document.definitions[0].content.nodes[0].id;
    let second = e.insert_component(definition, 300., 60.).unwrap();
    assert_eq!(texts(&e), ["Source", "Source"]);
    if let NodeContent::Instance { overrides, .. } =
        &mut e.document.pages[0].content.nodes[1].content
    {
        overrides.push(Override {
            path: vec![child],
            patch: Patch {
                properties: Properties::from([("text".into(), Value::Text("Local".into()))]),
                ..Patch::default()
            },
        });
    }
    e.edit_definition(definition).unwrap();
    assert_eq!(e.surface_id(), definition);
    assert_eq!(e.current().controls[0].bounds.x, 0.);
    e.begin("Edit component text");
    e.set_control_property(child, "text", Value::Text("Updated".into()))
        .unwrap();
    e.commit();
    assert_eq!(texts(&e), ["Updated", "Local"]);
    e.switch_page(1);
    let before_detach = e.rendered_scene(1).unwrap().svg();
    e.detach_component(second).unwrap();
    assert_eq!(before_detach, e.rendered_scene(1).unwrap().svg());
    e.undo();
    assert!(e.instance_definition(second).is_some());
    e.redo();
    assert!(e.is_group(second));
    e.edit_definition(definition).unwrap();
    e.begin("Edit again");
    e.set_control_property(child, "text", Value::Text("Latest".into()))
        .unwrap();
    e.commit();
    assert_eq!(texts(&e), ["Latest", "Local"]);
    e.undo();
    assert_eq!(texts(&e), ["Updated", "Local"]);
    e.redo();
    let bytes = e.document.encode().unwrap();
    let reopened = Editor::new(decode(&bytes).unwrap());
    assert_eq!(texts(&reopened), ["Latest", "Local"]);
    assert_eq!(
        e.rendered_scene(1).unwrap().svg(),
        reopened.rendered_scene(1).unwrap().svg()
    );
}
#[test]
fn source_canvas_supports_groups_moves_clipboard_and_transactional_cycle_rejection() {
    let mut e = Editor::default();
    e.add(Kind::Button, 50., 60.);
    e.add(Kind::TextInput, 150., 110.);
    e.select_all();
    e.group();
    let original = e.rendered_scene(1).unwrap().svg();
    let definition = e.create_component("Form".into()).unwrap();
    assert_eq!(original, e.rendered_scene(1).unwrap().svg());
    e.edit_definition(definition).unwrap();
    e.select_all();
    e.group();
    assert!(e.error.is_none(), "{:?}", e.error);
    assert!(e.document.definitions[0].content.paint_order.is_empty());
    let group = e.nodes()[0].id;
    assert!(e.enter(group));
    let first = e.nodes()[0].id;
    e.select(first, false);
    e.move_by(10., 5.);
    assert_eq!(e.layers(1).unwrap()[0].control.bounds.x, 60.);
    let copy = e.copy();
    e.paste(&copy, 20.);
    assert_eq!(e.nodes().len(), 3);
    let before = e.document.clone();
    let selection = e.selection.clone();
    let undo = e.undo_name().map(str::to_owned);
    assert!(
        e.insert_component(definition, 0., 0.)
            .unwrap_err()
            .to_lowercase()
            .contains("cyclic")
    );
    assert_eq!(e.document, before);
    assert_eq!(e.selection, selection);
    assert_eq!(e.undo_name(), undo.as_deref());
    assert!(e.leave());
    assert!(e.leave());
    assert_eq!(e.definition, None);
    e.undo();
    assert_eq!(e.definition, Some(definition));
    assert_eq!(e.nodes().len(), 2);
    e.document.validate().unwrap();
}
#[test]
fn component_creation_rejects_locked_and_interleaved_content_without_changes() {
    let mut e = Editor::default();
    e.add(Kind::Button, 0., 0.);
    let first = e.nodes()[0].id;
    e.lock(true);
    let before = e.document.clone();
    assert!(e.create_component("Locked".into()).is_err());
    assert_eq!(e.document, before);
    e.lock(false);
    e.add(Kind::TextInput, 0., 30.);
    e.add(Kind::Button, 0., 60.);
    let third = e.nodes()[2].id;
    e.selection = HashSet::from([first, third]);
    e.group();
    let before = e.document.clone();
    assert!(e.create_component("Interleaved".into()).is_err());
    assert_eq!(e.document, before);
    e.undo();
    e.select(first, false);
    let def = e.create_component("Button".into()).unwrap();
    e.lock(true);
    let before = e.document.clone();
    assert!(e.detach_component(first).is_err());
    assert_eq!(e.document, before);
    e.rename_definition(def, "Named button".into()).unwrap();
    assert_eq!(e.document.definitions[0].name, "Named button");
    e.undo();
    assert_eq!(e.document.definitions[0].name, "Button");
}
