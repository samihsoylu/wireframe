// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    model::{Bounds, Kind},
    render::{Drawing, Scene},
    tree::{self, Tree},
    workspace::Editor,
};
fn fixture() -> (Editor, Id, Id) {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    let page = e.page;
    e.add_page();
    let dest = e.page;
    e.switch_page(page);
    e.add(Kind::Tree, 50., 50.);
    let id = *e.selection.iter().next().unwrap();
    (e, id, dest)
}
fn set(e: &mut Editor, id: Id, key: &str, value: Value) {
    e.begin("Edit tree");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
fn text(e: &mut Editor, id: Id, s: &str) {
    set(e, id, "text", Value::Text(s.into()));
}
fn bounds(e: &Editor) -> Bounds {
    e.selected_views()[0].bounds
}
fn colors(s: &Scene) -> Vec<[u8; 4]> {
    s.items
        .iter()
        .filter_map(|i| match &i.drawing {
            Drawing::Glyph(g) => Some(g.color),
            _ => None,
        })
        .collect()
}

#[test]
fn tree_grammar_preserves_indentation_markers_and_raw_labels() {
    assert!(Tree::parse("").unwrap().rows.is_empty());
    assert_eq!(tree::labels(" \t\n"), [""]);
    assert_eq!(tree::labels("First\n\nLast\n \t"), ["First", "", "Last"]);
    let t = Tree::parse(
        " .\tf  Label\nf\tLiteral\n[+] Plus\n[x] Checked\n_ [Link](https://example.org)\nV Literal",
    )
    .unwrap();
    assert_eq!(t.rows[0].depth, 3);
    assert_eq!(t.rows[0].source, " Label");
    assert_eq!(
        tree::icon(t.rows[0].marker.as_deref()),
        Some("folder-solid")
    );
    assert_eq!(t.rows[1].marker, None);
    assert_eq!(t.rows[1].source, "f\tLiteral");
    assert_eq!(t.rows[4].source, "[Link](https://example.org)");
    assert_eq!(tree::icon(t.rows[4].marker.as_deref()), None);
    assert_eq!(t.rows[5].marker, None);
    assert_eq!(tree::labels("A\u{feff}"), ["A"]);
    assert_eq!(tree::labels("A\u{85}"), ["A\u{85}"]);
    for marker in [
        "f", "F", ">", "<", "v", "-", "[]", "[ ]", "[+]", "[-]", "[=]", "[x]", "[*]",
    ] {
        let parsed = Tree::parse(marker).unwrap();
        let icon = tree::icon(parsed.rows[0].marker.as_deref()).unwrap();
        assert!(wireframe::icons::get(icon).is_some(), "{icon}");
    }
    assert!(Tree::parse(&"x".repeat(100_001)).is_err());
    assert!(Tree::parse(&"\n".repeat(10_000)).is_err());
}
#[test]
fn tree_autosize_fixed_geometry_and_border_padding_are_distinct() {
    let (mut e, id, _) = fixture();
    assert_eq!(bounds(&e), Bounds::new(50., 50., 300., 285.));
    text(&mut e, id, "F Workspace\n - File\nThird\nFourth");
    e.begin("Narrow tree");
    e.selected_mut(|c| {
        c.bounds.w = 90.;
        c.bounds.h = 50.;
    });
    e.commit();
    assert_eq!(bounds(&e).w, 90.);
    e.begin("Auto-size");
    e.auto_size_control(id).unwrap();
    e.commit();
    assert_eq!(bounds(&e).w, 300.);
    assert_eq!(bounds(&e).h, 96.);
    e.undo();
    assert_eq!(bounds(&e), Bounds::new(50., 50., 90., 50.));
    e.redo();
    let before = e.rendered_scene(e.page).unwrap();
    let icon = before
        .items
        .iter()
        .find_map(|i| {
            if let Drawing::Icon { bounds, .. } = i.drawing {
                Some(bounds)
            } else {
                None
            }
        })
        .unwrap();
    set(&mut e, id, "border", Value::Choice("none".into()));
    let after = e.rendered_scene(e.page).unwrap();
    let moved = after
        .items
        .iter()
        .find_map(|i| {
            if let Drawing::Icon { bounds, .. } = i.drawing {
                Some(bounds)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!((icon.x - moved.x, icon.y - moved.y), (8., 6.));
    assert_eq!(bounds(&e).h, 96.);
    text(&mut e, id, "");
    assert_eq!(bounds(&e).h, 12.);
    text(&mut e, id, "First\\rsecond\nLast");
    assert_eq!(bounds(&e).h, 70.);
}
#[test]
fn tree_clips_row_links_and_retains_disabled_navigation_and_decorative_scroll() {
    let (mut e, id, dest) = fixture();
    text(&mut e, id, "F Workspace\n - File\nThird\nFourth");
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![Some(Link::Page(dest)); 4]),
    );
    set(&mut e, id, "vertical_scrollbar", Value::Bool(true));
    set(&mut e, id, "state", Value::Choice("disabled".into()));
    set(&mut e, id, "selected_index", Value::Number(2.));
    e.begin("Size");
    e.selected_mut(|c| {
        c.bounds.w = 200.;
        c.bounds.h = 50.;
    });
    e.commit();
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        let a = e.rendered_scene(e.page).unwrap();
        assert_eq!(a.links.len(), 3);
        assert_eq!(a.links[2].bounds.h, 2.);
        assert!(a.hit_link(60., 60.).is_some());
        assert!(a.hit_link(245., 60.).is_none());
        assert!(a.hit_link(60., 101.).is_none());
        assert!(colors(&a).iter().all(|c| *c == [128, 128, 139, 255]));
        let glyphs = |s: &Scene| {
            s.items
                .iter()
                .filter_map(|i| {
                    if let Drawing::Glyph(g) = &i.drawing {
                        Some((g.text.clone(), g.x, g.baseline))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };
        set(&mut e, id, "value", Value::Number(1.));
        let b = e.rendered_scene(e.page).unwrap();
        assert_eq!(glyphs(&a), glyphs(&b));
        assert!(b.png().is_ok());
        assert!(
            b.links
                .iter()
                .all(|link| link.target == format!("page:{dest}"))
        );
        assert!(b.svg().contains("clipPath"));
        set(&mut e, id, "value", Value::Number(0.));
    }
}
#[test]
fn tree_dark_and_transparent_fills_use_readable_ink_and_source_links() {
    let (mut e, id, dest) = fixture();
    text(
        &mut e,
        id,
        "F Workspace\n - Selected\n_ [Inline](https://example.org)",
    );
    set(&mut e, id, "fill", Value::Color([0, 0, 0, 255]));
    set(&mut e, id, "selected_index", Value::Number(1.));
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![None, None, Some(Link::Page(dest))]),
    );
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        let s = e.rendered_scene(e.page).unwrap();
        let c = colors(&s);
        assert!(c.contains(&[255; 4]));
        assert!(c.contains(&if sketch {
            [0, 0, 0, 255]
        } else {
            [51, 51, 51, 255]
        }));
        assert_eq!(s.links.len(), 1);
        assert_eq!(s.links[0].target, format!("page:{dest}"));
        set(&mut e, id, "background_opacity", Value::Number(0.));
        assert!(!colors(&e.rendered_scene(e.page).unwrap()).contains(&[255; 4]));
        set(&mut e, id, "background_opacity", Value::Number(1.));
    }
}
#[test]
fn tree_source_transactions_keep_destinations_and_component_overrides() {
    let (mut e, id, dest) = fixture();
    text(&mut e, id, "f Same\n - Same\nOther");
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![Some(Link::Back), Some(Link::Page(dest)), None]),
    );
    set(&mut e, id, "selected_index", Value::Number(1.));
    let before = e.document.clone();
    e.begin("Replace source");
    e.set_control_property(id, "text", Value::Text("".into()))
        .unwrap();
    e.set_control_property(id, "text", Value::Text("Other\n F Same\n..- Same".into()))
        .unwrap();
    e.commit();
    let p = e.control_properties(id).unwrap().1;
    assert_eq!(
        p["row_links"],
        Value::Links(vec![None, Some(Link::Back), Some(Link::Page(dest))])
    );
    assert_eq!(p["selected_index"], Value::Number(2.));
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    e.create_component("Tree component".into()).unwrap();
    let defs = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    text(&mut e, child, "Same\nOther\nSame");
    assert_eq!(e.document.definitions, defs);
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
