// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{self, Dimension, NodeContent, Value},
    model::Kind,
    render::Drawing,
    workspace::Editor,
};
fn fixture(orientation: &str) -> (Editor, u64) {
    let mut editor = Editor::default();
    editor.add(Kind::Label, 50., 50.);
    let id = *editor.selection.iter().next().unwrap();
    editor
        .set_control_property(id, "text", Value::Text("Some text".into()))
        .unwrap();
    editor
        .set_control_property(id, "text_orientation", Value::Choice(orientation.into()))
        .unwrap();
    (editor, id)
}
#[test]
fn orientations_use_natural_text_dimensions_and_ignore_fixed_height() {
    for (orientation, width, height) in [
        ("upright", 59., 21.),
        ("left", 21., 59.),
        ("right", 21., 59.),
        ("diagonal", 60., 28.),
    ] {
        let (mut editor, _) = fixture(orientation);
        let b = editor.selected_views()[0].bounds;
        assert_eq!(
            (b.x, b.y, b.w, b.h),
            (50., 50., width, height),
            "{orientation}"
        );
        if orientation != "upright" {
            if let NodeContent::Control { size, .. } =
                &mut editor.document.pages[0].content.nodes[0].content
            {
                size.width = Dimension::Fixed(200.);
                size.height = Dimension::Fixed(400.);
            }
            let b = editor.selected_views()[0].bounds;
            assert_eq!(
                (b.x, b.y, b.w, b.h),
                (50., 50., 200., height),
                "{orientation}"
            );
        }
    }
}
#[test]
fn rotated_edit_font_autosize_and_history_keep_origin_for_every_alignment() {
    for orientation in ["left", "right", "diagonal"] {
        for align in ["left", "center", "right"] {
            let (mut editor, id) = fixture(orientation);
            editor
                .set_control_property(id, "align", Value::Choice(align.into()))
                .unwrap();
            let before = editor.document.clone();
            let draft = editor.text_preview_scene(id, "Longer label text").unwrap();
            assert_eq!(editor.document, before);
            editor.edit("Edit caption", |e| {
                e.set_control_property(id, "text", Value::Text("Longer label text".into()))
                    .unwrap()
            });
            let after = editor.document.clone();
            let b = editor.selected_views()[0].bounds;
            assert_eq!((b.x, b.y), (50., 50.));
            assert_eq!(b.h, if orientation == "diagonal" { 38. } else { 99. });
            let geometry = |scene: wireframe::render::Scene| {
                scene
                    .items
                    .into_iter()
                    .filter_map(|i| match i.drawing {
                        Drawing::Glyph(g) => Some((g.text, g.x, g.baseline, g.rotation)),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                geometry(draft),
                geometry(editor.rendered_scene(editor.page).unwrap())
            );
            editor.undo();
            assert_eq!(editor.document, before);
            editor.redo();
            assert_eq!(editor.document, after);
            assert_eq!(document::decode(&after.encode().unwrap()).unwrap(), after);
            editor
                .set_control_property(id, "font_size", Value::Number(32.))
                .unwrap();
            let b = editor.selected_views()[0].bounds;
            assert_eq!((b.x, b.y), (50., 50.));
            editor.auto_size_control(id).unwrap();
            let b = editor.selected_views()[0].bounds;
            assert_eq!((b.x, b.y), (50., 50.));
        }
    }
}
#[test]
fn fixed_width_rotation_does_not_realign_or_clip_natural_text() {
    for orientation in ["left", "right", "diagonal"] {
        let (mut e, id) = fixture(orientation);
        if let NodeContent::Control { size, .. } = &mut e.document.pages[0].content.nodes[0].content
        {
            size.width = Dimension::Fixed(10.);
        }
        let scene = e.rendered_scene(e.page).unwrap();
        e.set_control_property(id, "align", Value::Choice("right".into()))
            .unwrap();
        let right = e.rendered_scene(e.page).unwrap();
        assert_eq!(scene.svg(), right.svg());
        assert!(right.items.iter().all(|i| i.clip.is_none()));
        assert_eq!(e.selected_views()[0].bounds.w, 10.);
        if orientation == "diagonal" {
            assert!(right.bounds.w >= 59., "Export must retain overflowing ink");
        }
    }
}
#[test]
fn component_orientation_override_resets_without_changing_source_and_undoes() {
    let (mut e, _) = fixture("upright");
    e.create_component("Caption".into()).unwrap();
    let instance = *e.selection.iter().next().unwrap();
    let definitions = e.document.definitions.clone();
    assert!(e.enter(instance));
    let child = e.current().controls[0].id;
    let before = e.document.clone();
    e.edit("Rotate local caption", |e| {
        e.set_control_property(child, "text_orientation", Value::Choice("left".into()))
            .unwrap()
    });
    let changed = e.document.clone();
    assert_eq!(e.document.definitions, definitions);
    assert_eq!(e.current().controls[0].bounds.h, 59.);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, changed);
    e.reset_control_property(child, "text_orientation").unwrap();
    assert_eq!(e.current().controls[0].bounds.h, 21.);
    assert_eq!(e.document.definitions, definitions);
}
#[test]
fn rotated_rich_text_keeps_styles_decorations_and_actual_link_hit_shape() {
    for orientation in ["left", "right", "diagonal"] {
        let (mut e, id) = fixture(orientation);
        e.set_control_property(
            id,
            "text",
            Value::Text("*Bold* _Italic_ [Visit](https://example.com)".into()),
        )
        .unwrap();
        e.set_control_property(id, "underline", Value::Bool(true))
            .unwrap();
        let scene = e.rendered_scene(e.page).unwrap();
        let glyphs: Vec<_> = scene
            .items
            .iter()
            .filter_map(|i| match &i.drawing {
                Drawing::Glyph(g) => Some(g),
                _ => None,
            })
            .collect();
        assert!(glyphs.iter().any(|g| g.bold));
        assert!(glyphs.iter().any(|g| g.italic));
        assert!(glyphs.iter().all(|g| g.rotation != 0.));
        assert!(scene.items.iter().any(|i| matches!(
            i.drawing,
            Drawing::Primitive(wireframe::scene::Primitive::Line { .. })
        )));
        let link = scene
            .links
            .iter()
            .find(|l| l.target == "https://example.com")
            .unwrap();
        let quad = link.quad.unwrap();
        let x = quad.iter().map(|p| p[0]).sum::<f32>() / 4.;
        let y = quad.iter().map(|p| p[1]).sum::<f32>() / 4.;
        assert_eq!(scene.hit_link(x, y), Some("https://example.com"));
        if orientation == "diagonal" {
            assert!(
                scene
                    .hit_link(link.bounds.x + 0.01, link.bounds.y + 0.01)
                    .is_none()
            );
        }
        let svg = scene.svg();
        assert!(svg.contains("rotate("));
        assert!(svg.contains("<polygon points="));
        assert!(!scene.png().unwrap().is_empty());
    }
}
#[test]
fn reset_orientation_restores_upright_editing_anchor() {
    let (mut e, id) = fixture("diagonal");
    e.set_control_property(id, "align", Value::Choice("right".into()))
        .unwrap();
    e.reset_control_property(id, "text_orientation").unwrap();
    let b = e.selected_views()[0].bounds;
    e.set_control_property(id, "text", Value::Text("Longer label text".into()))
        .unwrap();
    let new = e.selected_views()[0].bounds;
    assert_eq!(new.x + new.w, b.x + b.w);
}
