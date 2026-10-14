// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{self, Value},
    mobile_keyboard::{Icon, Legend, Options},
    model::{Bounds, Kind},
    render::Drawing,
    workspace::Editor,
};
fn fixture() -> (Editor, u64) {
    let mut e = Editor::new(document::Project::default());
    e.add(Kind::MobileKeyboard, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
fn set(e: &mut Editor, id: u64, key: &str, value: Value) {
    e.begin("Edit keyboard");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
#[test]
fn variants_auto_size_fixed_bounds_history_and_storage() {
    let (mut e, id) = fixture();
    for (device, orientation, w, h) in [
        ("phone", "portrait", 226., 160.),
        ("phone", "landscape", 403., 114.),
        ("tablet", "portrait", 546., 188.),
        ("tablet", "landscape", 728., 251.),
    ] {
        set(&mut e, id, "keyboard_device", Value::Choice(device.into()));
        set(&mut e, id, "orientation", Value::Choice(orientation.into()));
        assert_eq!(e.selected_views()[0].bounds, Bounds::new(20., 30., w, h));
    }
    e.begin("Resize keyboard");
    e.selected_mut(|c| {
        c.bounds.w = 300.;
        c.bounds.h = 300.;
    });
    e.commit();
    set(&mut e, id, "keyboard_device", Value::Choice("phone".into()));
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(20., 30., 300., 300.)
    );
    let before = e.document.clone();
    e.begin("Auto-size");
    e.auto_size_control(id).unwrap();
    e.commit();
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(20., 30., 403., 114.)
    );
    e.undo();
    assert_eq!(before, e.document);
    e.redo();
    assert_eq!(
        document::decode(&e.document.encode().unwrap()).unwrap(),
        e.document
    );
    let (spec, _) = e.control_properties(id).unwrap();
    for name in ["keyboard_device", "orientation"] {
        assert!(spec.property(name).unwrap().visible);
    }
    assert!(spec.property("device_resizing").is_none());
}
#[test]
fn keyboard_variants_remain_local_to_component_instances() {
    let (mut e, id) = fixture();
    e.create_component("Keyboard".into()).unwrap();
    let defs = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    set(
        &mut e,
        child,
        "keyboard_device",
        Value::Choice("tablet".into()),
    );
    assert_eq!(defs, e.document.definitions);
    let before = e.document.clone();
    assert!(
        e.set_control_property(child, "keyboard_device", Value::Choice("invalid".into()))
            .is_err()
    );
    assert_eq!(before, e.document);
    e.begin("Reset keyboard");
    e.reset_control_property(child, "keyboard_device").unwrap();
    e.commit();
    assert_eq!(
        e.control_properties(child).unwrap().1["keyboard_device"],
        Value::Choice("phone".into())
    );
    e.undo();
    assert_eq!(e.document, before);
    assert!(e.rendered_scene(e.page).is_ok());
}
#[test]
fn proportional_fit_preserves_rows_legends_and_icon_scale() {
    for tablet in [false, true] {
        for landscape in [false, true] {
            let o = Options { tablet, landscape };
            let (w, h) = o.native();
            let base = o.layout(w, h);
            assert_eq!(base.keys.len(), if tablet { 38 } else { 33 });
            let letters: String = base
                .keys
                .iter()
                .filter_map(|k| match k.legend {
                    Legend::Text(s) if s.len() == 1 && s.chars().all(char::is_uppercase) => Some(s),
                    _ => None,
                })
                .collect();
            assert_eq!(letters, "QWERTYUIOPASDFGHJKLZXCVBNM");
            assert_eq!(
                base.keys
                    .iter()
                    .filter(|k| k.legend == Legend::Icon(Icon::Dismiss))
                    .count(),
                usize::from(tablet)
            );
            for scale in [0.01, 0.1, 0.5, 1., 2., 4.] {
                let l = o.layout(w * scale + 100., h * scale);
                assert!((l.frame.x - 50.).abs() < 0.001);
                for (a, b) in l.keys.iter().zip(&base.keys) {
                    assert_eq!(a.legend, b.legend);
                    assert!((a.bounds.x - 50. - b.bounds.x * scale).abs() < 0.001);
                    assert!((a.bounds.y - b.bounds.y * scale).abs() < 0.001);
                    assert!((a.font_size - b.font_size * scale).abs() < 0.001);
                    assert!((a.icon_size - b.icon_size * scale).abs() < 0.001);
                }
            }
        }
    }
}
#[test]
fn rendering_keeps_letterboxing_transparent_and_shared_icons_available() {
    let (mut e, id) = fixture();
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        for device in ["phone", "tablet"] {
            for orientation in ["portrait", "landscape"] {
                set(&mut e, id, "keyboard_device", Value::Choice(device.into()));
                set(&mut e, id, "orientation", Value::Choice(orientation.into()));
                e.selected_mut(|c| c.bounds = Bounds::new(0., 0., 300., 300.));
                let scene = e.rendered_scene(e.page).unwrap();
                finite_scene(&scene);
                let icons = scene
                    .items
                    .iter()
                    .filter_map(|i| {
                        if let Drawing::Icon { id, .. } = &i.drawing {
                            Some(id)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                assert_eq!(icons.len(), 2);
                assert!(
                    icons
                        .into_iter()
                        .all(|id| wireframe::icons::get(id).is_some())
                );
                let png = image::load_from_memory(&scene.png().unwrap())
                    .unwrap()
                    .to_rgba8();
                // Export adds a 24-unit margin; the square's top-left remains unpainted.
                assert_eq!(png.get_pixel(25, 25).0, [255; 4]);
                let o = Options {
                    tablet: device == "tablet",
                    landscape: orientation == "landscape",
                };
                let frame = o.layout(300., 300.).frame;
                assert_eq!(
                    png.get_pixel(24, 24 + frame.y.ceil() as u32 + 1).0,
                    [238, 238, 238, 255]
                );
            }
        }
    }
}
#[test]
fn tiny_frames_and_invalid_geometry_do_not_make_nonfinite_drawing() {
    let (mut e, _) = fixture();
    for (w, h) in [(1., 1.), (1., 1000.), (1000., 1.)] {
        e.selected_mut(|c| c.bounds = Bounds::new(0., 0., w, h));
        finite_scene(&e.rendered_scene(e.page).unwrap());
    }
    for (w, h) in [(0., 0.), (-1., 100.), (f32::NAN, 20.), (10., f32::INFINITY)] {
        let l = Options::default().layout(w, h);
        assert!(l.scale.is_finite());
        assert!(
            l.keys
                .iter()
                .all(|k| k.bounds.x.is_finite() && k.bounds.y.is_finite())
        );
    }
}

fn finite_scene(scene: &wireframe::render::Scene) {
    for item in &scene.items {
        match &item.drawing {
            Drawing::Primitive(p) => {
                let svg = wireframe::scene::primitive_svg(p.clone());
                assert!(!svg.contains("NaN") && !svg.contains("inf"));
            }
            Drawing::Glyph(g) => assert!(
                [g.x, g.baseline, g.size, g.advance]
                    .into_iter()
                    .all(f32::is_finite)
            ),
            Drawing::Icon { bounds, .. } => assert!(
                [bounds.x, bounds.y, bounds.w, bounds.h]
                    .into_iter()
                    .all(f32::is_finite)
            ),
            _ => {}
        }
    }
}
