// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{catalog, document::*, model::Bounds, workspace::Editor};

#[test]
fn resolved_selection_limits_follow_fixed_axes_overrides_and_editing_context() {
    let mut label = Node::control(4, "label", Point::new(0., 150.), Size::fixed(100., 20.));
    label.locked = true;
    let slider = catalog::get("vertical_slider")
        .unwrap()
        .create(5, Point::default());
    let mut group = Node::group(3, Point::default(), vec![label, slider]);
    group.crop = Some(Crop {
        x: 0.,
        y: 0.,
        width: 1.,
        height: 0.25,
    });
    let mut p = Project {
        next_id: 100,
        ..Project::default()
    };
    p.definitions.push(Definition {
        id: 20,
        name: "Panel".into(),
        content: Content {
            nodes: vec![group],
            paint_order: vec![],
        },
    });
    let mut instance = Node::group(21, Point::new(100., 100.), vec![]);
    instance.content = NodeContent::Instance {
        definition: 20,
        overrides: vec![Override {
            path: vec![3, 4],
            patch: Patch {
                size: Some(Size::fixed(100., 40.)),
                ..Patch::default()
            },
        }],
    };
    let arrow = Node::control(22, "arrow", Point::new(400., 0.), Size::fixed(100., 100.));
    let color = catalog::get("color_picker")
        .unwrap()
        .create(23, Point::new(500., 0.));
    p.pages[0].content.nodes = vec![instance, arrow, color];
    let mut e = {
        p.organize_components().unwrap();
        Editor::new(p)
    };
    let saved = e.document.clone();
    assert_eq!(e.resize_minimum(&[21]), (12., 40.));
    assert_eq!(e.resize_minimum(&[22]), (1., 1.));
    assert_eq!(e.resize_minimum(&[21, 22]), (12., 40.));
    let color = e
        .current()
        .controls
        .into_iter()
        .find(|c| c.id == 23)
        .unwrap()
        .bounds;
    assert_eq!(e.resize_minimum(&[23]), (color.w, color.h));
    assert_eq!(
        e.resize_minimum(&[21, 23]),
        (12_f32.max(color.w), 40_f32.max(color.h))
    );
    assert!(e.enter(21));
    assert_eq!(e.resize_minimum(&[3]), (12., 40.));
    assert!(e.enter(3));
    assert_eq!(e.resize_minimum(&[4]), (10., 40.));
    assert_eq!(e.resize_minimum(&[5]), (12., 10.));
    assert_eq!(e.document, saved);
}

#[test]
fn asymmetric_limits_preserve_opposite_edges_and_aspect_for_all_handles() {
    let original = Bounds::new(100., 200., 120., 80.);
    for x in [-1, 0, 1] {
        for y in [-1, 0, 1] {
            if (x, y) == (0, 0) {
                continue;
            }
            for aspect in [false, true] {
                let b = original.resized_with_minimum_size(
                    (x, y),
                    (-200. * x as f32, -200. * y as f32),
                    aspect,
                    (30., 40.),
                );
                assert!(b.w >= 30. && b.h >= 40.);
                if aspect {
                    assert_eq!(b.w / b.h, original.w / original.h);
                } else {
                    assert_eq!(b.w, if x == 0 { 120. } else { 30. });
                    assert_eq!(b.h, if y == 0 { 80. } else { 40. });
                }
                if x < 0 {
                    assert_eq!(b.x + b.w, original.x + original.w);
                } else if x > 0 {
                    assert_eq!(b.x, original.x);
                }
                if y < 0 {
                    assert_eq!(b.y + b.h, original.y + original.h);
                } else if y > 0 {
                    assert_eq!(b.y, original.y);
                }
            }
        }
    }
    // A stored fixed dimension may exceed the ordinary interactive size cap.
    let large = original.resized_with_minimum_size((1, 1), (-200., -200.), false, (12_000., 40.));
    assert_eq!((large.w, large.h), (12_000., 40.));
}
