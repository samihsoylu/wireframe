// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{model::Kind, workspace::Editor};
#[test]
fn position_lock_does_not_block_markup_classification_or_change_geometry() {
    let mut e = Editor::default();
    e.add(Kind::Button, 40., 50.);
    let id = e.nodes()[0].id;
    e.document.pages[0].content.nodes[0].locked = true;
    let before = e.document.clone();
    e.set_markup(id, true).unwrap();
    assert!(e.nodes()[0].markup);
    assert!(e.nodes()[0].locked);
    assert_eq!(e.nodes()[0].origin, before.pages[0].content.nodes[0].origin);
    e.undo();
    assert_eq!(e.document, before);
}

#[test]
fn mixed_locked_selection_toggles_as_one_action_and_round_trips() {
    let mut e = Editor::default();
    e.add(Kind::Button, 40., 50.);
    e.add(Kind::Callout, 160., 50.);
    e.select_all();
    e.document.pages[0].content.nodes[0].locked = true;
    let before = e.document.clone();
    assert_eq!(e.selection_markup_target(), Some(true));
    e.toggle_selection_markup().unwrap();
    assert!(e.nodes().iter().all(|n| n.markup));
    assert_eq!(e.selection_markup_target(), Some(false));
    let marked = e.document.clone();
    assert_eq!(
        wireframe::document::decode(&marked.encode().unwrap()).unwrap(),
        marked
    );
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, marked);
    e.toggle_selection_markup().unwrap();
    assert!(e.nodes().iter().all(|n| !n.markup));
    assert!(e.nodes()[0].locked);
    for (node, old) in e.nodes().iter().zip(&before.pages[0].content.nodes) {
        assert_eq!(node.origin, old.origin);
    }
    e.undo();
    assert_eq!(e.document, marked);
}

#[test]
fn inherited_instance_markup_uses_effective_value_and_keeps_source_unchanged() {
    let mut e = Editor::default();
    e.add(Kind::Button, 40., 50.);
    let definition = e.create_component("Shared".into()).unwrap();
    let (library, source) = e.document.component_source(definition).unwrap();
    let (library_id, placement) = (library.id, source.node);
    e.edit_library(library_id).unwrap();
    e.set_markup(placement, true).unwrap();
    e.library = None;
    e.definition = None;
    e.context.clear();
    let id = e.nodes()[0].id;
    e.select(id, false);
    let before = e.document.clone();
    assert!(!e.nodes()[0].markup);
    assert_eq!(e.selection_markup_target(), Some(false));
    e.toggle_selection_markup().unwrap();
    assert_eq!(e.selection_markup_target(), Some(true));
    assert_eq!(e.document.definitions, before.definitions);
    assert_eq!(e.document.libraries, before.libraries);
    assert!(!e.document.resolve(&e.nodes()[0]).unwrap().root.markup);
    e.undo();
    assert_eq!(e.document, before);
}

#[test]
fn locked_instance_child_classification_is_local_and_empty_selection_keeps_redo() {
    let mut e = Editor::default();
    e.add(Kind::Button, 40., 50.);
    e.create_component("Shared".into()).unwrap();
    let id = e.nodes()[0].id;
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    e.select(child, false);
    e.lock(true);
    let before = e.document.clone();
    e.toggle_selection_markup().unwrap();
    assert!(e.nodes()[0].markup);
    assert!(e.nodes()[0].locked);
    assert_eq!(e.document.definitions, before.definitions);
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.selection.clear();
    assert_eq!(e.selection_markup_target(), None);
    e.toggle_selection_markup().unwrap();
    e.redo();
    assert_eq!(e.document, after);
}
