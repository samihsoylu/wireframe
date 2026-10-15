// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::Value, model::Kind, render::Drawing, scene::Primitive, workspace::Editor,
};

fn fixture(kind: Kind, width: f32, height: f32, value: f64, state: Option<&str>) -> Editor {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    e.add(kind, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    e.set_control_property(id, "value", Value::Number(value))
        .unwrap();
    if kind == Kind::TextArea {
        e.set_control_property(id, "text", Value::Text(String::new()))
            .unwrap();
        e.set_control_property(id, "vertical_scrollbar", Value::Bool(true))
            .unwrap();
    }
    if let Some(state) = state {
        e.set_control_property(id, "state", Value::Choice(state.into()))
            .unwrap();
    }
    e.selected_mut(|c| {
        c.bounds.w = width;
        c.bounds.h = height;
    });
    e
}

#[test]
fn clean_standalone_and_embedded_tracks_have_identical_pixels_including_disabled_fields() {
    for value in [0., 0.5, 1.] {
        let bar = fixture(Kind::VerticalScrollbar, 16., 165., value, None);
        let s = bar.rendered_scene(bar.page).unwrap();
        let a = image::load_from_memory(&s.png().unwrap())
            .unwrap()
            .into_rgb8();
        let a = image::imageops::crop_imm(&a, 24, 24, 16, 165).to_image();
        for state in ["normal", "focused", "disabled"] {
            let e = fixture(Kind::TextArea, 220., 165., value, Some(state));
            let s = e.rendered_scene(e.page).unwrap();
            let b = image::load_from_memory(&s.png().unwrap())
                .unwrap()
                .into_rgb8();
            let b = image::imageops::crop_imm(&b, 228, 24, 16, 165).to_image();
            // Translated vector rasterization can differ by one channel level.
            let maximum = a
                .as_raw()
                .iter()
                .zip(b.as_raw())
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            assert!(
                maximum <= 1,
                "{state} at {value}: max channel difference {maximum}"
            );
        }
    }
}

#[test]
fn clean_bars_have_inside_pixel_frames_capsule_thumbs_and_open_arrow_compartments() {
    for vertical in [true, false] {
        let (kind, w, h) = if vertical {
            (Kind::VerticalScrollbar, 16., 165.)
        } else {
            (Kind::HorizontalScrollbar, 165., 16.)
        };
        let e = fixture(kind, w, h, 0., None);
        let s = e.rendered_scene(e.page).unwrap();
        let im = image::load_from_memory(&s.png().unwrap())
            .unwrap()
            .into_rgb8();
        let pixel = |along: u32, cross: u32| {
            if vertical {
                im.get_pixel(24 + cross, 24 + along).0
            } else {
                im.get_pixel(24 + along, 24 + cross).0
            }
        };
        for (a, c) in [
            (0, 0),
            (0, 15),
            (164, 0),
            (164, 15),
            (0, 8),
            (164, 8),
            (80, 0),
            (80, 15),
            (6, 7),
            (20, 7),
        ] {
            assert_eq!(pixel(a, c), [81; 3]);
        }
        for (a, c) in [(12, 2), (152, 2), (13, 4), (16, 2), (80, 8)] {
            assert_eq!(pixel(a, c), [255; 3]);
        }
        let arrow = s
            .items
            .iter()
            .find_map(|i| match &i.drawing {
                Drawing::Primitive(Primitive::Polygon { points, .. }) => Some(points),
                _ => None,
            })
            .unwrap();
        let leading = arrow
            .iter()
            .map(|p| p[if vertical { 1 } else { 0 }])
            .fold(f32::INFINITY, f32::min);
        assert_eq!(leading, if vertical { 34. } else { 23. });
        let thumb = s
            .items
            .iter()
            .find_map(|i| match &i.drawing {
                Drawing::Primitive(Primitive::Rect {
                    b, radius, fill, ..
                }) if (b.w, b.h) == if vertical { (8., 21.) } else { (21., 8.) } => {
                    Some((*radius, *fill))
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(thumb, (4., [81; 3]));
    }
}

#[test]
fn short_clean_bars_keep_the_thumb_threshold_and_finite_export_geometry() {
    for vertical in [false, true] {
        for length in [1., 10., 24., 45., 46., 47., 48.] {
            let (kind, w, h) = if vertical {
                (Kind::VerticalScrollbar, 16., length)
            } else {
                (Kind::HorizontalScrollbar, length, 16.)
            };
            let e = fixture(kind, w, h, 0.5, None);
            let s = e.rendered_scene(e.page).unwrap();
            let n=s.items.iter().filter(|i|matches!(&i.drawing,Drawing::Primitive(Primitive::Rect{radius,..}) if *radius==4.)).count();
            assert_eq!(n, usize::from(length > 45.));
            let svg = s.svg();
            assert!(!svg.contains("NaN") && !svg.contains("inf"));
            assert!(image::load_from_memory(&s.png().unwrap()).is_ok());
        }
    }
}
