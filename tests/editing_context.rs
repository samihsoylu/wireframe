// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, workspace::Editor};

#[test]
fn editing_emphasis_tracks_distance_from_each_unrelated_branch() {
    for instance in [false, true] {
        let button = |id, y| Node::control(id, "button", Point::new(0., y), Size::fixed(100., 40.));
        let inner = Node::group(3, Point::default(), vec![button(4, 0.)]);
        let middle = Node::group(2, Point::default(), vec![inner, button(5, 70.)]);
        let mut root = Node::group(1, Point::default(), vec![middle, button(6, 140.)]);
        let mut project = Project::default();
        if instance {
            project.definitions.push(Definition {
                id: 30,
                name: "Nested actions".into(),
                content: Content {
                    nodes: vec![root],
                    paint_order: vec![],
                },
            });
            root = Node::group(9, Point::default(), vec![]);
            root.content = NodeContent::Instance {
                definition: 30,
                overrides: vec![],
            };
        }
        project.pages[0].content.nodes = vec![root, button(7, 210.)];
        project.next_id = 31;
        let mut e = {
            project.organize_components().unwrap();
            Editor::new(project)
        };
        let before = e.document.clone();
        let canonical = e.rendered_scene(e.page).unwrap().svg();
        let mut scenes = vec![e.editing_scene().unwrap().svg()];
        let contexts: Vec<(Id, [f32; 4])> = if instance {
            vec![
                (9, [1., 1., 1., 0.4]),
                (1, [1., 1., 1., 0.2]),
                (2, [1., 1., 0.4, 0.4 / 3.]),
                (3, [1., 0.4, 0.2, 0.1]),
            ]
        } else {
            vec![
                (1, [1., 1., 1., 0.4]),
                (2, [1., 1., 0.4, 0.2]),
                (3, [1., 0.4, 0.2, 0.4 / 3.]),
            ]
        };
        for (id, expected) in contexts {
            assert!(e.enter(id));
            let layers = e.editing_layers().unwrap();
            for (leaf, opacity) in (4..=7).zip(expected) {
                assert_eq!(
                    layers
                        .iter()
                        .find(|l| l.control.id == leaf)
                        .unwrap()
                        .opacity,
                    opacity,
                    "instance={instance}, context={id}, leaf={leaf}"
                );
            }
            let scene = e.editing_scene().unwrap();
            assert!(scene.items.iter().any(|i| matches!(i.drawing, wireframe::render::Drawing::BeginOpacity(o) if o == expected[3])));
            scenes.push(scene.svg());
            assert_eq!(e.rendered_scene(e.page).unwrap().svg(), canonical);
        }
        scenes.pop();
        while let Some(scene) = scenes.pop() {
            assert!(e.leave());
            assert_eq!(e.editing_scene().unwrap().svg(), scene);
        }
        assert_eq!(e.document, before);
        assert!(!e.dirty());
        assert!(e.undo_name().is_none());
    }
}

#[test]
fn distant_unrelated_branches_keep_the_minimum_editing_emphasis() {
    let button = |id| Node::control(id, "button", Point::default(), Size::fixed(100., 40.));
    let mut active = button(100);
    let mut other = button(200);
    for id in (10..16).rev() {
        active = Node::group(id, Point::default(), vec![active]);
        other = Node::group(id + 20, Point::new(200., 0.), vec![other]);
    }
    let mut project = Project::default();
    project.pages[0].content.nodes = vec![active, other, button(300)];
    project.next_id = 301;
    let mut e = {
        project.organize_components().unwrap();
        Editor::new(project)
    };
    for (id, page_opacity) in (10..16).zip([0.4, 0.2, 0.4 / 3., 0.1, 0.1, 0.1]) {
        assert!(e.enter(id));
        let layers = e.editing_layers().unwrap();
        for (leaf, opacity) in [(100, 1.), (200, 0.1), (300, page_opacity)] {
            assert_eq!(
                layers
                    .iter()
                    .find(|l| l.control.id == leaf)
                    .unwrap()
                    .opacity,
                opacity
            );
        }
    }
}

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
    (
        {
            project.organize_components().unwrap();
            Editor::new(project)
        },
        id,
    )
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
    assert!(
        inner
            .items
            .iter()
            .any(|i| matches!(i.drawing, wireframe::render::Drawing::BeginOpacity(0.2)))
    );
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

