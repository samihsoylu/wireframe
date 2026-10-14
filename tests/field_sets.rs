// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{self, Value},
    field_set,
    layout::TextMetrics,
    model::{Bounds, Kind},
    render::Drawing,
    workspace::Editor,
};
fn fixture() -> (Editor, u64) {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    e.add(Kind::FieldSet, 0., 0.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
fn set(e: &mut Editor, id: u64, key: &str, value: Value) {
    e.begin("Edit section");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
#[test]
fn measured_label_controls_border_cutout_and_initial_width() {
    for (w, h) in [
        (0., 13.),
        (0., 24.),
        (74., 13.),
        (170., 13.),
        (171., 13.),
        (212., 13.),
        (391., 24.),
        (146., 29.),
        (160., 32.),
    ] {
        let m = TextMetrics {
            width: w,
            height: h,
            rows: 1,
        };
        assert_eq!(field_set::intrinsic(m), ((w + 30.).max(200.), 170.));
        for (bw, bh) in [
            (200., 170.),
            (90., 30.),
            (1., 1.),
            (200., 7.),
            (200., 8.),
            (200., 9.),
        ] {
            let l = field_set::layout(bw, bh, m);
            assert_eq!(l.label, Bounds::new(15., 0., w, h));
            assert_eq!(l.cutout, Bounds::new(12., 0., w + 6., h));
            assert_eq!(l.border, Bounds::new(0., h / 2., bw, (bh - h / 2.).max(1.)));
        }
    }
}
#[test]
fn contrast_has_an_opacity_gate_and_strict_brightness_boundary() {
    for opacity in [0., 0.749, 0.75, 1.] {
        assert_eq!(field_set::light_text([0; 3], opacity), opacity >= 0.75);
        assert_eq!(field_set::light_text([152; 3], opacity), opacity >= 0.75);
        assert!(!field_set::light_text([153; 3], opacity));
        assert!(!field_set::light_text([255; 3], opacity));
    }
}
#[test]
fn automatic_then_fixed_sizes_preserve_source_and_history() {
    let (mut e, id) = fixture();
    let baseline = e.document.clone();
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(0., 0., 200., 170.)
    );
    let (spec, p) = e.control_properties(id).unwrap();
    assert_eq!(spec.id, "field_set");
    assert!(spec.property("background_opacity").unwrap().visible);
    assert_eq!(p["font_size"], Value::Number(13.));
    let raw = "A sufficiently long form section title that needs more than two hundred units";
    set(&mut e, id, "text", Value::Text(raw.into()));
    assert!(e.selected_views()[0].bounds.w > 200.);
    e.undo();
    assert_eq!(e.document, baseline);
    e.redo();
    e.begin("Resize section");
    e.selected_mut(|c| c.bounds = Bounds::new(0., 0., 90., 30.));
    e.commit();
    set(&mut e, id, "font_size", Value::Number(24.));
    set(&mut e, id, "text", Value::Text(String::new()));
    assert_eq!(e.selected_views()[0].bounds, Bounds::new(0., 0., 90., 30.));
    assert!(e.auto_size_control(id).is_err());
    assert_eq!(
        document::decode(&e.document.encode().unwrap()).unwrap(),
        e.document
    );
}
#[test]
fn component_changes_are_local_resettable_and_invalid_edits_are_atomic() {
    let (mut e, id) = fixture();
    e.create_component("Form section".into()).unwrap();
    let defs = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    set(&mut e, child, "background_opacity", Value::Number(0.5));
    set(
        &mut e,
        child,
        "text",
        Value::Text("*Contact*\\roptional {mockup-name}".into()),
    );
    assert_eq!(e.document.definitions, defs);
    let before = e.document.clone();
    assert!(
        e.set_control_property(child, "background_opacity", Value::Number(1.01))
            .is_err()
    );
    assert_eq!(e.document, before);
    e.begin("Reset opacity");
    e.reset_control_property(child, "background_opacity")
        .unwrap();
    e.commit();
    assert!(
        !e.explicit_control_properties(child)
            .contains_key("background_opacity")
    );
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(
        document::decode(&e.document.encode().unwrap()).unwrap(),
        e.document
    );
}
#[test]
fn soft_lines_literal_macros_and_narrow_clipping_share_the_scene() {
    let (mut e, id) = fixture();
    set(
        &mut e,
        id,
        "text",
        Value::Text("Account *details*\\roptional {mockup-name}".into()),
    );
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
    assert!(glyphs.iter().any(|g| g.bold));
    assert!(
        glyphs
            .iter()
            .map(|g| g.text.as_str())
            .collect::<String>()
            .contains("{mockup-name}")
    );
    assert!(glyphs.iter().any(|g| g.baseline > glyphs[0].baseline));
    e.selected_mut(|c| c.bounds = Bounds::new(0., 0., 90., 30.));
    let s = e.rendered_scene(e.page).unwrap();
    for item in &s.items {
        let clip = item.clip.unwrap();
        assert!(clip.x >= 0. && clip.y >= 0. && clip.x + clip.w <= 90. && clip.y + clip.h <= 30.);
    }
    assert!(field_set::parse(&"x".repeat(100_001)).is_err());
}
#[test]
fn empty_label_hole_and_interior_reveal_an_underlying_color() {
    let (mut e, back) = fixture();
    set(&mut e, back, "fill", Value::Color([30, 170, 80, 255]));
    e.add(Kind::FieldSet, 0., 0.);
    let front = *e.selection.iter().next().unwrap();
    set(&mut e, front, "text", Value::Text(String::new()));
    set(&mut e, front, "background_opacity", Value::Number(0.));
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        let s = e.rendered_scene(e.page).unwrap();
        let image = image::load_from_memory(&s.png().unwrap())
            .unwrap()
            .to_rgba8();
        assert_eq!(image.get_pixel(24 + 15, 24 + 8).0, [30, 170, 80, 255]);
        assert_eq!(image.get_pixel(24 + 100, 24 + 100).0, [30, 170, 80, 255]);
        assert!((6..11).any(|y| image.get_pixel(24 + 4, 24 + y).0[1] < 100));
        assert!(s.svg().contains("clipPath"));
    }
}
#[test]
fn fill_opacity_does_not_fade_text_or_border() {
    let (mut e, id) = fixture();
    set(&mut e, id, "fill", Value::Color([0, 0, 0, 255]));
    for (opacity, ink) in [(1., [255; 3]), (0.5, [51; 3])] {
        set(&mut e, id, "background_opacity", Value::Number(opacity));
        let s = e.rendered_scene(e.page).unwrap();
        assert_eq!(s.items[0].opacity, opacity as f32);
        assert!(s.items.iter().skip(1).all(|i| i.opacity == 1.));
        assert!(
            s.items
                .iter()
                .filter_map(|i| if let Drawing::Glyph(g) = &i.drawing {
                    Some(g)
                } else {
                    None
                })
                .all(|g| g.color[..3] == ink)
        );
    }
}
