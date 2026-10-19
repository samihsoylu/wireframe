// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, model::Kind, workspace::Editor};

fn fixture() -> Editor {
    let mut editor = Editor::default();
    editor.add(Kind::Button, 30., 40.);
    editor.create_component("Action".into()).unwrap();
    editor
}
#[test]
fn migration_preserves_existing_graph_and_allocates_only_library_metadata() {
    let mut e = fixture();
    let root = *e.selection.iter().next().unwrap();
    e.enter(root);
    let child = e.nodes()[0].id;
    e.select(child, false);
    e.begin("Local text");
    e.set_control_property(child, "text", Value::Text("Local value".into()))
        .unwrap();
    e.commit();
    let mut old = e.document.clone();
    old.libraries.clear();
    old.default_library = None;
    let expected = old.clone();
    for version in 2..=5 {
        old.format_version = version;
        let bytes = serde_json::to_vec_pretty(&old).unwrap();
        let migrated = decode(&bytes).unwrap();
        assert_eq!(migrated.definitions, expected.definitions);
        assert_eq!(migrated.pages, expected.pages);
        assert_eq!(migrated.assets, expected.assets);
        assert_eq!(migrated.font, expected.font);
        assert_eq!(migrated.settings, expected.settings);
        assert_eq!(migrated.libraries.len(), 1);
        assert_eq!(migrated.default_library, Some(expected.next_id));
        assert_eq!(migrated.libraries[0].sources[0].node, expected.next_id + 1);
        assert_eq!(migrated.next_id, expected.next_id + 2);
        assert_eq!(
            migrated
                .resolve(&migrated.pages[0].content.nodes[0])
                .unwrap()
                .root,
            e.document
                .resolve(&e.document.pages[0].content.nodes[0])
                .unwrap()
                .root
        );
        assert_eq!(decode(&migrated.encode().unwrap()).unwrap(), migrated);
    }
}
#[test]
fn library_content_and_source_notes_round_trip_without_changing_source_artwork() {
    let mut e = fixture();
    let before = e.document.definitions.clone();
    let id = e.document.allocate_id().unwrap();
    let library = &mut e.document.libraries[0];
    library.notes = "Library notes".into();
    library.sources[0].notes = "Source notes".into();
    library.content.nodes.push(Node::control(
        id,
        "label",
        Point::new(600., 70.),
        Size::default(),
    ));
    let library_id = library.id;
    let definition = library.sources[0].definition;
    assert_eq!(e.document.surface_notes(library_id), "Library notes");
    assert_eq!(e.document.surface_notes(definition), "Source notes");
    assert!(e.document.surface(library_id).is_some());
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
    assert_eq!(e.document.definitions, before);
}
#[test]
fn invalid_ownership_and_overrides_are_rejected() {
    let p = fixture().document;
    let mut bad = p.clone();
    bad.libraries.clear();
    bad.default_library = None;
    assert!(bad.validate().is_err());
    let mut bad = p.clone();
    let duplicate = bad.libraries[0].sources[0].clone();
    bad.libraries[0].sources.push(duplicate);
    assert!(bad.validate().is_err());
    let mut bad = p.clone();
    bad.libraries[0].sources[0].node = 900;
    assert!(bad.validate().is_err());
    let mut bad = p.clone();
    bad.libraries[0].sources[0].definition = 900;
    assert!(bad.validate().is_err());
    let mut bad = p.clone();
    bad.default_library = Some(900);
    assert!(bad.validate().is_err());
    let mut bad = p.clone();
    bad.libraries[0].content.nodes[0]
        .properties
        .insert("text".into(), Value::Text("Override".into()));
    assert!(bad.validate().is_err());
    let mut bad = p.clone();
    bad.libraries[0].content.nodes[0].origin.x = f32::NAN;
    assert!(bad.validate().is_err());
    let mut bad = p.clone();
    bad.libraries[0].notes = "x".repeat(1_000_001);
    assert!(bad.validate().is_err());
    let mut bad = p.clone();
    bad.libraries[0].sources[0].notes = "x".repeat(1_000_001);
    assert!(bad.validate().is_err());
}
#[test]
fn old_tags_cannot_smuggle_library_data_and_bad_allocator_is_not_repaired() {
    let mut p = fixture().document;
    p.format_version = 5;
    assert!(
        decode(&serde_json::to_vec(&p).unwrap())
            .unwrap_err()
            .contains("version 6")
    );
    p.libraries.clear();
    p.default_library = None;
    p.next_id = 2;
    assert!(
        decode(&serde_json::to_vec(&p).unwrap())
            .unwrap_err()
            .contains("next ID")
    );
    p.format_version = FORMAT_VERSION;
    let before = p.clone();
    assert!(p.organize_components().is_err());
    assert_eq!(p, before);
}
#[test]
fn ownership_wrappers_do_not_spend_the_canonical_node_budget() {
    let mut p = Project {
        font: None,
        ..Project::default()
    };
    p.definitions.push(Definition {
        id: 2,
        name: "Large".into(),
        content: Content {
            nodes: (3..MAX_NODES as u64 + 3)
                .map(|id| Node::control(id, "label", Point::default(), Size::default()))
                .collect(),
            paint_order: vec![],
        },
    });
    p.next_id = MAX_NODES as u64 + 3;
    p.organize_components().unwrap();
    p.validate().unwrap();
    let id = p.allocate_id().unwrap();
    p.libraries[0].content.nodes.push(Node::control(
        id,
        "label",
        Point::default(),
        Size::default(),
    ));
    assert!(p.validate().unwrap_err().contains("50,000"));
}
#[test]
fn clipboard_preserves_source_notes_and_library_identity_without_copying_unrelated_controls() {
    let mut e = fixture();
    let id = e.document.allocate_id().unwrap();
    e.document.libraries[0].notes = "Shared".into();
    e.document.libraries[0].sources[0].notes = "Keep source notes".into();
    e.document.libraries[0].content.nodes.push(Node::control(
        id,
        "label",
        Point::default(),
        Size::default(),
    ));
    let copy = e.copy();
    assert_eq!(copy.libraries.len(), 1);
    assert_eq!(copy.libraries[0].content.nodes.len(), 1);
    let mut target = Editor::default();
    target.paste(&copy, 0.);
    assert!(target.error.is_none(), "{:?}", target.error);
    assert_eq!(target.document.libraries[0].notes, "Shared");
    assert_eq!(
        target.document.libraries[0].sources[0].notes,
        "Keep source notes"
    );
    target.document.validate().unwrap();
    let pasted = target.document.clone();
    target.undo();
    assert!(target.document.libraries.is_empty());
    target.redo();
    assert_eq!(target.document, pasted);
    let before = e.document.libraries.clone();
    e.paste(&copy, 20.);
    assert_eq!(e.document.libraries, before);
    e.document.validate().unwrap();
}
#[test]
fn source_delete_removes_ownership_but_preserves_missing_instances_and_undo() {
    let mut e = fixture();
    let before = e.document.clone();
    let definition = e.document.definitions[0].id;
    e.delete_definition(definition).unwrap();
    assert!(e.document.libraries[0].sources.is_empty());
    assert!(e.document.libraries[0].content.nodes.is_empty());
    assert_eq!(e.document.pages, before.pages);
    e.document.validate().unwrap();
    e.undo();
    assert_eq!(e.document, before);
}