#[test]
fn entering_empty_containers_hides_only_the_active_placeholder() {
    let empty = |id, x| Node::group(id, Point::new(x, 50.), vec![]);
    let mut project = Project {
        next_id: 100,
        ..Project::default()
    };
    project.definitions.push(Definition {
        id: 20,
        name: "Empty source".into(),
        content: Content::default(),
    });
    let mut instance = empty(30, 700.);
    instance.content = NodeContent::Instance {
        definition: 20,
        overrides: vec![],
    };
    project.pages[0].content.nodes = vec![
        empty(2, 50.),
        Node::group(3, Point::new(300., 0.), vec![empty(4, 0.)]),
        instance,
    ];
    let mut editor = {
        project.organize_components().unwrap();
        Editor::new(project)
    };
    let before = editor.document.clone();
    let canonical = editor.rendered_scene(editor.page).unwrap().svg();
    for path in [vec![2], vec![3, 4], vec![30]] {
        for id in &path {
            assert!(editor.enter(*id));
        }
        assert!(editor.current().controls.is_empty());
        let layers = editor.editing_layers().unwrap();
        let active = layers.iter().find(|l| l.path == path).unwrap();
        assert_eq!(active.opacity, 0.);
        assert!(
            layers
                .iter()
                .filter(|l| l.path != path)
                .all(|l| l.opacity > 0.)
        );
        let scene = editor.editing_scene().unwrap();
        let active_bounds = active.control.bounds;
        let image = image::load_from_memory(&scene.png().unwrap())
            .unwrap()
            .into_rgba8();
        let x = (active_bounds.x - scene.bounds.x + 24. + 4.).round() as u32;
        let y = (active_bounds.y - scene.bounds.y + 24. + 4.).round() as u32;
        assert_eq!(image.get_pixel(x, y).0, [255; 4]);
        assert_eq!(editor.rendered_scene(editor.page).unwrap().svg(), canonical);
        while !editor.context.is_empty() {
            assert!(editor.leave());
        }
        assert_eq!(editor.editing_scene().unwrap().svg(), canonical);
        assert_eq!(editor.document, before);
        assert!(editor.undo_name().is_none());
    }
}

#[test]
fn empty_context_visibility_tracks_insert_delete_undo_and_redo() {
    let mut project = Project {
        next_id: 100,
        ..Project::default()
    };
    project.pages[0].content.nodes = vec![Node::group(2, Point::new(80., 90.), vec![])];
    let mut editor = {
        project.organize_components().unwrap();
        Editor::new(project)
    };
    assert!(editor.enter(2));
    let hidden = editor.editing_scene().unwrap().svg();
    assert_eq!(editor.editing_layers().unwrap()[0].opacity, 0.);
    editor.add(wireframe::model::Kind::Button, 0., 0.);
    let visible = editor.editing_scene().unwrap().svg();
    assert_ne!(visible, hidden);
    assert_eq!(editor.editing_layers().unwrap()[0].opacity, 1.);
    editor.undo();
    assert_eq!(editor.editing_scene().unwrap().svg(), hidden);
    editor.redo();
    assert_eq!(editor.editing_scene().unwrap().svg(), visible);
    editor.select_all();
    editor.delete();
    assert_eq!(editor.editing_scene().unwrap().svg(), hidden);
    editor.undo();
    assert_eq!(editor.editing_scene().unwrap().svg(), visible);
    editor.redo();
    assert_eq!(editor.editing_scene().unwrap().svg(), hidden);
    assert!(editor.leave());
    assert_eq!(editor.editing_layers().unwrap()[0].opacity, 1.);
    let reopened = Editor::new(decode(&editor.document.encode().unwrap()).unwrap());
    assert_eq!(
        reopened.editing_scene().unwrap().svg(),
        editor.editing_scene().unwrap().svg()
    );
}
