// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::collections::HashSet;
use wireframe::{document::*, model::Kind, workspace::Editor};

#[test]
fn duplicate_preview_retains_groups_links_crop_and_locked_siblings_then_cancels_redo() {
    let mut e = Editor::default();
    e.add(Kind::Button, 40., 50.);
    e.add(Kind::Label, 150., 60.);
    e.select_all();
    e.group();
    let group = *e.selection.iter().next().unwrap();
    e.document.pages[0].content.nodes[0].crop = Some(Crop {
        width: 0.8,
        ..Crop::ALL
    });
    e.document.pages[0].content.nodes[0]
        .properties
        .insert("link".into(), Value::Text("page:1".into()));
    e.add(Kind::Button, 300., 50.);
    let locked = *e.selection.iter().next().unwrap();
    e.document.pages[0].content.nodes[1].locked = true;
    // Establish a redo branch and ensure the preview never truncates it.
    e.add(Kind::Checkbox, 400., 50.);
    let redo = e.document.clone();
    e.undo();
    e.selection = HashSet::from([group, locked]);
    let before = e.document.clone();
    let redo_name = e.redo_name().map(str::to_owned);
    e.begin("Move");
    e.preview_move(
        &[
            (group, Point::new(80., 30.)),
            (locked, Point::new(80., 30.)),
        ],
        true,
    )
    .unwrap();
    assert_eq!(e.nodes().len(), 3);
    assert_eq!(&e.nodes()[..2], &before.pages[0].content.nodes);
    let mut expected = before.pages[0].content.nodes[0].clone();
    expected.id = e.nodes()[2].id;
    expected.origin = expected.origin.translated(Point::new(80., 30.));
    assert_eq!(e.nodes()[2], expected);
    assert_eq!(e.redo_name(), redo_name.as_deref());
    e.preview_move(&[(group, Point::new(80., 30.))], false)
        .unwrap();
    assert_eq!(e.nodes().len(), 2);
    e.preview_move(&[(group, Point::new(80., 30.))], true)
        .unwrap();
    assert_eq!(e.nodes()[2], expected); // stable IDs across modifier toggles
    e.cancel();
    assert_eq!(e.document, before);
    assert_eq!(e.selection, HashSet::from([group, locked]));
    e.redo();
    assert_eq!(e.document, redo);
}

#[test]
fn duplicate_inside_group_uses_local_coordinates_and_one_history_entry() {
    let mut e = Editor::default();
    e.add(Kind::Button, 40., 50.);
    e.add(Kind::Label, 150., 60.);
    e.select_all();
    e.group();
    let group = *e.selection.iter().next().unwrap();
    assert!(e.enter(group));
    let id = e.nodes()[0].id;
    e.select(id, false);
    let before = e.document.clone();
    let node = e.nodes()[0].clone();
    e.begin("Move");
    e.preview_move(&[(id, Point::new(90., -20.))], true)
        .unwrap();
    let copy = e.nodes().last().unwrap().clone();
    assert_eq!(copy.origin, node.origin.translated(Point::new(90., -20.)));
    assert_eq!(e.nodes()[0], node);
    assert_eq!(e.context, vec![group]);
    e.commit();
    assert!(e.error.is_none(), "{:?}", e.error);
    let after = e.document.clone();
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
    assert_eq!(e.undo_name(), Some("Duplicate controls"));
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
}

#[test]
fn instance_copy_keeps_definition_but_instance_child_cannot_duplicate() {
    let mut e = Editor::default();
    e.add(Kind::Button, 40., 50.);
    e.add(Kind::Label, 150., 60.);
    e.select_all();
    e.group();
    e.create_component("Shared".into()).unwrap();
    let id = e.nodes()[0].id;
    e.select(id, false);
    let before = e.document.clone();
    e.begin("Move");
    e.preview_move(&[(id, Point::new(80., 0.))], true).unwrap();
    assert_eq!(e.nodes().len(), 2);
    assert_eq!(e.nodes()[0].content, e.nodes()[1].content);
    assert_eq!(e.document.definitions, before.definitions);
    e.cancel();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    e.select(child, false);
    assert!(!e.can_duplicate_move());
    e.begin("Move");
    e.preview_move(&[(child, Point::new(20., 10.))], true)
        .unwrap();
    assert_eq!(e.nodes().len(), 2);
    assert_eq!(e.document.definitions, before.definitions);
    e.cancel();
    assert_eq!(e.document, before);
}

#[test]
fn library_source_drag_copies_definition_notes_and_crop_without_entering_it() {
    let mut e = Editor::default();
    e.add(Kind::Button, 40., 50.);
    e.select_all();
    e.group();
    let definition = e.create_component("Shared".into()).unwrap();
    e.edit_source_notes(definition, "Source notes – 世界")
        .unwrap();
    let library = e.document.default_library.unwrap();
    e.edit_library(library).unwrap();
    let id = e.nodes()[0].id;
    e.select(id, false);
    let before = e.document.clone();
    e.begin("Move");
    e.preview_move(&[(id, Point::new(70., 40.))], true).unwrap();
    assert_eq!(e.nodes().len(), 2);
    assert_eq!(e.definition, None);
    assert_eq!(e.library, Some(library));
    let copy_id = *e.selection.iter().next().unwrap();
    let source = e.document.libraries[0]
        .sources
        .iter()
        .find(|s| s.node == copy_id)
        .unwrap();
    assert_ne!(source.definition, definition);
    assert_eq!(source.notes, "Source notes – 世界");
    assert_eq!(
        e.document.definitions[1].content,
        before.definitions[0].content
    );
    assert_eq!(e.document.definitions[1].name, "Shared");
    assert_eq!(e.document.pages, before.pages);
    e.commit();
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
}

#[test]
fn invalid_preview_offset_rolls_back_without_touching_history() {
    let mut e = Editor::default();
    e.add(Kind::Button, 40., 50.);
    let id = *e.selection.iter().next().unwrap();
    let before = e.document.clone();
    e.begin("Move");
    assert!(
        e.preview_move(&[(id, Point::new(f32::NAN, 0.))], true)
            .is_err()
    );
    assert_eq!(e.document, before);
    e.cancel();
    assert_eq!(e.undo_name(), Some("Add control"));
}

#[test]
fn duplication_alignment_can_snap_to_originals_without_changing_ordinary_targets() {
    use wireframe::{model::Bounds, snapping::Constraint};
    let mut e = Editor::default();
    e.add(Kind::CircleButton, 50., 50.);
    let wanted = Bounds::new(150., 52., 48., 48.);
    let ordinary = e
        .alignment_targets(5.)
        .unwrap()
        .moved(wanted, Constraint::Free);
    let duplicate = e
        .alignment_targets_with_selection(5., true)
        .unwrap()
        .moved(wanted, Constraint::Free);
    assert_eq!(ordinary.bounds.y, 52.);
    assert_eq!(duplicate.bounds.y, 50.);
    assert!(!duplicate.guides.is_empty());
}
