// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::Value, model::Kind, render::Drawing, scene::Primitive, workspace::Editor,
};

fn fixture(sketch: bool) -> Editor {
    let mut e = Editor::default();
    e.document.settings.sketch = sketch;
    e.add(Kind::Table, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    for (key, value) in [
        (
            "text",
            Value::Text(
                "Name ^, Count v, Ready ^v\n[x], [x], [x]\n(o), (o), (o)\n{33L,33C,34R}".into(),
            ),
        ),
        ("font_size", Value::Number(16.)),
        ("header", Value::Bool(true)),
        ("row_height", Value::Number(36.)),
        ("horizontal_lines", Value::Bool(true)),
        ("vertical_lines", Value::Bool(true)),
    ] {
        e.set_control_property(id, key, value).unwrap();
    }
    e.selected_mut(|c| {
        c.bounds.w = 440.;
        c.bounds.h = 185.;
    });
    e
}

#[test]
fn table_grid_has_crisp_style_weighted_separators_and_an_inside_frame() {
    for sketch in [false, true] {
        let e = fixture(sketch);
        let scene = e.rendered_scene(e.page).unwrap();
        let png = image::load_from_memory(&scene.png().unwrap())
            .unwrap()
            .into_rgb8();
        let pixel = |x: u32, y: u32| png.get_pixel(x + 24, y + 24).0;
        let ink = if sketch { [0; 3] } else { [51; 3] };
        for x in [146, 287] {
            assert_eq!(pixel(x, 80), ink);
            assert_eq!(pixel(x - 1, 80), if sketch { ink } else { [255; 3] });
            assert_eq!(pixel(x + 1, 80), [255; 3]);
        }
        for y in [20, 56, 92, 128, 164] {
            assert_eq!(pixel(100, y), ink);
            assert_ne!(pixel(100, y + 1), ink);
            assert_eq!(pixel(100, y - 1) == ink, sketch);
        }
        for (x, y) in [(0, 80), (439, 80), (100, 0), (100, 184)] {
            assert_eq!(pixel(x, y), ink);
        }
        // The frame does not spill outside the control into export padding.
        assert_eq!(png.get_pixel(23, 104).0, [255; 3]);
        assert_eq!(png.get_pixel(464, 104).0, [255; 3]);
    }
}

#[test]
fn table_sort_marks_are_filled_and_choice_alignment_keeps_symbol_sizes() {
    for sketch in [false, true] {
        let e = fixture(sketch);
        let scene = e.rendered_scene(e.page).unwrap();
        let triangles: Vec<_> = scene
            .items
            .iter()
            .filter_map(|i| match &i.drawing {
                Drawing::Primitive(Primitive::Polygon { points, fill }) => Some((points, fill)),
                _ => None,
            })
            .collect();
        assert_eq!(triangles.len(), 4);
        let ink = if sketch { [0; 3] } else { [51; 3] };
        let png = image::load_from_memory(&scene.png().unwrap())
            .unwrap()
            .into_rgb8();
        for (points, fill) in triangles {
            assert_eq!(*fill, ink);
            let x = points.iter().map(|p| p[0]).sum::<f32>() / 3.;
            let y = points.iter().map(|p| p[1]).sum::<f32>() / 3.;
            let px = (x - scene.bounds.x + 24.).floor() as u32;
            let py = (y - scene.bounds.y + 24.).floor() as u32;
            assert_eq!(png.get_pixel(px, py).0, ink);
        }
        let symbols: Vec<_> = scene
            .items
            .iter()
            .filter_map(|i| match &i.drawing {
                Drawing::Icon { bounds, .. } => Some(bounds),
                _ => None,
            })
            .collect();
        assert_eq!(symbols.len(), 6);
        for (i, b) in symbols.iter().enumerate() {
            let size = if i < 3 { 16. } else { 13. };
            assert_eq!((b.w, b.h), (size, size));
            assert_eq!(b.x, [27., 230., 437.][i % 3]);
        }
    }
}
