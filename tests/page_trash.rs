// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, model::Kind, workspace::Editor};

fn hierarchy() -> Editor {
    let mut p = Project::default();
    let page = p.pages[0].clone();
    p.pages = [
        (1, "Previous", None),
        (2, "Parent", None),
        (3, "Child", Some(2)),
    ]
    .into_iter()
    .map(|(id, name, parent)| Page {
        id,
        name: name.into(),
        parent,
        ..page.clone()
    })
    .collect();
    p.next_id = 4;
    Editor::new(p)
}

fn parent(e: &Editor, id: Id) -> Option<Id> {
    e.document.pages.iter().find(|p| p.id == id).unwrap().parent
}

fn live(e: &Editor) -> Vec<Id> {
    e.document.live_pages().map(|p| p.id).collect()
}

#[test]
fn trash_fallback_follows_surviving_order_and_undo_keeps_current_view() {
    let mut project = Project {
        font: None,
        ..Project::default()
    };
    let template = project.pages[0].clone();
    project.pages = (1..=8)
        .map(|id| Page {
            id,
            name: format!("Page {id}"),
            ..template.clone()
        })
        .collect();
    project.next_id = 9;
    for (current, removed, expected) in [
        (6, vec![1, 6], 7),
        (6, vec![6, 7], 8),
        (6, vec![6, 7, 8], 5),
        (3, vec![3], 4),
        (4, vec![1, 2], 4),
    ] {
        let mut e = Editor::new(project.clone());
        e.switch_page(current);
        e.trash_pages(&removed).unwrap();
        assert_eq!(e.page, expected, "removed {removed:?}");
        let after = e.document.clone();
        for _ in 0..3 {
            e.undo();
            assert_eq!(e.page, expected);
            assert_eq!(e.document, project);
            e.redo();
            assert_eq!(e.page, expected);
            assert_eq!(e.document, after);
        }
    }
    // Retained pages do not participate in fallback, and later view changes
    // are not undone by restoring resources.
    project.pages[3].trashed = true;
    let mut e = Editor::new(project);
    e.switch_page(5);
    let alternate = e.create_version("Compact").unwrap();
    e.switch_page(3);
    e.trash_pages(&[3]).unwrap();
    assert_eq!(e.page, 5);
    assert_eq!(e.active_alternate(5), Some(alternate));
    e.switch_page(8);
    e.undo();
    assert_eq!(e.page, 8);
    assert_eq!(e.active_alternate(5), Some(alternate));
    assert!(e.document.live_page(3).is_some());
}

#[test]
fn expanded_parent_adoption_survives_undo_and_unrelated_edit_history() {
    let mut e = hierarchy();
    e.rename_page(1, "Earlier edit").unwrap();
    e.switch_page(2);
    let before = e.document.clone();
    e.trash_pages(&[2]).unwrap();
    assert_eq!(live(&e), [1, 3]);
    assert_eq!(parent(&e, 3), Some(1));
    assert_eq!(e.page, 3);
    assert_eq!(e.undo_name(), Some("Move to Trash"));
    e.undo();
    let mut expected = before.clone();
    expected.pages[2].parent = Some(1);
    assert_eq!(e.document, expected);
    assert_eq!(e.page, 3);
    e.undo(); // Rename must not silently restore the child's old parent.
    assert_eq!(e.document.pages[0].name, "Previous");
    assert_eq!(parent(&e, 3), Some(1));
    e.redo();
    assert_eq!(e.document, expected);
    e.redo();
    assert_eq!(live(&e), [1, 3]);
    e.undo();
    assert_eq!(e.document, expected);
}

#[test]
fn explicit_subtree_scope_retains_and_restores_internal_relationships() {
    let mut e = hierarchy();
    let before = e.document.clone();
    e.trash_pages(&[2, 3, 2]).unwrap();
    assert_eq!(live(&e), [1]);
    assert_eq!(parent(&e, 3), Some(2));
    for _ in 0..3 {
        e.undo();
        assert_eq!(e.document, before);
        e.redo();
        assert_eq!(live(&e), [1]);
        assert_eq!(parent(&e, 3), Some(2));
    }
    e.restore_pages(&[3, 2]).unwrap(); // Canonical order independent of input order.
    assert_eq!(e.document, before);
    assert_eq!(e.page, 1); // Explicit restore preserves the current page.
    e.undo();
    assert_eq!(live(&e), [1]);
    e.redo();
    assert_eq!(e.document, before);
}

#[test]
fn child_restored_before_parent_keeps_hidden_identity_then_becomes_root() {
    let mut e = hierarchy();
    e.trash_pages(&[3]).unwrap();
    e.trash_pages(&[2]).unwrap();
    e.restore_pages(&[3]).unwrap();
    assert_eq!(parent(&e, 3), Some(2));
    assert_eq!(
        e.document.live_parent(e.document.live_page(3).unwrap()),
        None
    );
    let bytes = e.document.encode().unwrap();
    let mut e = Editor::new(decode(&bytes).unwrap());
    e.restore_pages(&[2]).unwrap();
    assert_eq!(parent(&e, 3), None);
    e.undo();
    assert_eq!(parent(&e, 3), None);
    assert_eq!(live(&e), [1, 3]);
    e.redo();
    assert_eq!(live(&e), [1, 2, 3]);
    assert_eq!(parent(&e, 3), None);
}

