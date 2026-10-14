// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    model::{Bounds, Kind},
    render::{Drawing, Scene},
    workspace::{Editor, ImageTarget, ImportChoice},
};
fn set(e: &mut Editor, id: Id, key: &str, v: Value) {
    e.begin("Change icon");
    e.set_control_property(id, key, v).unwrap();
    e.commit();
}
fn choice(e: &mut Editor, id: Id, key: &str, v: &str) {
    set(e, id, key, Value::Choice(v.into()));
}
fn add(kind: Kind) -> (Editor, Id) {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    e.add(kind, 50., 50.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
fn b(e: &Editor) -> Bounds {
    e.selected_views()[0].bounds
}
fn assign(e: &mut Editor, id: Id) {
    set(
        e,
        id,
        "icon",
        Value::Icon(IconSource::Catalog("arrow-right-solid".into())),
    );
}
fn icons(s: &Scene) -> usize {
    s.items
        .iter()
        .filter(|i| matches!(i.drawing, Drawing::Icon { .. }))
        .count()
}
#[test]
fn square_sizes_fallback_removal_and_width_only_labels() {
    let (mut e, id) = add(Kind::Icon);
    assert_eq!((b(&e).w, b(&e).h), (24., 24.));
    assign(&mut e, id);
    for size in [16, 24, 32, 48, 64, 128] {
        choice(&mut e, id, "icon_size", &size.to_string());
        assert_eq!((b(&e).w, b(&e).h), (size as f32, size as f32));
        for angle in ["0", "90", "180", "270"] {
            choice(&mut e, id, "icon_rotation", angle);
            assert_eq!(b(&e).w, size as f32);
        }
    }
    e.begin("Try fixed dimensions");
    e.selected_mut(|c| {
        c.bounds.w = 99.;
        c.bounds.h = 85.;
    });
    e.commit();
    assert_eq!((b(&e).w, b(&e).h), (128., 128.));
    set(&mut e, id, "icon", Value::Null);
    assert_eq!((b(&e).w, b(&e).h), (24., 24.));
    e.undo();
    assert_eq!(b(&e).w, 128.);
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
    let (mut e, id) = add(Kind::IconLabel);
    set(&mut e, id, "text", Value::Text("Go".into()));
    assign(&mut e, id);
    let bottom = b(&e);
    assert_eq!(bottom.w, 48.);
    assert!(bottom.h > 50.);
    choice(&mut e, id, "label_position", "right");
    let side = b(&e);
    assert!(side.w > 51.);
    assert_eq!(side.h, 48.);
    choice(&mut e, id, "label_position", "left");
    assert_eq!(b(&e), side);
    e.begin("Resize width");
    e.selected_mut(|c| {
        c.bounds.w = 160.;
        c.bounds.h = 300.;
    });
    e.commit();
    assert_eq!((b(&e).w, b(&e).h), (160., 48.));
    choice(&mut e, id, "label_position", "bottom");
    assert_eq!(b(&e).w, 160.);
    assert_eq!(b(&e).h, bottom.h);
    set(&mut e, id, "text", Value::Text(String::new()));
    assert!(b(&e).h >= 63.);
    let before = e.document.clone();
    assert!(
        e.set_control_property(id, "label_position", Value::Choice("top".into()))
            .is_err()
    );
    assert_eq!(before, e.document);
}
#[test]
fn narrow_visibility_inset_state_rotation_and_label_links() {
    let (mut e, id) = add(Kind::IconLabel);
    set(
        &mut e,
        id,
        "text",
        Value::Text("[Go](https://example.org)".into()),
    );
    assign(&mut e, id);
    e.begin("Narrow");
    e.selected_mut(|c| c.bounds.w = 47.);
    e.commit();
    assert_eq!(icons(&e.rendered_scene(e.page).unwrap()), 0);
    e.begin("Exact fit");
    e.selected_mut(|c| c.bounds.w = 48.);
    e.commit();
    assert_eq!(icons(&e.rendered_scene(e.page).unwrap()), 1);
    choice(&mut e, id, "border", "solid");
    choice(&mut e, id, "icon_rotation", "90");
    let s = e.rendered_scene(e.page).unwrap();
    assert!(s.items.iter().any(|i|matches!(&i.drawing,Drawing::Icon{bounds,turns,color,..}if *bounds==Bounds::new(54.,54.,40.,40.) && *turns==1 && *color==[51,51,51,255])));
    assert!(s.svg().contains("https://example.org"));
    assert!(s.svg().contains("rotate(-90"));
    assert!(s.png().is_ok());
    choice(&mut e, id, "state", "disabled");
    assert!(
        e.rendered_scene(e.page)
            .unwrap()
            .items
            .iter()
            .any(|i| matches!(&i.drawing,Drawing::Icon{color,..}if *color==[128,128,139,255]))
    );
    choice(&mut e, id, "label_position", "left");
    let s = e.rendered_scene(e.page).unwrap();
    assert_eq!(icons(&s), 1);
    assert!(
        !s.items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Glyph(_)))
    );
    choice(&mut e, id, "label_position", "right");
    e.begin("Too narrow");
    e.selected_mut(|c| c.bounds.w = 35.);
    e.commit();
    let s = e.rendered_scene(e.page).unwrap();
    assert_eq!(icons(&s), 0);
    assert!(
        !s.items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Glyph(_)))
    );
}
#[test]
fn standalone_import_is_atomic_and_survives_components_copy_and_reset() {
    let mut output = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        4,
        2,
        image::Rgba([200, 30, 50, 128]),
    ))
    .write_to(&mut output, image::ImageFormat::Png)
    .unwrap();
    let bytes = output.into_inner();
    for kind in [
        Kind::Icon,
        Kind::IconLabel,
        Kind::CircleButton,
        Kind::PointedButton,
    ] {
        let (mut e, id) = add(kind);
        let baseline = e.document.clone();
        let image = wireframe::images::Prepared::from_bytes("Local.png", &bytes).unwrap();
        let asset = e
            .import_image_at(ImageTarget::Icon(id), image, ImportChoice::KeepBoth)
            .unwrap();
        e.undo();
        assert_eq!(e.document, baseline);
        e.redo();
        assert_eq!(e.document.assets[0].bytes, bytes);
        if kind == Kind::CircleButton {
            e.begin("Enlarge circle");
            e.selected_mut(|c| {
                c.bounds.w = 96.;
                c.bounds.h = 96.;
            });
            e.commit();
        }
        e.create_component("Symbol".into()).unwrap();
        let source = e.document.definitions.clone();
        assert!(e.enter(id));
        let child = e.nodes()[0].id;
        choice(&mut e, child, "icon_size", "64");
        choice(&mut e, child, "icon_rotation", "270");
        choice(&mut e, child, "border", "solid");
        assert_eq!(e.document.definitions, source);
        let scene = e.rendered_scene(e.page).unwrap();
        assert!(scene.svg().contains("data:image/png;base64,"));
        assert!(scene.png().is_ok());
        e.leave();
        e.select(id, false);
        let copy = e.copy();
        assert_eq!(copy.assets.len(), 1);
        let mut other = Editor::default();
        other.paste(&copy, 0.);
        assert!(other.error.is_none());
        assert_eq!(other.document.assets[0].bytes, bytes);
        assert_eq!(
            decode(&other.document.encode().unwrap()).unwrap(),
            other.document
        );
        assert!(e.enter(id));
        e.reset_local_changes(child).unwrap();
        assert_eq!(
            e.control_properties(child).unwrap().1["icon"],
            Value::Icon(IconSource::Asset(asset))
        );
        assert_eq!(e.document.definitions, source);
    }
}
