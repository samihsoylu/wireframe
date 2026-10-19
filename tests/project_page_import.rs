// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::collections::BTreeMap;
use wireframe::{
    document::*,
    workspace::{Editor, PageImportChoice as Choice},
};

fn fixture() -> Project {
    let mut p = Project {
        font: None,
        next_id: 100,
        ..Project::default()
    };
    let template = p.pages[0].clone();
    p.pages = ["Landing", "Contact", "Order"]
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            let mut page = template.clone();
            page.id = i as Id + 1;
            page.name = name.into();
            page.notes = format!("{name} notes");
            let mut node = Node::control(20 + i as Id, "button", Point::default(), Size::default());
            node.properties.insert(
                "link".into(),
                Value::Link(Link::Page((i as Id + 1) % 3 + 1)),
            );
            page.content.nodes.push(node);
            page
        })
        .collect();
    p.pages[2].parent = Some(1);
    p.versions.push(NamedVersion {
        id: 10,
        name: "Phone".into(),
    });
    let alternate_content = p.pages[0].content.clone();
    p.pages[1].alternates.push(Alternate {
        id: 11,
        name: "Phone".into(),
        version: Some(10),
        notes: "Phone notes".into(),
        content: alternate_content,
    });
    p.validate().unwrap();
    p
}
fn target(source: &Project) -> (Editor, Vec<Id>) {
    let mut e = Editor::new(Project {
        font: None,
        ..Project::default()
    });
    let prepared = e
        .prepare_project_copy(source, "source-project", &[1, 2, 3])
        .unwrap();
    let ids = prepared.imported_pages().to_vec();
    e.apply_project_copy(prepared).unwrap();
    (e, ids)
}
fn choices(choice: Choice) -> BTreeMap<Id, Choice> {
    (1..=3).map(|id| (id, choice)).collect()
}

#[test]
fn equivalent_pages_use_mapped_links_and_recheck_keep_both() {
    let source = fixture();
    let (e, _) = target(&source);
    let before = e.document.clone();
    let plan = e
        .plan_project_page_import(&source, "source-project", &[1, 2, 3])
        .unwrap();
    assert_eq!(
        plan.equivalent_pages(&BTreeMap::new(), &BTreeMap::new(), &BTreeMap::new())
            .unwrap(),
        vec![1, 2, 3]
    );
    let separate_contact = BTreeMap::from([(2, Choice::KeepBoth)]);
    // Landing points to Contact, so retaining its old link would be incorrect.
    assert_eq!(
        plan.equivalent_pages(&separate_contact, &BTreeMap::new(), &BTreeMap::new())
            .unwrap(),
        vec![3]
    );
    let retained_contact = BTreeMap::from([(2, Choice::UseExisting)]);
    assert_eq!(
        plan.equivalent_pages(&retained_contact, &BTreeMap::new(), &BTreeMap::new())
            .unwrap(),
        vec![1, 3]
    );
    assert_eq!(e.document, before);
}

#[test]
fn equivalence_preserves_real_edits_and_ignores_only_scoped_control_identity() {
    let source = fixture();
    let (e, _) = target(&source);
    for change in 0..7 {
        let mut incoming = source.clone();
        match change {
            0 => incoming.pages[0].content.nodes[0].id = 95,
            1 => incoming.pages[0].content.nodes[0].origin.x += 10.,
            2 => incoming.pages[0].notes.push_str(" changed"),
            3 => incoming.pages[0].content.nodes[0].locked = true,
            4 => incoming.pages[0].content.nodes[0]
                .properties
                .insert("text".into(), Value::Text("Changed".into()))
                .map(|_| ())
                .unwrap_or(()),
            5 => incoming.pages[1].alternates[0].notes.push_str(" changed"),
            _ => {
                use wireframe::review::{Author, Body};
                incoming
                    .reviews
                    .entry(1)
                    .or_default()
                    .post(
                        Author {
                            id: "reviewer".into(),
                            name: "Reviewer".into(),
                        },
                        None,
                        Body {
                            text: "New feedback".into(),
                            ..Body::default()
                        },
                        1,
                    )
                    .unwrap();
            }
        }
        let plan = e
            .plan_project_page_import(&incoming, "source-project", &[1, 2, 3])
            .unwrap();
        let same = plan
            .equivalent_pages(&BTreeMap::new(), &BTreeMap::new(), &BTreeMap::new())
            .unwrap();
        assert_eq!(
            same,
            match change {
                0 => vec![1, 2, 3],
                5 => vec![1, 3],
                _ => vec![2, 3],
            },
            "change={change}"
        );
    }
}

