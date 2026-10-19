// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, workspace::Editor};
fn selection(kinds: &[&str]) -> Editor {
    let mut project = Project {
        next_id: kinds.len() as u64 + 2,
        ..Project::default()
    };
    project.pages[0].content.nodes = kinds
        .iter()
        .enumerate()
        .map(|(i, kind)| {
            Node::control(
                i as u64 + 2,
                *kind,
                Point::new(i as f32 * 200., 50.),
                Size::fixed(160., 260.),
            )
        })
        .collect();
    let mut editor = Editor::new(project);
    editor.select_all();
    editor
}
fn keys(editor: &Editor) -> Vec<&'static str> {
    editor
        .selection_properties()
        .into_iter()
        .filter(|p| p.specialized())
        .map(|p| p.key)
        .collect()
}
#[test]
fn specialized_fields_obey_family_boundaries_even_when_choice_enums_match() {
    for (kind, expected) in [
        (
            "phone",
            vec![
                "device_model",
                "orientation",
                "transparent_background",
                "status_bar",
                "background_pattern",
                "device_resizing",
            ],
        ),
        (
            "tablet",
            vec![
                "orientation",
                "transparent_background",
                "status_bar",
                "device_resizing",
            ],
        ),
        (
            "smartphone",
            vec![
                "orientation",
                "transparent_background",
                "status_bar",
                "device_resizing",
            ],
        ),
        ("mobile_keyboard", vec!["keyboard_device", "orientation"]),
        ("label", vec!["text_orientation"]),
        ("progress", vec!["value", "progress_style", "indeterminate"]),
        (
            "window",
            vec![
                "value",
                "header_height",
                "footer_height",
                "minimize",
                "maximize",
                "close",
                "resize_handle",
                "vertical_scrollbar",
            ],
        ),
        ("search", vec!["shape", "search_icon", "microphone_icon"]),
    ] {
        assert_eq!(keys(&selection(&[kind, kind])), expected, "{kind}");
    }
    for kinds in [
        ["phone", "tablet"],
        ["tablet", "smartphone"],
        ["phone", "mobile_keyboard"],
        ["horizontal_slider", "progress"],
        ["list", "table"],
        ["tabs", "vertical_tabs"],
    ] {
        let keys = keys(&selection(&kinds));
        assert!(
            !keys.iter().any(|k| matches!(
                *k,
                "orientation"
                    | "status_bar"
                    | "device_resizing"
                    | "slider_thumb"
                    | "header"
                    | "row_height"
                    | "tab_position"
            )),
            "{kinds:?}: {keys:?}"
        );
    }
    assert_eq!(
        keys(&selection(&["horizontal_slider", "vertical_slider"])),
        ["slider_thumb", "value"]
    );
    assert_eq!(
        keys(&selection(&["horizontal_scrollbar", "vertical_scrollbar"])),
        ["value"]
    );
}
#[test]
fn device_and_label_changes_preserve_explicit_geometry_and_exact_history() {
    for (kind, key, value) in [
        ("phone", "orientation", Value::Choice("landscape".into())),
        (
            "mobile_keyboard",
            "keyboard_device",
            Value::Choice("tablet".into()),
        ),
        ("label", "text_orientation", Value::Choice("left".into())),
    ] {
        let mut editor = selection(&[kind, kind]);
        editor.current_mut().content.nodes[1].locked = true;
        let before = editor.document.clone();
        editor.begin("Shared property");
        editor.set_selection_property(key, value.clone()).unwrap();
        editor.commit();
        for node in editor.nodes().iter() {
            assert_eq!(
                editor.control_properties(node.id).unwrap().1.get(key),
                Some(&value)
            );
            let old = before.pages[0]
                .content
                .nodes
                .iter()
                .find(|n| n.id == node.id)
                .unwrap();
            assert_eq!(node.origin, old.origin);
            assert_eq!(node.content, old.content);
            assert_eq!(node.locked, old.locked);
        }
        let after = editor.document.clone();
        editor.undo();
        assert_eq!(editor.document, before);
        editor.redo();
        assert_eq!(editor.document, after);
        assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
    }
}
#[test]
fn slider_preview_updates_share_one_history_entry_and_rollback_on_invalid_value() {
    let mut editor = selection(&["horizontal_slider", "vertical_slider"]);
    let before = editor.document.clone();
    editor.begin("Adjust sliders");
    for value in [0.2, 0.4, 60. / 99.] {
        editor
            .set_selection_property("value", Value::Number(value))
            .unwrap();
    }
    editor.commit();
    let after = editor.document.clone();
    assert_eq!(
        editor.control_properties(2).unwrap().1["value"],
        Value::Number(60. / 99.)
    );
    assert_eq!(
        editor.control_properties(3).unwrap().1["value"],
        Value::Number(60. / 99.)
    );
    editor.undo();
    assert_eq!(editor.document, before);
    editor.redo();
    assert_eq!(editor.document, after);
    assert!(
        editor
            .set_selection_property("value", Value::Number(f64::NAN))
            .is_err()
    );
    assert_eq!(editor.document, after);
    editor.begin("Cancel preview");
    editor
        .set_selection_property("slider_thumb", Value::Choice("circle".into()))
        .unwrap();
    editor.cancel();
    assert_eq!(editor.document, after);
}
#[test]
fn specialized_reset_and_unavailable_mixed_write_do_not_damage_document() {
    let mut editor = selection(&["table", "table"]);
    editor.begin("Table rows");
    editor
        .set_selection_property("header", Value::Bool(false))
        .unwrap();
    editor
        .set_selection_property("row_height", Value::Number(42.))
        .unwrap();
    editor.commit();
    let before = editor.document.clone();
    editor.begin("Reset row height");
    editor.reset_selection_property("row_height").unwrap();
    editor.commit();
    assert!(
        editor
            .nodes()
            .iter()
            .all(|n| !n.properties.contains_key("row_height"))
    );
    editor.undo();
    assert_eq!(editor.document, before);
    let mut editor = selection(&["phone", "tablet"]);
    let before = editor.document.clone();
    assert!(
        editor
            .set_selection_property("orientation", Value::Choice("landscape".into()))
            .is_err()
    );
    assert_eq!(editor.document, before);
}
