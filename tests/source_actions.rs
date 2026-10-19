// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, workspace::Editor};

fn fixture() -> (Editor, Id, Id, Id) {
    let mut p = Project {
        font: None,
        next_id: 4,
        ..Project::default()
    };
    let mut child = Node::control(
        3,
        "button",
        Point::default(),
        Size {
            width: Dimension::Fixed(240.),
            height: Dimension::Fixed(120.),
        },
    );
    child.locked = true;
    let mut group = Node::group(2, Point::new(50., 50.), vec![child]);
    group.crop = Some(Crop {
        width: 0.8,
        height: 0.8,
        ..Crop::ALL
    });
    p.pages[0].content.nodes.push(group);
    let mut e = Editor::new(p);
    e.select(2, false);
    let definition = e.create_component("Source".into()).unwrap();
    let library = e.document.default_library.unwrap();
    let placement = e.document.libraries[0].sources[0].node;
    e.edit_source_notes(definition, "Source notes").unwrap();
    (e, definition, library, placement)
}
#[test]
fn duplication_is_independent_and_keeps_canonical_children_notes_crop_and_undo() {
    let (mut e, definition, library, placement) = fixture();
    e.edit_library_source(definition).unwrap();
    let before = e.document.clone();
    e.duplicate_source(definition).unwrap();
    let copy = e.definition.unwrap();
    assert_ne!(copy, definition);
    assert_eq!(e.library, Some(library));
    assert!(e.selection.is_empty());
    let mut unlocked_content = before.definitions[0].content.clone();
    unlocked_content.nodes[0].locked = false;
    assert_eq!(e.document.definitions[1].content, unlocked_content);
    assert_eq!(e.document.definitions[0], before.definitions[0]);
    assert_eq!(e.document.definitions[1].name, "Source copy");
    assert_eq!(
        e.document.source_appearance(definition),
        e.document.source_appearance(copy)
    );
    assert_eq!(e.document.surface_notes(copy), "Source notes");
    let nodes = &e.document.libraries[0].content.nodes;
    assert_eq!(
        nodes[1].origin,
        nodes[0].origin.translated(Point::new(10., 10.))
    );
    assert_ne!(nodes[1].id, placement);
    assert_eq!(e.document.pages, before.pages);
    let duplicated = e.document.clone();
    assert_eq!(decode(&duplicated.encode().unwrap()).unwrap(), duplicated);
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(e.definition, Some(definition));
    e.redo();
    assert_eq!(e.document, duplicated);
    assert_eq!(e.definition, Some(copy));
    e.delete_definition(copy).unwrap();
    assert!(
        e.document
            .resolve(&e.document.pages[0].content.nodes[0])
            .unwrap()
            .orphan_overrides
            .is_empty()
    );
}
#[test]
fn group_undo_restores_old_identities_but_ungroup_promotes_fresh_sources() {
    let (mut e, original, library, _) = fixture();
    e.duplicate_source(original).unwrap();
    let old_definitions = e.document.definitions.clone();
    let old_placements = e.document.libraries[0].content.nodes.clone();
    e.edit_library(library).unwrap();
    e.selection = old_placements.iter().map(|n| n.id).collect();
    let before = e.document.clone();
    e.group();
    assert!(e.error.is_none(), "{:?}", e.error);
    let outer = e.definition.unwrap();
    assert!(!old_definitions.iter().any(|d| d.id == outer));
    assert_eq!(e.document.definitions.len(), 1);
    assert_eq!(e.document.libraries[0].sources.len(), 1);
    let children = &e.document.definitions[0].content.nodes;
    assert_eq!(children.len(), 2);
    for (child, old) in children.iter().zip(&old_definitions) {
        let NodeContent::Group { children: contents } = &child.content else {
            panic!()
        };
        assert_eq!(contents, &old.content.nodes);
        assert_eq!(
            child.properties.get(GROUP_NAME),
            Some(&Value::Text(old.name.clone()))
        );
        assert_eq!(
            child.properties.get(GROUP_NOTES),
            Some(&Value::Text("Source notes".into()))
        );
    }
    assert_eq!(e.document.pages, before.pages);
    let resolved = e
        .document
        .resolve(&e.document.pages[0].content.nodes[0])
        .unwrap()
        .root;
    assert!(resolved.properties.contains_key(MISSING_COMPONENT));
    let grouped = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(e.definition, None);
    e.redo();
    assert_eq!(e.document, grouped);
    e.edit_library(library).unwrap();
    let outer_node = e.document.libraries[0].sources[0].node;
    e.select(outer_node, false);
    e.ungroup();
    assert!(e.error.is_none(), "{:?}", e.error);
    assert_eq!(e.definition, None);
    assert_eq!(e.selection.len(), 2);
    assert_eq!(e.document.definitions.len(), 2);
    for (new, old) in e.document.definitions.iter().zip(&old_definitions) {
        assert!(!old_definitions.iter().any(|d| d.id == new.id));
        assert_eq!(new.content, old.content);
        assert_eq!(new.name, old.name);
        assert_eq!(e.document.surface_notes(new.id), "Source notes");
    }
    for (new, old) in e.document.libraries[0]
        .content
        .nodes
        .iter()
        .zip(&old_placements)
    {
        assert_ne!(new.id, old.id);
        assert_eq!(new.origin, old.origin);
        assert_eq!(new.crop, old.crop);
    }
    assert_eq!(e.document.pages, before.pages);
    assert!(
        e.document
            .resolve(&e.document.pages[0].content.nodes[0])
            .unwrap()
            .root
            .properties
            .contains_key(MISSING_COMPONENT)
    );
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
    e.undo();
    assert_eq!(e.document, grouped);
    e.undo();
    assert_eq!(e.document, before);
}
#[test]
fn source_grouping_retains_unexpanded_nested_instances_and_full_notes() {
    let (mut e, inner, library, _) = fixture();
    let outer = e.document.allocate_id().unwrap();
    let nested = e.document.pages[0].content.nodes[0].clone();
    e.document.definitions.push(Definition {
        id: outer,
        name: "Outer".into(),
        content: Content {
            nodes: vec![nested.clone()],
            paint_order: vec![],
        },
    });
    e.document.organize_components().unwrap();
    e.edit_source_notes(outer, &"N".repeat(150_000)).unwrap();
    let outer_node = e.document.component_source(outer).unwrap().1.node;
    e.edit_library(library).unwrap();
    e.select(outer_node, false);
    e.group();
    assert!(e.error.is_none(), "{:?}", e.error);
    let combined = e.definition.unwrap();
    let root = &e
        .document
        .definitions
        .iter()
        .find(|d| d.id == combined)
        .unwrap()
        .content
        .nodes[0];
    let NodeContent::Group { children } = &root.content else {
        panic!()
    };
    assert_eq!(children, &vec![nested]);
    assert!(e.document.definitions.iter().any(|d| d.id == inner));
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
    e.edit_library(library).unwrap();
    let id = e.document.component_source(combined).unwrap().1.node;
    e.select(id, false);
    e.ungroup();
    assert!(e.error.is_none(), "{:?}", e.error);
    let new = e
        .document
        .definitions
        .iter()
        .find(|d| d.id != inner)
        .unwrap()
        .id;
    assert_eq!(e.document.surface_notes(new).len(), 150_000);
}
#[test]
fn invalid_duplication_is_atomic_and_locked_library_selection_is_not_grouped() {
    let (mut e, definition, library, placement) = fixture();
    e.document.libraries[0].content.nodes[0].origin.x = 100_000.;
    e.edit_library_source(definition).unwrap();
    let before = e.document.clone();
    let history = e.undo_name().map(str::to_owned);
    assert!(e.duplicate_source(definition).is_err());
    assert_eq!(e.document, before);
    assert_eq!(e.definition, Some(definition));
    assert_eq!(e.undo_name(), history.as_deref());
    assert!(e.duplicate_source(9999).is_err());
    assert_eq!(e.document, before);
    e.document.libraries[0].content.nodes[0].origin.x = 50.;
    e.document.libraries[0].content.nodes[0].locked = true;
    e.edit_library(library).unwrap();
    e.select(placement, false);
    let before = e.document.clone();
    e.group();
    e.ungroup();
    assert_eq!(e.document, before);
}
#[test]
fn library_duplicate_route_and_mixed_ungroup_keep_ordinary_content_and_order() {
    let (mut e, definition, library, placement) = fixture();
    e.edit_library(library).unwrap();
    e.select(placement, false);
    e.duplicate();
    assert!(e.error.is_none(), "{:?}", e.error);
    assert_ne!(e.definition, Some(definition));
    assert_eq!(e.document.definitions[1].name, "Source");
    assert!(e.document.definitions[0].content.nodes[0].locked);
    assert!(!e.document.definitions[1].content.nodes[0].locked);
    e.edit_library(library).unwrap();
    let id = e.document.allocate_id().unwrap();
    e.document.libraries[0].content.nodes.push(Node::control(
        id,
        "label",
        Point::new(700., 40.),
        Size::default(),
    ));
    e.document.libraries[0].content.paint_order =
        e.document.libraries[0].content.paint_paths().unwrap();
    e.selection = [placement, id].into_iter().collect();
    e.group();
    assert!(e.error.is_none(), "{:?}", e.error);
    let grouped = e.document.clone();
    let outer = e.definition.unwrap();
    e.edit_library(library).unwrap();
    let id = e.document.component_source(outer).unwrap().1.node;
    e.select(id, false);
    e.ungroup();
    assert!(e.error.is_none(), "{:?}", e.error);
    e.document.validate().unwrap();
    assert_eq!(e.document.libraries[0].content.nodes.len(), 3);
    assert!(
        e.document.libraries[0]
            .content
            .nodes
            .iter()
            .any(|n| matches!(n.content, NodeContent::Control { .. }))
    );
    assert_eq!(e.document.libraries[0].sources.len(), 2);
    e.undo();
    assert_eq!(e.document, grouped);
}

