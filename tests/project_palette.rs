// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{self, Palette, Project, Value},
    model::Kind,
    render::{Drawing, Scene},
    workspace::Editor,
};
fn ink(s: &Scene, text: &str) -> [u8; 4] {
    s.items
        .iter()
        .rev()
        .find_map(|i| match &i.drawing {
            Drawing::Glyph(g) if g.text == text => Some(g.color),
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing {text}"))
}
#[test]
fn legacy_documents_omit_default_palette_and_custom_colors_round_trip() {
    let mut p = Project::default();
    let legacy = p.encode().unwrap();
    assert!(!String::from_utf8_lossy(&legacy).contains("palette"));
    assert_eq!(document::decode(&legacy).unwrap(), p);
    p.settings.palette = Palette {
        selection: [32; 3],
        link: [139, 0, 139],
    };
    assert_eq!(document::decode(&p.encode().unwrap()).unwrap(), p);
    let mut json: serde_json::Value = serde_json::from_slice(&legacy).unwrap();
    json["settings"]["palette"] = serde_json::json!({"selection":[256,0,0]});
    assert!(document::decode(&serde_json::to_vec(&json).unwrap()).is_err());
    json["settings"]["palette"] = serde_json::json!({"link":[1,2]});
    assert!(document::decode(&serde_json::to_vec(&json).unwrap()).is_err());
}
#[test]
fn selected_families_share_project_colors_and_boundary_contrast() {
    for sketch in [false, true] {
        for (kind, text, selected, label, menu) in [
            (
                Kind::Table,
                "Heading\n[Selected](https://example.test/)",
                0,
                "Selected",
                false,
            ),
            (
                Kind::List,
                "[Selected](https://example.test/)\nOther",
                0,
                "Selected",
                false,
            ),
            (
                Kind::Tree,
                "v Parent\n- [Selected](https://example.test/)",
                1,
                "Selected",
                false,
            ),
            (
                Kind::Button,
                "[Selected](https://example.test/)",
                -1,
                "Selected",
                false,
            ),
            (
                Kind::Dropdown,
                "Choose\n[Selected](https://example.test/)",
                1,
                "Selected",
                true,
            ),
            (
                Kind::Accordion,
                "[Selected](https://example.test/)\nOther",
                0,
                "Selected",
                false,
            ),
        ] {
            let mut e = Editor::default();
            e.document.settings.sketch = sketch;
            e.add(kind, 20., 20.);
            let id = *e.selection.iter().next().unwrap();
            e.set_control_property(id, "text", Value::Text(text.into()))
                .unwrap();
            if selected >= 0 {
                e.set_control_property(id, "selected_index", Value::Number(selected as f64))
                    .unwrap();
            }
            if kind == Kind::Button {
                e.set_control_property(id, "state", Value::Choice("selected".into()))
                    .unwrap();
            }
            e.selected_mut(|n| {
                n.bounds.w = 250.;
                n.bounds.h = 130.;
            });
            for level in [32, 152, 153, 255] {
                e.document.settings.palette = Palette {
                    selection: [level; 3],
                    link: [139, 0, 139],
                };
                let s = e.rendered_scene(e.page).unwrap();
                let expected = if level < 153 {
                    [255; 4]
                } else if menu {
                    [0, 0, 0, 255]
                } else {
                    [139, 0, 139, 255]
                };
                assert_eq!(
                    ink(&s, label),
                    expected,
                    "{kind:?} sketch={sketch} level={level}"
                );
            }
        }
    }
}
#[test]
fn project_color_edit_updates_all_pages_and_undo_preserves_nodes() {
    let mut e = Editor::default();
    for i in 0..2 {
        if i > 0 {
            e.add_page();
        }
        e.add(Kind::Link, 30., 30.);
        let id = *e.selection.iter().next().unwrap();
        e.set_control_property(id, "text", Value::Text("Open".into()))
            .unwrap();
    }
    e.mark_saved();
    let old = e.document.clone();
    e.edit("Change colors", |e| {
        e.document.settings.palette.link = [139, 0, 139]
    });
    for p in &e.document.pages {
        assert_eq!(
            ink(&e.rendered_scene(p.id).unwrap(), "Open"),
            [139, 0, 139, 255]
        );
    }
    assert_eq!(e.document.pages, old.pages);
    let changed = e.document.clone();
    assert!(e.dirty());
    e.undo();
    assert_eq!(e.document, old);
    assert!(!e.dirty());
    e.redo();
    assert_eq!(e.document, changed);
}
#[test]
fn explicit_and_disabled_text_survive_palette_changes() {
    let mut e = Editor::default();
    e.document.settings.palette = Palette {
        selection: [32; 3],
        link: [139, 0, 139],
    };
    e.add(Kind::Button, 0., 0.);
    let id = *e.selection.iter().next().unwrap();
    e.set_control_property(
        id,
        "text",
        Value::Text("Plain [Open](https://example.test/)".into()),
    )
    .unwrap();
    e.set_control_property(id, "state", Value::Choice("selected".into()))
        .unwrap();
    e.set_control_property(id, "text_color", Value::Color([200, 10, 10, 255]))
        .unwrap();
    let s = e.rendered_scene(e.page).unwrap();
    assert_eq!(ink(&s, "Plain "), [200, 10, 10, 255]);
    assert_eq!(ink(&s, "Open"), [255; 4]);
    e.set_control_property(id, "state", Value::Choice("disabled".into()))
        .unwrap();
    let s = e.rendered_scene(e.page).unwrap();
    assert_eq!(ink(&s, "Plain "), [128, 128, 139, 255]);
    assert_eq!(ink(&s, "Open"), [128, 128, 139, 255]);
}
