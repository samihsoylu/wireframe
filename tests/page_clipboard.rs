// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::collections::{BTreeMap, HashSet};
use wireframe::{
    document::*,
    model::Kind,
    workspace::{Clipboard, Editor, PageImportChoice},
};

fn fixture() -> Editor {
    let mut p = Project {
        font: None,
        next_id: 100,
        ..Project::default()
    };
    let template = p.pages[0].clone();
    p.pages = ["Landing", "Contact", "Private"]
        .iter()
        .enumerate()
        .map(|(i, name)| Page {
            id: i as Id + 1,
            name: (*name).into(),
            notes: format!("{name} notes"),
            ..template.clone()
        })
        .collect();
    p.versions.push(NamedVersion {
        id: 10,
        name: "Phone".into(),
    });
    p.pages[0].alternates.push(Alternate {
        id: 11,
        name: "Phone".into(),
        version: Some(10),
        notes: "Phone notes".into(),
        content: Content::default(),
    });
    let mut button = Node::control(20, "button", Point::new(30., 40.), Size::default());
    button
        .properties
        .insert("link".into(), Value::Link(Link::Page(1)));
    p.pages[0].content.nodes.push(button.clone());
    button
        .properties
        .insert("link".into(), Value::Link(Link::Page(3)));
    p.pages[0].alternates[0].content.nodes.push(button);
    p.validate().unwrap();
    Editor::new(p)
}
fn paste(e: &mut Editor, copy: &Clipboard, selected: &[Id], collapsed: &[Id]) -> Vec<Id> {
    let prepared = e
        .prepare_same_project_page_paste(
            copy,
            "origin",
            selected,
            &collapsed.iter().copied().collect(),
        )
        .unwrap();
    e.apply_page_paste(prepared).unwrap()
}

#[test]
fn copy_round_trip_has_only_selected_surfaces_and_content_free_link_targets() {
    let e = fixture();
    let copy = e.copy_pages(&[1], "origin").unwrap();
    assert_eq!(copy.len(), 1);
    assert!(!copy.is_empty());
    let text = serde_json::to_string(&copy).unwrap();
    assert!(!text.contains("Private notes"));
    assert!(!text.contains("Contact notes"));
    let decoded: Clipboard = serde_json::from_str(&text).unwrap();
    let source = decoded.page_payload().unwrap().project();
    assert_eq!(source.pages, vec![e.document.pages[0].clone()]);
    assert_eq!(
        source.removed_pages.iter().copied().collect::<Vec<_>>(),
        vec![3]
    );
    assert_eq!(source.versions, e.document.versions);
}

#[test]
fn same_project_copy_keeps_links_all_versions_and_current_view_with_exact_history() {
    let mut e = fixture();
    e.switch_version(1, Some(11)).unwrap();
    let copy = e.copy_pages(&[1], "origin").unwrap();
    let before = e.document.clone();
    let ids = paste(&mut e, &copy, &[1], &[]);
    assert_eq!(e.page, 1);
    assert_eq!(e.active_alternate(1), Some(11));
    let p = &e.document.pages[1];
    assert_eq!(p.id, ids[0]);
    assert_eq!(p.name, "Landing (1)");
    assert_ne!(p.alternates[0].id, 11);
    assert_eq!(p.content, before.pages[0].content);
    assert_eq!(
        p.alternates[0].content,
        before.pages[0].alternates[0].content
    );
    assert_eq!(p.alternates[0].notes, "Phone notes");
    assert_eq!(p.alternates[0].version, Some(10));
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
    assert_eq!(paste(&mut e, &copy, &[1], &[]).len(), 1);
    assert_eq!(e.document.pages[1].name, "Landing (2)");
}

#[test]
fn cut_is_not_trash_and_paste_restores_absent_identities_without_overwriting_repeated_paste() {
    let mut e = fixture();
    let copy = e.copy_pages(&[1], "origin").unwrap();
    let before = e.document.clone();
    e.cut_pages(&[1]).unwrap();
    assert_eq!(e.page, 2);
    assert!(e.retained_resources().next().is_none());
    assert!(e.document.removed_pages.contains(&1));
    assert!(e.document.removed_pages.contains(&11));
    let cut = e.document.clone();
    let ids = paste(&mut e, &copy, &[2], &[]);
    assert_eq!(ids, vec![1]);
    assert_eq!(e.document.pages[1], before.pages[0]);
    assert_eq!(e.page, 2);
    assert!(!e.document.removed_pages.contains(&1));
    assert!(!e.document.removed_pages.contains(&11));
    e.undo();
    assert_eq!(e.document, cut);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, cut);
    e.redo();
    assert_eq!(e.document.pages[1], before.pages[0]);
    let new = paste(&mut e, &copy, &[2], &[]);
    assert_ne!(new, vec![1]);
    assert_eq!(e.document.live_page(1).unwrap(), &before.pages[0]);
}

