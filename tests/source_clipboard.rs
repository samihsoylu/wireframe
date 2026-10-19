// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    workspace::{Clipboard, Editor},
};

fn fixture() -> (Editor, Id, Id) {
    let mut p = Project {
        font: None,
        next_id: 4,
        ..Project::default()
    };
    let mut child = Node::control(3, "button", Point::new(4., 4.), Size::default());
    child.locked = true;
    let mut group = Node::group(2, Point::new(6., 6.), vec![child]);
    group.crop = Some(Crop {
        width: 0.8,
        height: 0.8,
        ..Crop::ALL
    });
    p.pages[0].content.nodes.push(group);
    let mut e = Editor::new(p);
    e.select(2, false);
    let definition = e.create_component("Contact".into()).unwrap();
    let library = e.document.default_library.unwrap();
    e.edit_source_notes(definition, "Reusable contact note")
        .unwrap();
    e.document.libraries[0].content.nodes[0].locked = true;
    e.document.libraries[0].content.nodes[0].origin = Point::new(6., 6.);
    e.crop_group(2, None, Point::default()).unwrap();
    (e, definition, library)
}
fn only_definition(node: &Node) -> Id {
    let NodeContent::Instance { definition, .. } = node.content else {
        panic!()
    };
    definition
}
#[test]
fn source_copy_materializes_contents_and_pastes_by_destination() {
    let (mut e, definition, library) = fixture();
    let before = e.document.clone();
    let copy = e.copy_source(definition, "named-project-a").unwrap();
    assert_eq!(e.document, before);
    assert!(copy.definitions.is_empty());
    assert!(matches!(copy.nodes[0].content, NodeContent::Group { .. }));
    let json = serde_json::to_string(&copy).unwrap();
    let copy: Clipboard = serde_json::from_str(&json).unwrap();
    e.edit_library(library).unwrap();
    e.paste_from(&copy, 10., Some("named-project-a"));
    assert!(e.error.is_none(), "{:?}", e.error);
    let pasted = e.document.libraries[0].content.nodes.last().unwrap();
    assert!(!pasted.locked);
    assert_eq!(pasted.origin, Point::new(16., 16.));
    let new = only_definition(pasted);
    assert_ne!(new, definition);
    assert_eq!(e.definition, Some(new));
    assert!(e.selection.is_empty());
    assert_eq!(e.document.definitions.last().unwrap().name, "Contact (1)");
    assert!(!e.document.definitions.last().unwrap().content.nodes[0].locked);
    assert_eq!(e.document.surface_notes(new), "Reusable contact note");
    assert_eq!(
        e.document.source_appearance(new),
        e.document.source_appearance(definition)
    );
    assert_eq!(e.document.pages, before.pages);
    e.undo();
    assert_eq!(e.document, before);
    e.edit_library_source(definition).unwrap();
    e.paste_from(&copy, 20., Some("named-project-a"));
    assert!(e.error.is_none(), "{:?}", e.error);
    assert_eq!(e.document.definitions.len(), 1);
    assert_eq!(e.nodes().last().unwrap().origin, Point::new(20., 20.));
    assert!(matches!(
        e.nodes().last().unwrap().content,
        NodeContent::Group { .. }
    ));
    assert!(e.document.libraries[0].content.nodes[0].locked);
    assert!(e.selection.is_empty());
    e.undo();
    assert_eq!(e.document, before);
    e.switch_page(e.page);
    e.paste_from(&copy, 0., Some("named-project-a"));
    assert!(e.error.is_none(), "{:?}", e.error);
    assert!(matches!(
        e.nodes().last().unwrap().content,
        NodeContent::Group { .. }
    ));
    assert_eq!(e.document.definitions, before.definitions);
    assert_eq!(e.nodes()[0], before.pages[0].content.nodes[0]);
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
}
#[test]
fn cut_paste_retargets_missing_source_and_undo_preserves_local_overrides() {
    let (mut e, definition, library) = fixture();
    let before = e.document.clone();
    let NodeContent::Instance { overrides, .. } = &before.pages[0].content.nodes[0].content else {
        panic!()
    };
    assert!(
        overrides
            .iter()
            .any(|o| o.path.is_empty() && o.patch.crop == Some(CropOverride::Clear))
    );
    let copy = e.copy_source(definition, "same-project").unwrap();
    e.delete_definition(definition).unwrap();
    let cut = e.document.clone();
    e.edit_library(library).unwrap();
    e.paste_from(&copy, 0., Some("same-project"));
    assert!(e.error.is_none(), "{:?}", e.error);
    let new = e.document.definitions.last().unwrap().id;
    let mut expected = before.pages.clone();
    let NodeContent::Instance { definition: id, .. } = &mut expected[0].content.nodes[0].content
    else {
        panic!()
    };
    *id = new;
    assert_ne!(new, definition);
    assert_eq!(e.document.pages, expected);
    let pasted = e.document.clone();
    e.undo();
    assert_eq!(e.document, cut);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    e.redo();
    assert_eq!(e.document, pasted);
}
#[test]
fn absent_or_foreign_origin_never_reconnects_same_numeric_source_id() {
    let (original, definition, library) = fixture();
    let copy = original.copy_source(definition, "origin-a").unwrap();
    for origin in [None, Some("origin-b")] {
        let mut e = Editor::new(original.document.clone());
        e.delete_definition(definition).unwrap();
        let page = e.document.pages.clone();
        e.edit_library(library).unwrap();
        e.paste_from(&copy, 0., origin);
        assert!(e.error.is_none(), "{:?}", e.error);
        assert_eq!(e.document.pages, page);
        assert_eq!(
            only_definition(&e.document.pages[0].content.nodes[0]),
            definition
        );
        assert_ne!(e.document.definitions.last().unwrap().id, definition);
    }
}
#[test]
fn invalid_source_paste_rolls_back_content_allocator_selection_and_history() {
    let (mut e, definition, library) = fixture();
    e.edit_library(library).unwrap();
    let mut copy = e.copy_source(definition, "origin").unwrap();
    let before = e.document.clone();
    let selection = e.selection.clone();
    let undo = e.undo_name().map(str::to_owned);
    copy.nodes[0].origin.x = 100_000.;
    e.paste_from(&copy, 10., Some("origin"));
    assert!(e.error.take().is_some());
    assert_eq!(e.document, before);
    assert_eq!(e.selection, selection);
    assert_eq!(e.undo_name(), undo.as_deref());
    copy.nodes[0].origin.x = 6.;
    copy.source.as_mut().unwrap().node += 1;
    e.paste_from(&copy, 0., Some("origin"));
    assert!(e.error.take().is_some());
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), undo.as_deref());
}
#[test]
fn cross_project_source_import_preserves_nested_dependencies_and_high_scoped_ids() {
    let (mut original, definition, library) = fixture();
    let dependency = original
        .create_empty_source(library, "Dependency", Point::default())
        .unwrap();
    let mut instance = Node::group(900_000, Point::default(), vec![]);
    instance.content = NodeContent::Instance {
        definition: dependency,
        overrides: vec![],
    };
    original.document.definitions[0]
        .content
        .nodes
        .push(instance);
    original.document.next_id = 900_001;
    let copy = original.copy_source(definition, "origin").unwrap();
    assert_eq!(copy.definitions.len(), 1);
    let mut e = Editor::new(Project {
        font: None,
        ..Project::default()
    });
    let library = e.create_library("Destination").unwrap();
    e.edit_library(library).unwrap();
    e.paste_from(&copy, 0., Some("other"));
    assert!(e.error.is_none(), "{:?}", e.error);
    assert!(e.document.next_id > 900_000);
    assert_eq!(e.document.definitions.len(), 2);
    let source = e
        .document
        .definitions
        .iter()
        .find(|d| d.name == "Contact")
        .unwrap();
    let nested = only_definition(&source.content.nodes[1]);
    assert!(
        e.document
            .definitions
            .iter()
            .any(|d| d.id == nested && d.name == "Dependency")
    );
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
}

