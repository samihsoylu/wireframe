// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, model::Kind, workspace::Editor};

fn field(e: &mut Editor, x: f32) -> Id {
    e.add(Kind::TextInput, x, 40.);
    let id = *e.selection.iter().next().unwrap();
    let mut view = e.selected_views()[0].clone();
    view.bounds.w = 500.;
    e.edit("Width", |e| e.update_control(view));
    id
}

#[test]
fn locked_content_and_auto_size_preserve_position_and_protect_manual_geometry() {
    let mut e = Editor::default();
    let id = field(&mut e, 50.);
    e.lock(true);
    let origin = e.nodes()[0].origin;
    assert!(e.prepare_text_edit(id).unwrap().is_some());
    e.begin("Content");
    e.set_control_property(id, "text", Value::Text("Contact".into()))
        .unwrap();
    e.set_control_property(id, "bold", Value::Bool(true))
        .unwrap();
    e.commit();
    let saved = e.document.clone();
    let mut c = e.selected_views()[0].clone();
    c.bounds.x += 20.;
    c.bounds.w += 80.;
    e.update_control(c);
    e.move_by(20., 30.);
    e.delete();
    assert_eq!(e.document, saved);
    e.select(id, false);
    e.begin("Auto-size");
    e.auto_size_control(id).unwrap();
    e.commit();
    assert!(e.current().controls[0].bounds.w < 500.);
    assert_eq!(e.nodes()[0].origin, origin);
    assert!(e.own_locked(id));
    e.undo();
    assert_eq!(e.document, saved);
    e.redo();
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
}

#[test]
fn entered_locked_group_protects_children_without_blocking_content() {
    let mut e = Editor::default();
    let a = field(&mut e, 20.);
    let b = field(&mut e, 600.);
    e.select_all();
    e.group();
    let group = *e.selection.iter().next().unwrap();
    e.lock(true);
    assert!(e.enter(group));
    assert!(!e.own_locked(a));
    assert!(e.position_locked(a));
    e.select(a, false);
    e.begin("Content");
    e.set_control_property(a, "italic", Value::Bool(true))
        .unwrap();
    e.commit();
    let before = e.document.clone();
    e.move_by(10., 10.);
    e.delete();
    assert_eq!(e.document, before);
    assert!(e.nodes().iter().any(|n| n.id == b));
    e.begin("Auto-size");
    e.auto_size_control(a).unwrap();
    e.commit();
    assert!(e.position_locked(a));
    e.leave();
    assert!(e.group_crop_geometry(group).is_err());
}

#[test]
fn mixed_formatting_updates_both_but_movement_only_updates_unlocked_member() {
    let mut e = Editor::default();
    let a = field(&mut e, 20.);
    e.lock(true);
    let b = field(&mut e, 600.);
    e.select(a, true);
    e.begin("Italic");
    for id in [a, b] {
        e.set_control_property(id, "italic", Value::Bool(true))
            .unwrap();
    }
    e.commit();
    e.move_by(1., 0.);
    for n in e.nodes().iter() {
        assert_eq!(n.properties["italic"], Value::Bool(true));
    }
    assert_eq!(e.nodes()[0].origin.x, 20.);
    assert_eq!(e.nodes()[1].origin.x, 601.);
}

#[test]
fn instance_local_locks_are_isolated_reversible_and_persistent() {
    let mut e = Editor::default();
    let root = field(&mut e, 20.);
    let definition = e.create_component("Contact field".into()).unwrap();
    let child = e.document.definitions[0].content.nodes[0].id;
    e.edit_definition(definition).unwrap();
    e.select(child, false);
    e.lock(true);
    let source = e.document.definitions.clone();
    e.switch_page(1);
    e.insert_component(definition, 600., 40.).unwrap();
    let sibling = *e.selection.iter().next().unwrap();
    assert!(e.enter(root));
    assert!(!e.own_locked(child));
    e.select(child, false);
    e.lock(true);
    assert!(e.own_locked(child));
    assert_eq!(e.document.definitions, source);
    e.undo();
    assert!(!e.own_locked(child));
    e.redo();
    assert!(e.own_locked(child));
    e.begin("Content");
    e.set_control_property(child, "text", Value::Text("Local".into()))
        .unwrap();
    e.commit();
    e.begin("Reset property");
    e.reset_control_property(child, "text").unwrap();
    e.commit();
    assert!(e.own_locked(child));
    let mut e = Editor::new(decode(&e.document.encode().unwrap()).unwrap());
    assert!(e.enter(root));
    assert!(e.own_locked(child));
    e.select(child, false);
    e.lock(false);
    assert!(!e.own_locked(child));
    e.undo();
    assert!(e.own_locked(child));
    e.reset_local_changes(child).unwrap();
    assert!(!e.own_locked(child));
    e.undo();
    assert!(e.own_locked(child));
    e.leave();
    assert!(e.enter(sibling));
    assert!(!e.own_locked(child));
    assert_eq!(e.document.definitions, source);
}

#[test]
fn single_badge_unlock_preserves_sibling_locks_and_never_unlocks_ancestors() {
    let mut e = Editor::default();
    let a = field(&mut e, 20.);
    let b = field(&mut e, 600.);
    e.select_all();
    e.group();
    let group = *e.selection.iter().next().unwrap();
    e.lock(true);
    assert!(e.enter(group));
    e.select(a, false);
    let before = e.document.clone();
    let undo = e.undo_name().map(str::to_owned);
    e.unlock_selected_control(a);
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), undo.as_deref());
    assert!(e.position_locked(a));
    // A child's own lock may be cleared, but its ancestor still locks geometry.
    e.select_all();
    e.lock(true);
    let before = e.document.clone();
    e.unlock_selected_control(a);
    assert!(!e.own_locked(a));
    assert!(e.own_locked(b));
    assert!(e.position_locked(a));
    assert_eq!(e.selection, std::collections::HashSet::from([a]));
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert!(!e.own_locked(a));
    assert!(e.own_locked(b));
}

