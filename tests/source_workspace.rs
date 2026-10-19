// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    model::Kind,
    workspace::{ComponentUsage, Editor},
};

fn populated() -> (Editor, Id, Id) {
    let mut editor = Editor::default();
    editor.add(Kind::Button, 40., 50.);
    let definition = editor.create_component("Action".into()).unwrap();
    let library = editor.document.default_library.unwrap();
    (editor, library, definition)
}
#[test]
fn library_sources_retain_canonical_locks_ordinary_instances_do_not() {
    let (mut editor, library, definition) = populated();
    editor.document.definitions[0].content.nodes[0].locked = true;
    editor.edit_library(library).unwrap();
    let instance = editor.insert_component(definition, 500., 60.).unwrap();
    let resolved = editor.document.resolve_library(library).unwrap();
    for (root, locked) in resolved.iter().zip([true, false]) {
        let NodeContent::Group { children } = &root.root.content else {
            panic!()
        };
        assert_eq!(children[0].locked, locked);
    }
    let layers = editor.layers(library).unwrap();
    assert_eq!(layers.len(), 2);
    assert!(layers[0].control.locked);
    assert!(!layers[1].control.locked);
    let before = editor.document.clone();
    assert!(!editor.rendered_scene(library).unwrap().svg().is_empty());
    assert_eq!(editor.document, before);
    let placement = editor.document.libraries[0].sources[0].node;
    assert_eq!(editor.owning_source(placement), Some(definition));
    assert!(editor.enter(placement));
    assert_eq!(editor.definition, Some(definition));
    assert!(!editor.editing_instance());
    assert!(editor.nodes()[0].locked);
    assert!(editor.leave());
    assert_eq!(editor.surface_id(), library);
    assert!(editor.enter(instance));
    assert!(editor.editing_instance());
    assert!(!editor.nodes()[0].locked);
}
#[test]
fn library_metadata_does_not_reduce_maximum_canonical_depth_or_node_budget() {
    let mut node = Node::control(30, "label", Point::default(), Size::default());
    for id in (2..2 + MAX_DEPTH as u64).rev() {
        node = Node::group(id, Point::default(), vec![node]);
    }
    let mut project = Project {
        font: None,
        next_id: 100,
        ..Project::default()
    };
    project.definitions.push(Definition {
        id: 40,
        name: "Deep".into(),
        content: Content {
            nodes: vec![node],
            paint_order: vec![],
        },
    });
    project.organize_components().unwrap();
    let library = project.default_library.unwrap();
    assert!(project.resolve_library(library).is_ok());
    let editor = Editor::new(project);
    assert!(editor.layout_page(library).is_ok());
    let mut project = Project {
        font: None,
        next_id: MAX_NODES as u64 + 3,
        ..Project::default()
    };
    project.definitions.push(Definition {
        id: 2,
        name: "Large".into(),
        content: Content {
            nodes: (3..MAX_NODES as u64 + 3)
                .map(|id| Node::control(id, "label", Point::default(), Size::default()))
                .collect(),
            paint_order: vec![],
        },
    });
    project.organize_components().unwrap();
    assert!(
        project
            .resolve_library(project.default_library.unwrap())
            .is_ok()
    );
}
#[test]
fn library_and_source_creation_are_atomic_undoable_and_keep_page_version_return() {
    let mut editor = Editor::default();
    let alternate = editor.create_version("Compact").unwrap();
    let page = editor.page;
    let before = editor.document.clone();
    let library = editor.create_library("Shared controls").unwrap();
    assert_eq!(editor.surface_id(), library);
    assert_eq!(editor.active_alternate(page), Some(alternate));
    let source = editor
        .create_empty_source(library, "Empty card", Point::new(40., 60.))
        .unwrap();
    assert_eq!(editor.definition, Some(source));
    assert!(editor.nodes().is_empty());
    editor.add(Kind::Label, 5., 10.);
    let content = editor.document.definitions[0].content.clone();
    let filled = editor.document.clone();
    editor.undo();
    assert_eq!(editor.definition, Some(source));
    assert!(editor.nodes().is_empty());
    editor.redo();
    assert_eq!(editor.document, filled);
    assert_eq!(editor.document.definitions[0].content, content);
    editor.switch_page(page);
    assert!(!editor.in_source_workspace());
    assert!(editor.selection.is_empty());
    assert_eq!(editor.surface_id(), alternate);
    assert_eq!(editor.document.pages, before.pages);
    let reopened = decode(&filled.encode().unwrap()).unwrap();
    assert_eq!(reopened, filled);
    editor.undo();
    editor.undo();
    assert_eq!(editor.surface_id(), library);
    editor.undo();
    assert_eq!(editor.document, before);
    assert_eq!(editor.library, None);
}
#[test]
fn failed_source_creation_restores_allocator_view_selection_and_history() {
    let (mut editor, library, _) = populated();
    let before = editor.document.clone();
    let selection = editor.selection.clone();
    let undo = editor.undo_name().map(str::to_owned);
    assert!(
        editor
            .create_empty_source(library, "Bad", Point::new(f32::NAN, 0.))
            .is_err()
    );
    assert!(
        editor
            .create_empty_source(999, "Missing", Point::default())
            .is_err()
    );
    assert!(editor.create_library("\n").is_err());
    assert_eq!(editor.document, before);
    assert_eq!(editor.selection, selection);
    assert_eq!(editor.library, None);
    assert_eq!(editor.undo_name(), undo.as_deref());
}
#[test]
fn source_notes_keep_their_owner_after_navigation_and_undo_restores_only_notes() {
    let (mut editor, library, definition) = populated();
    let page = editor.page;
    editor.edit_library_source(definition).unwrap();
    editor.switch_page(page);
    editor
        .edit_source_notes(definition, "Shared action")
        .unwrap();
    editor
        .edit_source_notes(library, "Library description")
        .unwrap();
    assert_eq!(editor.surface_id(), page);
    assert_eq!(editor.document.surface_notes(definition), "Shared action");
    assert_eq!(
        editor.document.surface_notes(library),
        "Library description"
    );
    assert_eq!(editor.document.surface_notes(page), "");
    editor.undo();
    assert_eq!(editor.document.surface_notes(library), "");
    assert_eq!(editor.document.surface_notes(definition), "Shared action");
    let before = editor.document.clone();
    assert!(editor.edit_source_notes(999, "Unavailable").is_err());
    assert!(
        editor
            .edit_source_notes(definition, &"é".repeat(500_001))
            .is_err()
    );
    assert_eq!(editor.document, before);
}
#[test]
fn ordinary_library_content_and_source_removal_preserve_ownership_and_history() {
    let (mut editor, library, definition) = populated();
    let page = editor.page;
    let pages = editor.document.pages.clone();
    editor.edit_library(library).unwrap();
    editor.add(Kind::Label, 500., 50.);
    let ordinary = *editor.selection.iter().next().unwrap();
    editor.move_by(20., 30.);
    assert!(editor.error.is_none(), "{:?}", editor.error);
    let placement = editor.document.libraries[0].sources[0].node;
    editor.select(placement, false);
    let before = editor.document.clone();
    editor.move_by(10., 20.);
    assert!(editor.error.is_none(), "{:?}", editor.error);
    assert_eq!(editor.document.definitions, before.definitions);
    editor.undo();
    assert_eq!(editor.document, before);
    editor.delete();
    assert!(editor.error.is_none(), "{:?}", editor.error);
    assert!(editor.document.component_source(definition).is_none());
    assert!(
        editor.document.libraries[0]
            .content
            .nodes
            .iter()
            .any(|n| n.id == ordinary)
    );
    assert_eq!(editor.document.pages, pages);
    assert!(
        editor
            .rendered_scene(page)
            .unwrap()
            .svg()
            .contains("Component not found")
    );
    editor.undo();
    assert_eq!(editor.document, before);
    assert_eq!(editor.library, Some(library));
}
#[test]
fn usage_links_distinguish_page_alternate_nested_source_and_ordinary_library_instance() {
    let (mut editor, library, definition) = populated();
    let page = editor.page;
    let page_node = editor.nodes()[0].id;
    let alternate = editor.create_version("Compact").unwrap();
    editor.edit_library(library).unwrap();
    let ordinary = editor.insert_component(definition, 500., 20.).unwrap();
    let outer = editor
        .create_empty_source(library, "Outer", Point::new(500., 100.))
        .unwrap();
    let nested = editor.insert_component(definition, 5., 10.).unwrap();
    let uses = editor.component_usages(definition);
    assert_eq!(
        uses,
        vec![
            ComponentUsage {
                surface: page,
                path: vec![page_node]
            },
            ComponentUsage {
                surface: alternate,
                path: vec![page_node]
            },
            ComponentUsage {
                surface: outer,
                path: vec![nested]
            },
            ComponentUsage {
                surface: library,
                path: vec![ordinary]
            },
        ]
    );
    editor.visit_component_usage(definition, &uses[1]).unwrap();
    assert_eq!(editor.surface_id(), alternate);
    assert!(editor.selection.is_empty());
    editor.visit_component_usage(definition, &uses[2]).unwrap();
    assert_eq!(editor.definition, Some(outer));
    assert_eq!(editor.library, Some(library));
    assert!(editor.selection.is_empty());
    let state = (editor.page, editor.library, editor.definition);
    assert!(
        editor
            .visit_component_usage(
                definition,
                &ComponentUsage {
                    surface: page,
                    path: vec![999]
                }
            )
            .is_err()
    );
    assert_eq!((editor.page, editor.library, editor.definition), state);
}

#[test]
fn page_creation_routes_leave_library_and_preserve_operation_specific_undo() {
    let (mut editor, library, definition) = populated();
    let page = editor.page;
    editor.edit_library_source(definition).unwrap();
    let before = editor.document.clone();
    assert!(editor.create_version("Wrong owner").is_err());
    assert_eq!(editor.document, before);
    let duplicate = editor.duplicate_page(page).unwrap();
    assert_eq!(editor.page, duplicate);
    assert!(!editor.in_source_workspace());
    editor.undo();
    // Duplication deliberately returns to the original page on Undo.
    assert_eq!(editor.page, page);
    assert!(!editor.in_source_workspace());
    editor.edit_library_source(definition).unwrap();
    editor.add_page();
    assert!(!editor.in_source_workspace());
    editor.undo();
    assert_eq!(editor.library, Some(library));
    assert_eq!(editor.definition, Some(definition));
    assert_eq!(editor.document, before);
}