#[test]
fn retargeting_covers_alternates_nested_definitions_and_library_controls() {
    let (mut e, definition, library) = fixture();
    let instance = e.document.pages[0].content.nodes[0].clone();
    let alternate = e.document.allocate_id().unwrap();
    e.document.pages[0].alternates.push(Alternate {
        id: alternate,
        name: "Alternative".into(),
        version: None,
        notes: "Retained notes".into(),
        content: Content {
            nodes: vec![instance.clone()],
            paint_order: vec![],
        },
    });
    let outer = e
        .create_empty_source(library, "Outer", Point::default())
        .unwrap();
    e.document
        .definitions
        .iter_mut()
        .find(|d| d.id == outer)
        .unwrap()
        .content
        .nodes
        .push(instance.clone());
    let mut ordinary = instance;
    ordinary.id = e.document.allocate_id().unwrap();
    e.document.libraries[0].content.nodes.push(ordinary);
    e.document.validate().unwrap();
    let copy = e.copy_source(definition, "same").unwrap();
    e.delete_definition(definition).unwrap();
    let before_paste = e.document.clone();
    e.edit_library(library).unwrap();
    e.paste_from(&copy, 0., Some("same"));
    assert!(e.error.is_none(), "{:?}", e.error);
    let new = e
        .document
        .definitions
        .iter()
        .find(|d| d.name == "Contact")
        .unwrap()
        .id;
    assert_eq!(
        only_definition(&e.document.pages[0].alternates[0].content.nodes[0]),
        new
    );
    assert_eq!(
        only_definition(
            &e.document
                .definitions
                .iter()
                .find(|d| d.id == outer)
                .unwrap()
                .content
                .nodes[0]
        ),
        new
    );
    assert_eq!(
        only_definition(&e.document.libraries[0].content.nodes[1]),
        new
    );
    e.undo();
    assert_eq!(e.document, before_paste);
}

