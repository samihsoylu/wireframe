// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
mod support;
use wireframe::{
    catalog,
    document::{self, Alternate, Content, Link, Node, NodeContent, Point, Project, Value},
    model::Kind,
    render::Drawing,
    scene::Primitive,
    workspace::Editor,
};

fn state(e: &Editor, id: u64) -> Value {
    e.control_properties(id).unwrap().1["state"].clone()
}
fn choice(s: &str) -> Value {
    Value::Choice(s.into())
}

#[test]
fn newly_inserted_switch_is_on_intrinsic_and_cannot_be_resized() {
    for sketch in [true, false] {
        let mut e = Editor::default();
        e.document.settings.sketch = sketch;
        e.add(Kind::Toggle, 20., 30.);
        let id = *e.selection.iter().next().unwrap();
        assert_eq!(state(&e, id), choice("on"));
        assert_eq!(
            e.control_properties(id).unwrap().1["fill"],
            Value::Color([106, 168, 79, 255])
        );
        let b = e.selected_views()[0].bounds;
        assert_eq!((b.x, b.y, b.w, b.h), (20., 30., 42., 24.));
        assert!(e.layout_page(e.page).unwrap().warnings.is_empty());
        assert_eq!(
            catalog::get("toggle")
                .unwrap()
                .resized((42., 24.), (500., 700.)),
            (42., 24.)
        );
        support::stored_size(&mut e, 90., 60.);
        let saved = e.document.encode().unwrap();
        let mut reopened = Editor::new(document::decode(&saved).unwrap());
        reopened.select(id, false);
        let b = reopened.selected_views()[0].bounds;
        assert_eq!((b.w, b.h), (42., 24.));
        assert_eq!(
            reopened.document.encode().unwrap(),
            saved,
            "Layout must not rewrite saved geometry"
        );
    }
}

#[test]
fn switch_edits_reset_history_links_and_storage_agree() {
    let mut e = Editor::default();
    e.add(Kind::Toggle, 0., 0.);
    let id = *e.selection.iter().next().unwrap();
    let before = e.document.clone();
    e.begin("Edit switch");
    e.set_control_property(id, "state", choice("off")).unwrap();
    e.set_control_property(id, "fill", Value::Color([32, 64, 128, 255]))
        .unwrap();
    e.set_control_property(id, "link", Value::Link(Link::Back))
        .unwrap();
    e.commit();
    let after = e.document.clone();
    assert_eq!(state(&e, id), choice("off"));
    assert_eq!(e.rendered_scene(e.page).unwrap().links[0].target, "back:");
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(
        document::decode(&e.document.encode().unwrap()).unwrap(),
        after
    );
    e.reset_control_property(id, "state").unwrap();
    assert_eq!(state(&e, id), choice("on"));
}