#[test]
fn equivalent_group_ids_retain_effective_paint_order() {
    let mut source = fixture();
    let child = source.pages[0].content.nodes[0].clone();
    let mut second = child.clone();
    second.id = 45;
    second
        .properties
        .insert("text".into(), Value::Text("Second".into()));
    source.pages[0].content.nodes = vec![Node::group(46, Point::default(), vec![child, second])];
    source.pages[0].content.paint_order = vec![vec![46, 45], vec![46, 20]];
    let (e, _) = target(&source);
    let mut incoming = source.clone();
    incoming.pages[0].content.nodes[0].id = 47;
    if let NodeContent::Group { children } = &mut incoming.pages[0].content.nodes[0].content {
        children[0].id = 48;
        children[1].id = 49;
    }
    incoming.pages[0].content.paint_order = vec![vec![47, 49], vec![47, 48]];
    for changed_order in [false, true] {
        if changed_order {
            incoming.pages[0].content.paint_order.reverse();
        }
        let plan = e
            .plan_project_page_import(&incoming, "source-project", &[1, 2, 3])
            .unwrap();
        let same = plan
            .equivalent_pages(&BTreeMap::new(), &BTreeMap::new(), &BTreeMap::new())
            .unwrap();
        assert_eq!(same.contains(&1), !changed_order);
    }
}

#[test]
fn mixed_decisions_preserve_existing_links_and_rewire_incoming_pages_and_definitions() {
    let mut source = fixture();
    let mut button = Node::control(53, "button", Point::default(), Size::default());
    button
        .properties
        .insert("link".into(), Value::Link(Link::Page(2)));
    source.definitions.push(Definition {
        id: 51,
        name: "Card".into(),
        content: Content {
            nodes: vec![button],
            paint_order: vec![],
        },
    });
    let mut placement = Node::group(52, Point::default(), vec![]);
    placement.content = NodeContent::Instance {
        definition: 51,
        overrides: vec![],
    };
    source.libraries.push(ComponentLibrary {
        id: 50,
        trashed: false,
        name: "Cards".into(),
        notes: String::new(),
        content: Content {
            nodes: vec![placement],
            paint_order: vec![],
        },
        sources: vec![ComponentSource {
            definition: 51,
            node: 52,
            notes: String::new(),
        }],
        imports: vec![],
    });
    source.default_library = Some(50);
    let (mut e, ids) = target(&source);
    e.rename_page(ids[0], "Home").unwrap();
    e.edit_page_notes(ids[1], None, "Local contact notes")
        .unwrap();
    source.pages[0].notes = "Updated landing".into();
    let before = e.document.clone();
    let plan = e
        .plan_project_page_import(&source, "source-project", &[1, 2, 3])
        .unwrap();
    assert_eq!(plan.entries()[0].existing, Some(ids[0]));
    let prepared = plan
        .prepare(&BTreeMap::from([
            (1, Choice::UseNew),
            (2, Choice::UseExisting),
            (3, Choice::KeepBoth),
        ]))
        .unwrap();
    assert_eq!(e.document, before);
    assert_eq!(prepared.counts().pages, 1);
    assert_eq!(prepared.counts().pages_replaced, 1);
    assert_eq!(prepared.counts().pages_reused, 1);
    let fresh = prepared.imported_pages()[2];
    let after = prepared.preview().clone();
    assert_eq!(after.live_page(ids[0]).unwrap().name, "Home");
    assert_eq!(after.live_page(ids[0]).unwrap().notes, "Updated landing");
    assert_eq!(after.live_page(ids[1]), before.live_page(ids[1]));
    assert_eq!(after.live_page(fresh).unwrap().parent, Some(ids[0]));
    assert_eq!(
        after.live_page(ids[0]).unwrap().content.nodes[0].properties["link"],
        Value::Link(Link::Page(ids[1]))
    );
    assert_eq!(
        after.live_page(fresh).unwrap().content.nodes[0].properties["link"],
        Value::Link(Link::Page(ids[0]))
    );
    assert_eq!(after.definitions[0], before.definitions[0]);
    assert_eq!(
        after.definitions[1].content.nodes[0].properties["link"],
        Value::Link(Link::Page(ids[1]))
    );
    e.apply_project_copy(prepared).unwrap();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
}