#[test]
fn last_page_cut_creates_blank_and_undo_restores_exact_document() {
    let mut e = fixture();
    let before = e.document.clone();
    let copy = e.copy_pages(&[3, 1, 2], "origin").unwrap();
    e.cut_pages(&[1, 2, 3]).unwrap();
    assert_eq!(e.document.live_pages().count(), 1);
    let blank = e.page;
    assert!(
        e.document
            .live_page(blank)
            .unwrap()
            .content
            .nodes
            .is_empty()
    );
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(paste(&mut e, &copy, &[blank], &[]), vec![1, 2, 3]);
    assert_eq!(e.page, blank);
}

#[test]
fn cut_parent_adopts_live_children_and_detaches_retained_children_without_losing_data() {
    let mut e = fixture();
    e.document.pages[1].parent = Some(1);
    e.document.pages[2].parent = Some(1);
    e.document.pages[2].trashed = true;
    let before = e.document.clone();
    e.cut_pages(&[1]).unwrap();
    assert_eq!(e.document.pages[0].parent, None);
    assert_eq!(e.document.pages[1].parent, None);
    assert_eq!(e.document.pages[1].notes, "Private notes");
    assert!(e.document.pages[1].trashed);
    e.undo();
    assert_eq!(e.document, before);
}

#[test]
fn expanded_parent_paste_inserts_first_child_and_collapsed_subtree_paste_inserts_siblings() {
    let mut e = fixture();
    e.document.pages[1].parent = Some(1);
    let copy = e.copy_pages(&[1, 2], "origin").unwrap();
    let ids = paste(&mut e, &copy, &[1], &[]);
    assert_eq!(
        e.document.pages.iter().map(|p| p.id).collect::<Vec<_>>(),
        vec![1, ids[0], ids[1], 2, 3]
    );
    assert_eq!(e.document.pages[1].parent, Some(1));
    assert_eq!(e.document.pages[2].parent, Some(ids[0]));
    e.undo();
    let ids = paste(&mut e, &copy, &[1, 2], &[1]);
    assert_eq!(
        e.document.pages.iter().map(|p| p.id).collect::<Vec<_>>(),
        vec![1, 2, ids[0], ids[1], 3]
    );
    assert_eq!(e.document.pages[2].parent, None);
    assert_eq!(e.document.pages[3].parent, Some(ids[0]));
}

#[test]
fn occupied_page_or_alternate_in_trash_gets_new_identity() {
    let mut e = fixture();
    let copy = e.copy_pages(&[1], "origin").unwrap();
    e.trash_pages(&[1]).unwrap();
    let ids = paste(&mut e, &copy, &[2], &[]);
    let p = e.document.live_page(ids[0]).unwrap();
    assert_ne!(p.id, 1);
    assert_ne!(p.alternates[0].id, 11);
    assert!(e.document.pages.iter().any(|p| p.id == 1 && p.trashed));
}

#[test]
fn saved_or_renamed_session_keeps_identity_but_foreign_project_requires_review() {
    let mut e = fixture();
    let copy = e.copy_pages(&[1], "old-path").unwrap();
    let prepared = e
        .prepare_same_project_page_paste(&copy, "new-path", &[1], &HashSet::new())
        .unwrap();
    e.apply_page_paste(prepared).unwrap();
    let reopened = Editor::new(e.document.clone());
    assert!(
        reopened
            .prepare_same_project_page_paste(&copy, "old-path", &[1], &HashSet::new())
            .is_ok()
    );
    assert!(
        reopened
            .prepare_same_project_page_paste(&copy, "other-path", &[1], &HashSet::new())
            .is_err()
    );
}

