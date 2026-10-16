// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, workspace::Editor};

fn fixture(instance: bool) -> (Editor, Id) {
    let mut contact = Node::control(4, "button", Point::default(), Size::fixed(100., 40.));
    contact
        .properties
        .insert("text".into(), Value::Text("Contact".into()));
    contact
        .properties
        .insert("link".into(), Value::Link(Link::Page(20)));
    let mut order = Node::control(5, "button", Point::new(100., 80.), Size::fixed(100., 40.));
    order
        .properties
        .insert("text".into(), Value::Text("Order".into()));
    order
        .properties
        .insert("link".into(), Value::Link(Link::Page(22)));
    let mut inner = Node::group(3, Point::default(), vec![contact, order]);
    inner.crop = Some(Crop {
        x: 0.25,
        y: 0.,
        width: 0.5,
        height: 1.,
    });
    let mut root = Node::group(6, Point::new(0., 100.), vec![inner]);
    root.crop = Some(Crop {
        x: 0.5,
        y: 0.,
        width: 0.5,
        height: 1.,
    });
    let mut project = Project {
        next_id: 100,
        ..Project::default()
    };
    project.pages[0].name = "Landing".into();
    if instance {
        root.origin = Point::default();
        project.definitions.push(Definition {
            id: 30,
            name: "Actions".into(),
            content: Content {
                nodes: vec![root],
                paint_order: vec![],
            },
        });
        root = Node::group(21, Point::new(0., 100.), vec![]);
        root.content = NodeContent::Instance {
            definition: 30,
            overrides: vec![],
        };
    }
    let id = root.id;
    project.pages[0].content.nodes = vec![
        root,
        Node::control(7, "button", Point::new(300., 100.), Size::fixed(100., 40.)),
    ];
    for (id, name) in [(20, "Contact"), (22, "Order")] {
        let mut page = project.pages[0].clone();
        page.id = id;
        page.name = name.into();
        page.content = Content::default();
        project.pages.push(page);
    }
    (Editor::new(project), id)
}

#[test]
fn nested_editing_reveals_ancestor_crops_for_selection_without_changing_saved_content() {
    for instance in [false, true] {
        let (mut e, id) = fixture(instance);
        let before = e.document.clone();
        let export = e.rendered_scene(e.page).unwrap().svg();
        assert!(e.hit(20., 120.).is_none());
        assert!(e.enter(id));
        if instance {
            assert!(e.enter(6));
        }
        assert_eq!(e.hit(60., 120.).unwrap().id, 3);
        assert!(e.hit(20., 120.).is_none());
        assert!(e.enter(3));
        assert_eq!(e.hit(20., 120.).map(|c| c.id), Some(4));
        assert_eq!(e.hit(180., 200.).map(|c| c.id), Some(5));
        assert!(e.hit(320., 120.).is_none());
        assert_eq!(e.rendered_scene(e.page).unwrap().svg(), export);
        while e.leave() {}
        assert!(e.hit(20., 120.).is_none());
        assert_eq!(e.document, before);
        assert!(!e.dirty());
        assert!(e.undo_name().is_none());
    }
}

#[test]
fn editing_scene_reveals_masks_by_level_while_exports_and_links_remain_cropped() {
    let (mut e, id) = fixture(false);
    let page = e.editing_scene().unwrap().svg();
    let canonical = e.rendered_scene(e.page).unwrap();
    assert_eq!(canonical.hit_link(20., 120.), None);
    assert_eq!(canonical.hit_link(120., 200.), Some("page:22"));
    assert!(e.enter(id));
    let outer = e.editing_scene().unwrap();
    assert_ne!(outer.svg(), page);
    assert_eq!(outer.hit_link(20., 120.), None);
    assert_eq!(outer.hit_link(60., 120.), Some("page:20"));
    let layers = e.editing_layers().unwrap();
    let contact = layers.iter().find(|l| l.control.id == 4).unwrap();
    assert_eq!(contact.clip.unwrap().x, 50.);
    assert_eq!(contact.clip.unwrap().w, 100.);
    assert_eq!(
        layers.iter().find(|l| l.control.id == 7).unwrap().opacity,
        0.4
    );
    assert!(e.enter(3));
    let inner = e.editing_scene().unwrap();
    assert_eq!(inner.hit_link(20., 120.), Some("page:20"));
    assert_eq!(inner.hit_link(180., 200.), Some("page:22"));
    let layers = e.editing_layers().unwrap();
    assert!(
        layers
            .iter()
            .filter(|l| l.control.id == 4 || l.control.id == 5)
            .all(|l| l.clip.is_none() && l.opacity == 1.)
    );
    assert_eq!(
        layers.iter().find(|l| l.control.id == 7).unwrap().opacity,
        0.2
    );
    assert!(inner.items.iter().any(|i| i.opacity == 0.2));
    assert_eq!(e.rendered_scene(e.page).unwrap().svg(), canonical.svg());
    assert_eq!(e.rendered_scene(e.page).unwrap().hit_link(20., 120.), None);
    assert_eq!(e.editing_scene().unwrap().svg(), inner.svg());
    e.leave();
    assert_eq!(e.editing_scene().unwrap().svg(), outer.svg());
    e.leave();
    assert_eq!(e.editing_scene().unwrap().svg(), page);
}

#[test]
fn revealed_component_edits_invalidate_the_view_without_changing_source_or_crop() {
    let (mut e, id) = fixture(true);
    assert!(e.enter(id));
    assert!(e.enter(6));
    assert!(e.enter(3));
    let before = e.document.clone();
    let before_scene = e.editing_scene().unwrap().svg();
    e.begin("Edit revealed button");
    e.set_control_property(4, "text", Value::Text("Contact us".into()))
        .unwrap();
    e.commit();
    let after = e.document.clone();
    let after_scene = e.editing_scene().unwrap().svg();
    assert_ne!(before_scene, after_scene);
    assert_eq!(after.definitions, before.definitions);
    let NodeContent::Instance { overrides, .. } = &after.pages[0].content.nodes[0].content else {
        panic!("Expected instance")
    };
    assert!(
        overrides
            .iter()
            .all(|o| o.patch.crop.is_none() && o.patch.origin.is_none() && o.patch.size.is_none())
    );
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(e.editing_scene().unwrap().svg(), before_scene);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(e.editing_scene().unwrap().svg(), after_scene);
    let reopened = Editor::new(decode(&after.encode().unwrap()).unwrap());
    assert_eq!(
        reopened.editing_scene().unwrap().svg(),
        e.rendered_scene(e.page).unwrap().svg()
    );
    assert_eq!(
        reopened
            .rendered_scene(reopened.page)
            .unwrap()
            .hit_link(20., 120.),
        None
    );
}
