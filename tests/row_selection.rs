// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    workspace::{Editor, selectable_row_labels},
};
fn specimen(kinds: &[&str], texts: &[&str]) -> Editor {
    let mut project = Project {
        next_id: kinds.len() as u64 + 2,
        ..Project::default()
    };
    project.pages[0].content.nodes = kinds
        .iter()
        .zip(texts)
        .enumerate()
        .map(|(i, (kind, text))| {
            let mut node = Node::control(
                i as u64 + 2,
                *kind,
                Point::new(i as f32 * 220., 50.),
                Size::fixed(200., 150.),
            );
            node.properties
                .insert("text".into(), Value::Text((*text).into()));
            node.properties
                .insert("selected_index".into(), Value::Number(0.));
            node
        })
        .collect();
    let mut editor = Editor::new(project);
    editor.select_all();
    editor
}
fn index(e: &Editor, id: Id) -> f64 {
    match e.control_properties(id).unwrap().1["selected_index"] {
        Value::Number(v) => v,
        _ => panic!(),
    }
}
#[test]
fn labels_follow_click_order_and_mixed_rows_have_no_common_value() {
    let mut e = specimen(&["list", "list"], &["Alpha\nBeta\nGamma", "One\nTwo"]);
    let rows = e.row_selection().unwrap();
    assert_eq!(rows.labels, ["Alpha / One", "Beta / Two", "Gamma / -"]);
    assert!(rows.allow_none);
    assert!(!rows.mixed);
    assert_eq!(rows.value, 0);
    e.select(3, false);
    e.select(2, true);
    assert_eq!(
        e.row_selection().unwrap().labels,
        ["One / Alpha", "Two / Beta", "- / Gamma"]
    );
    e.begin("Row");
    e.set_selection_row(2).unwrap();
    e.commit();
    assert_eq!((index(&e, 2), index(&e, 3)), (2., 0.));
    let rows = e.row_selection().unwrap();
    assert!(rows.mixed);
    assert_eq!(rows.value, 0);
}
#[test]
fn missing_rows_are_skipped_and_history_is_exact() {
    let mut e = specimen(
        &["list", "list", "dropdown"],
        &["Alpha\nBeta\nGamma", "One\nTwo", "Choose\nRed\nBlue"],
    );
    e.document.pages[0].content.nodes[0].locked = true;
    let before = e.document.clone();
    assert!(!e.row_selection().unwrap().allow_none);
    for invalid in [-2, -1, 3, i32::MAX] {
        assert!(e.set_selection_row(invalid).is_err());
        assert_eq!(e.document, before);
    }
    e.begin("Row");
    e.set_selection_row(2).unwrap();
    e.commit();
    assert_eq!((index(&e, 2), index(&e, 3), index(&e, 4)), (2., 0., 2.));
    assert_eq!(
        e.document.pages[0].content.nodes[1],
        before.pages[0].content.nodes[1]
    );
    let after = e.document.clone();
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
    e.undo();
    assert_eq!(e.document, before);
    e.begin("Same row");
    e.set_selection_row(0).unwrap();
    e.commit();
    assert!(e.redo_name().is_some());
    e.redo();
    assert_eq!(e.document, after);
    e.select(2, false);
    e.select(3, true);
    e.begin("None");
    e.set_selection_row(-1).unwrap();
    e.commit();
    assert_eq!((index(&e, 2), index(&e, 3), index(&e, 4)), (-1., -1., 2.));
    e.undo();
    assert_eq!(e.document, after);
}
#[test]
fn table_rows_exclude_header_sizes_and_empty_rows_but_preserve_choices_and_escaped_cells() {
    let mut e = specimen(
        &["table", "table"],
        &[
            "Name ^,Ready\nA\\, B,[x]\n\nC,(o)\n{70L,30C}",
            "Item v,State\nOne,[]",
        ],
    );
    e.set_control_property(2, "header", Value::Bool(true))
        .unwrap();
    e.set_control_property(3, "header", Value::Bool(false))
        .unwrap();
    assert_eq!(
        e.row_selection().unwrap().labels,
        ["A, B, [x] / Item, State", "C, (o) / One, []"]
    );
    assert_eq!(
        e.row_selection().unwrap().captions(),
        ["A, B, x / Item, State", "C, (o) / One, []"]
    );
    e.set_selection_row(1).unwrap();
    assert_eq!((index(&e, 2), index(&e, 3)), (1., 1.));
    let (_, mut values) = e.control_properties(2).unwrap();
    values.insert("text".into(), Value::Text("\n{100}".into()));
    assert!(selectable_row_labels("table", &values).unwrap().is_empty());
}
#[test]
fn shared_rows_require_all_controls_and_component_edits_remain_local() {
    let e = specimen(&["list", "label"], &["One\nTwo", "Label"]);
    assert!(e.row_selection().is_none());
    let mut e = specimen(&["list", "list"], &["One\nTwo", "A\nB"]);
    e.group();
    let definition = e.create_component("Lists".into()).unwrap();
    let instance = *e.selection.iter().next().unwrap();
    e.enter(instance);
    e.select_all();
    let before = e.document.clone();
    e.begin("Row");
    e.set_selection_row(1).unwrap();
    e.commit();
    assert_eq!(e.document.definitions, before.definitions);
    assert_eq!(
        e.document.component_source(definition),
        before.component_source(definition)
    );
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    e.begin("Reset");
    e.reset_selection_row().unwrap();
    e.commit();
    assert!(!e.row_selection().unwrap().explicit);
    e.undo();
    assert_eq!(e.document, after);
}
#[test]
fn every_row_family_is_available_and_empty_dropdown_title_is_an_item() {
    for kind in [
        "list",
        "dropdown",
        "table",
        "accordion",
        "tree",
        "menu",
        "menu_bar",
        "tabs",
        "vertical_tabs",
        "link_bar",
        "button_bar",
    ] {
        let e = specimen(&[kind, kind], &["First\nSecond", "Third\nFourth"]);
        assert!(e.row_selection().is_some(), "{kind}");
    }
    let e = specimen(
        &["dropdown", "list"],
        &["\n\nChoice\n  \nLast", "First\nSecond"],
    );
    assert_eq!(
        e.row_selection().unwrap().labels,
        [" / First", "Choice / Second", "Last / -"]
    );
}
