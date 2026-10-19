// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::collections::BTreeSet;
use wireframe::{document::Point, review::*};

fn author(id: &str) -> Author {
    Author {
        id: id.into(),
        name: format!("Reviewer {id}"),
    }
}
fn body(text: &str) -> Body {
    Body {
        text: text.into(),
        ..Body::default()
    }
}
fn post(d: &mut Discussion, parent: Option<Id>, text: &str, time: u64) -> Id {
    d.post(author("local"), parent, body(text), time).unwrap()
}
fn rows(d: &Discussion, cache: &[Id]) -> Vec<(Id, usize)> {
    d.rows(&cache.iter().copied().collect())
        .unwrap()
        .iter()
        .map(|r| (r.comment.id, r.depth))
        .collect()
}

#[test]
fn threads_sort_roots_and_replies_independently_and_keep_deleted_ancestors() {
    let mut d = Discussion::default();
    let root = post(&mut d, None, "root", 10);
    let reply = post(&mut d, Some(root), "later reply", 15);
    let early = post(&mut d, Some(root), "early reply", 11);
    let child = post(&mut d, Some(early), "nested", 13);
    let other = post(&mut d, None, "new root", 20);
    let gone = post(&mut d, None, "deleted root", 30);
    d.delete(root).unwrap();
    d.delete(early).unwrap();
    d.delete(gone).unwrap();
    assert_eq!(
        rows(&d, &[]),
        vec![(other, 0), (root, 0), (early, 1), (child, 2), (reply, 1)]
    );
    assert_eq!(rows(&d, &[gone])[0], (gone, 0));
    assert!(Draft::new(&d, Some(root)).is_err());
    d.delete(child).unwrap();
    assert!(!rows(&d, &[]).iter().any(|(id, _)| *id == early));
}

#[test]
fn deletion_erases_serialized_body_and_cache_only_restores_current_deletion() {
    let mut d = Discussion::default();
    let mut draft = Draft::new(&d, None).unwrap();
    draft.body = body("Private feedback");
    draft
        .add_callout(&d, Point::new(40., 60.), draft.body.text.len())
        .unwrap();
    let id = draft.commit(&mut d, author("local"), 100).unwrap();
    let cached = d.delete(id).unwrap();
    let bytes = serde_json::to_vec(&d).unwrap();
    assert!(
        !String::from_utf8(bytes.clone())
            .unwrap()
            .contains("Private feedback")
    );
    let reopened: Discussion = serde_json::from_slice(&bytes).unwrap();
    reopened.validate().unwrap();
    assert!(rows(&reopened, &[]).is_empty());
    assert!(reopened.callout(1).is_none());
    d.restore_deleted(&cached).unwrap();
    assert_eq!(d.comments[&id].body, draft.body);
    let current = d.delete(id).unwrap();
    let before = d.clone();
    assert!(d.restore_deleted(&cached).is_err());
    assert_eq!(d, before);
    let mut replaced = d.clone();
    replaced.comments.get_mut(&id).unwrap().imported = true;
    let before = replaced.clone();
    assert!(replaced.restore_deleted(&current).is_err());
    assert_eq!(replaced, before);
    d.restore_deleted(&current).unwrap();
}

#[test]
fn drafts_are_isolated_preserve_unicode_and_commit_marker_defaults() {
    let mut d = Discussion::default();
    let original = d.clone();
    let mut draft = Draft::new(&d, None).unwrap();
    draft.body = body("é界after");
    let before = draft.body.clone();
    assert!(draft.add_callout(&d, Point::new(10., 20.), 1).is_err());
    assert_eq!(draft.body, before);
    let marker = draft.add_callout(&d, Point::new(10., 20.), 5).unwrap();
    assert_eq!(draft.body.text, "é界 [(1)](callout:1) after");
    assert_eq!(d, original);
    assert_eq!(draft.body.callouts[0].size(), [65., 40.]);
    assert_eq!(draft.body.callouts[0].color, Color::Blue);
    let id = draft.commit(&mut d, author("local"), 100).unwrap();
    assert_eq!(d.callout(marker).unwrap().position, Point::new(10., 20.));
    assert!(d.comments[&id].read_by.contains("local"));
    let reopened: Discussion = serde_json::from_slice(&serde_json::to_vec(&d).unwrap()).unwrap();
    assert_eq!(reopened, d);
    reopened.validate().unwrap();
}

