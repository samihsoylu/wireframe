// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    model::{Bounds, Kind},
    render::{Drawing, Scene},
    scene::Primitive,
    workspace::Editor,
};
fn set(e: &mut Editor, id: Id, key: &str, value: Value) {
    e.begin("Edit circle");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
fn choice(e: &mut Editor, id: Id, key: &str, value: &str) {
    set(e, id, key, Value::Choice(value.into()));
}
fn fixture() -> (Editor, Id) {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    e.add(Kind::CircleButton, 50., 50.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
fn bounds(e: &Editor) -> Bounds {
    e.selected_views()[0].bounds
}
fn fixed(e: &mut Editor, w: f32, h: f32) {
    e.begin("Resize");
    e.selected_mut(|c| {
        c.bounds.w = w;
        c.bounds.h = h;
    });
    e.commit();
}
fn scene(e: &Editor) -> Scene {
    e.rendered_scene(e.page).unwrap()
}
fn icon(s: &Scene) -> Option<(Bounds, [u8; 4], u8)> {
    s.items.iter().find_map(|i| match &i.drawing {
        Drawing::Icon {
            bounds,
            color,
            turns,
            ..
        } => Some((*bounds, *color, *turns)),
        _ => None,
    })
}
fn has_text(s: &Scene) -> bool {
    s.items
        .iter()
        .any(|i| matches!(i.drawing, Drawing::Glyph(_)))
}
#[test]
fn circle_default_resize_autosize_history_and_strict_icon_fit() {
    let (mut e, id) = fixture();
    assert_eq!(bounds(&e), Bounds::new(50., 50., 48., 48.));
    assert_eq!(
        e.control_properties(id).unwrap().1["icon"],
        Value::Icon(IconSource::Catalog("plus-solid".into()))
    );
    assert!(wireframe::icons::get("plus-solid").is_some());
    assert!(!has_text(&scene(&e)));
    choice(&mut e, id, "icon_size", "48");
    assert!(icon(&scene(&e)).is_none());
    fixed(&mut e, 49., 48.);
    assert!(icon(&scene(&e)).is_none());
    fixed(&mut e, 49., 49.);
    assert_eq!(
        icon(&scene(&e)).unwrap().0,
        Bounds::new(50.5, 50.5, 48., 48.)
    );
    e.undo();
    assert!(icon(&scene(&e)).is_none());
    e.redo();
    assert!(icon(&scene(&e)).is_some());
    e.auto_size_control(id).unwrap();
    assert_eq!(bounds(&e).w, 48.);
    assert_eq!(bounds(&e).h, 48.);
    choice(&mut e, id, "icon_size", "24");
    set(&mut e, id, "text", Value::Text("Go".into()));
    let baseline = e.document.clone();
    set(&mut e, id, "icon", Value::Null);
    assert!(icon(&scene(&e)).is_none());
    assert!(has_text(&scene(&e)));
    assert_eq!(bounds(&e).w, 48.);
    e.undo();
    assert_eq!(e.document, baseline);
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
}
#[test]
fn circle_label_boundaries_positions_multiline_and_visible_links() {
    let (mut e, id) = fixture();
    set(&mut e, id, "text", Value::Text("Go".into()));
    fixed(&mut e, 39., 39.);
    assert!(!has_text(&scene(&e)));
    assert_eq!(
        icon(&scene(&e)).unwrap().0,
        Bounds::new(57.5, 57.5, 24., 24.)
    );
    fixed(&mut e, 40., 40.);
    assert!(has_text(&scene(&e)));
    assert!(icon(&scene(&e)).unwrap().0.y < 58.);
    fixed(&mut e, 96., 48.);
    let bottom = icon(&scene(&e)).unwrap().0;
    choice(&mut e, id, "label_position", "left");
    let left = icon(&scene(&e)).unwrap().0;
    choice(&mut e, id, "label_position", "right");
    let right = icon(&scene(&e)).unwrap().0;
    assert!(left.x > bottom.x && right.x < bottom.x);
    assert_eq!(left.y, 62.);
    assert_eq!(right.y, 62.);
    assert_eq!(bounds(&e), Bounds::new(50., 50., 96., 48.));
    set(
        &mut e,
        id,
        "text",
        Value::Text("[Go](https://example.org)".into()),
    );
    assert!(!scene(&e).links.is_empty());
    fixed(&mut e, 24., 24.);
    assert!(!has_text(&scene(&e)));
    assert!(scene(&e).links.is_empty());
    choice(&mut e, id, "label_position", "bottom");
    fixed(&mut e, 96., 96.);
    set(&mut e, id, "text", Value::Text("First\nSecond".into()));
    let s = scene(&e);
    assert!(has_text(&s));
    let ys: std::collections::BTreeSet<_> = s
        .items
        .iter()
        .filter_map(|i| {
            if let Drawing::Glyph(g) = &i.drawing {
                Some(g.baseline.round() as i32)
            } else {
                None
            }
        })
        .collect();
    assert!(ys.len() >= 2);
    assert!(s.svg().contains("<ellipse"));
    assert!(s.png().is_ok());
}
#[test]
fn circle_geometry_state_contrast_border_and_rotation() {
    let (mut e, id) = fixture();
    fixed(&mut e, 96., 48.);
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        let t = if sketch { 3. } else { 1. };
        for (state, fill, stroke, ink) in [
            (
                "normal",
                [204; 3],
                if sketch { [0; 3] } else { [51; 3] },
                if sketch { [0; 3] } else { [51; 3] },
            ),
            (
                "selected",
                [149, 188, 242],
                if sketch { [0; 3] } else { [51; 3] },
                if sketch { [0; 3] } else { [51; 3] },
            ),
            (
                "focused",
                [204; 3],
                [149, 188, 242],
                if sketch { [0; 3] } else { [51; 3] },
            ),
            ("disabled", [204; 3], [128, 128, 139], [128, 128, 139]),
        ] {
            choice(&mut e, id, "state", state);
            let s = scene(&e);
            assert_eq!(icon(&s).unwrap().1, [ink[0], ink[1], ink[2], 255]);
            assert!(s.items.iter().any(|i|matches!(i.drawing,Drawing::Primitive(Primitive::StyledEllipse{b,fill:f,stroke:c,width})if b==Bounds::new(74.+t/2.,50.+t/2.,48.-t,48.-t)&&f==fill&&c==stroke&&width==t)));
        }
    }
    choice(&mut e, id, "state", "normal");
    set(&mut e, id, "fill", Value::Color([0, 0, 0, 255]));
    assert_eq!(icon(&scene(&e)).unwrap().1, [255; 4]);
    choice(&mut e, id, "icon_rotation", "90");
    assert_eq!(icon(&scene(&e)).unwrap().2, 1);
    choice(&mut e, id, "border", "none");
    assert!(scene(&e).items.iter().any(|i| matches!(
        i.drawing,
        Drawing::Primitive(Primitive::StyledEllipse { width: 0., .. })
    )));
    assert!(scene(&e).svg().contains("stroke=\"none\""));
    choice(&mut e, id, "state", "disabled");
    assert_eq!(icon(&scene(&e)).unwrap().1, [128, 128, 139, 255]);
}
