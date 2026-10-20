// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::sync::Arc;
use wireframe::{
    document::*,
    model::Kind,
    workspace::{CatalogCategory, CatalogItem, ComponentPreviews, Editor},
};

fn instance(id: Id, definition: Id) -> Node {
    let mut n = Node::group(id, Point::default(), vec![]);
    n.content = NodeContent::Instance {
        definition,
        overrides: vec![],
    };
    n
}
fn fixture() -> Editor {
    let mut p = Project {
        next_id: 100,
        font: None,
        ..Project::default()
    };
    p.definitions = vec![
        Definition {
            id: 10,
            name: "Button".into(),
            content: Content {
                nodes: vec![Node::control(
                    11,
                    "button",
                    Point::default(),
                    Size::fixed(240., 120.),
                )],
                ..Default::default()
            },
        },
        Definition {
            id: 20,
            name: "Parent".into(),
            content: Content {
                nodes: vec![Node::group(
                    21,
                    Point::new(10., 20.),
                    vec![instance(22, 10)],
                )],
                ..Default::default()
            },
        },
        Definition {
            id: 30,
            name: "Grandparent".into(),
            content: Content {
                nodes: vec![instance(31, 20)],
                ..Default::default()
            },
        },
        Definition {
            id: 40,
            name: "Button".into(),
            content: Content::default(),
        },
    ];
    p.organize_components().unwrap();
    p.validate().unwrap();
    Editor::new(p)
}
fn enabled(e: &Editor, id: Id) -> bool {
    e.catalog_entries()
        .iter()
        .find(|entry| entry.item == CatalogItem::Component(id))
        .unwrap()
        .enabled
}
fn frozen() -> wireframe::tablet::Clock {
    wireframe::tablet::Clock::new(9, 52).unwrap()
}
fn fresh_scene(p: &Project, id: Id) -> String {
    let mut e = Editor::with_clock(p.clone(), frozen);
    e.document.pages[0].content = Content::default();
    e.insert_component(id, 0., 0.).unwrap();
    e.rendered_scene(e.page).unwrap().svg()
}

#[test]
fn catalog_identity_survives_duplicate_names_and_builtin_collisions() {
    let e = fixture();
    let entries = e.search_catalog("button");
    let exact: Vec<_> = entries
        .iter()
        .filter(|entry| entry.name == "Button")
        .map(|entry| entry.item)
        .collect();
    assert_eq!(
        exact,
        vec![
            CatalogItem::Control(Kind::Button),
            CatalogItem::Component(10),
            CatalogItem::Component(40)
        ]
    );
    assert_eq!(
        e.catalog_entries().len(),
        Kind::ALL.len() + wireframe::workspace::CatalogPreset::ALL.len() + 4
    );
    let lower: Vec<_> = e
        .catalog_entries()
        .iter()
        .map(|entry| entry.name.to_lowercase())
        .collect();
    assert!(lower.windows(2).all(|pair| pair[0] <= pair[1]));
}

#[test]
fn search_preserves_builtin_alias_matches_and_excludes_disabled_components() {
    let mut e = fixture();
    for query in [
        "",
        "button",
        "btn",
        "radio",
        "iphone",
        "text input",
        "horizontal rule",
        "line",
        "window",
        "panel",
        "ellipse",
        "shape",
        "zzzzzzz",
    ] {
        let expected: Vec<_> = wireframe::catalog::search(query)
            .iter()
            .filter_map(|spec| {
                Kind::ALL
                    .into_iter()
                    .find(|k| k.catalog_id() == spec.id)
                    // These canonical catalog entries use legacy document kinds.
                    // Neither has a separate canonical Kind to prefer instead.
                    .or(match spec.id {
                        "horizontal_rule" => Some(Kind::Line),
                        "window" => Some(Kind::Panel),
                        _ => None,
                    })
            })
            .collect();
        let actual: Vec<_> = e
            .search_catalog(query)
            .iter()
            .filter_map(|entry| match entry.item {
                CatalogItem::Control(k) => Some(k),
                _ => None,
            })
            .collect();
        // Display labels can introduce valid new matches, while established
        // alias results must retain their membership and relative ranking.
        let retained: Vec<_> = actual
            .iter()
            .copied()
            .filter(|kind| expected.contains(kind))
            .collect();
        assert_eq!(retained, expected, "{query}");
        for kind in actual.iter().filter(|kind| !expected.contains(kind)) {
            let display = kind.catalog_name().to_lowercase();
            assert!(
                query.split_whitespace().all(|word| display.contains(word)),
                "{query}: {kind:?}"
            );
        }
    }
    e.edit_library_source(10).unwrap();
    assert!(
        e.search_catalog("")
            .iter()
            .all(|entry| !matches!(entry.item, CatalogItem::Component(10 | 20 | 30)))
    );
    assert!(
        e.search_catalog("  bUtToN  ")
            .iter()
            .any(|entry| entry.item == CatalogItem::Component(40))
    );
}

