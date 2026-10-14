// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    alert::{self, Alert},
    document::*,
    layout::{TextMeasurer, TextMetrics, TextStyle},
    model::{Bounds, Kind},
    render::Drawing,
    workspace::Editor,
};
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&self, text: &str, style: TextStyle, wrap: Option<f32>) -> TextMetrics {
        let widths: Vec<_> = text
            .split('\n')
            .map(|s| s.chars().count() as f32 * 7.)
            .collect();
        let rows: usize = widths
            .iter()
            .map(|w| wrap.map_or(1, |limit| (w / limit).ceil().max(1.) as usize))
            .sum();
        TextMetrics {
            width: widths.iter().copied().fold(0., f32::max),
            height: rows as f32 * style.size,
            rows,
        }
    }
}
const STYLE: TextStyle = TextStyle {
    size: 13.,
    bold: false,
    italic: false,
};
fn fixture() -> (Editor, Id, Id) {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    let page = e.page;
    e.add_page();
    let dest = e.page;
    e.switch_page(page);
    e.add(Kind::Alert, 50., 50.);
    let id = *e.selection.iter().next().unwrap();
    (e, id, dest)
}
fn set(e: &mut Editor, id: Id, key: &str, value: Value) {
    e.begin("Change dialog");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
#[test]
fn grammar_distinguishes_missing_and_blank_fields_and_preserves_literal_commas() {
    let blank = Alert::parse("").unwrap();
    assert_eq!(blank.title.display_text(true), "");
    assert!(blank.message.is_none());
    assert!(blank.buttons.is_empty());
    assert_eq!(blank.measure(&Metrics, STYLE).height, 43.);
    let a = Alert::parse("Title\n\n").unwrap();
    assert_eq!(a.message.unwrap().display_text(true), "");
    assert_eq!(a.buttons.len(), 1);
    assert_eq!(a.buttons[0].source, "");
    assert_eq!(
        alert::labels("Title\nMessage\n One , Two, Ignored\nExtra"),
        ["One", "Two"]
    );
    assert_eq!(
        alert::labels("Title\nMessage\nFirst\\,part,Second"),
        ["First\\", "part"]
    );
    assert_eq!(
        alert::labels("Title\nMessage\n\u{feff}One\u{a0},\u{2003}Two\u{2003}"),
        ["One", "Two"]
    );
    assert_eq!(
        alert::labels("Title\nMessage\n\u{85}One\u{85}"),
        ["\u{85}One\u{85}"]
    );
    assert!(Alert::parse(&"x".repeat(100_001)).is_err());
}
#[test]
fn intrinsic_measurement_and_strict_visibility_thresholds() {
    let a = Alert::parse("Title\nMessage\nLeft,Right").unwrap();
    let m = a.measure(&Metrics, STYLE);
    assert_eq!((m.width, m.height), (200., 102.));
    for (height, title, body, buttons) in [
        (25., false, false, 0),
        (26., true, false, 0),
        (58., true, false, 0),
        (59., true, true, 0),
        (81., true, true, 0),
        (82., true, true, 2),
    ] {
        let l = a.layout(&m, &Metrics, STYLE, 200., height, false);
        assert_eq!(
            (l.title.is_some(), l.message.is_some(), l.buttons.len()),
            (title, body, buttons),
            "height={height}"
        );
    }
    let l = a.layout(&m, &Metrics, STYLE, 200., 102., false);
    assert_eq!(l.message, Some(Bounds::new(10., 36., 180., 13.)));
    assert_eq!(l.divider_y, Some(74.));
    assert!(l.split);
    assert_eq!(l.buttons[0].link, Some(Bounds::new(0., 74., 99., 27.)));
    let a = Alert::parse("Title\\rtwo\nMessage\\rtwo\nLeft\\rtwo,Right").unwrap();
    let m = a.measure(&Metrics, STYLE);
    assert_eq!(m.height, 143.);
    assert_eq!(m.title.height, 30.);
    assert_eq!(m.buttons[0].height, 26.);
    let a = Alert::parse("Title\nAn arbitrarily long message wraps into a taller box without widening the dialog.\nLeft,Right").unwrap();
    let m = a.measure(&Metrics, STYLE);
    assert_eq!(m.width, 200.);
    assert!(m.height > 102.);
}
#[test]
fn clipped_inline_button_keeps_its_destination_in_both_styles() {
    let (mut e, id, dest) = fixture();
    set(
        &mut e,
        id,
        "text",
        Value::Text("Notice\nMessage\nA very long left button,Right".into()),
    );
    set(&mut e, id, "button_style", Value::Choice("inline".into()));
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![Some(Link::Page(dest)), Some(Link::Back)]),
    );
    e.begin("Narrow dialog");
    e.selected_mut(|c| {
        c.bounds.w = 70.;
        c.bounds.h = 190.;
    });
    e.commit();
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        let s = e.rendered_scene(e.page).unwrap();
        assert_eq!(s.links.len(), 1);
        assert_eq!(s.links[0].target, "back:");
        let b = s.links[0].bounds;
        assert!(b.x >= 50. && b.x + b.w <= 120. && b.y + b.h <= 240.);
        assert_eq!(s.hit_link(b.x + b.w / 2., b.y + b.h / 2.), Some("back:"));
        assert!(s.hit_link(85., 90.).is_none());
    }
    e.begin("Short dialog");
    e.selected_mut(|c| c.bounds.h = 40.);
    e.commit();
    assert!(e.rendered_scene(e.page).unwrap().links.is_empty());
}
#[test]
fn colors_are_button_only_and_inline_default_can_be_reset() {
    let (mut e, id, _) = fixture();
    set(
        &mut e,
        id,
        "text",
        Value::Text("Title\nMessage\nLeft,Right".into()),
    );
    set(&mut e, id, "button_style", Value::Choice("inline".into()));
    assert_eq!(
        e.control_properties(id).unwrap().1["text_color"],
        Value::Color([8, 83, 148, 255])
    );
    set(&mut e, id, "text_color", Value::Color([51, 51, 51, 255]));
    assert_eq!(
        e.control_properties(id).unwrap().1["text_color"],
        Value::Color([51, 51, 51, 255])
    );
    set(&mut e, id, "text_color", Value::Color([255, 0, 0, 255]));
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        let s = e.rendered_scene(e.page).unwrap();
        let mut title = false;
        let mut button = false;
        for i in s.items {
            if let Drawing::Glyph(g) = i.drawing {
                if g.text.contains("Title") || g.text.contains("Message") {
                    assert_eq!(
                        g.color,
                        if sketch {
                            [0, 0, 0, 255]
                        } else {
                            [51, 51, 51, 255]
                        }
                    );
                    title = true;
                }
                if g.text.contains("Left") || g.text.contains("Right") {
                    assert_eq!(g.color, [255, 0, 0, 255]);
                    button = true;
                }
            }
        }
        assert!(title && button);
    }
    e.reset_control_property(id, "text_color").unwrap();
    assert_eq!(
        e.control_properties(id).unwrap().1["text_color"],
        Value::Color([8, 83, 148, 255])
    );
    set(&mut e, id, "button_style", Value::Choice("divided".into()));
    assert_eq!(
        e.control_properties(id).unwrap().1["text_color"],
        Value::Color([0, 0, 0, 255])
    );
}
#[test]
fn source_transactions_remap_buttons_and_keep_component_overrides_local() {
    let (mut e, id, dest) = fixture();
    set(
        &mut e,
        id,
        "text",
        Value::Text("Title\nMessage\nKeep,Apply".into()),
    );
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![Some(Link::Back), Some(Link::Page(dest))]),
    );
    let before = e.document.clone();
    e.begin("Replace text");
    e.set_control_property(id, "text", Value::Text("".into()))
        .unwrap();
    e.set_control_property(
        id,
        "text",
        Value::Text("New title\nMessage\nApply,Keep".into()),
    )
    .unwrap();
    e.commit();
    let p = e.control_properties(id).unwrap().1;
    assert_eq!(
        p["row_links"],
        Value::Links(vec![Some(Link::Page(dest)), Some(Link::Back)])
    );
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    e.create_component("Confirmation dialog".into()).unwrap();
    let definitions = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    set(
        &mut e,
        child,
        "text",
        Value::Text("Local\nMessage\nKeep,Apply".into()),
    );
    set(
        &mut e,
        child,
        "button_style",
        Value::Choice("inline".into()),
    );
    assert_eq!(e.document.definitions, definitions);
    e.reset_local_changes(child).unwrap();
    assert_eq!(e.control_properties(child).unwrap().1["text"], p["text"]);
    let before = e.document.clone();
    assert!(
        e.set_control_property(child, "text", Value::Text("x".repeat(100_001)))
            .is_err()
    );
    assert_eq!(e.document, before);
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
}
#[test]
fn manual_resize_and_auto_size_round_trip_through_history() {
    let (mut e, id, _) = fixture();
    let initial = e.selected_views()[0].bounds;
    let original = e.document.clone();
    e.begin("Resize");
    e.selected_mut(|c| {
        c.bounds.w = 90.;
        c.bounds.h = 60.;
    });
    e.commit();
    assert!((e.selected_views()[0].bounds.h - 60.).abs() < 0.0001);
    assert_eq!(e.selected_views()[0].bounds.w, 90.);
    e.undo();
    assert_eq!(e.document, original);
    assert_eq!(e.selected_views()[0].bounds, initial);
    e.redo();
    e.begin("Automatic size");
    e.auto_size_control(id).unwrap();
    e.commit();
    assert_eq!(e.selected_views()[0].bounds, initial);
    e.undo();
    assert!((e.selected_views()[0].bounds.h - 60.).abs() < 0.0001);
    e.redo();
    assert_eq!(e.selected_views()[0].bounds, initial);
    set(&mut e, id, "text", Value::Text("".into()));
    assert!(e.selected_views()[0].bounds.h < initial.h);
}
#[test]
fn formatted_urls_do_not_create_navigation_but_explicit_button_links_do() {
    let (mut e, id, dest) = fixture();
    set(&mut e,id,"text",Value::Text("[Title](https://example.org)\n[Message](https://example.org)\n[Go](https://example.org),Stay".into()));
    assert!(e.rendered_scene(e.page).unwrap().links.is_empty());
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![Some(Link::Page(dest)), None]),
    );
    let s = e.rendered_scene(e.page).unwrap();
    assert_eq!(s.links.len(), 1);
    assert_eq!(s.links[0].target, format!("page:{dest}"));
    assert!(s.png().is_ok());
    assert!(s.svg().contains("clipPath"));
}

#[test]
fn divided_buttons_ignore_global_underline_but_inline_buttons_force_it() {
    use wireframe::scene::Primitive;
    let (mut e, id, _) = fixture();
    set(
        &mut e,
        id,
        "text",
        Value::Text("Title\nMessage\nLeft,Right".into()),
    );
    let rules = |e: &Editor| {
        e.rendered_scene(e.page).unwrap().items.iter().filter(|i|matches!(&i.drawing,Drawing::Primitive(Primitive::Line {points,..}) if points.len()==2 && points[0][1]==points[1][1])).count()
    };
    let plain = rules(&e);
    set(&mut e, id, "underline", Value::Bool(true));
    assert_eq!(rules(&e), plain + 2);
    set(&mut e, id, "button_style", Value::Choice("inline".into()));
    // The horizontal separator disappears and two button underlines appear.
    assert_eq!(rules(&e), plain + 3);
    set(&mut e, id, "underline", Value::Bool(false));
    assert_eq!(rules(&e), plain + 1);
}
