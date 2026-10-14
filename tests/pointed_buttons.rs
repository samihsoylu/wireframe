// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    model::{Bounds, Kind},
    render::Drawing,
    scene::Primitive,
    workspace::Editor,
};
fn edit(e: &mut Editor, id: Id, key: &str, value: Value) {
    e.begin("Edit pointed button");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
fn choice(e: &mut Editor, id: Id, key: &str, value: &str) {
    edit(e, id, key, Value::Choice(value.into()));
}
fn fixture() -> (Editor, Id) {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    e.add(Kind::PointedButton, 50., 50.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
fn bounds(e: &Editor) -> Bounds {
    e.selected_views()[0].bounds
}
#[test]
fn pointed_defaults_direction_menu_width_and_horizontal_resize() {
    let (mut e, id) = fixture();
    let baseline = bounds(&e);
    assert_eq!(baseline.h, 27.);
    let values = e.control_properties(id).unwrap().1;
    assert_eq!(values["direction"], Value::Choice("left".into()));
    assert_eq!(values["border"], Value::Choice("none".into()));
    choice(&mut e, id, "direction", "center");
    assert_eq!(bounds(&e).w, baseline.w - 10.);
    choice(&mut e, id, "direction", "right");
    assert_eq!(bounds(&e), baseline);
    edit(&mut e, id, "menu_icon", Value::Bool(true));
    assert_eq!(bounds(&e).w, baseline.w + 13.);
    edit(
        &mut e,
        id,
        "icon",
        Value::Icon(IconSource::Catalog("arrow-right-solid".into())),
    );
    choice(&mut e, id, "icon_size", "48");
    assert_eq!(bounds(&e).w, baseline.w + 64.);
    assert_eq!(bounds(&e).h, 48.);
    e.begin("Horizontal resize");
    e.selected_mut(|c| {
        c.bounds.w = 35.;
        c.bounds.h = 90.;
    });
    e.commit();
    assert_eq!(bounds(&e).w, 35.);
    assert_eq!(bounds(&e).h, 48.);
    let s = e.rendered_scene(e.page).unwrap();
    let i = s
        .items
        .iter()
        .find(|i| matches!(i.drawing, Drawing::Icon { .. }))
        .unwrap();
    assert_eq!(i.clip, Some(bounds(&e)));
    assert!(matches!(i.drawing,Drawing::Icon{bounds:b,..} if b.w==48. && b.h==48.));
    let saved = e.document.clone();
    e.undo();
    assert!(bounds(&e).w > 35.);
    e.redo();
    assert_eq!(saved, e.document);
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), saved);
    e.auto_size_control(id).unwrap();
    assert_eq!(bounds(&e).w, baseline.w + 64.);
}
#[test]
fn pointed_styles_contrast_rotation_and_control_link_without_inline_hotspots() {
    let (mut e, id) = fixture();
    edit(
        &mut e,
        id,
        "icon",
        Value::Icon(IconSource::Catalog("arrow-right-solid".into())),
    );
    choice(&mut e, id, "icon_rotation", "90");
    edit(
        &mut e,
        id,
        "text",
        Value::Text("[Go](https://example.org/inline)".into()),
    );
    edit(
        &mut e,
        id,
        "link",
        Value::Link(Link::Url("https://example.org/control".into())),
    );
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        choice(&mut e, id, "border", "solid");
        edit(&mut e, id, "fill", Value::Color([0, 0, 0, 255]));
        choice(&mut e, id, "state", "normal");
        let s = e.rendered_scene(e.page).unwrap();
        assert_eq!(s.links.len(), 1);
        assert_eq!(s.links[0].target, "https://example.org/control");
        assert!(s.items.iter().any(|i| matches!(
            i.drawing,
            Drawing::Icon {
                color: [255, 255, 255, 255],
                turns: 1,
                ..
            }
        )));
        assert!(
            s.items
                .iter()
                .any(|i| matches!(i.drawing, Drawing::Primitive(Primitive::Polygon { .. })))
        );
        choice(&mut e, id, "state", "disabled");
        let s = e.rendered_scene(e.page).unwrap();
        assert!(s.items.iter().any(|i| matches!(
            i.drawing,
            Drawing::Icon {
                color: [128, 128, 139, 255],
                ..
            }
        )));
        let expected = if sketch { [153; 3] } else { [128, 128, 139] };
        assert!(s.items.iter().any(|i|matches!(&i.drawing,Drawing::Primitive(Primitive::Line{color,..}) if *color==expected)));
        assert!(s.svg().contains("Font Awesome"));
        assert!(s.png().is_ok());
    }
}