#[test]
fn source_self_and_transitive_dependents_are_disabled_without_hiding_them() {
    let mut e = fixture();
    assert!([10, 20, 30, 40].into_iter().all(|id| enabled(&e, id)));
    e.edit_library_source(10).unwrap();
    assert!([10, 20, 30].into_iter().all(|id| !enabled(&e, id)));
    assert!(enabled(&e, 40));
    e.edit_library_source(20).unwrap();
    assert!(enabled(&e, 10));
    assert!(!enabled(&e, 20));
    assert!(!enabled(&e, 30));
    e.edit_library_source(40).unwrap();
    assert!(enabled(&e, 10)); // Equal display names do not imply recursion.
}

#[test]
fn whole_library_and_instance_context_have_distinct_insertion_rules() {
    let mut e = fixture();
    e.edit_library(e.document.default_library.unwrap()).unwrap();
    assert!([10, 20, 30, 40].into_iter().all(|id| !enabled(&e, id)));
    assert!(
        e.catalog_entries()
            .iter()
            .filter(|entry| matches!(entry.item, CatalogItem::Control(_)))
            .all(|entry| entry.enabled)
    );
    let before = e.document.clone();
    assert!(e.insert_catalog_component(10, 50., 50.).is_err());
    assert_eq!(e.document, before);
    e.switch_page(e.page);
    let root = e.insert_catalog_component(20, 50., 50.).unwrap();
    assert!(e.enter(root));
    assert!(e.catalog_entries().iter().all(|entry| !entry.enabled));
    assert!(e.insert_catalog_component(10, 50., 50.).is_err());
}

#[test]
fn rejection_preserves_pending_edits_ids_selection_and_redo() {
    let mut e = fixture();
    e.edit_library_source(10).unwrap();
    e.begin("Pending label");
    e.set_control_property(11, "text", Value::Text("Uncommitted text".into()))
        .unwrap();
    let before = e.document.clone();
    let selection = e.selection.clone();
    let undo = e.undo_name().map(str::to_owned);
    let redo = e.redo_name().map(str::to_owned);
    for id in [10, 20, 30, 999] {
        assert!(e.insert_catalog_component(id, 0., 0.).is_err());
        assert_eq!(e.document, before);
        assert_eq!(e.selection, selection);
        assert_eq!(e.undo_name(), undo.as_deref());
        assert_eq!(e.redo_name(), redo.as_deref());
    }
    e.commit();
    e.undo();
    assert_ne!(e.document, before);
    e.redo();
    assert_eq!(e.document, before);
}

#[test]
fn page_and_nested_source_insertions_have_one_step_history_and_roundtrip() {
    let mut e = fixture();
    for source in [None, Some(20)] {
        if let Some(source) = source {
            e.edit_library_source(source).unwrap();
            assert!(e.enter(21));
        }
        let before = e.document.clone();
        let id = e.insert_catalog_component(10, 87., 96.).unwrap();
        assert_eq!(e.instance_definition(id), Some(10));
        let after = e.document.clone();
        assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
        e.undo();
        assert_eq!(e.document, before);
        e.redo();
        assert_eq!(e.document, after);
    }
}

