// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::collections::HashSet;
use wireframe::{document::*, storage::ProjectStore, workspace::Editor};

fn leaf(id: Id, x: f32, y: f32) -> Node {
    Node::control(id, "rectangle", Point::new(x, y), Size::fixed(80., 40.))
}
fn fixture() -> Project {
    let mut inner = Node::group(
        3,
        Point::new(10., 5.),
        vec![leaf(2, 0., 0.), leaf(4, 100., 0.)],
    );
    inner.crop = Some(Crop {
        x: 0.25,
        y: 0.,
        width: 0.5,
        height: 1.,
    });
    let mut locked_child = leaf(5, 20., 90.);
    locked_child.locked = true;
    let mut group = Node::group(2, Point::new(50., 60.), vec![inner, locked_child]);
    group.crop = Some(Crop {
        x: 0.25,
        y: 0.2,
        width: 0.5,
        height: 0.6,
    });
    let mut p = Project {
        next_id: 100,
        ..Project::default()
    };
    p.pages[0].content = Content {
        nodes: vec![leaf(3, 400., 0.), group],
        paint_order: vec![vec![2, 3, 2], vec![3], vec![2, 5], vec![2, 3, 4]],
    };
    p
}
#[test]
fn cropped_ungroup_reveals_children_without_moving_them_and_preserves_inner_crop() {
    let before = fixture();
    let mut expected = before.clone();
    expected.pages[0].content.nodes[1].crop = None;
    let expected = Editor::new(expected);
    let pixels = expected.rendered_scene(1).unwrap().png().unwrap();
    let svg = expected.rendered_scene(1).unwrap().svg();
    let mut e = Editor::new(before.clone());
    let clipped_pixels = e.rendered_scene(1).unwrap().png().unwrap();
    assert_ne!(clipped_pixels, pixels);
    e.select(2, false);
    e.ungroup();
    assert!(e.error.is_none(), "{:?}", e.error);
    assert_eq!(e.nodes().len(), 3);
    let nodes = e.nodes();
    let inner = nodes
        .iter()
        .find(|n| matches!(n.content, NodeContent::Group { .. }))
        .unwrap();
    assert_ne!(inner.id, 3); // Colliding sibling identity was repaired.
    assert_eq!(inner.origin, Point::new(60., 65.));
    assert_eq!(
        inner.crop,
        before.pages[0].content.nodes[1].clone().ungroup().unwrap()[0].crop
    );
    let locked = nodes.iter().find(|n| n.id == 5).unwrap();
    assert!(locked.locked);
    assert_eq!(locked.origin, Point::new(70., 150.));
    assert_eq!(e.rendered_scene(1).unwrap().png().unwrap(), pixels);
    assert_eq!(e.rendered_scene(1).unwrap().svg(), svg);
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(e.rendered_scene(1).unwrap().png().unwrap(), clipped_pixels);
    e.redo();
    assert_eq!(e.document, after);
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("ungrouped.wireframe");
    ProjectStore.save(&path, &after).unwrap();
    let reopened = Editor::new(ProjectStore.load(&path).unwrap());
    assert_eq!(reopened.rendered_scene(1).unwrap().png().unwrap(), pixels);
    assert_eq!(reopened.document, after);
}
#[test]
fn mixed_ungroup_keeps_locked_groups_other_selected_controls_and_layer_order() {
    let mut p = fixture();
    let mut locked = Node::group(8, Point::new(500., 200.), vec![leaf(9, 0., 0.)]);
    locked.locked = true;
    p.pages[0].content.nodes.push(locked.clone());
    p.pages[0].content.paint_order.push(vec![8, 9]);
    let mut e = Editor::new(p);
    e.selection = HashSet::from([2, 3, 8]);
    e.ungroup();
    assert!(e.error.is_none(), "{:?}", e.error);
    assert_eq!(e.nodes().iter().find(|n| n.id == 8), Some(&locked));
    assert!(e.selection.contains(&3) && e.selection.contains(&8));
    assert_eq!(e.selection.len(), 4);
    let order = &e.document.pages[0].content.paint_order;
    assert_eq!(order[1], vec![3]);
    assert_eq!(order[2], vec![5]);
    assert_eq!(order[4], vec![8, 9]);
    e.document.validate().unwrap();
    let mut e = Editor::new(e.document);
    e.selection = HashSet::from([8]);
    e.ungroup();
    assert!(e.undo_name().is_none() && !e.dirty());
}
#[test]
fn ungroup_empty_cropped_group_removes_its_paint_unit_and_undo_restores_it() {
    let mut p = Project {
        next_id: 100,
        ..Project::default()
    };
    let mut empty = Node::group(2, Point::new(50., 60.), vec![]);
    empty.crop = Some(Crop {
        x: 0.2,
        y: 0.2,
        width: 0.5,
        height: 0.5,
    });
    p.pages[0].content = Content {
        nodes: vec![empty, leaf(3, 400., 200.)],
        paint_order: vec![vec![2], vec![3]],
    };
    let mut e = Editor::new(p.clone());
    e.selection = HashSet::from([2, 3]);
    e.ungroup();
    assert!(e.error.is_none(), "{:?}", e.error);
    assert_eq!(e.nodes().len(), 1);
    assert_eq!(e.selection, HashSet::from([3]));
    assert_eq!(e.document.pages[0].content.paint_order, vec![vec![3]]);
    e.document.validate().unwrap();
    e.undo();
    assert_eq!(e.document, p);
}

#[test]
fn nested_ungroup_retains_ancestor_crop_and_context() {
    let mut p = fixture();
    let children = std::mem::take(&mut p.pages[0].content.nodes);
    let mut parent = Node::group(20, Point::new(30., 40.), children);
    parent.crop = Some(Crop {
        x: 0.1,
        y: 0.1,
        width: 0.7,
        height: 0.7,
    });
    p.pages[0].content.nodes = vec![parent];
    for path in &mut p.pages[0].content.paint_order {
        path.insert(0, 20);
    }
    let mut expected = p.clone();
    if let NodeContent::Group { children } = &mut expected.pages[0].content.nodes[0].content {
        children[1].crop = None;
    }
    let expected = Editor::new(expected)
        .rendered_scene(1)
        .unwrap()
        .png()
        .unwrap();
    let mut e = Editor::new(p.clone());
    assert!(e.enter(20));
    e.select(2, false);
    e.ungroup();
    assert!(e.error.is_none(), "{:?}", e.error);
    assert_eq!(e.context, vec![20]);
    assert_eq!(
        e.document.pages[0].content.nodes[0].crop,
        p.pages[0].content.nodes[0].crop
    );
    assert_eq!(e.rendered_scene(1).unwrap().png().unwrap(), expected);
    e.undo();
    assert_eq!(e.document, p);
    assert_eq!(e.context, vec![20]);
    assert_eq!(e.selection, HashSet::from([2]));
}
