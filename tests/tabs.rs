// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    model::{Bounds, Kind},
    render::Drawing,
    scene::Primitive,
    tabs,
    workspace::Editor,
};
fn fixture(kind: Kind) -> (Editor, Id, Id) {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    let page = e.page;
    e.add_page();
    let dest = e.page;
    e.switch_page(page);
    e.add(kind, 50., 50.);
    let id = *e.selection.iter().next().unwrap();
    (e, id, dest)
}
fn set(e: &mut Editor, id: Id, key: &str, v: Value) {
    e.begin("Edit tabs");
    e.set_control_property(id, key, v).unwrap();
    e.commit();
}
fn text(e: &mut Editor, id: Id, s: &str) {
    set(e, id, "text", Value::Text(s.into()));
}
fn b(e: &Editor) -> Bounds {
    e.selected_views()[0].bounds
}
fn resize(e: &mut Editor, w: f32, h: f32) {
    e.begin("Resize tabs");
    e.selected_mut(|c| {
        c.bounds.w = w;
        c.bounds.h = h;
    });
    e.commit();
}
#[test]
fn intrinsic_labels_empty_slots_styles_and_fixed_dimensions() {
    assert_eq!(
        tabs::labels(" A\\,B,  C,,D", false),
        [" A,B", " C", "", "D"]
    );
    assert_eq!(tabs::labels(" A\n\nC", true), [" A", "", "C"]);
    for (kind, size) in [
        (Kind::Tabs, (150., 100.)),
        (Kind::VerticalTabs, (200., 100.)),
    ] {
        let (mut e, id, _) = fixture(kind);
        assert!(e.auto_size_control(id).is_err());
        text(&mut e, id, "");
        assert_eq!((b(&e).w, b(&e).h), size);
        assert_eq!(
            e.control_properties(id).unwrap().1["text"],
            Value::Text("".into())
        );
        text(
            &mut e,
            id,
            if kind == Kind::Tabs {
                "First\nline, Second"
            } else {
                "A\n\nC"
            },
        );
        if kind == Kind::VerticalTabs {
            assert_eq!(b(&e).h, 149.);
            e.document.settings.sketch = true;
            assert_eq!(b(&e).h, 152.);
        }
        resize(&mut e, 50., 60.);
        assert!((b(&e).w - 50.).abs() < 0.001 && (b(&e).h - 60.).abs() < 0.001);
        let doc = e.document.clone();
        assert!(
            e.set_control_property(id, "text", Value::Text(",\n".repeat(10002)))
                .is_err()
        );
        assert_eq!(e.document, doc);
        assert_eq!(decode(&doc.encode().unwrap()).unwrap(), doc);
    }
}
#[test]
fn hidden_tab_links_appear_only_after_enlargement_and_clips_limit_hits() {
    for (kind, selected) in [(Kind::Tabs, 2), (Kind::VerticalTabs, 3)] {
        let (mut e, id, dest) = fixture(kind);
        set(&mut e, id, "selected_index", Value::Number(selected as f64));
        let mut links = vec![None; 4];
        links[selected] = Some(Link::Page(dest));
        set(&mut e, id, "row_links", Value::Links(links));
        resize(
            &mut e,
            if kind == Kind::Tabs { 90. } else { 200. },
            if kind == Kind::Tabs { 100. } else { 60. },
        );
        assert!(e.rendered_scene(e.page).unwrap().links.is_empty());
        let before = e.document.clone();
        resize(&mut e, 150., 150.);
        let scene = e.rendered_scene(e.page).unwrap();
        assert_eq!(scene.links.len(), 1);
        let hit = scene.links[0].bounds;
        assert_eq!(
            scene.hit_link(hit.x + 1., hit.y + 1.),
            Some(format!("page:{dest}").as_str())
        );
        e.undo();
        assert_eq!(e.document, before);
        e.redo();
        assert_eq!(e.rendered_scene(e.page).unwrap().links.len(), 1);
        e.create_component("Tabs".into()).unwrap();
        let bounds = b(&e);
        e.document.pages[0].content.nodes[0].crop = Some(Crop {
            x: 0.,
            y: 0.,
            width: 0.5,
            height: 0.5,
        });
        let scene = e.rendered_scene(e.page).unwrap();
        for link in scene.links {
            assert!(link.bounds.x + link.bounds.w <= bounds.x + bounds.w * 0.5 + 0.001);
            assert!(link.bounds.y + link.bounds.h <= bounds.y + bounds.h * 0.5 + 0.001);
        }
    }
}
#[test]
fn position_alignment_and_vertical_label_clipping_are_shared_by_exports() {
    let (mut e, id, _) = fixture(Kind::Tabs);
    text(&mut e, id, "A, B");
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![Some(Link::Back); 2]),
    );
    resize(&mut e, 230., 130.);
    let left = e.rendered_scene(e.page).unwrap().links[0].bounds;
    assert_eq!(left.x, 70.);
    assert_eq!(left.y, 50.);
    set(&mut e, id, "align", Value::Choice("right".into()));
    let right = e.rendered_scene(e.page).unwrap();
    assert!(right.links[0].bounds.x > left.x);
    assert_eq!(
        right.links[1].bounds.x + right.links[1].bounds.w,
        50. + 230. - 20. - 24. - 1.
    );
    set(&mut e, id, "tab_position", Value::Choice("bottom".into()));
    let scene = e.rendered_scene(e.page).unwrap();
    assert_eq!(scene.links[0].bounds.y + scene.links[0].bounds.h, 180.);
    assert!(scene.png().is_ok());
    assert!(scene.svg().contains("<clipPath"));
    let (mut e, id, _) = fixture(Kind::VerticalTabs);
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![Some(Link::Back); 4]),
    );
    resize(&mut e, 50., 150.);
    let scene = e.rendered_scene(e.page).unwrap();
    assert_eq!(scene.links.len(), 4);
    assert!(scene.links.iter().all(|l| l.bounds.x + l.bounds.w <= 100.));
    set(&mut e, id, "tab_position", Value::Choice("right".into()));
    assert!(
        e.rendered_scene(e.page)
            .unwrap()
            .links
            .iter()
            .all(|l| l.bounds.x >= 50.)
    );
}
#[test]
fn selection_fill_opacity_border_scrollbar_and_inline_links() {
    let (mut e, id, _) = fixture(Kind::Tabs);
    text(&mut e, id, "[One](https://example.org/inline), Two");
    set(&mut e, id, "selected_index", Value::Number(1.));
    set(&mut e, id, "fill", Value::Color([0, 0, 0, 255]));
    for (opacity, color) in [(0.5, [51, 51, 51, 255]), (1., [255; 4])] {
        set(&mut e, id, "background_opacity", Value::Number(opacity));
        let s = e.rendered_scene(e.page).unwrap();
        assert!(
            s.items
                .iter()
                .any(|i| matches!(&i.drawing,Drawing::Glyph(g) if g.color==color))
        );
        assert!(s.links.is_empty());
        assert!(s.items.iter().any(|i| i.opacity == opacity as f32));
    }
    set(&mut e, id, "border", Value::Choice("none".into()));
    let s = e.rendered_scene(e.page).unwrap();
    assert!(
        !s.items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Primitive(Primitive::Rect { .. })))
    );
    set(&mut e, id, "vertical_scrollbar", Value::Bool(true));
    assert!(e.rendered_scene(e.page).unwrap().items.len() > s.items.len());
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![
            None,
            Some(Link::Url("https://example.org/tab".into())),
        ]),
    );
    let s = e.rendered_scene(e.page).unwrap();
    assert!(s.svg().contains("href=\"https://example.org/tab\""));
    assert!(!s.svg().contains("href=\"https://example.org/inline\""));
}
#[test]
fn continuous_reorder_undo_components_copy_and_page_deletion() {
    for kind in [Kind::Tabs, Kind::VerticalTabs] {
        let (mut e, id, dest) = fixture(kind);
        let sep = if kind == Kind::Tabs { ", " } else { "\n" };
        let original = ["One", "Two", "Three"].join(sep);
        let reordered = ["Three", "One", "Two"].join(sep);
        text(&mut e, id, &original);
        set(&mut e, id, "selected_index", Value::Number(2.));
        set(
            &mut e,
            id,
            "row_links",
            Value::Links(vec![None, None, Some(Link::Page(dest))]),
        );
        let before = e.document.clone();
        e.begin("Type new labels");
        e.set_control_property(id, "text", Value::Text("".into()))
            .unwrap();
        e.set_control_property(id, "text", Value::Text(reordered.clone()))
            .unwrap();
        e.commit();
        let p = e.control_properties(id).unwrap().1;
        assert_eq!(p["selected_index"], Value::Number(0.));
        assert_eq!(
            p["row_links"],
            Value::Links(vec![Some(Link::Page(dest)), None, None])
        );
        e.undo();
        assert_eq!(e.document, before);
        e.redo();
        e.create_component("Tabs".into()).unwrap();
        let source = e.document.definitions.clone();
        assert!(e.enter(id));
        let child = e.nodes()[0].id;
        text(&mut e, child, "Only");
        assert_eq!(e.document.definitions, source);
        e.reset_local_changes(child).unwrap();
        assert_eq!(
            e.control_properties(child).unwrap().1["text"],
            Value::Text(reordered)
        );
        e.leave();
        e.select(id, false);
        let clip = e.copy();
        let mut other = Editor::default();
        other.paste(&clip, 0.);
        assert!(other.error.is_none());
        assert!(other.document.encode().is_ok());
        e.switch_page(dest);
        e.delete_page();
        assert!(e.document.encode().is_ok());
        e.undo();
        assert_eq!(e.document.definitions, source);
    }
}
