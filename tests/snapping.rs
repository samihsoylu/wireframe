// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::Value,
    model::{Bounds, Kind as ControlKind},
    snapping::{Axis, Constraint, Kind, Motion, Targets},
    workspace::Editor,
};
#[test]
fn movement_priority_spacing_boundaries_and_target_order_are_stable() {
    let mut t = Targets::new(5.);
    t.frame(Bounds::new(100., 100., 100., 40.), false);
    let run = |x| {
        t.moved(Bounds::new(x, 300., 40., 20.), Constraint::Free)
            .bounds
            .x
    };
    assert_eq!(
        [run(95.), run(94.), run(126.), run(80.)],
        [100., 94., 130., 84.]
    );
    let mut conflict = Targets::new(5.);
    conflict.add(Axis::X, Kind::Edge, 100.);
    conflict.add(Axis::X, Kind::Edge, 106.);
    assert_eq!(
        conflict
            .moved(Bounds::new(101., 300., 40., 20.), Constraint::Free)
            .bounds
            .x,
        106.
    );
    conflict.add(Axis::X, Kind::Center, 124.);
    assert_eq!(
        conflict
            .moved(Bounds::new(100., 300., 40., 20.), Constraint::Free)
            .bounds
            .x,
        104.
    );
    assert_eq!(
        conflict
            .moved(Bounds::new(101., 300., 40., 20.), Constraint::Vertical)
            .bounds
            .x,
        101.
    );
}
#[test]
fn resize_preserves_opposite_edges_and_has_narrower_center_attraction() {
    let mut t = Targets::new(5.);
    t.frame(Bounds::new(50., 50., 48., 48.), false);
    assert_eq!(
        t.resized(Bounds::new(101., 100., 148., 48.), (-1, 0), None)
            .bounds,
        Bounds::new(98., 100., 151., 48.)
    );
    let mut t = Targets::new(5.);
    t.add(Axis::X, Kind::Center, 148.);
    t.add(Axis::X, Kind::Center, 153.);
    assert_eq!(
        t.resized(Bounds::new(100., 300., 100., 20.), (1, 0), None)
            .bounds
            .w,
        100.
    );
    let mut t = Targets::new(5.);
    t.add(Axis::X, Kind::Center, 153.);
    t.add(Axis::X, Kind::Center, 148.);
    assert_eq!(
        t.resized(Bounds::new(100., 300., 100., 20.), (1, 0), None)
            .bounds
            .w,
        96.
    );
}
#[test]
fn movement_sampler_uses_prior_event_distances_and_recovers_after_still_samples() {
    let mut m = Motion::default();
    for x in [0., 3., 6., 9.] {
        m.push([x, 0.]);
        assert!(!m.too_fast());
    }
    let mut m = Motion::default();
    for x in [0., 4., 8.] {
        m.push([x, 0.]);
    }
    assert!(m.too_fast());
    m.push([8., 0.]);
    m.push([8., 0.]);
    assert!(!m.too_fast());
}
#[test]
fn target_collection_excludes_selection_but_retains_locked_neighbors() {
    let mut e = Editor::default();
    e.add(ControlKind::CircleButton, 50., 50.);
    e.lock(true);
    e.add(ControlKind::CircleButton, 201., 100.);
    let t = e.alignment_targets(5.).unwrap();
    let r = t.moved(Bounds::new(51., 100., 48., 48.), Constraint::Free);
    assert_eq!(r.bounds, Bounds::new(50., 98., 48., 48.));
    assert!(t.matches(Bounds::new(201., 100., 48., 48.)).is_empty());
}
#[test]
fn window_content_targets_follow_header_and_omit_hidden_footer() {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    e.add(ControlKind::Panel, 100., 200.);
    let id = *e.selection.iter().next().unwrap();
    e.set_control_property(id, "header_height", Value::Number(60.))
        .unwrap();
    e.selection.clear();
    let t = e.alignment_targets(0.).unwrap();
    assert!(
        t.matches(Bounds::new(116., 276., 20., 20.))
            .iter()
            .any(|g| g.axis == Axis::Y && g.kind == Kind::Spacing && g.position == 276.)
    );
    e.select(id, false);
    e.selected_mut(|c| c.bounds.h = 50.);
    e.selection.clear();
    let t = e.alignment_targets(0.).unwrap();
    assert!(
        !t.matches(Bounds::new(116., 209., 20., 20.))
            .iter()
            .any(|g| g.axis == Axis::Y && g.kind == Kind::Spacing && g.position == 209.)
    );
}
#[test]
fn target_context_uses_global_coordinates_for_current_group_siblings() {
    let mut e = Editor::default();
    e.add(ControlKind::CircleButton, 150., 150.);
    e.add(ControlKind::CircleButton, 301., 200.);
    e.select_all();
    e.group();
    let group = *e.selection.iter().next().unwrap();
    assert!(e.enter(group));
    let c = e.current().controls;
    e.select(c[1].id, false);
    let r = e
        .alignment_targets(5.)
        .unwrap()
        .moved(Bounds::new(151., 200., 48., 48.), Constraint::Free);
    assert_eq!(r.bounds, Bounds::new(150., 198., 48., 48.));
}
