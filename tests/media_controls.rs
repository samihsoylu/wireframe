// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    catalog,
    document::{self, Dimension, NodeContent, Project, Value},
    media_controls,
    model::{Bounds, Kind},
    workspace::Editor,
};
const KINDS: [Kind; 3] = [Kind::VolumeSlider, Kind::Playback, Kind::Webcam];
#[test]
fn defaults_and_static_property_contracts() {
    for (kind, id, size) in [
        (Kind::VolumeSlider, "volume_slider", (72., 16.)),
        (Kind::Playback, "playback", (110., 36.)),
        (Kind::Webcam, "webcam", (177., 146.)),
    ] {
        let mut e = Editor::new(Project::default());
        e.add(kind, 20., 30.);
        let c = &e.selected_views()[0];
        assert_eq!((c.bounds.w, c.bounds.h), size);
        assert_eq!(c.text, "");
        let spec = catalog::get(id).unwrap();
        assert!(spec.properties.iter().all(|p| !p.visible));
        assert!(spec.property("value").is_none());
        assert!(e.rendered_scene(e.page).unwrap().links.is_empty());
    }
}
#[test]
fn sizing_and_legacy_properties_survive_save_without_changing_rendering() {
    for kind in KINDS {
        let mut e = Editor::new(Project::default());
        e.add(kind, 20., 30.);
        let id = *e.selection.iter().next().unwrap();
        e.selected_mut(|c| {
            c.bounds.w = 1.;
            c.bounds.h = 1.;
        });
        let expected = match kind {
            Kind::VolumeSlider => (1., 16.),
            Kind::Playback => (110., 36.),
            _ => (1., 1.),
        };
        let b = e.selected_views()[0].bounds;
        assert_eq!((b.w, b.h), expected);
        let png = e.rendered_scene(e.page).unwrap().png().unwrap();
        e.document.pages[0].content.nodes[0]
            .properties
            .insert("value".into(), Value::Number(0.8));
        let bytes = e.document.encode().unwrap();
        let mut reopened = Editor::new(document::decode(&bytes).unwrap());
        reopened.select(id, false);
        assert_eq!(reopened.selected_views()[0].bounds, b);
        assert_eq!(reopened.document.encode().unwrap(), bytes);
        assert_eq!(
            reopened
                .rendered_scene(reopened.page)
                .unwrap()
                .png()
                .unwrap(),
            png
        );
    }
}
#[test]
fn playback_ignores_explicit_saved_dimensions() {
    let mut e = Editor::new(Project::default());
    e.add(Kind::Playback, 20., 30.);
    if let NodeContent::Control { size, .. } = &mut e.document.pages[0].content.nodes[0].content {
        size.width = Dimension::Fixed(220.);
        size.height = Dimension::Fixed(72.);
    }
    let e = Editor::new(document::decode(&e.document.encode().unwrap()).unwrap());
    let b = e.current().controls[0].bounds;
    assert_eq!((b.w, b.h), (110., 36.));
}
#[test]
fn volume_compresses_fixed_art_below_fifty_four_and_extends_track_above() {
    for w in [1., 10., 16., 32., 52., 53., 54., 55., 72., 200., 300.] {
        let l = media_controls::layout(Kind::VolumeSlider, w, 16., false);
        assert!((l.volume_left + l.volume_middle + l.volume_right - w).abs() < 0.0001);
        assert!((l.volume_left - 52. * (w / 54.).min(1.)).abs() < 0.0001);
        assert_eq!(l.volume_middle, (w - 54.).max(0.));
        assert_eq!(l.frame.h, 16.);
    }
}
#[test]
fn webcam_minimum_resize_preserves_anchor_and_shift_aspect() {
    let b = Bounds::new(20., 30., 177., 146.);
    for handle in [(-1, -1), (1, 1), (-1, 0), (0, -1), (1, 0), (0, 1)] {
        for aspect in [false, true] {
            let r = media_controls::resize_placeholder(
                b,
                handle,
                (-1000. * handle.0 as f32, -1000. * handle.1 as f32),
                aspect,
            );
            assert!(r.w >= 10. && r.h >= 10.);
            if handle.0 < 0 {
                assert!((r.x + r.w - b.x - b.w).abs() < 0.001);
            }
            if handle.1 < 0 {
                assert!((r.y + r.h - b.y - b.h).abs() < 0.001);
            }
            if aspect {
                assert!((r.w / r.h - b.w / b.h).abs() < 0.001);
            }
        }
    }
}
#[test]
fn renderers_clip_tiny_controls_and_export_both_styles() {
    for kind in KINDS {
        for sketch in [false, true] {
            let mut e = Editor::new(Project::default());
            e.document.settings.sketch = sketch;
            e.add(kind, 20., 30.);
            e.selected_mut(|c| {
                c.bounds.w = 1.;
                c.bounds.h = 1.;
            });
            let b = e.selected_views()[0].bounds;
            let scene = e.rendered_scene(e.page).unwrap();
            assert!(!scene.items.is_empty());
            assert!(scene.items.iter().all(|i| i.clip == Some(b)));
            assert!(scene.svg().contains("clipPath"));
            assert!(scene.png().unwrap().starts_with(b"\x89PNG"));
        }
    }
}
#[test]
fn webcam_component_resize_is_local_undoable_and_persistent() {
    let mut e = Editor::new(Project::default());
    e.add(Kind::Webcam, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    e.create_component("Portrait".into()).unwrap();
    let defs = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    e.select(child, false);
    let before = e.document.clone();
    e.begin("Resize portrait");
    e.selected_mut(|c| {
        c.bounds.w = 300.;
        c.bounds.h = 73.;
    });
    e.commit();
    let after = e.document.clone();
    assert_eq!(after.definitions, defs);
    assert_eq!(
        (
            e.selected_views()[0].bounds.w,
            e.selected_views()[0].bounds.h
        ),
        (300., 73.)
    );
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(
        document::decode(&e.document.encode().unwrap()).unwrap(),
        after
    );
}
#[test]
fn media_fills_are_convex_for_native_canvas_tessellation() {
    use wireframe::{render::Drawing, scene::Primitive};
    for kind in KINDS {
        for sketch in [false, true] {
            let mut e = Editor::new(Project::default());
            e.document.settings.sketch = sketch;
            e.add(kind, 0., 0.);
            for item in e.rendered_scene(e.page).unwrap().items {
                if let Drawing::Primitive(Primitive::Polygon { points, .. }) = item.drawing {
                    let mut sign = 0.;
                    for i in 0..points.len() {
                        let a = points[i];
                        let b = points[(i + 1) % points.len()];
                        let c = points[(i + 2) % points.len()];
                        let cross = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
                        if cross.abs() > 0.0001 {
                            if sign != 0. {
                                assert_eq!(cross.signum(), sign, "{kind:?} {points:?}");
                            }
                            sign = cross.signum();
                        }
                    }
                }
            }
        }
    }
}
