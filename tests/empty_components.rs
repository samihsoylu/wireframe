// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, model::Kind, workspace::Editor};

fn leaf(id: Id, x: f32) -> Node {
    Node::control(id, "rectangle", Point::new(x, 0.), Size::fixed(80., 40.))
}
fn layered() -> Editor {
    let mut p = Project {
        next_id: 100,
        ..Project::default()
    };
    p.pages[0].content = Content {
        nodes: vec![
            leaf(2, 400.),
            Node::group(3, Point::new(50., 60.), vec![leaf(4, 0.)]),
            leaf(5, 600.),
        ],
        paint_order: vec![vec![5], vec![3, 4], vec![2]],
    };
    Editor::new(p)
}
#[test]
fn empty_sources_detach_repopulate_and_reopen_without_rebinding_old_overrides() {
    let mut e = Editor::default();
    e.add(Kind::Button, 50., 60.);
    let first = e.nodes()[0].id;
    let source = e.create_component("Reusable".into()).unwrap();
    e.enter(first);
    let child = e.nodes()[0].id;
    e.set_control_property(child, "text", Value::Text("Old local value".into()))
        .unwrap();
    e.leave();
    let second = e.insert_component(source, 300., 60.).unwrap();
    let canonical = e.document.pages[0].content.clone();
    let populated = e.rendered_scene(1).unwrap().svg();
    e.edit_definition(source).unwrap();
    e.select_all();
    e.delete();
    assert!(e.nodes().is_empty());
    assert!(e.current().controls.is_empty());
    assert_eq!(e.document.pages[0].content, canonical);
    let empty = e.rendered_scene(1).unwrap().svg();
    assert_eq!(empty.matches("Empty component").count(), 2);
    assert!(e.rendered_scene(1).unwrap().png().unwrap().len() > 100);
    e.undo();
    assert_eq!(e.rendered_scene(1).unwrap().svg(), populated);
    e.redo();
    assert_eq!(e.rendered_scene(1).unwrap().svg(), empty);
    let reopened = Editor::new(decode(&e.document.encode().unwrap()).unwrap());
    assert_eq!(reopened.rendered_scene(1).unwrap().svg(), empty);
    e.switch_page(1);
    assert!(e.empty_container(first));
    assert!(!e.missing_component(first));
    assert!(e.enter(first));
    assert!(e.nodes().is_empty());
    e.leave();
    e.detach_component(second).unwrap();
    assert!(e.is_group(second));
    assert!(e.empty_container(second));
    assert!(!e.nodes()[1].properties.contains_key(EMPTY_COMPONENT));
    assert_eq!(
        e.rendered_scene(1)
            .unwrap()
            .svg()
            .matches("Empty group")
            .count(),
        1
    );
    e.edit_definition(source).unwrap();
    e.add(Kind::Label, 0., 0.);
    let new_child = e.nodes()[0].id;
    assert_ne!(new_child, child);
    assert!(
        !e.rendered_scene(1)
            .unwrap()
            .svg()
            .contains("Old local value")
    );
    assert!(
        !e.rendered_scene(1)
            .unwrap()
            .svg()
            .contains("Empty component")
    );
    assert_eq!(e.document.pages[0].content.nodes[0], canonical.nodes[0]);
    assert_eq!(
        e.document
            .resolve(&canonical.nodes[0])
            .unwrap()
            .orphan_overrides
            .len(),
        1
    );
    e.undo();
    assert!(
        e.rendered_scene(1)
            .unwrap()
            .svg()
            .contains("Empty component")
    );
    e.redo();
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
}
#[test]
fn last_child_deletion_first_child_add_paste_and_insert_keep_layer_position() {
    let mut e = layered();
    e.enter(3);
    e.select_all();
    e.delete();
    assert!(e.error.is_none(), "{:?}", e.error);
    assert_eq!(
        e.document.pages[0].content.paint_order,
        vec![vec![5], vec![3], vec![2]]
    );
    assert_eq!(
        e.layers(1)
            .unwrap()
            .iter()
            .map(|l| l.path.clone())
            .collect::<Vec<_>>(),
        vec![vec![5], vec![3], vec![2]]
    );
    e.add(Kind::Label, 50., 60.);
    let added = e.nodes()[0].id;
    assert_eq!(
        e.document.pages[0].content.paint_order,
        vec![vec![5], vec![3, added], vec![2]]
    );
    e.undo();
    assert!(e.nodes().is_empty());
    e.redo();
    assert_eq!(e.nodes()[0].id, added);
    e.undo();
    let mut other = Editor::default();
    other.add(Kind::Button, 50., 60.);
    let copy = other.copy();
    e.paste(&copy, 0.);
    let pasted = e.nodes()[0].id;
    assert_eq!(
        e.document.pages[0].content.paint_order,
        vec![vec![5], vec![3, pasted], vec![2]]
    );
    e.undo();
    let definition = e.document.allocate_id().unwrap();
    e.document.definitions.push(Definition {
        id: definition,
        name: "Empty source".into(),
        content: Content::default(),
    });
    e.document.organize_components().unwrap();
    let inserted = e.insert_component(definition, 50., 60.).unwrap();
    assert_eq!(
        e.document.pages[0].content.paint_order,
        vec![vec![5], vec![3, inserted], vec![2]]
    );
    e.document.validate().unwrap();
    e.detach_component(inserted).unwrap();
    assert!(e.is_group(inserted));
    e.ungroup();
    assert!(e.error.is_none(), "{:?}", e.error);
    assert_eq!(
        e.document.pages[0].content.paint_order,
        vec![vec![5], vec![3], vec![2]]
    );
}
#[test]
fn legacy_omitted_empty_groups_load_without_rewriting_and_invalid_orders_still_fail() {
    let mut e = layered();
    if let NodeContent::Group { children } = &mut e.document.pages[0].content.nodes[1].content {
        children.clear();
    }
    e.document.pages[0].content.paint_order = vec![vec![5], vec![2]];
    let before = e.document.clone();
    let encoded = before.encode().unwrap();
    let reopened = Editor::new(decode(&encoded).unwrap());
    assert_eq!(reopened.document, before);
    assert_eq!(
        reopened.document.pages[0].content.leaves().unwrap().len(),
        3
    );
    let paths = reopened
        .layers(1)
        .unwrap()
        .iter()
        .map(|l| l.path.clone())
        .collect::<Vec<_>>();
    assert_eq!(paths, vec![vec![5], vec![2], vec![3]]);
    assert!(
        reopened
            .rendered_scene(1)
            .unwrap()
            .svg()
            .contains("Empty group")
    );
    assert_eq!(reopened.document.encode().unwrap(), encoded);
    e.document.pages[0].content.paint_order = vec![vec![5]];
    assert!(e.document.validate().is_err());
    e.document.pages[0].content.paint_order = vec![vec![5], vec![2], vec![3], vec![3]];
    assert!(e.document.validate().is_err());
}
#[test]
fn empty_groups_are_selectable_editable_fixed_size_and_undoable_with_nested_clipping() {
    let mut e = layered();
    e.enter(3);
    e.select_all();
    e.delete();
    e.leave();
    let hit = e.hit(70., 80.).unwrap();
    assert_eq!(hit.id, 3);
    assert_eq!((hit.bounds.w, hit.bounds.h), (150., 150.));
    let mut c = hit;
    c.bounds.x += 10.;
    c.bounds.w = 300.;
    e.begin("Move empty group");
    e.update_control(c);
    e.commit();
    assert_eq!(
        (
            e.current().controls[1].bounds.x,
            e.current().controls[1].bounds.w
        ),
        (60., 150.)
    );
    e.select(3, false);
    e.lock(true);
    assert!(e.enter(3));
    e.leave();
    e.move_by(10., 10.);
    assert_eq!(e.current().controls[1].bounds.x, 60.);
    e.lock(false);
    e.ungroup();
    assert!(!e.nodes().iter().any(|n| n.id == 3));
    e.undo();
    assert!(e.nodes().iter().any(|n| n.id == 3));
    // Nested empty component remains a group, with clipped rendering and original identity.
    let definition = e.document.allocate_id().unwrap();
    e.document.definitions.push(Definition {
        id: definition,
        name: "Nested empty".into(),
        content: Content::default(),
    });
    e.document.organize_components().unwrap();
    e.enter(3);
    let instance = e.insert_component(definition, 60., 60.).unwrap();
    e.leave();
    e.document.pages[0].content.nodes[1].crop = Some(Crop {
        x: 0.,
        y: 0.,
        width: 0.5,
        height: 1.,
    });
    let layer = e
        .layers(1)
        .unwrap()
        .into_iter()
        .find(|l| l.path == [3, instance])
        .unwrap();
    assert_eq!(layer.clip.unwrap().w, 75.);
    assert!(e.enter(3));
    assert!(e.enter(instance));
    assert!(e.nodes().is_empty());
    e.leave();
    // Copying a resolved empty child produces ordinary empty content without derived markers.
    let outer = e.create_component("Wrapper".into());
    assert!(outer.is_err()); // already an instance
    e.leave();
    e.select(3, false);
    let wrapper = e.create_component("Wrapper".into()).unwrap();
    assert!(e.enter(3));
    e.select(instance, false);
    let copy = e.copy();
    assert!(!copy.nodes[0].properties.contains_key(EMPTY_COMPONENT));
    assert!(matches!(&copy.nodes[0].content,NodeContent::Group{children} if children.is_empty()));
    assert!(e.document.definitions.iter().any(|d| d.id == wrapper));
    e.document.validate().unwrap();
}

