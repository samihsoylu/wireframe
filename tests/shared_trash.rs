// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    model::Kind,
    workspace::{Editor, RetainedResource, SceneScope},
};

fn fixture() -> (Editor, Id, Id) {
    let mut e = Editor::default();
    e.add(Kind::Button, 40., 60.);
    let source = e.create_component("Action".into()).unwrap();
    let library = e.document.component_source(source).unwrap().0.id;
    e.document.libraries[0].notes = "Retained library notes".into();
    e.document.libraries[0].sources[0].notes = "Retained source notes".into();
    e.add_page();
    (Editor::new(e.document), library, source)
}

#[test]
fn retained_library_preview_cannot_reactivate_source_surface_or_cached_exports() {
    let (mut e, library, source) = fixture();
    let live = e.rendered_scene(library).unwrap();
    assert!(!live.items.is_empty());
    e.rendered_scene(source).unwrap();
    e.trash_libraries(&[library]).unwrap();
    let before = e.document.clone();
    for _ in 0..2 {
        let preview = e.retained_preview(library).unwrap();
        assert_eq!(preview.svg(), live.svg());
        for id in [library, source] {
            assert!(e.document.surface(id).is_none());
            assert_eq!(e.document.surface_notes(id), "");
            assert!(e.layout_page(id).is_err());
            assert!(e.rendered_scene(id).is_err());
            assert!(e.export_scene(id, &SceneScope::Whole).is_err());
        }
        assert!(e.retained_preview(source).is_err());
        assert_eq!(e.document, before);
    }
    e.restore_resources(&[RetainedResource::Library(library)])
        .unwrap();
    assert_eq!(e.rendered_scene(library).unwrap().svg(), live.svg());
    assert!(e.retained_preview(library).is_err());
    e.undo();
    assert_eq!(e.document, before);
    assert!(e.rendered_scene(library).is_err());
    assert_eq!(e.retained_preview(library).unwrap().svg(), live.svg());
}

#[test]
fn mixed_restore_is_one_history_step_and_invalid_typed_selection_is_atomic() {
    let (mut e, library, source) = fixture();
    e.trash_libraries(&[library]).unwrap();
    e.trash_pages(&[1]).unwrap();
    let before = e.document.clone();
    let active = e.page;
    let page = RetainedResource::Page(1);
    let lib = RetainedResource::Library(library);
    let undo = e.undo_name().map(str::to_owned);
    for selection in [
        vec![],
        vec![page, page],
        vec![page, RetainedResource::Page(library)],
        vec![lib, RetainedResource::Library(1)],
        vec![lib, RetainedResource::Page(active)],
    ] {
        assert!(e.restore_resources(&selection).is_err());
        assert_eq!(e.document, before);
        assert_eq!(e.undo_name(), undo.as_deref());
    }
    e.restore_resources(&[lib, page]).unwrap();
    assert_eq!(e.undo_name(), Some("Restore from Trash"));
    assert!(e.document.live_page(1).is_some());
    assert!(e.document.definition_available(source));
    assert_eq!(e.page, active);
    let after = e.document.clone();
    for _ in 0..3 {
        e.undo();
        assert_eq!(e.document, before);
        assert_eq!(e.page, active);
        e.redo();
        assert_eq!(e.document, after);
        assert_eq!(e.page, active);
    }
}

#[test]
fn mixed_restore_preserves_page_adoption_and_library_only_restore_leaves_pages_exact() {
    let (mut e, library, _) = fixture();
    let parent = e.page;
    e.add_page();
    let child = e.page;
    e.document
        .pages
        .iter_mut()
        .find(|p| p.id == child)
        .unwrap()
        .parent = Some(parent);
    e = Editor::new(e.document);
    e.trash_libraries(&[library]).unwrap();
    e.trash_pages(&[parent]).unwrap();
    let before = e.document.clone();
    let child_parent = before.pages.iter().find(|p| p.id == child).unwrap().parent;
    assert_ne!(child_parent, Some(parent));
    e.restore_resources(&[
        RetainedResource::Page(parent),
        RetainedResource::Library(library),
    ])
    .unwrap();
    assert_eq!(
        e.document
            .pages
            .iter()
            .find(|p| p.id == child)
            .unwrap()
            .parent,
        child_parent
    );
    e.undo();
    assert_eq!(e.document, before);
    e.restore_resources(&[RetainedResource::Library(library)])
        .unwrap();
    assert_eq!(e.document.pages, before.pages);
    e.undo();
    assert_eq!(e.document, before);
}

#[test]
fn failed_mixed_restore_at_page_limit_keeps_pending_edit_and_redo() {
    let (mut e, library, _) = fixture();
    e.trash_libraries(&[library]).unwrap();
    e.trash_pages(&[1]).unwrap();
    let template = e.document.first_live_page().clone();
    while e.document.live_pages().count() < 1000 {
        let id = e.document.allocate_id().unwrap();
        e.document.pages.push(Page {
            id,
            ..template.clone()
        });
    }
    e.document.validate().unwrap();
    let active = e.page;
    let mut e = Editor::new(e.document);
    e.rename_page(active, "Redo this rename").unwrap();
    e.undo();
    let original = e.document.clone();
    let redo = e.redo_name().map(str::to_owned);
    e.begin("Pending title");
    e.document.title = "Still a draft".into();
    let draft = e.document.clone();
    for mixed in [false, true] {
        let error = if mixed {
            e.restore_resources(&[
                RetainedResource::Page(1),
                RetainedResource::Library(library),
            ])
        } else {
            e.restore_pages(&[1])
        }
        .unwrap_err();
        assert!(error.contains("1000 live"), "{error}");
        assert_eq!(e.document, draft);
        assert!(e.undo_name().is_none());
        assert_eq!(e.redo_name(), redo.as_deref());
    }
    e.cancel();
    assert_eq!(e.document, original);
    e.redo();
    assert_eq!(
        e.document.live_page(active).unwrap().name,
        "Redo this rename"
    );
}

#[test]
fn successful_page_and_mixed_retention_commit_draft_as_a_separate_history_step() {
    for mixed in [false, true] {
        let (mut e, library, _) = fixture();
        if mixed {
            e.trash_libraries(&[library]).unwrap();
            e.trash_pages(&[1]).unwrap();
        }
        let original = e.document.clone();
        e.begin("Pending title");
        e.document.title = "Committed title".into();
        let draft = e.document.clone();
        if mixed {
            e.restore_resources(&[
                RetainedResource::Page(1),
                RetainedResource::Library(library),
            ])
            .unwrap();
        } else {
            e.trash_pages(&[1]).unwrap();
        }
        let after = e.document.clone();
        e.cancel();
        assert_eq!(e.document, after);
        e.undo();
        assert_eq!(e.document, draft);
        assert_eq!(e.undo_name(), Some("Pending title"));
        e.undo();
        assert_eq!(e.document, original);
        e.redo();
        assert_eq!(e.document, draft);
        e.redo();
        assert_eq!(e.document, after);
    }
}
