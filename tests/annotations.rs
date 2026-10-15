// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    annotations::{self, Direction, Family},
    catalog,
    document::*,
    layout::TextMetrics,
    model::{Bounds, Kind},
    render::Drawing,
    workspace::Editor,
};
fn fixture(kind: Kind) -> (Editor, Id) {
    let mut e = Editor::default();
    e.add(kind, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
fn set(e: &mut Editor, id: Id, key: &str, value: Value) {
    e.begin("Edit annotation");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
fn text(e: &Editor) -> String {
    e.rendered_scene(e.page)
        .unwrap()
        .items
        .into_iter()
        .filter_map(|i| {
            if let Drawing::Glyph(g) = i.drawing {
                Some(g.text)
            } else {
                None
            }
        })
        .collect()
}
#[test]
fn defaults_fields_and_style_colors() {
    for (kind, id, default, markup) in [
        (Kind::Callout, "callout", "1", true),
        (Kind::StickyNote, "sticky_note", "A comment", true),
        (Kind::Tooltip, "tooltip", "a tooltip", false),
    ] {
        let (mut e, n) = fixture(kind);
        assert_eq!(e.selected_views()[0].text, default);
        assert_eq!(e.document.pages[0].content.nodes[0].markup, markup);
        assert_eq!(e.control_properties(n).unwrap().0.id, id);
        if kind == Kind::StickyNote {
            assert_eq!(
                e.selected_views()[0].bounds,
                Bounds::new(20., 30., 109., 123.)
            );
        }
        if kind == Kind::Callout {
            for (sketch, fill) in [(true, [255, 255, 0, 255]), (false, [38, 117, 231, 255])] {
                e.document.settings.sketch = sketch;
                assert_eq!(
                    e.control_properties(n).unwrap().1["fill"],
                    Value::Color(fill)
                );
                assert_eq!(
                    e.control_properties(n).unwrap().1["background_opacity"],
                    Value::Number(1.)
                );
            }
        }
    }
    assert!(
        catalog::get("tooltip")
            .unwrap()
            .property("direction")
            .unwrap()
            .visible
    );
    assert!(
        catalog::get("callout")
            .unwrap()
            .property("background_opacity")
            .unwrap()
            .visible
    );
}
#[test]
fn automatic_text_grows_fixed_text_retains_size_and_history() {
    let (mut e, id) = fixture(Kind::Callout);
    let before = e.document.clone();
    set(
        &mut e,
        id,
        "text",
        Value::Text("Several much longer words".into()),
    );
    let auto = e.selected_views()[0].bounds;
    assert!(auto.w > 100.);
    assert_eq!(auto.h, 39.);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.selected_views()[0].bounds, auto);
    e.begin("Resize");
    e.selected_mut(|c| {
        c.bounds.w = 80.;
        c.bounds.h = 60.;
    });
    e.commit();
    set(
        &mut e,
        id,
        "text",
        Value::Text("Text with even more words than before".into()),
    );
    assert_eq!(
        (
            e.selected_views()[0].bounds.w,
            e.selected_views()[0].bounds.h
        ),
        (80., 60.)
    );
    e.begin("Auto-size");
    e.auto_size_control(id).unwrap();
    e.commit();
    assert!(e.selected_views()[0].bounds.w > 80.);
    e.undo();
    assert_eq!(
        (
            e.selected_views()[0].bounds.w,
            e.selected_views()[0].bounds.h
        ),
        (80., 60.)
    );
}
#[test]
fn page_names_expand_without_mutating_text_and_tooltips_stay_literal() {
    for kind in [Kind::Callout, Kind::StickyNote, Kind::Tooltip] {
        let (mut e, id) = fixture(kind);
        e.document.pages[0].name = "Page *one*".into();
        set(&mut e, id, "text", Value::Text("{mockup-name}".into()));
        e.selected_mut(|c| {
            c.bounds.w = 300.;
            c.bounds.h = 100.;
        });
        let rendered = text(&e);
        assert!(
            rendered.contains(if kind == Kind::Tooltip {
                "{mockup-name}"
            } else {
                "Page *one*"
            }),
            "{kind:?}: {rendered}"
        );
        assert_eq!(
            e.control_properties(id).unwrap().1["text"],
            Value::Text("{mockup-name}".into())
        );
        assert!(
            !String::from_utf8(e.document.encode().unwrap())
                .unwrap()
                .contains("_page_title")
        );
        e.document.pages[0].name = "Renamed page".into();
        assert!(text(&e).contains(if kind == Kind::Tooltip {
            "{mockup-name}"
        } else {
            "Renamed page"
        }));
    }
}
#[test]
fn note_alignment_and_small_label_thresholds() {
    let m = TextMetrics {
        width: 35.,
        height: 16.,
        rows: 1,
    };
    for sketch in [true, false] {
        let left = annotations::layout(
            Family::StickyNote,
            200.,
            123.,
            sketch,
            m,
            true,
            Direction::TopLeft,
        );
        let centered = annotations::layout(
            Family::StickyNote,
            200.,
            123.,
            sketch,
            m,
            false,
            Direction::TopLeft,
        );
        assert_eq!(left.label.unwrap().x, 10.);
        assert_eq!(centered.label.unwrap().x, 82.5);
        let minw = if sketch { 24. } else { 20. };
        let minh = if sketch { 41. } else { 23. };
        assert!(
            annotations::layout(
                Family::StickyNote,
                minw,
                100.,
                sketch,
                m,
                false,
                Direction::TopLeft
            )
            .label
            .is_none()
        );
        assert!(
            annotations::layout(
                Family::StickyNote,
                100.,
                minh,
                sketch,
                m,
                false,
                Direction::TopLeft
            )
            .label
            .is_none()
        );
        assert!(
            annotations::layout(
                Family::StickyNote,
                minw + 1.,
                minh + 1.,
                sketch,
                m,
                false,
                Direction::TopLeft
            )
            .label
            .is_some()
        );
    }
    assert_eq!(
        annotations::layout(
            Family::StickyNote,
            10.,
            10.,
            true,
            m,
            false,
            Direction::TopLeft
        )
        .tape
        .unwrap()
        .x,
        -18.
    );
    let note = annotations::layout(
        Family::StickyNote,
        400.,
        123.,
        true,
        m,
        false,
        Direction::TopLeft,
    );
    assert_eq!(note.tape, Some(Bounds::new(150., 0., 100., 27.)));
    assert_eq!(note.frame, Bounds::new(0., 14., 400., 109.));
}
#[test]
fn pointer_boundary_directions_and_text_body() {
    for sketch in [false, true] {
        for d in [
            Direction::TopLeft,
            Direction::TopRight,
            Direction::BottomLeft,
            Direction::BottomRight,
        ] {
            for (w, h, visible) in [(31., 28., false), (32., 27., false), (32., 28., true)] {
                let a = annotations::layout(
                    Family::Tooltip,
                    w,
                    h,
                    sketch,
                    TextMetrics::default(),
                    false,
                    d,
                );
                assert_eq!(a.pointer.is_some(), visible);
                assert_eq!(a.mirror, [d.right(), d.bottom()]);
                assert_eq!(a.frame.y, if d.bottom() { 0. } else { 7. });
                assert_eq!(a.frame.h, h - 7.);
            }
        }
    }
}
#[test]
fn opacity_and_contrast_do_not_fade_outline_or_text() {
    let (mut e, id) = fixture(Kind::Callout);
    set(&mut e, id, "fill", Value::Color([0, 0, 0, 255]));
    for (alpha, white) in [(0.749, false), (0.75, true), (1., true)] {
        set(&mut e, id, "background_opacity", Value::Number(alpha));
        let s = e.rendered_scene(e.page).unwrap();
        assert_eq!(s.items[0].opacity, alpha as f32);
        assert!(s.items.iter().skip(1).all(|i| i.opacity == 1.));
        let glyph = s
            .items
            .iter()
            .find_map(|i| {
                if let Drawing::Glyph(g) = &i.drawing {
                    Some(g)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(glyph.color[..3], if white { [255; 3] } else { [0; 3] });
    }
    assert!(!annotations::light_text(
        Family::StickyNote,
        [100; 3],
        1.,
        true
    ));
    assert!(annotations::light_text(
        Family::StickyNote,
        [100; 3],
        1.,
        false
    ));
}
#[test]
fn rich_links_export_and_storage_roundtrip() {
    for kind in [Kind::Callout, Kind::StickyNote, Kind::Tooltip] {
        let (mut e, id) = fixture(kind);
        set(
            &mut e,
            id,
            "text",
            Value::Text("*Bold* _Italic_\n[Link](https://example.org)".into()),
        );
        e.selected_mut(|c| {
            c.bounds.w = 260.;
            c.bounds.h = 160.;
        });
        let scene = e.rendered_scene(e.page).unwrap();
        assert!(
            scene
                .items
                .iter()
                .any(|i| matches!(&i.drawing,Drawing::Glyph(g) if g.bold))
        );
        assert!(
            scene
                .items
                .iter()
                .any(|i| matches!(&i.drawing,Drawing::Glyph(g) if g.italic))
        );
        assert_eq!(scene.links.len(), 1);
        assert_eq!(scene.links[0].target, "https://example.org");
        assert!(scene.svg().contains("https://example.org"));
        assert!(scene.png().unwrap().len() > 1000);
        let encoded = e.document.encode().unwrap();
        let decoded = wireframe::document::decode(&encoded).unwrap();
        assert_eq!(decoded, e.document);
    }
}
#[test]
fn component_annotation_properties_are_local_and_undoable() {
    let (mut e, _) = fixture(Kind::Tooltip);
    e.create_component("Hint".into()).unwrap();
    let definition = e.document.definitions[0].clone();
    let instance = *e.selection.iter().next().unwrap();
    assert!(e.enter(instance));
    let id = e.nodes()[0].id;
    e.selection.insert(id);
    let before = e.document.clone();
    set(
        &mut e,
        id,
        "direction",
        Value::Choice("bottom_right".into()),
    );
    assert_eq!(e.document.definitions[0], definition);
    assert_eq!(
        e.control_properties(id).unwrap().1["direction"],
        Value::Choice("bottom_right".into())
    );
    e.undo();
    assert_eq!(e.document, before);
}

#[test]
fn narrow_tooltip_does_not_emit_an_oversized_glyph_or_link() {
    let (mut e, id) = fixture(Kind::Tooltip);
    set(
        &mut e,
        id,
        "text",
        Value::Text("[X](https://example.org)".into()),
    );
    e.selected_mut(|c| {
        c.bounds.w = 31.;
        c.bounds.h = 28.;
    });
    let scene = e.rendered_scene(e.page).unwrap();
    assert!(
        !scene
            .items
            .iter()
            .any(|i| matches!(&i.drawing,Drawing::Glyph(g) if g.text.contains('X')))
    );
    assert!(scene.links.is_empty());
}
