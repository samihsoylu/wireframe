// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::collections::HashMap;
use wireframe::{document::*, export_plan, model::Kind, pdf, workspace::Editor};

fn content_fixture() -> Project {
    let mut p = Project::default();
    let mut first = Node::control(2, "button", Point::new(10., 20.), Size::fixed(80., 30.));
    first
        .properties
        .insert("text".into(), Value::Text("Original".into()));
    first
        .properties
        .insert("link".into(), Value::Link(Link::Page(1)));
    let second = Node::control(3, "label", Point::new(5., 60.), Size::fixed(80., 30.));
    let mut group = Node::group(4, Point::new(100., 200.), vec![first, second]);
    group.locked = true;
    group.crop = Some(Crop {
        x: 0.1,
        y: 0.2,
        width: 0.8,
        height: 0.6,
    });
    let leaf = Node::control(5, "button", Point::new(300., 100.), Size::fixed(100., 30.));
    p.pages[0].content = Content {
        nodes: vec![group, leaf],
        paint_order: vec![vec![4, 2], vec![5], vec![4, 3]],
    };
    p.pages[0].notes = "Official notes".into();
    p.next_id = 10;
    p.validate().unwrap();
    p
}
fn set_notes(e: &mut Editor, notes: &str) {
    e.edit("Edit notes", |e| e.set_notes(notes.into()));
}

#[test]
fn version_two_migration_keeps_ambiguous_labels_independent_and_round_trips() {
    let mut p = content_fixture();
    let mut second = p.pages[0].clone();
    second.id = 20;
    second.notes = "Second official".into();
    p.pages.push(second);
    for (i, page) in p.pages.iter_mut().enumerate() {
        page.alternates.push(Alternate {
            id: 30 + i as u64,
            name: "Compact".into(),
            version: None,
            notes: String::new(),
            content: page.content.clone(),
        });
    }
    p.next_id = 40;
    let mut old = serde_json::to_value(&p).unwrap();
    old["format_version"] = 2.into();
    for page in old["pages"].as_array_mut().unwrap() {
        for alt in page["alternates"].as_array_mut().unwrap() {
            alt.as_object_mut().unwrap().remove("notes");
        }
    }
    let migrated = decode(&serde_json::to_vec(&old).unwrap()).unwrap();
    assert_eq!(migrated.format_version, FORMAT_VERSION);
    assert!(migrated.versions.is_empty());
    for (original, page) in p.pages.iter().zip(&migrated.pages) {
        assert_eq!(page.content, original.content);
        assert_eq!(page.alternates[0].content, original.alternates[0].content);
        assert_eq!(page.alternates[0].id, original.alternates[0].id);
        assert_eq!(page.alternates[0].notes, page.notes);
        assert_eq!(page.alternates[0].version, None);
    }
    assert_eq!(migrated.font, p.font);
    assert_eq!(decode(&migrated.encode().unwrap()).unwrap(), migrated);
    let mut e = Editor::new(migrated);
    e.rename_version(30, "Focused").unwrap();
    assert_eq!(e.document.pages[1].alternates[0].name, "Compact");
}

#[test]
fn editing_and_undo_stay_on_the_correct_surface_without_coupling_content() {
    let original = content_fixture();
    let mut e = Editor::new(original.clone());
    let alternate = e.create_version("Compact").unwrap();
    set_notes(&mut e, "Compact notes");
    e.add(Kind::Button, 500., 300.);
    assert_eq!(e.page_data().content, original.pages[0].content);
    assert_eq!(e.page_data().notes, "Official notes");
    assert_eq!(e.current().notes, "Compact notes");
    assert_eq!(e.nodes().len(), 3);
    e.switch_version(1, None).unwrap();
    e.undo();
    assert_eq!(e.surface_id(), alternate);
    assert_eq!(e.nodes().len(), 2);
    e.undo();
    assert_eq!(e.current().notes, "Official notes");
    e.redo();
    assert_eq!(e.current().notes, "Compact notes");
    e.redo();
    assert_eq!(e.nodes().len(), 3);
    let clone = e.create_version("Detailed").unwrap();
    set_notes(&mut e, "Detailed notes");
    e.switch_version(1, Some(alternate)).unwrap();
    assert_eq!(e.current().notes, "Compact notes");
    e.switch_version(1, Some(clone)).unwrap();
    assert_eq!(e.current().notes, "Detailed notes");
    e.document.validate().unwrap();
}

