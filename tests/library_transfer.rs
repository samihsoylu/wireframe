// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::collections::BTreeMap;
use wireframe::{
    document::*,
    workspace::{Clipboard, Editor},
};

fn instance(id: Id, definition: Id) -> Node {
    let mut node = Node::group(id, Point::new(20., 30.), vec![]);
    node.content = NodeContent::Instance {
        definition,
        overrides: vec![],
    };
    node
}
fn library(id: Id, definition: Id) -> ComponentLibrary {
    ComponentLibrary {
        id,
        trashed: false,
        name: format!("Library {id}"),
        notes: "Library notes".into(),
        content: Content {
            nodes: vec![instance(id + 2, definition)],
            paint_order: vec![],
        },
        sources: vec![ComponentSource {
            definition,
            node: id + 2,
            notes: "Source notes".into(),
        }],
        imports: vec![],
    }
}
fn binding(target: Id) -> LibraryImport {
    LibraryImport {
        project: "foreign-project".into(),
        library: 80,
        definitions: BTreeMap::from([(81, target)]),
    }
}
fn fixture() -> Editor {
    let mut p = Project {
        font: None,
        next_id: 60,
        ..Project::default()
    };
    let mut image = Node::control(15, "image", Point::default(), Size::default());
    image.properties.insert("image".into(), Value::Asset(50));
    let mut nested = instance(23, 31);
    if let NodeContent::Instance { overrides, .. } = &mut nested.content {
        let mut patch = Patch::default();
        patch.properties.insert("image".into(), Value::Asset(51));
        overrides.push(Override {
            path: vec![33],
            patch,
        });
    }
    p.definitions = vec![
        Definition {
            id: 11,
            name: "Container".into(),
            content: Content {
                nodes: vec![Node::group(
                    13,
                    Point::default(),
                    vec![instance(14, 21), image],
                )],
                paint_order: vec![],
            },
        },
        Definition {
            id: 21,
            name: "Nested".into(),
            content: Content {
                nodes: vec![nested],
                paint_order: vec![],
            },
        },
        Definition {
            id: 31,
            name: "Image".into(),
            content: Content {
                nodes: vec![Node::control(
                    33,
                    "image",
                    Point::default(),
                    Size::default(),
                )],
                paint_order: vec![],
            },
        },
    ];
    p.libraries = vec![library(10, 11), library(20, 21), library(30, 31)];
    p.default_library = Some(10);
    let mut label = Node::control(17, "label", Point::new(3., 4.), Size::default());
    label
        .properties
        .insert("link".into(), Value::Link(Link::Page(1)));
    let mut group = Node::group(16, Point::new(300., 400.), vec![label]);
    group.locked = true;
    group.crop = Some(Crop {
        width: 0.8,
        ..Crop::ALL
    });
    p.libraries[0].content.nodes.push(group);
    p.libraries[0].content.paint_order = vec![vec![16, 17], vec![12]];
    p.libraries[0].imports.push(binding(11));
    p.assets = (50..=52)
        .map(|id| Asset {
            attribution: None,
            trashed: false,
            id,
            name: format!("Asset {id}"),
            media_type: "image/png".into(),
            bytes: vec![1, 2, 3],
        })
        .collect();
    p.validate().unwrap();
    Editor::new(p)
}
#[test]
fn whole_library_copy_preserves_artwork_and_transitive_graph_without_mutating_source() {
    let mut e = fixture();
    e.document.assets[0].attribution = Some(ImageAttribution {
        title: "Artwork".into(),
        creator: "Example creator".into(),
        license: "CC BY 4.0".into(),
        license_url: "https://creativecommons.org/licenses/by/4.0/".into(),
        source_url: "https://commons.wikimedia.org/wiki/File:Artwork.jpg".into(),
    });
    let unrelated = e.create_library("Unrelated").unwrap();
    let before = e.document.clone();
    let copy = e.copy_libraries(&[10], "origin").unwrap();
    let resources = copy.library_resources().unwrap();
    assert_eq!(resources.requested, [10]);
    assert_eq!(
        resources.libraries.iter().map(|l| l.id).collect::<Vec<_>>(),
        [10, 20, 30]
    );
    assert!(!resources.libraries.iter().any(|l| l.id == unrelated));
    let mut expected = before.libraries[..3].to_vec();
    expected[0].imports.clear();
    assert_eq!(resources.libraries, expected);
    assert_eq!(resources.definitions, before.definitions);
    assert_eq!(resources.assets, before.assets[..2]);
    assert_eq!(e.document, before);
    let decoded: Clipboard = serde_json::from_slice(&serde_json::to_vec(&copy).unwrap()).unwrap();
    assert_eq!(
        decoded.library_resources().unwrap(),
        copy.library_resources().unwrap()
    );
    decoded.library_resources().unwrap();
    let both = e.copy_libraries(&[30, 10], "origin").unwrap();
    assert_eq!(both.library_resources().unwrap().requested, [30, 10]);
    assert_eq!(both.library_resources().unwrap().libraries.len(), 3);
}
#[test]
fn empty_library_is_nonempty_clipboard_and_cannot_enter_control_paste() {
    let mut e = Editor::default();
    let id = e.create_library("Empty").unwrap();
    let copy = e.copy_libraries(&[id], "origin").unwrap();
    assert!(!copy.is_empty());
    assert_eq!(copy.len(), 1);
    copy.library_resources().unwrap();
    let mut target = Editor::default();
    let before = target.document.clone();
    let selection = target.selection.clone();
    target.paste(&copy, 20.);
    assert!(target.error.as_deref().unwrap().contains("resource import"));
    assert_eq!(target.document, before);
    assert_eq!(target.selection, selection);
    assert!(target.undo_name().is_none());
}
#[test]
fn malformed_resource_graph_and_incompatible_envelopes_are_rejected() {
    let e = fixture();
    let good = e.copy_libraries(&[10], "origin").unwrap();
    for case in 0..10 {
        let mut bad = good.clone();
        let r = bad.resources.as_mut().unwrap();
        match case {
            0 => r.project = "invalid\norigin".into(),
            1 => r.requested.push(10),
            2 => r.requested = vec![999],
            3 => r.assets.pop().map(|_| ()).unwrap(),
            4 => r.libraries[1].id = 10,
            5 => r.libraries[0].sources[0].definition = 999,
            6 => r.libraries[0].imports.push(binding(11)),
            7 => r.requested = vec![30], // Other libraries are now unrelated.
            8 => bad.nodes.push(Node::group(99, Point::default(), vec![])),
            _ => r.definitions[0].id = 50, // Global asset/definition collision.
        }
        assert!(bad.library_resources().is_err(), "case {case}");
    }
    for requested in [vec![], vec![10, 10], vec![999]] {
        assert!(e.copy_libraries(&requested, "origin").is_err());
    }
    assert!(e.copy_libraries(&[10], "").is_err());
}
#[test]
fn missing_definition_and_external_links_stay_explicit_in_the_payload() {
    let mut e = Editor::default();
    // Use ID 1 as a missing definition, while the actual page has ID 2.
    e.document.pages[0].id = 2;
    e.document.next_id = 3;
    let library = e.create_library("Missing source").unwrap();
    let id = e.document.allocate_id().unwrap();
    e.document.libraries[0].content.nodes.push(instance(id, 1));
    e.document.validate().unwrap();
    let copy = e.copy_libraries(&[library], "origin").unwrap();
    assert!(matches!(
        copy.library_resources().unwrap().libraries[0].content.nodes[0].content,
        NodeContent::Instance { definition: 1, .. }
    ));
    // Validation must not strip an external page link from the actual payload.
    let e = fixture();
    let copy = e.copy_libraries(&[10], "origin").unwrap();
    let NodeContent::Group { children } =
        &copy.library_resources().unwrap().libraries[0].content.nodes[1].content
    else {
        panic!()
    };
    assert_eq!(
        children[0].properties.get("link"),
        Some(&Value::Link(Link::Page(1)))
    );
}
#[test]
fn version_seven_migration_preserves_graph_and_version_eight_round_trips_correspondence() {
    let mut p = fixture().document;
    let current = p.clone();
    assert_eq!(decode(&p.encode().unwrap()).unwrap(), current);
    p.format_version = 7;
    assert!(
        decode(&serde_json::to_vec(&p).unwrap())
            .unwrap_err()
            .contains("version 8")
    );
    p.libraries[0].imports.clear();
    let mut expected = p.clone();
    expected.format_version = FORMAT_VERSION;
    assert_eq!(decode(&serde_json::to_vec(&p).unwrap()).unwrap(), expected);
    let mut raw = serde_json::to_value(p).unwrap();
    raw["libraries"][0]["imports"] = serde_json::json!({});
    assert!(decode(&serde_json::to_vec(&raw).unwrap()).is_err());
}
#[test]
fn invalid_correspondence_is_rejected_and_historical_id_stays_reserved() {
    let p = fixture().document;
    for case in 0..7 {
        let mut bad = p.clone();
        let imports = &mut bad.libraries[0].imports;
        match case {
            0 => imports.push(imports[0].clone()),
            1 => imports[0].project.clear(),
            2 => imports[0].library = 0,
            3 => {
                imports[0].definitions.insert(82, 11);
            }
            4 => {
                imports[0].definitions.insert(0, 59);
            }
            5 => {
                imports[0].definitions.insert(82, 1);
            } // Page is not a source.
            _ => {
                imports[0].definitions.insert(82, 60);
            } // Stale allocator.
        }
        assert!(bad.validate().is_err(), "case {case}");
    }
    let mut p = p;
    p.libraries[0].imports[0].definitions.insert(82, 60);
    assert_eq!(p.allocate_id().unwrap(), 61);
    p.validate().unwrap();
    assert_eq!(decode(&p.encode().unwrap()).unwrap(), p);
    let mut e = Editor::new(p.clone());
    e.rename_library(10, "New name").unwrap();
    e.undo();
    assert_eq!(e.document, p);
    e.redo();
    assert_eq!(e.document.libraries[0].imports, p.libraries[0].imports);
}
#[test]
fn ordinary_clipboard_clears_foreign_numeric_correspondence() {
    let mut e = fixture();
    let id = e.document.allocate_id().unwrap();
    e.document.pages[0].content.nodes.push(instance(id, 11));
    e.select(id, false);
    let copy = e.copy();
    assert!(copy.libraries.iter().all(|l| l.imports.is_empty()));
    assert!(copy.resources.is_none());
    let mut target = Editor::default();
    target.paste(&copy, 0.);
    assert!(target.error.is_none(), "{:?}", target.error);
    target.document.validate().unwrap();
    assert!(
        target
            .document
            .libraries
            .iter()
            .all(|l| l.imports.is_empty())
    );
}
