// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, storage::ProjectStore, workspace::Editor};

fn instance(id: Id, definition: Id) -> Node {
    let mut node = Node::group(id, Point::new(20., 30.), vec![]);
    node.content = NodeContent::Instance {
        definition,
        overrides: vec![],
    };
    node
}
fn fixture() -> Project {
    let mut source = Project {
        font: None,
        next_id: 100,
        ..Project::default()
    };
    source.pages[0].name = "Landing".into();
    let mut button = Node::control(10, "button", Point::new(20., 20.), Size::default());
    button
        .properties
        .insert("link".into(), Value::Link(Link::Page(2)));
    source.pages[0].content.nodes = vec![button];
    let mut contact = source.pages[0].clone();
    contact.id = 2;
    contact.name = "Contact".into();
    contact.parent = Some(1);
    contact.notes = "Contact page notes".into();
    let mut component = instance(20, 40);
    if let NodeContent::Instance { overrides, .. } = &mut component.content {
        let mut patch = Patch::default();
        patch
            .properties
            .insert("link".into(), Value::Link(Link::Page(1)));
        patch
            .properties
            .insert("icon".into(), Value::Icon(IconSource::Asset(50)));
        overrides.push(Override {
            path: vec![41],
            patch: patch.clone(),
        });
        overrides.push(Override {
            path: vec![42, 46],
            patch,
        });
    }
    contact.content.nodes = vec![component];
    source.versions.push(NamedVersion {
        id: 60,
        name: "Phone".into(),
    });
    contact.alternates.push(Alternate {
        id: 61,
        name: "Phone".into(),
        version: Some(60),
        notes: "Phone notes".into(),
        content: source.pages[0].content.clone(),
    });
    source.pages.push(contact);
    let mut image = Node::control(41, "image", Point::default(), Size::default());
    image.properties.insert("image".into(), Value::Asset(50));
    image
        .properties
        .insert("link".into(), Value::Link(Link::Page(2)));
    source.definitions.push(Definition {
        id: 40,
        name: "Contact card".into(),
        content: Content {
            nodes: vec![image, instance(42, 45)],
            paint_order: vec![],
        },
    });
    source.definitions.push(Definition {
        id: 45,
        name: "Nested contact".into(),
        content: Content {
            nodes: vec![Node::control(
                46,
                "button",
                Point::default(),
                Size::default(),
            )],
            paint_order: vec![],
        },
    });
    source.libraries.push(ComponentLibrary {
        id: 30,
        trashed: false,
        name: "Cards".into(),
        notes: "Library notes".into(),
        content: Content {
            nodes: vec![instance(31, 40), instance(32, 45)],
            paint_order: vec![vec![31], vec![32]],
        },
        sources: vec![
            ComponentSource {
                definition: 40,
                node: 31,
                notes: "Source notes".into(),
            },
            ComponentSource {
                definition: 45,
                node: 32,
                notes: "Nested source notes".into(),
            },
        ],
        imports: vec![],
    });
    source.default_library = Some(30);
    source.assets.push(Asset {
        attribution: Some(ImageAttribution {
            title: "Photo".into(),
            creator: "Example creator".into(),
            license: "CC BY 4.0".into(),
            license_url: "https://creativecommons.org/licenses/by/4.0/".into(),
            source_url: "https://commons.wikimedia.org/wiki/File:Photo.jpg".into(),
        }),
        trashed: false,
        id: 50,
        name: "Photo".into(),
        media_type: "image/png".into(),
        bytes: vec![1, 2, 3],
    });
    source.assets.push(Asset {
        attribution: Some(ImageAttribution {
            title: "Photo".into(),
            creator: "Example creator".into(),
            license: "CC BY 4.0".into(),
            license_url: "https://creativecommons.org/licenses/by/4.0/".into(),
            source_url: "https://commons.wikimedia.org/wiki/File:Photo.jpg".into(),
        }),
        trashed: false,
        id: 51,
        name: "Unused photo".into(),
        media_type: "image/png".into(),
        bytes: vec![4, 5, 6],
    });
    source.validate().unwrap();
    source
}
#[test]
fn complete_graph_copies_links_alternates_assets_and_scoped_overrides_atomically() {
    let source = fixture();
    let source_before = source.clone();
    let mut target = Editor::new(Project {
        font: None,
        ..Project::default()
    });
    let before = target.document.clone();
    let prepared = target
        .prepare_project_copy(&source, "fixture-origin", &[2, 1])
        .unwrap();
    assert_eq!(source, source_before);
    assert_eq!(target.document, before);
    assert_eq!(prepared.counts().pages, 2);
    assert_eq!(prepared.counts().assets_added, 2);
    let pages = prepared.imported_pages().to_vec();
    let expected = prepared.preview().clone();
    assert_eq!(expected.title, before.title);
    assert_eq!(expected.settings, before.settings);
    assert_eq!(expected.font, before.font);
    assert_eq!(expected.pages[1].name, "Landing"); // Source ordering, not selection order.
    assert_eq!(expected.pages[2].parent, Some(pages[0]));
    assert_eq!(
        expected.pages[1].content.nodes[0].properties["link"],
        Value::Link(Link::Page(pages[1]))
    );
    let library = &expected.libraries[0];
    let definition = library.sources[0].definition;
    assert_ne!(definition, 40);
    assert_eq!(library.sources[0].node, 31);
    assert_eq!(library.imports[0].definitions[&40], definition);
    assert_eq!(library.content.paint_order, [vec![31], vec![32]]);
    assert_eq!(expected.assets[0].attribution, source.assets[0].attribution);
    assert_eq!(expected.assets[1].attribution, source.assets[1].attribution);
    let asset = expected.assets[0].id;
    let def = expected
        .definitions
        .iter()
        .find(|d| d.id == definition)
        .unwrap();
    assert_eq!(def.content.nodes[0].id, 41);
    assert!(
        matches!(def.content.nodes[1].content, NodeContent::Instance { definition, .. }
        if definition == library.sources[1].definition)
    );
    assert_eq!(
        def.content.nodes[0].properties["image"],
        Value::Asset(asset)
    );
    assert_eq!(
        def.content.nodes[0].properties["link"],
        Value::Link(Link::Page(pages[1]))
    );
    let NodeContent::Instance {
        definition: d,
        overrides,
    } = &expected.pages[2].content.nodes[0].content
    else {
        panic!()
    };
    assert_eq!(*d, definition);
    assert_eq!(overrides[0].path, [41]);
    assert_eq!(overrides[1].path, [42, 46]);
    assert_eq!(
        overrides[1].patch.properties["link"],
        Value::Link(Link::Page(pages[0]))
    );
    assert_eq!(
        overrides[0].patch.properties["link"],
        Value::Link(Link::Page(pages[0]))
    );
    assert_eq!(
        overrides[0].patch.properties["icon"],
        Value::Icon(IconSource::Asset(asset))
    );
    let alternate = &expected.pages[2].alternates[0];
    assert_ne!(alternate.id, 61);
    assert_eq!(alternate.version, Some(expected.versions[0].id));
    assert_eq!(alternate.notes, "Phone notes");
    let resolved = expected
        .resolve_all(&expected.pages[2].content.nodes)
        .unwrap();
    assert!(resolved.iter().all(|node| node.orphan_overrides.is_empty()));
    target.apply_project_copy(prepared).unwrap();
    assert_eq!(target.document, expected);
    assert_eq!(target.undo_name(), Some("Import project"));
    target.undo();
    assert_eq!(target.document, before);
    target.redo();
    assert_eq!(target.document, expected);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("imported.wireframe");
    ProjectStore.save(&path, &target.document).unwrap();
    assert_eq!(ProjectStore.load(&path).unwrap(), expected);
}
#[test]
fn keep_both_reserves_names_but_reuses_identical_assets_and_version_names() {
    let source = fixture();
    let mut target = Editor::new(source.clone());
    let prepared = target
        .prepare_project_copy(&source, "fixture-origin", &[1, 2])
        .unwrap();
    assert_eq!(prepared.counts().assets_added, 0);
    assert_eq!(prepared.counts().assets_reused, 2);
    assert_eq!(prepared.preview().versions.len(), 1);
    assert_eq!(prepared.preview().pages[2].name, "Landing (2)");
    assert_eq!(prepared.preview().libraries[1].name, "Cards (2)");
    target.apply_project_copy(prepared).unwrap();
    assert_eq!(&target.document.pages[..2], &source.pages);
    assert_eq!(&target.document.libraries[..1], &source.libraries);
    let again = target
        .prepare_project_copy(&source, "fixture-origin", &[1, 2])
        .unwrap();
    assert_eq!(again.preview().pages[4].name, "Landing (3)");
}
#[test]
fn omitted_and_removed_destinations_never_bind_to_same_numbered_target_pages() {
    let mut source = fixture();
    source.removed_pages.insert(90);
    source.pages[1].content.nodes[0].properties.insert(
        "links".into(),
        Value::Links(vec![
            Some(Link::Page(90)),
            Some(Link::Url("https://example.com".into())),
        ]),
    );
    source.validate().unwrap();
    let target = Editor::new(Project {
        font: None,
        ..Project::default()
    });
    let prepared = target
        .prepare_project_copy(&source, "source", &[2])
        .unwrap();
    let after = prepared.preview();
    assert_eq!(after.pages[1].parent, None);
    assert_eq!(after.removed_pages.len(), 2); // Omitted landing plus previously removed page.
    let NodeContent::Instance { overrides, .. } = &after.pages[1].content.nodes[0].content else {
        panic!()
    };
    let Value::Link(Link::Page(id)) = overrides[0].patch.properties["link"] else {
        panic!()
    };
    assert_ne!(id, 1);
    assert!(after.removed_pages.contains(&id));
    assert!(
        matches!(&after.pages[1].content.nodes[0].properties["links"], Value::Links(v) if v[1] == Some(Link::Url("https://example.com".into())))
    );
}
#[test]
fn unavailable_components_and_orphan_paths_do_not_capture_destination_content() {
    let mut source = fixture();
    source.libraries[0].trashed = true;
    if let NodeContent::Instance { overrides, .. } = &mut source.pages[1].content.nodes[0].content {
        overrides[0].path = vec![50_000];
    }
    source.validate().unwrap();
    let target = Editor::new(fixture());
    let prepared = target
        .prepare_project_copy(&source, "source", &[2])
        .unwrap();
    let after = prepared.preview();
    assert_eq!(after.libraries.len(), 1);
    let NodeContent::Instance {
        definition,
        overrides,
    } = &after.pages[2].content.nodes[0].content
    else {
        panic!()
    };
    assert_ne!(*definition, 40);
    assert!(!after.definitions.iter().any(|d| d.id == *definition));
    assert_eq!(overrides[0].path, [50_000]);
    assert!(after.next_id > 50_000);
}
#[test]
fn stale_preparation_and_invalid_selection_never_mutate_the_destination() {
    let source = fixture();
    let mut target = Editor::default();
    let before = target.document.clone();
    for pages in [vec![1, 1], vec![999]] {
        assert!(
            target
                .prepare_project_copy(&source, "source", &pages)
                .is_err()
        );
    }
    assert!(
        target
            .prepare_project_copy(&source, "bad\nidentity", &[1])
            .is_err()
    );
    assert_eq!(target.document, before);
    let prepared = target
        .prepare_project_copy(&source, "source", &[1, 2])
        .unwrap();
    target.rename_page(1, "Changed").unwrap();
    let changed = target.document.clone();
    assert!(!prepared.matches_destination(&changed));
    assert!(target.apply_project_copy(prepared).is_err());
    assert_eq!(target.document, changed);
    assert_ne!(target.undo_name(), Some("Import project"));
}
#[test]
fn cancellation_noop_and_exhausted_ids_create_no_history_or_partial_import() {
    let source = fixture();
    let mut target = Editor::default();
    let before = target.document.clone();
    drop(
        target
            .prepare_project_copy(&source, "source", &[1, 2])
            .unwrap(),
    );
    assert_eq!(target.document, before);
    assert_eq!(target.undo_name(), None);
    let empty = Project::default();
    let prepared = target.prepare_project_copy(&empty, "empty", &[]).unwrap();
    assert_eq!(prepared.counts().pages, 0);
    target.apply_project_copy(prepared).unwrap();
    assert_eq!(target.document, before);
    assert_eq!(target.undo_name(), None);
    target.document.next_id = u64::MAX;
    let exhausted = target.document.clone();
    assert!(
        target
            .prepare_project_copy(&source, "source", &[1, 2])
            .is_err()
    );
    assert_eq!(target.document, exhausted);
}