#[test]
fn preview_matches_fresh_instances_for_nested_cropped_and_empty_sources() {
    let mut e = fixture();
    let owner = e.document.component_source(10).unwrap().1.node;
    let root = e.document.libraries[0]
        .content
        .nodes
        .iter_mut()
        .find(|n| n.id == owner)
        .unwrap();
    root.crop = Some(Crop {
        width: 0.5,
        height: 0.75,
        ..Crop::ALL
    });
    root.markup = true;
    let before = e.document.clone();
    let mut previews = ComponentPreviews::default();
    for id in [10, 20, 30, 40] {
        let actual = previews.scene_with_clock(&e.document, id, frozen).unwrap();
        assert_eq!(actual.svg(), fresh_scene(&e.document, id), "source {id}");
        assert!(actual.bounds.w > 0. && actual.bounds.h > 0.);
    }
    assert!(
        previews
            .scene(&e.document, 40)
            .unwrap()
            .svg()
            .contains("Empty component")
    );
    assert_eq!(e.document, before);
}

#[test]
fn preview_cache_reuses_unchanged_graph_and_refreshes_dependency_appearance_and_style() {
    let mut e = fixture();
    let mut previews = ComponentPreviews::default();
    let first = previews.scene(&e.document, 30).unwrap();
    e.document.pages[0].notes = "Unrelated notes".into();
    e.document
        .definitions
        .iter_mut()
        .find(|d| d.id == 40)
        .unwrap()
        .name = "Unrelated rename".into();
    assert!(Arc::ptr_eq(
        &first,
        &previews.scene(&e.document, 30).unwrap()
    ));
    e.document.definitions[0].content.nodes[0]
        .properties
        .insert("text".into(), Value::Text("Changed child".into()));
    let changed = previews.scene(&e.document, 30).unwrap();
    assert!(!Arc::ptr_eq(&first, &changed));
    assert_ne!(first.svg(), changed.svg());
    let owner = e.document.component_source(10).unwrap().1.node;
    e.document.libraries[0]
        .content
        .nodes
        .iter_mut()
        .find(|n| n.id == owner)
        .unwrap()
        .crop = Some(Crop {
        width: 0.5,
        ..Crop::ALL
    });
    let cropped = previews.scene(&e.document, 30).unwrap();
    assert_ne!(changed.svg(), cropped.svg());
    e.document.settings.sketch = false;
    let clean = previews.scene(&e.document, 30).unwrap();
    assert_ne!(cropped.svg(), clean.svg());
    e.document.font = Some(wireframe::fonts::default_project_font());
    let font = previews.scene(&e.document, 30).unwrap();
    assert!(!Arc::ptr_eq(&clean, &font));
    assert_eq!(font.svg(), fresh_scene(&e.document, 30));
}

#[test]
fn removed_and_restored_dependencies_refresh_without_rebinding_source_identity() {
    let mut e = fixture();
    let mut previews = ComponentPreviews::default();
    let before = previews.scene(&e.document, 30).unwrap();
    e.delete_definitions(&[10]).unwrap();
    assert!(previews.scene(&e.document, 10).is_err());
    assert!(
        !e.catalog_entries()
            .iter()
            .any(|entry| entry.item == CatalogItem::Component(10))
    );
    let missing = previews.scene(&e.document, 30).unwrap();
    assert_ne!(before.svg(), missing.svg());
    assert_eq!(missing.svg(), fresh_scene(&e.document, 30));
    e.undo();
    let restored = previews.scene(&e.document, 30).unwrap();
    assert_eq!(before.svg(), restored.svg());
    let library = e.document.default_library.unwrap();
    e.trash_libraries(&[library]).unwrap();
    assert!(previews.scene(&e.document, 30).is_err());
    e.restore_libraries(&[library]).unwrap();
    assert_eq!(before.svg(), previews.scene(&e.document, 30).unwrap().svg());
}

#[test]
fn exhausted_identity_failure_does_not_commit_a_pending_edit() {
    let mut e = fixture();
    e.document.next_id = u64::MAX;
    e.begin("Pending title");
    e.document.title = "Still editing".into();
    let before = e.document.clone();
    let undo = e.undo_name().map(str::to_owned);
    assert!(e.insert_catalog_component(10, 0., 0.).is_err());
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), undo.as_deref());
    e.cancel();
    assert_eq!(e.document.title, "Untitled project");
}

