// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::collections::BTreeMap;
use wireframe::{
    control_browser::Snapshot,
    document::{FORMAT_VERSION, Id, Point, Project, decode},
    model::Kind,
    review::{Author, Body, Discussion, Draft},
    storage::ProjectStore,
    working_copy::WorkingCopies,
    workspace::{Editor, PageImportChoice},
};
fn author() -> Author {
    Author {
        id: "reviewer-fixture".into(),
        name: "Local reviewer".into(),
    }
}
fn body(text: &str) -> Body {
    Body {
        text: text.into(),
        ..Body::default()
    }
}
fn post(editor: &mut Editor, surface: Id, text: &str) -> Id {
    editor
        .change_review(surface, |d| d.post(author(), None, body(text), 100))
        .unwrap()
}
fn fixture(label: &str) -> (Editor, Id) {
    let mut editor = Editor::default();
    let alternate = editor.create_version("Phone").unwrap();
    post(&mut editor, 1, &format!("{label} official"));
    let mut draft = Draft::new(&Discussion::default(), None).unwrap();
    draft.body = body(&format!("{label} alternate"));
    draft
        .add_callout(
            &Discussion::default(),
            Point::new(25., 35.),
            draft.body.text.len(),
        )
        .unwrap();
    editor
        .change_review(alternate, |d| draft.commit(d, author(), 101))
        .unwrap();
    (Editor::new(editor.document), alternate)
}

#[test]
fn format_migration_rejects_disguised_reviews_and_validates_surface_ownership() {
    let empty = Project::default();
    for version in 2..FORMAT_VERSION {
        let mut old = serde_json::to_value(&empty).unwrap();
        old["format_version"] = version.into();
        let migrated = decode(&serde_json::to_vec(&old).unwrap()).unwrap();
        assert!(migrated.reviews.is_empty());
        assert_eq!(migrated.format_version, FORMAT_VERSION);
    }
    let (e, _) = fixture("Saved");
    let mut old = serde_json::to_value(&e.document).unwrap();
    old["format_version"] = 10.into();
    assert!(
        decode(&serde_json::to_vec(&old).unwrap())
            .unwrap_err()
            .contains("version 11")
    );
    let mut invalid = e.document.clone();
    let review = invalid.reviews.remove(&1).unwrap();
    invalid.reviews.insert(999, review);
    assert!(invalid.validate().unwrap_err().contains("owning page"));
    let mut invalid = e.document.clone();
    invalid
        .reviews
        .get_mut(&1)
        .unwrap()
        .comments
        .get_mut(&1)
        .unwrap()
        .parent = Some(999);
    assert!(invalid.validate().is_err());
}

#[test]
fn native_project_storage_and_working_copy_checkpoint_round_trip_reviews() {
    let (mut e, alternate) = fixture("Persisted");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Review – 世界.wireframe");
    ProjectStore.save(&path, &e.document).unwrap();
    assert_eq!(ProjectStore.load(&path).unwrap(), e.document);
    let copies = WorkingCopies::new(dir.path().join("copies")).unwrap();
    let mut copy = copies.open_named(&path).unwrap();
    e.change_review(alternate, |d| d.mark_liked(1, "reader", true))
        .unwrap();
    copy.checkpoint(&e.document).unwrap();
    drop(copy);
    let reopened = copies.open_named(&path).unwrap();
    assert_eq!(reopened.project(), &e.document);
    assert_ne!(ProjectStore.load(&path).unwrap(), e.document);
}

