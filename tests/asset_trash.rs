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
        AssetPreviews, ComponentPreviews, Editor, LibraryImportChoice, PageImportChoice,
        RetainedResource as R,
    },
};
fn fixture() -> Editor {
    let mut bytes = Cursor::new(vec![]);
    image::RgbaImage::from_pixel(12, 8, image::Rgba([200, 20, 60, 255]))
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
        name: "Private artwork.png".into(),
        media_type: "image/png".into(),
        bytes: bytes.into_inner(),
    });
    for (id, kind) in [(20, "image"), (21, "icon"), (22, "list")] {
        let mut n = Node::control(id, kind, Point::new(50. * id as f32, 0.), Size::default());
        let (key, value) = match kind {
            "icon" => ("icon", Value::Icon(IconSource::Asset(10))),
            "list" => ("icons", Value::Icons(vec![Some(IconSource::Asset(10))])),
            _ => ("image", Value::Asset(10)),
        };
        n.properties.insert(key.into(), value);
        p.pages[0].content.nodes.push(n);
    }
    Editor::new(p)
}
fn deleted() -> Editor {
    let mut e = fixture();
    e.trash_assets(&[10]).unwrap();
    e.permanently_remove_resources(&[R::Asset(10)]).unwrap();
    e
}
fn assert_deleted(e: &Editor) {
    assert!(e.document.assets.is_empty());
    assert!(e.document.removed_assets.contains(&10));
    assert!(!e.retained_resources().any(|(r, _)| r == R::Asset(10)));
    assert!(e.document.next_id > 10);
    e.document.validate().unwrap();
    assert!(
        !e.rendered_scene(e.page)
            .unwrap()
            .items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Image { .. }))
    );
}
#[test]
fn mixed_restore_preserves_view_and_restores_all_three_resource_types_in_one_step() {
    let mut e = fixture();
    e.selection = HashSet::from([20]);
    let source = e.create_component("Photo".into()).unwrap();
    let library = e.document.component_source(source).unwrap().0.id;
    e.add_page();
    let active = e.page;
    e.trash_pages(&[1]).unwrap();
    e.trash_libraries(&[library]).unwrap();
    e.trash_assets(&[10]).unwrap();
    let resources: Vec<_> = e.retained_resources().map(|(r, _)| r).collect();
    assert_eq!(
        resources,
        vec![R::Page(1), R::Asset(10), R::Library(library)]
    );
    let before = e.document.clone();
    for bad in [
        vec![R::Asset(1)],
        vec![R::Page(10)],
        vec![R::Asset(10), R::Asset(10)],
    ] {
        assert!(e.restore_resources(&bad).is_err());
        assert_eq!(e.document, before);
    }
    e.restore_resources(&resources).unwrap();
    let after = e.document.clone();
    assert!(e.document.live_asset(10).is_some());
    assert!(e.document.definition_available(source));
    assert_eq!(e.page, active);
    assert_eq!(e.undo_name(), Some("Restore from Trash"));
    for _ in 0..2 {
        e.undo();
        assert_eq!(e.document, before);
        assert_eq!(e.page, active);
        e.redo();
        assert_eq!(e.document, after);
    }
}
#[test]
fn retained_preview_reveals_only_its_image_without_changing_live_availability() {
    let mut e = fixture();
    let mut previews = AssetPreviews::default();
    let live = previews.scene(&e.document, 10).unwrap().svg();
    assert!(previews.retained_scene(&e.document, 10).is_err());
    e.trash_assets(&[10]).unwrap();
    let before = e.document.clone();
    assert_eq!(
        previews.retained_scene(&e.document, 10).unwrap().svg(),
        live
    );
    assert!(previews.scene(&e.document, 10).is_err());
    assert_eq!(e.document, before);
    e.permanently_remove_resources(&[R::Asset(10)]).unwrap();
    assert!(previews.retained_scene(&e.document, 10).is_err());
}
#[test]
fn permanent_image_removal_preserves_consumers_and_cannot_resurrect_bytes_through_history() {
    let mut e = fixture();
    let nodes = e.document.pages[0].content.clone();
    e.rename_page(1, "Before removal").unwrap();
    e.begin("Pending note");
    e.document.pages[0].notes = "Keep this note".into();
    e.trash_assets(&[10]).unwrap();
    e.restore_resources(&[R::Asset(10)]).unwrap();
    e.undo();
    e.permanently_remove_resources(&[R::Asset(10)]).unwrap();
    assert_deleted(&e);
    assert_eq!(e.document.pages[0].content, nodes);
    assert!(e.restore_assets(&[10]).is_err());
    let encoded = String::from_utf8(e.document.encode().unwrap()).unwrap();
    assert!(!encoded.contains("Private artwork"));
    for _ in 0..8 {
        e.undo();
        assert_deleted(&e);
    }
    for _ in 0..8 {
        e.redo();
        assert_deleted(&e);
    }
}
#[test]
fn permanent_mixed_removal_keeps_a_live_page_and_consumes_obsolete_restore_history() {
    let mut e = fixture();
    e.selection = HashSet::from([20]);
    let d = e.create_component("Photo".into()).unwrap();
    let lib = e.document.component_source(d).unwrap().0.id;
    e.add_page();
    let survivor = e.page;
    e.trash_pages(&[1]).unwrap();
    e.trash_libraries(&[lib]).unwrap();
    e.trash_assets(&[10]).unwrap();
    let selected = vec![R::Page(1), R::Asset(10), R::Library(lib)];
    e.restore_resources(&selected).unwrap();
    e.undo();
    e.permanently_remove_resources(&selected).unwrap();
    assert_eq!(e.page, survivor);
    assert_deleted(&e);
    assert!(e.document.definitions.is_empty());
    for _ in 0..6 {
        e.undo();
        assert_deleted(&e);
        assert!(e.document.pages.iter().all(|p| p.id != 1));
        assert!(e.document.libraries.is_empty());
    }
}
#[test]
fn stale_permanent_selection_is_atomic_and_preserves_pending_edit() {
    let mut e = fixture();
    e.trash_assets(&[10]).unwrap();
    e.begin("Typing");
    e.document.title = "Unsaved".into();
    let before = e.document.clone();
    for bad in [
        vec![R::Asset(10), R::Asset(99)],
        vec![R::Asset(10), R::Page(10)],
        vec![R::Asset(10), R::Asset(10)],
    ] {
        assert!(e.permanently_remove_resources(&bad).is_err());
        assert_eq!(e.document, before);
    }
    e.cancel();
    assert_eq!(e.document.title, "Untitled project");
}
#[test]
fn missing_image_markers_are_reserved_bounded_and_reject_old_version_tags() {
    let e = deleted();
    let p = e.document;
    let json = serde_json::to_value(&p).unwrap();
    for version in 1..15 {
        let mut old = json.clone();
        old["format_version"] = version.into();
        assert!(decode(&serde_json::to_vec(&old).unwrap()).is_err());
    }
    for id in [0, 1, 100] {
        let mut invalid = p.clone();
        invalid.removed_assets.insert(id);
        assert!(invalid.validate().is_err(), "{id}");
    }
    let mut invalid = p.clone();
    invalid.assets = fixture().document.assets;
    assert!(invalid.validate().is_err());
    let mut invalid = p;
    invalid.asset_imports.insert(
        10,
        vec![AssetImport {
            project: "other".into(),
            asset: 2,
        }],
    );
    assert!(invalid.validate().is_err());
}
#[test]
fn deleted_asset_survives_file_and_recovery_round_trip_without_its_bytes() {
    let e = deleted();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Deleted artwork.wireframe");
    ProjectStore.save(&path, &e.document).unwrap();
    let copies = WorkingCopies::new(dir.path().join("copies")).unwrap();
    let copy = copies.open_named(&path).unwrap();
    let id = copy.id().to_owned();
    drop(copy);
    let recovered = copies.resume(&id).unwrap();
    assert_eq!(recovered.project(), &e.document);
    assert!(!recovered.has_local_changes().unwrap());
    assert_deleted(&Editor::new(ProjectStore.load(&path).unwrap()));
}
#[test]
fn clipboard_and_project_copy_remap_missing_images_away_from_colliding_live_assets() {
    let mut source = deleted();
    source.selection = HashSet::from([20, 21, 22]);
    let copy = source.copy();
    assert_eq!(copy.removed_assets.len(), 1);
    let mut target = fixture();
    let before = target.document.clone();
    target.paste(&copy, 40.);
    assert!(target.error.is_none(), "{:?}", target.error);
    assert_eq!(target.document.assets, before.assets);
    assert!(!target.document.removed_assets.contains(&10));
    assert_eq!(target.document.removed_assets.len(), 1);
    let last = &target.document.pages[0]
        .content
        .nodes
        .last()
        .unwrap()
        .properties["icons"];
    assert!(
        matches!(last,Value::Icons(v) if matches!(v[0],Some(IconSource::Asset(id)) if id!=10 && target.document.removed_assets.contains(&id)))
    );
    target.undo();
    assert_eq!(target.document, before);
    let prepared = target
        .prepare_project_copy(&source.document, "source", &[1])
        .unwrap();
    let page = prepared.imported_pages()[0];
    target.apply_project_copy(prepared).unwrap();
    assert!(!target.document.removed_assets.contains(&10));
    assert!(
        !target
            .rendered_scene(page)
            .unwrap()
            .items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Image { .. }))
    );
}
#[test]
fn component_preview_and_library_copy_preserve_deleted_dependencies() {
    let mut source = fixture();
    source.selection = HashSet::from([20]);
    let d = source.create_component("Photo".into()).unwrap();
    let lib = source.document.component_source(d).unwrap().0.id;
    source.trash_assets(&[10]).unwrap();
    source
        .permanently_remove_resources(&[R::Asset(10)])
        .unwrap();
    let mut previews = ComponentPreviews::default();
    assert!(
        !previews
            .scene(&source.document, d)
            .unwrap()
            .items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Image { .. }))
    );
    let copy = source.copy_libraries(&[lib], "source").unwrap();
    copy.library_resources().unwrap().preview_project().unwrap();
    let mut target = fixture();
    let plan = target.plan_library_import(&copy, "destination").unwrap();
    target
        .apply_library_import(plan.prepare(&BTreeMap::new()).unwrap(), "destination")
        .unwrap();
    assert_eq!(target.document.assets.len(), 1);
    assert_eq!(target.document.removed_assets.len(), 1);
    assert!(!target.document.removed_assets.contains(&10));
    target.document.validate().unwrap();
    let again = target.plan_library_import(&copy, "destination").unwrap();
    let choices = again
        .entries()
        .iter()
        .map(|e| (e.incoming, LibraryImportChoice::UseExisting))
        .collect();
    let before = target.document.clone();
    target
        .apply_library_import(again.prepare(&choices).unwrap(), "destination")
        .unwrap();
    assert_eq!(target.document, before);
}
#[test]
fn repeat_page_import_with_existing_decision_does_not_accumulate_missing_markers() {
    let source = deleted();
    let mut target = fixture();
    target.document.pages[0].name = "Destination".into();
    let plan = target
        .plan_project_page_import(&source.document, "source", &[1])
        .unwrap();
    target
        .apply_project_copy(plan.prepare(&BTreeMap::new()).unwrap())
        .unwrap();
    let before = target.document.clone();
    let plan = target
        .plan_project_page_import(&source.document, "source", &[1])
        .unwrap();
    target
        .apply_project_copy(
            plan.prepare(&BTreeMap::from([(1, PageImportChoice::UseExisting)]))
                .unwrap(),
        )
        .unwrap();
    assert_eq!(target.document, before);
}

