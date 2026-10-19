// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::collections::HashSet;
use wireframe::{
    document::*,
    model::{Bounds, Kind},
    workspace::Editor,
};

fn fixture() -> Editor {
    let mut p = Project::default();
    let plain = Node::control(2, "rectangle", Point::new(20., 30.), Size::fixed(80., 40.));
    let mut marked = Node::control(
        3,
        "rectangle",
        Point::new(1000., 1000.),
        Size::fixed(80., 40.),
    );
    marked.markup = true;
    p.pages[0].content.nodes = vec![plain, marked];
    p.next_id = 10;
    Editor::new(p)
}

#[test]
fn hidden_artwork_cannot_be_selected_hit_or_snapped_and_toggle_is_not_an_edit() {
    let mut e = fixture();
    e.mark_saved();
    e.select_all();
    let document = e.document.clone();
    let full = e.rendered_scene(1).unwrap().svg();
    let b = Bounds::new(1000., 1000., 80., 40.);
    assert!(
        !e.alignment_targets_with_selection(4., true)
            .unwrap()
            .matches(b)
            .is_empty()
    );
    e.set_markup_visible(false);
    assert_eq!(e.selection, HashSet::from([2]));
    assert!(e.hit(1020., 1020.).is_none());
    assert!(
        e.alignment_targets_with_selection(4., true)
            .unwrap()
            .matches(b)
            .is_empty()
    );
    e.select(3, false);
    assert!(!e.selection.contains(&3));
    e.select_all();
    assert_eq!(e.selection, HashSet::from([2]));
    e.select_area(Bounds::new(0., 0., 2000., 2000.), &HashSet::new());
    assert_eq!(e.selection, HashSet::from([2]));
    assert_ne!(e.editing_scene().unwrap().svg(), full);
    assert_eq!(e.rendered_scene(1).unwrap().svg(), full);
    assert_eq!(e.document, document);
    assert!(!e.dirty());
    assert_eq!(e.undo_name(), None);
    e.set_markup_visible(true);
    assert_eq!(e.editing_scene().unwrap().svg(), full);
    assert_eq!(e.hit(1020., 1020.).unwrap().id, 3);
}

#[test]
fn entered_group_cache_and_inherited_visibility_do_not_restore_hidden_selection() {
    let mut e = fixture();
    let children = e.document.pages[0].content.nodes.clone();
    e.document.pages[0].content.nodes = vec![Node::group(4, Point::new(0., 0.), children)];
    assert!(e.enter(4));
    let full = e.editing_scene().unwrap().svg();
    e.set_markup_visible(false);
    let hidden = e.editing_scene().unwrap().svg();
    assert_ne!(full, hidden);
    e.set_markup_visible(true);
    assert_eq!(e.editing_scene().unwrap().svg(), full);
    e.document.pages[0].content.nodes[0].markup = true;
    e.set_markup_visible(false);
    assert!(e.editing_scene().unwrap().items.is_empty());
    assert!(e.current().controls.is_empty());
    e.select_all();
    assert!(e.selection.is_empty());
    assert!(e.leave());
    assert!(e.selection.is_empty());
}

#[test]
fn inherited_component_markup_and_local_clear_keep_source_intact() {
    let mut e = Editor::default();
    e.add(Kind::Button, 40., 50.);
    let definition = e.create_component("Shared".into()).unwrap();
    let (lib, source) = e.document.component_source(definition).unwrap();
    let (lib, placement) = (lib.id, source.node);
    e.edit_library(lib).unwrap();
    e.set_markup(placement, true).unwrap();
    e.library = None;
    e.definition = None;
    e.context.clear();
    let id = e.nodes()[0].id;
    let libraries = e.document.libraries.clone();
    e.set_markup_visible(false);
    assert!(e.editing_scene().unwrap().items.is_empty());
    e.set_markup(id, false).unwrap();
    assert!(!e.markup_visible());
    assert!(!e.editing_scene().unwrap().items.is_empty());
    assert_eq!(e.document.libraries, libraries);
    e.undo();
    assert!(!e.markup_visible());
    assert!(e.editing_scene().unwrap().items.is_empty());
}

#[test]
fn insertion_paste_and_history_reveal_new_markup_but_classification_does_not() {
    let mut e = fixture();
    e.select(3, false);
    let copy = e.copy();
    e.set_markup_visible(false);
    e.paste(&copy, 20.);
    assert!(e.markup_visible());
    let pasted = e.document.clone();
    let selected = e.selection.clone();
    e.undo();
    e.set_markup_visible(false);
    e.redo();
    assert!(e.markup_visible());
    assert_eq!(e.document, pasted);
    assert_eq!(e.selection, selected);
    e.delete();
    e.set_markup_visible(false);
    e.undo();
    assert!(e.markup_visible());
    assert_eq!(e.document, pasted);
    assert_eq!(e.selection, selected);
    e.set_markup_visible(false);
    e.set_markup(2, true).unwrap();
    assert!(!e.markup_visible());
    e.undo();
    assert!(!e.markup_visible());
    e.add(Kind::Button, 40., 50.);
    assert!(!e.markup_visible());
    e.add(Kind::Callout, 40., 50.);
    assert!(e.markup_visible());
}

#[test]
fn hidden_view_keeps_redo_and_text_preview_does_not_reintroduce_markup() {
    let mut e = fixture();
    e.add(Kind::Button, 200., 50.);
    let id = e.nodes().last().unwrap().id;
    e.undo();
    let redo = e.redo_name().unwrap().to_owned();
    e.set_markup_visible(false);
    assert_eq!(e.redo_name(), Some(redo.as_str()));
    e.redo();
    assert!(!e.markup_visible());
    let hidden = e.text_preview_scene(id, "New caption").unwrap();
    assert!(hidden.bounds.x + hidden.bounds.w < 1000.);
    e.set_markup_visible(true);
    let full = e.text_preview_scene(id, "New caption").unwrap();
    assert!(full.bounds.x + full.bounds.w >= 1080.);
}

#[test]
fn regrouping_existing_hidden_descendants_does_not_count_as_inserting_markup() {
    let mut e = fixture();
    let children = std::mem::take(&mut e.document.pages[0].content.nodes);
    e.document.pages[0].content.nodes = vec![Node::group(4, Point::default(), children)];
    e.add(Kind::Button, 200., 50.);
    e.set_markup_visible(false);
    e.select_all();
    e.group();
    assert!(!e.markup_visible());
    e.undo();
    assert!(!e.markup_visible());
    e.redo();
    assert!(!e.markup_visible());
    e.ungroup();
    assert!(!e.markup_visible());
}

#[test]
fn new_shared_instance_reveals_marked_descendants_despite_reused_definition_ids() {
    let mut e = fixture();
    e.select_all();
    e.group();
    e.create_component("Shared annotations".into()).unwrap();
    let instance = e.nodes()[0].id;
    e.select(instance, false);
    let copy = e.copy();
    e.set_markup_visible(false);
    e.paste(&copy, 20.);
    assert!(e.markup_visible());
    assert_eq!(e.nodes().len(), 2);
    assert_eq!(e.document.definitions.len(), 1);
    e.undo();
    e.set_markup_visible(false);
    e.redo();
    assert!(e.markup_visible());
}