#[test]
fn drawing_history_preserves_post_edit_delete_and_flags_without_adding_review_entries() {
    let mut e = Editor::default();
    e.add(Kind::Button, 10., 20.);
    let undo = e.undo_name().map(str::to_owned);
    let id = post(&mut e, 1, "First feedback");
    assert_eq!(e.undo_name(), undo.as_deref());
    e.mark_saved();
    e.change_review(1, |d| {
        d.update(id, "reviewer-fixture", body("Updated feedback"), 101)
    })
    .unwrap();
    assert!(e.dirty());
    e.change_review(1, |d| d.mark_liked(id, "reader", true))
        .unwrap();
    let review = e.document.reviews.clone();
    e.undo();
    assert!(e.document.pages[0].content.nodes.is_empty());
    assert_eq!(e.document.reviews, review);
    e.redo();
    assert_eq!(e.document.pages[0].content.nodes.len(), 1);
    assert_eq!(e.document.reviews, review);
    let cache = e.change_review(1, |d| d.delete(id)).unwrap();
    let deleted = e.document.reviews.clone();
    e.undo();
    e.redo();
    assert_eq!(e.document.reviews, deleted);
    assert!(
        !String::from_utf8(e.document.encode().unwrap())
            .unwrap()
            .contains("Updated feedback")
    );
    e.change_review(1, |d| d.restore_deleted(&cache)).unwrap();
    assert_eq!(
        e.document.reviews[&1].comments[&id].body.text,
        "Updated feedback"
    );
}

#[test]
fn undo_resource_creation_refuses_feedback_loss_without_advancing_history() {
    let mut e = Editor::default();
    e.add_page();
    let page = e.page;
    post(&mut e, page, "Keep me");
    let before = e.document.clone();
    let undo = e.undo_name().map(str::to_owned);
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), undo.as_deref());
    assert!(e.error.as_ref().unwrap().contains("review comments"));
    let mut e = Editor::default();
    let alternate = e.create_version("Phone").unwrap();
    post(&mut e, alternate, "Alternate feedback");
    let before = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), Some("Create version"));
    e.change_review(alternate, |d| d.delete(1)).unwrap();
    let before = e.document.clone();
    e.undo();
    assert_eq!(
        e.document, before,
        "Deleted records still belong to the alternate"
    );
}

#[test]
fn duplicate_and_create_do_not_clone_discussions_promote_and_merge_keep_scope() {
    let (mut e, alternate) = fixture("Source");
    let reviews = e.document.reviews.clone();
    let duplicate = e.duplicate_page(1).unwrap();
    assert!(!e.document.reviews.contains_key(&duplicate));
    assert_eq!(e.document.reviews, reviews);
    e.switch_page(1);
    e.switch_version(1, None).unwrap();
    let created = e.create_version("Tablet").unwrap();
    assert!(!e.document.reviews.contains_key(&created));
    e.promote_version(alternate).unwrap();
    assert_eq!(e.document.reviews, reviews);
    e.merge_version(alternate).unwrap();
    assert_eq!(e.document.reviews, reviews);
    e.document.validate().unwrap();
}

#[test]
fn discard_version_recovers_discussion_but_redo_refuses_later_feedback() {
    let (mut e, alternate) = fixture("Discard");
    let before = e.document.clone();
    e.discard_version(alternate).unwrap();
    assert!(!e.document.reviews.contains_key(&alternate));
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    e.undo();
    post(&mut e, alternate, "Later feedback");
    let changed = e.document.clone();
    e.redo();
    assert_eq!(e.document, changed);
    assert!(e.error.as_ref().unwrap().contains("comments have changed"));
}

#[test]
fn trash_retains_discussions_and_commented_placeholder_survives_undo() {
    let (mut e, _) = fixture("Trash");
    let reviews = e.document.reviews.clone();
    e.trash_pages(&[1]).unwrap();
    assert_eq!(e.document.reviews, reviews);
    assert!(
        e.change_review(1, |d| d.post(author(), None, body("no"), 200))
            .is_err()
    );
    let placeholder = e.page;
    post(&mut e, placeholder, "New feedback");
    e.undo();
    assert!(e.document.live_page(1).is_some());
    assert!(e.document.live_page(placeholder).is_some());
    assert!(e.document.has_review_comments(placeholder));
    e.document.validate().unwrap();
}