#[test]
fn source_lock_allows_insertion_but_locked_nested_group_does_not() {
    let mut e = fixture();
    let library = e.document.default_library.unwrap();
    let owner = e.document.component_source(20).unwrap().1.node;
    e.edit_library(library).unwrap();
    e.select(owner, false);
    e.lock(true);
    e.edit_library_source(20).unwrap();
    assert!(enabled(&e, 10));
    e.insert_catalog_component(10, 500., 500.).unwrap();
    e.switch_page(e.page);
    e.add(Kind::Button, 0., 0.);
    e.add(Kind::Button, 200., 0.);
    e.select_all();
    e.group();
    let group = *e.selection.iter().next().unwrap();
    assert!(e.enter(group));
    // The containing group's lock may change while its context remains open.
    e.document.pages[0]
        .content
        .nodes
        .iter_mut()
        .find(|n| n.id == group)
        .unwrap()
        .locked = true;
    let before = e.document.clone();
    assert!(!enabled(&e, 10));
    assert!(e.insert_catalog_component(10, 50., 50.).is_err());
    assert_eq!(e.document, before);
}

#[test]
fn asset_bytes_and_nested_override_assets_refresh_preview_but_unrelated_assets_do_not() {
    fn png(color: [u8; 4]) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba(color));
        let mut out = std::io::Cursor::new(vec![]);
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }
    let mut e = fixture();
    let asset = e.document.allocate_id().unwrap();
    let unrelated = e.document.allocate_id().unwrap();
    e.document.assets = vec![
        Asset {
            trashed: false,
            id: asset,
            name: "Image".into(),
            media_type: "image/png".into(),
            bytes: png([255, 0, 0, 255]),
        },
        Asset {
            trashed: false,
            id: unrelated,
            name: "Unused".into(),
            media_type: "image/png".into(),
            bytes: png([0, 0, 255, 255]),
        },
    ];
    let image = &mut e.document.definitions[0].content.nodes[0];
    image.content = NodeContent::Control {
        kind: "image".into(),
        size: Size::fixed(40., 40.),
    };
    image.properties.insert("image".into(), Value::Asset(asset));
    let mut previews = ComponentPreviews::default();
    let first = previews.scene(&e.document, 30).unwrap();
    assert_eq!(first.svg(), fresh_scene(&e.document, 30));
    e.document.assets[1].bytes = png([0, 255, 0, 255]);
    assert!(Arc::ptr_eq(
        &first,
        &previews.scene(&e.document, 30).unwrap()
    ));
    e.document.assets[0].bytes = png([255, 255, 0, 255]);
    let changed = previews.scene(&e.document, 30).unwrap();
    assert!(first.png().unwrap() != changed.png().unwrap());
    assert_eq!(changed.svg(), fresh_scene(&e.document, 30));
    // An outer instance override must pull in an asset absent from its source.
    let NodeContent::Group { children } = &mut e.document.definitions[1].content.nodes[0].content
    else {
        panic!()
    };
    let NodeContent::Instance { overrides, .. } = &mut children[0].content else {
        panic!()
    };
    overrides.push(Override {
        path: vec![11],
        patch: Patch {
            properties: Properties::from([("image".into(), Value::Asset(unrelated))]),
            ..Default::default()
        },
    });
    let overridden = previews.scene(&e.document, 30).unwrap();
    assert!(changed.png().unwrap() != overridden.png().unwrap());
    assert_eq!(overridden.svg(), fresh_scene(&e.document, 30));
}

