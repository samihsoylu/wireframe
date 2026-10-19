// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::{io::Cursor, sync::Arc};
use wireframe::{
    document::*,
    workspace::{AssetPreviews, Editor},
};
fn png(w: u32, h: u32, color: [u8; 4]) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(w, h, image::Rgba(color));
    let mut bytes = Cursor::new(vec![]);
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
}
fn fixture() -> Project {
    Project {
        next_id: 20,
        assets: vec![Asset {
            trashed: false,
            id: 10,
            name: "old-size-120x80.png".into(),
            media_type: "image/png".into(),
            bytes: png(80, 120, [240, 120, 30, 255]),
        }],
        ..Project::default()
    }
}
#[test]
fn fresh_asset_insertion_preserves_shared_bytes_and_natural_size_with_one_step_undo() {
    let mut p = fixture();
    let mut existing = Node::control(11, "image", Point::new(300., 200.), Size::fixed(31., 45.));
    existing.crop = Some(Crop {
        x: 0.1,
        y: 0.2,
        width: 0.5,
        height: 0.5,
    });
    existing.properties.insert("image".into(), Value::Asset(10));
    existing
        .properties
        .insert("rotation".into(), Value::Number(90.));
    p.pages[0].content.nodes.push(existing);
    let before = p.clone();
    let mut editor = Editor::new(p);
    let id = editor.insert_catalog_asset(10, 50., 60.).unwrap();
    assert_eq!(editor.document.assets, before.assets);
    assert_eq!(
        editor.document.pages[0].content.nodes[0],
        before.pages[0].content.nodes[0]
    );
    let node = editor.document.pages[0].content.nodes.last().unwrap();
    assert_eq!(node.crop, None);
    assert_eq!(node.origin, Point::new(50., 60.));
    assert_eq!(node.properties.get("image"), Some(&Value::Asset(10)));
    assert_ne!(node.properties.get("rotation"), Some(&Value::Number(90.)));
    let control = editor
        .current()
        .controls
        .into_iter()
        .find(|c| c.id == id)
        .unwrap();
    assert_eq!((control.bounds.w, control.bounds.h), (80., 120.));
    assert_eq!(editor.undo_name(), Some("Add image"));
    let saved = editor.document.clone();
    assert_eq!(decode(&saved.encode().unwrap()).unwrap(), saved);
    editor.undo();
    assert_eq!(editor.document, before);
    editor.redo();
    assert_eq!(editor.document, saved);
}
#[test]
fn stale_corrupt_and_exhausted_insertions_leave_document_and_history_unchanged() {
    for mode in 0..4 {
        let mut p = fixture();
        if mode == 1 {
            p.assets[0].bytes = vec![1, 2, 3];
        }
        if mode == 2 {
            p.next_id = Id::MAX;
        }
        let mut editor = Editor::new(p);
        let before = editor.document.clone();
        let result = editor.insert_catalog_asset(
            if mode == 0 { 99 } else { 10 },
            if mode == 3 { f32::NAN } else { 0. },
            0.,
        );
        assert!(result.is_err(), "mode {mode}");
        assert_eq!(editor.document, before);
        assert_eq!(editor.undo_name(), None);
        assert_eq!(editor.redo_name(), None);
    }
}
#[test]
fn nested_insertion_uses_workspace_coordinates_and_rejects_locked_context() {
    let mut p = fixture();
    p.pages[0]
        .content
        .nodes
        .push(Node::group(11, Point::new(40., 70.), vec![]));
    let mut editor = Editor::new(p);
    editor.context = vec![11];
    editor.insert_catalog_asset(10, 90., 110.).unwrap();
    let group = &editor.document.pages[0].content.nodes[0];
    let NodeContent::Group { children, .. } = &group.content else {
        panic!()
    };
    assert_eq!(children[0].origin, Point::new(50., 40.));
    editor.document.pages[0].content.nodes[0].locked = true;
    let before = editor.document.clone();
    assert!(editor.insert_catalog_asset(10, 90., 110.).is_err());
    assert_eq!(editor.document, before);
}
#[test]
fn preview_matches_fresh_insertion_and_reuses_unaffected_artwork() {
    for sketch in [false, true] {
        let mut p = fixture();
        p.settings.sketch = sketch;
        let mut cache = AssetPreviews::default();
        let first = cache.scene(&p, 10).unwrap();
        let mut editor = Editor::new(p.clone());
        editor.insert_catalog_asset(10, 0., 0.).unwrap();
        assert_eq!(
            first.svg(),
            editor.rendered_scene(editor.page).unwrap().svg()
        );
        p.assets[0].name = "renamed".into();
        p.pages[0].notes = "unrelated".into();
        p.next_id += 100;
        assert!(Arc::ptr_eq(&first, &cache.scene(&p, 10).unwrap()));
        p.assets[0].bytes = png(90, 50, [20, 40, 80, 255]);
        let second = cache.scene(&p, 10).unwrap();
        assert!(!Arc::ptr_eq(&first, &second));
        assert_eq!((second.bounds.w, second.bounds.h), (90., 50.));
        p.assets[0].bytes = vec![0];
        assert!(cache.scene(&p, 10).is_err());
        p.assets.clear();
        assert!(cache.scene(&p, 10).is_err());
    }
}
#[test]
fn equal_names_do_not_share_preview_identity_and_settings_refresh() {
    let mut p = fixture();
    let mut another = p.assets[0].clone();
    another.id = 12;
    p.assets.push(another);
    let mut cache = AssetPreviews::default();
    let first = cache.scene(&p, 10).unwrap();
    let second = cache.scene(&p, 12).unwrap();
    assert!(!Arc::ptr_eq(&first, &second));
    p.settings.sketch = !p.settings.sketch;
    assert!(!Arc::ptr_eq(&first, &cache.scene(&p, 10).unwrap()));
}
#[test]
fn preview_eviction_keeps_at_most_thirty_two_asset_entries() {
    let mut p = fixture();
    p.next_id = 100;
    for id in 21..54 {
        let mut another = p.assets[0].clone();
        another.id = id;
        p.assets.push(another);
    }
    let mut cache = AssetPreviews::default();
    let old = cache.scene(&p, 10).unwrap();
    for id in 21..54 {
        cache.scene(&p, id).unwrap();
    }
    assert!(!Arc::ptr_eq(&old, &cache.scene(&p, 10).unwrap()));
}