#[test]
fn permanent_page_removal_purges_reviews_and_history_cannot_resurrect_them() {
    let (mut e, alternate) = fixture("Secret payload");
    e.add(Kind::Button, 0., 0.);
    e.add_page();
    let survivor = e.page;
    post(&mut e, survivor, "Survivor");
    e.trash_pages(&[1]).unwrap();
    e.rename_page(survivor, "Renamed survivor").unwrap();
    e.permanently_remove_pages(&[1]).unwrap();
    for _ in 0..8 {
        assert!(!e.document.reviews.contains_key(&1));
        assert!(!e.document.reviews.contains_key(&alternate));
        assert!(
            !String::from_utf8(e.document.encode().unwrap())
                .unwrap()
                .contains("Secret payload")
        );
        e.undo();
    }
    for _ in 0..8 {
        e.redo();
        assert!(!e.document.reviews.contains_key(&1));
        assert!(!e.document.reviews.contains_key(&alternate));
        e.document.validate().unwrap();
    }
}

#[test]
fn all_import_choices_reconcile_page_and_alternate_discussions_with_exact_history() {
    let (source, source_alt) = fixture("Incoming");
    for choice in [
        PageImportChoice::UseNew,
        PageImportChoice::UseExisting,
        PageImportChoice::KeepBoth,
    ] {
        let (mut target, local_alt) = fixture("Local");
        let local_extra = target.create_version("Local only").unwrap();
        post(&mut target, local_extra, "Untouched branch");
        let before = target.document.clone();
        let plan = target
            .plan_project_page_import(&source.document, "source-origin", &[1])
            .unwrap();
        let prepared = plan
            .review_libraries(&BTreeMap::from([(1, choice)]), &BTreeMap::new())
            .unwrap()
            .prepare(&BTreeMap::new())
            .unwrap();
        let imported = prepared.imported_pages()[0];
        target.apply_project_copy(prepared).unwrap();
        let after = target.document.clone();
        assert_eq!(after.reviews[&local_extra], before.reviews[&local_extra]);
        if choice == PageImportChoice::UseExisting {
            assert_eq!(after.reviews, before.reviews);
        } else {
            let page = after.live_page(imported).unwrap();
            let alt = page
                .alternates
                .iter()
                .find(|a| a.name == "Phone")
                .unwrap()
                .id;
            assert_eq!(
                after.reviews[&imported].comments[&1].body,
                source.document.reviews[&1].comments[&1].body
            );
            assert_eq!(
                after.reviews[&alt].comments[&1].body,
                source.document.reviews[&source_alt].comments[&1].body
            );
            assert!(after.reviews[&alt].comments[&1].imported);
            assert_eq!(after.reviews[&alt].comments[&1].author, author());
            if choice == PageImportChoice::UseNew {
                assert_eq!(alt, local_alt);
            } else {
                assert_ne!(imported, 1);
                assert_eq!(after.reviews[&1], before.reviews[&1]);
            }
        }
        target.undo();
        assert_eq!(target.document, before);
        target.redo();
        assert_eq!(target.document, after);
        assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
    }
}

#[test]
fn import_undo_protects_later_feedback_and_reuse_is_noop_after_reopen() {
    let (source, _) = fixture("Incoming");
    let mut e = Editor::default();
    let prepared = e
        .prepare_project_copy(&source.document, "source-origin", &[1])
        .unwrap();
    let page = prepared.imported_pages()[0];
    e.apply_project_copy(prepared).unwrap();
    post(&mut e, page, "Later feedback");
    let before = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    assert!(e.error.as_ref().unwrap().contains("comments have changed"));
    let mut e = Editor::new(decode(&e.document.encode().unwrap()).unwrap());
    let plan = e
        .plan_project_page_import(&source.document, "source-origin", &[1])
        .unwrap();
    e.apply_project_copy(
        plan.prepare(&BTreeMap::from([(1, PageImportChoice::UseExisting)]))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(e.document, before);
    assert!(e.undo_name().is_none());
}

#[test]
fn comment_only_pages_are_importable_and_empty_incoming_review_clears_use_new() {
    let (e, _) = fixture("Only comments");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("review.wireframe");
    ProjectStore.save(&path, &e.document).unwrap();
    assert_eq!(Snapshot::local(&path).unwrap().import_pages(), vec![1]);
    let mut e = Editor::new(e.document);
    let source = Project::default();
    let plan = e.plan_project_page_import(&source, "blank", &[1]).unwrap();
    e.apply_project_copy(
        plan.prepare(&BTreeMap::from([(1, PageImportChoice::UseNew)]))
            .unwrap(),
    )
    .unwrap();
    assert!(!e.document.reviews.contains_key(&1));
    assert_eq!(
        e.document.reviews.len(),
        1,
        "Omitted local alternate review remains"
    );
}

#[test]
fn failed_review_change_leaves_document_and_drawing_history_intact() {
    let mut e = Editor::default();
    e.add(Kind::Button, 0., 0.);
    let before = e.document.clone();
    let name = e.undo_name().map(str::to_owned);
    assert!(
        e.change_review(1, |d| {
            d.next_id = 0;
            Ok(())
        })
        .is_err()
    );
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), name.as_deref());
    assert!(
        e.change_review(999, |d| d.post(author(), None, body("invalid surface"), 1))
            .is_err()
    );
    assert_eq!(e.document, before);
}

