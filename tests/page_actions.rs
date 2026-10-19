// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, model::Kind, workspace::Editor};

fn fixture() -> (Editor, Id) {
    let mut e = Editor::default();
    e.rename_page(1, "Landing").unwrap();
    e.add(Kind::Button, 50., 60.);
    let definition = e.create_component("Action".into()).unwrap();
    let child = e.document.definitions[0].content.nodes[0].id;
    if let NodeContent::Instance { overrides, .. } =
        &mut e.document.pages[0].content.nodes[0].content
    {
        overrides.push(Override {
            path: vec![child],
            patch: Patch {
                properties: Properties::from([
                    ("text".into(), Value::Text("Local label".into())),
                    ("link".into(), Value::Link(Link::Page(1))),
                ]),
                ..Patch::default()
            },
        });
    }
    e.insert_component(definition, 250., 60.).unwrap();
    let asset = e.document.allocate_id().unwrap();
    e.document.assets.push(Asset {
        trashed: false,
        id: asset, name: "Badge".into(), media_type: "image/svg+xml".into(),
        bytes: br#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="red"/></svg>"#.to_vec(),
    });
    e.add(Kind::Image, 100., 140.);
    let image = *e.selection.iter().next().unwrap();
    e.set_control_property(image, "image", Value::Asset(asset))
        .unwrap();

    e.select_all();
    e.group();
    e.document.pages[0].content.nodes[0].locked = true;
    e.document.pages[0].content.nodes[0].crop = Some(Crop {
        x: 0.,
        y: 0.,
        width: 0.75,
        height: 0.8,
    });
    e.document.pages[0].notes = "Official notes".into();
    e.document.pages[0].content.paint_order = e.document.pages[0].content.paint_paths().unwrap();
    let alternate = e.create_version("Compact").unwrap();
    e.edit("Notes", |e| e.set_notes("Alternate notes".into()));
    (e, alternate)
}

#[test]
fn complete_page_copy_preserves_scoped_data_links_and_shared_dependencies() {
    let (mut e, alternate) = fixture();
    let before = e.document.clone();
    let id = e.duplicate_page(1).unwrap();
    assert_eq!(e.page, id);
    let copied_alt = e.active_alternate(id).unwrap();
    assert_ne!(copied_alt, alternate);
    assert_eq!(e.undo_name(), Some("Duplicate wireframe"));
    let mut expected = before.pages[0].clone();
    expected.id = id;
    expected.name = "Landing copy".into();
    expected.alternates[0].id = copied_alt;
    assert_eq!(e.document.pages[1], expected);
    assert_eq!(e.document.pages[0], before.pages[0]);
    assert_eq!(e.document.definitions, before.definitions);
    assert_eq!(e.document.assets, before.assets);
    assert_eq!(e.document.versions, before.versions);
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(e.active_alternate(1), Some(alternate));
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(e.page, id);
    assert_eq!(e.active_alternate(id), None);
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), after);
}

#[test]
fn batch_copy_keeps_complete_alternates_and_shared_dependencies_without_retargeting_links() {
    let (mut e, alternate) = fixture();
    let mut child = e.document.pages[0].clone();
    child.id = e.document.allocate_id().unwrap();
    child.name = "Child".into();
    child.parent = Some(1);
    child.alternates[0].id = e.document.allocate_id().unwrap();
    let child_id = child.id;
    let child_alternate = child.alternates[0].id;
    e.document.pages.push(child);
    e.switch_version(child_id, Some(child_alternate)).unwrap();
    let before = e.document.clone();
    let copies = e.duplicate_pages(&[child_id, 1]).unwrap();
    for (index, &id) in copies.iter().enumerate() {
        let copy = e.document.live_page(id).unwrap();
        let mut expected = before.pages[index].clone();
        expected.id = id;
        expected.name = format!("{} copy", expected.name);
        expected.parent = (index == 1).then_some(copies[0]);
        expected.alternates[0].id = copy.alternates[0].id;
        assert_eq!(*copy, expected);
        assert_eq!(e.active_alternate(id), Some(copy.alternates[0].id));
        assert_ne!(copy.alternates[0].id, alternate);
        assert_ne!(copy.alternates[0].id, child_alternate);
    }
    assert_eq!(e.document.assets, before.assets);
    assert_eq!(e.document.definitions, before.definitions);
    assert_eq!(e.document.versions, before.versions);
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(e.page, child_id);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(e.page, copies[0]);
    assert_eq!(e.active_alternate(copies[0]), None);
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), after);
}

#[test]
fn duplicate_names_and_parent_placement_do_not_clone_descendant_pages() {
    let mut e = Editor::default();
    e.rename_page(1, "Landing").unwrap();
    e.add_page();
    let child = e.page;
    e.document.pages[1].parent = Some(1);
    e.add_page();
    let sibling = e.page;
    e.rename_page(sibling, "LANDING COPY").unwrap();
    let id = e.duplicate_page(1).unwrap();
    assert_eq!(
        e.document.pages.iter().map(|p| p.id).collect::<Vec<_>>(),
        [1, child, id, sibling]
    );
    assert_eq!(e.page_data().name, "Landing copy 2");
    let id2 = e.duplicate_page(id).unwrap();
    assert_eq!(e.page_data().name, "Landing copy 3");
    assert_ne!(id, id2);
    let child_copy = e.duplicate_page(child).unwrap();
    assert_eq!(e.page_data().parent, Some(1));
    assert_eq!(e.document.pages[2].id, child_copy);
}

#[test]
fn rename_changes_only_name_and_is_one_reversible_edit() {
    let (mut e, _) = fixture();
    let before = e.document.clone();
    e.rename_page(1, "Contact us").unwrap();
    assert_eq!(e.undo_name(), Some("Rename wireframe"));
    let mut expected = before.clone();
    expected.pages[0].name = "Contact us".into();
    assert_eq!(e.document, expected);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, expected);
    e.rename_page(1, "Contact us").unwrap();
    e.undo();
    assert_eq!(e.document, before);
}

#[test]
fn limits_and_stale_targets_leave_document_history_and_view_unchanged() {
    let (mut e, _) = fixture();
    e.document.next_id = u64::MAX - 1;
    e.document.validate().unwrap();
    let before = e.document.clone();
    let view = (e.page, e.surface_id(), e.selection.clone());
    let history = e.undo_name().map(str::to_owned);
    assert!(e.duplicate_page(1).is_err());
    assert!(e.duplicate_page(0).is_err());
    assert!(e.rename_page(0, "Missing").is_err());
    assert!(e.rename_page(1, " ").is_err());
    assert!(e.rename_page(1, &"x".repeat(10001)).is_err());
    assert_eq!(e.document, before);
    assert_eq!((e.page, e.surface_id(), e.selection.clone()), view);
    assert_eq!(e.undo_name(), history.as_deref());
}

#[test]
fn page_count_limit_is_atomic() {
    let mut project = Project::default();
    let page = project.pages[0].clone();
    for id in 2..=1000 {
        let mut next = page.clone();
        next.id = id;
        project.pages.push(next);
    }
    project.next_id = 1001;
    let mut editor = Editor::new(project.clone());
    assert!(
        editor
            .duplicate_page(1)
            .unwrap_err()
            .contains("1–1000 live pages")
    );
    assert_eq!(editor.document, project);
    assert_eq!(editor.undo_name(), None);
}
