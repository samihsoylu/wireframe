// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, model::Bounds, workspace::Editor};
fn leaf(id: Id, x: f32, y: f32) -> Node {
    Node::control(id, "rectangle", Point::new(x, y), Size::fixed(80., 40.))
}
#[test]
fn hidden_final_markup_child_keeps_container_link_and_scene_cache_is_separate() {
    let mut marked = leaf(4, 100., 0.);
    marked.markup = true;
    let mut group = Node::group(2, Point::new(20., 30.), vec![leaf(3, 0., 0.), marked]);
    group
        .properties
        .insert("link".into(), Value::Link(Link::Back));
    let mut project = Project {
        next_id: 100,
        ..Default::default()
    };
    project.pages[0].content.nodes = vec![group];
    let mut e = Editor::new(project.clone());
    let full = e.rendered_scene(1).unwrap().svg();
    let hidden = e.presentation_scene(1, false).unwrap();
    assert_eq!(hidden.hit_link(40., 40.), Some("back:"));
    assert_eq!(hidden.bounds, Bounds::new(20., 30., 80., 40.));
    assert_ne!(hidden.svg(), full);
    assert_eq!(e.rendered_scene(1).unwrap().svg(), full);
    assert_eq!(e.presentation_scene(1, false).unwrap().svg(), hidden.svg());
    e.set_markup(2, true).unwrap();
    assert!(e.presentation_scene(1, false).unwrap().items.is_empty());
    e.undo();
    assert_eq!(e.document, project);
    assert_eq!(e.presentation_scene(1, false).unwrap().svg(), hidden.svg());
}
#[test]
fn extents_include_alternates_exclude_trash_and_leave_selection_and_document_intact() {
    let mut e = Editor::default();
    e.add(wireframe::model::Kind::Button, 20., 30.);
    let version = e.create_version("Phone").unwrap();
    e.add_page();
    let removed = e.page;
    e.trash_pages(&[removed]).unwrap();
    let document = e.document.clone();
    let selected = e.selection.clone();
    let page = e.page;
    let extents = e.presentation_extents().unwrap();
    assert!(extents.iter().any(|p| p.surface == version));
    assert!(!extents.iter().any(|p| p.surface == removed));
    assert_eq!(e.document, document);
    assert_eq!(e.selection, selected);
    assert_eq!(e.page, page);
}