#[test]
fn old_control_clipboards_decode_without_source_provenance() {
    let json = r#"{"nodes":[],"definitions":[],"assets":[],"libraries":[]}"#;
    let copy: Clipboard = serde_json::from_str(json).unwrap();
    assert!(copy.source.is_none());
    assert!(!serde_json::to_string(&copy).unwrap().contains("source"));
}

#[test]
fn source_paste_names_reuse_numeric_suffix_and_compare_case_insensitively() {
    for (name, occupied, expected) in [
        ("Contact (7)", vec![], "Contact (1)"),
        ("Contact (1)", vec!["contact (2)"], "Contact (3)"),
        ("Contact", vec!["contact (1)", "Contact (3)"], "Contact (2)"),
        ("Contact(x)", vec![], "Contact(x) (1)"),
        ("Contact(9)", vec![], "Contact(1)"),
    ] {
        let (mut e, source, library) = fixture();
        e.rename_definition(source, name.into()).unwrap();
        for other in occupied {
            e.create_empty_source(library, other, Point::default())
                .unwrap();
        }
        let copy = e.copy_source(source, "origin").unwrap();
        e.edit_library(library).unwrap();
        let before = e.document.clone();
        e.paste_from(&copy, 10., Some("origin"));
        assert!(e.error.is_none(), "{:?}", e.error);
        assert_eq!(e.document.definitions.last().unwrap().name, expected);
        e.undo();
        assert_eq!(e.document, before);
    }
}