#[test]
fn shared_rename_move_conflict_and_project_selection_match_multi_page_workflow() {
    let mut e = Editor::default();
    let landing = e.page;
    let compact_landing = e.create_version("Compact").unwrap();
    e.add_page();
    let contact = e.page;
    let compact_contact = e.create_version("Compact").unwrap();
    let group = e.page_data().alternates[0].version.unwrap();
    assert_eq!(e.document.versions.len(), 1);
    e.rename_version(compact_contact, "Focused").unwrap();
    assert!(
        e.document
            .pages
            .iter()
            .all(|p| p.alternates[0].name == "Focused")
    );
    e.undo();
    assert!(
        e.document
            .pages
            .iter()
            .all(|p| p.alternates[0].name == "Compact")
    );
    e.redo();
    e.add_page();
    let order = e.page;
    let detailed_order = e.create_version("Detailed").unwrap();
    e.switch_version(landing, None).unwrap();
    e.select_project_version(Some(group)).unwrap();
    assert_eq!(e.active_surface(landing), compact_landing);
    assert_eq!(e.active_surface(contact), compact_contact);
    assert_eq!(e.active_surface(order), detailed_order);
    e.switch_page(contact);
    set_notes(&mut e, "Contact compact notes");
    let content = e.page_data().alternates[0].content.clone();
    e.rename_version(compact_contact, "Detailed").unwrap();
    assert_eq!(e.current().notes, "Contact compact notes");
    assert_eq!(e.page_data().alternates[0].content, content);
    assert_eq!(e.document.pages[0].alternates[0].name, "Focused");
    e.undo();
    assert_eq!(e.page_data().alternates[0].version, Some(group));
    e.redo();
    e.create_version("Focused").unwrap();
    let before = e.document.clone();
    assert!(e.rename_version(compact_contact, "Focused").is_err());
    assert_eq!(e.document, before);
}

#[test]
fn selection_preferences_do_not_dirty_or_enter_history_and_stale_ids_are_ignored() {
    let mut e = Editor::default();
    let first = e.page;
    let alt = e.create_version("Compact").unwrap();
    e.add_page();
    let second = e.page;
    let other = e.create_version("Detailed").unwrap();
    e.mark_saved();
    let undo = e.undo_name().map(str::to_owned);
    e.switch_page(first);
    assert_eq!(e.surface_id(), alt);
    e.switch_version(first, None).unwrap();
    e.switch_page(second);
    assert_eq!(e.surface_id(), other);
    assert!(!e.dirty());
    assert_eq!(e.undo_name(), undo.as_deref());
    let saved = e.version_selections();
    let mut reopened = Editor::new(decode(&e.document.encode().unwrap()).unwrap());
    reopened.restore_version_selections(saved);
    reopened.switch_page(second);
    assert_eq!(reopened.surface_id(), other);
    assert!(!reopened.dirty());
    reopened.restore_version_selections(HashMap::from([(first, other), (second, 999), (999, alt)]));
    assert!(reopened.version_selections().is_empty());
    assert_eq!(reopened.surface_id(), second);
    assert!(reopened.switch_version(first, Some(other)).is_err());
    assert_eq!(reopened.surface_id(), second);
}

#[test]
fn promotion_preserves_former_official_and_alternate_with_atomic_undo() {
    let original = content_fixture();
    let mut e = Editor::new(original.clone());
    let alt = e.create_version("Compact").unwrap();
    set_notes(&mut e, "Compact notes");
    e.add(Kind::Button, 600., 200.);
    let before = e.document.clone();
    e.promote_version(alt).unwrap();
    assert_eq!(e.surface_id(), alt);
    assert_eq!(e.page_data().content, before.pages[0].alternates[0].content);
    assert_eq!(e.page_data().notes, "Compact notes");
    assert_eq!(e.page_data().alternates[0], before.pages[0].alternates[0]);
    let former = &e.page_data().alternates[1];
    assert_eq!(former.content, original.pages[0].content);
    assert_eq!(former.notes, "Official notes");
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
}