#[test]
fn cache_eviction_and_document_switch_do_not_return_another_sources_artwork() {
    let mut e = fixture();
    let mut previews = ComponentPreviews::default();
    for n in 0..34 {
        let id = e.document.allocate_id().unwrap();
        e.document.definitions.push(Definition {
            id,
            name: format!("Empty {n}"),
            content: Content::default(),
        });
    }
    e.document.organize_components().unwrap();
    let first = previews.scene(&e.document, 10).unwrap();
    for definition in &e.document.definitions {
        previews.scene(&e.document, definition.id).unwrap();
    }
    let evicted = previews.scene(&e.document, 10).unwrap();
    assert!(!Arc::ptr_eq(&first, &evicted));
    assert_eq!(evicted.svg(), fresh_scene(&e.document, 10));
    let mut other = fixture();
    other.document.definitions[0].content.nodes[0]
        .properties
        .insert("text".into(), Value::Text("Other project".into()));
    let switched = previews.scene(&other.document, 10).unwrap();
    assert_ne!(evicted.svg(), switched.svg());
    assert_eq!(switched.svg(), fresh_scene(&other.document, 10));
}

#[test]
fn combined_categories_deduplicate_by_identity_and_keep_disabled_source_entries() {
    let mut e = fixture();
    let categories = [
        CatalogCategory::Buttons,
        CatalogCategory::Common,
        CatalogCategory::Components,
        CatalogCategory::Buttons,
    ];
    let entries = e.catalog_categories(&categories);
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry.item == CatalogItem::Control(Kind::Button))
            .count(),
        1
    );
    assert_eq!(
        entries
            .iter()
            .filter(|entry| matches!(entry.item, CatalogItem::Component(_)))
            .count(),
        4
    );
    for (index, entry) in entries.iter().enumerate() {
        assert!(
            !entries[..index]
                .iter()
                .any(|earlier| earlier.item == entry.item)
        );
    }
    assert_eq!(
        e.catalog_categories(&[]),
        e.catalog_categories(&[CatalogCategory::All, CatalogCategory::Components])
    );
    e.edit_library_source(10).unwrap();
    let entries = e.catalog_categories(&[CatalogCategory::Components]);
    assert_eq!(entries.len(), 4);
    assert_eq!(entries.iter().filter(|entry| entry.enabled).count(), 1);
}

#[test]
fn retained_bridge_preserves_cycle_guards_and_renders_missing_dependency() {
    let mut e = fixture();
    let library = e.document.allocate_id().unwrap();
    let source = e.document.libraries[0]
        .sources
        .iter()
        .find(|s| s.definition == 20)
        .unwrap()
        .clone();
    let node = e.document.libraries[0]
        .content
        .nodes
        .iter()
        .find(|n| n.id == source.node)
        .unwrap()
        .clone();
    e.document.libraries[0]
        .sources
        .retain(|s| s.definition != 20);
    e.document.libraries[0]
        .content
        .nodes
        .retain(|n| n.id != source.node);
    e.document.libraries.push(ComponentLibrary {
        id: library,
        trashed: true,
        name: "Retained bridge".into(),
        notes: String::new(),
        content: Content {
            nodes: vec![node],
            ..Default::default()
        },
        sources: vec![source],
        imports: vec![],
    });
    e.document.validate().unwrap();
    e.edit_library_source(10).unwrap();
    assert!(!enabled(&e, 30));
    assert!(
        !e.catalog_entries()
            .iter()
            .any(|entry| entry.item == CatalogItem::Component(20))
    );
    assert!(e.insert_catalog_component(30, 0., 0.).is_err());
    let mut previews = ComponentPreviews::default();
    let missing = previews.scene(&e.document, 30).unwrap();
    assert_eq!(missing.svg(), fresh_scene(&e.document, 30));
    e.restore_libraries(&[library]).unwrap();
    let restored = previews.scene(&e.document, 30).unwrap();
    assert_ne!(restored.svg(), missing.svg());
    assert_eq!(restored.svg(), fresh_scene(&e.document, 30));
}

#[test]
fn preview_uses_no_additional_canonical_ids_beyond_a_fresh_instance() {
    let mut e = fixture();
    e.document.next_id = u64::MAX - 2;
    e.document.validate().unwrap();
    let before = e.document.clone();
    let mut previews = ComponentPreviews::default();
    assert_eq!(
        previews.scene(&e.document, 10).unwrap().svg(),
        fresh_scene(&e.document, 10)
    );
    assert_eq!(e.document, before);
}
