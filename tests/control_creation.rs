// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{self, Dimension, NodeContent, Point, Size, Value},
    model::Kind,
    workspace::Editor,
};

#[test]
fn text_and_grid_creation_preserve_automatic_dimensions_and_project_formatting() {
    for kind in [Kind::Label, Kind::Link, Kind::Paragraph, Kind::Table] {
        let mut editor = Editor::default();
        editor.document.settings.font_size = 19.;
        let before = editor.document.clone();
        editor.add(kind, 65., 235.);
        assert!(editor.error.is_none(), "{:?}", editor.error);
        let node = &editor.nodes()[0];
        assert_eq!(node.origin, Point::new(65., 235.));
        let NodeContent::Control { size, .. } = &node.content else {
            panic!()
        };
        let expected = if kind == Kind::Paragraph {
            Size {
                width: Dimension::Fixed(275.),
                height: Dimension::Fixed(80.),
            }
        } else {
            Size {
                width: Dimension::Auto,
                height: Dimension::Auto,
            }
        };
        assert_eq!(*size, expected, "{kind:?}");
        assert!(!node.properties.contains_key("font_size"), "{kind:?}");
        assert_eq!(editor.selected_views()[0].font_size, 19., "{kind:?}");
        let after = editor.document.clone();
        assert_eq!(document::decode(&after.encode().unwrap()).unwrap(), after);
        editor.undo();
        assert_eq!(editor.document, before);
        editor.redo();
        assert_eq!(editor.document, after);
    }
}

#[test]
fn created_label_tracks_text_and_font_height_after_horizontal_resize() {
    let mut editor = Editor::default();
    editor.add(Kind::Label, 65., 235.);
    let id = editor.nodes()[0].id;
    let initial = editor.document.clone();
    editor.edit("Headline", |editor| {
        let mut control = editor.selected_views()[0].clone();
        control.bounds.w = 830.;
        editor.update_control(control);
        editor
            .set_control_property(id, "font_size", Value::Number(32.))
            .unwrap();
        editor
            .set_control_property(
                id,
                "text",
                Value::Text("Make room for the next idea.".into()),
            )
            .unwrap();
    });
    let control = &editor.selected_views()[0];
    assert_eq!(
        (
            control.bounds.x,
            control.bounds.y,
            control.bounds.w,
            control.bounds.h
        ),
        (65., 235., 830., 40.)
    );
    let NodeContent::Control { size, .. } = &editor.nodes()[0].content else {
        panic!()
    };
    assert_eq!(size.height, Dimension::Auto);
    let edited = editor.document.clone();
    let reopened = Editor::new(document::decode(&edited.encode().unwrap()).unwrap());
    assert_eq!(reopened.current().controls[0].bounds, control.bounds);
    editor.undo();
    assert_eq!(editor.document, initial);
    editor.redo();
    assert_eq!(editor.document, edited);
}
