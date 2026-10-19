// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    arrows,
    document::{self, Point, Value},
    model::Kind,
    quick_draw::{Gesture, Mode},
    workspace::Editor,
};
#[test]
fn reverse_and_vertical_arrows_point_at_the_release_position_and_degenerates_stay_finite() {
    for end in [
        Point::new(20., 150.),
        Point::new(100., 10.),
        Point::new(100., 300.),
        Point::new(100., 100.),
    ] {
        let mut g = Gesture::new(Point::new(100., 100.));
        for i in 0..=10 {
            let t = i as f32 / 10.;
            g.motion(Point::new(
                100. + t * (end.x - 100.),
                100. + t * (end.y - 100.),
            ));
        }
        g.end = end;
        let n = g.node(Mode::Arrow, g.bounds());
        let (a, _) = arrows::node_geometry(&n).unwrap();
        let endpoint = if n.properties["end_head"] == Value::Bool(true) {
            a.end
        } else {
            a.start
        };
        assert_eq!(endpoint.translated(n.origin), end);
        let mut e = Editor::default();
        e.insert_quick_draw(n).unwrap();
        assert_eq!(
            document::decode(&e.document.encode().unwrap()).unwrap(),
            e.document
        );
    }
}
#[test]
fn insertion_inside_group_uses_world_coordinates_and_one_history_entry() {
    let mut e = Editor::default();
    e.add(Kind::Button, 150., 120.);
    e.add(Kind::Button, 250., 120.);
    e.select_all();
    e.group();
    let group = *e.selection.iter().next().unwrap();
    assert!(e.enter(group));
    let before = e.document.clone();
    let mut g = Gesture::new(Point::new(190., 160.));
    g.end = Point::new(380., 260.);
    let id = e
        .insert_quick_draw(g.node(Mode::Arrow, g.bounds()))
        .unwrap();
    let view = e.selected_views().into_iter().find(|c| c.id == id).unwrap();
    assert_eq!((view.bounds.x, view.bounds.y), (190., 160.));
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
}
#[test]
fn locked_groups_instances_and_invalid_geometry_reject_without_mutation() {
    let mut e = Editor::default();
    e.add(Kind::Button, 0., 0.);
    e.add(Kind::Button, 100., 0.);
    e.select_all();
    e.group();
    let root = *e.selection.iter().next().unwrap();
    e.lock(true);
    assert!(e.enter(root));
    let mut g = Gesture::new(Point::new(100., 100.));
    g.end = Point::new(200., 200.);
    let before = e.document.clone();
    assert!(!e.can_quick_draw());
    assert!(
        e.insert_quick_draw(g.node(Mode::Image, g.bounds()))
            .is_err()
    );
    assert_eq!(e.document, before);
    e.leave();
    e.lock(false);
    e.create_component("Example".into()).unwrap();
    assert!(e.enter(root));
    let before = e.document.clone();
    assert!(!e.can_quick_draw());
    assert!(
        e.insert_quick_draw(g.node(Mode::Image, g.bounds()))
            .is_err()
    );
    assert_eq!(e.document, before);
    let mut e = Editor::default();
    let before = e.document.clone();
    let mut node = g.node(Mode::Arrow, g.bounds());
    node.origin.x = f32::NAN;
    assert!(e.insert_quick_draw(node).is_err());
    assert_eq!(e.document, before);
}