#[test]
fn malformed_clipboard_is_atomic_and_legacy_clipboard_gets_ownership() {
    let source = fixture();
    let copy = source.copy();
    for invalid in 0..3 {
        let mut bad = copy.clone();
        match invalid {
            0 => bad.libraries[0].sources[0].node = 999,
            1 => bad.libraries[0].content.nodes.push(Node::control(
                999,
                "label",
                Point::default(),
                Size::default(),
            )),
            _ => bad.libraries[0].id = bad.definitions[0].id,
        }
        let mut target = Editor::default();
        let before = target.document.clone();
        let selection = target.selection.clone();
        target.paste(&bad, 0.);
        assert!(
            target.error.is_some(),
            "invalid clipboard {invalid} accepted"
        );
        assert_eq!(target.document, before);
        assert_eq!(target.selection, selection);
        assert!(target.undo_name().is_none());
    }
    let mut legacy = serde_json::to_value(&copy).unwrap();
    legacy.as_object_mut().unwrap().remove("libraries");
    let legacy = serde_json::from_value(legacy).unwrap();
    let mut target = Editor::default();
    target.paste(&legacy, 0.);
    assert!(target.error.is_none(), "{:?}", target.error);
    target.document.validate().unwrap();
    assert_eq!(target.document.libraries.len(), 1);
    assert_eq!(target.document.definitions.len(), 1);
}