#[test]
fn empty_import_does_not_assign_an_existing_optional_default_library() {
    let mut document = fixture();
    document.default_library = None;
    let mut target = Editor::new(document.clone());
    let prepared = target
        .prepare_project_copy(&Project::default(), "empty", &[])
        .unwrap();
    target.apply_project_copy(prepared).unwrap();
    assert_eq!(target.document, document);
    assert_eq!(target.undo_name(), None);
}

#[test]
fn two_saveable_projects_cannot_produce_an_unsaveable_prepared_import() {
    let mut source = Project {
        font: None,
        next_id: 100,
        ..Project::default()
    };
    let page = source.pages[0].clone();
    source.pages = (1..=18)
        .map(|id| {
            let mut page = page.clone();
            page.id = id;
            page.name = format!("Notes {id}");
            page.notes = "\n".repeat(1_000_000);
            page
        })
        .collect();
    assert!(source.encode().unwrap().len() < MAX_FILE_BYTES);
    let target = Editor::new(source.clone());
    let pages: Vec<_> = source.pages.iter().map(|p| p.id).collect();
    let error = target
        .prepare_project_copy(&source, "source", &pages)
        .err()
        .unwrap();
    assert!(error.contains("64 MB"), "{error}");
    assert_eq!(target.document, source);
    assert_eq!(target.undo_name(), None);
}