#[test]
fn ungroup_retains_ancestor_markup_over_an_instance_local_clear() {
    let (mut e, _, _, _) = fixture();
    let mut instance = e.document.pages[0].content.nodes[0].clone();
    if let NodeContent::Instance { overrides, .. } = &mut instance.content {
        overrides.push(Override {
            path: vec![],
            patch: Patch {
                markup: Some(false),
                ..Patch::default()
            },
        });
    }
    let mut parent = Node::group(900, Point::default(), vec![instance]);
    parent.markup = true;
    let children = parent.ungroup().unwrap();
    assert!(e.document.resolve(&children[0]).unwrap().root.markup);
    e.document.pages[0].content.nodes = children;
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
}
#[test]
fn group_depth_failure_rolls_back_sources_allocator_selection_and_history() {
    let (mut e, definition, library, placement) = fixture();
    let mut child = Node::control(900, "label", Point::default(), Size::default());
    for id in (901..901 + MAX_DEPTH as u64).rev() {
        child = Node::group(id, Point::default(), vec![child]);
    }
    e.document
        .definitions
        .iter_mut()
        .find(|d| d.id == definition)
        .unwrap()
        .content
        .nodes = vec![child];
    e.document.next_id = 1000;
    e.document.pages[0].content.nodes.clear();
    e.document.validate().unwrap();
    e.edit_library(library).unwrap();
    e.select(placement, false);
    let before = e.document.clone();
    let selection = e.selection.clone();
    let history = e.undo_name().map(str::to_owned);
    e.group();
    assert!(e.error.is_some());
    assert_eq!(e.document, before);
    assert_eq!(e.selection, selection);
    assert_eq!(e.undo_name(), history.as_deref());
    assert_eq!(e.definition, None);
}

