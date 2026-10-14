// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{catalog, document::*, model::Bounds, workspace::Editor};
fn fixture() -> (Editor, Id, Id) {
    let mut p = Project::default();
    let mut rect = Node::control(4, "rectangle", Point::new(0., 0.), Size::fixed(80., 30.));
    rect.properties
        .insert("text".into(), Value::Text("Panel".into()));
    let mut label = catalog::get("label")
        .unwrap()
        .create(5, Point::new(0., 100.));
    label
        .properties
        .insert("text".into(), Value::Text("Bottom label".into()));
    p.definitions.push(Definition {
        id: 2,
        name: "Inner".into(),
        content: Content {
            nodes: vec![rect],
            paint_order: vec![],
        },
    });
    let mut nested = Node::group(6, Point::default(), vec![]);
    nested.content = NodeContent::Instance {
        definition: 2,
        overrides: vec![],
    };
    p.definitions.push(Definition {
        id: 3,
        name: "Outer".into(),
        content: Content {
            nodes: vec![nested, label],
            paint_order: vec![],
        },
    });
    let mut instance = Node::group(7, Point::new(50., 60.), vec![]);
    instance.content = NodeContent::Instance {
        definition: 3,
        overrides: vec![],
    };
    let mut other = instance.clone();
    other.id = 8;
    other.origin.y = 250.;
    p.pages[0].content.nodes = vec![instance, other];
    p.next_id = 9;
    p.validate().unwrap();
    (Editor::new(p), 7, 5)
}
fn resize(e: &mut Editor, id: Id, b: Bounds) {
    e.begin("Resize component");
    let mut c = e
        .current()
        .controls
        .into_iter()
        .find(|c| c.id == id)
        .unwrap();
    c.bounds = b;
    e.update_control(c);
    e.commit();
    assert!(e.error.is_none(), "{:?}", e.error);
}
#[test]
fn whole_instance_resize_freezes_geometry_preserves_source_and_resets() {
    let (mut e, root, label) = fixture();
    let definitions = e.document.definitions.clone();
    e.enter(root);
    e.begin("Local text");
    e.set_control_property(label, "text", Value::Text("Local bottom".into()))
        .unwrap();
    e.commit();
    e.leave();
    let b = e.current().controls[0].bounds;
    let before = e.rendered_scene(1).unwrap().svg();
    resize(&mut e, root, Bounds::new(b.x, b.y, b.w * 2., b.h * 2.));
    let view = e.current().controls[0].bounds;
    assert_eq!(view.w, b.w * 2.);
    assert_eq!(view.h, b.h * 2.);
    assert_eq!(e.document.definitions, definitions);
    let layers = e.layers(1).unwrap();
    assert_eq!(layers[0].control.bounds.w, 160.);
    assert_eq!(layers[0].control.bounds.h, 60.);
    assert_eq!(
        layers[1].control.bounds.y + layers[1].control.bounds.h,
        view.y + view.h
    );
    assert_eq!(layers[1].control.font_size, 13.);
    assert_eq!(layers[1].control.text, "Local bottom");
    assert_eq!(layers[2].control.bounds.w, 80.);
    let NodeContent::Instance { overrides, .. } = &e.nodes()[0].content else {
        panic!()
    };
    assert!(overrides.iter().any(|o| o.path == [6, 4]
        && o.patch.origin == Some(Point::default())
        && o.patch.size == Some(Size::fixed(160., 60.))));
    let resized = e.rendered_scene(1).unwrap().svg();
    e.undo();
    assert_eq!(e.rendered_scene(1).unwrap().svg(), before);
    e.redo();
    assert_eq!(e.rendered_scene(1).unwrap().svg(), resized);
    let reopened = Editor::new(decode(&e.document.encode().unwrap()).unwrap());
    assert_eq!(resized, reopened.rendered_scene(1).unwrap().svg());
    e.reset_instance(root).unwrap();
    assert_eq!(e.layers(1).unwrap()[0].control.bounds.w, 80.);
    assert_eq!(e.layers(1).unwrap()[1].control.text, "Bottom label");
    e.undo();
    assert_eq!(resized, e.rendered_scene(1).unwrap().svg());
    // Later source position/size edits do not overwrite resized instance geometry.
    e.edit_definition(2).unwrap();
    let mut c = e.current().controls[0].clone();
    c.bounds.x = 10.;
    c.bounds.w = 90.;
    e.begin("Source geometry");
    e.update_control(c);
    e.commit();
    assert_eq!(e.layers(1).unwrap()[0].control.bounds.x, 50.);
    assert_eq!(e.layers(1).unwrap()[0].control.bounds.w, 160.);
    assert_eq!(e.layers(1).unwrap()[2].control.bounds.x, 60.);
    assert_eq!(e.layers(1).unwrap()[2].control.bounds.w, 90.);
}
#[test]
fn group_resize_handles_nested_instances_and_keeps_fixed_height_bottom_anchor() {
    let (mut e, root, _) = fixture();
    e.select_all();
    e.group();
    let group = e.nodes()[0].id;
    let b = e.current().controls[0].bounds;
    let definitions = e.document.definitions.clone();
    resize(
        &mut e,
        group,
        Bounds::new(b.x + 20., b.y + 10., b.w * 1.5, b.h * 1.5),
    );
    assert_eq!(e.document.definitions, definitions);
    let layers = e.layers(1).unwrap();
    assert_eq!(layers[0].control.bounds.w, 120.);
    assert_eq!(layers[2].control.bounds.w, 120.);
    let label_height = layers[1].control.bounds.h;
    assert_eq!(label_height, layers[3].control.bounds.h);
    e.enter(group);
    let first = e
        .current()
        .controls
        .into_iter()
        .find(|c| c.id == root)
        .unwrap();
    assert_eq!(
        layers[1].control.bounds.y + label_height,
        first.bounds.y + first.bounds.h
    );
    e.document.validate().unwrap();
}
#[test]
fn moving_an_instance_does_not_create_geometry_overrides_and_locked_resize_is_ignored() {
    let (mut e, root, _) = fixture();
    e.select(root, false);
    e.move_by(20., 10.);
    let NodeContent::Instance { overrides, .. } = &e.nodes()[0].content else {
        panic!()
    };
    assert!(overrides.is_empty());
    e.lock(true);
    let old = e.document.clone();
    let b = e.current().controls[0].bounds;
    resize(&mut e, root, Bounds::new(b.x, b.y, b.w * 2., b.h * 2.));
    assert_eq!(e.document, old);
}
