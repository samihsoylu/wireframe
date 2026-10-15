// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, workspace::Editor};
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
