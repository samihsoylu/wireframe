// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, model::Kind, workspace::Editor};

fn fixture() -> (Editor, Id) {
    let mut e = Editor::default();
    e.add(Kind::Button, 30., 40.);
    e.create_version("Compact").unwrap();
    e.add_page();
    let survivor = e.page;
    e.add(Kind::Button, 80., 90.);
    e.document.pages[1].content.nodes[0]
        .properties
        .insert("link".into(), Value::Link(Link::Page(1)));
    e.create_version("Compact").unwrap();
    e.switch_version(survivor, None).unwrap();
    e.document.validate().unwrap();
    (Editor::new(e.document), survivor)
}

#[test]
fn removal_keeps_incoming_links_shared_versions_and_exact_restart_state() {
    let (mut e, survivor) = fixture();
    e.trash_pages(&[1]).unwrap();
    let kept = e
        .document
        .pages
        .iter()
        .find(|p| p.id == survivor)
        .unwrap()
        .clone();
    let versions = e.document.versions.clone();
    e.mark_saved();
    e.permanently_remove_pages(&[1]).unwrap();
    assert_eq!(e.document.pages, vec![kept]);
    assert_eq!(e.document.versions, versions);
    assert_eq!(
        e.document.removed_pages.iter().copied().collect::<Vec<_>>(),
        vec![1]
    );
    assert!(e.dirty());
    let reopened = Editor::new(decode(&e.document.encode().unwrap()).unwrap());
    assert_eq!(reopened.document, e.document);
    assert_eq!(reopened.page, survivor);
    e.trash_pages(&[survivor]).unwrap();
    let blank = e.document.first_live_page().clone();
    e.permanently_remove_pages(&[survivor]).unwrap();
    assert_eq!(e.document.pages, vec![blank]);
    assert!(e.document.versions.is_empty());
    assert_eq!(e.document.removed_pages.len(), 2);
}

#[test]
fn unrelated_undo_redo_survives_but_deleted_target_history_is_consumed() {
    let (mut e, survivor) = fixture();
    e.rename_page(1, "Removed name").unwrap();
    e.trash_pages(&[1]).unwrap();
    e.rename_page(survivor, "Survivor checked").unwrap();
    e.permanently_remove_pages(&[1]).unwrap();
    let after = e.document.clone();
    e.undo();
    assert_ne!(e.document.pages[0].name, "Survivor checked");
    assert_eq!(e.document.pages.len(), 1);
    e.redo();
    assert_eq!(e.document, after);
    e.undo();
    let before = e.document.clone();
    for _ in 0..2 {
        e.error = None;
        e.undo();
        assert!(e.error.as_deref().unwrap().contains("permanently deleted"));
        assert_eq!(e.document, before);
    }
    assert!(e.undo_name().is_none());
    e.redo(); // Only the unrelated surviving rename enters Redo.
    assert_eq!(e.document, after);
    assert!(e.redo_name().is_none());
}

#[test]
fn removing_last_page_preserves_its_placeholder_through_failed_history() {
    let mut e = Editor::default();
    e.rename_page(1, "Disposable").unwrap();
    e.trash_pages(&[1]).unwrap();
    let blank = e.document.first_live_page().clone();
    e.permanently_remove_pages(&[1]).unwrap();
    let after = e.document.clone();
    for _ in 0..2 {
        e.undo();
        assert_eq!(e.document, after);
        assert!(e.redo_name().is_none());
    }
    assert!(e.undo_name().is_none());
    assert_eq!(e.document.pages, vec![blank]);
}

#[test]
fn redo_branch_cannot_resurrect_removed_pages_or_lose_unrelated_edits() {
    let (mut e, survivor) = fixture();
    e.rename_page(survivor, "Retain redo").unwrap();
    e.trash_pages(&[1]).unwrap();
    e.rename_page(survivor, "Latest redo").unwrap();
    e.undo();
    e.permanently_remove_pages(&[1]).unwrap();
    e.redo();
    assert_eq!(e.document.pages[0].name, "Latest redo");
    assert_eq!(e.document.pages.len(), 1);
    e.undo();
    e.undo(); // Consumes the removed target's Trash operation.
    e.undo();
    assert_ne!(e.document.pages[0].name, "Retain redo");
    assert_eq!(e.document.pages.len(), 1);
    e.document.validate().unwrap();
}

