// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, model::Kind, workspace::Editor};
fn text(e: &Editor) -> Vec<String> {
    e.layers(1)
        .unwrap()
        .iter()
        .map(|l| l.control.text.clone())
        .collect()
}
fn patches(e: &Editor) -> &Vec<Override> {
    let NodeContent::Instance { overrides, .. } = &e.document.pages[0].content.nodes[0].content
    else {
        panic!()
    };
    overrides
}
fn set(e: &mut Editor, id: Id, key: &str, v: Value) {
    e.begin("Edit local property");
    e.set_control_property(id, key, v).unwrap();
    e.commit();
}
fn fixture() -> (Editor, Id, Id, Id, Id) {
    let mut e = Editor::default();
    e.add(Kind::TextInput, 50., 50.);
    let first = e.nodes()[0].id;
    set(&mut e, first, "text", Value::Text("Source".into()));
    let inner = e.create_component("Inner".into()).unwrap();
    let child = e.document.definitions[0].content.nodes[0].id;
    e.select(first, false);
    e.add(Kind::Button, 250., 50.);
    e.select_all();
    e.group();
    let outer_instance = e.nodes()[0].id;
    let outer = e.create_component("Outer".into()).unwrap();
    e.insert_component(outer, 50., 150.).unwrap();
    (e, outer_instance, first, child, inner)
}
#[test]
fn deep_local_edits_reset_to_inherited_values_and_survive_history_and_storage() {
    let (mut e, outer, nested, child, inner) = fixture();
    let sources = e.document.definitions.clone();
    assert!(e.enter(outer));
    assert!(e.enter(nested));
    assert!(e.editing_instance());
    assert_eq!(e.current().controls[0].text, "Source");
    set(&mut e, child, "text", Value::Text("Local".into()));
    set(&mut e, child, "bold", Value::Bool(true));
    assert_eq!(e.document.definitions, sources);
    assert_eq!(patches(&e)[0].path, [nested, child]);
    assert_eq!(text(&e), ["Local", "Button", "Source", "Button"]);
    assert!(e.explicit_control_properties(child).contains_key("text"));
    e.begin("Reset text");
    e.reset_control_property(child, "text").unwrap();
    e.commit();
    assert_eq!(text(&e)[0], "Source");
    assert!(!e.explicit_control_properties(child).contains_key("text"));
    assert!(e.explicit_control_properties(child).contains_key("bold"));
    e.undo();
    assert_eq!(text(&e)[0], "Local");
    e.redo();
    assert_eq!(text(&e)[0], "Source");
    e.edit_definition(inner).unwrap();
    set(&mut e, child, "text", Value::Text("New source".into()));
    assert_eq!(text(&e), ["New source", "Button", "New source", "Button"]);
    e.switch_page(1);
    assert!(e.enter(outer));
    assert!(e.enter(nested));
    set(&mut e, child, "text", Value::Text("Local again".into()));
    let before = e.rendered_scene(1).unwrap().svg();
    let mut reopened = Editor::new(decode(&e.document.encode().unwrap()).unwrap());
    assert_eq!(before, reopened.rendered_scene(1).unwrap().svg());
    assert!(reopened.enter(outer));
    assert!(reopened.enter(nested));
    assert_eq!(reopened.current().controls[0].text, "Local again");
    reopened.reset_local_changes(child).unwrap();
    assert!(patches(&reopened).is_empty());
    assert_eq!(text(&reopened)[0], "New source");
    reopened.undo();
    assert_eq!(text(&reopened)[0], "Local again");
    // Explicitly choosing an inherited value removes the local value too.
    set(
        &mut reopened,
        child,
        "text",
        Value::Text("New source".into()),
    );
    assert!(
        !reopened
            .explicit_control_properties(child)
            .contains_key("text")
    );
}
#[test]
fn inherited_nondefault_values_can_be_overridden_with_catalog_defaults() {
    let (mut e, outer, nested, child, inner) = fixture();
    e.edit_definition(inner).unwrap();
    set(&mut e, child, "bold", Value::Bool(true));
    let definitions = e.document.definitions.clone();
    e.switch_page(1);
    e.enter(outer);
    e.enter(nested);
    set(&mut e, child, "bold", Value::Bool(false));
    assert_eq!(
        e.control_properties(child).unwrap().1["bold"],
        Value::Bool(false)
    );
    assert_eq!(patches(&e)[0].patch.properties["bold"], Value::Bool(false));
    assert_eq!(e.document.definitions, definitions);
    e.begin("Reset bold");
    e.reset_control_property(child, "bold").unwrap();
    e.commit();
    assert_eq!(
        e.control_properties(child).unwrap().1["bold"],
        Value::Bool(true)
    );
    assert!(patches(&e).is_empty());
    // A definition can itself contain an overridden instance. Outer reset inherits it.
    let outer_definition = e.instance_definition(outer);
    assert!(outer_definition.is_none());
    e.leave();
    e.leave();
    let outer_definition = e.instance_definition(outer).unwrap();
    e.edit_definition(outer_definition).unwrap();
    e.enter(nested);
    set(&mut e, child, "text", Value::Text("Middle".into()));
    e.switch_page(1);
    e.enter(outer);
    e.enter(nested);
    set(&mut e, child, "text", Value::Text("Outer".into()));
    e.edit_definition(inner).unwrap();
    set(&mut e, child, "text", Value::Text("Root changed".into()));
    assert_eq!(text(&e), ["Outer", "Button", "Middle", "Button"]);
    e.switch_page(1);
    e.enter(outer);
    e.enter(nested);
    e.begin("Reset text");
    e.reset_control_property(child, "text").unwrap();
    e.commit();
    assert_eq!(text(&e), ["Middle", "Button", "Middle", "Button"]);
}
#[test]
fn local_geometry_copy_reset_and_guards_preserve_source_structure() {
    let (mut e, outer, nested, child, inner) = fixture();
    let definitions = e.document.definitions.clone();
    e.enter(outer);
    e.enter(nested);
    e.select(child, false);
    let old = e.selected_views()[0].bounds;
    e.move_by(15., 8.);
    assert_eq!(e.selected_views()[0].bounds.x, old.x + 15.);
    e.begin("Resize local input");
    let mut c = e.selected_views()[0].clone();
    c.bounds.w = 180.;
    e.update_control(c);
    e.commit();
    assert_eq!(e.selected_views()[0].bounds.w, 180.);
    assert!(patches(&e)[0].patch.size.is_some());
    e.begin("Automatic local size");
    e.auto_size_control(child).unwrap();
    e.commit();
    assert_eq!(e.selected_views()[0].bounds.w, old.w);
    assert!(patches(&e)[0].patch.size.is_none());
    let copy = e.copy();
    assert_eq!(copy.nodes[0].origin.x, old.x + 15.);
    let before = e.document.clone();
    e.delete();
    e.group();
    e.ungroup();
    e.reorder(true);
    e.add(Kind::Button, 0., 0.);
    e.paste(&copy, 20.);
    assert!(e.create_component("No".into()).is_err());
    assert!(e.insert_component(inner, 0., 0.).is_err());
    assert!(e.detach_component(child).is_err());
    assert_eq!(e.document, before);
    assert_eq!(e.document.definitions, definitions);
    e.reset_local_changes(child).unwrap();
    assert_eq!(e.selected_views()[0].bounds, old);
    e.undo();
    assert_eq!(e.selected_views()[0].bounds.x, old.x + 15.);
    e.leave();
    e.leave();
    e.reset_instance(outer).unwrap();
    assert!(patches(&e).is_empty());
    e.undo();
    assert!(!patches(&e).is_empty());
    e.select(outer, false);
    e.lock(true);
    assert!(e.enter(outer));
    e.leave();
    assert!(e.reset_instance(outer).is_err());
    e.lock(false);
    e.edit_definition(inner).unwrap();
    e.select(child, false);
    e.lock(true);
    e.switch_page(1);
    e.enter(outer);
    e.enter(nested);
    let sources = e.document.definitions.clone();
    assert!(!e.own_locked(child));
    set(&mut e, child, "text", Value::Text("Editable".into()));
    e.reset_local_changes(child).unwrap();
    assert_eq!(e.document.definitions, sources);
}

#[test]
fn projected_text_and_choice_edits_remain_valid_local_properties() {
    let mut e = Editor::default();
    e.add(Kind::Checkbox, 50., 50.);
    let root = e.nodes()[0].id;
    set(&mut e, root, "state", Value::Choice("selected".into()));
    e.create_component("Choice".into()).unwrap();
    e.enter(root);
    let child = e.nodes()[0].id;
    e.begin("Edit projected choice");
    let mut c = e.current().controls[0].clone();
    c.checked = false;
    c.text = "Local choice".into();
    e.update_control(c);
    e.commit();
    assert_eq!(
        e.control_properties(child).unwrap().1["state"],
        Value::Choice("normal".into())
    );
    assert_eq!(e.current().controls[0].text, "Local choice");
    e.rendered_scene(1).unwrap();
    e.begin("Reset choice state");
    e.reset_control_property(child, "state").unwrap();
    e.commit();
    assert!(e.current().controls[0].checked);
    assert_eq!(e.current().controls[0].text, "Local choice");
    e.undo();
    assert!(!e.current().controls[0].checked);
}
