// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, export_plan, model::Kind, workspace::Editor};

fn fixture() -> (Project, Id, Id) {
    let mut e = Editor::default();
    e.rename_page(1, "Retained parent").unwrap();
    e.add(Kind::Button, 30., 40.);
    e.document.pages[0].notes = "Official retained notes".into();
    let alt = e.create_version("Compact").unwrap();
    e.set_notes("Retained alternate notes".into());
    e.commit();
    e.add_page();
    let live = e.page;
    e.rename_page(live, "Live child").unwrap();
    e.add(Kind::Button, 80., 90.);
    e.document.pages[1].parent = Some(1);
    e.document.pages[1].content.nodes[0]
        .properties
        .insert("link".into(), Value::Link(Link::Page(1)));
    e.document.pages[0].trashed = true;
    e.document.validate().unwrap();
    (e.document, live, alt)
}

#[test]
fn retained_content_roundtrips_but_cannot_be_selected_edited_or_exported() {
    let (p, live, alternate) = fixture();
    let encoded = p.encode().unwrap();
    assert_eq!(decode(&encoded).unwrap(), p);
    assert_eq!(p.live_pages().map(|p| p.id).collect::<Vec<_>>(), vec![live]);
    assert_eq!(p.live_parent(p.live_page(live).unwrap()), None);
    assert_eq!(p.pages[1].parent, Some(1));
    assert!(p.surface(1).is_none());
    assert!(p.surface(alternate).is_none());
    assert!(p.surface_notes(alternate).is_empty());
    let mut e = Editor::new(p.clone());
    assert_eq!(e.page, live);
    e.switch_page(1);
    assert_eq!(e.page, live);
    assert!(e.switch_version(1, Some(alternate)).is_err());
    assert!(e.rename_page(1, "Hidden edit").is_err());
    assert!(e.duplicate_page(1).is_err());
    assert!(e.layout_page(1).is_err());
    assert!(e.rendered_page(alternate).is_err());
    e.delete_page(); // A retained page does not make deleting the last live page safe.
    assert_eq!(e.document, p);
    assert_eq!(export_plan::entries(&p).len(), 1);
    for id in [1, alternate] {
        assert!(export_plan::select(&p, &export_plan::Selection::Current(id), true).is_err());
        assert!(
            export_plan::Entry {
                id,
                owner: 1,
                page_name: "Hidden".into(),
                alternate_name: None
            }
            .page(&p)
            .is_err()
        );
    }
    let (map, links) = wireframe::site_map::from_project(&p);
    assert!(!map.contains("Retained parent"));
    assert!(map.contains("\n- Live child"));
    assert_eq!(links, vec![None, Some(Link::Page(live))]);
}

#[test]
fn recovery_preview_matches_official_artwork_without_opening_hidden_routes() {
    let (p, live, alternate) = fixture();
    let mut shown = p.clone();
    shown.pages[0].trashed = false;
    let expected = Editor::new(shown).rendered_scene(1).unwrap().svg();
    let mut e = Editor::new(p.clone());
    assert_eq!(e.retained_preview(1).unwrap().svg(), expected);
    assert_eq!(e.retained_preview(1).unwrap().svg(), expected); // Cached path.
    assert!(e.retained_preview(live).is_err());
    assert!(e.retained_preview(alternate).is_err());
    assert!(e.layout_page(1).is_err());
    assert!(e.rendered_scene(1).is_err());
    assert!(e.rendered_page(1).is_err());
    assert!(e.layers(1).is_err());
    assert!(
        e.export_scene(1, &wireframe::workspace::SceneScope::Whole)
            .is_err()
    );
    e.switch_page(1);
    assert_eq!(e.page, live);
    assert_eq!(e.document, p);
    assert!(!e.dirty());
    assert!(e.undo_name().is_none());
    e.restore_pages(&[1]).unwrap();
    assert!(e.retained_preview(1).is_err());
    assert_eq!(e.rendered_scene(1).unwrap().svg(), expected);
}

#[test]
fn retained_resources_share_validation_and_allocator_reservations() {
    let (p, _, _) = fixture();
    let invalid = |mut p: Project, mutate: fn(&mut Project)| {
        mutate(&mut p);
        assert!(p.validate().is_err());
    };
    invalid(p.clone(), |p| p.pages[0].id = p.pages[1].id);
    invalid(p.clone(), |p| p.pages[0].parent = Some(p.pages[1].id));
    invalid(p.clone(), |p| p.pages[0].parent = Some(999));
    invalid(p.clone(), |p| {
        p.pages[0].content.nodes[0]
            .properties
            .insert("image".into(), Value::Asset(999));
    });
    invalid(p.clone(), |p| {
        p.pages[0].content.nodes[0].origin.x = f32::NAN
    });
    invalid(p.clone(), |p| p.pages[0].alternates[0].version = Some(999));
    invalid(p.clone(), |p| p.pages[1].trashed = true);
    let mut p = p;
    let orphan = p.next_id;
    p.pages[0].content.nodes.push(Node {
        id: 2,
        content: NodeContent::Instance {
            definition: 90,
            overrides: vec![Override {
                path: vec![100],
                patch: Patch::default(),
            }],
        },
        ..Node::group(2, Point::default(), vec![])
    });
    // Dangling component definitions remain reserved and override paths may sit above next_id.
    p.pages[0].content.nodes.last_mut().unwrap().id = orphan;
    p.next_id = 100;
    p.validate().unwrap();
    assert_eq!(p.allocate_id().unwrap(), 101);
    let bytes = p.encode().unwrap();
    assert_eq!(decode(&bytes).unwrap(), p);
}

