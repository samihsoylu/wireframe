// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, model::Bounds, workspace::Editor};
fn fixture() -> Editor {
    let mut p = Project {
        next_id: 100,
        ..Default::default()
    };
    let a = Node::control(3, "rectangle", Point::new(0., 0.), Size::fixed(80., 40.));
    let mut b = Node::control(4, "rectangle", Point::new(300., 0.), Size::fixed(80., 40.));
    b.markup = true;
    p.pages[0].content.nodes = vec![Node::group(2, Point::new(100., 100.), vec![a, b])];
    let mut e = Editor::new(p);
    e.select(2, false);
    e.set_markup_visible(false);
    e
}
#[test]
fn inspector_displays_visible_bounds_but_size_edit_targets_full_group_frame() {
    let mut e = fixture();
    let before = e.document.clone();
    assert_eq!(e.bounds(), Some(Bounds::new(100., 100., 380., 40.)));
    let state = e.selection_geometry().unwrap();
    assert_eq!(state.bounds, Bounds::new(100., 100., 80., 40.));
    e.begin("Resize group");
    e.update_selection_geometry(Bounds::new(100., 100., 160., 40.));
    e.commit();
    assert_eq!(e.bounds(), Some(Bounds::new(100., 100., 160., 40.)));
    assert_eq!(
        e.selection_geometry().unwrap().bounds,
        Bounds::new(100., 100., 34., 40.)
    );
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert!(!e.markup_visible());
}
#[test]
fn inspector_position_action_preserves_full_size_and_moves_hidden_descendants() {
    let mut e = fixture();
    let before = e.document.clone();
    e.begin("Move group");
    e.update_selection_position(Point::new(140., 160.));
    e.commit();
    assert_eq!(e.bounds(), Some(Bounds::new(140., 160., 380., 40.)));
    assert_eq!(
        e.selection_geometry().unwrap().bounds,
        Bounds::new(140., 160., 80., 40.)
    );
    assert_eq!(
        e.nodes()[0].content,
        before.pages[0].content.nodes[0].content
    );
    e.undo();
    assert_eq!(e.document, before);
}
#[test]
fn source_local_hidden_geometry_preserves_definition_and_locked_group_is_read_only() {
    let mut e = fixture();
    e.set_markup_visible(true);
    e.create_component("Shared annotations".into()).unwrap();
    let source = e.document.definitions.clone();
    e.set_markup_visible(false);
    let mut b = e.selection_geometry().unwrap().bounds;
    b.w = 160.;
    e.begin("Resize instance");
    e.update_selection_geometry(b);
    e.commit();
    assert_eq!(e.selection_geometry().unwrap().bounds.w, 34.);
    assert_eq!(e.document.definitions, source);
    e.undo();
    e.lock(true);
    let before = e.document.clone();
    let state = e.selection_geometry().unwrap();
    assert!(state.locked);
    assert_eq!(state.bounds.w, 80.);
    e.update_selection_position(Point::new(200., 200.));
    e.update_selection_geometry(Bounds::new(200., 200., 160., 40.));
    assert_eq!(e.document, before);
}

#[test]
fn hidden_crop_origin_preserves_children_and_failed_empty_preview_preserves_history() {
    let mut e = fixture();
    let before = e.document.clone();
    e.crop_group_at(
        2,
        Crop {
            x: 0.1,
            y: 0.,
            width: 0.5,
            height: 1.,
        },
        Point::new(120., 100.),
    )
    .unwrap();
    let (full, visible, _) = e.group_crop_geometry(2).unwrap();
    assert_eq!(full, Bounds::new(82., 100., 380., 40.));
    assert_eq!(visible, Bounds::new(120., 100., 190., 40.));
    assert_eq!(
        e.selection_geometry().unwrap().bounds,
        Bounds::new(120., 100., 42., 40.)
    );
    assert_eq!(
        e.nodes()[0].content,
        before.pages[0].content.nodes[0].content
    );
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    // A fully clipped plain child leaves no finite crop preview to manipulate.
    e.crop_group_at(
        2,
        Crop {
            x: 0.5,
            y: 0.,
            width: 0.5,
            height: 1.,
        },
        Point::new(120., 100.),
    )
    .unwrap();
    let empty = e.document.clone();
    assert!(e.group_crop_edit_geometry(2).is_err());
    assert_eq!(e.document, empty);
    e.undo();
    assert_eq!(e.document, after);
}

