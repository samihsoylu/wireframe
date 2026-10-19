// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, workspace::Editor};

fn crop() -> Crop {
    Crop {
        width: 0.8,
        height: 0.8,
        ..Crop::ALL
    }
}
fn fixture() -> (Editor, Id, Id, Id) {
    let mut document = Project {
        font: None,
        next_id: 4,
        ..Project::default()
    };
    let child = Node::control(
        3,
        "button",
        Point::default(),
        Size {
            width: Dimension::Fixed(240.),
            height: Dimension::Fixed(120.),
        },
    );
    let mut group = Node::group(2, Point::new(50., 50.), vec![child]);
    group.crop = Some(crop());
    group.markup = true;
    document.pages[0].content.nodes.push(group);
    let mut e = Editor::new(document);
    e.select(2, false);
    let definition = e.create_component("Cropped source".into()).unwrap();
    let library = e.document.default_library.unwrap();
    let placement = e.document.libraries[0].sources[0].node;
    (e, definition, library, placement)
}
fn root(e: &Editor) -> Node {
    e.document
        .resolve(&e.document.pages[0].content.nodes[0])
        .unwrap()
        .root
}
#[test]
fn conversion_shared_edits_local_clear_reset_history_and_roundtrip() {
    let (mut e, definition, library, placement) = fixture();
    assert_eq!(
        e.document.source_appearance(definition),
        (Some(crop()), true)
    );
    assert_eq!((root(&e).crop, root(&e).markup), (Some(crop()), true));
    let instance = &e.document.pages[0].content.nodes[0];
    assert_eq!((instance.crop, instance.markup), (None, false));
    let initial = e.document.clone();
    e.crop_group(2, None, Point::default()).unwrap();
    e.set_markup(2, false).unwrap();
    assert_eq!((root(&e).crop, root(&e).markup), (None, false));
    assert_eq!(
        e.document.source_appearance(definition),
        (Some(crop()), true)
    );
    let NodeContent::Instance { overrides, .. } = &e.document.pages[0].content.nodes[0].content
    else {
        panic!()
    };
    assert_eq!(overrides.len(), 1);
    assert_eq!(overrides[0].path, Vec::<Id>::new());
    assert_eq!(overrides[0].patch.crop, Some(CropOverride::Clear));
    assert_eq!(overrides[0].patch.markup, Some(false));
    let local = e.document.clone();
    assert_eq!(decode(&local.encode().unwrap()).unwrap(), local);
    e.reset_instance(2).unwrap();
    assert_eq!(e.document, initial);
    e.undo();
    assert_eq!(e.document, local);
    e.redo();
    assert_eq!(e.document, initial);
    e.edit_library(library).unwrap();
    e.crop_group(placement, None, Point::default()).unwrap();
    e.set_markup(placement, false).unwrap();
    assert_eq!((root(&e).crop, root(&e).markup), (None, false));
    assert_eq!(e.document.definitions, initial.definitions);
    assert!(e.reset_instance(placement).is_err());
}
#[test]
fn source_child_editing_is_uncropped_but_library_and_instance_share_crop() {
    let (mut e, definition, library, _) = fixture();
    let page = e.page;
    let page_layout = e.layout_page(page).unwrap();
    let library_layout = e.layout_page(library).unwrap();
    assert_eq!(page_layout.nodes[0].bounds.width, 192.);
    assert_eq!(library_layout.nodes[0].bounds.width, 192.);
    e.edit_library_source(definition).unwrap();
    assert_eq!(
        e.layout_page(definition).unwrap().nodes[0].bounds.width,
        240.
    );
    let saved = e.document.clone();
    for surface in [page, library, definition] {
        assert!(!e.rendered_scene(surface).unwrap().svg().is_empty());
    }
    assert_eq!(e.document, saved);
}
#[test]
fn clipboard_reuses_only_equal_shared_appearance_and_preserves_local_clear() {
    let (mut original, definition, library, placement) = fixture();
    original.crop_group(2, None, Point::default()).unwrap();
    original.set_markup(2, false).unwrap();
    original.select(2, false);
    let copy = original.copy();
    let mut destination = Editor::new(original.document.clone());
    destination.edit_library(library).unwrap();
    destination
        .crop_group(placement, None, Point::default())
        .unwrap();
    destination.switch_page(destination.page);
    destination.paste(&copy, 10.);
    assert!(destination.error.is_none(), "{:?}", destination.error);
    assert_eq!(destination.document.definitions.len(), 2);
    let inserted = *destination.selection.iter().next().unwrap();
    let new_definition = destination.instance_definition(inserted).unwrap();
    assert_ne!(definition, new_definition);
    assert_eq!(
        destination.document.source_appearance(new_definition),
        (Some(crop()), true)
    );
    let pasted = destination
        .document
        .resolve(
            destination
                .nodes()
                .iter()
                .find(|n| n.id == inserted)
                .unwrap(),
        )
        .unwrap()
        .root;
    assert_eq!((pasted.crop, pasted.markup), (None, false));
    let mut equal = Editor::new(original.document.clone());
    equal.paste(&copy, 10.);
    assert_eq!(equal.document.definitions.len(), 1);
    let mut blank = Editor::default();
    blank.paste(&copy, 0.);
    assert!(blank.error.is_none(), "{:?}", blank.error);
    assert_eq!(blank.document.definitions.len(), 1);
    assert_eq!(
        decode(&blank.document.encode().unwrap()).unwrap(),
        blank.document
    );
}
#[test]
fn legacy_markup_remains_local_to_library_until_instance_reset() {
    let (mut e, definition, _, _) = fixture();
    e.document.libraries[0].content.nodes[0].crop = None;
    e.document.format_version = 6;
    let bytes = serde_json::to_vec_pretty(&e.document).unwrap();
    let migrated = decode(&bytes).unwrap();
    let mut editor = Editor::new(migrated);
    assert!(editor.document.source_appearance(definition).1);
    assert!(!root(&editor).markup);
    assert_eq!(editor.document.definitions, e.document.definitions);
    editor.reset_instance(2).unwrap();
    assert!(root(&editor).markup);
    assert_eq!(
        decode(&editor.document.encode().unwrap()).unwrap(),
        editor.document
    );
}
#[test]
fn invalid_root_overrides_and_old_version_masquerading_are_rejected() {
    let (e, _, _, _) = fixture();
    let mut old = e.document.clone();
    old.format_version = 6;
    assert!(decode(&serde_json::to_vec(&old).unwrap()).is_err());
    let mut invalid = e.document.clone();
    let NodeContent::Instance { overrides, .. } = &mut invalid.pages[0].content.nodes[0].content
    else {
        panic!()
    };
    overrides.push(Override {
        path: vec![],
        patch: Patch {
            origin: Some(Point::default()),
            ..Patch::default()
        },
    });
    assert!(invalid.validate().is_err());
    let mut patched = e.document.clone();
    patched.libraries[0].content.nodes[0].crop = None;
    let NodeContent::Instance { overrides, .. } = &mut patched.pages[0].content.nodes[0].content
    else {
        panic!()
    };
    overrides.push(Override {
        path: vec![],
        patch: Patch {
            crop: Some(CropOverride::Clear),
            ..Patch::default()
        },
    });
    patched.format_version = 6;
    assert!(decode(&serde_json::to_vec(&patched).unwrap()).is_err());
    let mut invalid = e.document.clone();
    invalid.libraries[0].content.nodes[0]
        .crop
        .as_mut()
        .unwrap()
        .width = f32::NAN;
    assert!(invalid.validate().is_err());
}
#[test]
fn nested_local_crop_and_markup_override_shared_root_and_reset_to_source() {
    let (mut e, definition, _, _) = fixture();
    let outer = e.document.allocate_id().unwrap();
    let mut nested = e.document.pages[0].content.nodes[0].clone();
    nested.origin = Point::default();
    e.document.definitions.push(Definition {
        id: outer,
        name: "Outer".into(),
        content: Content {
            nodes: vec![nested],
            paint_order: vec![],
        },
    });
    e.document.organize_components().unwrap();
    let outer_instance = e.insert_component(outer, 400., 50.).unwrap();
    e.enter(outer_instance);
    e.crop_group(2, None, Point::default()).unwrap();
    e.set_markup(2, false).unwrap();
    assert_eq!((e.nodes()[0].crop, e.nodes()[0].markup), (None, false));
    assert_eq!(
        e.document.source_appearance(definition),
        (Some(crop()), true)
    );
    e.reset_local_changes(2).unwrap();
    assert_eq!(
        (e.nodes()[0].crop, e.nodes()[0].markup),
        (Some(crop()), true)
    );
}

#[test]
fn detach_materializes_shared_root_and_preserves_it_through_history_and_storage() {
    let (mut e, _, _, _) = fixture();
    let expected = root(&e);
    let before = e.document.clone();
    e.detach_component(2).unwrap();
    assert_eq!(e.document.pages[0].content.nodes[0], expected);
    let detached = e.document.clone();
    assert_eq!(decode(&detached.encode().unwrap()).unwrap(), detached);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, detached);
}