#[test]
fn combined_page_library_import_keeps_missing_override_and_alternate_images_isolated() {
    let mut source = fixture();
    source.selection = HashSet::from([20]);
    let d = source.create_component("Photo".into()).unwrap();
    let child = source
        .document
        .definitions
        .iter()
        .find(|def| def.id == d)
        .unwrap()
        .content
        .nodes[0]
        .id;
    if let NodeContent::Instance { overrides, .. } =
        &mut source.document.pages[0].content.nodes[0].content
    {
        overrides.push(Override {
            path: vec![child],
            patch: Patch {
                properties: BTreeMap::from([("image".into(), Value::Asset(10))]),
                ..Patch::default()
            },
        });
    }
    source.create_version("Phone").unwrap();
    source.trash_assets(&[10]).unwrap();
    source
        .permanently_remove_resources(&[R::Asset(10)])
        .unwrap();
    let mut target = fixture();
    target.document.pages[0].name = "Destination".into();
    let plan = target
        .plan_project_page_import(&source.document, "source", &[1])
        .unwrap();
    let libraries = plan
        .review_libraries(&BTreeMap::new(), &BTreeMap::new())
        .unwrap();
    target
        .apply_project_copy(libraries.prepare(&BTreeMap::new()).unwrap())
        .unwrap();
    target.document.validate().unwrap();
    assert_eq!(target.document.assets.len(), 1);
    assert_eq!(target.document.removed_assets.len(), 1);
    assert!(!target.document.removed_assets.contains(&10));
    let before = target.document.clone();
    let plan = target
        .plan_project_page_import(&source.document, "source", &[1])
        .unwrap();
    let libraries = plan
        .review_libraries(
            &BTreeMap::from([(1, PageImportChoice::UseExisting)]),
            &BTreeMap::new(),
        )
        .unwrap();
    let choices = libraries
        .entries()
        .iter()
        .filter(|entry| entry.action == wireframe::workspace::LibraryImportAction::Conflict)
        .map(|entry| (entry.incoming, LibraryImportChoice::UseExisting))
        .collect();
    target
        .apply_project_copy(libraries.prepare(&choices).unwrap())
        .unwrap();
    assert_eq!(target.document, before);
}