#[test]
fn merge_preserves_nested_clips_locks_links_and_interleaved_order_with_fresh_root_ids() {
    let original = content_fixture();
    let mut e = Editor::new(original.clone());
    let alt = e.create_version("Compact").unwrap();
    set_notes(&mut e, "Compact notes");
    let offset = e
        .rendered_page(1)
        .unwrap()
        .controls
        .iter()
        .map(|c| c.bounds)
        .reduce(wireframe::model::Bounds::union)
        .unwrap()
        .w
        + 20.;
    let before = e.document.clone();
    e.merge_version(alt).unwrap();
    assert_eq!(e.surface_id(), 1);
    assert_eq!(e.page_data().notes, "Official notes\n--\nCompact notes");
    let c = &e.page_data().content;
    assert_eq!(c.nodes.len(), 4);
    assert_eq!(&c.nodes[..2], original.pages[0].content.nodes.as_slice());
    let imported: Vec<_> = c.nodes[2..].iter().map(|n| n.id).collect();
    for (source, copy) in original.pages[0].content.nodes.iter().zip(&c.nodes[2..]) {
        assert_ne!(source.id, copy.id);
        let mut expected = source.clone();
        expected.id = copy.id;
        expected.origin.x += offset;
        assert_eq!(*copy, expected);
    }
    assert_eq!(
        c.paint_order,
        vec![
            vec![4, 2],
            vec![5],
            vec![4, 3],
            vec![imported[0], 2],
            vec![imported[1]],
            vec![imported[0], 3]
        ]
    );
    assert_eq!(e.selection, imported.into_iter().collect());
    assert_eq!(e.page_data().alternates, before.pages[0].alternates);
    e.document.validate().unwrap();
    let merged = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(e.surface_id(), 1);
    e.redo();
    assert_eq!(e.document, merged);
}

#[test]
fn discarding_active_version_falls_back_and_undo_restores_content_and_selection() {
    let mut e = Editor::new(content_fixture());
    let alt = e.create_version("Compact").unwrap();
    set_notes(&mut e, "Compact notes");
    let before = e.document.clone();
    e.discard_version(alt).unwrap();
    assert_eq!(e.surface_id(), 1);
    assert!(e.document.versions.is_empty());
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(e.surface_id(), alt);
    e.redo();
    assert_eq!(e.surface_id(), 1);
}

#[test]
fn validation_rejects_dangling_duplicate_or_inconsistent_named_membership() {
    let mut e = Editor::default();
    e.create_version("Compact").unwrap();
    let good = e.document;
    let mut bad = good.clone();
    bad.pages[0].alternates[0].version = Some(999);
    assert!(bad.validate().is_err());
    let mut bad = good.clone();
    bad.pages[0].alternates[0].name = "Other".into();
    assert!(bad.validate().is_err());
    let mut bad = good.clone();
    let mut copy = bad.pages[0].alternates[0].clone();
    copy.id = bad.allocate_id().unwrap();
    bad.pages[0].alternates.push(copy);
    assert!(bad.validate().is_err());
    let mut bad = good.clone();
    bad.versions[0].id = 1;
    assert!(bad.validate().is_err());
    let mut bad = good;
    let id = bad.allocate_id().unwrap();
    bad.versions.push(NamedVersion {
        id,
        name: "Compact".into(),
    });
    assert!(bad.validate().is_err());
}