#[test]
fn ordinary_library_controls_validate_references_reserve_paths_and_clear_removed_page_links() {
    let mut editor = fixture();
    editor.add_page();
    let removed_page = editor.page;
    let id = editor.document.allocate_id().unwrap();
    let definition = editor.document.definitions[0].id;
    let mut node = Node::group(id, Point::default(), vec![]);
    node.content = NodeContent::Instance {
        definition,
        overrides: vec![Override {
            path: vec![editor.document.next_id],
            patch: Patch::default(),
        }],
    };
    node.properties
        .insert("link".into(), Value::Link(Link::Page(removed_page)));
    editor.document.libraries[0].content.nodes.push(node);
    let reserved = editor.document.next_id;
    assert!(editor.document.allocate_id().unwrap() > reserved);
    editor.document.validate().unwrap();
    let mut bad = editor.document.clone();
    bad.libraries[0].content.nodes[1]
        .properties
        .insert("image".into(), Value::Asset(999));
    assert!(bad.validate().is_err());
    editor.delete_page();
    assert!(editor.error.is_none(), "{:?}", editor.error);
    assert_eq!(
        editor.document.libraries[0].content.nodes[1]
            .properties
            .get("link"),
        Some(&Value::Null)
    );
    editor.document.validate().unwrap();
}

#[test]
fn importing_source_into_empty_ordered_group_updates_ownership_and_paint_paths_atomically() {
    let source = fixture();
    let mut project = Project::default();
    project.pages[0].content = Content {
        nodes: vec![Node::group(2, Point::default(), vec![])],
        paint_order: vec![vec![2]],
    };
    project.next_id = 3;
    let mut target = Editor::new(project.clone());
    assert!(target.enter(2));
    target.paste(&source.copy(), 0.);
    assert!(target.error.is_none(), "{:?}", target.error);
    assert_eq!(target.nodes().len(), 1);
    let inserted = target.nodes()[0].id;
    assert_eq!(
        target.document.pages[0].content.paint_order,
        vec![vec![2, inserted]]
    );
    assert_eq!(target.document.libraries.len(), 1);
    target.document.validate().unwrap();
    let pasted = target.document.clone();
    target.undo();
    assert_eq!(target.document, project);
    target.redo();
    assert_eq!(target.document, pasted);
}

#[test]
fn clipboard_remaps_source_placements_simultaneously_when_old_and_new_ids_overlap() {
    let mut source = fixture();
    source.add(Kind::Label, 200., 60.);
    source.create_component("Caption".into()).unwrap();
    source.select_all();
    let mut copy = source.copy();
    let library = &mut copy.libraries[0];
    for (index, id) in [6, 5].into_iter().enumerate() {
        library.content.nodes[index].id = id;
        library.sources[index].node = id;
    }
    library.content.paint_order = vec![vec![5], vec![6]];
    let mut target = Editor::default();
    target.paste(&copy, 0.);
    assert!(target.error.is_none(), "{:?}", target.error);
    let library = &target.document.libraries[0];
    assert_eq!(library.content.nodes[0].id, 5);
    assert_eq!(library.content.nodes[1].id, 6);
    assert_eq!(library.content.paint_order, vec![vec![6], vec![5]]);
    assert_eq!(library.sources[0].node, 5);
    assert_eq!(library.sources[1].node, 6);
    for (node, source) in library.content.nodes.iter().zip(&library.sources) {
        assert!(
            matches!(node.content, NodeContent::Instance { definition, .. } if definition == source.definition)
        );
    }
    target.document.validate().unwrap();
}

#[test]
fn legacy_projects_with_thousands_of_empty_sources_migrate_within_coordinate_limits() {
    let old = Project {
        font: None,
        definitions: (2..1702)
            .map(|id| Definition {
                id,
                name: format!("Source {id}"),
                content: Content::default(),
            })
            .collect(),
        next_id: 1702,
        format_version: 5,
        ..Project::default()
    };
    let migrated = decode(&serde_json::to_vec(&old).unwrap()).unwrap();
    assert_eq!(migrated.definitions, old.definitions);
    assert_eq!(migrated.libraries[0].sources.len(), 1700);
    let positions: std::collections::HashSet<_> = migrated.libraries[0]
        .content
        .nodes
        .iter()
        .map(|n| (n.origin.x.to_bits(), n.origin.y.to_bits()))
        .collect();
    assert_eq!(positions.len(), 1700);
    migrated.validate().unwrap();
}