#[test]
fn stale_preparation_and_mixed_or_overbroad_payloads_leave_document_unchanged() {
    let mut e = fixture();
    let copy = e.copy_pages(&[1], "origin").unwrap();
    let prepared = e
        .prepare_same_project_page_paste(&copy, "origin", &[1], &HashSet::new())
        .unwrap();
    e.rename_page(2, "Changed").unwrap();
    let before = e.document.clone();
    assert!(e.apply_page_paste(prepared).is_err());
    assert_eq!(e.document, before);
    let mut mixed = copy.clone();
    mixed
        .nodes
        .push(e.document.pages[0].content.nodes[0].clone());
    assert!(mixed.page_payload().is_err());
    let mut json = serde_json::to_value(&copy).unwrap();
    json["pages"]["project"]["pages"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::to_value(&e.document.pages[2]).unwrap());
    let invalid: Clipboard = serde_json::from_value(json).unwrap();
    assert!(invalid.page_payload().is_err());
    assert!(e.copy_pages(&[1, 1], "origin").is_err());
    assert!(e.cut_pages(&[1, 999]).is_err());
    assert_eq!(e.document, before);
}

#[test]
fn component_closure_omits_unused_sources_and_preserves_current_dependency_on_same_project_paste() {
    let mut e = fixture();
    e.add(Kind::Button, 1., 2.);
    let used = e.create_component("Used".into()).unwrap();
    e.switch_page(3);
    e.add(Kind::Button, 10., 20.);
    let unused = e.create_component("Private component".into()).unwrap();
    let copy = e.copy_pages(&[1], "origin").unwrap();
    let source = copy.page_payload().unwrap().project();
    assert_eq!(
        source.definitions.iter().map(|d| d.id).collect::<Vec<_>>(),
        vec![used]
    );
    assert!(
        !source
            .libraries
            .iter()
            .flat_map(|l| &l.sources)
            .any(|s| s.definition == unused)
    );
    e.document
        .definitions
        .iter_mut()
        .find(|d| d.id == used)
        .unwrap()
        .name = "Edited after copy".into();
    let definitions = e.document.definitions.clone();
    let libraries = e.document.libraries.clone();
    paste(&mut e, &copy, &[2], &[]);
    assert_eq!(e.document.definitions, definitions);
    assert_eq!(e.document.libraries, libraries);
}

#[test]
fn retained_image_bytes_are_not_published_and_remain_unavailable_after_paste() {
    let mut e = fixture();
    e.document.assets.push(Asset {
        id: 30,
        trashed: true,
        name: "Private image".into(),
        media_type: "image/svg+xml".into(),
        bytes: br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"/>"#.to_vec(),
    });
    let mut image = Node::control(31, "image", Point::default(), Size::default());
    image.properties.insert("image".into(), Value::Asset(30));
    e.document.pages[0].content.nodes.push(image);
    let copy = e.copy_pages(&[1], "origin").unwrap();
    let project = copy.page_payload().unwrap().project();
    assert!(project.assets.is_empty());
    assert!(project.removed_assets.contains(&30));
    let assets = e.document.assets.clone();
    paste(&mut e, &copy, &[2], &[]);
    assert_eq!(e.document.assets, assets);
    assert!(e.document.live_asset(30).is_none());
}

#[test]
fn foreign_import_retargets_selected_pages_keeps_omitted_destinations_missing_and_uses_target_font()
{
    let source = fixture();
    let copy = source.copy_pages(&[1, 2], "foreign").unwrap();
    let mut target = Editor::default();
    let before = target.document.clone();
    let plan = target.plan_page_clipboard_import(&copy).unwrap();
    let prepared = plan
        .review_libraries(&BTreeMap::new(), &BTreeMap::new())
        .unwrap()
        .prepare(&BTreeMap::new())
        .unwrap();
    let prepared = target
        .position_page_clipboard_import(&copy, prepared, &[1], &HashSet::new())
        .unwrap();
    let ids = target.apply_page_paste(prepared).unwrap();
    let p = target.document.live_page(ids[0]).unwrap();
    assert_eq!(
        p.content.nodes[0].properties["link"],
        Value::Link(Link::Page(ids[0]))
    );
    let Value::Link(Link::Page(missing)) = p.alternates[0].content.nodes[0].properties["link"]
    else {
        panic!()
    };
    assert_ne!(missing, 3);
    assert!(target.document.removed_pages.contains(&missing));
    assert_eq!(target.document.font, before.font);
    assert_eq!(target.page, 1);
    let after = target.document.clone();
    target.undo();
    assert_eq!(target.document, before);
    target.redo();
    assert_eq!(target.document, after);
    let repeat = target.plan_page_clipboard_import(&copy).unwrap();
    let choices = BTreeMap::from([
        (1, PageImportChoice::UseExisting),
        (2, PageImportChoice::UseExisting),
    ]);
    let prepared = repeat
        .review_libraries(&choices, &BTreeMap::new())
        .unwrap()
        .prepare(&BTreeMap::new())
        .unwrap();
    let prepared = target
        .position_page_clipboard_import(&copy, prepared, &[1], &HashSet::new())
        .unwrap();
    target.apply_page_paste(prepared).unwrap();
    assert_eq!(target.document, after);
}