#[test]
fn multiple_sources_preserve_destination_selection_and_canonical_contents() {
    let (mut e, first, library) = fixture();
    e.duplicate_source(first).unwrap();
    let second = e.definition.unwrap();
    let before = e.document.clone();
    let copy = e.copy_sources(&[first, second], "same").unwrap();
    assert!(copy.source.is_none());
    assert_eq!(copy.sources.len(), 2);
    assert!(copy.definitions.is_empty());
    let copy: Clipboard = serde_json::from_str(&serde_json::to_string(&copy).unwrap()).unwrap();
    e.edit_library(library).unwrap();
    e.paste_from(&copy, 10., Some("same"));
    assert!(e.error.is_none(), "{:?}", e.error);
    assert_eq!(e.definition, None);
    assert_eq!(e.selection.len(), 2);
    assert_eq!(e.document.definitions.len(), 4);
    assert_eq!(e.document.pages, before.pages);
    for (new, original) in e.document.libraries[0].content.nodes[2..]
        .iter()
        .zip(&copy.nodes)
    {
        assert_eq!(new.origin, original.origin.translated(Point::new(10., 10.)));
        assert!(!new.locked);
        assert_eq!(new.crop, original.crop);
        assert_eq!(
            e.document.surface_notes(only_definition(new)),
            "Reusable contact note"
        );
    }
    e.undo();
    assert_eq!(e.document, before);
    e.edit_library_source(first).unwrap();
    e.paste_from(&copy, 20., Some("same"));
    assert!(e.error.is_none(), "{:?}", e.error);
    assert!(e.selection.is_empty());
    assert!(
        e.nodes()[1..]
            .iter()
            .all(|n| matches!(n.content, NodeContent::Group { .. }))
    );
    e.undo();
    assert_eq!(e.document, before);
    e.switch_page(e.page);
    e.paste_from(&copy, 0., Some("same"));
    assert_eq!(e.selection.len(), 2);
    assert!(
        e.nodes()[1..]
            .iter()
            .all(|n| matches!(n.content, NodeContent::Group { .. }))
    );
    assert_eq!(e.document.definitions, before.definitions);
}

#[test]
fn multiple_source_cut_reconnects_only_absent_sources_and_undo_is_atomic() {
    for cut_both in [false, true] {
        let (mut e, first, library) = fixture();
        e.duplicate_source(first).unwrap();
        let second = e.definition.unwrap();
        e.switch_page(e.page);
        e.insert_component(second, 100., 100.).unwrap();
        let before = e.document.clone();
        let copy = e.copy_sources(&[first, second], "same").unwrap();
        let removed = if cut_both {
            vec![first, second]
        } else {
            vec![first]
        };
        e.delete_definitions(&removed).unwrap();
        let cut = e.document.clone();
        e.edit_library(library).unwrap();
        e.paste_from(&copy, 0., Some("same"));
        assert!(e.error.is_none(), "{:?}", e.error);
        let new = &e.document.libraries[0].content.nodes;
        let a = only_definition(&new[new.len() - 2]);
        let b = only_definition(&new[new.len() - 1]);
        let mut expected = before.pages.clone();
        for node in &mut expected[0].content.nodes {
            if let NodeContent::Instance { definition, .. } = &mut node.content {
                if *definition == first {
                    *definition = a;
                } else if cut_both && *definition == second {
                    *definition = b;
                }
            }
        }
        assert_eq!(e.document.pages, expected);
        let pasted = e.document.clone();
        e.undo();
        assert_eq!(e.document, cut);
        e.undo();
        assert_eq!(e.document, before);
        e.redo();
        e.redo();
        assert_eq!(e.document, pasted);
        assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
    }
}

#[test]
fn invalid_multiple_source_metadata_rolls_back_everything() {
    let (mut e, first, library) = fixture();
    e.duplicate_source(first).unwrap();
    let second = e.definition.unwrap();
    let valid = e.copy_sources(&[first, second], "same").unwrap();
    e.edit_library(library).unwrap();
    let before = e.document.clone();
    let history = e.undo_name().map(str::to_owned);
    for case in 0..4 {
        let mut bad = valid.clone();
        match case {
            0 => bad.sources[1].node = bad.sources[0].node,
            1 => bad.sources[1].definition = bad.sources[0].definition,
            2 => bad.sources[1].project.clear(),
            _ => bad.source = Some(bad.sources[0].clone()),
        }
        e.paste_from(&bad, 0., Some("same"));
        assert!(e.error.take().is_some());
        assert_eq!(e.document, before);
        assert_eq!(e.undo_name(), history.as_deref());
        assert!(e.selection.is_empty());
    }
    assert!(e.copy_sources(&[first, first], "same").is_err());
    assert!(e.delete_definitions(&[first, u64::MAX]).is_err());
    assert_eq!(e.document, before);
}

