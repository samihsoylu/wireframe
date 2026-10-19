// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    model::Bounds,
    pdf,
    raster::plan::{Prepared, Request},
    workspace::{Editor, SceneScope},
};
fn fixture() -> Editor {
    let mut p = Project {
        next_id: 100,
        ..Project::default()
    };
    let plain = Node::control(3, "rectangle", Point::new(0., 0.), Size::fixed(80., 40.));
    let mut marked = Node::control(4, "rectangle", Point::new(100., 0.), Size::fixed(80., 40.));
    marked.markup = true;
    marked.properties.insert(
        "link".into(),
        Value::Link(Link::Url("https://example.com/annotation".into())),
    );
    let mut group = Node::group(2, Point::new(50., 60.), vec![plain, marked]);
    group.crop = Some(Crop {
        x: 0.25,
        y: 0.,
        width: 0.5,
        height: 1.,
    });
    p.pages[0].content.nodes = vec![group];
    Editor::new(p)
}
#[test]
fn image_snapshot_freezes_markup_preference_and_explicit_override_keeps_full_scene() {
    let mut e = fixture();
    let document = e.document.clone();
    let full = e.rendered_scene(1).unwrap();
    e.set_markup_visible(false);
    let expected = e.export_scene(1, &SceneScope::Whole).unwrap();
    assert_eq!(expected.bounds, Bounds::new(95., 60., 35., 40.));
    assert!(expected.links.is_empty());
    let hidden = e.export_snapshot();
    let overridden = e.export_snapshot().with_markup(true);
    e.set_markup_visible(true);
    let request = Request::new(1);
    let prepared = Prepared::from_snapshot(hidden, &request).unwrap();
    let hidden = prepared.scene(0).unwrap();
    assert_eq!(hidden.svg(), expected.svg());
    assert_eq!(hidden.png().unwrap(), expected.png().unwrap());
    assert_eq!(
        Prepared::from_snapshot(overridden, &request)
            .unwrap()
            .scene(0)
            .unwrap()
            .svg(),
        full.svg()
    );
    assert_eq!(e.document, document);
    assert_eq!(e.rendered_scene(1).unwrap().svg(), full.svg());
}
#[test]
fn hidden_scope_exports_keep_mask_positions_and_do_not_mutate_context_or_selection() {
    let mut e = fixture();
    e.set_markup_visible(false);
    e.select(2, false);
    assert!(e.enter(2));
    e.select(3, false);
    let document = e.document.clone();
    let selection = e.selection.clone();
    for scope in [
        SceneScope::Whole,
        SceneScope::Group(vec![2]),
        SceneScope::Selection(vec![vec![2, 3]]),
    ] {
        let scene = e.export_scene(1, &scope).unwrap();
        assert_eq!(scene.bounds, Bounds::new(95., 60., 35., 40.));
    }
    assert!(
        e.export_scene(1, &SceneScope::Selection(vec![vec![2, 4]]))
            .unwrap()
            .items
            .is_empty()
    );
    assert_eq!(e.document, document);
    assert_eq!(e.context, vec![2]);
    assert_eq!(e.selection, selection);
}
#[test]
fn pdf_visibility_changes_artwork_bounds_and_link_annotations_without_changing_project() {
    let e = fixture();
    let before = e.document.encode().unwrap();
    let shown = pdf::plan(
        &e.document,
        &pdf::Options {
            margins: false,
            ..Default::default()
        },
    )
    .unwrap();
    let hidden = pdf::plan(
        &e.document,
        &pdf::Options {
            margins: false,
            markup_visible: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(hidden[0].scene.bounds, Bounds::new(95., 60., 35., 40.));
    assert!(hidden[0].links.is_empty());
    assert!(!shown[0].links.is_empty());
    assert!(shown[0].size[0] > hidden[0].size[0]);
    assert!(
        pdf::encode("Visibility", &hidden)
            .unwrap()
            .starts_with(b"%PDF")
    );
    assert_eq!(e.document.encode().unwrap(), before);
}
