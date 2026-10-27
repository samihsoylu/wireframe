// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    accordion::{self, Accordion},
    document::*,
    layout::{TextMeasurer, TextMetrics, TextStyle},
    model::{Bounds, Kind},
    render::Drawing,
    workspace::Editor,
};
const SOURCE: &str = "Overview\nPages\n- Landing\n- Settings\nAssets\nNotes";
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&self, text: &str, _: TextStyle, _: Option<f32>) -> TextMetrics {
        TextMetrics {
            width: text.len() as f32 * 7.,
            height: 13.,
            rows: 1,
        }
    }
}
fn fixture() -> (Editor, Id, Id) {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    let page = e.page;
    e.add_page();
    let dest = e.page;
    e.switch_page(page);
    e.add(Kind::Accordion, 50., 50.);
    let id = *e.selection.iter().next().unwrap();
    // Exercise nested-row behavior independently of the catalog starter.
    e.set_control_property(id, "text", Value::Text(SOURCE.into()))
        .unwrap();
    (e, id, dest)
}
fn set(e: &mut Editor, id: Id, key: &str, value: Value) {
    e.begin("Change accordion");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
fn size(e: &Editor) -> Bounds {
    e.selected_views()[0].bounds
}
#[test]
fn grammar_and_selection_retain_source_rows_and_one_level_children() {
    let a = Accordion::parse_source(
        "- Invalid\n\tChild\nRoot\n- \tBoth\n\t- Literal\n  Root\n",
        1,
    )
    .unwrap();
    assert_eq!(a.rows.len(), 7);
    assert_eq!(a.rows[0].source, accordion::FIRST_CHILD_ERROR);
    assert_eq!(a.rows[0].children, 1);
    assert!(a.rows[0].selected && a.rows[1].selected && a.rows[1].visible);
    assert_eq!(a.rows[3].source, "Both");
    assert_eq!(a.rows[4].source, "- Literal");
    assert_eq!(a.rows[5].source, "  Root");
    assert_eq!(a.rows[6].source, "");
    for selected in [-1, 0, 1, 2, 3, 5, 99] {
        let a = Accordion::parse_source(SOURCE, selected).unwrap();
        assert_eq!(
            a.rows.iter().filter(|r| r.visible).count(),
            if (1..=3).contains(&selected) { 6 } else { 4 }
        );
    }
    assert_eq!(Accordion::parse_source("", 0).unwrap().rows.len(), 1);
    assert_eq!(accordion::labels(""), accordion::labels(accordion::DEFAULT));
    assert!(Accordion::parse(&"x".repeat(100_001), 0).is_err());
    assert!(Accordion::parse(&"\n".repeat(10_000), 0).is_err());
}
#[test]
fn sizing_counts_visible_rows_but_measures_hidden_labels() {
    let fonts = Metrics;
    let style = TextStyle {
        size: 13.,
        bold: false,
        italic: false,
    };
    for (sketch, collapsed, expanded) in [(false, 196., 254.), (true, 186., 240.)] {
        let a = Accordion::parse(SOURCE, 0).unwrap();
        assert_eq!(a.measure(&fonts, style, sketch).height, collapsed);
        let a = Accordion::parse(SOURCE, 2).unwrap();
        assert_eq!(a.measure(&fonts, style, sketch).height, expanded);
        let a = Accordion::parse(
            "Short\n- A hidden label longer than the minimum pane width\nOther",
            99,
        )
        .unwrap();
        let m = a.measure(&fonts, style, sketch);
        assert!(m.width > 150.);
        assert_eq!(m.height, 186.);
    }
}
#[test]
fn layout_anchors_lower_headings_and_excludes_scrollbar_from_child_links() {
    let a = Accordion::parse(SOURCE, 2).unwrap();
    let m = a.measure(
        &Metrics,
        TextStyle {
            size: 13.,
            bold: false,
            italic: false,
        },
        false,
    );
    let l = a.layout(&m, 150., 250., false, true);
    assert_eq!(
        l.rows.iter().map(|r| r.bounds.y).collect::<Vec<_>>(),
        [0., 29., 58., 87., 191., 220.]
    );
    assert_eq!(
        l.rows.iter().map(|r| r.link.w).collect::<Vec<_>>(),
        [150., 150., 134., 134., 150., 150.]
    );
    assert_eq!(l.scrollbar, Some(Bounds::new(134., 58., 16., 134.)));
    let short = a.layout(&m, 90., 60., false, true);
    assert_eq!(
        short.rows.iter().map(|r| r.index).collect::<Vec<_>>(),
        [0, 1]
    );
    assert!(short.scrollbar.is_none());
    let none = Accordion::parse(SOURCE, -1).unwrap();
    let m = none.measure(
        &Metrics,
        TextStyle {
            size: 13.,
            bold: false,
            italic: false,
        },
        false,
    );
    let l = none.layout(&m, 150., 250., false, true);
    assert_eq!(
        l.rows.iter().map(|r| r.index).collect::<Vec<_>>(),
        [0, 1, 4, 5]
    );
    assert!(l.scrollbar.is_none());
    assert_eq!(l.rows.last().unwrap().bounds.y, 87.);
}
#[test]
fn manual_resize_undo_restores_automatic_dimensions_and_selection() {
    let (mut e, id, _) = fixture();
    assert_eq!(size(&e), Bounds::new(50., 50., 150., 196.));
    let before = e.document.clone();
    set(&mut e, id, "selected_index", Value::Number(2.));
    assert_eq!(size(&e).h, 254.);
    e.begin("Resize");
    e.selected_mut(|c| {
        c.bounds.w = 90.;
        c.bounds.h = 60.;
    });
    e.commit();
    assert_eq!(size(&e).w, 90.);
    e.undo();
    assert_eq!(size(&e).h, 254.);
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(size(&e).h, 196.);
    e.redo();
    e.redo();
    assert_eq!(size(&e).h, 60.);
    e.begin("Auto-size");
    e.auto_size_control(id).unwrap();
    e.commit();
    assert_eq!(size(&e).h, 254.);
    e.undo();
    assert_eq!(size(&e).h, 60.);
}
#[test]
fn navigation_uses_visible_source_indices_and_scroll_value_is_decorative() {
    let (mut e, id, dest) = fixture();
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![
            Some(Link::Page(dest)),
            Some(Link::Back),
            Some(Link::Page(dest)),
            Some(Link::Back),
            Some(Link::Page(dest)),
            Some(Link::Back),
        ]),
    );
    set(&mut e, id, "selected_index", Value::Number(-1.));
    let scene = e.rendered_scene(e.page).unwrap();
    assert_eq!(scene.links.len(), 4);
    assert_eq!(scene.links[2].target, format!("page:{dest}"));
    set(&mut e, id, "selected_index", Value::Number(2.));
    set(&mut e, id, "vertical_scrollbar", Value::Bool(true));
    e.begin("Size");
    e.selected_mut(|c| {
        c.bounds.w = 200.;
        c.bounds.h = 250.;
    });
    e.commit();
    let glyphs = |s: wireframe::render::Scene| {
        s.items
            .into_iter()
            .filter_map(|i| match i.drawing {
                Drawing::Glyph(g) => Some((g.text, g.x, g.baseline)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(e.page).unwrap();
        assert_eq!(scene.links.len(), 6);
        assert!(scene.hit_link(245., 125.).is_none());
        assert!(scene.hit_link(75., 200.).is_none());
        assert!(scene.hit_link(75., 125.).is_some());
        assert!(scene.hit_link(75., 290.).is_some());
        let before = glyphs(scene);
        set(&mut e, id, "value", Value::Number(1.));
        assert_eq!(before, glyphs(e.rendered_scene(e.page).unwrap()));
        set(&mut e, id, "value", Value::Number(0.));
    }
}
#[test]
fn source_transactions_preserve_duplicate_destinations_and_component_overrides() {
    let (mut e, id, dest) = fixture();
    set(
        &mut e,
        id,
        "text",
        Value::Text("Same\n- Same\nOther".into()),
    );
    set(&mut e, id, "selected_index", Value::Number(1.));
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![Some(Link::Back), Some(Link::Page(dest)), None]),
    );
    let before = e.document.clone();
    e.begin("Replace source");
    e.set_control_property(id, "text", Value::Text("".into()))
        .unwrap();
    e.set_control_property(id, "text", Value::Text("Other\nSame\n- Same".into()))
        .unwrap();
    e.commit();
    let p = e.control_properties(id).unwrap().1;
    assert_eq!(p["selected_index"], Value::Number(2.));
    assert_eq!(
        p["row_links"],
        Value::Links(vec![None, Some(Link::Back), Some(Link::Page(dest))])
    );
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    e.create_component("Accordion component".into()).unwrap();
    let definitions = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    set(
        &mut e,
        child,
        "text",
        Value::Text("Other\nSame\nSame".into()),
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
fn formatted_labels_keep_visual_styles_without_owning_navigation() {
    let (mut e, id, dest) = fixture();
    set(
        &mut e,
        id,
        "text",
        Value::Text("{color:red}Heading{color}\n- [Child](https://example.org)\nLast".into()),
    );
    set(&mut e, id, "selected_index", Value::Number(1.));
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![None, Some(Link::Page(dest)), None]),
    );
    let scene = e.rendered_scene(e.page).unwrap();
    assert_eq!(scene.links.len(), 1);
    assert_eq!(scene.links[0].target, format!("page:{dest}"));
    assert!(scene.items.iter().any(|i|matches!(&i.drawing,Drawing::Glyph(g) if u16::from(g.color[0])>u16::from(g.color[1])*2&&u16::from(g.color[0])>u16::from(g.color[2])*2)));
    assert!(scene.png().is_ok());
    assert!(scene.svg().contains("clipPath"));
}