#[test]
fn deleted_marker_ids_never_retarget_dangling_reply_links() {
    let mut d = Discussion::default();
    let mut draft = Draft::new(&d, None).unwrap();
    draft.add_callout(&d, Point::new(0., 0.), 0).unwrap();
    let root = draft.commit(&mut d, author("local"), 1).unwrap();
    let reply = post(&mut d, Some(root), "See [(1)](callout:1)", 2);
    d.delete(root).unwrap();
    let mut existing_reply = Draft::edit(&d, reply, "local").unwrap();
    existing_reply.body.text.push_str(" revised");
    existing_reply.commit(&mut d, author("local"), 4).unwrap();
    let mut next = Draft::new(&d, None).unwrap();
    let marker = next.add_callout(&d, Point::new(1., 1.), 0).unwrap();
    assert_eq!(marker, 2);
    next.commit(&mut d, author("local"), 3).unwrap();
    assert_eq!(
        d.comments[&reply].body.marker_references().unwrap(),
        BTreeSet::from([1])
    );
    assert!(d.callout(1).is_none());
    let before = d.clone();
    assert!(d.post(author("local"), None, draft.body, 4).is_err());
    assert_eq!(before, d);
}

#[test]
fn editing_removes_unreferenced_markers_and_rejects_stale_body_even_after_aba() {
    let mut d = Discussion::default();
    let mut draft = Draft::new(&d, None).unwrap();
    draft.body = body("Original");
    draft.add_callout(&d, Point::new(0., 0.), 8).unwrap();
    let id = draft.commit(&mut d, author("local"), 10).unwrap();
    let stale = Draft::edit(&d, id, "local").unwrap();
    let original = stale.body.clone();
    let mut changed = original.clone();
    changed.text.push_str(" revised");
    d.update(id, "local", changed, 11).unwrap();
    d.update(id, "local", original, 12).unwrap();
    let before = d.clone();
    assert!(stale.commit(&mut d, author("local"), 13).is_err());
    assert_eq!(d, before);
    let mut edit = Draft::edit(&d, id, "local").unwrap();
    edit.body.text = "Marker removed".into();
    edit.commit(&mut d, author("local"), 9).unwrap();
    assert!(d.comments[&id].body.callouts.is_empty());
    assert_eq!(d.comments[&id].edited_ms, Some(12));
}

#[test]
fn flags_are_sets_replies_mark_parent_read_and_wrong_author_or_import_cannot_edit() {
    let mut d = Discussion::default();
    let root = post(&mut d, None, "root", 1);
    d.mark_read(root, "local", false).unwrap();
    let mut draft = Draft::new(&d, Some(root)).unwrap();
    draft.body = body("reply");
    draft.commit(&mut d, author("local"), 2).unwrap();
    assert!(d.comments[&root].read_by.contains("local"));
    d.mark_liked(root, "local", true).unwrap();
    d.mark_liked(root, "local", true).unwrap();
    assert_eq!(d.comments[&root].liked_by.len(), 1);
    d.mark_liked(root, "local", false).unwrap();
    let before = d.clone();
    assert!(d.update(root, "other", body("changed"), 3).is_err());
    assert_eq!(d, before);
    assert!(Draft::edit(&d, root, "other").is_err());
    d.comments.get_mut(&root).unwrap().imported = true;
    assert!(Draft::edit(&d, root, "local").is_err());
    assert!(d.update(root, "local", body("changed"), 3).is_err());
    d.delete(root).unwrap();
    assert!(d.mark_liked(root, "other", true).is_err());
    assert!(d.mark_read(root, "other", true).is_err());
}

#[test]
fn stale_parent_or_marker_collision_fails_atomically_and_keeps_draft() {
    let mut d = Discussion::default();
    let root = post(&mut d, None, "root", 1);
    let mut reply = Draft::new(&d, Some(root)).unwrap();
    reply.body = body("unsent");
    d.delete(root).unwrap();
    let before = d.clone();
    assert!(reply.commit(&mut d, author("local"), 2).is_err());
    assert_eq!(d, before);
    assert_eq!(reply.body.text, "unsent");
    let mut first = Draft::new(&d, None).unwrap();
    let mut second = first.clone();
    first.add_callout(&d, Point::new(0., 0.), 0).unwrap();
    second.add_callout(&d, Point::new(9., 9.), 0).unwrap();
    first.commit(&mut d, author("local"), 3).unwrap();
    let before = d.clone();
    assert!(d.visible_callouts(Some(&second), Some(1)).is_err());
    assert!(second.commit(&mut d, author("local"), 4).is_err());
    assert_eq!(d, before);
}

