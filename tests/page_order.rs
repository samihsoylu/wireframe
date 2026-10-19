// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, workspace::Editor};

fn project() -> Project {
    let mut p = Project {
        font: None,
        ..Project::default()
    };
    let template = p.pages[0].clone();
    p.pages = [None, None, Some(2), Some(3), Some(2), None, Some(6)]
        .into_iter()
        .enumerate()
        .map(|(i, parent)| Page {
            id: i as u64 + 1,
            name: format!("Page {}", i + 1),
            notes: format!("Preserved notes {}", i + 1),
            parent,
            ..template.clone()
        })
        .collect();
    p.next_id = 8;
    p
}

fn shape(e: &Editor) -> Vec<(Id, Option<Id>)> {
    e.document.pages.iter().map(|p| (p.id, p.parent)).collect()
}

fn roundtrip(e: &mut Editor, before: &Project) {
    let after = e.document.clone();
    let saved = after.encode().unwrap();
    assert_eq!(decode(&saved).unwrap(), after);
    for page in &after.pages {
        let mut original = before
            .pages
            .iter()
            .find(|p| p.id == page.id)
            .unwrap()
            .clone();
        original.parent = page.parent;
        assert_eq!(page, &original, "ordering may not change page payloads");
    }
    for _ in 0..3 {
        e.undo();
        assert_eq!(&e.document, before);
        assert!(e.undo_name().is_none());
        e.redo();
        assert_eq!(e.document, after);
    }
}

#[test]
fn moving_expanded_parent_adopts_unselected_children_without_moving_them() {
    let before = project();
    let mut e = Editor::new(before.clone());
    e.switch_page(4);
    e.reorder_pages(&[2], 7).unwrap();
    assert_eq!(
        shape(&e),
        [
            (1, None),
            (3, Some(1)),
            (4, Some(3)),
            (5, Some(1)),
            (6, None),
            (7, Some(6)),
            (2, None)
        ]
    );
    assert_eq!(e.page, 4);
    assert_eq!(e.undo_name(), Some("Reorder wireframes"));
    roundtrip(&mut e, &before);
    assert_eq!(e.page, 4);
}

#[test]
fn moving_subtree_before_child_inherits_destination_parent() {
    let before = project();
    let mut e = Editor::new(before.clone());
    e.reorder_pages(&[5, 4, 2, 3, 2], 6).unwrap();
    assert_eq!(
        shape(&e),
        [
            (1, None),
            (6, None),
            (2, Some(6)),
            (3, Some(2)),
            (4, Some(3)),
            (5, Some(2)),
            (7, Some(6))
        ]
    );
    roundtrip(&mut e, &before);
}

#[test]
fn sparse_move_rebuilds_selected_ancestry_and_preserves_remaining_tree() {
    let before = project();
    let mut e = Editor::new(before.clone());
    e.reorder_pages(&[4, 1], 7).unwrap();
    assert_eq!(
        shape(&e),
        [
            (2, None),
            (3, Some(2)),
            (5, Some(2)),
            (6, None),
            (7, Some(6)),
            (1, None),
            (4, Some(1))
        ]
    );
    roundtrip(&mut e, &before);
}

#[test]
fn indent_parent_alone_keeps_its_children_at_their_existing_depth() {
    let before = project();
    let mut e = Editor::new(before.clone());
    e.change_page_indent(&[2], true).unwrap();
    assert_eq!(
        shape(&e),
        [
            (1, None),
            (2, Some(1)),
            (3, Some(1)),
            (4, Some(3)),
            (5, Some(1)),
            (6, None),
            (7, Some(6))
        ]
    );
    roundtrip(&mut e, &before);
}

