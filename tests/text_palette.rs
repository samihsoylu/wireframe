// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{self, Value},
    model::Kind,
    render::{Drawing, Scene},
    workspace::Editor,
};

fn fixture(kind: Kind, sketch: bool, text: &str) -> (Editor, u64) {
    let mut e = Editor::default();
    e.document.settings.sketch = sketch;
    e.add(kind, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    e.set_control_property(id, "text", Value::Text(text.into()))
        .unwrap();
    e.set_control_property(id, "font_size", Value::Number(24.))
        .unwrap();
    (e, id)
}
fn colors(scene: &Scene) -> Vec<(&str, [u8; 4])> {
    scene
        .items
        .iter()
        .filter_map(|item| match &item.drawing {
            Drawing::Glyph(g) => Some((g.text.as_str(), g.color)),
            _ => None,
        })
        .collect()
}
#[test]
fn text_and_links_follow_style_and_disabled_state_in_scene_and_raster() {
    for sketch in [true, false] {
        for kind in [Kind::Label, Kind::Paragraph, Kind::Link] {
            let (mut e, id) = fixture(kind, sketch, "Text [Next](page:2)");
            let scene = e.rendered_scene(e.page).unwrap();
            let ink = if kind == Kind::Link {
                [8, 83, 148, 255]
            } else if sketch {
                [0, 0, 0, 255]
            } else {
                [51, 51, 51, 255]
            };
            let runs = colors(&scene);
            assert!(runs.iter().any(|(s, c)| s.contains("Text") && *c == ink));
            assert!(
                runs.iter()
                    .any(|(s, c)| *s == "Next" && *c == [8, 83, 148, 255])
            );
            let png = image::load_from_memory(&scene.png().unwrap())
                .unwrap()
                .into_rgba8();
            assert!(png.pixels().any(|p| p.0 == ink));
            assert!(png.pixels().any(|p| p.0 == [8, 83, 148, 255]));
            let links = scene.links.clone();
            e.set_control_property(id, "state", Value::Choice("disabled".into()))
                .unwrap();
            let disabled = e.rendered_scene(e.page).unwrap();
            assert!(
                colors(&disabled)
                    .iter()
                    .all(|(_, c)| *c == [128, 128, 139, 255])
            );
            assert_eq!(disabled.bounds, scene.bounds);
            assert_eq!(disabled.links.len(), links.len());
        }
    }
}
#[test]
fn explicit_colors_and_inline_markup_survive_style_history_and_roundtrip() {
    for kind in [Kind::Label, Kind::Paragraph] {
        let (mut e, id) = fixture(
            kind,
            false,
            "Text {color:#d22b3c}Accent{color} [Next](page:2)",
        );
        let clean = e.rendered_scene(e.page).unwrap();
        assert!(
            colors(&clean)
                .iter()
                .any(|(s, c)| *s == "Accent" && *c == [210, 43, 60, 255])
        );
        let original = e.document.clone();
        e.edit("Set text color", |e| {
            e.set_control_property(id, "text_color", Value::Color([0, 0, 0, 255]))
                .unwrap()
        });
        let explicit = e.document.clone();
        e.edit("Change style", |e| e.document.settings.sketch = true);
        e.edit("Change style", |e| e.document.settings.sketch = false);
        let scene = e.rendered_scene(e.page).unwrap();
        assert_eq!(scene.bounds, clean.bounds);
        assert!(
            colors(&scene)
                .iter()
                .any(|(s, c)| s.contains("Text") && *c == [0, 0, 0, 255])
        );
        assert!(
            colors(&scene)
                .iter()
                .any(|(s, c)| *s == "Accent" && *c == [210, 43, 60, 255])
        );
        assert_eq!(e.document, explicit);
        let bytes = e.document.encode().unwrap();
        assert_eq!(document::decode(&bytes).unwrap(), explicit);
        e.undo();
        e.undo();
        e.undo();
        assert_eq!(e.document, original);
        assert_eq!(e.rendered_scene(e.page).unwrap().svg(), clean.svg());
    }
}