#[test]
fn repeated_reuse_is_a_true_noop_and_does_not_allocate_provisional_resources() {
    let source = fixture();
    let (mut e, _) = target(&source);
    let before = e.document.clone();
    let prepared = e
        .plan_project_page_import(&source, "source-project", &[1, 2, 3])
        .unwrap()
        .prepare(&choices(Choice::UseExisting))
        .unwrap();
    assert_eq!(prepared.preview(), &before);
    assert_eq!(prepared.counts().pages_reused, 3);
    e.apply_project_copy(prepared).unwrap();
    e.undo();
    assert_eq!(e.document.pages.len(), 1); // Reuse did not add another history entry.
}

#[test]
fn alternate_identity_survives_rename_removal_and_return() {
    let mut source = fixture();
    let (mut e, ids) = target(&source);
    let alternate = e.document.live_page(ids[1]).unwrap().alternates[0].id;
    source.versions[0].name = "Tablet".into();
    source.pages[1].alternates[0].name = "Tablet".into();
    let prepared = e
        .plan_project_page_import(&source, "source-project", &[1, 2, 3])
        .unwrap()
        .prepare(&choices(Choice::UseNew))
        .unwrap();
    assert_eq!(
        prepared.preview().live_page(ids[1]).unwrap().alternates[0].id,
        alternate
    );
    e.apply_project_copy(prepared).unwrap();
    let saved = source.pages[1].alternates.pop().unwrap();
    let prepared = e
        .plan_project_page_import(&source, "source-project", &[1, 2, 3])
        .unwrap()
        .prepare(&choices(Choice::UseNew))
        .unwrap();
    e.apply_project_copy(prepared).unwrap();
    assert_eq!(e.document.live_page(ids[1]).unwrap().alternates.len(), 1);
    // Removing a branch from the source alone does not remove the local copy.
    // Simulate an explicit destination removal before its later re-import.
    e.document
        .pages
        .iter_mut()
        .find(|p| p.id == ids[1])
        .unwrap()
        .alternates
        .clear();
    assert_eq!(
        e.document.page_imports[&ids[1]][0].alternates[&11],
        alternate
    );
    source.pages[1].alternates.push(saved);
    let prepared = e
        .plan_project_page_import(&source, "source-project", &[1, 2, 3])
        .unwrap()
        .prepare(&choices(Choice::UseNew))
        .unwrap();
    assert_eq!(
        prepared.preview().live_page(ids[1]).unwrap().alternates[0].id,
        alternate
    );
}

#[test]
fn origin_matches_are_reserved_before_name_matches_and_trash_is_not_replaced() {
    let mut source = fixture();
    let (mut e, ids) = target(&source);
    e.document.page_imports.remove(&ids[1]);
    source.pages[1].name = "Landing".into();
    source.pages.swap(0, 1);
    let plan = e
        .plan_project_page_import(&source, "source-project", &[1, 2, 3])
        .unwrap();
    assert_eq!(plan.entries()[0].incoming, 2);
    assert_eq!(plan.entries()[0].existing, None);
    assert_eq!(plan.entries()[1].existing, Some(ids[0]));
    e.trash_pages(&[ids[0]]).unwrap();
    let plan = e
        .plan_project_page_import(&source, "source-project", &[1])
        .unwrap();
    assert_eq!(plan.entries()[0].existing, None);
    assert!(e.document.page_imports.contains_key(&ids[0]));
    e.restore_pages(&[ids[0]]).unwrap();
    assert_eq!(
        e.plan_project_page_import(&source, "source-project", &[1])
            .unwrap()
            .entries()[0]
            .existing,
        Some(ids[0])
    );
}

