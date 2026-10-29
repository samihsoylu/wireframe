// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::{
    collections::{BTreeMap, HashSet},
    io::Cursor,
};
use wireframe::{
    document::*,
    render::Drawing,
    storage::ProjectStore,
    working_copy::WorkingCopies,
    workspace::{
        AssetImportAction, AssetImportChoice, AssetPreviews, CatalogItem, Editor, ImportChoice,
    },
};
fn fixture() -> Project {
    let pixels = image::RgbaImage::from_pixel(120, 80, image::Rgba([200, 50, 20, 255]));
    let mut bytes = Cursor::new(vec![]);
    pixels
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let mut p = Project {
        next_id: 100,
        font: None,
        ..Project::default()
    };
    p.assets.push(Asset {
        attribution: None,
        id: 10,
        trashed: false,
        name: "Photo.png".into(),
        media_type: "image/png".into(),
        bytes: bytes.into_inner(),
    });
    for (id, kind, size) in [
        (20, "image", Size::default()),
        (21, "image", Size::fixed(250., 170.)),
        (22, "cover_flow", Size::fixed(300., 150.)),
        (23, "icon", Size::fixed(32., 32.)),
    ] {
        let mut node = Node::control(id, kind, Point::new((id - 20) as f32 * 350., 0.), size);
        if kind == "icon" {
            node.properties
                .insert("icon".into(), Value::Icon(IconSource::Asset(10)));
        } else {
            node.properties.insert("image".into(), Value::Asset(10));
        }
        p.pages[0].content.nodes.push(node);
    }
    p.validate().unwrap();
    p
}
fn image_count(e: &Editor, page: Id) -> usize {
    e.rendered_scene(page)
        .unwrap()
        .items
        .iter()
        .filter(|item| matches!(item.drawing, Drawing::Image { .. }))
        .count()
}
#[test]
fn retained_image_keeps_references_bytes_geometry_and_restores_artwork() {
    for sketch in [false, true] {
        let mut p = fixture();
        p.settings.sketch = sketch;
        let mut e = Editor::new(p);
        let before = e.document.clone();
        let count = image_count(&e, e.page);
        assert!(count >= 4);
        let mut previews = AssetPreviews::default();
        previews.scene(&e.document, 10).unwrap();
        e.trash_assets(&[10]).unwrap();
        let trashed = e.document.clone();
        assert!(trashed.assets[0].trashed);
        assert_eq!(trashed.assets[0].bytes, before.assets[0].bytes);
        assert_eq!(trashed.pages, before.pages);
        assert_eq!(trashed.next_id, before.next_id);
        assert!(e.document.live_asset(10).is_none());
        assert!(
            !e.catalog_entries()
                .iter()
                .any(|entry| entry.item == CatalogItem::Asset(10))
        );
        assert!(previews.scene(&e.document, 10).is_err());
        assert_eq!(image_count(&e, e.page), 0);
        let controls = e.current().controls;
        let auto = controls.iter().find(|c| c.id == 20).unwrap();
        let fixed = controls.iter().find(|c| c.id == 21).unwrap();
        assert_eq!((auto.bounds.w, auto.bounds.h), (77., 79.));
        assert_eq!((fixed.bounds.w, fixed.bounds.h), (250., 170.));
        for _ in 0..2 {
            e.undo();
            assert_eq!(e.document, before);
            e.redo();
            assert_eq!(e.document, trashed);
        }
        e.restore_assets(&[10]).unwrap();
        assert_eq!(e.document, before);
        assert_eq!(image_count(&e, e.page), count);
        previews.scene(&e.document, 10).unwrap();
        e.undo();
        assert_eq!(e.document, trashed);
        e.redo();
        assert_eq!(e.document, before);
    }
}
#[test]
fn stale_and_mixed_selections_preserve_pending_edits_and_history() {
    for ids in [vec![], vec![10, 10], vec![10, 999], vec![999]] {
        let mut e = Editor::new(fixture());
        e.begin("Pending title");
        e.document.title = "Still typing".into();
        let before = e.document.clone();
        assert!(e.trash_assets(&ids).is_err());
        assert_eq!(e.document, before);
        assert_eq!(e.undo_name(), None);
        e.cancel();
        assert_eq!(e.document.title, "Untitled project");
    }
    let mut e = Editor::new(fixture());
    e.trash_assets(&[10]).unwrap();
    e.begin("Pending title");
    e.document.title = "Still typing".into();
    let before = e.document.clone();
    assert!(e.trash_assets(&[10]).is_err());
    assert!(e.restore_assets(&[10, 999]).is_err());
    assert!(e.insert_catalog_asset(10, 0., 0.).is_err());
    assert!(e.assign_image(20, Some(10)).is_err());
    assert_eq!(e.document, before);
    e.cancel();
    assert_eq!(e.document.title, "Untitled project");
}
#[test]
fn removal_does_not_commit_or_lose_an_unrelated_pending_edit_on_serialization_failure() {
    let mut e = Editor::new(fixture());
    e.begin("Pending title");
    e.document.title = "Still typing".into();
    e.document.assets[0].bytes = vec![0; MAX_FILE_BYTES];
    let before = e.document.clone();
    assert!(e.trash_assets(&[10]).is_err());
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), None);
    e.cancel();
    assert_eq!(e.document, fixture());
}
#[test]
fn recoverable_asset_survives_saved_file_and_working_copy_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Image project.wireframe");
    let mut e = Editor::new(fixture());
    e.trash_assets(&[10]).unwrap();
    ProjectStore.save(&path, &e.document).unwrap();
    let copies = WorkingCopies::new(dir.path().join("copies")).unwrap();
    let mut copy = copies.open_named(&path).unwrap();
    let id = copy.id().to_string();
    assert_eq!(copy.project(), &e.document);
    e.restore_assets(&[10]).unwrap();
    copy.checkpoint(&e.document).unwrap();
    drop(copy);
    let copy = copies.resume(&id).unwrap();
    assert_eq!(copy.project(), &e.document);
    assert!(copy.has_local_changes().unwrap());
    let saved = ProjectStore.load(&path).unwrap();
    assert!(saved.assets[0].trashed);
    let mut restored = Editor::new(saved);
    restored.restore_assets(&[10]).unwrap();
    assert_eq!(restored.document, e.document);
}
#[test]
fn old_documents_migrate_without_retention_and_cannot_smuggle_new_state() {
    let p = fixture();
    let mut old = serde_json::to_value(&p).unwrap();
    assert!(old["assets"][0].get("trashed").is_none());
    old["format_version"] = 14.into();
    assert_eq!(decode(&serde_json::to_vec(&old).unwrap()).unwrap(), p);
    old["assets"][0]["trashed"] = true.into();
    for version in 1..15 {
        old["format_version"] = version.into();
        assert!(
            decode(&serde_json::to_vec(&old).unwrap())
                .unwrap_err()
                .contains("version 15")
        );
    }
    old["format_version"] = 15.into();
    assert!(decode(&serde_json::to_vec(&old).unwrap()).unwrap().assets[0].trashed);
}
#[test]
fn imported_file_does_not_reuse_or_replace_hidden_asset_implicitly() {
    let mut e = Editor::new(fixture());
    e.trash_assets(&[10]).unwrap();
    let prepared = wireframe::images::Prepared {
        attribution: None,
        name: "Photo.png".into(),
        media_type: "image/png".into(),
        bytes: e.document.assets[0].bytes.clone(),
    };
    assert!(e.image_conflict(&prepared).is_none());
    let id = e
        .import_image(20, prepared, ImportChoice::UseExisting)
        .unwrap();
    assert_ne!(id, 10);
    assert!(e.document.assets[0].trashed);
    assert!(e.document.live_asset(id).is_some());
    e.undo();
    assert!(e.document.assets[0].trashed);
    assert_eq!(e.document.assets.len(), 1);
}
#[test]
fn cross_project_clipboard_preserves_hidden_dependencies_without_resurrecting_them() {
    let mut source = Editor::new(fixture());
    source.trash_assets(&[10]).unwrap();
    source.selection = HashSet::from([20, 21, 22, 23]);
    let copy = source.copy();
    assert!(copy.assets[0].trashed);
    let mut destination = Editor::new(fixture());
    let before = destination.document.clone();
    destination.paste(&copy, 40.);
    assert!(destination.error.is_none(), "{:?}", destination.error);
    assert_eq!(destination.document.assets.len(), 2);
    assert_eq!(destination.document.live_assets().count(), 1);
    let imported = destination
        .document
        .assets
        .iter()
        .find(|a| a.trashed)
        .unwrap()
        .id;
    assert_ne!(imported, 10);
    let count = image_count(&destination, destination.page);
    destination.restore_assets(&[imported]).unwrap();
    assert!(image_count(&destination, destination.page) > count);
    destination.undo();
    destination.undo();
    assert_eq!(destination.document, before);
}
#[test]
fn automatic_project_copy_distinguishes_equal_bytes_with_different_availability() {
    let source = fixture();
    let mut target = Editor::new(fixture());
    target.trash_assets(&[10]).unwrap();
    let prepared = target
        .prepare_project_copy(&source, "source", &[1])
        .unwrap();
    assert_eq!(prepared.counts().assets_added, 1);
    assert_eq!(prepared.counts().assets_reused, 0);
    let imported = prepared.imported_pages()[0];
    target.apply_project_copy(prepared).unwrap();
    assert_eq!(target.document.assets.len(), 2);
    assert_eq!(image_count(&target, 1), 0);
    assert!(image_count(&target, imported) > 0);
}
#[test]
fn reviewed_import_surfaces_availability_conflicts_and_applies_explicit_choice() {
    for choice in [
        AssetImportChoice::UseExisting,
        AssetImportChoice::UseNew,
        AssetImportChoice::KeepBoth,
    ] {
        let source = fixture();
        let mut target = Editor::new(fixture());
        target.trash_assets(&[10]).unwrap();
        target.document.pages[0].name = "Destination".into();
        let before = target.document.clone();
        let plan = target
            .plan_project_page_import(&source, "source", &[1])
            .unwrap();
        assert_eq!(plan.asset_entries()[0].action, AssetImportAction::Conflict);
        let prepared = plan
            .prepare_with_assets(&BTreeMap::new(), &BTreeMap::from([(10, choice)]))
            .unwrap();
        target.apply_project_copy(prepared).unwrap();
        assert_eq!(
            target.document.assets[0].trashed,
            choice != AssetImportChoice::UseNew
        );
        assert_eq!(
            target.document.assets.len(),
            if choice == AssetImportChoice::KeepBoth {
                2
            } else {
                1
            }
        );
        target.undo();
        assert_eq!(target.document, before);
    }
}

