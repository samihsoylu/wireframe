// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::collections::BTreeMap;
use wireframe::{document::*, workspace::Editor};

fn instance(id: Id, definition: Id, child: Id) -> Node {
    let mut node = Node::group(id, Point::new(30., 50.), vec![]);
    node.content = NodeContent::Instance {
        definition,
        overrides: vec![Override {
            path: vec![child],
            patch: Patch {
                properties: Properties::from([("text".into(), Value::Text("Local action".into()))]),
                ..Patch::default()
            },
        }],
    };
    node
}
fn fixture() -> (Editor, Id, Id, Id) {
    let mut e = Editor::default();
    let library = e.create_library("Actions").unwrap();
    let source = e
        .create_empty_source(library, "Action", Point::new(6., 6.))
        .unwrap();
    let child = e.document.allocate_id().unwrap();
    let asset = e.document.allocate_id().unwrap();
    let mut button = Node::control(child, "button", Point::default(), Size::fixed(120., 40.));
    button
        .properties
        .insert("text".into(), Value::Text("Send".into()));
    button
        .properties
        .insert("image".into(), Value::Asset(asset));
    e.document.definitions[0].content.nodes.push(button);
    e.document.assets.push(Asset {
        trashed: false,
        id: asset,
        name: "Retained artwork".into(),
        media_type: "image/png".into(),
        bytes: vec![1, 2, 3],
    });
    e.document.libraries[0].content.nodes[0].locked = true;
    e.document.libraries[0].content.nodes[0].crop = Some(Crop {
        x: 0.1,
        y: 0.2,
        width: 0.7,
        height: 0.6,
    });
    e.document.libraries[0].notes = "Library notes".into();
    e.document.libraries[0].sources[0].notes = "Source notes".into();
    e.document.default_library = Some(library);
    let other = e.create_library("Forms").unwrap();
    let parent = e
        .create_empty_source(other, "Form", Point::default())
        .unwrap();
    let nested = e.document.allocate_id().unwrap();
    e.document
        .definitions
        .iter_mut()
        .find(|d| d.id == parent)
        .unwrap()
        .content
        .nodes
        .push(instance(nested, source, child));
    let direct = e.document.allocate_id().unwrap();
    let through_parent = e.document.allocate_id().unwrap();
    e.document.pages[0].content.nodes = vec![
        instance(direct, source, child),
        instance(through_parent, parent, nested),
    ];
    let alternate = e.document.allocate_id().unwrap();
    let alternate_content = e.document.pages[0].content.clone();
    e.document.pages[0].alternates.push(Alternate {
        id: alternate,
        name: "Mobile".into(),
        version: None,
        notes: "Alternate notes".into(),
        content: alternate_content,
    });
    e.document.validate().unwrap();
    (Editor::new(e.document), library, source, other)
}
fn missing(root: &Node) -> bool {
    root.properties.get(MISSING_COMPONENT) == Some(&Value::Bool(true))
}
#[test]
fn retained_library_keeps_graph_but_disables_direct_nested_and_alternate_instances() {
    let (mut e, library, source, other) = fixture();
    e.edit_library_source(source).unwrap();
    let before = e.document.clone();
    let original = before.resolve_all(&before.pages[0].content.nodes).unwrap();
    e.trash_libraries(&[library]).unwrap();
    assert_eq!(e.library, None);
    assert_eq!(e.definition, None);
    let mut expected = before.clone();
    expected.libraries[0].trashed = true;
    assert_eq!(e.document, expected);
    assert_eq!(
        e.document
            .live_libraries()
            .map(|l| l.id)
            .collect::<Vec<_>>(),
        [other]
    );
    assert_eq!(e.document.live_definitions().count(), 1);
    assert!(e.document.component_source(source).is_some());
    assert!(e.document.live_component_source(source).is_none());
    for content in [
        &e.document.pages[0].content,
        &e.document.pages[0].alternates[0].content,
    ] {
        let resolved = e.document.resolve_all(&content.nodes).unwrap();
        assert!(missing(&resolved[0].root));
        let NodeContent::Group { children } = &resolved[1].root.content else {
            panic!()
        };
        assert!(missing(&children[0]));
    }
    assert!(e.document.resolve_library(library).is_err());
    assert_eq!(
        e.document.resolve_library_preview(library).unwrap(),
        before.resolve_library(library).unwrap()
    );
    assert_eq!(e.document, expected);
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), expected);
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(e.definition, Some(source));
    e.redo();
    assert_eq!(e.document, expected);
    e.restore_libraries(&[library]).unwrap();
    assert_eq!(e.document, before);
    assert_eq!(
        e.document
            .resolve_all(&e.document.pages[0].content.nodes)
            .unwrap(),
        original
    );
    e.undo();
    assert_eq!(e.document, expected);
    e.redo();
    assert_eq!(e.document, before);
}
#[test]
fn retained_edit_entrypoints_and_stale_transitions_are_mutation_free() {
    let (mut e, library, source, other) = fixture();
    e.trash_libraries(&[library]).unwrap();
    let before = e.document.clone();
    let undo = e.undo_name().map(str::to_owned);
    assert!(e.edit_library(library).is_err());
    assert!(e.edit_library_source(source).is_err());
    assert!(e.edit_definition(source).is_err());
    assert!(e.insert_component(source, 0., 0.).is_err());
    assert!(e.rename_definition(source, "Changed".into()).is_err());
    assert!(e.rename_library(library, "Changed").is_err());
    assert!(
        e.create_empty_source(library, "New", Point::default())
            .is_err()
    );
    assert!(e.edit_source_notes(source, "Changed").is_err());
    assert!(e.delete_definition(source).is_err());
    assert!(e.copy_source(source, "origin").is_err());
    assert!(e.copy_libraries(&[library], "origin").is_err());
    for ids in [
        vec![],
        vec![library],
        vec![other, library],
        vec![other, other],
        vec![other, 99999],
    ] {
        assert!(e.trash_libraries(&ids).is_err());
    }
    assert!(e.restore_libraries(&[library, other]).is_err());
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), undo.as_deref());
}
#[test]
fn all_libraries_may_be_retained_and_new_default_does_not_steal_their_sources() {
    let (mut e, library, _, other) = fixture();
    e.trash_libraries(&[library, other]).unwrap();
    let before = e.document.clone();
    assert_eq!(e.document.live_libraries().count(), 0);
    e.document.organize_components().unwrap();
    assert_eq!(e.document, before);
    e.add(wireframe::model::Kind::Button, 300., 200.);
    let created = e.create_component("New action".into()).unwrap();
    let owner = e.document.live_component_source(created).unwrap().0.id;
    assert_ne!(owner, library);
    assert_ne!(owner, other);
    assert_eq!(e.document.default_library, Some(owner));
    assert_eq!(e.document.libraries[..2], before.libraries);
    assert_eq!(e.document.definitions[..2], before.definitions);
    e.document.validate().unwrap();
}
#[test]
fn cut_and_retained_copy_paste_use_distinct_identities_with_exact_history() {
    for retain in [false, true] {
        let (mut e, library, source, _) = fixture();
        let before = e.document.clone();
        let copy = e.copy_libraries(&[library], "origin").unwrap();
        if retain {
            e.trash_libraries(&[library]).unwrap();
        } else {
            e.cut_libraries(&[library]).unwrap();
        }
        let removed = e.document.clone();
        assert_eq!(removed.pages, before.pages);
        assert_eq!(removed.assets, before.assets);
        assert!(!removed.definition_available(source));
        let plan = e.plan_library_import(&copy, "origin").unwrap();
        let prepared = plan.prepare(&BTreeMap::new()).unwrap();
        assert_eq!(e.apply_library_import(prepared, "origin").unwrap().added, 1);
        if retain {
            assert!(
                e.document
                    .libraries
                    .iter()
                    .find(|l| l.id == library)
                    .unwrap()
                    .trashed
            );
            assert!(!e.document.definition_available(source));
            assert_eq!(e.document.libraries.len(), 3);
        } else {
            assert!(e.document.definition_available(source));
            assert_eq!(e.document.libraries.len(), 2);
            assert_eq!(
                e.document.definitions.iter().find(|d| d.id == source),
                before.definitions.iter().find(|d| d.id == source)
            );
        }
        assert_eq!(e.document.pages, before.pages);
        let pasted = e.document.clone();
        e.undo();
        assert_eq!(e.document, removed);
        e.redo();
        assert_eq!(e.document, pasted);
        e.undo();
        e.undo();
        assert_eq!(e.document, before);
    }
}
#[test]
fn copy_of_live_consumers_does_not_resurrect_retained_dependencies() {
    let (mut e, library, source, other) = fixture();
    e.trash_libraries(&[library]).unwrap();
    let resources = e.copy_libraries(&[other], "origin").unwrap();
    let payload = resources.library_resources().unwrap();
    assert_eq!(payload.libraries.len(), 1);
    assert!(!payload.definitions.iter().any(|d| d.id == source));
    e.select(e.document.pages[0].content.nodes[0].id, false);
    let ordinary = e.copy();
    assert!(ordinary.definitions.is_empty());
    assert!(ordinary.libraries.is_empty());
    assert!(
        matches!(ordinary.nodes[0].content,NodeContent::Instance {definition,..} if definition==source)
    );
}
#[test]
fn version_eight_migrates_without_retained_state_and_old_schema_cannot_hide_it() {
    let (mut e, library, _, _) = fixture();
    let before = e.document.clone();
    let mut old = serde_json::to_value(&before).unwrap();
    old["format_version"] = 8.into();
    assert_eq!(decode(&serde_json::to_vec(&old).unwrap()).unwrap(), before);
    e.trash_libraries(&[library]).unwrap();
    let mut old = serde_json::to_value(&e.document).unwrap();
    old["format_version"] = 8.into();
    assert!(
        decode(&serde_json::to_vec(&old).unwrap())
            .unwrap_err()
            .contains("version 9")
    );
    let mut invalid = e.document.clone();
    invalid.libraries[0].sources[0].definition = 99999;
    assert!(invalid.validate().is_err());
    let mut invalid = e.document.clone();
    invalid.next_id = library;
    assert!(invalid.validate().is_err());
}

