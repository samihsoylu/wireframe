// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, model::Bounds, storage::ProjectStore, workspace::Editor};
fn leaf(id: Id, x: f32, y: f32) -> Node {
    Node::control(id, "rectangle", Point::new(x, y), Size::fixed(80., 40.))
}
fn region() -> Crop {
    Crop {
        x: 0.25,
        y: 0.2,
        width: 0.5,
        height: 0.6,
    }
}
fn fixture() -> Project {
    let mut nested = Node::group(
        3,
        Point::new(10., 5.),
        vec![leaf(4, 0., 0.), leaf(5, 100., 0.)],
    );
    nested.crop = Some(Crop {
        x: 0.25,
        y: 0.,
        width: 0.5,
        height: 1.,
    });
    let mut group = Node::group(2, Point::new(50., 60.), vec![nested, leaf(6, 20., 90.)]);
    group.crop = Some(region());
    let mut p = Project {
        next_id: 100,
        ..Project::default()
    };
    p.pages[0].content = Content {
        nodes: vec![group, leaf(7, 400., 0.)],
        paint_order: vec![vec![2, 3, 5], vec![7], vec![2, 6], vec![2, 3, 4]],
    };
    p
}
#[test]
fn removal_anchors_visible_corner_preserves_nested_geometry_and_round_trips() {
    let before = fixture();
    let mut e = Editor::new(before.clone());
    e.select(2, false);
    let (full, visible, _) = e.group_crop_geometry(2).unwrap();
    assert_ne!((full.x, full.y), (50., 60.));
    e.crop_group(2, None, Point::default()).unwrap();
    let (expanded, frame, crop) = e.group_crop_geometry(2).unwrap();
    assert_eq!(crop, None);
    assert_eq!(expanded, frame);
    assert_eq!(
        (frame.x, frame.y, frame.w, frame.h),
        (visible.x, visible.y, full.w, full.h)
    );
    assert_eq!(
        e.nodes()[0].content,
        before.pages[0].content.nodes[0].content
    );
    assert_eq!(
        e.document.pages[0].content.paint_order,
        before.pages[0].content.paint_order
    );
    let after = e.document.clone();
    let png = e.rendered_scene(1).unwrap().png().unwrap();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("crop.wireframe");
    ProjectStore.save(&path, &after).unwrap();
    let reopened = Editor::new(ProjectStore.load(&path).unwrap());
    assert_eq!(reopened.document, after);
    assert_eq!(reopened.rendered_scene(1).unwrap().png().unwrap(), png);
}
#[test]
fn preview_reveals_target_only_preserves_nested_crop_and_relative_paint_order() {
    let p = fixture();
    let e = Editor::new(p.clone());
    let scene = e.group_crop_preview(2).unwrap();
    let mut expected = p.clone();
    expected.pages[0].content.nodes.remove(1);
    expected.pages[0].content.nodes[0].crop = None;
    expected.pages[0].content.paint_order.retain(|p| p[0] == 2);
    let expected = Editor::new(expected).rendered_scene(1).unwrap();
    assert_eq!(scene.png().unwrap(), expected.png().unwrap());
    assert_eq!(scene.svg(), expected.svg());
    assert_eq!(e.document, p);
    assert!(!e.dirty());
    assert!(e.undo_name().is_none());
}
#[test]
fn apply_moves_artwork_and_changes_mask_atomically_without_resizing_children() {
    let before = fixture();
    let mut e = Editor::new(before.clone());
    let (full, _, _) = e.group_crop_geometry(2).unwrap();
    e.crop_group(
        2,
        Some(Crop {
            x: 0.1,
            y: 0.1,
            width: 0.7,
            height: 0.7,
        }),
        Point::new(12., -8.),
    )
    .unwrap();
    let (after, visible, _) = e.group_crop_geometry(2).unwrap();
    assert_eq!(
        after,
        Bounds::new(full.x + 12., full.y - 8., full.w, full.h)
    );
    assert!((visible.x - (after.x + after.w * 0.1)).abs() < 0.001);
    assert_eq!(
        e.nodes()[0].content,
        before.pages[0].content.nodes[0].content
    );
    e.undo();
    assert_eq!(e.document, before);
    assert!(e.undo_name().is_none());
}
#[test]
fn local_crop_clear_and_reset_preserve_source_and_unrelated_overrides() {
    let mut p = fixture();
    let group = p.pages[0].content.nodes.remove(0);
    p.definitions.push(Definition {
        id: 20,
        name: "Card".into(),
        content: Content {
            nodes: vec![group],
            paint_order: vec![],
        },
    });
    let mut instance = Node::group(21, Point::new(30., 40.), vec![]);
    instance.content = NodeContent::Instance {
        definition: 20,
        overrides: vec![],
    };
    p.pages[0].content = Content {
        nodes: vec![instance],
        paint_order: vec![],
    };
    let mut e = Editor::new(p);
    let source = e.document.definitions.clone();
    e.crop_group(21, Some(region()), Point::default()).unwrap();
    e.enter(21);
    let (_, visible, _) = e.group_crop_geometry(2).unwrap();
    e.crop_group(2, None, Point::default()).unwrap();
    let (_, after, crop) = e.group_crop_geometry(2).unwrap();
    assert_eq!(crop, None);
    assert_eq!((after.x, after.y), (visible.x, visible.y));
    assert!(e.has_local_crop(2));
    let NodeContent::Instance { overrides, .. } = &e.document.pages[0].content.nodes[0].content
    else {
        panic!()
    };
    let patch = overrides.iter().find(|o| o.path == [2]).unwrap();
    assert_eq!(patch.patch.crop, Some(CropOverride::Clear));
    let origin = patch.patch.origin;
    assert!(origin.is_some());
    e.reset_local_crop(2).unwrap();
    assert!(!e.has_local_crop(2));
    assert_eq!(e.group_crop_geometry(2).unwrap().2, Some(region()));
    let NodeContent::Instance { overrides, .. } = &e.document.pages[0].content.nodes[0].content
    else {
        panic!()
    };
    assert_eq!(overrides[0].patch.origin, origin);
    assert_eq!(e.document.definitions, source);
    e.undo();
    assert_eq!(e.group_crop_geometry(2).unwrap().2, None);
}
#[test]
fn nested_preview_uses_world_origin_and_guards_reject_locked_empty_and_invalid_targets() {
    let p = fixture();
    let mut e = Editor::new(p);
    e.enter(2);
    let scene = e.group_crop_preview(3).unwrap();
    let mut expected = Project::default();
    let mut group = e.nodes()[0].clone();
    group.origin = Point::new(60., 65.);
    group.crop = None;
    expected.pages[0].content.nodes = vec![group];
    // Relative paint order is reversed in this fixture.
    expected.pages[0].content.paint_order = vec![vec![3, 5], vec![3, 4]];
    assert_eq!(
        scene.png().unwrap(),
        Editor::new(expected)
            .rendered_scene(1)
            .unwrap()
            .png()
            .unwrap()
    );
    e.context.clear();
    e.document.pages[0].content.nodes[0].locked = true;
    e.context = vec![2];
    let before = e.document.clone();
    assert!(e.crop_group(3, Some(region()), Point::default()).is_err());
    assert_eq!(e.document, before);
    let mut e = Editor::default();
    e.document.pages[0].content.nodes = vec![Node::group(2, Point::default(), vec![])];
    assert!(e.group_crop_geometry(2).is_err());
    assert!(e.group_crop_geometry(99).is_err());
    let mut e = Editor::new(fixture());
    let before = e.document.clone();
    assert!(
        e.crop_group(2, Some(Crop { x: -1., ..region() }), Point::default())
            .is_err()
    );
    assert!(
        e.crop_group(2, Some(region()), Point::new(f32::NAN, 0.))
            .is_err()
    );
    assert_eq!(e.document, before);
}

#[test]
fn crop_background_omits_only_the_target_and_keeps_the_normal_scene_cache() {
    let p = fixture();
    let mut e = Editor::new(p.clone());
    let original = e.rendered_scene(1).unwrap().png().unwrap();
    let background = e.crop_background(2).unwrap();
    let mut expected = p.clone();
    expected.pages[0].content.nodes.remove(0);
    expected.pages[0].content.paint_order = vec![vec![7]];
    assert_eq!(
        background.png().unwrap(),
        Editor::new(expected)
            .rendered_scene(1)
            .unwrap()
            .png()
            .unwrap()
    );
    assert_eq!(e.rendered_scene(1).unwrap().png().unwrap(), original);
    e.enter(2);
    let background = e.crop_background(3).unwrap();
    assert_ne!(background.png().unwrap(), original);
    assert_eq!(e.rendered_scene(1).unwrap().png().unwrap(), original);
}
