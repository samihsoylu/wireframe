// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    model::{Bounds, Kind},
    navigation::{self, Navigation},
    render::{Drawing, Scene},
    scene::Primitive,
    workspace::Editor,
};
fn fixture(kind: Kind) -> (Editor, Id, Id) {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    let page = e.page;
    e.add_page();
    let destination = e.page;
    e.switch_page(page);
    e.add(kind, 50., 50.);
    let id = *e.selection.iter().next().unwrap();
    (e, id, destination)
}
fn set(e: &mut Editor, id: Id, key: &str, value: Value) {
    e.begin("Edit navigation");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
fn text(e: &mut Editor, id: Id, s: &str) {
    set(e, id, "text", Value::Text(s.into()));
}
fn bounds(e: &Editor) -> Bounds {
    e.selected_views()[0].bounds
}
fn glyph_colors(s: &Scene) -> Vec<[u8; 4]> {
    s.items
        .iter()
        .filter_map(|i| {
            if let Drawing::Glyph(g) = &i.drawing {
                Some(g.color)
            } else {
                None
            }
        })
        .collect()
}
#[test]
fn escaped_commas_whitespace_and_empty_navigation_have_distinct_contracts() {
    assert_eq!(navigation::split(" A\\,B,  C,,D"), [" A,B", " C", "", "D"]);
    assert_eq!(
        Navigation::parse(" A,  B,C", true).unwrap().texts[0].display_text(true),
        " A"
    );
    assert_eq!(
        Navigation::parse(" A,  B,C", false).unwrap().texts[0].display_text(true),
        "A"
    );
    assert_eq!(navigation::labels("*One*, Two", true), ["One", "Two"]);
    assert_eq!(navigation::labels("*One*, Two", false), ["*One*", "Two"]);
    assert_eq!(Navigation::parse("Home", true).unwrap().texts.len(), 2);
    assert_eq!(navigation::labels("Home", true).len(), 1);
    assert_eq!(Navigation::parse("Home", false).unwrap().texts.len(), 1);
    for kind in [Kind::Breadcrumbs, Kind::LinkBar] {
        let (mut e, id, _) = fixture(kind);
        let original = e.document.clone();
        let default = bounds(&e);
        assert_eq!(default.h, 21.);
        text(&mut e, id, "First\nline, Second");
        assert_eq!(bounds(&e).h, 37.);
        let measured = bounds(&e);
        e.begin("Resize attempt");
        e.selected_mut(|c| {
            c.bounds.w = 20.;
            c.bounds.h = 50.;
        });
        e.commit();
        assert_eq!(bounds(&e), measured);
        e.document.settings.sketch = true;
        assert_eq!(bounds(&e), measured);
        text(&mut e, id, "");
        if kind == Kind::LinkBar {
            assert_eq!(bounds(&e), default);
            assert_eq!(
                e.control_properties(id).unwrap().1["text"],
                Value::Text(navigation::LINK_BAR.into())
            );
        } else {
            assert!(bounds(&e).w < 30.);
            assert_eq!(
                e.control_properties(id).unwrap().1["text"],
                Value::Text("".into())
            );
        }
        let unchanged = e.document.clone();
        assert!(
            e.set_control_property(id, "text", Value::Text(",".repeat(10002)))
                .is_err()
        );
        assert_eq!(e.document, unchanged);
        assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
        assert_ne!(e.document, original);
    }
}
#[test]
fn item_hits_exclude_separator_gaps_and_final_crumb_remains_linkable() {
    for kind in [Kind::Breadcrumbs, Kind::LinkBar] {
        let (mut e, id, destination) = fixture(kind);
        text(&mut e, id, "First\nline, Second");
        set(
            &mut e,
            id,
            "row_links",
            Value::Links(vec![Some(Link::Back), Some(Link::Page(destination))]),
        );
        let scene = e.rendered_scene(e.page).unwrap();
        assert_eq!(scene.links.len(), 2);
        let a = scene.links[0].bounds;
        let b = scene.links[1].bounds;
        assert_eq!(scene.hit_link(a.x + 1., a.y + 1.), Some("back:"));
        assert_eq!(
            scene.hit_link(b.x + 1., b.y + 1.),
            Some(format!("page:{destination}").as_str())
        );
        assert_eq!(scene.hit_link((a.x + a.w + b.x) / 2., a.y + 2.), None);
        if kind == Kind::Breadcrumbs {
            assert_eq!(a.h, 37.);
            assert_eq!(b.h, 37.);
            assert!(
                scene
                    .items
                    .iter()
                    .any(|i| matches!(&i.drawing,Drawing::Icon{id,..}if id=="angle-right-solid"))
            );
        } else {
            assert_eq!(a.y, 54.);
            assert_eq!(a.h, 29.);
            assert_eq!(b.h, 13.);
            assert_eq!(scene.hit_link(b.x + 1., 51.), None);
        }
        assert!(scene.png().is_ok());
        text(&mut e, id, "[Site](https://example.org/inline), Second");
        assert!(
            !e.rendered_scene(e.page)
                .unwrap()
                .svg()
                .contains("href=\"https://example.org/inline\"")
        );
        set(
            &mut e,
            id,
            "row_links",
            Value::Links(vec![
                None,
                Some(Link::Url("https://example.org/final".into())),
            ]),
        );
        assert!(
            e.rendered_scene(e.page)
                .unwrap()
                .svg()
                .contains("href=\"https://example.org/final\"")
        );
    }
    let (mut e, id, _) = fixture(Kind::LinkBar);
    text(&mut e, id, "One, Two");
    set(&mut e, id, "text_color", Value::Color([255, 0, 0, 255]));
    set(&mut e, id, "border_color", Value::Color([0, 128, 0, 255]));
    set(&mut e, id, "selected_index", Value::Number(1.));
    let s = e.rendered_scene(e.page).unwrap();
    let colors = glyph_colors(&s);
    assert!(colors.contains(&[255, 0, 0, 255]));
    assert!(colors.contains(&[0, 128, 0, 255]));
    assert!(s.items.iter().any(|i|matches!(&i.drawing,Drawing::Primitive(Primitive::Line{color:[0,128,0],width,..})if *width==2.)));
}
#[test]
fn continuous_edits_preserve_explicit_empty_links_selection_and_repeated_labels() {
    for kind in [Kind::Breadcrumbs, Kind::LinkBar] {
        let (mut e, id, destination) = fixture(kind);
        text(&mut e, id, "One, Two, Three");
        set(
            &mut e,
            id,
            "row_links",
            Value::Links(vec![None, Some(Link::Page(destination)), None]),
        );
        if kind == Kind::LinkBar {
            set(&mut e, id, "selected_index", Value::Number(1.));
        }
        let original = e.document.clone();
        e.begin("Type replacement");
        e.set_control_property(id, "text", Value::Text("".into()))
            .unwrap();
        e.set_control_property(id, "text", Value::Text("Three, One, Two".into()))
            .unwrap();
        e.commit();
        let p = e.control_properties(id).unwrap().1;
        assert_eq!(
            p["row_links"],
            Value::Links(vec![None, None, Some(Link::Page(destination))])
        );
        if kind == Kind::LinkBar {
            assert_eq!(p["selected_index"], Value::Number(2.));
        }
        e.undo();
        assert_eq!(e.document, original);
        e.redo();
        e.begin("Canvas edit");
        e.selected_mut(|c| c.text = "Three, One, Renamed".into());
        e.commit();
        assert_eq!(
            e.control_properties(id).unwrap().1["row_links"],
            p["row_links"]
        );
        text(&mut e, id, "Same, Same, Other");
        set(
            &mut e,
            id,
            "row_links",
            Value::Links(vec![Some(Link::Back), Some(Link::Page(destination)), None]),
        );
        text(&mut e, id, "Other, Same, Same");
        assert_eq!(
            e.control_properties(id).unwrap().1["row_links"],
            Value::Links(vec![None, Some(Link::Back), Some(Link::Page(destination))])
        );
        assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
    }
}
#[test]
fn local_clear_reset_copy_and_destination_deletion_preserve_sources() {
    for kind in [Kind::Breadcrumbs, Kind::LinkBar] {
        let (mut e, id, destination) = fixture(kind);
        text(&mut e, id, "One, Two, Three");
        set(
            &mut e,
            id,
            "row_links",
            Value::Links(vec![None, Some(Link::Page(destination)), None]),
        );
        e.create_component("Navigation".into()).unwrap();
        let source = e.document.definitions.clone();
        assert!(e.enter(id));
        let child = e.nodes()[0].id;
        text(&mut e, child, "");
        let p = e.control_properties(child).unwrap().1;
        assert_eq!(
            p["text"],
            Value::Text(
                if kind == Kind::Breadcrumbs {
                    ""
                } else {
                    navigation::LINK_BAR
                }
                .into()
            )
        );
        assert_eq!(p["row_links"], Value::Links(vec![]));
        assert_eq!(e.document.definitions, source);
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
        e.switch_page(destination);
        e.delete_page();
        assert!(e.document.encode().is_ok());
        e.undo();
        assert_eq!(e.document.definitions, source);
    }
}
#[test]
fn whitespace_only_link_remains_editable_and_ancestor_crop_limits_item_targets() {
    let (mut e, id, _) = fixture(Kind::LinkBar);
    text(&mut e, id, " ");
    assert!(e.rendered_scene(e.page).is_ok());
    assert!(bounds(&e).w > 0.);
    for kind in [Kind::Breadcrumbs, Kind::LinkBar] {
        let (mut e, id, _) = fixture(kind);
        text(&mut e, id, "One, Two, Three");
        set(
            &mut e,
            id,
            "row_links",
            Value::Links(vec![Some(Link::Back), Some(Link::Back), Some(Link::Back)]),
        );
        let original = bounds(&e);
        e.create_component("Cropped navigation".into()).unwrap();
        assert!(matches!(
            e.document.pages[0].content.nodes[0].content,
            NodeContent::Instance { .. }
        ));
        e.document.pages[0].content.nodes[0].crop = Some(Crop {
            x: 0.,
            y: 0.,
            width: 0.4,
            height: 1.,
        });
        let scene = e.rendered_scene(e.page).unwrap();
        assert!(!scene.links.is_empty());
        for link in &scene.links {
            assert!(link.bounds.x + link.bounds.w <= original.x + original.w * 0.4 + 0.001);
        }
        assert!(scene.items.iter().all(|i| i.clip.is_some()));
        assert!(scene.png().is_ok());
        assert!(e.document.encode().is_ok());
    }
}