#[test]
fn cut_removes_requested_owner_but_preserves_clipboard_only_dependency() {
    let (mut e, dependency, source, requested) = fixture();
    let before = e.document.clone();
    let copy = e.copy_libraries(&[requested], "origin").unwrap();
    assert_eq!(copy.library_resources().unwrap().libraries.len(), 2);
    e.cut_libraries(&[requested]).unwrap();
    assert_eq!(e.document.libraries, vec![before.libraries[0].clone()]);
    assert_eq!(e.document.libraries[0].id, dependency);
    assert!(e.document.definition_available(source));
    assert_eq!(e.document.assets, before.assets);
    assert_eq!(e.document.pages, before.pages);
    let removed = e.document.clone();
    let prepared = e
        .plan_library_import(&copy, "origin")
        .unwrap()
        .prepare(&BTreeMap::new())
        .unwrap();
    let counts = e.apply_library_import(prepared, "origin").unwrap();
    assert_eq!(counts.added, 1);
    assert_eq!(counts.reused, 1);
    assert_eq!(e.document.libraries[0], before.libraries[0]);
    e.undo();
    assert_eq!(e.document, removed);
    e.undo();
    assert_eq!(e.document, before);
}

#[test]
fn permanent_library_removal_keeps_consumers_overrides_assets_and_other_sources() {
    use wireframe::workspace::RetainedResource;
    let (mut e, library, source, other) = fixture();
    e.trash_libraries(&[library]).unwrap();
    let before = e.document.clone();
    let mut expected = before.clone();
    expected.libraries.retain(|l| l.id != library);
    expected.definitions.retain(|d| d.id != source);
    expected.default_library = None;
    e.mark_saved();
    e.permanently_remove_resources(&[RetainedResource::Library(library)])
        .unwrap();
    assert_eq!(e.document, expected);
    assert!(e.dirty());
    assert_eq!(e.document.live_libraries().next().unwrap().id, other);
    let resolved = e
        .document
        .resolve_all(&e.document.pages[0].content.nodes)
        .unwrap();
    assert!(missing(&resolved[0].root));
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), expected);
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, after);
    assert!(e.error.as_deref().unwrap().contains("permanently deleted"));
    assert!(e.redo_name().is_none());
}

