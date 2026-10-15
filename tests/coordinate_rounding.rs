// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, model::Bounds, workspace::Editor};
fn leaf(id: Id, x: f32, y: f32, w: f32, h: f32) -> Node {
    Node::control(id, "rectangle", Point::new(x, y), Size::fixed(w, h))
}
fn fixture(instance: bool) -> Editor {
    let inner = Node::group(
        4,
        Point::new(75., 20.),
        vec![leaf(5, 0., 0., 35., 25.), leaf(6, 80., 45., 45., 25.)],
    );
    let mut outer = Node::group(
        2,
        Point::new(100., 100.),
        vec![leaf(3, 0., 0., 40., 30.), inner],
    );
    let mut p = Project {
        next_id: 100,
        ..Project::default()
    };
    if instance {
        outer.origin = Point::default();
        p.definitions.push(Definition {
            id: 20,
            name: "Panel".into(),
            content: Content {
                nodes: vec![outer],
                paint_order: vec![],
            },
        });
        let mut n = Node::group(21, Point::new(100., 100.), vec![]);
        n.content = NodeContent::Instance {
            definition: 20,
            overrides: vec![],
        };
        let mut sibling = n.clone();
        sibling.id = 22;
        sibling.origin.x = 500.;
        p.pages[0].content.nodes = vec![n, sibling];
    } else {
        p.pages[0].content.nodes = vec![outer];
    }
    Editor::new(p)
}
fn resize(e: &mut Editor, id: Id, w: f32, h: f32) {
    e.select(id, false);
    e.begin("Resize");
    e.selected_mut(|c| {
        c.bounds.w = w;
        c.bounds.h = h;
    });
    e.commit();
    assert!(e.error.is_none(), "{:?}", e.error);
}
#[test]
fn nested_rounding_includes_parent_displacement_and_preserves_history() {
    for instance in [false, true] {
        for width in [201., 203.] {
            let mut e = fixture(instance);
            let id = if instance { 21 } else { 2 };
            let before = e.document.clone();
            resize(&mut e, id, width, 90.);
            let layout = e.layout_page(e.page).unwrap();
            let root = &layout.nodes[0];
            let group = if instance { &root.children[0] } else { root };
            let inner = &group.children[1];
            assert_eq!(root.bounds.width, width);
            assert_eq!(inner.bounds.x, if width == 201. { 175. } else { 176. });
            assert_eq!(
                inner.children[1].bounds.x,
                if width == 201. { 256. } else { 257. }
            );
            assert_eq!(
                inner.children[1].bounds.width,
                if width == 201. { 45. } else { 46. }
            );
            let after = e.document.clone();
            assert_eq!(after.definitions, before.definitions);
            if instance {
                assert_eq!(
                    after.pages[0].content.nodes[1],
                    before.pages[0].content.nodes[1]
                );
            }
            e.undo();
            assert_eq!(e.document, before);
            e.redo();
            assert_eq!(e.document, after);
            let svg = e.rendered_scene(e.page).unwrap().svg();
            let reopened = Editor::new(decode(&after.encode().unwrap()).unwrap());
            assert_eq!(reopened.rendered_scene(reopened.page).unwrap().svg(), svg);
            resize(&mut e, id, width, 93.);
            let layout = e.layout_page(e.page).unwrap();
            let root = &layout.nodes[0];
            let group = if instance { &root.children[0] } else { root };
            assert_eq!(group.children[1].bounds.y, 121.);
            assert_eq!(group.children[1].children[1].bounds.y, 167.);
            assert_eq!(group.children[1].children[1].bounds.height, 26.);
        }
    }
}
#[test]
fn zero_group_crop_keeps_content_and_can_be_removed_and_undone() {
    let mut p = Project::default();
    let mut group = Node::group(2, Point::new(75., 75.), vec![leaf(3, 0., 0., 100., 100.)]);
    group.crop = Some(Crop {
        x: 0.25,
        y: 0.25,
        width: 0.001,
        height: 0.001,
    });
    p.pages[0].content.nodes.push(group);
    p.next_id = 4;
    let mut e = Editor::new(p.clone());
    e.select(2, false);
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(100., 100., 0., 0.)
    );
    let bytes = e.document.encode().unwrap();
    assert!(e.rendered_scene(e.page).unwrap().png().is_ok());
    assert_eq!(e.document.encode().unwrap(), bytes);
    e.crop_group(2, None, Point::default()).unwrap();
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(100., 100., 100., 100.)
    );
    e.undo();
    assert_eq!(e.document, p);
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(100., 100., 0., 0.)
    );
}

#[test]
fn explicit_resize_rounds_half_ties_but_loading_and_moving_keep_fractions() {
    let mut p = Project {
        next_id: 4,
        ..Project::default()
    };
    p.pages[0].content.nodes = vec![leaf(3, -10.5, 10.5, 20.25, 30.75)];
    let bytes = p.encode().unwrap();
    let mut e = Editor::new(decode(&bytes).unwrap());
    e.select(3, false);
    assert_eq!(e.document.encode().unwrap(), bytes);
    e.edit("Move", |e| e.selected_mut(|c| c.bounds.x += 0.25));
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(-10.25, 10.5, 20.25, 30.75)
    );
    e.undo();
    resize(&mut e, 3, 11.5, 30.75);
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(-10., 11., 12., 31.)
    );
    e.undo();
    assert_eq!(e.document, p);
    let mut p = Project {
        next_id: 4,
        ..Project::default()
    };
    p.pages[0].content.nodes = vec![leaf(3, -11.5, -1.5, 20., 30.)];
    let mut e = Editor::new(p);
    resize(&mut e, 3, 11.5, 30.);
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(-11., -1., 12., 30.)
    );
}