#[test]
fn exports_resolve_each_versions_notes_including_an_intentionally_empty_note() {
    let mut e = Editor::new(content_fixture());
    let alt = e.create_version("Compact").unwrap();
    set_notes(&mut e, "");
    let options = pdf::Options {
        include_alternates: true,
        notes: true,
        ..Default::default()
    };
    let pages = pdf::plan(&e.document, &options).unwrap();
    assert_eq!(pages.len(), 2);
    assert!(pages[0].size[1] > pages[1].size[1]);
    let plain = pdf::plan(
        &e.document,
        &pdf::Options {
            include_alternates: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(pages[1].size, plain[1].size);
    let entries = export_plan::entries(&e.document);
    assert_eq!(entries[1].id, alt);
    assert_eq!(
        entries[0].page(&e.document).unwrap().notes,
        "Official notes"
    );
    assert_eq!(entries[1].page(&e.document).unwrap().notes, "");
}

#[test]
fn instance_overrides_stay_local_while_source_edits_propagate_to_every_version() {
    let mut p = Project::default();
    let mut child = Node::control(2, "label", Point::new(0., 0.), Size::default());
    child
        .properties
        .insert("text".into(), Value::Text("Shared".into()));
    p.definitions.push(Definition {
        id: 10,
        name: "Shared label".into(),
        content: Content {
            nodes: vec![child.clone()],
            ..Default::default()
        },
    });
    p.pages[0].content.nodes.push(Node {
        id: 3,
        content: NodeContent::Instance {
            definition: 10,
            overrides: vec![],
        },
        ..child
    });
    p.next_id = 20;
    let mut e = {
        p.organize_components().unwrap();
        Editor::new(p)
    };
    let alt = e.create_version("Compact").unwrap();
    assert!(e.enter(3));
    e.begin("Change local text");
    e.set_control_property(2, "text", Value::Text("Local".into()))
        .unwrap();
    e.commit();
    assert_eq!(e.nodes()[0].properties["text"], Value::Text("Local".into()));
    e.switch_version(1, None).unwrap();
    assert!(e.enter(3));
    assert_eq!(
        e.nodes()[0].properties["text"],
        Value::Text("Shared".into())
    );
    e.edit_definition(10).unwrap();
    e.begin("Change source text");
    e.set_control_property(2, "text", Value::Text("Updated source".into()))
        .unwrap();
    e.commit();
    e.switch_version(1, None).unwrap();
    assert!(e.enter(3));
    assert_eq!(
        e.nodes()[0].properties["text"],
        Value::Text("Updated source".into())
    );
    e.switch_version(1, Some(alt)).unwrap();
    assert!(e.enter(3));
    assert_eq!(e.nodes()[0].properties["text"], Value::Text("Local".into()));
    e.undo();
    assert_eq!(e.definition, Some(10));
    assert_eq!(
        e.nodes()[0].properties["text"],
        Value::Text("Shared".into())
    );
    e.document.validate().unwrap();
}

#[test]
fn empty_group_first_child_rewrites_only_the_alternate_paint_order_and_undo_restores_it() {
    let mut p = Project::default();
    p.pages[0].content = Content {
        nodes: vec![Node::group(2, Point::new(10., 20.), vec![])],
        paint_order: vec![vec![2]],
    };
    p.next_id = 3;
    let original = p.pages[0].content.clone();
    let mut e = {
        p.organize_components().unwrap();
        Editor::new(p)
    };
    let alt = e.create_version("Compact").unwrap();
    assert!(e.enter(2));
    e.add(Kind::Button, 10., 20.);
    assert_eq!(e.page_data().content, original);
    let content = &e.page_data().alternates[0].content;
    assert_eq!(content.paint_order.len(), 1);
    assert_eq!(content.paint_order[0].len(), 2);
    e.undo();
    assert_eq!(e.surface_id(), alt);
    assert_eq!(e.page_data().alternates[0].content, original);
    e.redo();
    assert_eq!(e.page_data().alternates[0].content.paint_order[0].len(), 2);
    e.document.validate().unwrap();
}

#[test]
fn failed_merge_restores_document_selected_version_and_history() {
    let mut e = Editor::default();
    set_notes(&mut e, &"O".repeat(600_000));
    let alt = e.create_version("Compact").unwrap();
    set_notes(&mut e, &"A".repeat(600_000));
    e.mark_saved();
    let before = e.document.clone();
    let undo = e.undo_name().map(str::to_owned);
    assert!(e.merge_version(alt).is_err());
    assert_eq!(e.document, before);
    assert_eq!(e.surface_id(), alt);
    assert_eq!(e.undo_name(), undo.as_deref());
    assert!(!e.dirty());
}