#[test]
fn hidden_crop_removal_uses_visible_uncropped_origin_and_keeps_exact_history() {
    let mut e = fixture();
    let NodeContent::Group { children } = &mut e.document.pages[0].content.nodes[0].content else {
        unreachable!()
    };
    children[0] = Node::control(3, "rectangle", Point::new(0., 0.), Size::fixed(81., 40.));
    children[0].markup = true;
    children[1] = Node::control(4, "rectangle", Point::new(299., 0.), Size::fixed(81., 40.));
    let baseline = e.document.clone();
    e.crop_group_at(
        2,
        Crop {
            x: 0.,
            y: 0.,
            width: 0.802,
            height: 0.8,
        },
        Point::new(399., 100.),
    )
    .unwrap();
    assert_eq!(
        e.selection_geometry().unwrap().bounds,
        Bounds::new(698., 100., 6., 32.)
    );
    let cropped = e.document.clone();
    e.crop_group(2, None, Point::default()).unwrap();
    assert_eq!(e.nodes()[0].origin, Point::new(698., 100.));
    assert_eq!(
        e.selection_geometry().unwrap().bounds,
        Bounds::new(997., 100., 81., 40.)
    );
    assert_eq!(
        e.nodes()[0].content,
        baseline.pages[0].content.nodes[0].content
    );
    let removed = e.document.clone();
    e.undo();
    assert_eq!(e.document, cropped);
    e.redo();
    assert_eq!(e.document, removed);
    e.undo();
    e.undo();
    assert_eq!(e.document, baseline);
}

#[test]
fn offset_hidden_crop_removal_is_local_and_empty_masks_remain_removable() {
    for instance in [false, true] {
        let mut e = fixture();
        if instance {
            e.create_component("Crop subject".into()).unwrap();
            e.set_markup_visible(false);
        }
        assert!(!e.markup_visible());
        let source = e.document.definitions.clone();
        let baseline = e.document.clone();
        e.crop_group_at(
            2,
            Crop {
                x: 0.1,
                y: 0.,
                width: 0.5,
                height: 1.,
            },
            Point::new(120., 100.),
        )
        .unwrap();
        let cropped = e.document.clone();
        e.crop_group(2, None, Point::default()).unwrap();
        assert_eq!(e.nodes()[0].origin, Point::new(112., 100.));
        assert_eq!(
            e.selection_geometry().unwrap().bounds,
            Bounds::new(112., 100., 80., 40.)
        );
        assert_eq!(e.document.definitions, source);
        let removed = e.document.clone();
        e.undo();
        assert_eq!(e.document, cropped);
        e.redo();
        assert_eq!(e.document, removed);
        e.undo();
        e.undo();
        assert_eq!(e.document, baseline);
        e.crop_group_at(
            2,
            Crop {
                x: 0.5,
                y: 0.,
                width: 0.5,
                height: 1.,
            },
            Point::new(120., 100.),
        )
        .unwrap();
        assert!(e.group_crop_edit_geometry(2).is_err());
        let empty = e.document.clone();
        e.crop_group(2, None, Point::default()).unwrap();
        assert_eq!(e.group_crop_geometry(2).unwrap().2, None);
        assert!(e.nodes()[0].origin.x.is_finite() && e.nodes()[0].origin.y.is_finite());
        assert_eq!(e.document.definitions, source);
        e.undo();
        assert_eq!(e.document, empty);
    }
}