#[test]
fn review_link_import_maps_kept_alternates_and_omitted_pages_without_touching_literals() {
    let (mut source, source_alt) = fixture("Source");
    source.add_page();
    let related = source.page;
    source.add_page();
    let omitted = source.page;
    let text = format!(
        "*See [phone](surface:{source_alt})* and [missing](surface:{omitted}) \\[literal](surface:{source_alt}) [site](https://example.invalid/surface:{source_alt})"
    );
    post(&mut source, related, &text);
    let (target, local_alt) = fixture("Local");
    let mut target = Editor::new(target.document);
    let prepared = target
        .plan_project_page_import(&source.document, "links", &[1, related])
        .unwrap()
        .review_libraries(
            &BTreeMap::from([(1, PageImportChoice::UseExisting)]),
            &BTreeMap::new(),
        )
        .unwrap()
        .prepare(&BTreeMap::new())
        .unwrap();
    let target_page = prepared.imported_pages()[1];
    target.apply_project_copy(prepared).unwrap();
    let imported = &target.document.reviews[&target_page].comments[&1].body;
    let references = imported.surface_references().unwrap();
    assert!(references.contains(&local_alt));
    let missing = *references.iter().find(|id| **id != local_alt).unwrap();
    assert!(target.document.removed_pages.contains(&missing));
    assert!(!target.document.review_surfaces().any(|id| id == missing));
    assert!(
        imported
            .text
            .contains(&format!("\\[literal](surface:{source_alt})"))
    );
    assert!(
        imported
            .text
            .contains(&format!("https://example.invalid/surface:{source_alt}"))
    );
    assert_eq!(
        target.document.reviews[&1].comments[&1].body.text,
        "Local official"
    );
    target.document.validate().unwrap();
}

#[test]
fn link_target_rewriting_preserves_nested_formatting_unicode_and_escaped_markup() {
    let text = "é *[bold](surface:2)* {color:red}[red](surface:3){color} \\[literal](surface:2) [web](https://site.invalid/surface:2)";
    let rewritten = wireframe::rich_text::rewrite_link_targets(text, |target| match target {
        "surface:2" => Some("surface:100".into()),
        "surface:3" => Some("surface:101".into()),
        _ => None,
    })
    .unwrap();
    assert_eq!(
        rewritten,
        "é *[bold](surface:100)* {color:red}[red](surface:101){color} \\[literal](surface:2) [web](https://site.invalid/surface:2)"
    );
}

#[test]
fn undo_of_review_link_destination_keeps_feedback_and_reserves_broken_identity() {
    let mut e = Editor::default();
    e.add_page();
    let destination = e.page;
    post(
        &mut e,
        1,
        &format!("See [destination](surface:{destination})"),
    );
    e.undo();
    assert!(e.document.live_page(destination).is_none());
    assert!(
        e.document.reviews[&1].comments[&1]
            .body
            .surface_references()
            .unwrap()
            .contains(&destination)
    );
    assert!(e.document.removed_pages.contains(&destination));
    assert!(e.document.next_id > destination);
    e.redo();
    assert!(e.document.live_page(destination).is_some());
    e.undo();
    e.add_page();
    assert_ne!(e.page, destination);
    e.document.validate().unwrap();
}
