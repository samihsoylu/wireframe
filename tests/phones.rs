// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{Properties, Value},
    layout::TextMetrics,
    phone::{Model, Options, Pattern, StatusKind},
};
fn metrics(size: f32) -> (TextMetrics, TextMetrics) {
    (
        TextMetrics {
            width: 1.5 * size,
            height: size,
            rows: 1,
        },
        TextMetrics {
            width: 4. * size,
            height: size,
            rows: 1,
        },
    )
}
#[test]
fn device_fit_stays_centered_and_orientation_preserves_native_ratio() {
    for model in [Model::Classic, Model::Tall, Model::Wide, Model::Notched] {
        for landscape in [false, true] {
            let o = Options {
                model,
                landscape,
                ..Options::default()
            };
            let (nw, nh) = o.native();
            for (w, h) in [(300., 500.), (500., 300.), (1., 1.), (10000., 20.)] {
                let b = o.frame(w, h);
                assert!(b.w <= w + 0.001 && b.h <= h + 0.001);
                assert!((b.x * 2. + b.w - w).abs() < 0.001);
                assert!((b.y * 2. + b.h - h).abs() < 0.001);
                assert!((b.w / b.h - nw / nh).abs() < 0.001);
                assert_eq!(
                    Options { stretch: true, ..o }.frame(w, h),
                    wireframe::model::Bounds::new(0., 0., w, h)
                );
            }
        }
    }
}
#[test]
fn status_visibility_has_strict_style_and_width_boundaries() {
    for model in [Model::Classic, Model::Tall, Model::Wide, Model::Notched] {
        for sketch in [false, true] {
            for landscape in [false, true] {
                let o = Options {
                    model,
                    landscape,
                    stretch: true,
                    ..Options::default()
                };
                let s = model.sections(sketch);
                let h = if landscape { s[1] + s[2] } else { s[0] + s[3] };
                assert!(!o.layout(500., h, sketch, metrics).status_visible);
                assert!(o.layout(500., h + 1., sketch, metrics).status_visible);
                if model == Model::Notched {
                    let w = if landscape { 215. } else { 260. };
                    assert!(!o.layout(w, 500., sketch, metrics).status_visible);
                    assert!(o.layout(w + 1., 500., sketch, metrics).status_visible);
                }
            }
        }
    }
    let o = Options {
        landscape: true,
        stretch: true,
        ..Options::default()
    };
    assert!(!o.layout(300., 200., true, metrics).status_visible);
    assert!(o.layout(300., 200., false, metrics).status_visible);
}
#[test]
fn transparency_removes_only_the_base_fill_and_keeps_pattern_bars() {
    let o = Options {
        model: Model::Notched,
        stretch: true,
        pattern: Pattern::HeaderDarkFooter,
        ..Options::default()
    };
    let opaque = o.layout(300., 500., false, metrics);
    let transparent = Options {
        transparent: true,
        ..o
    }
    .layout(300., 500., false, metrics);
    assert_eq!(opaque.fills.len(), transparent.fills.len() + 1);
    for (a, b) in opaque.fills.iter().skip(1).zip(&transparent.fills) {
        assert_eq!(a.bounds, b.bounds);
        assert_eq!(a.color, b.color);
    }
    assert_eq!(opaque.status_items.len(), transparent.status_items.len());
    assert_eq!(opaque.guides, transparent.guides);
    let white = Options {
        transparent: true,
        pattern: Pattern::White,
        ..o
    }
    .layout(300., 500., false, metrics);
    assert!(white.fills.is_empty());
    assert!(!white.status_items.is_empty());
    let no_status = Options { status: false, ..o }.layout(300., 500., false, metrics);
    assert!(no_status.status_items.is_empty());
    assert_eq!(no_status.fills.len(), 3);
}
#[test]
fn small_fitted_notched_frame_can_keep_time_without_icons() {
    let l = Options {
        model: Model::Notched,
        ..Options::default()
    }
    .layout(270., 430., true, metrics);
    assert!(l.status_items.iter().any(|i| i.kind == StatusKind::Time));
    assert!(
        l.status_items
            .iter()
            .all(|i| matches!(i.kind, StatusKind::Time | StatusKind::Carrier))
    );
    for item in l.status_items {
        assert!(item.size > 0.);
    }
}
#[test]
fn stored_device_defaults_and_explicit_choices_are_distinct() {
    let defaults = Options::from_properties(&Properties::new());
    assert_eq!(defaults.model, Model::Classic);
    assert!(!defaults.stretch);
    assert!(defaults.status);
    let p = Properties::from([
        ("device_model".into(), Value::Choice("notched".into())),
        ("device_resizing".into(), Value::Choice("stretch".into())),
        ("orientation".into(), Value::Choice("landscape".into())),
        ("background_pattern".into(), Value::Choice("dark".into())),
        ("status_bar".into(), Value::Bool(false)),
        ("transparent_background".into(), Value::Bool(true)),
    ]);
    let o = Options::from_properties(&p);
    assert_eq!(o.native(), (867., 436.));
    assert_eq!(o.pattern, Pattern::Dark);
    assert!(o.stretch && o.transparent && !o.status);
}

