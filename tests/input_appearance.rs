// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    model::Kind,
    render::{Drawing, Scene},
    scene::Primitive,
    workspace::Editor,
};
fn color(scene: &Scene) -> [u8; 4] {
    scene
        .items
        .iter()
        .find_map(|i| {
            if let Drawing::Glyph(g) = &i.drawing {
                Some(g.color)
            } else {
                None
            }
        })
        .unwrap()
}
#[test]
fn input_contrast_threshold_explicit_color_reset_and_storage() {
    for sketch in [true, false] {
        for kind in [Kind::TextInput, Kind::TextArea] {
            let mut e = Editor::default();
            e.document.settings.sketch = sketch;
            e.add(kind, 20., 20.);
            let id = e.nodes()[0].id;
            e.set_control_property(
                id,
                "text",
                Value::Text("Text [link](https://example.com)".into()),
            )
            .unwrap();
            e.set_control_property(id, "fill", Value::Color([0, 0, 0, 255]))
                .unwrap();
            let default = if sketch {
                [0, 0, 0, 255]
            } else {
                [51, 51, 51, 255]
            };
            for (alpha, expected) in [
                (0., default),
                (0.5, default),
                (0.749, default),
                (0.75, [255; 4]),
                (1., [255; 4]),
            ] {
                e.set_control_property(id, "background_opacity", Value::Number(alpha))
                    .unwrap();
                let s = e.rendered_scene(1).unwrap();
                assert_eq!(color(&s), expected);
                assert_eq!(s.items[0].opacity, alpha as f32);
                assert!(s.items[1..].iter().all(|i| i.opacity == 1.));
                let colors = s
                    .items
                    .iter()
                    .filter_map(|i| {
                        if let Drawing::Glyph(g) = &i.drawing {
                            Some(g.color)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    *colors.last().unwrap(),
                    if alpha >= 0.75 {
                        [255; 4]
                    } else {
                        [8, 83, 148, 255]
                    }
                );
            }
            // A chosen black remains explicit even if it equals a light-fill default.
            e.set_control_property(id, "background_opacity", Value::Number(0.5))
                .unwrap();
            e.set_control_property(id, "text_color", Value::Color([0, 0, 0, 255]))
                .unwrap();
            e.set_control_property(id, "background_opacity", Value::Number(1.))
                .unwrap();
            assert_eq!(color(&e.rendered_scene(1).unwrap()), [0, 0, 0, 255]);
            e.begin("Reset text color");
            e.reset_control_property(id, "text_color").unwrap();
            e.commit();
            assert_eq!(color(&e.rendered_scene(1).unwrap()), [255; 4]);
            e.undo();
            assert_eq!(color(&e.rendered_scene(1).unwrap()), [0, 0, 0, 255]);
            e.redo();
            for (fill, expected) in [
                ([152, 152, 152, 255], [255; 4]),
                ([153, 153, 153, 255], default),
            ] {
                e.set_control_property(id, "fill", Value::Color(fill))
                    .unwrap();
                assert_eq!(color(&e.rendered_scene(1).unwrap()), expected);
            }
            e.set_control_property(id, "state", Value::Choice("disabled".into()))
                .unwrap();
            assert_eq!(color(&e.rendered_scene(1).unwrap()), [128, 128, 139, 255]);
            let bytes = e.document.encode().unwrap();
            let reopened = Editor::new(decode(&bytes).unwrap());
            assert_eq!(
                reopened.rendered_scene(1).unwrap().svg(),
                e.rendered_scene(1).unwrap().svg()
            );
            assert!(
                e.set_control_property(id, "background_opacity", Value::Number(1.1))
                    .is_err()
            );
        }
    }
}
#[test]
fn input_borders_preserve_geometry_and_textarea_border_draws_after_text() {
    for sketch in [true, false] {
        let mut e = Editor::default();
        e.document.settings.sketch = sketch;
        e.add(Kind::TextInput, 20., 20.);
        let id = e.nodes()[0].id;
        e.set_control_property(id, "text", Value::Text("Example".into()))
            .unwrap();
        e.set_control_property(id, "border_color", Value::Color([190, 30, 70, 255]))
            .unwrap();
        let size = e.current().controls[0].bounds;
        let before = e.rendered_scene(1).unwrap();
        let first_x = |s: &Scene| {
            s.items
                .iter()
                .find_map(|i| {
                    if let Drawing::Glyph(g) = &i.drawing {
                        Some(g.x)
                    } else {
                        None
                    }
                })
                .unwrap()
        };
        e.set_control_property(id, "border", Value::Choice("line".into()))
            .unwrap();
        let after = e.rendered_scene(1).unwrap();
        assert_eq!(e.current().controls[0].bounds, size);
        assert_eq!(
            first_x(&before) - first_x(&after),
            if sketch { 7. } else { 5. }
        );
        for state in ["normal", "focused", "disabled"] {
            e.set_control_property(id, "state", Value::Choice(state.into()))
                .unwrap();
            let s = e.rendered_scene(1).unwrap();
            let (points, color) = s
                .items
                .iter()
                .find_map(|i| {
                    if let Drawing::Primitive(Primitive::Line { points, color, .. }) = &i.drawing {
                        Some((points, *color))
                    } else {
                        None
                    }
                })
                .unwrap();
            assert_eq!(points.len(), 2);
            assert_eq!(points[0][1], points[1][1]);
            assert_eq!(
                color,
                match state {
                    "normal" => [190, 30, 70],
                    "disabled" => [128, 128, 139],
                    _ => [149, 188, 242],
                }
            );
        }
        let mut area = Editor::default();
        area.add(Kind::TextArea, 0., 0.);
        let id = area.nodes()[0].id;
        area.set_control_property(id, "text", Value::Text("Text".into()))
            .unwrap();
        let s = area.rendered_scene(1).unwrap();
        assert!(matches!(
            s.items.last().unwrap().drawing,
            Drawing::Primitive(Primitive::Line { .. } | Primitive::Mesh { .. })
        ));
    }
}
#[test]
fn translucent_fill_composites_over_underlying_content_in_raster_export() {
    let mut e = Editor::default();
    e.add(Kind::TextArea, 0., 0.);
    let back = e.nodes()[0].id;
    e.set_control_property(back, "fill", Value::Color([255, 0, 0, 255]))
        .unwrap();
    e.add(Kind::TextArea, 0., 0.);
    let front = e.nodes()[1].id;
    e.set_control_property(front, "fill", Value::Color([0, 0, 255, 255]))
        .unwrap();
    e.set_control_property(front, "background_opacity", Value::Number(0.5))
        .unwrap();
    let s = e.rendered_scene(1).unwrap();
    assert!(s.svg().contains("<g opacity=\"0.5\">"));
    let png = resvg::tiny_skia::Pixmap::decode_png(&s.png().unwrap()).unwrap();
    let px = png.pixel(100, 100).unwrap();
    assert!(px.red().abs_diff(127) <= 1);
    assert_eq!(px.green(), 0);
    assert!(px.blue().abs_diff(128) <= 1);
    assert_eq!(px.alpha(), 255);
}
