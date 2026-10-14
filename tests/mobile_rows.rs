// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{self, Dimension, Link, NodeContent, Value},
    layout::{TextMeasurer, TextMetrics, TextStyle},
    mobile_menu::{Mark, Menu},
    mobile_picker::Picker,
    model::{Bounds, Kind},
    render::Drawing,
    rich_text::RichText,
    workspace::Editor,
};
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&self, text: &str, style: TextStyle, _: Option<f32>) -> TextMetrics {
        TextMetrics {
            width: text.len() as f32 * style.size / 2.,
            height: style.size * text.split('\n').count() as f32,
            rows: 1,
        }
    }
}
fn style() -> TextStyle {
    TextStyle {
        size: 13.,
        bold: false,
        italic: false,
    }
}
fn fixture(kind: Kind) -> (Editor, u64) {
    let mut e = Editor::default();
    e.add(kind, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
fn set(e: &mut Editor, id: u64, key: &str, value: Value) {
    e.begin("Edit mobile control");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
fn text(e: &mut Editor, id: u64, s: &str) {
    set(e, id, "text", Value::Text(s.into()))
}
#[test]
fn menu_syntax_preserves_spacing_escaped_commas_and_empty_rows() {
    let m=Menu::parse("+ One\\, two, three\\, four >\n__ \n_, value\n, caption\n \n+No space,ON\n + Leading, ON \n").unwrap();
    assert_eq!(m.rows.len(), 8);
    let r = &m.rows[0];
    assert_eq!(r.leading, Some(Mark::Add));
    assert_eq!(r.trailing, Some(Mark::Chevron));
    assert_eq!(r.main.as_ref().unwrap().source, "One, two");
    assert_eq!(r.caption.as_ref().unwrap().source, " three, four");
    assert!(m.rows[1].separator);
    assert_eq!(m.rows[1].leading, Some(Mark::LargeSpace));
    assert!(m.rows[2].separator);
    assert!(m.rows[3].separator);
    assert!(!m.rows[4].separator);
    assert!(m.rows[7].separator);
    assert_eq!(m.rows[5].leading, None);
    assert_eq!(m.rows[5].trailing, None);
    assert_eq!(m.rows[6].trailing, None);
    let r = Menu::parse("A,b,c").unwrap();
    assert_eq!(r.rows[0].caption.as_ref().unwrap().source, "b,c");
    assert!(Menu::parse(&"\n".repeat(10_002)).is_err());
}
#[test]
fn menu_size_and_symbol_visibility_cover_independent_limits() {
    let m = Menu::parse("A\n__ B\nC, ON\n__ , OFF").unwrap();
    for width in [None, Some(1.), Some(20.), Some(100.)] {
        let l = m.layout(&Metrics, style(), width);
        assert_eq!(l.height, 148.);
        assert_eq!(l.width, width.unwrap_or(215.));
        assert_eq!(l.rows[3].bounds.h, 29.);
    }
    assert_eq!(
        m.layout(
            &Metrics,
            TextStyle {
                size: 15.,
                ..style()
            },
            None
        )
        .width,
        217.5
    );
    let m = Menu::parse("+ A, >").unwrap();
    assert!(
        m.layout(&Metrics, style(), Some(23.)).rows[0]
            .leading
            .is_none()
    );
    assert!(
        m.layout(&Metrics, style(), Some(24.)).rows[0]
            .leading
            .is_some()
    );
    assert!(
        m.layout(&Metrics, style(), Some(15.)).rows[0]
            .trailing
            .is_none()
    );
    assert!(
        m.layout(&Metrics, style(), Some(16.)).rows[0]
            .trailing
            .is_some()
    );
    let no_icon = Menu::parse("A, caption")
        .unwrap()
        .layout(&Metrics, style(), None);
    let icon = Menu::parse("A, caption >")
        .unwrap()
        .layout(&Metrics, style(), None);
    assert!(icon.rows[0].main.unwrap().w > no_icon.rows[0].main.unwrap().w);
    assert_eq!(icon.rows[0].main.unwrap().x, 8.);
}
#[test]
fn picker_keeps_sparse_rows_rich_cells_and_only_seven_visible_rows() {
    let source = "\n,\n a\\,b , c \n*Four*, _right_\nsoft\\rbreak\n\nlast\nhidden\nalso hidden";
    let p = Picker::parse(source).unwrap();
    assert_eq!(p.rows.len(), 7);
    assert!(p.rows[0][0].is_none());
    assert!(p.rows[1][0].is_some());
    assert_eq!(p.rows[2][0].as_ref().unwrap().source, "a,b");
    assert_eq!(p.rows[3][0].as_ref().unwrap().display_text(true), "Four");
    let l = p.layout(&Metrics, 18.);
    assert_eq!(l.cells[0][0].y, 8.);
    assert_eq!(l.cells[6][0].y, 140.);
    assert_eq!(wireframe::mobile_picker::style(3, 18.).size, 18.);
    let p = Picker::parse(&format!("{}, second, third", "wide".repeat(30))).unwrap();
    let l = p.layout(&Metrics, 13.);
    assert!(l.cells[0][1].x > 214.);
    let p = Picker::parse("literal_marker, ").unwrap();
    assert_eq!(p.rows[0][0].as_ref().unwrap().source, "literal_marker");
}
#[test]
fn computed_sizes_and_menu_width_survive_history_and_storage() {
    let (mut e, id) = fixture(Kind::MobileMenu);
    text(&mut e, id, "First\nSecond");
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(20., 30., 215., 68.)
    );
    e.begin("Resize");
    e.selected_mut(|c| {
        c.bounds.w = 100.;
        c.bounds.h = 500.;
    });
    e.commit();
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(20., 30., 100., 68.)
    );
    set(&mut e, id, "font_size", Value::Number(24.));
    assert_eq!(e.selected_views()[0].bounds.w, 100.);
    let before = e.document.clone();
    e.begin("Auto-size");
    e.auto_size_control(id).unwrap();
    e.commit();
    assert_eq!(e.selected_views()[0].bounds.w, 348.);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(
        document::decode(&e.document.encode().unwrap()).unwrap(),
        e.document
    );
    let (mut e, id) = fixture(Kind::MobilePicker);
    let source = "1\n2\n3\n4\n5\n6\n7\n8\n9";
    text(&mut e, id, source);
    e.begin("Resize");
    e.selected_mut(|c| {
        c.bounds.w = 500.;
        c.bounds.h = 400.;
    });
    e.commit();
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(20., 30., 214., 160.)
    );
    assert_eq!(
        e.control_properties(id).unwrap().1["text"],
        Value::Text(source.into())
    );
    if let NodeContent::Control { size, .. } = &mut e.document.pages[0].content.nodes[0].content {
        size.width = Dimension::Fixed(500.);
        size.height = Dimension::Fixed(400.);
    }
    assert_eq!(e.selected_views()[0].bounds.w, 214.);
    assert_eq!(
        document::decode(&e.document.encode().unwrap()).unwrap(),
        e.document
    );
}
#[test]
fn menu_links_include_blank_rows_remap_and_stay_local_in_components() {
    let (mut e, id) = fixture(Kind::MobileMenu);
    text(&mut e, id, "One\n\nThree");
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![
            Some(Link::Back),
            Some(Link::Url("https://example.com".into())),
            None,
        ]),
    );
    let scene = e.rendered_scene(e.page).unwrap();
    assert_eq!(scene.links.len(), 2);
    assert_eq!(scene.links[1].bounds, Bounds::new(20., 64., 215., 29.));
    text(&mut e, id, "\nOne\nThree\nNew");
    let links = e.control_properties(id).unwrap().1["row_links"].clone();
    assert_eq!(
        links,
        Value::Links(vec![
            Some(Link::Url("https://example.com".into())),
            Some(Link::Back),
            None,
            None
        ])
    );
    e.create_component("Settings menu".into()).unwrap();
    let definitions = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    text(&mut e, child, "Local\nOne");
    assert_eq!(definitions, e.document.definitions);
    let scene = e.rendered_scene(e.page).unwrap();
    assert!(!scene.items.is_empty());
    e.undo();
    assert_eq!(e.control_properties(child).unwrap().1["row_links"], links);
}
#[test]
fn drawing_keeps_caption_style_macros_and_clips_both_controls() {
    let (mut e, id) = fixture(Kind::MobileMenu);
    text(&mut e, id, "Main, Caption\n{project-title}, =");
    for key in ["bold", "italic", "underline"] {
        set(&mut e, id, key, Value::Bool(true));
    }
    let scene = e.rendered_scene(e.page).unwrap();
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
    let main = glyphs.iter().find(|g| g.text.contains("Main")).unwrap();
    let caption = glyphs.iter().find(|g| g.text.contains("Caption")).unwrap();
    assert!(main.bold && main.italic);
    assert!(!caption.bold && !caption.italic);
    assert_eq!(main.size - caption.size, 2.);
    assert!(glyphs.iter().any(|g| g.text.contains("{project-title}")));
    let (mut e, id) = fixture(Kind::MobilePicker);
    text(&mut e, id, "1\n2\n3\n4\n5\n6\n7\nHIDDEN");
    let scene = e.rendered_scene(e.page).unwrap();
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
    assert_eq!(glyphs.len(), 7);
    assert_eq!(
        glyphs.iter().map(|g| g.size).collect::<Vec<_>>(),
        vec![10., 11., 12., 13., 12., 11., 10.]
    );
    assert!(scene.links.is_empty());
    assert!(
        scene
            .items
            .iter()
            .all(|i| i.clip == Some(Bounds::new(20., 30., 214., 160.)))
    );
    assert!(scene.png().is_ok());
}
#[test]
fn catalogue_exposes_only_mobile_fields_and_allows_original_rich_text() {
    for (kind, allowed) in [
        (
            Kind::MobileMenu,
            vec![
                "text",
                "bold",
                "italic",
                "underline",
                "font_size",
                "border",
                "row_links",
            ],
        ),
        (Kind::MobilePicker, vec!["text"]),
    ] {
        let (e, id) = fixture(kind);
        let (spec, _) = e.control_properties(id).unwrap();
        let visible: Vec<_> = spec
            .properties
            .iter()
            .filter(|p| p.visible)
            .map(|p| p.key)
            .collect();
        assert_eq!(visible.len(), allowed.len());
        for key in allowed {
            assert!(visible.contains(&key), "{key}");
        }
    }
    assert_eq!(
        RichText::parse("[Help](https://example.com)", false).display_text(true),
        "Help"
    );
}

#[test]
fn filled_marks_do_not_inherit_an_unrequested_ellipse_outline() {
    let (mut e, id) = fixture(Kind::MobileMenu);
    text(&mut e, id, "* Notice, ON\nInactive, OFF");
    let scene = e.rendered_scene(e.page).unwrap();
    assert!(!scene.items.iter().any(|item| matches!(
        item.drawing,
        Drawing::Primitive(wireframe::scene::Primitive::Ellipse { .. })
    )));
    assert!(scene.items.iter().any(|item| matches!(&item.drawing, Drawing::Primitive(wireframe::scene::Primitive::Polygon{fill:[0,0,255],points}) if points.len()>20)));
}
