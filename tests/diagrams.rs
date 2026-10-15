// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    catalog,
    document::{self, Dimension, Link, NodeContent, Value},
    layout::{TextMeasurer, TextMetrics, TextStyle},
    model::Kind,
    popover::{self, Side},
    site_map,
    workspace::Editor,
};
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&self, text: &str, _: TextStyle, _: Option<f32>) -> TextMetrics {
        let lines: Vec<_> = text.split('\n').collect();
        TextMetrics {
            width: lines.iter().map(|s| s.len()).max().unwrap_or(0) as f32 * 6.,
            height: lines.len() as f32 * 15.,
            rows: lines.len(),
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
#[test]
fn pointers_at_every_side_and_stop_keep_complete_fill_and_strict_outline_gates() {
    for sketch in [false, true] {
        for side in [Side::Top, Side::Right, Side::Bottom, Side::Left] {
            for position in 0..=4 {
                for (w, h) in [
                    (1., 100.),
                    (100., 1.),
                    (41., 100.),
                    (70., 100.),
                    (71., 100.),
                    (72., 100.),
                    (73., 100.),
                    (222., 187.),
                    (400., 100.),
                    (100., 400.),
                ] {
                    let l = popover::layout(w, h, side, position, sketch);
                    assert_eq!(
                        l.draw_box,
                        w > if sketch { 70. } else { 72. } && h > if sketch { 70. } else { 72. }
                    );
                    let tris = popover::triangles(&l.outline);
                    let area = |a: [f32; 2], b: [f32; 2], c: [f32; 2]| {
                        ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])) / 2.
                    };
                    let polygon: f32 = l
                        .outline
                        .iter()
                        .zip(l.outline.iter().cycle().skip(1))
                        .map(|(a, b)| (a[0] * b[1] - b[0] * a[1]) / 2.)
                        .sum();
                    assert_eq!(tris.is_empty(), polygon.abs() < 0.00001);
                    let mesh: f32 = tris
                        .chunks(3)
                        .map(|t| {
                            let a = area(
                                l.outline[t[0] as usize],
                                l.outline[t[1] as usize],
                                l.outline[t[2] as usize],
                            );
                            assert!(a >= 0.);
                            a
                        })
                        .sum();
                    assert!(
                        (polygon.abs() - mesh).abs() < 0.05,
                        "{w} {h} {side:?} {position}: {polygon} {mesh}"
                    );
                }
            }
        }
    }
    assert!(!popover::layout(40., 100., Side::Top, 2, false).draw_arrow);
    assert!(popover::layout(40.01, 100., Side::Top, 2, false).draw_arrow);
    assert!(popover::layout(1., 1., Side::Top, 2, false).label.is_none());
}
#[test]
fn hierarchy_indentation_intrinsic_layout_and_readable_errors() {
    let t = site_map::Tree::parse("Root\n-- One\n-- Two");
    assert_eq!(
        t.nodes.iter().map(|n| n.depth).collect::<Vec<_>>(),
        [0, 1, 1]
    );
    let v = t.layout(&Metrics, style(), false);
    assert_eq!((v.width, v.height), (220., 190.));
    assert_eq!((v.nodes[0].x, v.nodes[0].y), (60., 0.));
    assert_eq!(v.edges[0], [[110., 70.], [110., 95.]]);
    let h = t.layout(&Metrics, style(), true);
    assert_eq!((h.width, h.height), (270., 88.5));
    assert_eq!(h.edges[0][1][0] - h.edges[0][0][0], 25.);
    assert!(!site_map::Tree::parse("Root\n\tOne\n\t\tChild").invalid);
    for text in ["Root\nOther", "Root\n One", "Root\n\n- Child"] {
        let t = site_map::Tree::parse(text);
        assert!(t.invalid);
        let l = t.layout(&Metrics, style(), false);
        assert!(l.error && l.height >= 50.);
        assert_eq!(l.nodes[0].w, l.width);
        assert!(l.links.is_empty());
        assert!(site_map::labels(text).is_empty());
    }
    let t = site_map::Tree::parse("");
    assert_eq!(t.nodes.len(), 1);
    assert!(!t.invalid);
}
#[test]
fn computed_size_and_population_are_one_history_action_with_destinations() {
    let mut e = Editor::default();
    e.document.title = "Study".into();
    e.document.pages[0].name = "Start".into();
    let page_id = e.document.allocate_id().unwrap();
    let mut page = e.document.pages[0].clone();
    page.id = page_id;
    page.name = "Details".into();
    page.parent = Some(e.page);
    e.document.pages.push(page);
    e.add(Kind::SiteMap, 20., 20.);
    let id = *e.selection.iter().next().unwrap();
    let before = e.document.clone();
    e.edit("Populate", |e| e.populate_site_map(id).unwrap());
    let after = e.document.clone();
    let (_, p) = e.control_properties(id).unwrap();
    assert_eq!(p["text"], Value::Text("Study\n- Start\n-- Details".into()));
    assert_eq!(
        p["row_links"],
        Value::Links(vec![
            None,
            Some(Link::Page(e.page)),
            Some(Link::Page(page_id))
        ])
    );
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    let b = e.selected_views()[0].bounds;
    let mut c = e.selected_views()[0].clone();
    c.bounds.w = 10.;
    c.bounds.h = 10.;
    e.update_control(c);
    assert_eq!(e.selected_views()[0].bounds, b);
    assert!(e.auto_size_control(id).is_err());
    assert_eq!(
        document::decode(&e.document.encode().unwrap()).unwrap(),
        e.document
    );
    assert_eq!(e.rendered_scene(e.page).unwrap().links.len(), 2);
}
#[test]
fn popover_size_properties_history_and_link_contrast_survive_storage() {
    let mut e = Editor::default();
    e.add(Kind::Popover, 10., 10.);
    let id = *e.selection.iter().next().unwrap();
    let b = e.selected_views()[0].bounds;
    assert_eq!((b.w, b.h), (222., 187.));
    e.edit("Panel", |e| {
        e.set_control_property(
            id,
            "text",
            Value::Text("Title\n[Details](https://example.com)".into()),
        )
        .unwrap();
        e.set_control_property(id, "direction", Value::Choice("left".into()))
            .unwrap();
        e.set_control_property(id, "arrow_position", Value::Number(4.))
            .unwrap();
        e.set_control_property(id, "fill", Value::Color([0, 0, 0, 255]))
            .unwrap();
        let mut c = e.selected_views()[0].clone();
        c.bounds.w = 300.;
        e.update_control(c);
    });
    let saved = e.document.clone();
    assert_eq!(e.rendered_scene(e.page).unwrap().links.len(), 1);
    e.edit("Fit", |e| e.auto_size_control(id).unwrap());
    assert_eq!(e.selected_views()[0].bounds.w, 222.);
    e.undo();
    assert_eq!(e.document, saved);
    assert_eq!(
        document::decode(&e.document.encode().unwrap()).unwrap(),
        saved
    );
    for kind in ["popover", "site_map"] {
        let spec = catalog::get(kind).unwrap();
        assert!(spec.property("direction").unwrap().visible);
        assert!(
            !spec
                .property("background_opacity")
                .is_some_and(|p| p.visible)
        );
    }
    if let NodeContent::Control { size, .. } = &mut e.document.pages[0].content.nodes[0].content {
        size.width = Dimension::Fixed(1.);
        size.height = Dimension::Fixed(1.);
    }
    assert_eq!(e.selected_views()[0].bounds.w, 1.);
    assert!(e.rendered_scene(e.page).is_ok());
}
#[test]
fn row_links_follow_reordered_nodes_and_local_text_edits() {
    let mut e = Editor::default();
    e.add(Kind::SiteMap, 0., 0.);
    let id = *e.selection.iter().next().unwrap();
    e.edit("Edit", |e| {
        e.set_control_property(id, "text", Value::Text("Root\n- A\n- B".into()))
            .unwrap();
        e.set_control_property(
            id,
            "row_links",
            Value::Links(vec![
                None,
                Some(Link::Back),
                Some(Link::Url("https://example.com".into())),
            ]),
        )
        .unwrap();
    });
    e.edit("Reorder", |e| {
        let mut c = e.selected_views()[0].clone();
        c.text = "Root\n- B\n- A".into();
        e.update_control(c);
    });
    assert_eq!(
        e.control_properties(id).unwrap().1["row_links"],
        Value::Links(vec![
            None,
            Some(Link::Url("https://example.com".into())),
            Some(Link::Back)
        ])
    );
    e.undo();
    assert_eq!(
        e.control_properties(id).unwrap().1["text"],
        Value::Text("Root\n- A\n- B".into())
    );
}
#[test]
fn component_population_is_local_and_invalid_text_does_not_discard_links() {
    let mut e = Editor::default();
    e.add(Kind::SiteMap, 0., 0.);
    let instance = *e.selection.iter().next().unwrap();
    e.create_component("Map".into()).unwrap();
    let child = e.document.definitions[0].content.nodes[0].id;
    let definitions = e.document.definitions.clone();
    assert!(e.enter(instance));
    e.edit("Populate", |e| e.populate_site_map(child).unwrap());
    assert_eq!(e.document.definitions, definitions);
    let (_, p) = e.control_properties(child).unwrap();
    let links = p["row_links"].clone();
    let text = p["text"].clone();
    let after = e.document.clone();
    e.undo();
    assert_ne!(e.document, after);
    e.redo();
    assert_eq!(e.document, after);
    e.edit("Invalid", |e| {
        e.set_control_property(child, "text", Value::Text("Two\nRoots".into()))
            .unwrap()
    });
    assert_eq!(e.control_properties(child).unwrap().1["row_links"], links);
    assert!(e.rendered_scene(e.page).unwrap().links.is_empty());
    e.edit("Repair", |e| {
        e.set_control_property(child, "text", text).unwrap()
    });
    assert_eq!(e.control_properties(child).unwrap().1["row_links"], links);
    assert_eq!(e.document.definitions, definitions);
    let data = e.document.encode().unwrap();
    assert_eq!(document::decode(&data).unwrap(), e.document);
}