#[test]
fn indent_selected_branch_and_outdent_absorbing_following_siblings() {
    let before = project();
    let mut e = Editor::new(before.clone());
    e.change_page_indent(&[5, 4, 3, 2], true).unwrap();
    assert_eq!(
        shape(&e),
        [
            (1, None),
            (2, Some(1)),
            (3, Some(2)),
            (4, Some(3)),
            (5, Some(2)),
            (6, None),
            (7, Some(6))
        ]
    );
    roundtrip(&mut e, &before);
    let mut e = Editor::new(before.clone());
    e.change_page_indent(&[3], false).unwrap();
    assert_eq!(
        shape(&e),
        [
            (1, None),
            (2, None),
            (3, None),
            (4, Some(3)),
            (5, Some(3)),
            (6, None),
            (7, Some(6))
        ]
    );
    roundtrip(&mut e, &before);
}

#[test]
fn stale_retained_and_invalid_drop_targets_fail_without_history_or_partial_edits() {
    let mut p = project();
    p.pages[6].trashed = true;
    let mut e = Editor::new(p.clone());
    for selected in [&[2, 999][..], &[2, 7]] {
        assert!(e.reorder_pages(selected, 0).is_err());
        assert!(e.change_page_indent(selected, true).is_err());
        assert!(e.change_page_indent(selected, false).is_err());
    }
    assert!(e.reorder_pages(&[2], 7).is_err());
    assert_eq!(e.document, p);
    assert!(e.undo_name().is_none());
    e.reorder_pages(&[], 0).unwrap();
    e.change_page_indent(&[], true).unwrap();
    e.change_page_indent(&[1], true).unwrap();
    e.change_page_indent(&[1, 2, 6], false).unwrap();
    assert!(e.undo_name().is_none());
}

#[test]
fn retained_pages_keep_slots_and_stored_parent_when_live_pages_move() {
    let mut before = project();
    before.pages[4].trashed = true;
    let retained = before.pages[4].clone();
    let mut e = Editor::new(before.clone());
    e.reorder_pages(&[2, 3, 4], 6).unwrap();
    assert_eq!(e.document.pages[4], retained);
    assert_eq!(
        shape(&e),
        [
            (1, None),
            (6, None),
            (7, Some(6)),
            (2, None),
            (5, Some(2)),
            (3, Some(2)),
            (4, Some(3))
        ]
    );
    roundtrip(&mut e, &before);
}

#[test]
fn maximum_indent_is_noop_and_noop_move_preserves_redo() {
    let mut before = project();
    for i in 1..5 {
        before.pages[i].parent = Some(i as u64);
    }
    let mut e = Editor::new(before.clone());
    e.change_page_indent(&[5], true).unwrap();
    assert_eq!(e.document, before);
    assert!(e.undo_name().is_none());
    e.reorder_pages(&[6, 7], 0).unwrap();
    let after = e.document.clone();
    e.undo();
    e.reorder_pages(&[1, 2, 3, 4, 5], 0).unwrap();
    e.redo();
    assert_eq!(e.document, after);
}

#[test]
fn ordering_preserves_component_overrides_links_alternates_and_current_selection() {
    let mut setup = Editor::new(project());
    setup.switch_page(2);
    setup.add(wireframe::model::Kind::Button, 30., 40.);
    let node = *setup.selection.iter().next().unwrap();
    setup
        .set_control_property(node, "link", Value::Link(Link::Page(6)))
        .unwrap();
    setup.create_component("Shared action".into()).unwrap();
    let version = setup.create_version("Compact").unwrap();
    setup.set_notes("Alternate notes".into());
    setup.commit();
    let before = setup.document.clone();
    let mut e = Editor::new(before.clone());
    e.switch_page(2);
    e.switch_version(2, Some(version)).unwrap();
    e.select_all();
    let selected = e.selection.clone();
    e.reorder_pages(&[2, 3, 4, 5], 7).unwrap();
    assert_eq!(e.document.definitions, before.definitions);
    assert_eq!(e.document.reviews, before.reviews);
    assert_eq!(e.selection, selected);
    assert_eq!(e.active_alternate(2), Some(version));
    roundtrip(&mut e, &before);
    assert_eq!(e.active_alternate(2), Some(version));
    assert_eq!(e.selection, selected);
}