#[test]
fn permanent_library_removal_sanitizes_rename_source_edits_and_redo_history() {
    use wireframe::workspace::RetainedResource;
    let (mut e, library, source, other) = fixture();
    e.rename_library(library, "Removed library rename").unwrap();
    e.edit("Edit removed source", |e| {
        e.document
            .definitions
            .iter_mut()
            .find(|d| d.id == source)
            .unwrap()
            .content
            .nodes[0]
            .properties
            .insert("text".into(), Value::Text("Changed source".into()));
    });
    e.trash_libraries(&[library]).unwrap();
    e.rename_library(other, "Keep this rename").unwrap();
    e.undo();
    e.permanently_remove_resources(&[RetainedResource::Library(library)])
        .unwrap();
    e.redo();
    assert_eq!(e.document.libraries[0].name, "Keep this rename");
    e.undo();
    let before = e.document.clone();
    for _ in 0..3 {
        e.undo();
        assert_eq!(e.document, before);
        assert!(e.error.as_deref().unwrap().contains("permanently deleted"));
    }
    assert!(e.undo_name().is_none());
    e.redo();
    assert_eq!(e.document.libraries[0].name, "Keep this rename");
    assert!(e.document.definitions.iter().all(|d| d.id != source));
    assert!(e.redo_name().is_none());
}

