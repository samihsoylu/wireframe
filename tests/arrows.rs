// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    arrows::{self, Arrow, Handle, Mode},
    catalog,
    document::{self, Point, Value},
    model::Kind,
    render::Drawing,
    scene::Primitive,
    workspace::Editor,
};
fn choice(s: &str) -> Value {
    Value::Choice(s.into())
}
fn fixture() -> (Editor, u64) {
    let mut e = Editor::default();
    e.add(Kind::Arrow, 100., 150.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
fn geometry(e: &Editor, id: u64) -> Arrow {
    arrows::node_geometry(e.nodes().iter().find(|n| n.id == id).unwrap())
        .unwrap()
        .0
}
fn close(a: Point, b: Point) {
    assert!(
        (a.x - b.x).abs() < 0.001 && (a.y - b.y).abs() < 0.001,
        "{a:?} / {b:?}"
    );
}

#[test]
fn arrow_insertion_defaults_and_caption_position_are_functional() {
    let (mut e, id) = fixture();
    let node = &e.nodes()[0];
    assert!(node.markup);
    assert_eq!(geometry(&e, id), Arrow::straight(150., 100.));
    assert!(e.layout_page(e.page).unwrap().warnings.is_empty());
    let p = e.control_properties(id).unwrap().1;
    assert_eq!(p["start_head"], Value::Bool(true));
    assert_eq!(p["end_head"], Value::Bool(true));
    e.set_control_property(id, "end_head", Value::Bool(false))
        .unwrap();
    assert_eq!(
        e.control_properties(id).unwrap().1["label_position"],
        Value::Number(8.)
    );
    e.set_control_property(id, "label_position", Value::Number(5.))
        .unwrap();
    assert_eq!(
        e.control_properties(id).unwrap().1["label_position"],
        Value::Number(5.)
    );
    e.reset_control_property(id, "label_position").unwrap();
    assert_eq!(
        e.control_properties(id).unwrap().1["label_position"],
        Value::Number(8.)
    );
    assert_eq!(
        document::decode(&e.document.encode().unwrap()).unwrap(),
        e.document
    );
}
#[test]
fn handle_drag_mode_change_resize_and_cancel_preserve_history_and_points() {
    let (mut e, id) = fixture();
    let original = e.document.clone();
    e.begin("Bend arrow");
    e.move_arrow_handle(id, Handle::Middle, Point::new(90., -60.), false)
        .unwrap();
    e.reset_preview();
    e.move_arrow_handle(id, Handle::Middle, Point::new(40., 120.), false)
        .unwrap();
    e.commit();
    let bent = e.document.clone();
    e.undo();
    assert_eq!(e.document, original);
    e.redo();
    assert_eq!(e.document, bent);
    let node = e.nodes()[0].clone();
    let a = geometry(&e, id);
    e.begin("Change mode");
    e.set_control_property(id, "shape", choice("lines"))
        .unwrap();
    e.commit();
    let n = e.nodes()[0].clone();
    let line = geometry(&e, id);
    close(
        a.start.translated(node.origin),
        line.start.translated(n.origin),
    );
    close(a.end.translated(node.origin), line.end.translated(n.origin));
    close(
        a.middle().translated(node.origin),
        line.middle().translated(n.origin),
    );
    let before = e.document.clone();
    let mut c = e.selected_views()[0].clone();
    let old = c.bounds;
    c.bounds.w *= 2.;
    c.bounds.h *= 0.5;
    let expected = line.resize(Point::new(old.w, old.h), c.bounds);
    e.begin("Resize arrow");
    e.update_control(c);
    e.commit();
    assert_eq!(geometry(&e, id), expected.arrow);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    let before = e.document.clone();
    e.begin("Cancelled drag");
    e.move_arrow_handle(id, Handle::End, Point::new(-80., -30.), true)
        .unwrap();
    e.reset_preview();
    e.commit();
    assert_eq!(e.document, before);
    assert_eq!(
        document::decode(&e.document.encode().unwrap()).unwrap(),
        e.document
    );
}
#[test]
fn component_arrow_edits_and_group_resize_preserve_the_source() {
    let (mut e, root) = fixture();
    let source = e.create_component("Connector".into()).unwrap();
    let definitions = e.document.definitions.clone();
    assert!(e.enter(root));
    let id = e.nodes()[0].id;
    e.begin("Local bend");
    e.move_arrow_handle(id, Handle::Middle, Point::new(60., -40.), false)
        .unwrap();
    e.set_control_property(id, "shape", choice("lines"))
        .unwrap();
    e.commit();
    assert_eq!(geometry(&e, id).mode, Mode::Lines);
    assert_eq!(e.document.definitions, definitions);
    e.reset_control_property(id, "shape").unwrap();
    assert_eq!(geometry(&e, id).mode, Mode::Curve);
    e.leave();
    e.insert_component(source, 400., 150.).unwrap();
    e.select_all();
    e.group();
    let mut group = e.selected_views()[0].clone();
    group.bounds.w *= 1.5;
    group.bounds.h *= 2.;
    e.begin("Resize group");
    e.update_control(group);
    e.commit();
    assert_eq!(e.document.definitions, definitions);
    let layers = e.layers(e.page).unwrap();
    assert_eq!(layers.len(), 2);
    for layer in layers {
        let a = Arrow::from_properties(
            &layer.properties,
            layer.control.bounds.w,
            layer.control.bounds.h,
        );
        assert!(a.start.x.is_finite() && a.middle().y.is_finite());
    }
    assert_eq!(
        document::decode(&e.document.encode().unwrap()).unwrap(),
        e.document
    );
}
#[test]
fn arrow_styles_heads_caption_links_and_overflow_reach_shared_exports() {
    for sketch in [true, false] {
        let (mut e, id) = fixture();
        e.document.settings.sketch = sketch;
        e.set_control_property(
            id,
            "text",
            Value::Text("[Continue](https://example.com)".into()),
        )
        .unwrap();
        e.set_control_property(id, "border_color", Value::Color([120, 40, 180, 255]))
            .unwrap();
        e.set_control_property(id, "label_position", Value::Number(0.))
            .unwrap();
        e.move_arrow_handle(id, Handle::Middle, Point::new(60., -90.), false)
            .unwrap();
        let mut outputs = std::collections::HashSet::new();
        for stroke in ["solid", "dashed", "dotted"] {
            e.set_control_property(id, "stroke", choice(stroke))
                .unwrap();
            let scene = e.rendered_scene(e.page).unwrap();
            let frame = e.selected_views()[0].bounds;
            assert!(
                scene.bounds.x < frame.x && scene.bounds.y < frame.y,
                "caption and head overflow must be exported"
            );
            assert_eq!(scene.links.len(), 1);
            assert_eq!(scene.links[0].target, "https://example.com");
            assert!(scene.items.iter().any(|i|matches!(&i.drawing,Drawing::Primitive(Primitive::Mesh{fill,..}) if *fill==[120,40,180])));
            outputs.insert(scene.svg());
            let mut png = vec![];
            wireframe::raster::write_png(&scene, Default::default(), &mut png, |_, _| Ok(()))
                .unwrap();
            assert!(png.starts_with(b"\x89PNG"));
        }
        assert_eq!(outputs.len(), 3);
        let before = e.rendered_scene(e.page).unwrap().svg();
        e.set_control_property(id, "start_head", Value::Bool(false))
            .unwrap();
        e.set_control_property(id, "end_head", Value::Bool(false))
            .unwrap();
        assert_ne!(before, e.rendered_scene(e.page).unwrap().svg());
    }
}
#[test]
fn transparent_arrow_keeps_only_caption_visible_in_shared_outputs() {
    for sketch in [true, false] {
        let (mut e, id) = fixture();
        e.document.settings.sketch = sketch;
        e.set_control_property(id, "text", Value::Text("Continue".into()))
            .unwrap();
        for alpha in [0., 0.5, 1.] {
            e.set_control_property(id, "background_opacity", Value::Number(alpha))
                .unwrap();
            let scene = e.rendered_scene(e.page).unwrap();
            let mut opacity = vec![1.];
            let mut glyphs = 0;
            for item in &scene.items {
                match &item.drawing {
                    Drawing::BeginOpacity(value) => {
                        assert_eq!(opacity.len(), 1, "arrow parts do not nest GPU scopes");
                        opacity.push(opacity.last().unwrap() * value);
                    }
                    Drawing::EndOpacity => {
                        assert!(opacity.len() > 1);
                        opacity.pop();
                    }
                    Drawing::Glyph(_) => {
                        glyphs += 1;
                        assert_eq!(item.opacity * opacity.last().unwrap(), 1.);
                    }
                    Drawing::Primitive(_) if alpha == 0. => {
                        assert_eq!(item.opacity * opacity.last().unwrap(), 0.);
                    }
                    _ => {}
                }
            }
            assert_eq!(opacity, [1.]);
            assert!(glyphs > 0);
            let mut png = Vec::new();
            wireframe::raster::write_png(&scene, Default::default(), &mut png, |_, _| Ok(()))
                .unwrap();
            let pixels = image::load_from_memory(&png).unwrap().into_rgba8();
            assert!(pixels.pixels().any(|p| p.0[0] < 200));
            let page = wireframe::pdf::Page {
                scene,
                decoration: None,
                size: [400., 400.],
                origin: [0., 0.],
                scale: 1.,
                links: vec![],
            };
            assert!(wireframe::pdf::encode("Arrow caption", &[page]).is_ok());
        }
        e.set_control_property(id, "text", Value::Text(String::new()))
            .unwrap();
        e.set_control_property(id, "background_opacity", Value::Number(0.))
            .unwrap();
        let scene = e.rendered_scene(e.page).unwrap();
        let mut png = Vec::new();
        wireframe::raster::write_png(&scene, Default::default(), &mut png, |_, _| Ok(())).unwrap();
        let pixels = image::load_from_memory(&png).unwrap().into_rgba8();
        assert!(pixels.pixels().all(|p| p.0 == [255; 4]));
    }
}
#[test]
fn dotted_arrow_marks_keep_style_width_and_round_caps() {
    for sketch in [true, false] {
        let (mut e, id) = fixture();
        e.document.settings.sketch = sketch;
        e.set_control_property(id, "stroke", choice("dotted"))
            .unwrap();
        // Isolate the dotted body; arrowheads also use filled meshes.
        for key in ["start_head", "end_head"] {
            e.set_control_property(id, key, Value::Bool(false)).unwrap();
        }
        let scene = e.rendered_scene(e.page).unwrap();
        let thickness = if sketch { 4. } else { 2. };
        let mut dots = 0;
        for item in scene.items {
            if let Drawing::Primitive(Primitive::Mesh { points, .. }) = item.drawing {
                dots += 1;
                for axis in 0..2 {
                    let lo = points.iter().map(|p| p[axis]).fold(f32::INFINITY, f32::min);
                    let hi = points
                        .iter()
                        .map(|p| p[axis])
                        .fold(f32::NEG_INFINITY, f32::max);
                    assert!(hi - lo >= thickness * 0.99);
                    assert!(hi - lo <= thickness + 1.01);
                }
            }
        }
        assert!(dots >= 20, "one-unit marks with eight-unit gaps");
    }
}
#[test]
fn older_arrows_keep_horizontal_geometry_and_component_size_defaults() {
    let (mut e, id) = fixture();
    e.document.format_version = 12;
    e.document.pages[0].content.nodes[0].markup = false;
    let bytes = serde_json::to_vec_pretty(&e.document).unwrap();
    let project = document::decode(&bytes).unwrap();
    assert_eq!(project.format_version, document::FORMAT_VERSION);
    assert!(!project.pages[0].content.nodes[0].markup);
    let mut old = Editor::new(project);
    old.select(id, false);
    let a = geometry(&old, id);
    close(a.start, Point::new(0., 50.));
    close(a.end, Point::new(150., 50.));
    let values = old.control_properties(id).unwrap().1;
    assert_eq!(values["start_head"], Value::Bool(false));
    assert_eq!(values["border_color"], Value::Color([44, 48, 55, 255]));
    let mut node = old.nodes()[0].clone();
    if let document::NodeContent::Control { size, .. } = &mut node.content {
        *size = document::Size::fixed(300., 60.);
    }
    let a = arrows::node_geometry(&node).unwrap().0;
    close(a.start, Point::new(0., 30.));
    close(a.end, Point::new(300., 30.));
    let migrated = old.document.encode().unwrap();
    assert_eq!(
        document::decode(&migrated).unwrap().encode().unwrap(),
        migrated
    );
    assert_eq!(
        catalog::get("arrow").unwrap().minimum_resize_dimension(),
        1.
    );
}

#[test]
fn arrow_hit_testing_uses_the_visible_path_with_fifteen_unit_tolerance() {
    let (mut e, id) = fixture();
    assert_eq!(e.hit(175., 200.).unwrap().id, id);
    assert!(
        e.hit(110., 240.).is_none(),
        "empty corners of the arrow frame are not the arrow"
    );
    e.move_arrow_handle(id, Handle::Middle, Point::new(75., -90.), false)
        .unwrap();
    for mode in ["curve", "lines"] {
        e.set_control_property(id, "shape", choice(mode)).unwrap();
        let layer = e.layers(e.page).unwrap().remove(0);
        let b = layer.control.bounds;
        let a = Arrow::from_properties(&layer.properties, b.w, b.h);
        let offset = a.display_offset(&layer.properties);
        for t in [0., 0.25, 0.5, 0.75, 1.] {
            let p = a.at(t);
            assert_eq!(
                e.hit(b.x + p.x - offset.x, b.y + p.y - offset.y)
                    .unwrap()
                    .id,
                id
            );
        }
    }
}

#[test]
fn faded_heads_overlap_the_line_without_fading_their_own_joint_twice() {
    for sketch in [true, false] {
        let (mut e, id) = fixture();
        e.document.settings.sketch = sketch;
        e.set_control_property(id, "background_opacity", Value::Number(0.5))
            .unwrap();
        e.set_control_property(id, "start_head", Value::Bool(false))
            .unwrap();
        e.set_control_property(id, "end_head", Value::Bool(false))
            .unwrap();
        let sample = |e: &Editor| {
            let mut scene = e.rendered_scene(e.page).unwrap();
            scene.bounds = wireframe::model::Bounds::new(0., 0., 400., 400.);
            let png = wireframe::scene::png_svg(&scene.svg()).unwrap();
            let image = image::load_from_memory(&png).unwrap().into_rgba8();
            // Arrow endpoint in document coordinates, plus the SVG export margin.
            image.get_pixel(124, 174).0
        };
        let line = sample(&e);
        let expected_line = if sketch { 128_u8 } else { 153 };
        assert!(line[0].abs_diff(expected_line) <= 2, "{line:?}");
        e.set_control_property(id, "start_head", Value::Bool(true))
            .unwrap();
        let overlap = sample(&e);
        let expected_overlap = if sketch { 64_u8 } else { 102 };
        assert!(overlap[0].abs_diff(expected_overlap) <= 2, "{overlap:?}");
        assert_eq!(overlap[3], 255);
    }
}
