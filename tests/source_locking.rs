// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::collections::HashSet;
use wireframe::{
    document::*,
    model::{Bounds, Kind},
    workspace::Editor,
};

fn fixture() -> (Editor, Id, Id, Id) {
    let mut e = Editor::default();
    e.add(Kind::Button, 0., 0.);
    e.add(Kind::TextInput, 200., 50.);
    e.select_all();
    e.group();
    let definition = e.create_component("Contact controls".into()).unwrap();
    let (library, source) = e.document.component_source(definition).unwrap();
    let (library, owner) = (library.id, source.node);
    e.edit_library(library).unwrap();
    e.select(owner, false);
    e.lock(true);
    e.edit_library_source(definition).unwrap();
    (e, definition, library, owner)
}

#[test]
fn owner_lock_preserves_selection_but_protects_geometry_structure_and_history() {
    let (mut e, _, _, _) = fixture();
    e.select_all();
    assert_eq!(e.selection.len(), 2);
    assert!(e.selected_views().iter().all(|c| c.locked));
    assert!(
        e.nodes()
            .iter()
            .all(|n| !e.own_locked(n.id) && e.position_locked(n.id))
    );
    e.select_area(Bounds::new(-10., -10., 1000., 1000.), &HashSet::new());
    assert_eq!(e.selection.len(), 2);
    let before = e.document.clone();
    let undo = e.undo_name().map(str::to_owned);
    let mut control = e.selected_views()[0].clone();
    control.bounds.x += 40.;
    control.bounds.w += 100.;
    e.edit("Manual geometry", |e| e.update_control(control));
    e.move_by(1., 1.);
    e.align(0, 0);
    e.delete();
    e.group();
    e.ungroup();
    let child = e.nodes()[0].id;
    e.select(child, false);
    assert!(e.create_component("Blocked conversion".into()).is_err());
    e.select_all();
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), undo.as_deref());
    assert_eq!(e.selection.len(), 2);
}

#[test]
fn owner_lock_allows_text_measurement_insertion_and_own_lock_selection() {
    let (mut e, _, _, _) = fixture();
    let field = e.nodes()[1].id;
    e.select(field, false);
    assert!(e.prepare_text_edit(field).unwrap().is_some());
    let origin = e.nodes()[1].origin;
    let before = e.document.clone();
    e.edit("Text", |e| {
        let mut c = e.selected_views()[0].clone();
        c.text = "Contact details".into();
        e.update_control(c);
    });
    assert_eq!(e.selected_views()[0].text, "Contact details");
    assert_eq!(e.nodes()[1].origin, origin);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    e.begin("Auto size");
    e.auto_size_control(field).unwrap();
    e.commit();
    assert_eq!(e.nodes()[1].origin, origin);
    let count = e.nodes().len();
    e.add(Kind::Label, 44., 44.);
    assert_eq!(e.nodes().len(), count + 1);
    let added = *e.selection.iter().next().unwrap();
    assert!(e.position_locked(added));
    assert!(!e.own_locked(added));
    e.lock(true);
    e.select_all();
    assert!(!e.selection.contains(&added));
    assert_eq!(e.selection.len(), count);
    e.select_area(Bounds::new(-10., -10., 1000., 1000.), &HashSet::new());
    assert!(!e.selection.contains(&added));
    assert_eq!(e.selection.len(), count);
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
}

#[test]
fn owner_unlock_history_and_ordinary_instance_are_independent() {
    let (mut e, definition, library, owner) = fixture();
    let child = e.nodes()[0].id;
    let page = e.page;
    let instance = e.page_data().content.nodes[0].id;
    e.switch_page(page);
    assert!(!e.position_locked(instance));
    assert!(e.enter(instance));
    assert!(!e.position_locked(child));
    e.select(child, false);
    let canonical = e.document.definitions.clone();
    let x = e.selected_views()[0].bounds.x;
    e.move_by(1., 0.);
    assert_eq!(e.selected_views()[0].bounds.x, x + 1.);
    assert_eq!(e.document.definitions, canonical);
    e.edit_library(library).unwrap();
    e.select(owner, false);
    e.lock(false);
    e.edit_library_source(definition).unwrap();
    assert!(!e.position_locked(child));
    e.select(child, false);
    e.move_by(1., 0.);
    assert_eq!(e.nodes()[0].origin.x, 1.);
    e.undo();
    assert_eq!(e.nodes()[0].origin.x, 0.);
    e.undo();
    e.edit_library_source(definition).unwrap();
    assert!(e.position_locked(child));
}

#[test]
fn owner_crop_does_not_clip_focused_artwork_and_lock_reaches_nested_children() {
    let (mut e, definition, library, owner) = fixture();
    // Assemble the nested source before entering its already-locked owner.
    let children = std::mem::take(&mut e.document.definitions[0].content.nodes);
    let child = children[0].id;
    let group = e.document.allocate_id().unwrap();
    e.document.definitions[0].content.nodes = vec![Node::group(group, Point::default(), children)];
    e.document.definitions[0].content.paint_order.clear();
    e.document
        .libraries
        .iter_mut()
        .find(|l| l.id == library)
        .unwrap()
        .content
        .nodes
        .iter_mut()
        .find(|n| n.id == owner)
        .unwrap()
        .crop = Some(Crop {
        width: 0.5,
        height: 0.5,
        ..Crop::ALL
    });
    let full = e.current().controls[0].bounds;
    let library_bounds = e.layout_page(library).unwrap().nodes[0].bounds;
    assert!(full.w > library_bounds.width);
    let svg = e.rendered_scene(definition).unwrap().svg();
    assert!(e.enter(group));
    assert!(e.position_locked(child));
    e.select_all();
    assert_eq!(e.selection.len(), 2);
    let before = e.document.clone();
    e.move_by(1., 0.);
    e.delete();
    assert_eq!(e.document, before);
    e.leave();
    assert_eq!(e.rendered_scene(definition).unwrap().svg(), svg);
}

#[test]
fn locked_source_allows_ten_unit_child_duplicate_and_layering_with_exact_undo() {
    let (mut e, definition, _, _) = fixture();
    let child = e.nodes()[0].id;
    e.select(child, false);
    let before = e.document.clone();
    let original = e.nodes()[0].clone();
    e.duplicate();
    assert!(e.error.is_none(), "{:?}", e.error);
    assert_eq!(e.definition, Some(definition));
    assert_eq!(e.nodes().len(), 3);
    let copied = e
        .nodes()
        .iter()
        .find(|n| e.selection.contains(&n.id))
        .unwrap()
        .clone();
    assert_ne!(copied.id, child);
    assert!(!copied.locked);
    assert_eq!(
        copied.origin,
        original.origin.translated(Point::new(10., 10.))
    );
    assert_eq!(e.nodes()[0], original);
    assert_eq!(e.document.pages, before.pages);
    assert_eq!(e.document.libraries, before.libraries);
    let duplicated = e.document.clone();
    e.select(child, false);
    e.reorder(true);
    assert_eq!(e.nodes().last().unwrap().id, child);
    assert_eq!(e.document.pages, before.pages);
    let reordered = e.document.clone();
    e.undo();
    assert_eq!(e.document, duplicated);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    e.redo();
    assert_eq!(e.document, reordered);
}