#[test]
fn permanent_mixed_removal_consumes_old_restore_without_resurrection() {
    use wireframe::workspace::RetainedResource::{Library, Page};
    let (mut e, library, source, _) = fixture();
    e.trash_libraries(&[library]).unwrap();
    e.trash_pages(&[1]).unwrap();
    e.restore_resources(&[Page(1), Library(library)]).unwrap();
    e.undo();
    let placeholder = e.document.first_live_page().clone();
    e.permanently_remove_resources(&[Page(1), Library(library)])
        .unwrap();
    let after = e.document.clone();
    e.redo();
    assert_eq!(e.document, after);
    assert!(e.error.as_deref().unwrap().contains("permanently deleted"));
    for _ in 0..2 {
        e.undo();
        assert_eq!(e.document, after);
    }
    assert_eq!(e.document.pages, vec![placeholder]);
    assert!(e.document.definitions.iter().all(|d| d.id != source));
    assert!(e.undo_name().is_none());
    assert!(e.redo_name().is_none());
}

#[test]
fn stale_mixed_permanent_selection_is_atomic() {
    use wireframe::workspace::RetainedResource::{Library, Page};
    let (mut e, library, _, other) = fixture();
    e.trash_libraries(&[library]).unwrap();
    let before = e.document.clone();
    let undo = e.undo_name().map(str::to_owned);
    for selection in [
        vec![],
        vec![Library(library), Library(library)],
        vec![Library(library), Library(other)],
        vec![Library(library), Page(1)],
        vec![Library(library), Page(999)],
    ] {
        assert!(e.permanently_remove_resources(&selection).is_err());
        assert_eq!(e.document, before);
        assert_eq!(e.undo_name(), undo.as_deref());
    }
}
