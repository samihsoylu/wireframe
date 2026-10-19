// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
mod support;
use wireframe::{
    catalog,
    document::{self, Node, NodeContent, Point, Project, Size, Value},
    model::{Bounds, Kind},
    render::{Drawing, Item},
    scene::Primitive,
    workspace::Editor,
};
const STYLES: [&str; 5] = [
    "solid",
    "dashed",
    "dotted",
    "breakline_top",
    "breakline_bottom",
];

#[test]
fn rounded_rule_marks_keep_caps_and_stems_at_one_opacity() {
    for kind in [Kind::Line, Kind::VerticalRule] {
        for sketch in [false, true] {
            for style in ["dashed", "dotted"] {
                let mut e = Editor::default();
                e.document.settings.sketch = sketch;
                e.add(kind, 20., 30.);
                let id = *e.selection.iter().next().unwrap();
                e.set_control_property(id, "stroke", Value::Choice(style.into()))
                    .unwrap();
                e.set_control_property(id, "border_color", Value::Color([0, 0, 0, 255]))
                    .unwrap();
                e.set_control_property(id, "background_opacity", Value::Number(0.5))
                    .unwrap();
                let scene = e.rendered_scene(e.page).unwrap();
                // Every mark must be a single native filled surface, rather than
                // a square-ended canvas line or overlapping translucent pieces.
                for item in &scene.items {
                    assert_eq!(item.opacity, 0.5);
                    let Drawing::Primitive(Primitive::Mesh {
                        points, indices, ..
                    }) = &item.drawing
                    else {
                        panic!("{kind:?} {style} requires one round filled surface per mark");
                    };
                    assert!(indices.iter().all(|i| (*i as usize) < points.len()));
                    assert!(points.iter().flatten().all(|v| v.is_finite()));
                }
                let image =
                    image::load_from_memory(&wireframe::scene::png_svg(&scene.svg()).unwrap())
                        .unwrap()
                        .into_rgba8();
                let ink: Vec<_> = image.pixels().filter(|p| p[0] < 200 && p[3] > 0).collect();
                assert!(!ink.is_empty());
                // Half-opacity black on white cannot be darker than 127. Dark
                // overlaps here would reveal cap/stem or triangle-edge seams.
                assert!(
                    ink.iter().all(|p| p[0] >= 126),
                    "{kind:?} {style} opacity seam"
                );
            }
        }
    }
}
fn fixture() -> (Editor, u64) {
    let mut editor = Editor::default();
    editor.add(Kind::Line, 20., 30.);
    let id = *editor.selection.iter().next().unwrap();
    (editor, id)
}
fn set_style(e: &mut Editor, id: u64, style: &str) {
    e.set_control_property(id, "stroke", Value::Choice(style.into()))
        .unwrap();
}

#[test]
fn rules_insert_at_fixed_height_and_style_resize_has_one_step_history() {
    let (mut e, id) = fixture();
    assert_eq!(e.control_properties(id).unwrap().0.id, "horizontal_rule");
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(20., 30., 100., 10.)
    );
    for style in STYLES {
        let original = e.document.clone();
        e.begin("Style and resize rule");
        set_style(&mut e, id, style);
        let height = if style.starts_with("breakline") {
            38.
        } else {
            10.
        };
        let axis = e.linear_axis(id).unwrap();
        assert!(!axis.vertical);
        assert_eq!(axis.thickness, height);
        let mut c = e.selected_views()[0].clone();
        assert_eq!(c.bounds.h, height);
        c.bounds = axis.resize(c.bounds, -1, -25.);
        e.update_control(c);
        e.commit();
        let changed = e.document.clone();
        let b = e.selected_views()[0].bounds;
        assert_eq!(b.h, height);
        assert_eq!(b.x + b.w, 120.);
        assert_eq!(e.resize_minimum(&[id]), (10., height));
        assert_eq!(
            document::decode(&changed.encode().unwrap()).unwrap(),
            changed
        );
        e.undo();
        assert_eq!(e.document, original);
        e.redo();
        assert_eq!(e.document, changed);
    }
}