#[test]
fn assets_category_and_quick_add_keep_duplicate_names_and_deduplicate_unions() {
    use wireframe::workspace::{CatalogCategory as C, CatalogItem as I};
    let mut p = fixture();
    let mut duplicate = p.assets[0].clone();
    duplicate.id = 12;
    p.assets.push(duplicate);
    let mut invalid = p.assets[0].clone();
    invalid.id = 13;
    invalid.bytes = vec![0];
    p.assets.push(invalid);
    let editor = Editor::new(p);
    let entries = editor.catalog_categories(&[C::Assets]);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, entries[1].name);
    assert_eq!(
        entries.iter().map(|e| e.item).collect::<Vec<_>>(),
        vec![I::Asset(10), I::Asset(12)]
    );
    let union = editor.catalog_categories(&[C::All, C::Assets]);
    assert_eq!(
        union
            .iter()
            .filter(|e| matches!(e.item, I::Asset(_)))
            .count(),
        2
    );
    assert_eq!(editor.search_catalog("old-size-120x80.png").len(), 2);
    assert!(editor.catalog_categories(&[C::Components]).is_empty());
    assert_eq!(
        serde_json::from_str::<C>(&serde_json::to_string(&C::Assets).unwrap()).unwrap(),
        C::Assets
    );
}

#[test]
fn exhausted_asset_insertion_does_not_commit_an_unrelated_pending_edit() {
    let mut p = fixture();
    p.next_id = Id::MAX;
    let mut editor = Editor::new(p);
    editor.begin("Pending title");
    editor.document.title = "Still editing".into();
    let before = editor.document.clone();
    assert!(editor.insert_catalog_asset(10, 0., 0.).is_err());
    assert_eq!(editor.document, before);
    assert_eq!(editor.undo_name(), None);
    editor.cancel();
    assert_eq!(editor.document.title, "Untitled project");
}