#[test]
fn last_page_trash_retains_versions_links_and_dependencies_and_reuses_blank() {
    let mut e = Editor::default();
    e.add(Kind::Button, 50., 60.);
    let button = *e.selection.iter().next().unwrap();
    e.set_control_property(button, "link", Value::Link(Link::Page(1)))
        .unwrap();
    e.create_component("Action".into()).unwrap();
    e.document.pages[0].notes = "Official notes".into();
    e.create_version("Compact").unwrap();
    e.set_notes("Alternate notes".into());
    e.commit();
    e.mark_saved();
    let before = e.document.clone();
    e.trash_pages(&[1]).unwrap();
    let mut retained = before.pages[0].clone();
    retained.trashed = true;
    assert_eq!(e.document.pages[0], retained);
    assert_eq!(e.document.definitions, before.definitions);
    assert_eq!(e.document.versions, before.versions);
    let blank = e.page;
    assert_ne!(blank, 1);
    assert_eq!(live(&e), [blank]);
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    assert!(!e.dirty());
    assert_eq!(e.page, 1);
    assert_eq!(e.active_alternate(1), None);
    e.redo();
    assert_eq!(e.document, after);
    e.restore_pages(&[1]).unwrap();
    assert_eq!(live(&e), [1, blank]);
    assert_eq!(e.page, blank);
    let bytes = e.document.encode().unwrap();
    assert_eq!(decode(&bytes).unwrap(), e.document);
}

#[test]
fn untouched_blank_is_removed_but_modified_or_referenced_blank_survives() {
    for case in ["notes", "content", "link"] {
        let mut e = Editor::default();
        e.trash_pages(&[1]).unwrap();
        let blank = e.page;
        // Simulate changes outside this operation's history, as recovery and
        // targeted operation undo must not discard a now-meaningful page.
        match case {
            "notes" => e.document.pages[1].notes = "Keep this".into(),
            "content" => e.document.pages[1].content.nodes.push(Node::control(
                99,
                "button",
                Point::default(),
                Size::default(),
            )),
            "link" => {
                let mut node = Node::control(99, "button", Point::default(), Size::default());
                node.properties
                    .insert("link".into(), Value::Link(Link::Page(blank)));
                e.document.pages[0].content.nodes.push(node);
            }
            _ => unreachable!(),
        }
        e.document.next_id = 100;
        e.document.validate().unwrap();
        e.undo();
        assert_eq!(live(&e), [1, blank], "{case}");
        assert_eq!(e.page, blank);
        assert_eq!(e.redo_name(), Some("Move to Trash"));
    }
}

#[test]
fn stale_selection_and_limits_fail_atomically_including_history_replay() {
    let mut e = hierarchy();
    e.switch_page(2);
    let before = e.document.clone();
    for ids in [&[2, 999][..], &[0], &[999]] {
        assert!(e.trash_pages(ids).is_err());
        assert_eq!(e.document, before);
        assert_eq!(e.page, 2);
        assert!(e.undo_name().is_none());
    }
    assert!(e.restore_pages(&[2]).is_err());
    e.trash_pages(&[]).unwrap();
    assert!(e.undo_name().is_none());

    let page = e.document.pages[0].clone();
    for id in 4..=1003 {
        e.document.pages.push(Page {
            id,
            trashed: true,
            ..page.clone()
        });
    }
    e.document.next_id = 1004;
    let before = e.document.clone();
    assert!(e.trash_pages(&[2]).is_err());
    assert_eq!(e.document, before);
    assert!(e.undo_name().is_none());

    let mut e = Editor::default();
    e.document.next_id = u64::MAX;
    let before = e.document.clone();
    assert!(e.trash_pages(&[1]).is_err());
    assert_eq!(e.document, before);
    assert!(e.undo_name().is_none());

    let mut e = hierarchy();
    e.trash_pages(&[2]).unwrap();
    let page = e.document.pages[0].clone();
    for id in 4..=1001 {
        e.document.pages.push(Page { id, ..page.clone() });
    }
    e.document.next_id = 1002;
    e.document.validate().unwrap();
    let before = e.document.clone();
    assert!(e.restore_pages(&[2]).is_err());
    e.undo(); // Cannot exceed the live-page limit during history either.
    assert!(e.error.as_ref().unwrap().contains("1000 live"));
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), Some("Move to Trash"));
    assert_eq!(e.redo_name(), None);
}

#[test]
fn rejected_or_empty_page_retention_keeps_pending_edit_and_redo() {
    for action in 0..4 {
        let mut e = hierarchy();
        e.rename_page(1, "Redo this rename").unwrap();
        e.undo();
        let original = e.document.clone();
        let redo = e.redo_name().map(str::to_owned);
        e.begin("Pending notes");
        e.document.pages[0].notes = "Still a draft".into();
        let draft = e.document.clone();
        match action {
            0 => assert!(e.trash_pages(&[2, 999]).is_err()),
            1 => assert!(e.restore_pages(&[2]).is_err()),
            2 => e.trash_pages(&[]).unwrap(),
            _ => e.restore_pages(&[]).unwrap(),
        }
        assert_eq!(e.document, draft);
        assert!(e.undo_name().is_none());
        assert_eq!(e.redo_name(), redo.as_deref());
        e.cancel();
        assert_eq!(e.document, original);
        e.redo();
        assert_eq!(e.document.pages[0].name, "Redo this rename");
    }
}

