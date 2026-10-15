// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
mod support;
use wireframe::{
    catalog,
    document::{self, Dimension, NodeContent, Value},
    layout::TextMetrics,
    model::{Bounds, Kind},
    render::Drawing,
    scene::Primitive,
    text_controls as text,
    workspace::Editor,
};
fn fixture(kind: Kind) -> (Editor, u64) {
    let mut e = Editor::default();
    e.add(kind, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
#[test]
fn placeholder_conversion_is_an_identity_preserving_undo_step() {
    for (kind, target, w, h) in [
        (Kind::TextBlock, Kind::Paragraph, 275., 80.),
        (Kind::TextLine, Kind::Subtitle, 200., 27.),
    ] {
        let (mut e, id) = fixture(kind);
        let before = e.document.clone();
        assert_eq!(
            (
                e.selected_views()[0].bounds.w,
                e.selected_views()[0].bounds.h
            ),
            (w, h)
        );
        let (draft_id, draft) = e.prepare_text_edit(id).unwrap().unwrap();
        assert_eq!(draft_id, id);
        assert!(!draft.is_empty());
        assert_eq!(e.selected_views()[0].kind, target);
        assert_eq!(e.selected_views()[0].bounds.x, 20.);
        assert_eq!(e.selected_views()[0].bounds.w, w);
        assert_eq!(e.undo_name(), Some("Convert to text"));
        let converted = e.document.clone();
        // Cancelling a draft makes no mutation. Conversion itself remains undoable.
        e.undo();
        assert_eq!(e.document, before);
        assert!(e.selection.contains(&id));
        e.redo();
        assert_eq!(e.document, converted);
        e.edit("Edit text", |e| {
            e.set_control_property(id, "text", Value::Text("A new caption".into()))
                .unwrap();
        });
        e.undo();
        assert_eq!(e.document, converted);
        e.undo();
        assert_eq!(e.document, before);
        e.redo();
        e.redo();
        let data = e.document.encode().unwrap();
        assert_eq!(document::decode(&data).unwrap(), e.document);
    }
}
#[test]
fn locked_and_component_instance_placeholders_do_not_convert_accidentally() {
    let (mut e, id) = fixture(Kind::TextLine);
    e.document.pages[0].content.nodes[0].locked = true;
    let before = e.document.clone();
    assert!(e.prepare_text_edit(id).unwrap().is_none());
    assert_eq!(e.document, before);
    e.document.pages[0].content.nodes[0].locked = false;
    e.create_component("Text symbol".into()).unwrap();
    let instance = *e.selection.iter().next().unwrap();
    assert!(e.enter(instance));
    let child = e.current().controls[0].id;
    let before = e.document.clone();
    assert!(e.prepare_text_edit(child).is_err());
    assert_eq!(e.document, before);
}
#[test]
fn title_sizing_uses_text_height_and_preserves_explicit_formatting() {
    for (kind, font) in [(Kind::Heading, 40.), (Kind::Subtitle, 24.)] {
        let (mut e, id) = fixture(kind);
        assert_eq!(e.selected_views()[0].font_size, font);
        assert!(!e.selected_views()[0].bold);
        let natural = e.selected_views()[0].bounds;
        if let NodeContent::Control { size, .. } = &mut e.document.pages[0].content.nodes[0].content
        {
            size.width = Dimension::Fixed(80.);
            size.height = Dimension::Fixed(1.);
        }
        assert_eq!(e.selected_views()[0].bounds.h, natural.h);
        assert_eq!(e.selected_views()[0].bounds.w, 80.);
        e.edit("Fit text", |e| e.auto_size_control(id).unwrap());
        assert_eq!(e.selected_views()[0].bounds, natural);
        e.undo();
        assert_eq!(e.selected_views()[0].bounds.w, 80.);
        e.set_control_property(id, "font_size", Value::Number(28.))
            .unwrap();
        e.set_control_property(id, "bold", Value::Bool(true))
            .unwrap();
        e.set_control_property(id, "text", Value::Text("".into()))
            .unwrap();
        assert!(!e.selected_views()[0].text.is_empty());
        let decoded = document::decode(&e.document.encode().unwrap()).unwrap();
        let loaded = Editor::new(decoded);
        assert_eq!(loaded.current().controls[0].font_size, 28.);
        assert!(loaded.current().controls[0].bold);
        e.set_control_property(
            id,
            "text",
            Value::Text("One\nTwo\\rThree {mockup-name}".into()),
        )
        .unwrap();
        let s = e.rendered_scene(e.page).unwrap();
        let glyphs: Vec<_> = s
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
        assert!(glyphs.len() >= 3);
        assert!(glyphs[1].baseline > glyphs[0].baseline);
        assert!(glyphs[2].baseline > glyphs[1].baseline);
    }
}
#[test]
fn placeholder_rows_obey_strict_height_and_project_font_without_mutating_nodes() {
    assert!(text::block_rows(100., 16., 13.).is_empty());
    assert_eq!(
        text::block_rows(100., 17., 13.),
        vec![Bounds::new(0., 3., 100., 13.)]
    );
    assert_eq!(text::line_width(200., 27.), 1521.);
    assert_eq!(text::line_width(5000., 72.), 4056.);
    for kind in [Kind::TextBlock, Kind::TextLine] {
        let (mut e, id) = fixture(kind);
        e.document.settings.sketch = false;
        e.document.settings.font_size = 13.;
        let first = e.rendered_scene(e.page).unwrap();
        e.document.settings.font_size = 24.;
        let before = e.document.clone();
        let second = e.rendered_scene(e.page).unwrap();
        assert_eq!(e.document, before);
        if kind == Kind::TextBlock {
            assert_ne!(first.svg(), second.svg());
        } else {
            assert_eq!(first.svg(), second.svg());
        }
        assert!(first.items.iter().all(|i| i.opacity == 0.5
            && matches!(
                &i.drawing,
                Drawing::Primitive(Primitive::Rect { stroke: false, .. })
            )));
        assert!(e.auto_size_control(id).is_err());
        for sketch in [false, true] {
            e.document.settings.sketch = sketch;
            support::stored_size(&mut e, 1., 80.);
            let s = e.rendered_scene(e.page).unwrap();
            assert!(
                s.items
                    .iter()
                    .all(|i| i.clip == Some(Bounds::new(20., 30., 1., 80.)))
            );
            assert!(s.png().is_ok());
        }
    }
}
#[test]
fn cloud_spaces_size_cadence_wrap_equality_and_visibility_are_stable() {
    assert_eq!(
        text::cloud_labels("web\\ design  code"),
        vec!["web design", "", "code"]
    );
    assert_eq!(text::cloud_labels(""), vec!["", "..."]);
    let labels = vec!["two words".into(), "next".into()];
    assert_eq!(text::cloud_labels(&text::cloud_rebuild(&labels)), labels);
    let measure = |_: &wireframe::rich_text::RichText, _| TextMetrics {
        width: 20.,
        height: 10.,
        rows: 1,
    };
    let hidden = text::cloud_layout_with("one two", 0, 100., 25., measure);
    assert!(hidden.iter().all(|l| !l.visible));
    let shown = text::cloud_layout_with("one two", 0, 100., 26., measure);
    assert!(shown.iter().all(|l| l.visible));
    assert_eq!(shown[0].bounds, Bounds::new(9., 15., 20., 10.));
    let wrapped = text::cloud_layout_with("one two", 0, 34., 50., measure);
    assert_eq!(wrapped[0].bounds.y, 39.);
    assert!(wrapped[0].visible);
    assert!(!wrapped[1].visible);
    for id in [0, 1, 7, 37, 74, 100, u64::MAX] {
        for i in 0..160 {
            assert!((6. ..=21.).contains(&text::cloud_font(id, i)));
            assert_eq!(text::cloud_font(id, i), text::cloud_font(id, i + 76));
        }
    }
}
#[test]
fn cloud_rendering_preserves_literal_macros_rich_tags_and_links() {
    let (mut e, id) = fixture(Kind::TagCloud);
    e.set_control_property(
        id,
        "text",
        Value::Text("{mockup-name} {color:#FF0000}red{color} [link](https://example.com)".into()),
    )
    .unwrap();
    let s = e.rendered_scene(e.page).unwrap();
    assert!(
        s.items
            .iter()
            .any(|i| matches!(&i.drawing,Drawing::Glyph(g) if g.text.contains("{mockup-name}")))
    );
    assert!(s.items.iter().any(
        |i| matches!(&i.drawing,Drawing::Glyph(g) if g.text=="red" && g.color[..3]==[255,0,0])
    ));
    assert!(s.links.iter().any(|l| l.target == "https://example.com"));
    assert!(s.svg().contains("https://example.com"));
    assert_eq!(
        e.control_properties(id)
            .unwrap()
            .0
            .properties
            .iter()
            .filter(|p| p.visible)
            .map(|p| p.key)
            .collect::<Vec<_>>(),
        vec!["text"]
    );
    for id in ["text_block", "text_line"] {
        assert!(
            catalog::get(id)
                .unwrap()
                .properties
                .iter()
                .all(|p| !p.visible)
        );
    }
}

#[test]
fn component_text_overrides_remain_local_and_undoable() {
    for kind in [Kind::Heading, Kind::Subtitle, Kind::TagCloud] {
        let (mut e, _) = fixture(kind);
        e.create_component("Text component".into()).unwrap();
        let definition = e.document.definitions[0].clone();
        let instance = *e.selection.iter().next().unwrap();
        assert!(e.enter(instance));
        let id = e.nodes()[0].id;
        e.selection.insert(id);
        let before = e.document.clone();
        e.edit("Edit instance text", |e| {
            e.set_control_property(id, "text", Value::Text("Local text\nSecond line".into()))
                .unwrap()
        });
        assert_eq!(e.document.definitions[0], definition);
        assert_eq!(
            e.control_properties(id).unwrap().1["text"],
            Value::Text("Local text\nSecond line".into())
        );
        assert!(e.rendered_scene(e.surface_id()).is_ok());
        let data = e.document.encode().unwrap();
        assert_eq!(document::decode(&data).unwrap(), e.document);
        e.undo();
        assert_eq!(e.document, before);
    }
}
