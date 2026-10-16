// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, model::Bounds, workspace::Editor};
fn fixture(instance: bool) -> Editor {
    let a = Node::control(3, "rectangle", Point::default(), Size::fixed(100., 40.));
    let mut b = Node::control(4, "rectangle", Point::new(170., 0.), Size::fixed(10., 40.));
    b.locked = true;
    let mut group = Node::group(2, Point::new(55., 92.), vec![a, b]);
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
    if instance {
        group.origin = Point::default();
        p.definitions.push(Definition {
            id: 20,
            name: "Panel".into(),
            content: Content {
                nodes: vec![group],
                paint_order: vec![],
            },
        });
        let mut n = Node::group(21, Point::new(55., 92.), vec![]);
        n.content = NodeContent::Instance {
            definition: 20,
            overrides: vec![],
        };
        let mut sibling = n.clone();
        sibling.id = 22;
        sibling.origin.x = 500.;
        p.pages[0].content.nodes = vec![n, sibling];
    } else {
        p.pages[0].content.nodes = vec![group];
    }
    Editor::new(p)
}
fn resize(e: &mut Editor, id: Id, width: f32) {
    let mut c = e
        .current()
        .controls
        .into_iter()
        .find(|c| c.id == id)
        .unwrap();
    c.bounds.w = width;
    e.begin("Resize");
    e.update_control(c);
    e.commit();
    assert!(e.error.is_none(), "{:?}", e.error);
}
#[test]
fn cropped_parent_resize_respects_child_minimum_and_retains_lock_and_mask() {
    for instance in [false, true] {
        let mut e = fixture(instance);
        let id = if instance { 21 } else { 2 };
        let before = e.document.clone();
        let mask = e.document.resolve(&e.nodes()[0]).unwrap().root;
        resize(&mut e, id, 10.);
        let root = e.document.resolve(&e.nodes()[0]).unwrap().root;
        let group = if instance { &children(&root)[0] } else { &root };
        assert_eq!(
            group.crop,
            if instance {
                children(&mask)[0].crop
            } else {
                mask.crop
            }
        );
        let children = children(group);
        let NodeContent::Control { size, .. } = children[1].content else {
            panic!()
        };
        assert_eq!(size.width, Dimension::Fixed(10.));
        assert!(children[1].locked);
        let layout = e.layout_page(e.page).unwrap();
        let root_layout = &layout.nodes[0];
        let group_layout = if instance {
            &root_layout.children[0]
        } else {
            root_layout
        };
        assert_eq!(group_layout.children[1].bounds.x, 114.);
        let visible = e.current().controls[0].bounds;
        assert_eq!(visible.w, 15., "Rounded minimum-clamped child union");
        assert_eq!(visible.x, 102., "Rounded crop follows the actual union");
        assert_eq!(e.document.definitions, before.definitions);
        if instance {
            assert_eq!(
                e.document.pages[0].content.nodes[1],
                before.pages[0].content.nodes[1]
            );
        }
        let after = e.document.clone();
        let svg = e.rendered_scene(1).unwrap().svg();
        e.undo();
        assert_eq!(e.document, before);
        e.redo();
        assert_eq!(e.document, after);
        let reopened = Editor::new(decode(&after.encode().unwrap()).unwrap());
        assert_eq!(reopened.rendered_scene(1).unwrap().svg(), svg);
    }
}
#[test]
fn moving_groups_preserves_existing_subminimum_geometry() {
    for instance in [false, true] {
        let mut e = fixture(instance);
        let group = if instance {
            &mut e.document.definitions[0].content.nodes[0]
        } else {
            &mut e.document.pages[0].content.nodes[0]
        };
        let NodeContent::Group { children } = &mut group.content else {
            panic!()
        };
        let NodeContent::Control { size, .. } = &mut children[0].content else {
            panic!()
        };
        *size = Size::fixed(2., 3.);
        let before = e.document.resolve(&e.nodes()[0]).unwrap().root;
        e.select(if instance { 21 } else { 2 }, false);
        e.move_by(20., 10.);
        let after = e.document.resolve(&e.nodes()[0]).unwrap().root;
        assert_eq!(before.content, after.content);
        assert_eq!(after.origin, before.origin.translated(Point::new(20., 10.)));
    }
}

fn children(n: &Node) -> &[Node] {
    match &n.content {
        NodeContent::Group { children } => children,
        _ => panic!("Expected group"),
    }
}

