// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    model::{Bounds, Kind},
    render::{Drawing, Scene},
    scene::Primitive,
    workspace::Editor,
};
fn thumb(scene: &Scene) -> Option<(Bounds, [u8; 3])> {
    scene.items.iter().find_map(|i| match &i.drawing {
        Drawing::Primitive(Primitive::Rect { b, fill, .. }) if b.w == 8. && b.h == 21. => {
            Some((*b, *fill))
        }
        _ => None,
    })
}
fn glyphs(scene: &Scene) -> Vec<(String, f32, f32)> {
    scene
        .items
        .iter()
        .filter_map(|i| match &i.drawing {
            Drawing::Glyph(g) => Some((g.text.clone(), g.x, g.baseline)),
            _ => None,
        })
        .collect()
}
#[test]
fn embedded_scrollbars_keep_size_text_and_history_while_moving_the_thumb() {
    for sketch in [true, false] {
        for kind in [Kind::Dropdown, Kind::TextArea] {
            let mut e = Editor::default();
            e.document.settings.sketch = sketch;
            e.add(kind, 10., 20.);
            let id = e.nodes()[0].id;
            e.set_control_property(
                id,
                "text",
                Value::Text("Choose\nAlpha\nBeta\nGamma\nDelta".into()),
            )
            .unwrap();
            let bounds = e.current().controls[0].bounds;
            assert!(thumb(&e.rendered_scene(1).unwrap()).is_none());
            e.begin("Show scrollbar");
            e.set_control_property(id, "vertical_scrollbar", Value::Bool(true))
                .unwrap();
            e.commit();
            assert_eq!(e.current().controls[0].bounds, bounds);
            assert_eq!(
                e.control_properties(id).unwrap().1["value"],
                Value::Number(0.)
            );
            let start = e.rendered_scene(1).unwrap();
            let text = glyphs(&start);
            let top = thumb(&start).unwrap().0;
            let height = if kind == Kind::Dropdown {
                bounds.h - if sketch { 27. } else { 24. } - 2.
            } else {
                bounds.h - if sketch { 4. } else { 0. }
            };
            for value in [0.5, 1.] {
                e.begin("Move scrollbar");
                e.set_control_property(id, "value", Value::Number(value))
                    .unwrap();
                e.commit();
                let scene = e.rendered_scene(1).unwrap();
                let moved = thumb(&scene).unwrap().0;
                assert_eq!(moved.y, top.y + (height - 48.) * value as f32);
                assert_eq!(glyphs(&scene), text);
                assert_eq!(e.current().controls[0].bounds, bounds);
            }
            e.undo();
            assert_eq!(
                e.control_properties(id).unwrap().1["value"],
                Value::Number(0.5)
            );
            e.redo();
            let bytes = e.document.encode().unwrap();
            let reopened = Editor::new(decode(&bytes).unwrap());
            assert_eq!(
                reopened.rendered_scene(1).unwrap().svg(),
                e.rendered_scene(1).unwrap().svg()
            );
            for state in ["normal", "focused", "disabled"] {
                e.set_control_property(id, "state", Value::Choice(state.into()))
                    .unwrap();
                let color = thumb(&e.rendered_scene(1).unwrap()).unwrap().1;
                assert_eq!(
                    color,
                    if !sketch {
                        [81; 3]
                    } else if state == "disabled" {
                        [128, 128, 139]
                    } else {
                        [0; 3]
                    }
                );
            }
            if kind == Kind::Dropdown {
                for state in ["closed", "focused_closed", "disabled_closed"] {
                    e.set_control_property(id, "state", Value::Choice(state.into()))
                        .unwrap();
                    assert!(thumb(&e.rendered_scene(1).unwrap()).is_none());
                }
                e.set_control_property(id, "state", Value::Choice("normal".into()))
                    .unwrap();
                e.set_control_property(id, "text", Value::Text("Title only".into()))
                    .unwrap();
                assert!(thumb(&e.rendered_scene(1).unwrap()).is_none());
            }
        }
    }
}
#[test]
fn textarea_short_track_threshold_and_reserved_text_width() {
    for sketch in [true, false] {
        let mut e = Editor::default();
        e.document.settings.sketch = sketch;
        e.add(Kind::TextArea, 10., 20.);
        let id = e.nodes()[0].id;
        e.set_control_property(id, "vertical_scrollbar", Value::Bool(true))
            .unwrap();
        for height in [45., 46.] {
            if let NodeContent::Control { size, .. } =
                &mut e.document.pages[0].content.nodes[0].content
            {
                size.height = Dimension::Fixed(height + if sketch { 4. } else { 0. });
            }
            assert_eq!(thumb(&e.rendered_scene(1).unwrap()).is_some(), height > 45.);
        }
        if let NodeContent::Control { size, .. } = &mut e.document.pages[0].content.nodes[0].content
        {
            size.height = Dimension::Fixed(140.);
            size.width = Dimension::Fixed(100.);
        }
        e.set_control_property(
            id,
            "text",
            Value::Text("[Alpha Beta Gamma Delta Epsilon](page:1)".into()),
        )
        .unwrap();
        let enabled = e.rendered_scene(1).unwrap();
        let track_left = 10. + 100. - 16. - if sketch { 2. } else { 0. };
        assert!(!enabled.links.is_empty());
        assert!(
            enabled
                .links
                .iter()
                .all(|l| l.bounds.x + l.bounds.w <= track_left)
        );
        e.set_control_property(id, "vertical_scrollbar", Value::Bool(false))
            .unwrap();
        let disabled = e.rendered_scene(1).unwrap();
        assert!(glyphs(&enabled).last().unwrap().2 >= glyphs(&disabled).last().unwrap().2);
        assert_ne!(glyphs(&enabled), glyphs(&disabled));
    }
}