#[test]
fn failed_last_page_placeholder_allocation_keeps_pending_edit_and_redo() {
    let p = Project {
        next_id: u64::MAX - 1,
        ..Project::default()
    };
    p.validate().unwrap();
    let mut e = Editor::new(p);
    e.rename_page(1, "Redo this rename").unwrap();
    e.undo();
    let original = e.document.clone();
    let redo = e.redo_name().map(str::to_owned);
    e.begin("Pending notes");
    e.document.pages[0].notes = "Still a draft".into();
    let draft = e.document.clone();
    assert!(e.trash_pages(&[1]).is_err());
    assert_eq!(e.document, draft);
    assert!(e.undo_name().is_none());
    assert_eq!(e.redo_name(), redo.as_deref());
    e.cancel();
    assert_eq!(e.document, original);
}

#[test]
fn adoption_prefers_nearest_same_depth_then_shallower_and_keeps_descendants() {
    let mut e = hierarchy();
    let page = e.document.pages[0].clone();
    e.document.pages = [
        (1, None),
        (2, Some(1)),
        (3, Some(2)),
        (4, Some(1)),
        (5, Some(4)),
        (6, Some(5)),
        (7, Some(6)),
    ]
    .into_iter()
    .map(|(id, parent)| Page {
        id,
        parent,
        ..page.clone()
    })
    .collect();
    e.document.next_id = 8;
    e.trash_pages(&[4]).unwrap();
    assert_eq!(parent(&e, 5), Some(2)); // Same-depth sibling, not its deeper child3.
    assert_eq!(parent(&e, 6), Some(5));
    assert_eq!(parent(&e, 7), Some(6));
    e.trash_pages(&[5]).unwrap();
    assert_eq!(parent(&e, 6), Some(3));
    let mut e = hierarchy();
    e.trash_pages(&[1, 2]).unwrap();
    assert_eq!(parent(&e, 3), None);
    assert_eq!(live(&e), [3]);
}

#[test]
fn restoring_a_parent_after_its_child_in_canonical_order_clears_invalid_placement() {
    let mut e = hierarchy();
    e.document.pages.swap(1, 2);
    e.trash_pages(&[2, 3]).unwrap();
    e.restore_pages(&[2, 3]).unwrap();
    assert_eq!(parent(&e, 3), None);
    e.document.validate().unwrap();
}

#[test]
fn redo_blank_identity_cannot_collide_with_a_new_dependency() {
    let mut e = Editor::default();
    e.trash_pages(&[1]).unwrap();
    let blank = e.page;
    e.undo();
    e.document.versions.push(NamedVersion {
        id: blank,
        name: "Retained identity".into(),
    });
    e.document.next_id = blank + 1;
    e.document.validate().unwrap();
    e.redo();
    assert_ne!(e.page, blank);
    assert!(e.document.versions.iter().any(|v| v.id == blank));
    assert_eq!(e.undo_name(), Some("Move to Trash"));
    e.document.validate().unwrap();
}

#[test]
fn all_small_ordered_forests_and_selection_scopes_keep_a_valid_graph() {
    // Every ordered five-page forest and every nonempty explicit selection.
    // This covers mixed ancestors/siblings, partial subtrees and removing all.
    // Reuse the font/render context; this test exercises only document history.
    let mut e = Editor::default();
    for p2 in 0..2 {
        for p3 in 0..3 {
            for p4 in 0..4 {
                for p5 in 0..5 {
                    let mut project = Project {
                        font: None,
                        ..Project::default()
                    };
                    let page = project.pages[0].clone();
                    project.pages = [0, p2, p3, p4, p5]
                        .into_iter()
                        .enumerate()
                        .map(|(i, parent)| Page {
                            id: i as Id + 1,
                            parent: (parent != 0).then_some(parent),
                            ..page.clone()
                        })
                        .collect();
                    project.next_id = 6;
                    for mask in 1..32 {
                        let ids: Vec<_> =
                            (1..=5).filter(|id| mask & (1 << (id - 1)) != 0).collect();
                        e.document = project.clone();
                        e.page = 1;
                        e.trash_pages(&ids).unwrap();
                        let removed = e.document.clone();
                        e.undo();
                        e.document.validate().unwrap();
                        assert_eq!(live(&e), [1, 2, 3, 4, 5]);
                        for (actual, original) in e.document.pages.iter().zip(&project.pages) {
                            let mut expected = original.clone();
                            expected.parent = actual.parent;
                            assert_eq!(actual, &expected); // Only live adoption may persist.
                        }
                        e.redo();
                        assert_eq!(e.document, removed);
                    }
                }
            }
        }
    }
}
