// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{self, Value},
    layout::TextMetrics,
    model::{Bounds, Kind},
    render::Drawing,
    scene::Primitive,
    tablet::{Clock, DetailKind, Options, StatusKind},
    workspace::Editor,
};
fn metrics(size: f32) -> TextMetrics {
    TextMetrics {
        width: size * 2.,
        height: size,
        rows: 1,
    }
}
fn frozen() -> Clock {
    Clock::new(9, 52).unwrap()
}
fn fixture() -> (Editor, u64) {
    let mut e = Editor::with_clock(document::Project::default(), frozen);
    e.add(Kind::Tablet, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
fn set(e: &mut Editor, id: u64, key: &str, value: Value) {
    e.begin("Edit tablet");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
#[test]
fn fit_centers_the_frame_and_stretch_retains_dimensions() {
    for landscape in [false, true] {
        let o = Options {
            landscape,
            ..Default::default()
        };
        for (w, h) in [(300., 500.), (500., 300.), (1., 1.), (10000., 20.)] {
            let l = o.layout(w, h, false, metrics);
            assert!((l.frame.x * 2. + l.frame.w - w).abs() < 0.001);
            assert!((l.frame.y * 2. + l.frame.h - h).abs() < 0.001);
            assert!((l.frame.w / l.frame.h - o.native().0 / o.native().1).abs() < 0.001);
            assert_eq!(l.status_items.len(), 4);
            assert_eq!(
                Options { stretch: true, ..o }.frame(w, h),
                Bounds::new(0., 0., w, h)
            );
        }
    }
}
#[test]
fn status_thresholds_are_strict_and_small_fill_fallback_hides_items() {
    for landscape in [false, true] {
        let o = Options {
            landscape,
            stretch: true,
            ..Default::default()
        };
        let offset = if landscape { 186. } else { 64. };
        for (kind, boundary) in [
            (StatusKind::Wifi, offset + 42.),
            (StatusKind::Battery, offset + 66.),
            (StatusKind::Time, if landscape { 306. } else { 185. }),
        ] {
            assert!(
                !o.layout(boundary, 500., false, metrics)
                    .status_items
                    .iter()
                    .any(|i| i.kind == kind)
            );
            assert!(
                o.layout(boundary + 1., 500., false, metrics)
                    .status_items
                    .iter()
                    .any(|i| i.kind == kind)
            );
        }
        let threshold = if landscape { 80. } else { 202. };
        assert!(
            o.layout(500., threshold - 1., true, metrics)
                .status_items
                .is_empty()
        );
        let at = o.layout(500., threshold, true, metrics);
        assert_eq!(at.fills.len(), 1);
        assert!(at.status_items.is_empty());
        assert_eq!(
            o.layout(500., threshold + 1., true, metrics)
                .status_items
                .len(),
            4
        );
        let tiny = o.layout(1., 1., true, metrics);
        assert_eq!(tiny.screen, Bounds::new(2., 2., -3., -3.));
        assert!(tiny.status_items.is_empty() && tiny.details.is_empty());
    }
}
#[test]
fn details_rotate_to_home_left_and_camera_right() {
    for sketch in [false, true] {
        for stretch in [false, true] {
            let l = Options {
                landscape: true,
                stretch,
                ..Default::default()
            }
            .layout(916., 616., sketch, metrics);
            let home = l
                .details
                .iter()
                .find(|d| d.kind == DetailKind::Home)
                .unwrap()
                .bounds;
            let camera = l
                .details
                .iter()
                .find(|d| d.kind == DetailKind::Camera)
                .unwrap()
                .bounds;
            assert!(home.x < 94. && camera.x > 820.);
            assert!((home.y + home.h / 2. - 308.).abs() < 0.001);
            assert!((camera.y + camera.h / 2. - 308.).abs() < 0.001);
        }
    }
}
#[test]
fn insertion_auto_size_fixed_size_history_and_storage() {
    let (mut e, id) = fixture();
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(20., 30., 616., 916.)
    );
    let (spec, p) = e.control_properties(id).unwrap();
    assert_eq!(p["device_resizing"], Value::Choice("stretch".into()));
    for key in [
        "orientation",
        "device_resizing",
        "transparent_background",
        "status_bar",
    ] {
        assert!(spec.property(key).unwrap().visible);
    }
    assert!(
        spec.property("device_model").is_none() && spec.property("background_pattern").is_none()
    );
    set(&mut e, id, "orientation", Value::Choice("landscape".into()));
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(20., 30., 916., 616.)
    );
    e.begin("Resize tablet");
    e.selected_mut(|c| {
        c.bounds.w = 300.;
        c.bounds.h = 500.;
    });
    e.commit();
    set(&mut e, id, "device_resizing", Value::Choice("fit".into()));
    set(&mut e, id, "orientation", Value::Choice("portrait".into()));
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(20., 30., 300., 500.)
    );
    let before = e.document.clone();
    set(&mut e, id, "transparent_background", Value::Bool(true));
    e.undo();
    assert_eq!(before, e.document);
    e.redo();
    e.begin("Auto-size");
    e.auto_size_control(id).unwrap();
    e.commit();
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(20., 30., 616., 916.)
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
fn local_overrides_reset_without_changing_component_source() {
    let (mut e, id) = fixture();
    e.create_component("Tablet frame".into()).unwrap();
    let defs = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    set(&mut e, child, "status_bar", Value::Bool(false));
    set(
        &mut e,
        child,
        "orientation",
        Value::Choice("landscape".into()),
    );
    assert_eq!(e.document.definitions, defs);
    let before = e.document.clone();
    assert!(
        e.set_control_property(child, "orientation", Value::Choice("invalid".into()))
            .is_err()
    );
    assert_eq!(e.document, before);
    e.begin("Reset orientation");
    e.reset_control_property(child, "orientation").unwrap();
    e.commit();
    assert_eq!(
        e.control_properties(child).unwrap().1["orientation"],
        Value::Choice("portrait".into())
    );
    e.undo();
    assert_eq!(e.document, before);
    assert!(e.rendered_scene(e.page).is_ok());
}
#[test]
fn clock_formats_midnight_noon_and_invalid_values() {
    for (h, m, want) in [
        (0, 0, "12:00 AM"),
        (0, 9, "12:09 AM"),
        (12, 0, "12:00 PM"),
        (13, 5, "1:05 PM"),
        (23, 59, "11:59 PM"),
    ] {
        assert_eq!(Clock::new(h, m).unwrap().label(), want);
    }
    assert!(Clock::new(24, 0).is_none() && Clock::new(0, 60).is_none());
}
#[test]
fn clock_stays_per_control_until_its_contents_relayout() {
    use std::sync::atomic::{AtomicU8, Ordering};
    static MINUTE: AtomicU8 = AtomicU8::new(1);
    fn tick() -> Clock {
        Clock::new(10, MINUTE.load(Ordering::SeqCst)).unwrap()
    }
    fn clocks(e: &Editor) -> Vec<String> {
        e.rendered_scene(e.page)
            .unwrap()
            .items
            .iter()
            .filter_map(|i| {
                if let Drawing::Glyph(g) = &i.drawing {
                    g.text.contains(':').then(|| g.text.clone())
                } else {
                    None
                }
            })
            .collect()
    }
    let mut e = Editor::with_clock(document::Project::default(), tick);
    e.add(Kind::Tablet, 0., 0.);
    let a = *e.selection.iter().next().unwrap();
    e.add(Kind::Tablet, 700., 0.);
    let b = *e.selection.iter().next().unwrap();
    assert_eq!(clocks(&e), ["10:01 AM", "10:01 AM"]);
    MINUTE.store(2, Ordering::SeqCst);
    assert_eq!(clocks(&e), ["10:01 AM", "10:01 AM"]);
    e.selected_mut(|c| c.bounds.x += 1.);
    assert_eq!(clocks(&e), ["10:01 AM", "10:01 AM"]);
    e.document.title = "Renamed".into();
    assert_eq!(clocks(&e), ["10:01 AM", "10:01 AM"]);
    set(&mut e, a, "status_bar", Value::Bool(false));
    assert_eq!(clocks(&e), ["10:01 AM"]);
    set(&mut e, a, "status_bar", Value::Bool(true));
    assert_eq!(clocks(&e), ["10:02 AM", "10:01 AM"]);
    MINUTE.store(3, Ordering::SeqCst);
    set(&mut e, b, "orientation", Value::Choice("landscape".into()));
    assert_eq!(clocks(&e), ["10:02 AM", "10:03 AM"]);
    assert!(
        !e.document
            .encode()
            .unwrap()
            .windows(8)
            .any(|v| v == b"10:03 AM")
    );
}
#[test]
fn transparent_frame_has_no_seams_and_tiny_geometry_stays_finite() {
    let mut e = Editor::with_clock(document::Project::default(), frozen);
    e.add(Kind::FieldSet, 0., 0.);
    let back = *e.selection.iter().next().unwrap();
    set(&mut e, back, "text", Value::Text(String::new()));
    set(&mut e, back, "fill", Value::Color([30, 170, 80, 255]));
    e.selected_mut(|c| c.bounds = Bounds::new(0., 0., 300., 500.));
    e.add(Kind::Tablet, 0., 0.);
    let id = *e.selection.iter().next().unwrap();
    set(&mut e, id, "transparent_background", Value::Bool(true));
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        e.selected_mut(|c| c.bounds = Bounds::new(0., 0., 300., 500.));
        let scene = e.rendered_scene(e.page).unwrap();
        let image = image::load_from_memory(&scene.png().unwrap())
            .unwrap()
            .to_rgba8();
        assert_eq!(image.get_pixel(24 + 150, 24 + 200).0, [30, 170, 80, 255]);
        let body = if sketch {
            [69, 69, 69, 255]
        } else {
            [222, 221, 221, 255]
        };
        for y in 110..390 {
            for x in 10..25 {
                assert_eq!(image.get_pixel(24 + x, 24 + y).0, body, "{x},{y}");
            }
        }
        for mode in ["fit", "stretch"] {
            set(&mut e, id, "device_resizing", Value::Choice(mode.into()));
            for orientation in ["portrait", "landscape"] {
                set(&mut e, id, "orientation", Value::Choice(orientation.into()));
                for (w, h) in [
                    (1., 1.),
                    (20., 1000.),
                    (1000., 20.),
                    (300., 201.),
                    (300., 202.),
                    (300., 203.),
                ] {
                    e.selected_mut(|c| c.bounds = Bounds::new(0., 0., w, h));
                    let scene = e.rendered_scene(e.page).unwrap();
                    for item in &scene.items {
                        if let Drawing::Primitive(p) = &item.drawing {
                            let svg = wireframe::scene::primitive_svg(p.clone());
                            assert!(!svg.contains("NaN") && !svg.contains("inf"));
                            if let Primitive::Mesh {
                                points, indices, ..
                            } = p
                            {
                                assert!(indices.iter().all(|i| (*i as usize) < points.len()));
                            }
                        }
                    }
                }
            }
        }
        set(
            &mut e,
            id,
            "device_resizing",
            Value::Choice("stretch".into()),
        );
        set(&mut e, id, "orientation", Value::Choice("portrait".into()));
    }
}