#[test]
fn component_previews_and_library_transfer_preserve_retained_asset_state() {
    let mut source = Editor::new(fixture());
    source.selection = HashSet::from([20]);
    let definition = source.create_component("Photo".into()).unwrap();
    let mut previews = wireframe::workspace::ComponentPreviews::default();
    let live = previews.scene(&source.document, definition).unwrap();
    assert!(
        live.items
            .iter()
            .any(|item| matches!(item.drawing, Drawing::Image { .. }))
    );
    source.trash_assets(&[10]).unwrap();
    let retained = previews.scene(&source.document, definition).unwrap();
    assert!(
        !retained
            .items
            .iter()
            .any(|item| matches!(item.drawing, Drawing::Image { .. }))
    );
    let library = source.document.libraries[0].id;
    let copy = source.copy_libraries(&[library], "source").unwrap();
    let mut target = Editor::new(fixture());
    let before = target.document.clone();
    let plan = target.plan_library_import(&copy, "target").unwrap();
    target
        .apply_library_import(plan.prepare(&BTreeMap::new()).unwrap(), "target")
        .unwrap();
    assert_eq!(target.document.assets.len(), 2);
    assert_eq!(target.document.live_assets().count(), 1);
    let imported = target
        .document
        .assets
        .iter()
        .find(|a| a.trashed)
        .unwrap()
        .id;
    assert_ne!(imported, 10);
    let imported_definition = target.document.definitions[0].id;
    assert!(
        !previews
            .scene(&target.document, imported_definition)
            .unwrap()
            .items
            .iter()
            .any(|item| matches!(item.drawing, Drawing::Image { .. }))
    );
    target.restore_assets(&[imported]).unwrap();
    assert!(
        previews
            .scene(&target.document, imported_definition)
            .unwrap()
            .items
            .iter()
            .any(|item| matches!(item.drawing, Drawing::Image { .. }))
    );
    target.undo();
    target.undo();
    assert_eq!(target.document, before);
}