#[test]
fn unresolved_unknown_and_stale_decisions_do_not_change_the_project() {
    let source = fixture();
    let (mut e, ids) = target(&source);
    let before = e.document.clone();
    let plan = e
        .plan_project_page_import(&source, "source-project", &[1, 2, 3])
        .unwrap();
    assert!(plan.prepare(&BTreeMap::new()).is_err());
    let mut unknown = choices(Choice::UseExisting);
    unknown.insert(999, Choice::KeepBoth);
    assert!(plan.prepare(&unknown).is_err());
    assert_eq!(e.document, before);
    let prepared = plan.prepare(&choices(Choice::UseNew)).unwrap();
    e.rename_page(ids[0], "Changed").unwrap();
    let changed = e.document.clone();
    assert!(!plan.matches_destination(&changed));
    assert!(e.apply_project_copy(prepared).is_err());
    assert_eq!(e.document, changed);
    // Incoming hierarchy must not move an existing destination page. Applying
    // the source parent here would create a cycle with the retained child.
    let mut cyclic = source.clone();
    cyclic.pages[2].parent = None;
    cyclic.pages[0].parent = Some(3);
    let plan = e
        .plan_project_page_import(&cyclic, "source-project", &[1, 2, 3])
        .unwrap();
    let prepared = plan
        .prepare(&BTreeMap::from([
            (1, Choice::UseNew),
            (2, Choice::UseExisting),
            (3, Choice::UseExisting),
        ]))
        .unwrap();
    assert_eq!(prepared.preview().live_page(ids[0]).unwrap().parent, None);
    assert_eq!(
        prepared.preview().live_page(ids[2]).unwrap().parent,
        Some(ids[0])
    );
    assert_eq!(e.document, changed);
}

#[test]
fn replacement_at_page_capacity_validates_the_final_graph_not_temporary_copies() {
    let mut source = Project {
        font: None,
        next_id: 1001,
        ..Project::default()
    };
    let template = source.pages[0].clone();
    source.pages = (1..=1000)
        .map(|id| {
            let mut p = template.clone();
            p.id = id;
            p.name = format!("Page {id}");
            p
        })
        .collect();
    let e = Editor::new(source.clone());
    let ids: Vec<_> = (1..=1000).collect();
    assert!(
        e.prepare_project_copy(&source, "source-project", &ids)
            .is_err()
    );
    let plan = e
        .plan_project_page_import(&source, "source-project", &ids)
        .unwrap();
    let prepared = plan
        .prepare(&ids.iter().map(|&id| (id, Choice::UseNew)).collect())
        .unwrap();
    assert_eq!(prepared.preview().pages.len(), 1000);
    assert_eq!(prepared.counts().pages_replaced, 1000);
}

#[test]
fn permanent_page_removal_scrubs_origin_records_from_current_and_history() {
    let source = fixture();
    let (mut e, ids) = target(&source);
    e.trash_pages(&[ids[1]]).unwrap();
    assert!(e.document.page_imports.contains_key(&ids[1]));
    e.permanently_remove_pages(&[ids[1]]).unwrap();
    assert!(!e.document.page_imports.contains_key(&ids[1]));
    for _ in 0..4 {
        e.undo();
        assert!(!e.document.page_imports.contains_key(&ids[1]));
        e.document.validate().unwrap();
    }
    for _ in 0..4 {
        e.redo();
        assert!(!e.document.page_imports.contains_key(&ids[1]));
        e.document.validate().unwrap();
    }
}

#[test]
fn optional_origin_metadata_is_backward_compatible_and_rejects_invalid_bindings() {
    let source = fixture();
    assert!(
        decode(&source.encode().unwrap())
            .unwrap()
            .page_imports
            .is_empty()
    );
    let (e, ids) = target(&source);
    for kind in 0..5 {
        let mut p = e.document.clone();
        let record = &mut p.page_imports.get_mut(&ids[1]).unwrap()[0];
        match kind {
            0 => record.project = "bad\norigin".into(),
            1 => record.page = 0,
            2 => {
                record.alternates.insert(0, 99);
            }
            3 => {
                record.alternates.insert(12, ids[0]);
            }
            _ => {
                record.alternates.insert(12, record.alternates[&11]);
            }
        }
        assert!(p.validate().is_err(), "case {kind}");
    }
    let mut p = e.document.clone();
    p.page_imports.get_mut(&ids[1]).unwrap()[0]
        .alternates
        .insert(12, p.next_id);
    assert!(p.validate().is_err()); // Retired local identities must stay reserved.
}