#[test]
fn all_styles_export_and_breakline_white_remains_opaque_with_zero_border_opacity() {
    for sketch in [true, false] {
        let (mut e, id) = fixture();
        e.document.settings.sketch = sketch;
        let mut output = std::collections::HashSet::new();
        for style in STYLES {
            set_style(&mut e, id, style);
            e.set_control_property(id, "border_color", Value::Color([120, 40, 180, 255]))
                .unwrap();
            e.set_control_property(id, "background_opacity", Value::Number(0.4))
                .unwrap();
            let scene = e.rendered_scene(e.page).unwrap();
            assert!(!scene.items.is_empty());
            assert!(scene.items.iter().any(|i| i.opacity == 0.4));
            output.insert(scene.svg());
            let mut png = vec![];
            wireframe::raster::write_png(&scene, Default::default(), &mut png, |_, _| Ok(()))
                .unwrap();
            assert!(png.starts_with(b"\x89PNG"));
            let pdf = wireframe::pdf::encode(
                "Rules",
                &[wireframe::pdf::Page {
                    scene,
                    decoration: None,
                    size: [200., 100.],
                    origin: [0., 0.],
                    scale: 1.,
                    links: vec![],
                }],
            )
            .unwrap();
            assert!(pdf.starts_with(b"%PDF"));
        }
        assert_eq!(output.len(), 5);
        for (style, white_y, color_y) in [("breakline_top", 2, 36), ("breakline_bottom", 36, 2)] {
            set_style(&mut e, id, style);
            e.set_control_property(id, "background_opacity", Value::Number(0.))
                .unwrap();
            for width in [64., 65., 180.] {
                support::stored_size(&mut e, width, 80.);
                let b = e.selected_views()[0].bounds;
                assert_eq!(b.h, 38.);
                let mut scene = e.rendered_scene(e.page).unwrap();
                assert_eq!(scene.items[0].opacity, 1.);
                assert!(scene.items.iter().skip(1).all(|i| i.opacity == 0.));
                scene.items.insert(
                    0,
                    Item {
                        drawing: Drawing::Primitive(Primitive::Rect {
                            b,
                            fill: [20, 100, 160],
                            stroke: false,
                            radius: 0.,
                        }),
                        opacity: 1.,
                        clip: None,
                    },
                );
                scene.bounds = b;
                let image =
                    image::load_from_memory(&wireframe::scene::png_svg(&scene.svg()).unwrap())
                        .unwrap()
                        .into_rgba8();
                assert_eq!(
                    image.get_pixel(24 + width as u32 / 2, 24 + white_y).0,
                    [255; 4]
                );
                assert_eq!(
                    image.get_pixel(24 + width as u32 / 2, 24 + color_y).0,
                    [20, 100, 160, 255]
                );
            }
        }
    }
}

#[test]
fn breakline_transition_retains_finite_complete_white_geometry() {
    for width in [1., 10., 64., 65., 100., 10000.] {
        let edge = wireframe::rules::edge(width, 19., 1.);
        assert_eq!(edge.len() == 2, width < 65.);
        for above in [true, false] {
            let p = wireframe::rules::white_region(width, 38., above);
            assert!(p.iter().all(|p| p.iter().all(|v| v.is_finite())));
            let triangles = wireframe::popover::triangles(&p);
            let area: f32 = triangles
                .as_chunks::<3>()
                .0
                .iter()
                .map(|i| {
                    let [a, b, c] = i.map(|j| p[j as usize]);
                    ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() / 2.
                })
                .sum();
            let polygon = p
                .iter()
                .zip(p.iter().cycle().skip(1))
                .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
                .sum::<f32>()
                .abs()
                / 2.;
            assert!(
                area > 0. && (area - polygon).abs() < 0.1,
                "{width} {above}: {area}/{polygon}"
            );
        }
    }
}

