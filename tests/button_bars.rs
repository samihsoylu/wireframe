// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    button_bar::ButtonBar,
    document::*,
    model::{Bounds, Kind},
    render::Drawing,
    scene::Primitive,
    workspace::Editor,
};
fn edit(e: &mut Editor, id: Id, key: &str, value: Value) {
    e.begin("Edit segments");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
fn fixture() -> (Editor, Id, Id) {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    let first = e.page;
    e.add_page();
    let dest = e.page;
    e.switch_page(first);
    e.add(Kind::ButtonBar, 50., 50.);
    let id = *e.selection.iter().next().unwrap();
    (e, id, dest)
}
fn bounds(e: &Editor) -> Bounds {
    e.selected_views()[0].bounds
}
fn resize(e: &mut Editor, w: f32, h: f32) {
    e.begin("Resize bar");
    e.selected_mut(|c| {
        c.bounds.w = w;
        c.bounds.h = h;
    });
    e.commit();
}
#[test]
fn comma_segments_keep_spaces_empty_entries_multiline_and_visual_fallback() {
    let parsed = ButtonBar::parse("A,  B,,C\nD").unwrap();
    assert_eq!(
        parsed
            .texts
            .iter()
            .map(|s| s.display_text(true))
            .collect::<Vec<_>>(),
        ["A", " B", "", "C\nD"]
    );
    assert_eq!(
        ButtonBar::parse("One").unwrap().texts[1].display_text(true),
        "..."
    );
    assert_eq!(wireframe::button_bar::labels("One"), ["One"]);
    let (mut e, id, _) = fixture();
    assert_eq!(
        e.control_properties(id).unwrap().1["selected_index"],
        Value::Number(0.)
    );
    let clean = bounds(&e);
    assert_eq!(clean.h, 21.);
    e.document.settings.sketch = true;
    assert_eq!(bounds(&e).w, clean.w);
    assert_eq!(bounds(&e).h, 27.);
    edit(
        &mut e,
        id,
        "text",
        Value::Text("First\nline, Second".into()),
    );
    assert_eq!(bounds(&e).h, 43.);
    resize(&mut e, 210., 50.);
    assert_eq!(bounds(&e), Bounds::new(50., 50., 210., 50.));
    let saved = e.document.clone();
    e.begin("Auto-size");
    e.auto_size_control(id).unwrap();
    e.commit();
    assert_eq!(bounds(&e).h, 43.);
    e.undo();
    assert_eq!(e.document, saved);
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), saved);
    let old = e.document.clone();
    assert!(
        e.set_control_property(id, "text", Value::Text(",".repeat(10002)))
            .is_err()
    );
    assert_eq!(e.document, old);
}
#[test]
fn narrow_bars_omit_entire_segments_and_their_links_in_canvas_and_exports() {
    let (mut e, id, dest) = fixture();
    edit(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![
            Some(Link::Back),
            Some(Link::Page(dest)),
            Some(Link::Url("https://example.org/third".into())),
        ]),
    );
    edit(&mut e, id, "selected_index", Value::Number(1.));
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        resize(&mut e, 80., 30.);
        let s = e.rendered_scene(e.page).unwrap();
        assert_eq!(s.links.len(), 1);
        assert_eq!(s.hit_link(55., 55.), Some("back:"));
        assert_eq!(s.hit_link(120., 55.), None);
        assert!(!s.items.iter().any(|i| matches!(
            &i.drawing,
            Drawing::Primitive(Primitive::Rect {
                fill: [149, 188, 242],
                ..
            })
        )));
        assert!(!s.svg().contains("href=\"https://example.org/third\""));
        resize(&mut e, 100., 30.);
        let s = e.rendered_scene(e.page).unwrap();
        assert_eq!(s.links.len(), 2);
        assert!(s.links[1].target == format!("page:{dest}"));
        assert!(s.items.iter().any(|i| matches!(
            &i.drawing,
            Drawing::Primitive(Primitive::Rect {
                fill: [149, 188, 242],
                ..
            })
        )));
        resize(&mut e, 210., 50.);
        let s = e.rendered_scene(e.page).unwrap();
        assert_eq!(s.links.len(), 3);
        assert!((s.links.iter().map(|l| l.bounds.w).sum::<f32>() - 210.).abs() < 0.001);
        assert!(s.png().is_ok());
        assert!(s.svg().contains("href=\"https://example.org/third\""));
        resize(&mut e, 20., 50.);
        let s = e.rendered_scene(e.page).unwrap();
        assert!(s.links.is_empty());
        assert!(
            !s.items
                .iter()
                .any(|i| matches!(i.drawing, Drawing::Glyph(_)))
        );
    }
}
#[test]
fn segment_metadata_survives_continuous_comma_edits_and_canvas_edits() {
    let (mut e, id, dest) = fixture();
    edit(
        &mut e,
        id,
        "text",
        Value::Text("Same, Same,Other\nline".into()),
    );
    let links = vec![
        Some(Link::Back),
        Some(Link::Page(dest)),
        Some(Link::Url("https://example.org".into())),
    ];
    edit(&mut e, id, "row_links", Value::Links(links.clone()));
    edit(&mut e, id, "selected_index", Value::Number(2.));
    let original = e.document.clone();
    e.begin("Continuous edit");
    e.set_control_property(id, "text", Value::Text("".into()))
        .unwrap();
    e.set_control_property(id, "text", Value::Text("Other\nline, Same,Same,New".into()))
        .unwrap();
    e.commit();
    let p = e.control_properties(id).unwrap().1;
    assert_eq!(
        p["row_links"],
        Value::Links(vec![
            links[2].clone(),
            links[0].clone(),
            links[1].clone(),
            None
        ])
    );
    assert_eq!(p["selected_index"], Value::Number(0.));
    e.undo();
    assert_eq!(e.document, original);
    e.redo();
    e.begin("Canvas text");
    e.selected_mut(|c| c.text = "Other\nline, Same,Same,Renamed".into());
    e.commit();
    assert_eq!(
        e.control_properties(id).unwrap().1["row_links"],
        p["row_links"]
    );
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
}
#[test]
fn segment_links_respect_component_reset_clipboard_and_page_deletion() {
    let (mut e, id, dest) = fixture();
    edit(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![Some(Link::Page(dest)), None, Some(Link::Back)]),
    );
    e.create_component("Navigation".into()).unwrap();
    let source = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    edit(&mut e, child, "text", Value::Text("Three, Two, One".into()));
    assert_eq!(e.document.definitions, source);
    assert_eq!(
        e.control_properties(child).unwrap().1["row_links"],
        Value::Links(vec![Some(Link::Back), None, Some(Link::Page(dest))])
    );
    e.reset_local_changes(child).unwrap();
    assert_eq!(
        e.control_properties(child).unwrap().1["text"],
        Value::Text("One, Two, Three".into())
    );
    e.leave();
    e.select(id, false);
    let clipboard = e.copy();
    let mut other = Editor::default();
    other.paste(&clipboard, 0.);
    assert!(other.error.is_none());
    assert!(other.document.encode().is_ok());
    e.switch_page(dest);
    e.delete_page();
    assert!(e.document.encode().is_ok());
    e.undo();
    assert_eq!(e.document.definitions, source);
}