#[test]
fn cropped_frame_controls_fixed_height_anchoring_through_nested_groups_and_instances() {
    for nested in [false, true] {
        for instance in [false, true] {
            for locked in [false, true] {
                let panel =
                    Node::control(3, "rectangle", Point::default(), Size::fixed(100., 100.));
                let mut label =
                    Node::control(4, "label", Point::new(0., 80.), Size::fixed(100., 20.));
                label
                    .properties
                    .insert("text".into(), Value::Text("Bottom label".into()));
                label
                    .properties
                    .insert("font_size".into(), Value::Number(12.));
                label.locked = locked;
                let mut group = Node::group(2, Point::default(), vec![panel, label]);
                group.crop = Some(Crop {
                    x: 0.,
                    y: 0.,
                    width: 1.,
                    height: 0.5,
                });
                let mut root = if nested {
                    Node::group(5, Point::default(), vec![group])
                } else {
                    group
                };
                let mut p = Project {
                    next_id: 100,
                    ..Project::default()
                };
                if instance {
                    p.definitions.push(Definition {
                        id: 20,
                        name: "Cropped panel".into(),
                        content: Content {
                            nodes: vec![root],
                            paint_order: vec![],
                        },
                    });
                    root = Node::group(21, Point::default(), vec![]);
                    root.content = NodeContent::Instance {
                        definition: 20,
                        overrides: vec![],
                    };
                }
                root.origin = Point::new(100., 100.);
                let id = root.id;
                p.pages[0].content.nodes = vec![root];
                let mut e = Editor::new(p);
                let before = e.document.clone();
                let mut control = e.current().controls[0].clone();
                assert_eq!(control.bounds.h, 50.);
                control.bounds.h = 100.;
                e.begin("Resize cropped panel");
                e.update_control(control);
                e.commit();
                assert!(e.error.is_none(), "{:?}", e.error);
                let layout = e.layers(e.page).unwrap();
                let label = layout.iter().find(|l| l.control.id == 4).unwrap();
                assert_eq!(
                    label.control.bounds.y, 260.,
                    "nested={nested}, instance={instance}, locked={locked}"
                );
                assert_eq!(label.control.bounds.h, 20.);
                assert_eq!(label.control.locked, locked);
                assert_eq!(e.current().controls[0].bounds.h, 100.);
                assert_eq!(e.document.definitions, before.definitions);
                assert_eq!(e.current().controls[0].id, id);
                let after = e.document.clone();
                e.undo();
                assert_eq!(e.document, before);
                e.redo();
                assert_eq!(e.document, after);
                assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
            }
        }
    }
}

#[test]
fn offset_crop_resize_keeps_fixed_label_at_visible_bottom_and_clips_its_left_half() {
    for instance in [false, true] {
        for height in [20., 200.] {
            let mut label = Node::control(4, "label", Point::new(0., 130.), Size::fixed(100., 20.));
            label.locked = true;
            label
                .properties
                .insert("font_size".into(), Value::Number(12.));
            let mut inner = Node::group(
                3,
                Point::default(),
                vec![
                    Node::control(5, "rectangle", Point::default(), Size::fixed(200., 200.)),
                    label,
                ],
            );
            inner.crop = Some(Crop {
                x: 0.25,
                y: 0.25,
                width: 0.5,
                height: 0.5,
            });
            let mut root = Node::group(6, Point::new(50., 50.), vec![inner]);
            let mut p = Project {
                next_id: 100,
                ..Project::default()
            };
            if instance {
                root.origin = Point::default();
                p.definitions.push(Definition {
                    id: 20,
                    name: "Offset panel".into(),
                    content: Content {
                        nodes: vec![root],
                        paint_order: vec![],
                    },
                });
                root = Node::group(21, Point::new(50., 50.), vec![]);
                root.content = NodeContent::Instance {
                    definition: 20,
                    overrides: vec![],
                };
            }
            let mut e = Editor::new(p);
            e.document.pages[0].content.nodes = vec![root];
            let before = e.document.clone();
            let mut control = e.current().controls[0].clone();
            assert_eq!(control.bounds, Bounds::new(100., 100., 100., 100.));
            assert_eq!(e.resize_minimum(&[control.id]), (10., 20.));
            control.bounds.h = height;
            e.begin("Resize offset crop");
            e.update_control(control);
            e.commit();
            let layout = e.layout_page(e.page).unwrap();
            let leaves = layout.leaves();
            let (label, clipped) = leaves
                .iter()
                .find(|(n, _)| n.path.last() == Some(&4))
                .unwrap();
            assert_eq!(label.bounds.y, 100. + height - 20.);
            assert_eq!(label.bounds.height, 20.);
            assert_eq!(
                (clipped.x, clipped.y, clipped.width, clipped.height),
                (100., 100. + height - 20., 50., 20.)
            );
            assert_eq!(
                e.current().controls[0].bounds,
                Bounds::new(100., 100., 100., height)
            );
            assert_eq!(e.document.definitions, before.definitions);
            let after = e.document.clone();
            let svg = e.rendered_scene(e.page).unwrap().svg();
            e.undo();
            assert_eq!(e.document, before);
            e.redo();
            assert_eq!(e.document, after);
            let reopened = Editor::new(decode(&after.encode().unwrap()).unwrap());
            assert_eq!(reopened.rendered_scene(reopened.page).unwrap().svg(), svg);
        }
    }
}
