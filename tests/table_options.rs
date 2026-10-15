// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{self, Dimension, NodeContent, Value},
    model::{Bounds, Kind},
    render::{Drawing, Scene},
    scene::Primitive,
    workspace::Editor,
};

fn fixture(sketch: bool, header: bool, border: bool) -> (Editor, u64) {
    let mut e = Editor::default();
    e.document.settings.sketch = sketch;
    e.add(Kind::Table, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    for (key, value) in [
        ("text", Value::Text("Item ^, Status, More ^v\n[Notebook](https://example.test/), Ready, [x]\nSecond, Waiting, (o)\n{55L,25C,20R}".into())),
        ("font_size", Value::Number(16.)),
        ("row_height", Value::Number(32.)),
        ("header", Value::Bool(header)),
        ("border", Value::Choice(if border { "solid" } else { "none" }.into())),
        ("vertical_lines", Value::Bool(false)),
    ] { e.set_control_property(id, key, value).unwrap(); }
    e.selected_mut(|c| {
        c.bounds.w = 420.;
        c.bounds.h = 165.;
    });
    (e, id)
}
fn thumb(s: &Scene) -> Option<Bounds> {
    s.items.iter().find_map(|i| match &i.drawing {
        Drawing::Primitive(Primitive::Rect {
            b, stroke: false, ..
        }) if b.w == 8. && b.h == 21. => Some(*b),
        _ => None,
    })
}
fn glyphs(s: &Scene) -> Vec<(String, f32, f32)> {
    s.items
        .iter()
        .filter_map(|i| match &i.drawing {
            Drawing::Glyph(g) => Some((g.text.clone(), g.x, g.baseline)),
            _ => None,
        })
        .collect()
}

#[test]
fn table_scroll_thumb_moves_without_scrolling_rows_and_respects_header_and_frame() {
    for sketch in [false, true] {
        for header in [false, true] {
            for border in [false, true] {
                let (mut e, id) = fixture(sketch, header, border);
                assert!(thumb(&e.rendered_scene(e.page).unwrap()).is_none());
                e.set_control_property(id, "vertical_scrollbar", Value::Bool(true))
                    .unwrap();
                let top = e.rendered_scene(e.page).unwrap();
                let first = thumb(&top).unwrap();
                let inset = if sketch && border { 2. } else { 0. };
                assert_eq!(first.x, 428. - inset);
                let padding = if border {
                    if sketch { 2. } else { 1. }
                } else {
                    0.
                };
                assert_eq!(first.y, 43. + padding + if header { 19. } else { 0. });
                e.set_control_property(id, "value", Value::Number(0.5))
                    .unwrap();
                let middle = e.rendered_scene(e.page).unwrap();
                e.set_control_property(id, "value", Value::Number(1.))
                    .unwrap();
                let bottom = e.rendered_scene(e.page).unwrap();
                let last = thumb(&bottom).unwrap();
                assert_eq!(last.y + last.h, 181. - inset);
                assert_eq!(thumb(&middle).unwrap().y, (first.y + last.y) / 2.);
                assert_eq!(glyphs(&top), glyphs(&bottom));
                assert_eq!(top.links.len(), 1);
                assert!(
                    top.links
                        .iter()
                        .all(|l| l.bounds.x + l.bounds.w <= 424. - inset)
                );
            }
        }
    }
}

#[test]
fn table_borderless_output_removes_frame_and_top_padding() {
    for sketch in [false, true] {
        let (mut e, id) = fixture(sketch, false, true);
        let framed = e.rendered_scene(e.page).unwrap();
        e.set_control_property(id, "border", Value::Choice("none".into()))
            .unwrap();
        let plain = e.rendered_scene(e.page).unwrap();
        let a = image::load_from_memory(&framed.png().unwrap())
            .unwrap()
            .into_rgb8();
        let b = image::load_from_memory(&plain.png().unwrap())
            .unwrap()
            .into_rgb8();
        for (x, y) in [(24, 100), (443, 100), (124, 24), (124, 188)] {
            assert_eq!(a.get_pixel(x, y).0, if sketch { [0; 3] } else { [51; 3] });
            assert_eq!(
                b.get_pixel(x, y).0,
                if y == 188 { [238; 3] } else { [255; 3] }
            );
        }
        let shift = if sketch { 2. } else { 1. };
        for ((text, x, y), (t, xx, yy)) in glyphs(&framed).iter().zip(glyphs(&plain)) {
            assert_eq!(text, &t);
            assert_eq!(*x, xx);
            assert_eq!(*y - yy, shift);
        }
    }
}

#[test]
fn table_options_are_editable_persistent_undoable_and_autosize_only_adds_track_width() {
    for sketch in [false, true] {
        let (mut e, id) = fixture(sketch, true, true);
        for key in ["border", "vertical_scrollbar", "value"] {
            assert!(
                wireframe::catalog::get("table")
                    .unwrap()
                    .property(key)
                    .unwrap()
                    .visible
            );
        }
        if let NodeContent::Control { size, .. } = &mut e.document.pages[0].content.nodes[0].content
        {
            size.width = Dimension::Auto;
            size.height = Dimension::Auto;
        }
        let original = e.document.clone();
        let b = e.current().controls[0].bounds;
        e.edit("Table options", |e| {
            e.set_control_property(id, "vertical_scrollbar", Value::Bool(true))
                .unwrap();
            e.set_control_property(id, "border", Value::Choice("none".into()))
                .unwrap();
            e.set_control_property(id, "value", Value::Number(0.5))
                .unwrap();
        });
        let after = e.current().controls[0].bounds;
        assert_eq!(after.w, b.w + 16.);
        assert_eq!(after.h, b.h);
        assert_eq!(
            document::decode(&e.document.encode().unwrap()).unwrap(),
            e.document
        );
        e.undo();
        assert_eq!(e.document, original);
        e.redo();
        assert_eq!(e.current().controls[0].bounds, after);
    }
}
