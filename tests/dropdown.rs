// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    dropdown::{Mark, Menu, selectable_labels},
    model::Kind,
    render::Drawing,
    scene::Primitive,
    workspace::Editor,
};
const SOURCE: &str = "Choose\nAlpha\n\nx Beta\no Gamma >\nDelta, literal";

#[test]
fn dropdown_rows_preserve_selection_mapping_markers_commas_and_separators() {
    let menu = Menu::parse(SOURCE, 3).unwrap();
    assert_eq!(menu.title, "Gamma");
    assert_eq!(
        selectable_labels(SOURCE),
        ["Choose", "Alpha", "x Beta", "o Gamma >", "Delta, literal"]
    );
    assert_eq!(menu.rows.len(), 5);
    assert!(menu.rows[1].separator);
    assert_eq!(menu.rows[2].mark, Mark::Check);
    assert_eq!(menu.rows[3].mark, Mark::Dot);
    assert!(menu.rows[3].selected && menu.rows[3].arrow);
    assert_eq!(menu.rows[4].text, "Delta, literal");
    assert_eq!(Menu::parse(SOURCE, 50).unwrap().title, "");
    let edge = Menu::parse("\n---\n = = \n\t\nx Label >>>", 1).unwrap();
    assert_eq!(edge.title, "---");
    assert!(edge.rows[0].separator && edge.rows[1].separator);
    assert!(!edge.rows[2].separator);
    assert!(edge.rows[3].arrow);
    assert!(Menu::parse(&"row\n".repeat(10_002), 0).is_err());
}

#[test]
fn dropdown_open_closed_states_use_menu_height_palette_and_selected_title() {
    for sketch in [true, false] {
        let mut e = Editor::default();
        e.document.settings.sketch = sketch;
        e.add(Kind::Dropdown, 10., 20.);
        let id = e.nodes()[0].id;
        e.set_control_property(id, "text", Value::Text(SOURCE.into()))
            .unwrap();
        e.set_control_property(id, "selected_index", Value::Number(3.))
            .unwrap();
        for state in [
            "normal",
            "focused",
            "disabled",
            "closed",
            "focused_closed",
            "disabled_closed",
        ] {
            e.set_control_property(id, "state", Value::Choice(state.into()))
                .unwrap();
            let closed = state.ends_with("closed");
            let h = if closed {
                if sketch { 27. } else { 24. }
            } else if sketch {
                106.
            } else {
                98.
            };
            assert_eq!(
                e.current().controls[0].bounds.h,
                h,
                "{state} sketch={sketch}"
            );
            let s = e.rendered_scene(1).unwrap();
            let text = s
                .items
                .iter()
                .filter_map(|i| {
                    if let Drawing::Glyph(g) = &i.drawing {
                        Some(g.text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<String>();
            assert!(text.starts_with("Gamma"));
            assert_eq!(text == "Gamma", closed);
            let expected = if state.starts_with("focused") {
                if sketch {
                    [149, 188, 242]
                } else {
                    [151, 189, 240]
                }
            } else if state.starts_with("disabled") {
                if sketch { [137; 3] } else { [204; 3] }
            } else {
                [0; 3]
            };
            let color = s
                .items
                .iter()
                .find_map(|i| {
                    if let Drawing::Primitive(Primitive::Line { color, .. }) = &i.drawing {
                        Some(*color)
                    } else {
                        None
                    }
                })
                .unwrap();
            assert_eq!(color, expected);
            assert!(s.items.iter().all(|i| i.clip.is_some()));
        }
        e.set_control_property(id, "state", Value::Choice("normal".into()))
            .unwrap();
        e.begin("Change selection");
        e.set_control_property(id, "selected_index", Value::Number(1.))
            .unwrap();
        e.commit();
        e.undo();
        assert_eq!(
            e.control_properties(id).unwrap().1["selected_index"],
            Value::Number(3.)
        );
        let bytes = e.document.encode().unwrap();
        let reopened = Editor::new(decode(&bytes).unwrap());
        assert_eq!(
            reopened.rendered_scene(1).unwrap().svg(),
            e.rendered_scene(1).unwrap().svg()
        );
        assert_eq!(e.current().controls[0].text, SOURCE);
    }
}
