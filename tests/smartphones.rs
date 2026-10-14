// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{self, Value},
    layout::TextMetrics,
    model::{Bounds, Kind},
    render::Drawing,
    scene::Primitive,
    smartphone::{DetailKind, Options, StatusKind},
    tablet::Clock,
    workspace::Editor,
};
fn metrics(size: f32) -> TextMetrics {
    TextMetrics {
        width: 2. * size,
        height: size,
        rows: 1,
    }
}
fn frozen() -> Clock {
    Clock::new(9, 52).unwrap()
}
fn fixture() -> (Editor, u64) {
    let mut e = Editor::with_clock(document::Project::default(), frozen);
    e.add(Kind::Smartphone, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
fn set(e: &mut Editor, id: u64, key: &str, value: Value) {
    e.begin("Edit smartphone");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
#[test]
fn insertion_auto_size_fixed_size_history_and_storage() {
    let (mut e, id) = fixture();
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(20., 30., 356., 725.)
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
        Bounds::new(20., 30., 725., 356.)
    );
    e.begin("Resize smartphone");
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
        Bounds::new(20., 30., 356., 725.)
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
    e.create_component("Smartphone frame".into()).unwrap();
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
fn strict_status_boundaries_depend_on_measured_text_width() {
    for (sketch, thresholds) in [(true, [86., 105., 125.]), (false, [82., 97., 113.])] {
        let o = Options {
            stretch: true,
            ..Default::default()
        };
        for (kind, first) in [StatusKind::Battery, StatusKind::Signal, StatusKind::Wifi]
            .into_iter()
            .zip(thresholds)
        {
            let has = |w, extra| {
                o.layout(w, 500., sketch, |s| TextMetrics {
                    width: 2. * s + extra,
                    ..metrics(s)
                })
                .status_items
                .iter()
                .any(|i| i.kind == kind)
            };
            assert!(!has(first - 1., 0.) && has(first, 0.));
            assert!(!has(first + 5., 6.) && has(first + 6., 6.));
        }
        for (w, h) in [(70., 500.), (300., 167.)] {
            assert!(o.layout(w, h, sketch, metrics).status_items.is_empty());
            assert!(o.layout(w, h, sketch, metrics).details.is_empty());
        }
        let narrow = o.layout(71., 500., sketch, |s| TextMetrics {
            width: 2.5 * s,
            ..metrics(s)
        });
        assert_eq!(narrow.fills.len(), 2);
        assert!(narrow.status_items.is_empty());
        assert_eq!(o.layout(300., 168., sketch, metrics).status_items.len(), 4);
    }
}
#[test]
fn fit_scales_frame_but_caps_font_and_fixed_text_margin() {
    for landscape in [false, true] {
        let o = Options {
            landscape,
            ..Default::default()
        };
        let (w, h) = o.native();
        for scale in [0.1, 0.25, 0.5, 1., 2.] {
            let l = o.layout(w * scale + 100., h * scale, false, metrics);
            assert!((l.frame.x - 50.).abs() < 0.001);
            assert!((l.zoom - scale).abs() < 0.001);
            let t = l.status_items[0];
            assert_eq!(t.kind, StatusKind::Time);
            assert!((t.size - 12_f32.min(16. * scale)).abs() < 0.001);
            assert!((l.screen.x + l.screen.w - t.bounds.x - t.bounds.w - 5.).abs() < 0.001);
            assert_eq!(l.status_items.len(), if scale <= 0.25 { 1 } else { 4 });
            let g = l.guides.unwrap();
            assert!((g[1] - l.screen.y - 16. * scale).abs() < 0.001);
            assert!((g[2] - l.screen.x - l.screen.w).abs() < 0.001);
        }
    }
}
#[test]
fn stretch_hardware_has_strict_width_thresholds_and_rotates_left_to_right() {
    for landscape in [false, true] {
        let o = Options {
            landscape,
            stretch: true,
            ..Default::default()
        };
        for (kind, first) in [
            (DetailKind::Bottom, 170.),
            (DetailKind::Speaker, if landscape { 238. } else { 237. }),
        ] {
            let l = |side| {
                if landscape {
                    o.layout(500., side, true, metrics)
                } else {
                    o.layout(side, 500., true, metrics)
                }
            };
            assert!(!l(first - 1.).details.iter().any(|d| d.kind == kind));
            assert!(l(first).details.iter().any(|d| d.kind == kind));
        }
        assert!(o.layout(500., 500., false, metrics).guides.is_none());
    }
    let l = Options {
        landscape: true,
        stretch: true,
        ..Default::default()
    }
    .layout(725., 356., false, metrics);
    assert!(
        l.details
            .iter()
            .find(|d| d.kind == DetailKind::Speaker)
            .unwrap()
            .bounds
            .x
            < 79.
    );
    assert!(
        l.details
            .iter()
            .find(|d| d.kind == DetailKind::Bottom)
            .unwrap()
            .bounds
            .x
            > 645.
    );
}
#[test]
fn only_fitted_frames_expose_interior_alignment_targets() {
    use wireframe::snapping::{Axis, Kind as GuideKind};
    let (mut e, id) = fixture();
    for (mode, count) in [("stretch", 0), ("fit", 4)] {
        set(&mut e, id, "device_resizing", Value::Choice(mode.into()));
        e.selection.clear();
        let targets = e.alignment_targets(0.).unwrap();
        // Generic targets include 16-unit exterior spacing in both modes.
        let outside = targets.matches(Bounds::new(4., 14., 388., 757.));
        assert_eq!(
            outside
                .iter()
                .filter(|v| v.kind == GuideKind::Spacing)
                .count(),
            4
        );

        let g = Options::default()
            .layout(356., 725., true, metrics)
            .guides
            .unwrap();
        let found = targets.matches(Bounds::new(
            20. + g[0],
            30. + g[1],
            g[2] - g[0],
            g[3] - g[1],
        ));
        assert_eq!(
            found
                .iter()
                .filter(|v| v.kind == GuideKind::Spacing)
                .count(),
            count
        );
        if count > 0 {
            assert!(
                found
                    .iter()
                    .any(|v| v.axis == Axis::Y && v.position == 133.)
            );
        }
    }
}
#[test]
fn clocks_cache_independently_across_frame_families_and_relayout() {
    use std::sync::atomic::{AtomicU8, Ordering};
    static MINUTE: AtomicU8 = AtomicU8::new(1);
    fn tick() -> Clock {
        Clock::new(13, MINUTE.load(Ordering::SeqCst)).unwrap()
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
    assert_eq!(Clock::new(0, 5).unwrap().label_24(), "0:05");
    assert_eq!(Clock::new(23, 59).unwrap().label_24(), "23:59");
    let mut e = Editor::with_clock(document::Project::default(), tick);
    e.add(Kind::Smartphone, 0., 0.);
    let id = *e.selection.iter().next().unwrap();
    e.add(Kind::Tablet, 500., 0.);
    assert_eq!(clocks(&e), ["13:01", "1:01 PM"]);
    MINUTE.store(2, Ordering::SeqCst);
    e.selected_mut(|c| c.bounds.x += 5.);
    assert_eq!(clocks(&e), ["13:01", "1:01 PM"]);
    set(&mut e, id, "status_bar", Value::Bool(false));
    assert_eq!(clocks(&e), ["1:01 PM"]);
    set(&mut e, id, "status_bar", Value::Bool(true));
    assert_eq!(clocks(&e), ["13:02", "1:01 PM"]);
    let saved = e.document.encode().unwrap();
    assert!(!String::from_utf8_lossy(&saved).contains("13:02"));
    let reopened = Editor::with_clock(document::decode(&saved).unwrap(), tick);
    assert_eq!(clocks(&reopened), ["13:02", "1:02 PM"]);
}
#[test]
fn transparency_has_a_real_opening_and_tiny_frames_are_safe() {
    let mut e = Editor::with_clock(document::Project::default(), frozen);
    e.add(Kind::FieldSet, 0., 0.);
    let back = *e.selection.iter().next().unwrap();
    set(&mut e, back, "text", Value::Text(String::new()));
    set(&mut e, back, "fill", Value::Color([30, 170, 80, 255]));
    e.selected_mut(|c| c.bounds = Bounds::new(0., 0., 300., 500.));
    e.add(Kind::Smartphone, 0., 0.);
    let id = *e.selection.iter().next().unwrap();
    set(&mut e, id, "transparent_background", Value::Bool(true));
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        e.selected_mut(|c| c.bounds = Bounds::new(0., 0., 300., 500.));
        let image = image::load_from_memory(&e.rendered_scene(e.page).unwrap().png().unwrap())
            .unwrap()
            .to_rgba8();
        assert_eq!(image.get_pixel(174, 224).0, [30, 170, 80, 255]);
        for y in 120..380 {
            for x in 7..14 {
                assert_eq!(image.get_pixel(24 + x, 24 + y).0, [255; 4]);
            }
        }
        for mode in ["fit", "stretch"] {
            set(&mut e, id, "device_resizing", Value::Choice(mode.into()));
            for (w, h) in [
                (1., 1.),
                (20., 1000.),
                (1000., 20.),
                (70., 500.),
                (300., 167.),
                (300., 168.),
            ] {
                e.selected_mut(|c| c.bounds = Bounds::new(0., 0., w, h));
                for i in e.rendered_scene(e.page).unwrap().items {
                    if let Drawing::Primitive(p) = i.drawing {
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
        set(
            &mut e,
            id,
            "device_resizing",
            Value::Choice("stretch".into()),
        );
    }
}
