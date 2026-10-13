// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    model::Kind,
    render::{Drawing, Scene},
    scene::Primitive,
    workspace::Editor,
};

fn microphone(scene: &Scene) -> bool {
    scene.items.iter().any(|i| matches!(&i.drawing, Drawing::Primitive(Primitive::Rect {b,..}) if b.w == 4. && b.h == 7.5))
}

#[test]
fn search_defaults_states_shapes_and_icons_share_geometry_and_survive_storage() {
    for sketch in [true, false] {
        let mut e = Editor::default();
        e.document.settings.sketch = sketch;
        e.add(Kind::Search, 10., 20.);
        let id = e.nodes()[0].id;
        let initial = e.current().controls[0].clone();
        assert_eq!(initial.text, "search");
        assert_eq!(
            (initial.bounds.w, initial.bounds.h),
            (120., if sketch { 25. } else { 26. })
        );
        for state in ["normal", "focused", "disabled"] {
            e.set_control_property(id, "state", Value::Choice(state.into()))
                .unwrap();
            for shape in ["rounded", "rectangle"] {
                e.set_control_property(id, "shape", Value::Choice(shape.into()))
                    .unwrap();
                let s = e.rendered_scene(1).unwrap();
                let expected = match state {
                    "disabled" => [128, 128, 139],
                    "focused" if sketch => [149, 188, 242],
                    "focused" => [151, 189, 240],
                    _ => [0, 0, 0],
                };
                assert!(
                    matches!(&s.items.last().unwrap().drawing, Drawing::Primitive(Primitive::Line {color,..}) if *color==expected)
                );
                assert_eq!(e.current().controls[0].bounds, initial.bounds);
                let glyph = s
                    .items
                    .iter()
                    .find_map(|i| {
                        if let Drawing::Glyph(g) = &i.drawing {
                            Some(g)
                        } else {
                            None
                        }
                    })
                    .unwrap();
                assert_eq!(glyph.x, 32.);
                assert_eq!(
                    glyph.color,
                    if sketch || state == "disabled" {
                        [128, 128, 139, 255]
                    } else {
                        [51, 51, 51, 255]
                    }
                );
                assert!(!microphone(&s));
            }
        }
        e.begin("Configure search icons");
        e.set_control_property(id, "search_icon", Value::Bool(false))
            .unwrap();
        e.set_control_property(id, "microphone_icon", Value::Bool(true))
            .unwrap();
        e.commit();
        let s = e.rendered_scene(1).unwrap();
        assert!(microphone(&s));
        assert_eq!(e.current().controls[0].bounds, initial.bounds);
        assert!(
            s.items
                .iter()
                .any(|i| matches!(&i.drawing,Drawing::Glyph(g) if g.x==16.))
        );
        let reopened = Editor::new(decode(&e.document.encode().unwrap()).unwrap());
        assert_eq!(e.current().controls, reopened.current().controls);
        e.undo();
        assert!(!microphone(&e.rendered_scene(1).unwrap()));
        e.redo();
        assert!(microphone(&e.rendered_scene(1).unwrap()));
    }
}

#[test]
fn microphone_requires_more_than_eighteen_units_of_remaining_space() {
    let mut e = Editor::default();
    e.add(Kind::Search, 0., 0.);
    let id = e.nodes()[0].id;
    e.set_control_property(id, "microphone_icon", Value::Bool(true))
        .unwrap();
    for icon in [true, false] {
        e.set_control_property(id, "search_icon", Value::Bool(icon))
            .unwrap();
        e.auto_size_control(id).unwrap();
        e.set_control_property(
            id,
            "text",
            Value::Text("Find a project by its title".into()),
        )
        .unwrap();
        let width = e.current().controls[0].bounds.w;
        assert!(width > 120.);
        // Above the minimum width, the automatic frame is measured text + 26.
        let text_width = width - 26.;
        let left = if icon { 22. } else { 6. };
        for (extra, visible) in [(18., false), (19., true)] {
            if let NodeContent::Control { size, .. } =
                &mut e.document.pages[0].content.nodes[0].content
            {
                size.width = Dimension::Fixed(left + text_width + extra);
            }
            assert_eq!(
                microphone(&e.rendered_scene(1).unwrap()),
                visible,
                "icon={icon}, extra={extra}"
            );
        }
    }
}
