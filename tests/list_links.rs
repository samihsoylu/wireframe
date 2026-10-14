// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, model::Kind, workspace::Editor};
fn fixture() -> (Editor, Id, Id) {
    let mut e = Editor::default();
    let first = e.page;
    e.add_page();
    let destination = e.page;
    e.switch_page(first);
    e.add(Kind::List, 50., 50.);
    let id = *e.selection.iter().next().unwrap();
    (e, id, destination)
}
fn set(e: &mut Editor, id: Id, key: &str, value: Value) {
    e.begin("Edit row links");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
fn links(e: &Editor, id: Id) -> Vec<Option<Link>> {
    match &e.control_properties(id).unwrap().1["row_links"] {
        Value::Links(v) => v.clone(),
        _ => panic!(),
    }
}
#[test]
fn row_metadata_tracks_occurrences_rename_delete_and_continuous_input() {
    let (mut e, id, dest) = fixture();
    set(
        &mut e,
        id,
        "text",
        Value::Text("Title\nSame\nSame\nOther".into()),
    );
    let values = vec![
        Some(Link::Page(dest)),
        Some(Link::Back),
        Some(Link::Url("https://example.test".into())),
        None,
    ];
    set(&mut e, id, "row_links", Value::Links(values.clone()));
    let original = e.document.clone();
    e.begin("Edit text");
    e.set_control_property(id, "text", Value::Text(String::new()))
        .unwrap();
    assert!(links(&e, id).is_empty());
    e.set_control_property(
        id,
        "text",
        Value::Text("Title\nSame\nOther\nSame\nSame".into()),
    )
    .unwrap();
    e.commit();
    assert_eq!(
        links(&e, id),
        vec![
            values[0].clone(),
            values[1].clone(),
            None,
            values[2].clone(),
            None
        ]
    );
    e.undo();
    assert_eq!(e.document, original);
    e.redo();
    set(
        &mut e,
        id,
        "text",
        Value::Text("Title\nRenamed\nOther\nSame\nSame".into()),
    );
    assert_eq!(links(&e, id)[1], Some(Link::Back));
    set(&mut e, id, "text", Value::Text("Title\nOther".into()));
    assert_eq!(links(&e, id), vec![values[0].clone(), None]);
    let decoded = wireframe::document::decode(&e.document.encode().unwrap()).unwrap();
    assert_eq!(decoded, e.document);
    assert!(
        e.set_control_property(
            id,
            "row_links",
            Value::Links(vec![Some(Link::Url("javascript:alert(1)".into()))])
        )
        .is_err()
    );
    assert!(
        e.set_control_property(id, "row_links", Value::Links(vec![None; 10002]))
            .is_err()
    );
}
#[test]
fn row_link_regions_cover_background_header_partial_rows_and_scrollbar() {
    let (mut e, id, dest) = fixture();
    e.document.settings.sketch = false;
    set(
        &mut e,
        id,
        "text",
        Value::Text("Header\nA\nB\nC\nD\nE\nF\nG".into()),
    );
    set(&mut e, id, "header", Value::Bool(true));
    set(&mut e, id, "vertical_scrollbar", Value::Bool(true));
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![
            Some(Link::Page(dest)),
            Some(Link::Back),
            Some(Link::Url("https://example.test/?a=1&b=2".into())),
            None,
            None,
            None,
            Some(Link::Back),
            Some(Link::Page(dest)),
        ]),
    );
    let s = e.rendered_scene(e.page).unwrap();
    assert_eq!(s.hit_link(149., 60.), Some(format!("page:{dest}").as_str()));
    assert_eq!(s.hit_link(145., 80.), Some("back:")); // Includes scrollbar column.
    assert_eq!(s.hit_link(55., 100.), Some("https://example.test/?a=1&b=2"));
    assert_eq!(s.hit_link(55., 190.), Some("back:"));
    assert_eq!(s.hit_link(55., 199.), Some(format!("page:{dest}").as_str())); // Three visible pixels of final row.
    assert_eq!(s.hit_link(55., 201.), None); // Below the frame has no hit area.
    let svg = s.svg();
    assert!(svg.contains("href=\"https://example.test/?a=1&amp;b=2\""));
    e.add(Kind::Rectangle, 50., 50.);
    assert_eq!(e.rendered_scene(e.page).unwrap().hit_link(55., 60.), None);
}
#[test]
fn component_links_reset_delete_page_and_cross_document_paste_are_consistent() {
    let (mut e, id, dest) = fixture();
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![Some(Link::Page(dest)), None, None]),
    );
    let definition = e.create_component("Navigation".into()).unwrap();
    let source = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    set(
        &mut e,
        child,
        "row_links",
        Value::Links(vec![None, Some(Link::Back)]),
    );
    assert_eq!(e.document.definitions, source);
    e.reset_local_changes(child).unwrap();
    assert_eq!(links(&e, child)[0], Some(Link::Page(dest)));
    e.leave();
    e.select(id, false);
    let clipboard = e.copy();
    let mut target = Editor::default();
    target.paste(&clipboard, 0.);
    assert!(target.error.is_none(), "{:?}", target.error);
    assert!(target.document.encode().is_ok());
    e.switch_page(dest);
    e.delete_page();
    assert!(e.document.encode().is_ok());
    assert!(e.document.definitions.iter().any(|d| d.id == definition));
    e.undo();
    assert!(e.document.pages.iter().any(|p| p.id == dest));
    assert_eq!(e.document.definitions, source);
    let mut invalid = e.document.clone();
    invalid.pages[0].content.nodes[0]
        .properties
        .insert("bad".into(), Value::Links(vec![Some(Link::Page(999999))]));
    assert!(invalid.encode().is_err());
}