#[test]
fn historical_alternate_identity_cannot_be_claimed_by_two_pages() {
    let (e, ids) = target(&fixture());
    let mut p = e.document.clone();
    let retired = p.allocate_id().unwrap();
    for id in [ids[0], ids[1]] {
        p.page_imports.get_mut(&id).unwrap()[0]
            .alternates
            .insert(99, retired);
    }
    assert!(p.validate().is_err());
}

#[test]
fn imported_page_and_alternate_exports_preserve_project_correspondence() {
    use wireframe::{
        export_plan::Selection,
        pdf,
        raster::plan::{Prepared, Request},
    };
    let (e, ids) = target(&fixture());
    let before = e.document.encode().unwrap();
    let alternate = e.document.live_page(ids[1]).unwrap().alternates[0].id;
    for (id, title) in [(ids[0], "Landing"), (alternate, "Contact — Phone")] {
        let pages = pdf::plan(
            &e.document,
            &pdf::Options {
                selection: Selection::Current(id),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].scene.title, title);
        assert!(!pages[0].scene.items.is_empty());
        assert!(
            pdf::encode("Imported", &pages)
                .unwrap()
                .starts_with(b"%PDF")
        );
        let raster = Prepared::new(
            &e.document,
            &Request {
                selection: Selection::Current(id),
                ..Request::new(id)
            },
        )
        .unwrap();
        assert_eq!(raster.entries().len(), 1);
        let scene = raster.scene(0).unwrap();
        assert_eq!(scene.title, title);
        assert!(!scene.items.is_empty());
    }
    assert_eq!(e.document.encode().unwrap(), before);
}

#[test]
fn new_alternate_name_match_cannot_steal_a_later_origin_match() {
    let mut source = fixture();
    let (e, ids) = target(&source);
    let original = e.document.live_page(ids[1]).unwrap().alternates[0].id;
    let mut added = source.pages[1].alternates[0].clone();
    added.id = source.allocate_id().unwrap();
    added.version = None;
    source.pages[1].alternates[0].name = "Renamed".into();
    source.pages[1].alternates[0].version = None;
    source.pages[1].alternates.insert(0, added);
    let prepared = e
        .plan_project_page_import(&source, "source-project", &[1, 2, 3])
        .unwrap()
        .prepare(&choices(Choice::UseNew))
        .unwrap();
    let alternates = &prepared.preview().live_page(ids[1]).unwrap().alternates;
    assert_eq!(
        alternates.iter().find(|a| a.name == "Renamed").unwrap().id,
        original
    );
    assert_ne!(
        alternates.iter().find(|a| a.name == "Phone").unwrap().id,
        original
    );
}

#[test]
fn replacement_preserves_local_hierarchy_and_unmatched_alternates_with_exact_history() {
    let mut source = fixture();
    let (mut e, ids) = target(&source);
    let local_id = e.document.allocate_id().unwrap();
    let mut local = e.document.live_page(ids[1]).unwrap().alternates[0].clone();
    local.id = local_id;
    local.version = None;
    local.name = "Local experiment".into();
    local.notes = "Retain this work".into();
    let contact = e
        .document
        .pages
        .iter_mut()
        .find(|p| p.id == ids[1])
        .unwrap();
    contact.parent = Some(ids[0]);
    contact.alternates.push(local.clone());
    let before = e.document.clone();
    let page_order: Vec<_> = before.pages.iter().map(|p| p.id).collect();
    source.pages[1].parent = Some(3);
    source.pages[1].notes = "New official notes".into();
    source.pages[1].alternates[0].notes = "New phone notes".into();
    let prepared = e
        .plan_project_page_import(&source, "source-project", &[1, 2, 3])
        .unwrap()
        .prepare(&choices(Choice::UseNew))
        .unwrap();
    e.apply_project_copy(prepared).unwrap();
    let contact = e.document.live_page(ids[1]).unwrap();
    assert_eq!(contact.parent, Some(ids[0]));
    assert_eq!(contact.notes, "New official notes");
    assert_eq!(contact.alternates[0].notes, "New phone notes");
    assert_eq!(contact.alternates[1], local);
    assert_eq!(
        e.document.pages.iter().map(|p| p.id).collect::<Vec<_>>(),
        page_order
    );
    let after = e.document.clone();
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
}