#[test]
fn hidden_contents_cannot_bypass_global_node_or_resource_budgets() {
    let (mut p, _, _) = fixture();
    p.pages[0].content.nodes = (1..=MAX_NODES as u64)
        .map(|id| Node::control(id, "button", Point::default(), Size::default()))
        .collect();
    p.next_id = MAX_NODES as u64 + 1;
    assert!(p.validate().unwrap_err().contains("50,000"));
    let (mut p, _, _) = fixture();
    let template = p.pages[0].clone();
    p.pages[0].alternates.clear();
    for i in 0..1000 {
        let mut page = template.clone();
        page.id = 100 + i;
        page.alternates.clear();
        p.pages.push(page);
    }
    p.next_id = 1200;
    assert!(p.validate().unwrap_err().contains("1000 trashed"));
}

#[test]
fn named_versions_in_trash_survive_pruning_and_shared_rename() {
    let (p, live, hidden_alternate) = fixture();
    let mut e = Editor::new(p);
    let temporary = e.create_version("Temporary").unwrap();
    e.discard_version(temporary).unwrap();
    assert_eq!(e.document.versions[0].name, "Compact");
    let shared = e.create_version("Compact").unwrap();
    e.rename_version(shared, "Narrow").unwrap();
    assert_eq!(e.document.pages[0].alternates[0].id, hidden_alternate);
    assert_eq!(e.document.pages[0].alternates[0].name, "Narrow");
    assert_eq!(
        e.document.live_page(live).unwrap().alternates[0].name,
        "Narrow"
    );
    e.document.validate().unwrap();
    e.undo();
    assert_eq!(e.document.pages[0].alternates[0].name, "Compact");
}

#[test]
fn old_documents_migrate_without_silently_accepting_retained_state() {
    let mut p = Project {
        font: None,
        ..Project::default()
    };
    for version in [2, 3, 4] {
        p.format_version = version;
        let bytes = serde_json::to_vec_pretty(&p).unwrap();
        let migrated = decode(&bytes).unwrap();
        assert_eq!(migrated.format_version, FORMAT_VERSION);
        assert!(!migrated.pages[0].trashed);
        let mut expected = p.clone();
        expected.format_version = FORMAT_VERSION;
        assert_eq!(migrated, expected);
        let mut invalid: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        invalid["pages"][0]["trashed"] = true.into();
        assert!(
            decode(&serde_json::to_vec(&invalid).unwrap())
                .unwrap_err()
                .contains("version 5")
        );
    }
    let mut legacy = serde_json::to_value(wireframe::model::Document::default()).unwrap();
    legacy["pages"][0]["trashed"] = true.into();
    assert!(
        decode(&serde_json::to_vec(&legacy).unwrap())
            .unwrap_err()
            .contains("version 5")
    );
    p.format_version = FORMAT_VERSION + 1;
    assert!(decode(&serde_json::to_vec(&p).unwrap()).is_err());
}

#[test]
fn reversible_removal_repairs_retained_links_and_parents_without_dropping_contents() {
    let (p, live, _) = fixture();
    let mut e = Editor::new(p);
    e.add_page();
    let second = e.page;
    e.document.pages[0].parent = Some(second);
    let hidden = &mut e.document.pages[0];
    hidden.content.nodes[0]
        .properties
        .insert("link".into(), Value::Link(Link::Page(second)));
    hidden.alternates[0].content.nodes[0]
        .properties
        .insert("link".into(), Value::Link(Link::Page(second)));
    let before = e.document.clone();
    e.delete_page();
    assert_eq!(e.page, live);
    assert!(e.document.pages[0].trashed);
    assert_eq!(e.document.pages[0].parent, None);
    assert_eq!(
        e.document.pages[0].content.nodes[0].properties.get("link"),
        Some(&Value::Null)
    );
    assert_eq!(
        e.document.pages[0].alternates[0].content.nodes[0]
            .properties
            .get("link"),
        Some(&Value::Null)
    );
    e.document.validate().unwrap();
    e.undo();
    assert_eq!(e.document, before);
}
