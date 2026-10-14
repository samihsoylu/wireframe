// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{self, Value},
    model::{Bounds, Kind},
    render::Drawing,
    window::{Button, Options},
    workspace::Editor,
};
fn fixture() -> (Editor, u64) {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    e.add(Kind::Panel, 0., 0.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
fn set(e: &mut Editor, id: u64, key: &str, v: Value) {
    e.begin("Edit window");
    e.set_control_property(id, key, v).unwrap();
    e.commit();
}
#[test]
fn title_space_and_chrome_thresholds_are_independent() {
    let o = Options {
        minimize: true,
        maximize: true,
        close: true,
        resize_handle: true,
        scrollbar: true,
        ..Default::default()
    };
    for sketch in [false, true] {
        assert_eq!(o.layout(58., 180., 16., sketch).title.w, 0.);
        assert_eq!(
            o.layout(59., 180., 16., sketch).title.w,
            if sketch { 0. } else { 1. }
        );
        for (button, cutoff) in [
            (Button::Minimize, 51.),
            (Button::Maximize, 35.),
            (Button::Close, if sketch { 20. } else { 21. }),
        ] {
            assert!(
                !o.layout(cutoff, 19., 16., sketch)
                    .buttons
                    .iter()
                    .any(|(b, _)| *b == button)
            );
            assert!(
                o.layout(cutoff + 1., 19., 16., sketch)
                    .buttons
                    .iter()
                    .any(|(b, _)| *b == button)
            );
        }
        assert!(o.layout(450., 18., 16., sketch).buttons.is_empty());
        assert_eq!(
            o.layout(450., 28., 16., sketch).resize_handle.is_some(),
            sketch
        );
        assert!(o.layout(450., 29., 16., sketch).resize_handle.is_some());
        assert!(o.layout(17., 180., 16., sketch).resize_handle.is_none());
        let hidden = Options::default().layout(450., 180., 16., sketch);
        assert_eq!(hidden.title, o.layout(450., 180., 16., sketch).title);
    }
}
#[test]
fn hidden_footer_still_reserves_scroll_space_and_header_tracks_text() {
    let o = Options {
        scrollbar: true,
        ..Default::default()
    };
    for sketch in [false, true] {
        assert!(o.layout(300., 51., 16., sketch).footer.is_none());
        assert!(o.layout(300., 52., 16., sketch).footer.is_some());
        assert_eq!(o.layout(300., 50., 16., sketch).scrollbar.unwrap().h, 1.);
        assert!(o.layout(300., 49., 16., sketch).scrollbar.is_none());
        let large = o.layout(300., 60., 48., sketch);
        assert_eq!(large.header.h, 48.);
        assert!(large.footer.is_none());
        assert!(large.scrollbar.is_none());
        let blank = Options {
            header: 0.,
            footer: 0.,
            ..o
        }
        .layout(300., 100., 0., sketch);
        assert_eq!(blank.header.h, 0.);
        assert_eq!(blank.footer.unwrap().h, 0.);
    }
}
#[test]
fn window_properties_are_undoable_and_survive_component_and_file_round_trips() {
    let (mut e, id) = fixture();
    let original = e.document.clone();
    assert_eq!(
        e.control_properties(id).unwrap().1["font_size"],
        Value::Number(13.)
    );
    assert!(!e.explicit_control_properties(id).contains_key("font_size"));
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(0., 0., 450., 400.)
    );
    assert_eq!(e.control_properties(id).unwrap().0.id, "window");
    for (key, v) in [
        ("header_height", Value::Number(60.)),
        ("footer_height", Value::Number(55.)),
        ("close", Value::Bool(true)),
        ("resize_handle", Value::Bool(true)),
        ("vertical_scrollbar", Value::Bool(true)),
        ("value", Value::Number(0.5)),
    ] {
        set(&mut e, id, key, v);
    }
    for _ in 0..6 {
        e.undo();
    }
    assert_eq!(e.document, original);
    for _ in 0..6 {
        e.redo();
    }
    set(
        &mut e,
        id,
        "text",
        Value::Text("*Title*\\rcontinued {mockup-name}".into()),
    );
    assert_eq!(e.selected_views()[0].bounds.w, 450.);
    assert!(e.auto_size_control(id).is_err());
    let before = e.document.clone();
    assert_eq!(document::decode(&before.encode().unwrap()).unwrap(), before);
    e.create_component("Window frame".into()).unwrap();
    let definitions = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    set(&mut e, child, "header_height", Value::Number(80.));
    assert_eq!(e.document.definitions, definitions);
    e.begin("Reset header");
    e.reset_control_property(child, "header_height").unwrap();
    e.commit();
    assert!(
        !e.explicit_control_properties(child)
            .contains_key("header_height")
    );
    assert_eq!(
        document::decode(&e.document.encode().unwrap()).unwrap(),
        e.document
    );
}
#[test]
fn narrow_titles_clip_to_frame_without_exposing_inline_links() {
    let (mut e, id) = fixture();
    set(
        &mut e,
        id,
        "text",
        Value::Text("[Title](https://example.org)".into()),
    );
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        for width in [58., 59.] {
            e.selected_mut(|c| {
                c.bounds.w = width;
                c.bounds.h = 40.;
            });
            let scene = e.rendered_scene(e.page).unwrap();
            assert!(scene.links.is_empty());
            let glyphs = scene
                .items
                .iter()
                .filter(|i| matches!(i.drawing, Drawing::Glyph(_)))
                .count();
            assert_eq!(glyphs > 0, sketch || width == 58.);
            assert!(
                scene
                    .items
                    .iter()
                    .all(|i| i.clip == Some(Bounds::new(0., 0., width, 40.)))
            );
            assert!(!scene.png().unwrap().is_empty());
            assert!(scene.svg().contains("clipPath"));
        }
    }
}
#[test]
fn scrollbar_thumb_is_visible_only_above_the_height_boundary() {
    let (mut e, id) = fixture();
    set(&mut e, id, "vertical_scrollbar", Value::Bool(true));
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        for height in [94., 95.] {
            e.selected_mut(|c| {
                c.bounds.w = 300.;
                c.bounds.h = height;
            });
            let scene = e.rendered_scene(e.page).unwrap();
            let thumbs=scene.items.iter().filter(|i|matches!(&i.drawing,Drawing::Primitive(wireframe::scene::Primitive::Rect{b,..}) if b.w==8.&&b.h==21.)).count();
            assert_eq!(thumbs, usize::from(height == 95.));
        }
    }
}

#[test]
fn legacy_panel_geometry_text_and_font_remain_explicit_after_migration() {
    let mut legacy = wireframe::model::Document::default();
    let mut panel = wireframe::model::Control::new(2, Kind::Panel, 20., 30.);
    panel.bounds = Bounds::new(20., 30., 300., 220.);
    panel.font_size = 15.;
    panel.text = "Existing panel title".into();
    legacy.pages[0].controls.push(panel);
    legacy.next_id = 3;
    let project = document::Project::try_from(legacy).unwrap();
    let bytes = project.encode().unwrap();
    let restored = document::decode(&bytes).unwrap();
    assert_eq!(restored, project);
    let node = &restored.pages[0].content.nodes[0];
    assert_eq!(node.origin.x, 20.);
    assert_eq!(node.origin.y, 30.);
    let document::NodeContent::Control { kind, size } = &node.content else {
        panic!("control expected")
    };
    assert_eq!(kind, "panel");
    assert_eq!(*size, document::Size::fixed(300., 220.));
    assert_eq!(node.properties["font_size"], Value::Number(15.));
    assert_eq!(
        node.properties["text"],
        Value::Text("Existing panel title".into())
    );
}