#[test]
fn copying_a_numbered_page_fills_the_first_available_suffix_without_nesting() {
    let mut e = fixture();
    e.rename_page(1, "Landing (3)").unwrap();
    e.rename_page(2, "LANDING (1)").unwrap();
    let copy = e.copy_pages(&[1], "origin").unwrap();
    let ids = paste(&mut e, &copy, &[1], &[]);
    assert_eq!(e.document.live_page(ids[0]).unwrap().name, "Landing (2)");
}

#[test]
fn dependency_closure_includes_images_used_only_inside_a_component() {
    let mut e = fixture();
    e.add(Kind::Button, 1., 2.);
    e.create_component("Card".into()).unwrap();
    let asset = e.document.allocate_id().unwrap();
    e.document.assets.push(Asset {
        id: asset,
        trashed: false,
        name: "Placement icon".into(),
        media_type: "image/svg+xml".into(),
        bytes: br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"/>"#.to_vec(),
    });
    e.document.definitions[0].content.nodes[0]
        .properties
        .insert("icon".into(), Value::Icon(IconSource::Asset(asset)));
    e.document.validate().unwrap();
    let copy = e.copy_pages(&[1], "origin").unwrap();
    let payload = copy.page_payload().unwrap();
    assert_eq!(payload.project().assets.len(), 1);
    assert_eq!(payload.project().assets[0].id, asset);
    let target = Editor::default();
    let plan = target.plan_page_clipboard_import(&copy).unwrap();
    let prepared = plan
        .review_libraries(&BTreeMap::new(), &BTreeMap::new())
        .unwrap()
        .prepare(&BTreeMap::new())
        .unwrap();
    prepared.preview().validate().unwrap();
}

#[test]
fn paste_rebuilds_discontiguous_depths_and_applies_depth_limit_before_placing_children() {
    let mut e = fixture();
    let template = e.document.pages[0].clone();
    e.document.pages = (1..=8)
        .map(|id| Page {
            id,
            name: format!("Page {id}"),
            parent: match id {
                2..=5 => Some(id - 1),
                7 => Some(6),
                _ => None,
            },
            content: Content::default(),
            alternates: vec![],
            ..template.clone()
        })
        .collect();
    let copy = e.copy_pages(&[4, 5], "origin").unwrap();
    let ids = paste(&mut e, &copy, &[6], &[]);
    // Original depths 3/4 plus destination offset 1 both clamp to 4.
    assert_eq!(e.document.live_page(ids[0]).unwrap().parent, Some(6));
    assert_eq!(e.document.live_page(ids[1]).unwrap().parent, Some(6));
    e.undo();
    let copy = e.copy_pages(&[4, 5, 6, 7], "origin").unwrap();
    let ids = paste(&mut e, &copy, &[8], &[]);
    let parents: Vec<_> = ids
        .iter()
        .map(|id| e.document.live_page(*id).unwrap().parent)
        .collect();
    assert_eq!(parents, vec![None, Some(ids[0]), None, Some(ids[2])]);
}

#[test]
fn official_and_alternate_discussions_follow_pages_and_survive_cut_undo_and_reopen() {
    use wireframe::review::{Author, Body};
    let mut e = fixture();
    for surface in [1, 11, 3] {
        e.change_review(surface, |d| {
            d.post(
                Author {
                    id: "local-reviewer".into(),
                    name: "Reviewer".into(),
                },
                None,
                Body {
                    text: "See [private](surface:3)".into(),
                    callouts: vec![],
                },
                1,
            )
        })
        .unwrap();
    }
    let before = e.document.clone();
    let copy = e.copy_pages(&[1], "origin").unwrap();
    assert_eq!(copy.page_payload().unwrap().project().reviews.len(), 2);
    let ids = paste(&mut e, &copy, &[2], &[]);
    let alternate = e.document.live_page(ids[0]).unwrap().alternates[0].id;
    assert_eq!(e.document.reviews[&ids[0]], before.reviews[&1]);
    assert_eq!(e.document.reviews[&alternate], before.reviews[&11]);
    e.undo();
    assert_eq!(e.document, before);
    e.cut_pages(&[1]).unwrap();
    assert_eq!(e.document.reviews.len(), 1);
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
    e.undo();
    assert_eq!(e.document, before);
}