#[test]
fn malformed_threads_and_tombstones_are_rejected_without_mutation() {
    let mut d = Discussion::default();
    let root = post(&mut d, None, "root", 1);
    let reply = post(&mut d, Some(root), "reply", 2);
    let mut invalid = d.clone();
    invalid.comments.get_mut(&root).unwrap().parent = Some(reply);
    assert!(invalid.validate().unwrap_err().contains("cyclic"));
    invalid = d.clone();
    invalid.comments.get_mut(&reply).unwrap().parent = Some(999);
    assert!(invalid.validate().unwrap_err().contains("missing"));
    invalid = d.clone();
    invalid.comments.get_mut(&root).unwrap().deleted = true;
    assert!(invalid.validate().unwrap_err().contains("retains body"));
    invalid = d.clone();
    invalid.comments.get_mut(&root).unwrap().id = 999;
    assert!(invalid.validate().is_err());
    for bad in [
        body(" "),
        body("bad\0text"),
        body(&"x".repeat(MAX_BODY_BYTES + 1)),
    ] {
        let before = d.clone();
        assert!(d.post(author("local"), None, bad, 3).is_err());
        assert_eq!(d, before);
    }
    let before = d.clone();
    assert!(
        d.post(
            Author {
                id: "".into(),
                name: "name".into()
            },
            None,
            body("text"),
            3
        )
        .is_err()
    );
    assert_eq!(d, before);
}

#[test]
fn marker_links_use_parser_and_invalid_geometry_or_duplicates_are_rejected() {
    let mut d = Discussion::default();
    let mut draft = Draft::new(&d, None).unwrap();
    let before = draft.body.clone();
    assert!(draft.add_callout(&d, Point::new(f32::NAN, 0.), 0).is_err());
    assert_eq!(draft.body, before);
    draft.add_callout(&d, Point::new(-10., 20.), 0).unwrap();
    let id = draft.commit(&mut d, author("local"), 1).unwrap();
    let mut invalid = d.clone();
    let marker = invalid.comments[&id].body.callouts[0].clone();
    invalid
        .comments
        .get_mut(&id)
        .unwrap()
        .body
        .callouts
        .push(marker);
    assert!(invalid.validate().is_err());
    let mut edit = Draft::edit(&d, id, "local").unwrap();
    edit.body.text =
        r"Escaped \[(1)](callout:1) and [site](https://example.invalid/callout:1)".into();
    edit.commit(&mut d, author("local"), 2).unwrap();
    assert!(d.comments[&id].body.callouts.is_empty());
}

#[test]
fn excessive_depth_and_identity_exhaustion_are_atomic_failures() {
    let mut d = Discussion::default();
    let mut parent = None;
    for i in 0..MAX_THREAD_DEPTH {
        parent = Some(post(&mut d, parent, "reply", i as u64));
    }
    let before = d.clone();
    assert!(
        d.post(author("local"), parent, body("too deep"), 100)
            .is_err()
    );
    assert_eq!(d, before);
    d.next_id = u64::MAX;
    let before = d.clone();
    assert!(
        d.post(author("local"), None, body("overflow"), 100)
            .is_err()
    );
    assert_eq!(d, before);
}

#[test]
fn marker_visibility_is_selection_or_edited_thread_and_draft_replaces_stored_body() {
    let mut d = Discussion::default();
    let mut draft = Draft::new(&d, None).unwrap();
    let root_marker = draft.add_callout(&d, Point::new(10., 10.), 0).unwrap();
    let root = draft.commit(&mut d, author("local"), 1).unwrap();
    let mut reply = Draft::new(&d, Some(root)).unwrap();
    let reply_marker = reply.add_callout(&d, Point::new(20., 20.), 0).unwrap();
    let reply_id = reply.commit(&mut d, author("local"), 2).unwrap();
    let mut other = Draft::new(&d, None).unwrap();
    let other_marker = other.add_callout(&d, Point::new(30., 30.), 0).unwrap();
    other.commit(&mut d, author("local"), 3).unwrap();
    assert!(d.visible_callouts(None, None).unwrap().is_empty());
    let mut editing = Draft::edit(&d, reply_id, "local").unwrap();
    editing.body.callouts[0].position = Point::new(50., 50.);
    let shown = d.visible_callouts(Some(&editing), None).unwrap();
    assert_eq!(
        shown.iter().map(|c| c.id).collect::<BTreeSet<_>>(),
        BTreeSet::from([root_marker, reply_marker])
    );
    assert_eq!(
        shown
            .iter()
            .find(|c| c.id == reply_marker)
            .unwrap()
            .position,
        Point::new(50., 50.)
    );
    assert_eq!(
        d.callout(reply_marker).unwrap().position,
        Point::new(20., 20.)
    );
    assert_eq!(
        d.visible_callouts(Some(&editing), Some(other_marker))
            .unwrap()[0]
            .id,
        other_marker
    );
    assert!(
        d.visible_callouts(Some(&editing), Some(999))
            .unwrap()
            .is_empty()
    );
    let mut new = Draft::new(&d, None).unwrap();
    let new_marker = new.add_callout(&d, Point::new(0., 0.), 0).unwrap();
    assert_eq!(
        d.visible_callouts(Some(&new), None)
            .unwrap()
            .iter()
            .map(|c| c.id)
            .collect::<Vec<_>>(),
        vec![new_marker]
    );
}