#[test]
fn single_badge_unlock_in_instance_is_local_and_exactly_reversible() {
    let mut e = Editor::default();
    field(&mut e, 20.);
    field(&mut e, 600.);
    e.select_all();
    e.group();
    let root = *e.selection.iter().next().unwrap();
    let definition = e.create_component("Fields".into()).unwrap();
    e.insert_component(definition, 900., 40.).unwrap();
    let source = e.document.definitions.clone();
    assert!(e.enter(root));
    e.select_all();
    e.lock(true);
    let id = e.nodes()[0].id;
    let before = e.document.clone();
    e.unlock_selected_control(id);
    assert!(!e.own_locked(id));
    assert!(e.own_locked(e.nodes()[1].id));
    assert_eq!(e.document.definitions, source);
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
}

#[test]
fn nested_definition_locks_do_not_cross_instance_boundary_but_outer_patches_do() {
    let mut p = Project {
        next_id: 30,
        ..Project::default()
    };
    let mut leaf = Node::control(2, "text_input", Point::new(0., 0.), Size::fixed(100., 30.));
    leaf.locked = true;
    p.definitions = vec![
        Definition {
            id: 10,
            name: "Inner".into(),
            content: Content {
                nodes: vec![leaf],
                ..Content::default()
            },
        },
        Definition {
            id: 11,
            name: "Outer".into(),
            content: Content {
                nodes: vec![Node {
                    id: 3,
                    origin: Point::default(),
                    locked: true,
                    markup: false,
                    crop: None,
                    properties: Properties::new(),
                    content: NodeContent::Instance {
                        definition: 10,
                        overrides: vec![Override {
                            path: vec![2],
                            patch: Patch {
                                locked: Some(true),
                                ..Patch::default()
                            },
                        }],
                    },
                }],
                ..Content::default()
            },
        },
    ];
    let mut instance = Node {
        id: 4,
        origin: Point::default(),
        locked: false,
        markup: false,
        crop: None,
        properties: Properties::new(),
        content: NodeContent::Instance {
            definition: 11,
            overrides: vec![],
        },
    };
    let resolved = p.resolve(&instance).unwrap().root;
    let NodeContent::Group { children } = resolved.content else {
        panic!()
    };
    assert!(!children[0].locked);
    let NodeContent::Group { children } = &children[0].content else {
        panic!()
    };
    assert!(!children[0].locked);
    if let NodeContent::Instance { overrides, .. } = &mut instance.content {
        overrides.push(Override {
            path: vec![3, 2],
            patch: Patch {
                locked: Some(true),
                ..Patch::default()
            },
        });
    }
    let resolved = p.resolve(&instance).unwrap().root;
    let NodeContent::Group { children } = resolved.content else {
        panic!()
    };
    let NodeContent::Group { children } = &children[0].content else {
        panic!()
    };
    assert!(children[0].locked);
}

#[test]
fn version_three_migration_preserves_data_without_inventing_local_locks() {
    let mut e = Editor::default();
    let root = field(&mut e, 20.);
    e.create_component("Field".into()).unwrap();
    e.document.definitions[0].content.nodes[0].locked = true;
    let mut expected = e.document.clone();
    expected.libraries.clear();
    expected.default_library = None;
    let mut old = expected.clone();
    old.format_version = 3;
    expected.organize_components().unwrap();
    let bytes = serde_json::to_vec_pretty(&old).unwrap();
    let migrated = decode(&bytes).unwrap();
    assert_eq!(migrated, expected);
    let NodeContent::Instance { overrides, .. } = &migrated.pages[0].content.nodes[0].content
    else {
        panic!()
    };
    assert!(overrides.is_empty());
    let mut e = Editor::new(migrated);
    assert!(e.enter(root));
    assert!(!e.nodes()[0].locked);
}

#[test]
fn ancestor_lock_protects_position_but_child_crop_can_change() {
    let mut p = Project {
        next_id: 10,
        ..Project::default()
    };
    let child = Node::group(
        3,
        Point::new(10., 15.),
        vec![Node::control(
            4,
            "rectangle",
            Point::default(),
            Size::fixed(200., 100.),
        )],
    );
    let mut parent = Node::group(2, Point::new(50., 60.), vec![child]);
    parent.locked = true;
    p.pages[0].content.nodes = vec![parent];
    let mut e = Editor::new(p);
    assert!(e.enter(2));
    e.select(3, false);
    assert!(e.position_locked(3));
    assert!(!e.own_locked(3));
    let origin = e.nodes()[0].origin;
    e.crop_group(
        3,
        Some(Crop {
            x: 0.,
            y: 0.,
            width: 0.8,
            height: 0.8,
        }),
        Point::default(),
    )
    .unwrap();
    assert_eq!(e.nodes()[0].origin, origin);
    assert_eq!(e.group_crop_geometry(3).unwrap().1.w, 160.);
    let before = e.document.clone();
    e.move_by(10., 10.);
    e.delete();
    assert_eq!(e.document, before);
    e.select(3, false);
    e.crop_group(3, None, Point::default()).unwrap();
    assert_eq!(e.group_crop_geometry(3).unwrap().1.w, 200.);
    e.lock(true);
    let before = e.document.clone();
    assert!(e.crop_group(3, None, Point::default()).is_err());
    assert_eq!(e.document, before);
}