#[test]
fn library_canvas_copy_materializes_sources_and_preserves_plain_controls() {
    let (mut e, first, library) = fixture();
    e.duplicate_source(first).unwrap();
    e.edit_library(library).unwrap();
    e.add(wireframe::model::Kind::Button, 300., 50.);
    e.selection = e.nodes().iter().map(|n| n.id).collect();
    let copy = e.copy_with_origin("same").unwrap();
    assert_eq!(copy.nodes.len(), 3);
    assert_eq!(copy.sources.len(), 2);
    assert!(matches!(copy.nodes[0].content, NodeContent::Group { .. }));
    assert!(matches!(copy.nodes[2].content, NodeContent::Control { .. }));
    let before = e.document.clone();
    e.paste_from(&copy, 10., Some("same"));
    assert!(e.error.is_none(), "{:?}", e.error);
    assert_eq!(e.definition, None);
    assert_eq!(e.selection.len(), 3);
    assert_eq!(e.document.definitions.len(), 4);
    e.undo();
    assert_eq!(e.document, before);
}

#[test]
fn multi_source_reconnection_cannot_alias_new_definitions_after_allocator_rewind() {
    let (mut original, first, _) = fixture();
    original.duplicate_source(first).unwrap();
    let second = original.definition.unwrap();
    let mut copy = original.copy_sources(&[first, second], "same").unwrap();
    copy.sources[0].definition = 101;
    copy.sources[1].definition = 102;
    let project = Project {
        font: None,
        next_id: 100,
        ..Project::default()
    };
    let mut e = Editor::new(project);
    let library = e.create_library("Restored").unwrap();
    let before = e.document.clone();
    e.edit_library(library).unwrap();
    e.paste_from(&copy, 0., Some("same"));
    assert!(e.error.is_none(), "{:?}", e.error);
    let a = only_definition(&e.document.libraries[0].content.nodes[0]);
    let b = only_definition(&e.document.libraries[0].content.nodes[1]);
    assert_ne!(a, b);
    assert!(a > 102 && b > 102);
    assert!(e.document.pages[0].content.nodes.is_empty());
    e.undo();
    assert_eq!(e.document, before);
}

#[test]
fn cutting_related_sources_reconnects_nested_dependency_without_a_hidden_duplicate() {
    let (mut e, first, library) = fixture();
    e.duplicate_source(first).unwrap();
    let second = e.definition.unwrap();
    let id = e.document.allocate_id().unwrap();
    let mut nested = Node::group(id, Point::new(50., 50.), vec![]);
    nested.content = NodeContent::Instance {
        definition: second,
        overrides: vec![],
    };
    e.document
        .definitions
        .iter_mut()
        .find(|d| d.id == first)
        .unwrap()
        .content
        .nodes
        .push(nested);
    e.document.validate().unwrap();
    let before = e.document.clone();
    let copy = e.copy_sources(&[first, second], "same").unwrap();
    e.delete_definitions(&[first, second]).unwrap();
    e.edit_library(library).unwrap();
    e.paste_from(&copy, 0., Some("same"));
    assert!(e.error.is_none(), "{:?}", e.error);
    assert_eq!(e.document.definitions.len(), 2);
    assert_eq!(e.document.libraries.len(), 1);
    let a = only_definition(&e.document.libraries[0].content.nodes[0]);
    let b = only_definition(&e.document.libraries[0].content.nodes[1]);
    let a = e.document.definitions.iter().find(|d| d.id == a).unwrap();
    assert_eq!(only_definition(a.content.nodes.last().unwrap()), b);
    e.undo();
    e.undo();
    assert_eq!(e.document, before);
}