#[test]
fn retained_override_ids_above_allocator_cannot_attach_to_new_source_children() {
    let mut p = Project {
        next_id: 100,
        ..Project::default()
    };
    p.definitions.push(Definition {
        id: 10,
        name: "Empty".into(),
        content: Content::default(),
    });
    let mut node = Node::group(20, Point::default(), vec![]);
    node.content = NodeContent::Instance {
        definition: 10,
        overrides: vec![Override {
            path: vec![100],
            patch: Patch {
                properties: Properties::from([(
                    "text".into(),
                    Value::Text("Unrelated old text".into()),
                )]),
                ..Patch::default()
            },
        }],
    };
    p.pages[0].content.nodes.push(node.clone());
    if let NodeContent::Instance { overrides, .. } = &mut node.content {
        overrides[0].path = vec![101, u64::MAX];
    }
    p.pages[0].alternates.push(Alternate {
        version: None,
        notes: String::new(),
        id: 30,
        name: "Other".into(),
        content: Content {
            nodes: vec![node],
            paint_order: vec![],
        },
    });
    p.organize_components().unwrap();
    p.validate().unwrap();
    let original = decode(&p.encode().unwrap()).unwrap();
    assert_eq!(original, p);
    let mut e = Editor::new(original);
    e.edit_definition(10).unwrap();
    e.add(Kind::Label, 0., 0.);
    assert_eq!(e.nodes()[0].id, p.next_id);
    assert!(e.nodes()[0].id > 101);
    assert!(
        !e.rendered_scene(1)
            .unwrap()
            .svg()
            .contains("Unrelated old text")
    );
    assert_eq!(
        e.document
            .resolve(&e.document.pages[0].content.nodes[0])
            .unwrap()
            .orphan_overrides
            .len(),
        1
    );
    e.undo();
    assert_eq!(e.document, p);
    e.redo();
    e.document.validate().unwrap();
}