#[test]
fn old_lines_preserve_frame_center_and_ink_and_allow_reversible_new_styles() {
    for kind in ["line", "horizontal_rule"] {
        let mut p = Project {
            format_version: 13,
            next_id: 100,
            ..Project::default()
        };
        p.pages[0].content.nodes.push(Node::control(
            2,
            kind,
            Point::new(20., 30.),
            Size::fixed(180., 60.),
        ));
        let bytes = serde_json::to_vec(&p).unwrap();
        let p = document::decode(&bytes).unwrap();
        assert_eq!(p.format_version, document::FORMAT_VERSION);
        let mut e = Editor::new(p);
        e.select(2, false);
        assert_eq!(
            e.selected_views()[0].bounds,
            Bounds::new(20., 30., 180., 60.)
        );
        assert!(e.linear_axis(2).is_none());
        let scene = e.rendered_scene(e.page).unwrap();
        assert!(
            matches!(&scene.items[0].drawing, Drawing::Primitive(Primitive::Line {points,color,width})
            if points == &vec![[20.,60.],[200.,60.]] && *color == [44,48,55] && *width == 1.5)
        );
        set_style(&mut e, 2, "breakline_top");
        assert_eq!(e.selected_views()[0].bounds.h, 38.);
        assert_eq!(e.linear_axis(2).unwrap().thickness, 38.);
        set_style(&mut e, 2, "solid");
        assert_eq!(e.selected_views()[0].bounds.h, 60.);
        let saved = e.document.encode().unwrap();
        assert_eq!(document::decode(&saved).unwrap().encode().unwrap(), saved);
    }
}

#[test]
fn component_and_group_resizing_keep_rule_heights_and_source_definitions() {
    let (mut e, root) = fixture();
    let source = e.create_component("Separator".into()).unwrap();
    let definitions = e.document.definitions.clone();
    assert!(e.enter(root));
    let child = e.nodes()[0].id;
    set_style(&mut e, child, "breakline_bottom");
    assert_eq!(e.linear_axis(child).unwrap().thickness, 38.);
    e.leave();
    e.insert_component(source, 20., 150.).unwrap();
    e.select_all();
    e.group();
    let before = e.document.clone();
    let mut group = e.selected_views()[0].clone();
    assert!(e.linear_axis(group.id).is_none());
    group.bounds.w *= 2.;
    group.bounds.h *= 2.;
    e.begin("Resize separator group");
    e.update_control(group);
    e.commit();
    let after = e.document.clone();
    assert_eq!(e.document.definitions, definitions);
    let layers = e.layers(e.page).unwrap();
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].control.bounds.h, 38.);
    assert_eq!(layers[1].control.bounds.h, 10.);
    assert!(layers.iter().all(|l| l.control.bounds.w == 200.));
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(document::decode(&after.encode().unwrap()).unwrap(), after);
}

#[test]
fn rule_migration_reaches_nested_retained_and_component_content() {
    let (mut e, root) = fixture();
    e.create_component("Separator".into()).unwrap();
    // Keep the workspace valid while preparing a retained-page migration fixture.
    let retained_page = e.document.pages[0].id;
    e.add_page();
    e.trash_pages(&[retained_page]).unwrap();
    e.document.pages[0].alternates.push(document::Alternate {
        id: 900,
        name: "Earlier".into(),
        notes: String::new(),
        version: None,
        content: document::Content {
            nodes: vec![Node::group(
                901,
                Point::default(),
                vec![Node::control(
                    902,
                    "line",
                    Point::default(),
                    Size::fixed(70., 25.),
                )],
            )],
            paint_order: vec![],
        },
    });
    e.document.libraries[0].content.nodes.push(Node::control(
        903,
        "line",
        Point::default(),
        Size::fixed(90., 35.),
    ));
    e.document.next_id = 1000;
    e.document.format_version = 13;
    let old = serde_json::to_vec(&e.document).unwrap();
    let p = document::decode(&old).unwrap();
    assert_eq!(p.pages[0].content.nodes[0].id, root);
    let NodeContent::Group { children } = &p.pages[0].alternates[0].content.nodes[0].content else {
        panic!()
    };
    for node in [
        &children[0],
        &p.definitions[0].content.nodes[0],
        p.libraries[0].content.nodes.last().unwrap(),
    ] {
        assert_eq!(node.properties["legacy_rule_frame"], Value::Bool(true));
        assert_eq!(
            node.properties["border_color"],
            Value::Color([44, 48, 55, 255])
        );
    }
    assert_eq!(
        catalog::get("horizontal_rule").unwrap().id,
        "horizontal_rule"
    );
}
