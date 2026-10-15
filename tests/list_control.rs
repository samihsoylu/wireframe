// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, model::Kind, render::Drawing, scene::Primitive, workspace::Editor};
const ROWS: &str = "Title\nAlpha\n*Bravo*\n-Disabled-\nDelta\nEcho\nFoxtrot\nGolf";
fn add(e: &mut Editor) -> Id {
    e.add(Kind::List, 50., 50.);
    *e.selection.iter().next().unwrap()
}
fn set(e: &mut Editor, id: Id, key: &str, value: Value) {
    e.begin("List property");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
fn text(e: &mut Editor, id: Id, value: &str) {
    set(e, id, "text", Value::Text(value.into()));
}
fn dimensions(e: &Editor) -> (f32, f32) {
    let b = e.current().controls[0].bounds;
    (b.w, b.h)
}
fn selection(e: &Editor, id: Id) -> i32 {
    match e.control_properties(id).unwrap().1["selected_index"] {
        Value::Number(n) => n as i32,
        _ => panic!(),
    }
}
#[test]
fn list_measurement_preserves_header_height_rule_and_fixed_geometry() {
    let mut e = Editor::default();
    let id = add(&mut e);
    assert_eq!(dimensions(&e), (100., 126.));
    for key in [
        "header",
        "row_height",
        "border",
        "alternate_fill",
        "vertical_scrollbar",
        "selected_index",
    ] {
        assert!(
            wireframe::catalog::get("list")
                .unwrap()
                .property(key)
                .unwrap()
                .visible
        );
    }
    text(&mut e, id, ROWS);
    assert_eq!(dimensions(&e), (100., 174.));
    e.document.settings.sketch = false;
    assert_eq!(dimensions(&e), (100., 173.));
    set(&mut e, id, "header", Value::Bool(true));
    assert_eq!(dimensions(&e), (100., 150.));
    set(&mut e, id, "row_height", Value::Number(30.));
    assert_eq!(dimensions(&e), (100., 204.));
    set(&mut e, id, "vertical_scrollbar", Value::Bool(true));
    set(&mut e, id, "border", Value::Choice("none".into()));
    assert_eq!(dimensions(&e), (100., 204.));
    set(&mut e, id, "row_height", Value::Number(1.));
    assert_eq!(dimensions(&e), (100., 150.));
    let mut c = e.current().controls[0].clone();
    c.bounds.w = 70.;
    c.bounds.h = 75.;
    e.begin("Resize");
    e.update_control(c);
    e.commit();
    text(
        &mut e,
        id,
        "A very long row whose width exceeds the frame\n\nLast",
    );
    assert_eq!(dimensions(&e), (70., 75.));
    e.begin("Auto size");
    e.auto_size_control(id).unwrap();
    e.commit();
    assert!(dimensions(&e).0 > 100.);
    assert_eq!(dimensions(&e).1, 126.);
    text(&mut e, id, "");
    assert_eq!(dimensions(&e), (100., 126.));
}
#[test]
fn list_selection_tracks_complete_text_edits_in_inspector_and_canvas() {
    let mut e = Editor::default();
    let id = add(&mut e);
    text(&mut e, id, ROWS);
    set(&mut e, id, "header", Value::Bool(true));
    set(&mut e, id, "selected_index", Value::Number(1.));
    let before = e.document.clone();
    e.begin("Edit rows");
    e.set_control_property(id, "text", Value::Text("".into()))
        .unwrap();
    e.set_control_property(
        id,
        "text",
        Value::Text("Title\nAlpha\n-Disabled-\nDelta\n*Bravo*\nEcho\nFoxtrot\nGolf".into()),
    )
    .unwrap();
    e.commit();
    assert_eq!(selection(&e, id), 3);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(selection(&e, id), 3);
    // Canvas editing enters through the projected control, with the same transaction baseline.
    e.begin("Edit canvas rows");
    let mut c = e.current().controls[0].clone();
    c.text = "".into();
    e.update_control(c);
    let mut c = e.current().controls[0].clone();
    c.text = ROWS.into();
    e.update_control(c);
    e.commit();
    assert_eq!(selection(&e, id), 1);
    text(
        &mut e,
        id,
        "Title\nAlpha\nRenamed\n-Disabled-\nDelta\nEcho\nFoxtrot\nGolf",
    );
    assert_eq!(selection(&e, id), 1);
    text(&mut e, id, "Title\nAlpha\nDelta");
    assert_eq!(selection(&e, id), -1);
    assert_eq!(
        wireframe::list::selection_after_edit("T\nA\nB", "T\nB\nB", 1, true),
        0
    );
    let before = e.document.clone();
    assert!(
        e.set_control_property(id, "text", Value::Text("\n".repeat(10_002)))
            .is_err()
    );
    assert_eq!(e.document, before);
    e.select(id, false);
    e.lock(true);
    let before = e.document.clone();
    assert!(
        e.set_control_property(id, "text", Value::Text("Locked".into()))
            .is_err()
    );
    assert_eq!(e.document, before);
}
#[test]
fn list_rows_share_export_geometry_contrast_clipping_and_scrollbar_positions() {
    let mut e = Editor::default();
    let id = add(&mut e);
    e.document.settings.sketch = false;
    text(&mut e, id, ROWS);
    set(&mut e, id, "header", Value::Bool(true));
    set(&mut e, id, "selected_index", Value::Number(1.));
    set(&mut e, id, "alternate_fill", Value::Color([0, 0, 0, 255]));
    let scene = e.rendered_scene(e.page).unwrap();
    assert!(scene.items.iter().any(|i| matches!(&i.drawing,Drawing::Primitive(Primitive::Rect{b,fill,..}) if b.y==92. && *fill==[149,188,242])));
    let glyphs: Vec<_> = scene
        .items
        .iter()
        .filter_map(|i| {
            if let Drawing::Glyph(g) = &i.drawing {
                Some(g)
            } else {
                None
            }
        })
        .collect();
    assert!(glyphs.iter().any(|g| g.bold && g.text == "Bravo"));
    assert!(glyphs.iter().any(|g| g.color == [153, 153, 153, 255]));
    assert!(glyphs.iter().any(|g| g.color == [255; 4]));
    let rendered: String = glyphs.iter().map(|g| g.text.as_str()).collect();
    assert!(!rendered.contains("Golf"));
    assert!(scene.png().is_ok());
    let baseline: Vec<_> = glyphs
        .iter()
        .map(|g| (g.text.clone(), g.x, g.baseline))
        .collect();
    set(&mut e, id, "vertical_scrollbar", Value::Bool(true));
    set(&mut e, id, "value", Value::Number(1.));
    let end = e.rendered_scene(e.page).unwrap();
    let after: Vec<_> = end
        .items
        .iter()
        .filter_map(|i| {
            if let Drawing::Glyph(g) = &i.drawing {
                Some((g.text.clone(), g.x, g.baseline))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(baseline, after);
    let thumb = |s: &wireframe::render::Scene| {
        s.items
            .iter()
            .find_map(|i| match &i.drawing {
                Drawing::Primitive(Primitive::Rect { b, .. }) if b.w == 8. && b.h == 21. => {
                    Some(b.y)
                }
                _ => None,
            })
            .unwrap()
    };
    set(&mut e, id, "value", Value::Number(0.));
    let start = e.rendered_scene(e.page).unwrap();
    assert!(thumb(&end) > thumb(&start));
    let saved = e.document.encode().unwrap();
    let reopened = Editor::new(wireframe::document::decode(&saved).unwrap());
    assert_eq!(
        start.svg(),
        reopened.rendered_scene(reopened.page).unwrap().svg()
    );
}
#[test]
fn component_list_text_selection_and_reset_remain_local() {
    let mut e = Editor::default();
    let id = add(&mut e);
    text(&mut e, id, "Title\nA\nB");
    set(&mut e, id, "header", Value::Bool(true));
    set(&mut e, id, "selected_index", Value::Number(1.));
    let def = e.create_component("Options".into()).unwrap();
    let second = e.insert_component(def, 300., 50.).unwrap();
    let source = e.document.definitions.clone();
    e.enter(id);
    let child = e.nodes()[0].id;
    text(&mut e, child, "Title\nB\nA");
    assert_eq!(selection(&e, child), 0);
    assert_eq!(e.document.definitions, source);
    text(&mut e, child, "Title\nA");
    assert_eq!(selection(&e, child), -1);
    e.leave();
    e.enter(second);
    assert_eq!(selection(&e, e.nodes()[0].id), 1);
    e.leave();
    e.enter(id);
    e.reset_local_changes(child).unwrap();
    assert_eq!(selection(&e, child), 1);
    assert_eq!(
        e.control_properties(child).unwrap().1["text"],
        Value::Text("Title\nA\nB".into())
    );
}