#[test]
fn mixed_trash_history_restores_only_surviving_targets() {
    let (mut e, survivor) = fixture();
    e.trash_pages(&[1, survivor]).unwrap();
    e.permanently_remove_pages(&[1]).unwrap();
    e.undo();
    assert_eq!(
        e.document.live_pages().map(|p| p.id).collect::<Vec<_>>(),
        vec![survivor]
    );
    assert!(e.document.pages.iter().all(|p| p.id != 1));
    e.redo();
    assert!(
        e.document
            .pages
            .iter()
            .find(|p| p.id == survivor)
            .unwrap()
            .trashed
    );
    assert!(e.document.pages.iter().all(|p| p.id != 1));
    e.document.validate().unwrap();
}

#[test]
fn stale_or_invalid_selection_preserves_document_pending_edit_and_history() {
    let (mut e, survivor) = fixture();
    e.trash_pages(&[1]).unwrap();
    e.switch_page(survivor);
    e.begin("Pending notes");
    e.document.pages[1].notes = "Unsaved text".into();
    let before = e.document.clone();
    let undo = e.undo_name().map(str::to_owned);
    for ids in [vec![], vec![1, 1], vec![1, survivor], vec![1, 999]] {
        assert!(e.permanently_remove_pages(&ids).is_err());
        assert_eq!(e.document, before);
        assert_eq!(e.undo_name(), undo.as_deref());
    }
    e.permanently_remove_pages(&[1]).unwrap();
    assert_eq!(e.document.pages[0].notes, "Unsaved text");
    e.undo();
    assert_eq!(e.document.pages[0].notes, "");
    assert_eq!(e.document.pages.len(), 1);
    e.redo();
    assert_eq!(e.document.pages[0].notes, "Unsaved text");
}

#[test]
fn removed_destinations_require_versioned_valid_reserved_identities() {
    let (mut e, _) = fixture();
    e.trash_pages(&[1]).unwrap();
    e.permanently_remove_pages(&[1]).unwrap();
    let p = e.document;
    for version in 1..10 {
        let mut old = p.clone();
        old.format_version = version;
        assert!(
            decode(&serde_json::to_vec(&old).unwrap())
                .unwrap_err()
                .contains("version 10")
        );
    }
    let mut invalid = p.clone();
    invalid.removed_pages.clear();
    assert!(invalid.validate().is_err());
    for bad in [0, p.pages[0].id, p.versions[0].id, p.next_id] {
        let mut invalid = p.clone();
        invalid.removed_pages.insert(bad);
        assert!(invalid.validate().is_err(), "invalid removed ID {bad}");
    }
    let mut valid = p;
    let allocated = valid.allocate_id().unwrap();
    assert!(!valid.removed_pages.contains(&allocated));
    valid.validate().unwrap();
}

#[test]
fn nested_and_row_links_remain_broken_without_recreating_a_destination() {
    let (mut e, survivor) = fixture();
    let id = e.document.allocate_id().unwrap();
    let child = e.document.pages[1].content.nodes[0].clone();
    let mut group = Node::group(id, Point::default(), vec![child]);
    group.properties.insert(
        "links".into(),
        Value::Links(vec![Some(Link::Page(1)), None]),
    );
    e.document.pages[1].content.nodes = vec![group];
    e.document.pages[1].parent = Some(1);
    e.document.validate().unwrap();
    e = Editor::new(e.document);
    e.trash_pages(&[1]).unwrap();
    e.permanently_remove_pages(&[1]).unwrap();
    e.switch_page(survivor);
    let before = e.document.clone();
    e.select_all();
    let copy = e.copy();
    let bytes = serde_json::to_vec(&copy).unwrap();
    let copy = serde_json::from_slice(&bytes).unwrap();
    e.paste(&copy, 10.);
    assert!(e.error.is_none(), "{:?}", e.error);
    let pasted = e.document.pages[0].content.nodes.last().unwrap();
    assert_eq!(
        pasted.properties.get("links"),
        Some(&Value::Links(vec![Some(Link::Page(1)), None]))
    );
    let NodeContent::Group { children } = &pasted.content else {
        panic!("expected group")
    };
    assert_eq!(
        children[0].properties.get("link"),
        Some(&Value::Link(Link::Page(1)))
    );
    assert!(e.document.pages[0].parent.is_none());
    assert_eq!(e.document.pages.len(), 1);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
}

