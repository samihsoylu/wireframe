// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    menu::{self, Mark, Menu, MenuBar},
    model::{Bounds, Kind},
    render::{Drawing, Scene},
    scene::Primitive,
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
    e.begin("Edit menu");
    e.set_control_property(id, key, v).unwrap();
    e.commit();
}
fn text(e: &mut Editor, id: Id, s: &str) {
    set(e, id, "text", Value::Text(s.into()));
}
fn bounds(e: &Editor) -> Bounds {
    e.selected_views()[0].bounds
}
fn selected(s: &Scene) -> Vec<Bounds> {
    s.items
        .iter()
        .filter_map(|i| match i.drawing {
            Drawing::Primitive(Primitive::Rect {
                b,
                fill: [149, 188, 242],
                ..
            }) => Some(b),
            _ => None,
        })
        .collect()
}
#[test]
fn menu_grammar_preserves_rows_escapes_markers_and_shortcuts() {
    for s in ["", " ", "---", "==", " - - "] {
        assert!(menu::parse_row(s).separator, "{s:?}");
    }
    for s in ["\t", "-=", "−"] {
        assert!(!menu::parse_row(s).separator, "{s:?}");
    }
    assert_eq!(menu::parse_row("x -Disabled-").mark, Some(Mark::Check));
    assert_eq!(menu::parse_row("o Item").mark, Some(Mark::Dot));
    for s in ["X Item", " O Item", "-x Disabled-"] {
        assert_eq!(menu::parse_row(s).mark, None);
    }
    let r = menu::parse_row("Open\\, recent, Ctrl+O");
    assert_eq!(r.label.display_text(true), "Open, recent");
    assert_eq!(r.shortcut.unwrap().display_text(true), " Ctrl+O");
    let r = menu::parse_row("Long middle,inner,Ctrl+K");
    assert_eq!(r.label.display_text(true), "Long middle,inner");
    assert_eq!(r.shortcut.unwrap().display_text(true), "Ctrl+K");
    assert!(menu::parse_row(">").arrow);
    let r = menu::parse_row("Label,shortcut >>");
    assert!(r.arrow);
    assert!(r.shortcut.is_none());
    assert_eq!(r.label.display_text(true), "Label,shortcut");
    assert_eq!(menu::labels(" A\\,B,  C,,D", true), [" A,B", " C", "", "D"]);
    assert_eq!(MenuBar::parse("").unwrap().texts.len(), 2);
    assert_eq!(menu::labels("", true).len(), 1);
    assert!(Menu::parse(&"\n".repeat(10_000)).is_err());
    assert!(MenuBar::parse(&",".repeat(10_000)).is_err());
}
#[test]
fn menu_rows_resize_horizontally_and_keep_separate_rule_hit_height() {
    let (mut e, id, dest) = fixture(Kind::Menu);
    assert_eq!(bounds(&e).h, 151.);
    e.document.settings.sketch = true;
    assert_eq!(bounds(&e).h, 167.);
    text(&mut e, id, "First\n---\nLast");
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![Some(Link::Page(dest)); 3]),
    );
    set(&mut e, id, "selected_index", Value::Number(1.));
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        e.begin("Resize");
        e.selected_mut(|c| {
            c.bounds.w = 65.;
            c.bounds.h = 200.;
        });
        e.commit();
        assert_eq!(bounds(&e).w, 65.);
        assert_eq!(bounds(&e).h, if sketch { 57. } else { 45. });
        let s = e.rendered_scene(e.page).unwrap();
        assert_eq!(s.links.len(), 3);
        let rule = s.links[1].bounds;
        assert_eq!(rule.h, if sketch { 10. } else { 1. });
        let selection = selected(&s)[0];
        let inset = if sketch { 5. } else { 1. };
        assert_eq!(rule.x, 50.);
        assert_eq!(rule.w, 65.);
        assert_eq!(
            selection,
            Bounds::new(rule.x + inset, rule.y, rule.w - 2. * inset, rule.h)
        );
        assert_eq!(s.links[2].bounds.y - rule.y, if sketch { 5. } else { 1. });
        assert!(s.png().is_ok());
    }
    text(
        &mut e,
        id,
        "First\\rsecond, Ctrl\\rK\nx -Disabled-\n\nThird >",
    );
    set(&mut e, id, "font_size", Value::Number(24.));
    assert_eq!(bounds(&e).h, 140.);
    e.document.settings.sketch = false;
    assert_eq!(bounds(&e).h, 128.);
}
#[test]
fn skipped_bar_labels_preserve_source_destinations_and_selection_clipping() {
    let (mut e, id, dest) = fixture(Kind::MenuBar);
    text(&mut e, id, "VeryLongEntry, X, Y");
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![None, Some(Link::Page(dest)), Some(Link::Back)]),
    );
    set(&mut e, id, "selected_index", Value::Number(0.));
    e.begin("Narrow bar");
    e.selected_mut(|c| {
        c.bounds.w = 60.;
        c.bounds.h = 24.;
    });
    e.commit();
    let s = e.rendered_scene(e.page).unwrap();
    assert_eq!(s.links.len(), 1);
    assert_eq!(s.links[0].target, format!("page:{dest}"));
    let a = s.links[0].bounds;
    assert_eq!(a.x, 68.);
    assert_eq!(a.y, 52.);
    assert_eq!(a.h, 21.);
    assert_eq!(
        s.hit_link(a.x + 1., a.y + 1.),
        Some(format!("page:{dest}").as_str())
    );
    assert!(selected(&s)[0].w > 60.);
    for item in &s.items {
        assert_eq!(item.clip, Some(bounds(&e)));
    }
    assert!(s.svg().contains("<svg"));
    assert!(s.png().is_ok());
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![
            None,
            Some(Link::Url("https://example.org/menu".into())),
            None,
        ]),
    );
    assert!(
        e.rendered_scene(e.page)
            .unwrap()
            .svg()
            .contains("href=\"https://example.org/menu\"")
    );
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![Some(Link::Back), None, None]),
    );
    assert!(e.rendered_scene(e.page).unwrap().links.is_empty());
    text(&mut e, id, "");
    set(&mut e, id, "selected_index", Value::Number(1.));
    assert!(!selected(&e.rendered_scene(e.page).unwrap()).is_empty());
}
#[test]
fn disabled_menu_ink_and_optional_bar_icons_share_scene_exports() {
    for kind in [Kind::Menu, Kind::MenuBar] {
        let (mut e, id, dest) = fixture(kind);
        text(
            &mut e,
            id,
            if kind == Kind::Menu {
                "x -Disabled-\nNormal"
            } else {
                "-Disabled-, Normal"
            },
        );
        set(&mut e, id, "selected_index", Value::Number(0.));
        set(
            &mut e,
            id,
            "row_links",
            Value::Links(vec![Some(Link::Page(dest)); 2]),
        );
        for sketch in [false, true] {
            e.document.settings.sketch = sketch;
            let s = e.rendered_scene(e.page).unwrap();
            let colors: Vec<_> = s
                .items
                .iter()
                .filter_map(|i| match &i.drawing {
                    Drawing::Glyph(g) => Some(g.color),
                    _ => None,
                })
                .collect();
            assert!(colors.contains(&[128, 128, 139, 255]));
            assert!(colors.contains(&if sketch {
                [0, 0, 0, 255]
            } else {
                [51, 51, 51, 255]
            }));
            assert!(!colors.contains(&[153, 153, 153, 255]));
            assert_eq!(s.links.len(), 2);
            for i in &s.items {
                if let Drawing::Icon { color, .. } = i.drawing {
                    assert_eq!(color, [0, 0, 0, 255]);
                }
            }
        }
    }
    let (mut e, id, _) = fixture(Kind::MenuBar);
    text(&mut e, id, "File, Help");
    let before = bounds(&e);
    set(
        &mut e,
        id,
        "icon",
        Value::Icon(IconSource::Catalog("arrow-right-solid".into())),
    );
    set(&mut e, id, "icon_rotation", Value::Choice("90".into()));
    set(&mut e, id, "border", Value::Choice("none".into()));
    assert_eq!(bounds(&e).w, before.w + 42.);
    assert_eq!(bounds(&e).h, 27.);
    let s = e.rendered_scene(e.page).unwrap();
    assert!(
        s.items
            .iter()
            .any(|i| matches!(&i.drawing,Drawing::Icon{bounds:b,turns:1,..}if b.x==68.&&b.w==24.))
    );
    assert!(s.items.iter().any(|i| matches!(
        i.drawing,
        Drawing::Primitive(Primitive::Rect {
            fill: [255, 255, 255],
            ..
        })
    )));
    assert!(
        !s.items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Primitive(Primitive::Line { .. })))
    );
    assert!(s.png().is_ok());
    assert!(s.svg().contains("<svg"));
}
#[test]
fn editing_duplicate_items_preserves_destinations_history_and_component_sources() {
    for kind in [Kind::Menu, Kind::MenuBar] {
        let (mut e, id, dest) = fixture(kind);
        let sep = if kind == Kind::Menu { "\n" } else { ", " };
        text(&mut e, id, &["Same", "Same", "Other"].join(sep));
        set(
            &mut e,
            id,
            "row_links",
            Value::Links(vec![Some(Link::Back), Some(Link::Page(dest)), None]),
        );
        set(&mut e, id, "selected_index", Value::Number(1.));
        let before = e.document.clone();
        e.begin("Type replacement");
        e.set_control_property(id, "text", Value::Text("".into()))
            .unwrap();
        e.set_control_property(id, "text", Value::Text(["Other", "Same", "Same"].join(sep)))
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
        e.create_component("Menu component".into()).unwrap();
        let source = e.document.definitions.clone();
        assert!(e.enter(id));
        let child = e.nodes()[0].id;
        text(&mut e, child, &["Same", "Other", "Same"].join(sep));
        assert_eq!(e.document.definitions, source);
        assert_eq!(
            e.control_properties(child).unwrap().1["row_links"],
            Value::Links(vec![Some(Link::Back), None, Some(Link::Page(dest))])
        );
        e.reset_local_changes(child).unwrap();
        assert_eq!(e.control_properties(child).unwrap().1["text"], p["text"]);
        let unchanged = e.document.clone();
        assert!(
            e.set_control_property(child, "text", Value::Text("x".repeat(100_001)))
                .is_err()
        );
        assert_eq!(e.document, unchanged);
        assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
    }
}

#[test]
fn bar_selection_retains_style_ink_and_menu_dots_use_eight_unit_artwork() {
    let (mut e, id, _) = fixture(Kind::MenuBar);
    text(&mut e, id, "First, Second");
    set(&mut e, id, "selected_index", Value::Number(0.));
    let scene = e.rendered_scene(e.page).unwrap();
    let colors: Vec<_> = scene
        .items
        .iter()
        .filter_map(|i| match &i.drawing {
            Drawing::Glyph(g) => Some(g.color),
            _ => None,
        })
        .collect();
    assert!(!colors.is_empty());
    assert!(colors.iter().all(|c| *c == [51, 51, 51, 255]));
    let (mut e, id, _) = fixture(Kind::Menu);
    text(&mut e, id, "o Dot");
    let scene = e.rendered_scene(e.page).unwrap();
    assert!(scene.items.iter().any(|i|matches!(&i.drawing,Drawing::Icon{id,bounds,..}if id=="circle-solid"&&*bounds==Bounds::new(53.,56.,8.,8.))));
}
