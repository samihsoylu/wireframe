// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, workspace::Editor};
fn lists() -> Editor {
    let mut p = Project {
        next_id: 4,
        ..Project::default()
    };
    p.pages[0].content.nodes = (0..2)
        .map(|i| {
            let mut n = Node::control(
                i + 2,
                "list",
                Point::new(i as f32 * 240., 50.),
                Size::fixed(200., 150.),
            );
            n.properties.insert(
                "text".into(),
                Value::Text(
                    if i == 0 {
                        "Header\nBeta\nGamma"
                    } else {
                        "One\nTwo"
                    }
                    .into(),
                ),
            );
            n
        })
        .collect();
    p.pages[0].content.nodes[0]
        .properties
        .insert("header".into(), Value::Bool(true));
    let mut e = Editor::new(p);
    e.select(3, false);
    e.select(2, true);
    e
}
fn icon() -> Option<IconSource> {
    Some(IconSource::Catalog("backward-step-solid".into()))
}
fn set(e: &mut Editor, row: usize, value: Option<IconSource>) {
    let ids = e.ordered_selection();
    e.begin("Row icon");
    e.set_list_row_icon(&ids, row, value).unwrap();
    e.commit();
}
#[test]
fn row_icons_follow_click_order_include_headers_and_skip_missing_rows() {
    let mut e = lists();
    let ids = e.ordered_selection();
    let rows = e.list_row_icons(&ids).unwrap();
    assert_eq!(
        rows.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(),
        ["One / Header", "Two / Beta", "- / Gamma"]
    );
    assert_eq!(rows[2].targets, vec![2]);
    let before = e.document.clone();
    set(&mut e, 2, icon());
    assert_eq!(
        e.document.pages[0].content.nodes[1],
        before.pages[0].content.nodes[1]
    );
    assert_eq!(e.list_row_icons(&ids).unwrap()[2].icon, icon());
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    set(&mut e, 0, None);
    assert!(e.redo_name().is_some());
    e.redo();
    assert_eq!(e.document, after);
    set(&mut e, 0, icon());
    assert!(e.list_row_icons(&ids).unwrap()[0].icon.is_some());
    let after = e.document.clone();
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
    set(&mut e, 2, None);
    assert_eq!(e.list_row_icons(&ids).unwrap()[2].icon, None);
    e.undo();
    assert_eq!(e.document, after);
}
#[test]
fn row_icons_preserve_other_slots_and_reject_bad_targets_without_writes() {
    let mut e = lists();
    set(&mut e, 0, icon());
    let before = e.document.clone();
    for (ids, row, value) in [
        (vec![2, 999], 0, icon()),
        (vec![2, 3], 99, icon()),
        (
            vec![2, 3],
            1,
            Some(IconSource::Catalog("missing-unavailable-icon".into())),
        ),
        (vec![2, 3], 1, Some(IconSource::Asset(999))),
    ] {
        assert!(e.set_list_row_icon(&ids, row, value).is_err());
        assert_eq!(e.document, before);
    }
    e.document.pages[0].content.nodes[0].locked = true;
    set(&mut e, 1, Some(IconSource::Catalog("star-solid".into())));
    for id in [2, 3] {
        let (_, p) = e.control_properties(id).unwrap();
        let Value::Icons(icons) = &p["row_icons"] else {
            panic!()
        };
        assert_eq!(icons[0], icon());
        assert_eq!(icons[1], Some(IconSource::Catalog("star-solid".into())));
    }
}
#[test]
fn row_icon_component_overrides_do_not_change_sources_and_history_is_exact() {
    let mut e = lists();
    e.group();
    let def = e.create_component("Lists".into()).unwrap();
    let root = *e.selection.iter().next().unwrap();
    e.enter(root);
    e.select_all();
    let before = e.document.clone();
    set(&mut e, 1, icon());
    assert_eq!(e.document.definitions, before.definitions);
    assert_eq!(
        e.document.component_source(def),
        before.component_source(def)
    );
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
}
