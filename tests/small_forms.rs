// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    catalog,
    document::{self, Dimension, Link, NodeContent, Value},
    layout::TextMetrics,
    model::{Bounds, Kind},
    render::Drawing,
    scene::Primitive,
    small_forms as forms,
    workspace::Editor,
};
fn fixture(kind: Kind) -> (Editor, u64) {
    let mut e = Editor::default();
    e.add(kind, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
#[test]
fn stepper_text_sizes_history_and_saved_tiny_bounds() {
    let (mut e, id) = fixture(Kind::NumericStepper);
    assert_eq!(e.selected_views()[0].text, "3");
    let original = e.document.clone();
    let natural = e.selected_views()[0].bounds;
    e.edit("Change text", |e| {
        e.set_control_property(id, "text", Value::Text("-12345.75".into()))
            .unwrap()
    });
    assert!(e.selected_views()[0].bounds.w > natural.w);
    assert_eq!(e.selected_views()[0].bounds.h, natural.h);
    let edited = e.document.clone();
    e.undo();
    assert_eq!(e.document, original);
    e.redo();
    assert_eq!(e.document, edited);
    if let NodeContent::Control { size, .. } = &mut e.document.pages[0].content.nodes[0].content {
        size.width = Dimension::Fixed(1.);
        size.height = Dimension::Fixed(100.);
    }
    assert_eq!(e.selected_views()[0].bounds.w, 1.);
    assert_eq!(e.selected_views()[0].bounds.h, natural.h);
    let saved = e.document.encode().unwrap();
    assert_eq!(document::decode(&saved).unwrap(), e.document);
    assert!(
        e.rendered_scene(e.page)
            .unwrap()
            .items
            .iter()
            .all(|i| i.clip.is_some_and(|c| c.w <= 1.))
    );
    e.edit("Fit", |e| e.auto_size_control(id).unwrap());
    assert!(e.selected_views()[0].bounds.w > natural.w);
    e.undo();
    assert_eq!(e.document.encode().unwrap(), saved);
    e.set_control_property(id, "text", Value::Text(String::new()))
        .unwrap();
    assert!(e.selected_views()[0].text.is_empty());
    assert!(
        !e.rendered_scene(e.page)
            .unwrap()
            .items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Glyph(_)))
    );
}
#[test]
fn stepper_rich_text_keeps_literal_macros_and_does_not_register_text_links() {
    let (mut e, id) = fixture(Kind::NumericStepper);
    e.set_control_property(
        id,
        "text",
        Value::Text("One\n{mockup-name} [Link](https://example.com)".into()),
    )
    .unwrap();
    let scene = e.rendered_scene(e.page).unwrap();
    assert!(scene.links.is_empty());
    assert!(
        scene
            .items
            .iter()
            .any(|i| matches!(&i.drawing,Drawing::Glyph(g) if g.text.contains("{mockup-name}")))
    );
    let mut baselines: Vec<_> = scene
        .items
        .iter()
        .filter_map(|i| {
            if let Drawing::Glyph(g) = &i.drawing {
                Some(g.baseline)
            } else {
                None
            }
        })
        .collect();
    baselines.sort_by(f32::total_cmp);
    baselines.dedup();
    assert_eq!(baselines.len(), 2);
    e.set_control_property(id, "state", Value::Choice("disabled".into()))
        .unwrap();
    let scene = e.rendered_scene(e.page).unwrap();
    assert!(
        scene
            .items
            .iter()
            .filter_map(|i| if let Drawing::Glyph(g) = &i.drawing {
                Some(g.color)
            } else {
                None
            })
            .all(|c| c[..3] == [128, 128, 139])
    );
}
#[test]
fn fixed_controls_ignore_saved_dimensions_and_expose_only_supported_fields() {
    for (kind, key, size, fields) in [
        (Kind::ColorPicker, "color_picker", (26., 28.), vec!["fill"]),
        (Kind::HelpButton, "help_button", (18., 18.), vec!["link"]),
    ] {
        let (mut e, id) = fixture(kind);
        assert_eq!(forms::fixed_size(key), Some(size));
        if let NodeContent::Control { size, .. } = &mut e.document.pages[0].content.nodes[0].content
        {
            size.width = Dimension::Fixed(100.);
            size.height = Dimension::Fixed(1.);
        }
        assert_eq!(
            (
                e.selected_views()[0].bounds.w,
                e.selected_views()[0].bounds.h
            ),
            size
        );
        assert!(e.prepare_text_edit(id).unwrap().is_none());
        assert!(e.auto_size_control(id).is_err());
        assert_eq!(
            catalog::get(key)
                .unwrap()
                .properties
                .iter()
                .filter(|p| p.visible)
                .map(|p| p.key)
                .collect::<Vec<_>>(),
            fields
        );
        assert_eq!(
            document::decode(&e.document.encode().unwrap()).unwrap(),
            e.document
        );
    }
}
#[test]
fn swatch_and_help_link_render_consistently_with_clipping() {
    let (mut e, id) = fixture(Kind::ColorPicker);
    e.set_control_property(id, "fill", Value::Color([255, 102, 0, 255]))
        .unwrap();
    for sketch in [true, false] {
        e.document.settings.sketch = sketch;
        let s = e.rendered_scene(e.page).unwrap();
        assert!(s.items.iter().any(|i|matches!(&i.drawing,Drawing::Primitive(Primitive::Rect{fill,..}) if *fill==[255,102,0])));
        assert!(s.links.is_empty());
    }
    let (mut e, id) = fixture(Kind::HelpButton);
    e.set_control_property(
        id,
        "link",
        Value::Link(Link::Url("https://example.com".into())),
    )
    .unwrap();
    let s = e.rendered_scene(e.page).unwrap();
    assert_eq!(s.hit_link(29., 39.), Some("https://example.com"));
    assert!(s.hit_link(40., 39.).is_none());
    assert!(s.svg().contains("https://example.com"));
}
#[test]
fn original_frame_mapping_preserves_corners_and_compresses_tiny_frames() {
    for sketch in [true, false] {
        let h = if sketch { 26. } else { 24. };
        for (w, hh) in [(1., 1.), (10., 10.), (44., h), (200., 80.)] {
            assert_eq!(forms::stepper_point([0., 0.], w, hh, sketch), [0., 0.]);
            assert_eq!(forms::stepper_point([44., h], w, hh, sketch), [w, hh]);
            let mut prior = 0.;
            for x in 0..=44 {
                let p = forms::stepper_point([x as f32, h / 2.], w, hh, sketch);
                assert!(
                    p[0] >= prior && p[0] <= w,
                    "sketch={sketch} width={w} height={hh} x={x} prior={prior} current={}",
                    p[0]
                );
                prior = p[0];
            }
        }
    }
    assert_eq!(
        forms::stepper_size(
            TextMetrics {
                width: 8.,
                height: 13.,
                rows: 1
            },
            None
        ),
        (41., 24.)
    );
    assert_eq!(forms::stepper_text(1., 1.), Bounds::new(9., 6., 1., 1.));
    assert_eq!(
        forms::stepper_button(1., 24.),
        Bounds::new(-17., 0., 18., 24.)
    );
    assert_eq!(
        forms::stepper_colors(false, true, [255, 0, 0]).border,
        [205, 204, 204]
    );
}
#[test]
fn component_overrides_keep_sources_unchanged_and_history_intact() {
    for (kind, key, value) in [
        (Kind::NumericStepper, "text", Value::Text("12.5".into())),
        (Kind::ColorPicker, "fill", Value::Color([255, 102, 0, 255])),
        (Kind::HelpButton, "link", Value::Link(Link::Back)),
    ] {
        let (mut e, _) = fixture(kind);
        e.create_component("Form control".into()).unwrap();
        let definition = e.document.definitions[0].clone();
        let instance = *e.selection.iter().next().unwrap();
        assert!(e.enter(instance));
        let id = e.nodes()[0].id;
        e.selection.insert(id);
        let before = e.document.clone();
        e.edit("Change instance", |e| {
            e.set_control_property(id, key, value.clone()).unwrap()
        });
        assert_eq!(e.document.definitions[0], definition);
        assert_eq!(e.control_properties(id).unwrap().1[key], value);
        assert!(e.rendered_scene(e.surface_id()).is_ok());
        assert_eq!(
            document::decode(&e.document.encode().unwrap()).unwrap(),
            e.document
        );
        e.undo();
        assert_eq!(e.document, before);
    }
}
