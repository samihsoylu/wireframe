// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::collections::HashSet;
use wireframe::{document::*, page_tree::PageTree, workspace::Editor};

fn project() -> Project {
    let mut p = Project {
        font: None,
        ..Project::default()
    };
    let template = p.pages[0].clone();
    p.pages = [
        (1, "Previous", None),
        (2, "Parent", None),
        (3, "Child", Some(2)),
        (4, "Grandchild", Some(3)),
        (5, "Next", None),
    ]
    .into_iter()
    .map(|(id, name, parent)| Page {
        id,
        name: name.into(),
        parent,
        ..template.clone()
    })
    .collect();
    p.next_id = 6;
    p
}

#[test]
fn tree_projection_preserves_hidden_parents_and_orders_nested_rows() {
    let mut p = project();
    // An interleaved root still renders outside the preceding parent's subtree.
    p.pages.swap(3, 4);
    let tree = PageTree::new(&p);
    assert_eq!(
        tree.rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        [1, 2, 3, 4, 5]
    );
    assert_eq!(
        tree.subtree(2).iter().map(|r| r.id).collect::<Vec<_>>(),
        [2, 3, 4]
    );
    assert_eq!(
        tree.visible(&HashSet::from([2]))
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        [1, 2, 5]
    );
    p.pages[1].trashed = true;
    let before = p.encode().unwrap();
    let tree = PageTree::new(&p);
    assert_eq!(tree.row(3).unwrap().parent, None);
    assert_eq!(tree.row(4).unwrap().depth, 1);
    assert_eq!(p.pages[2].parent, Some(2));
    assert_eq!(p.encode().unwrap(), before);
    assert!(tree.subtree(999).is_empty());
}

#[test]
fn collapsed_subtree_duplicate_preserves_hierarchy_and_one_history_step() {
    let before = project();
    let mut e = Editor::new(before.clone());
    let copies = e.duplicate_pages(&[4, 2, 3, 2]).unwrap();
    assert_eq!(copies.len(), 3);
    assert_eq!(e.page, copies[0]);
    assert_eq!(e.undo_name(), Some("Duplicate wireframes"));
    assert_eq!(
        e.document.pages.iter().map(|p| p.id).collect::<Vec<_>>(),
        [1, 2, 3, 4, copies[0], copies[1], copies[2], 5]
    );
    assert_eq!(e.document.live_page(copies[0]).unwrap().parent, None);
    assert_eq!(
        e.document.live_page(copies[1]).unwrap().parent,
        Some(copies[0])
    );
    assert_eq!(
        e.document.live_page(copies[2]).unwrap().parent,
        Some(copies[1])
    );
    let after = e.document.clone();
    for _ in 0..3 {
        e.undo();
        assert_eq!(e.document, before);
        assert_eq!(e.page, 4);
        e.redo();
        assert_eq!(e.document, after);
        assert_eq!(e.page, copies[0]);
    }
}

#[test]
fn disjoint_duplicate_rebuilds_selected_depths_and_attaches_after_last_ancestor() {
    let mut e = Editor::new(project());
    let copies = e.duplicate_pages(&[4, 1]).unwrap();
    assert_eq!(
        e.document.pages.iter().map(|p| p.id).collect::<Vec<_>>(),
        [1, 2, 3, 4, copies[0], copies[1], 5]
    );
    assert_eq!(
        e.document.live_page(copies[1]).unwrap().parent,
        Some(copies[0])
    );
    assert_eq!(
        e.document.live_page(copies[0]).unwrap().name,
        "Previous copy"
    );
    assert_eq!(
        e.document.live_page(copies[1]).unwrap().name,
        "Grandchild copy"
    );
    e.undo();
    let copies = e.duplicate_pages(&[3, 5]).unwrap();
    assert!(
        copies
            .iter()
            .all(|id| e.document.live_page(*id).unwrap().parent.is_none())
    );
    assert_eq!(
        e.document.pages.iter().map(|p| p.id).collect::<Vec<_>>(),
        [1, 2, 3, 4, 5, copies[0], copies[1]]
    );
}

#[test]
fn batch_duplicate_rejects_stale_retained_and_resource_limit_targets_atomically() {
    let mut p = project();
    p.pages[4].trashed = true;
    let mut e = Editor::new(p.clone());
    for ids in [&[1, 999][..], &[1, 5]] {
        assert!(e.duplicate_pages(ids).is_err());
        assert_eq!(e.document, p);
        assert_eq!(e.page, 1);
        assert!(e.undo_name().is_none());
    }
    assert!(e.duplicate_pages(&[]).unwrap().is_empty());
    let mut p = project();
    let template = p.pages[0].clone();
    for id in 6..=999 {
        p.pages.push(Page {
            id,
            ..template.clone()
        });
    }
    p.next_id = 1000;
    let mut e = Editor::new(p.clone());
    assert!(e.duplicate_pages(&[1, 2]).is_err());
    assert_eq!(e.document, p);
    assert_eq!(e.page, 1);
    assert!(e.undo_name().is_none());
}