#[test]
fn navigator_copy_names_are_unique_in_the_owning_library() {
    let (mut e, definition, library, _) = fixture();
    let other = e.create_library("Other library").unwrap();
    e.create_empty_source(other, "Source copy", Point::default())
        .unwrap();
    for name in ["Source copy", "Source copy 1", "Source copy 2"] {
        e.duplicate_source(definition).unwrap();
        let copy = e.definition.unwrap();
        assert_eq!(
            e.document
                .definitions
                .iter()
                .find(|d| d.id == copy)
                .unwrap()
                .name,
            name
        );
        assert_eq!(e.document.component_source(copy).unwrap().0.id, library);
    }
    let latest = e.definition.unwrap();
    e.duplicate_source(latest).unwrap();
    assert_eq!(
        e.document.definitions.last().unwrap().name,
        "Source copy 2 copy"
    );
    assert_eq!(e.document.libraries.len(), 2);
}

#[test]
fn source_duplicate_unlocks_literal_tree_without_mutating_shared_dependencies() {
    let (mut e, definition, library, _) = fixture();
    let dependency = e
        .create_empty_source(library, "Dependency", Point::default())
        .unwrap();
    let original_child = e.document.definitions[0].content.nodes[0].clone();
    let instance_id = e.document.allocate_id().unwrap();
    let mut instance = Node::group(instance_id, Point::new(300., 0.), vec![]);
    instance.content = NodeContent::Instance {
        definition: dependency,
        overrides: vec![],
    };
    instance.locked = true;
    let group_id = e.document.allocate_id().unwrap();
    let mut group = Node::group(group_id, Point::default(), vec![original_child, instance]);
    group.locked = true;
    e.document.definitions[0].content.nodes = vec![group];
    e.document.libraries[0].content.nodes[0].locked = true;
    e.edit_library_source(definition).unwrap();
    let before = e.document.clone();
    e.duplicate_source(definition).unwrap();
    let copy = e.definition.unwrap();
    let (lib, owner) = e.document.component_source(copy).unwrap();
    assert!(
        !lib.content
            .nodes
            .iter()
            .find(|n| n.id == owner.node)
            .unwrap()
            .locked
    );
    let cloned = &e
        .document
        .definitions
        .iter()
        .find(|d| d.id == copy)
        .unwrap()
        .content
        .nodes[0];
    assert!(!cloned.locked);
    let NodeContent::Group { children } = &cloned.content else {
        panic!()
    };
    assert!(children.iter().all(|n| !n.locked));
    assert!(
        matches!(children[1].content, NodeContent::Instance { definition: id, .. } if id == dependency)
    );
    assert_eq!(&e.document.definitions[..2], &before.definitions[..]);
    assert_eq!(e.document.pages, before.pages);
    let after = e.document.clone();
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
}