#[test]
fn switch_states_have_correct_color_thumb_side_and_export_geometry() {
    for sketch in [true, false] {
        for on in [true, false] {
            for color in [[106, 168, 79], [32, 64, 128]] {
                let mut e = Editor::default();
                e.document.settings.sketch = sketch;
                e.add(Kind::Toggle, 10., 20.);
                let id = *e.selection.iter().next().unwrap();
                e.set_control_property(id, "state", choice(if on { "on" } else { "off" }))
                    .unwrap();
                e.set_control_property(
                    id,
                    "fill",
                    Value::Color([color[0], color[1], color[2], 255]),
                )
                .unwrap();
                let scene = e.rendered_scene(e.page).unwrap();
                let polygons: Vec<_> = scene
                    .items
                    .iter()
                    .filter_map(|i| match &i.drawing {
                        Drawing::Primitive(Primitive::Polygon { points, fill }) => {
                            Some((points, *fill))
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(
                    polygons[1].1,
                    if on {
                        color
                    } else if sketch {
                        [226; 3]
                    } else {
                        [255; 3]
                    }
                );
                assert_eq!(polygons[3].1, [255; 3]);
                let thumb_x =
                    polygons[3].0.iter().map(|p| p[0]).sum::<f32>() / polygons[3].0.len() as f32;
                assert!((thumb_x - if on { 40. } else { 22. }).abs() < 0.3);
                for (points, _) in &polygons {
                    assert!(points.iter().all(|p| (10.0..=52.0).contains(&p[0]) && (20.0..=44.0).contains(&p[1])));
                }
                let mut png = vec![];
                wireframe::raster::write_png(&scene, Default::default(), &mut png, |_, _| Ok(()))
                    .unwrap();
                assert!(png.starts_with(b"\x89PNG"));
                assert!(scene.svg().contains("<svg"));
            }
        }
    }
}

#[test]
fn older_sparse_switches_stay_off_and_explicit_states_keep_precedence() {
    for version in 2..12 {
        for (properties, expected) in [
            (vec![], "off"),
            (vec![("checked", Value::Bool(true))], "on"),
            (vec![("checked", Value::Bool(false))], "off"),
            (vec![("state", choice("on"))], "on"),
            (
                vec![("checked", Value::Bool(true)), ("state", choice("off"))],
                "off",
            ),
        ] {
            let mut p = Project {
                next_id: 101,
                ..Project::default()
            };
            let mut node = catalog::get("toggle")
                .unwrap()
                .create(100, Point::default());
            node.properties = properties.into_iter().map(|(k, v)| (k.into(), v)).collect();
            p.pages[0].content.nodes.push(node);
            let mut value = serde_json::to_value(&p).unwrap();
            value["format_version"] = version.into();
            let saved = serde_json::to_vec(&value).unwrap();
            let mut e = Editor::new(document::decode(&saved).unwrap());
            assert_eq!(state(&e, 100), choice(expected), "version {version}");
            let migrated = e.document.encode().unwrap();
            assert_eq!(
                document::decode(&migrated).unwrap().encode().unwrap(),
                migrated
            );
            e.reset_control_property(100, "state").unwrap();
            assert_eq!(state(&e, 100), choice("on"));
        }
    }
}

#[test]
fn migration_reaches_grouped_alternate_retained_and_component_content() {
    let mut e = Editor::default();
    e.add(Kind::Toggle, 0., 0.);
    let instance = *e.selection.iter().next().unwrap();
    e.create_component("Switch".into()).unwrap();
    let source = e.document.definitions[0].content.nodes[0].id;
    assert!(e.enter(instance));
    e.set_control_property(source, "checked", Value::Bool(true))
        .unwrap();
    e.switch_page(1);
    let raw = catalog::get("toggle")
        .unwrap()
        .create(100, Point::default());
    let mut group = Node::control(101, "toggle", Point::default(), Default::default());
    group.content = NodeContent::Group {
        children: vec![raw],
    };
    e.document.pages[0].alternates.push(Alternate {
        id: 102,
        name: "Retained".into(),
        version: None,
        notes: String::new(),
        content: Content {
            nodes: vec![group],
            ..Content::default()
        },
    });
    e.document.pages[0].trashed = true;
    e.document.next_id = 103;
    e.add_page();
    e.switch_page(1);
    let mut saved = serde_json::to_value(&e.document).unwrap();
    saved["format_version"] = 11.into();
    let migrated = document::decode(&serde_json::to_vec(&saved).unwrap()).unwrap();
    assert_eq!(
        migrated.definitions[0].content.nodes[0].properties["checked"],
        Value::Bool(false)
    );
    let NodeContent::Group { children } = &migrated.pages[0].alternates[0].content.nodes[0].content
    else {
        panic!()
    };
    assert_eq!(children[0].properties["checked"], Value::Bool(false));
    let mut reopened = Editor::new(migrated);
    reopened.document.pages[0].trashed = false;
    reopened.switch_page(1);
    assert!(
        reopened
            .layers(1)
            .unwrap()
            .iter()
            .any(|l| l.properties.get("state") == Some(&choice("on"))),
        "Instance checked override must survive migration"
    );
    assert!(reopened.enter(instance));
    reopened.reset_control_property(source, "checked").unwrap();
    assert_eq!(
        state(&reopened, source),
        choice("off"),
        "Reset inherits the migrated source"
    );
}