use wireframe::{
    document,
    model::{Bounds, Kind},
    render::Drawing,
    scene::Primitive,
    workspace::Editor,
};
fn fixture() -> (Editor, u64) {
    let mut e = Editor::default();
    e.add(Kind::Phone, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
fn set(e: &mut Editor, id: u64, key: &str, value: Value) {
    e.begin("Edit phone");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
#[test]
fn insertion_auto_size_fixed_geometry_history_and_storage_follow_model_choices() {
    let (mut e, id) = fixture();
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(20., 30., 436., 867.)
    );
    let (spec, p) = e.control_properties(id).unwrap();
    assert_eq!(p["device_resizing"], Value::Choice("stretch".into()));
    for key in [
        "device_model",
        "orientation",
        "device_resizing",
        "background_pattern",
        "transparent_background",
        "status_bar",
    ] {
        assert!(spec.property(key).unwrap().visible);
    }
    let baseline = e.document.clone();
    set(&mut e, id, "device_model", Value::Choice("classic".into()));
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(20., 30., 282., 510.)
    );
    e.undo();
    assert_eq!(e.document, baseline);
    e.redo();
    set(&mut e, id, "orientation", Value::Choice("landscape".into()));
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(20., 30., 510., 282.)
    );
    e.begin("Resize phone");
    e.selected_mut(|c| {
        c.bounds.w = 300.;
        c.bounds.h = 500.;
    });
    e.commit();
    set(&mut e, id, "device_model", Value::Choice("wide".into()));
    set(&mut e, id, "orientation", Value::Choice("portrait".into()));
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(20., 30., 300., 500.)
    );
    e.begin("Auto-size phone");
    e.auto_size_control(id).unwrap();
    e.commit();
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(20., 30., 380., 754.)
    );
    e.undo();
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(20., 30., 300., 500.)
    );
    assert_eq!(
        document::decode(&e.document.encode().unwrap()).unwrap(),
        e.document
    );
}
#[test]
fn phone_overrides_are_local_resettable_and_invalid_choices_are_atomic() {
    let (mut e, id) = fixture();
    e.create_component("Device".into()).unwrap();
    let defs = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    set(&mut e, child, "transparent_background", Value::Bool(true));
    set(&mut e, child, "device_model", Value::Choice("tall".into()));
    assert_eq!(e.document.definitions, defs);
    let before = e.document.clone();
    assert!(
        e.set_control_property(child, "device_model", Value::Choice("invalid".into()))
            .is_err()
    );
    assert_eq!(e.document, before);
    e.begin("Reset model");
    e.reset_control_property(child, "device_model").unwrap();
    e.commit();
    assert_eq!(
        e.control_properties(child).unwrap().1["device_model"],
        Value::Choice("notched".into())
    );
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(
        document::decode(&e.document.encode().unwrap()).unwrap(),
        e.document
    );
    assert!(e.rendered_scene(e.page).is_ok());
}
#[test]
fn legacy_phone_keeps_its_explicit_dimensions_and_stored_defaults() {
    let mut legacy = wireframe::model::Document::default();
    let phone = wireframe::model::Control::new(2, Kind::Phone, 20., 30.);
    let bounds = phone.bounds;
    legacy.pages[0].controls.push(phone);
    legacy.next_id = 3;
    let project = document::Project::try_from(legacy).unwrap();
    let node = &project.pages[0].content.nodes[0];
    let document::NodeContent::Control { kind, size } = &node.content else {
        panic!("control expected")
    };
    assert_eq!(kind, "phone");
    assert_eq!(*size, document::Size::fixed(bounds.w, bounds.h));
    assert_eq!(
        Options::from_properties(&node.properties).model,
        Model::Classic
    );
    assert!(!Options::from_properties(&node.properties).stretch);
    assert_eq!(
        document::decode(&project.encode().unwrap()).unwrap(),
        project
    );
}
#[test]
fn frame_is_seamless_and_transparent_screen_preserves_background_and_pattern_bars() {
    let mut e = Editor::default();
    e.add(Kind::FieldSet, 0., 0.);
    let back = *e.selection.iter().next().unwrap();
    set(&mut e, back, "text", Value::Text(String::new()));
    set(&mut e, back, "fill", Value::Color([30, 170, 80, 255]));
    e.selected_mut(|c| c.bounds = Bounds::new(0., 0., 300., 500.));
    e.add(Kind::Phone, 0., 0.);
    let front = *e.selection.iter().next().unwrap();
    e.selected_mut(|c| c.bounds = Bounds::new(0., 0., 300., 500.));
    set(&mut e, front, "transparent_background", Value::Bool(true));
    set(
        &mut e,
        front,
        "background_pattern",
        Value::Choice("header_dark_footer".into()),
    );
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        let s = e.rendered_scene(e.page).unwrap();
        assert_eq!(
            s.items
                .iter()
                .filter(|i| matches!(i.drawing, Drawing::Primitive(Primitive::Mesh { .. })))
                .count(),
            1
        );
        let image = image::load_from_memory(&s.png().unwrap())
            .unwrap()
            .to_rgba8();
        assert_eq!(image.get_pixel(24 + 150, 24 + 200).0, [30, 170, 80, 255]);
        assert_eq!(image.get_pixel(24 + 150, 24 + 80).0, [130, 136, 150, 255]);
        assert_eq!(image.get_pixel(24 + 150, 24 + 460).0, [63, 63, 63, 255]);
        let frame = if sketch {
            [193, 192, 192, 255]
        } else {
            [255; 4]
        };
        // A continuous strip catches independently-antialiased triangle seams.
        for y in 100..400 {
            for x in 10..17 {
                assert_eq!(image.get_pixel(24 + x, 24 + y).0, frame, "frame at {x},{y}");
            }
        }
    }
}
#[test]
fn every_model_renders_finite_clipped_geometry_at_small_and_extreme_sizes() {
    let (mut e, id) = fixture();
    for model in ["classic", "tall", "wide", "notched"] {
        set(&mut e, id, "device_model", Value::Choice(model.into()));
        for mode in ["fit", "stretch"] {
            set(&mut e, id, "device_resizing", Value::Choice(mode.into()));
            for orientation in ["portrait", "landscape"] {
                set(&mut e, id, "orientation", Value::Choice(orientation.into()));
                for (w, h) in [(1., 1.), (20., 1000.), (1000., 20.), (300., 500.)] {
                    e.selected_mut(|c| c.bounds = Bounds::new(20., 30., w, h));
                    for sketch in [false, true] {
                        e.document.settings.sketch = sketch;
                        let s = e.rendered_scene(e.page).unwrap();
                        for item in &s.items {
                            if let Drawing::Primitive(p) = &item.drawing {
                                let svg = wireframe::scene::primitive_svg(p.clone());
                                assert!(!svg.contains("NaN") && !svg.contains("inf"));
                            }
                            let clip = item.clip.unwrap();
                            assert!(
                                clip.x >= 20.
                                    && clip.y >= 30.
                                    && clip.x + clip.w <= 20. + w
                                    && clip.y + clip.h <= 30. + h
                            );
                            if let Drawing::Primitive(Primitive::Mesh {
                                points, indices, ..
                            }) = &item.drawing
                            {
                                assert!(indices.iter().all(|i| (*i as usize) < points.len()));
                                assert!(points.iter().flatten().all(|n| n.is_finite()));
                            }
                        }
                    }
                }
            }
        }
    }
}
