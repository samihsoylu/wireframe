// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    control_browser::{Catalog, Snapshot},
    document::*,
    workspace::Editor,
};

#[test]
fn original_starters_validate_render_and_select_complete_import_resources() {
    let catalog = Catalog::default();
    assert_eq!(catalog.entries.len(), 4);
    for entry in &catalog.entries {
        let snapshot = entry.load().unwrap();
        snapshot.project.encode().unwrap();
        snapshot.verify_source().unwrap();
        let initial = snapshot.initial_resource().unwrap();
        let editor = Editor::new(snapshot.project.clone());
        assert!(
            !editor.rendered_scene(initial).unwrap().items.is_empty(),
            "{}",
            entry.title
        );
        if snapshot.project.live_libraries().next().is_some() {
            assert_eq!(snapshot.import_pages().len(), 0);
        } else {
            assert_eq!(
                snapshot.import_pages().len(),
                snapshot.project.live_pages().count()
            );
        }
    }
    let landing = catalog.entries[0].load().unwrap();
    let targets: Vec<_> = landing.project.pages[0]
        .content
        .nodes
        .iter()
        .filter_map(|n| match n.properties.get("link") {
            Some(Value::Link(Link::Page(id))) => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(targets.len(), 2);
    assert_ne!(targets[0], targets[1]);
    assert!(
        targets
            .iter()
            .all(|id| landing.project.live_pages().any(|p| p.id == *id))
    );
}
#[test]
fn search_is_global_ranked_and_clearing_it_restores_category() {
    let mut c = Catalog {
        category: "Mobile".into(),
        ..Catalog::default()
    };
    assert_eq!(c.results().len(), 1);
    c.query = "CONTACT".into();
    let titles: Vec<_> = c
        .results()
        .iter()
        .map(|i| c.entries[*i].title.as_str())
        .collect();
    assert_eq!(
        titles,
        vec!["Contact forms", "Landing page", "Tablet order"]
    );
    c.query.clear();
    assert_eq!(c.entries[c.results()[0]].title, "Tablet order");
    c.query = "absent term".into();
    assert!(c.results().is_empty());
}
#[test]
fn local_preview_does_not_take_ownership_and_rejects_changed_or_missing_source() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Local project.wireframe");
    let p = Catalog::default().entries[0].load().unwrap().project;
    let original = p.encode().unwrap();
    std::fs::write(&path, &original).unwrap();
    let snapshot = Snapshot::local(&path).unwrap();
    snapshot.verify_source().unwrap();
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    assert_eq!(std::fs::read(&path).unwrap(), original);
    let mut changed = p.clone();
    changed.title = "Edited elsewhere".into();
    std::fs::write(&path, changed.encode().unwrap()).unwrap();
    assert!(snapshot.verify_source().unwrap_err().contains("changed"));
    assert_eq!(snapshot.project, p);
    let refreshed = Snapshot::local(&path).unwrap();
    assert_eq!(refreshed.origin, snapshot.origin);
    let mut c = Catalog::default();
    let index = c.add_local(&snapshot).unwrap();
    assert_eq!(c.add_local(&refreshed).unwrap(), index);
    assert_eq!(c.entries[index].title, "Edited elsewhere");
    std::fs::remove_file(&path).unwrap();
    assert!(snapshot.verify_source().is_err());
}
#[test]
fn invalid_and_oversized_local_files_are_bounded_and_alternate_content_is_retained() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("invalid.wireframe");
    std::fs::write(&path, b"not a project").unwrap();
    assert!(Snapshot::local(&path).is_err());
    std::fs::File::create(&path)
        .unwrap()
        .set_len(MAX_FILE_BYTES as u64 + 1)
        .unwrap();
    assert!(Snapshot::local(&path).err().unwrap().contains("64 MB"));
    let mut p = Project::default();
    p.pages[0].alternates.push(Alternate {
        id: 2,
        name: "Phone".into(),
        notes: "Review this".into(),
        version: None,
        content: Content::default(),
    });
    p.next_id = 3;
    std::fs::write(&path, p.encode().unwrap()).unwrap();
    let snapshot = Snapshot::local(&path).unwrap();
    assert_eq!(snapshot.import_pages(), vec![1]);
}
