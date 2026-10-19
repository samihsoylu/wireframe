// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    model::Bounds,
    workspace::{AlignmentAction, Editor},
};
fn specimen() -> Editor {
    let mut p = Project {
        next_id: 10,
        ..Project::default()
    };
    p.pages[0].content.nodes = [
        (50., 50., 101., 101.),
        (800., 40., 60., 50.),
        (260., 180., 80., 70.),
        (500., 330., 140., 80.),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (x, y, w, h))| {
        Node::control(i as u64 + 2, "button", Point::new(x, y), Size::fixed(w, h))
    })
    .collect();
    let mut e = Editor::new(p);
    e.select_all();
    e
}
#[test]
fn all_ten_arrangements_use_integer_centers_and_exact_snapshot_history() {
    use AlignmentAction::*;
    for (action, expected) in [
        (Left, [50., 50., 50., 50.]),
        (Center, [404., 425., 415., 385.]),
        (Right, [759., 800., 780., 720.]),
        (Top, [40., 40., 40., 40.]),
        (Middle, [174., 200., 190., 185.]),
        (Bottom, [309., 360., 340., 330.]),
        (DistributeHorizontal, [50., 800., 304., 517.]),
        (DistributeVertical, [116., 40., 233., 330.]),
        (SpaceHorizontal, [50., 800., 294., 517.]),
        (SpaceVertical, [113., 40., 237., 330.]),
    ] {
        let mut e = specimen();
        let before = e.document.clone();
        e.align_selection(action);
        let after = e.document.clone();
        for (i, node) in e.nodes().iter().enumerate() {
            assert_eq!(
                if action.vertical() {
                    node.origin.y
                } else {
                    node.origin.x
                },
                expected[i],
                "{action:?}"
            );
            assert_eq!(node.content, before.pages[0].content.nodes[i].content);
        }
        e.document.validate().unwrap();
        e.undo();
        assert_eq!(e.document, before);
        e.redo();
        assert_eq!(e.document, after);
        assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
    }
}
#[test]
fn locked_selection_does_not_define_alignment_target_and_single_movable_is_noop() {
    let mut e = specimen();
    e.current_mut().content.nodes[1].locked = true;
    let before = e.document.clone();
    e.align_selection(AlignmentAction::Right);
    assert_eq!(
        e.nodes().iter().map(|n| n.origin.x).collect::<Vec<_>>(),
        [539., 800., 560., 500.]
    );
    assert_eq!(e.nodes()[1], before.pages[0].content.nodes[1]);
    e.undo();
    assert_eq!(e.document, before);
    for n in e.current_mut().content.nodes.iter_mut().take(3) {
        n.locked = true;
    }
    let before = e.document.clone();
    assert!(!e.can_align_selection());
    e.align_selection(AlignmentAction::Left);
    e.distribute(0);
    assert_eq!(e.document, before);
}
#[test]
fn overlapping_controls_can_have_negative_equal_gaps_and_stable_center_ties() {
    let mut e = specimen();
    e.current_mut().content.nodes.truncate(3);
    for (node, x) in e
        .current_mut()
        .content
        .nodes
        .iter_mut()
        .zip([-10., 0., 10.])
    {
        node.origin.x = x;
        node.content = NodeContent::Control {
            kind: "button".into(),
            size: Size::fixed(100., 30.),
        };
    }
    e.select_all();
    let before = e.document.clone();
    e.align_selection(AlignmentAction::SpaceHorizontal);
    assert_eq!(e.document, before, "equal negative gaps are already spaced");
    e.current_mut().content.nodes[1].origin.x = 4.5;
    e.align_selection(AlignmentAction::SpaceHorizontal);
    assert_eq!(
        e.nodes().iter().map(|n| n.origin.x).collect::<Vec<_>>(),
        [-10., 0., 10.]
    );
}
#[test]
fn selection_autosize_includes_locked_controls_and_preserves_one_atomic_undo() {
    let mut e = specimen();
    e.current_mut().content.nodes[1].locked = true;
    assert!(e.can_auto_size_selection(true));
    let before = e.document.clone();
    e.auto_size_selection().unwrap();
    for (n, old) in e.nodes().iter().zip(&before.pages[0].content.nodes) {
        assert_eq!(n.origin, old.origin);
        assert_eq!(n.locked, old.locked);
        assert!(matches!(
            n.content,
            NodeContent::Control {
                size: Size {
                    width: Dimension::Auto,
                    height: Dimension::Auto
                },
                ..
            }
        ));
    }
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    e.auto_size_selection().unwrap();
    e.undo();
    assert_eq!(e.document, before, "a repeated no-op must not add history");
}
#[test]
fn autosize_availability_and_component_local_edits_keep_source_and_other_instances() {
    let mut e = specimen();
    e.group();
    let definition = e.create_component("Controls".into()).unwrap();
    let instance = *e.selection.iter().next().unwrap();
    assert!(!e.can_auto_size_selection(true));
    e.enter(instance);
    e.select_all();
    let before = e.document.clone();
    assert!(e.can_auto_size_selection(true));
    e.auto_size_selection().unwrap();
    assert_eq!(e.document.definitions, before.definitions);
    assert_eq!(e.document.libraries, before.libraries);
    e.undo();
    assert_eq!(e.document, before);
    e.leave();
    e.add(wireframe::model::Kind::Button, 1000., 1000.);
    e.select(instance, true);
    assert!(!e.can_auto_size_selection(true));
    assert!(e.can_auto_size_selection(false));
    let source = e.document.component_source(definition).unwrap().1.clone();
    e.auto_size_selection().unwrap();
    assert_eq!(e.document.component_source(definition).unwrap().1, &source);
}
#[test]
fn cropped_group_alignment_moves_visible_bounds_without_changing_children_or_crop() {
    let mut e = specimen();
    e.selection = [2, 4].into_iter().collect();
    e.group();
    let group = *e.selection.iter().next().unwrap();
    let node = e
        .current_mut()
        .content
        .nodes
        .iter_mut()
        .find(|n| n.id == group)
        .unwrap();
    node.crop = Some(Crop {
        x: 0.1,
        y: 0.15,
        width: 0.5,
        height: 0.5,
    });
    e.select(group, false);
    e.select(5, true);
    let before = e.document.clone();
    let old = e
        .selected_views()
        .into_iter()
        .map(|c| c.bounds)
        .reduce(Bounds::union)
        .unwrap();
    e.align_selection(AlignmentAction::Right);
    assert!(
        e.selected_views()
            .iter()
            .all(|c| (c.bounds.x + c.bounds.w - old.x - old.w).abs() < 0.001)
    );
    let nodes = e.nodes();
    let after = nodes.iter().find(|n| n.id == group).unwrap();
    let old = before.pages[0]
        .content
        .nodes
        .iter()
        .find(|n| n.id == group)
        .unwrap();
    assert_eq!(after.crop, old.crop);
    assert_eq!(after.content, old.content);
    e.undo();
    assert_eq!(e.document, before);
}