#[test]
fn component_sources_overrides_and_library_paste_keep_known_broken_links() {
    let (mut e, survivor) = fixture();
    e.switch_page(survivor);
    e.select_all();
    e.create_component("Linked button".into()).unwrap();
    let definition = e.document.definitions[0].id;
    let child = e.document.definitions[0].content.nodes[0].id;
    if let NodeContent::Instance { overrides, .. } =
        &mut e.document.pages[1].content.nodes[0].content
    {
        let mut patch = Patch::default();
        patch
            .properties
            .insert("link".into(), Value::Link(Link::Page(1)));
        overrides.push(Override {
            path: vec![child],
            patch,
        });
    } else {
        panic!("expected component instance")
    }
    e.document.validate().unwrap();
    e = Editor::new(e.document);
    e.trash_pages(&[1]).unwrap();
    e.permanently_remove_pages(&[1]).unwrap();
    let libraries = e.document.libraries.clone();
    let definitions = e.document.definitions.clone();
    assert!(definitions.iter().any(|d| d.id == definition));
    let copy = e
        .copy_libraries(&[libraries[0].id], "same-project")
        .unwrap();
    let plan = e.plan_library_import(&copy, "same-project").unwrap();
    let prepared = plan.prepare(&Default::default()).unwrap();
    e.apply_library_import(prepared, "same-project").unwrap();
    assert_eq!(e.document.definitions.len(), definitions.len() * 2);
    for d in &e.document.definitions {
        assert_eq!(
            d.content.nodes[0].properties.get("link"),
            Some(&Value::Link(Link::Page(1)))
        );
    }
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
    e.undo();
    assert_eq!(e.document.libraries, libraries);
    assert_eq!(e.document.definitions, definitions);
    assert_eq!(e.document.pages.len(), 1);
}

#[test]
fn limit_failure_does_not_commit_pending_changes_or_partially_remove_pages() {
    let (mut e, survivor) = fixture();
    e.trash_pages(&[1]).unwrap();
    e.document.removed_pages = (1000..1000 + MAX_NODES as u64).collect();
    e.document.next_id = 1001 + MAX_NODES as u64;
    e.document.validate().unwrap();
    e = Editor::new(e.document);
    e.switch_page(survivor);
    e.begin("Pending notes");
    e.document.pages[1].notes = "Pending".into();
    let before = e.document.clone();
    assert!(
        e.permanently_remove_pages(&[1])
            .unwrap_err()
            .contains("Too many")
    );
    assert_eq!(e.document, before);
    assert!(e.undo_name().is_none());
    e.cancel();
    assert_eq!(e.document.pages[1].notes, "");
    assert!(e.document.pages[0].trashed);
}

#[test]
fn history_before_the_survivor_existed_cannot_replace_its_current_content() {
    let mut e = Editor::default();
    e.add_page();
    let survivor = e.page;
    e.add(Kind::Button, 40., 50.);
    e.create_version("Survivor only").unwrap();
    e.trash_pages(&[1]).unwrap();
    e.permanently_remove_pages(&[1]).unwrap();
    e.undo(); // Consumes old Trash of removed page.
    e.undo(); // Surviving alternate remains ordinarily undoable.
    assert!(e.document.pages[0].alternates.is_empty());
    e.undo(); // Surviving control insertion remains undoable.
    let before = e.document.clone();
    e.undo(); // Creating the survivor depended on the removed previous page.
    assert!(e.error.as_deref().unwrap().contains("permanently deleted"));
    assert_eq!(e.document, before);
    assert_eq!(e.page, survivor);
    e.document.validate().unwrap();
    e.redo();
    assert_eq!(e.document.pages[0].content.nodes.len(), 1);
    e.redo();
    assert_eq!(e.document.pages[0].alternates.len(), 1);
}
