// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    model::Kind,
    render::Drawing,
    workspace::{Editor, ImageTarget, ImportChoice},
};
fn set(e: &mut Editor, id: Id, key: &str, value: Value) {
    e.begin("Edit icon");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
fn choice(e: &mut Editor, id: Id, key: &str, value: &str) {
    set(e, id, key, Value::Choice(value.into()));
}
fn fixture(kind: Kind) -> (Editor, Id) {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    e.add(kind, 50., 50.);
    let id = *e.selection.iter().next().unwrap();
    set(&mut e, id, "text", Value::Text("Go".into()));
    (e, id)
}
fn assign(e: &mut Editor, id: Id) {
    set(
        e,
        id,
        "icon",
        Value::Icon(IconSource::Catalog("arrow-right-solid".into())),
    );
}
fn bounds(e: &Editor) -> wireframe::model::Bounds {
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
fn asset(e: &Editor, id: Id) -> Id {
    match e.control_properties(id).unwrap().1["icon"] {
        Value::Icon(IconSource::Asset(id)) => id,
        _ => panic!(),
    }
}
fn prepared(name: &str) -> wireframe::images::Prepared {
    let im = image::RgbaImage::from_fn(4, 2, |x, y| {
        image::Rgba(if x < 2 {
            if y == 0 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 128]
            }
        } else {
            [0, 255, 0, 255]
        })
    });
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(im)
        .write_to(&mut out, image::ImageFormat::Png)
        .unwrap();
    wireframe::images::Prepared::from_bytes(name, &out.into_inner()).unwrap()
}
#[test]
fn optional_sizes_menu_arrows_history_defaults_and_validation() {
    for kind in [Kind::Button, Kind::Dropdown] {
        let (mut e, id) = fixture(kind);
        let base = bounds(&e);
        let no_icon = e.document.clone();
        assign(&mut e, id);
        e.undo();
        assert_eq!(e.document, no_icon);
        e.redo();
        for size in [16, 24, 32, 48, 64, 128] {
            choice(&mut e, id, "icon_size", &size.to_string());
            let b = bounds(&e);
            assert_eq!(
                b.w,
                base.w + size as f32 + if kind == Kind::Button { 3. } else { 0. }
            );
            assert_eq!(b.h, base.h.max(size as f32));
            let before = b;
            for angle in ["0", "90", "180", "270"] {
                choice(&mut e, id, "icon_rotation", angle);
                assert_eq!(bounds(&e), before);
            }
        }
        assert!(
            e.set_control_property(id, "icon_size", Value::Choice("17".into()))
                .is_err()
        );
        assert!(
            e.set_control_property(
                id,
                "icon",
                Value::Icon(IconSource::Catalog("bad/id".into()))
            )
            .is_err()
        );
        assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
        e.begin("Reset icon");
        e.reset_control_property(id, "icon").unwrap();
        e.commit();
        assert_eq!(bounds(&e), base);
        e.undo();
        assert!(matches!(
            e.control_properties(id).unwrap().1["icon"],
            Value::Icon(_)
        ));
    }
    let (mut e, id) = fixture(Kind::Button);
    let base = bounds(&e);
    set(&mut e, id, "menu_icon", Value::Bool(true));
    assert_eq!(bounds(&e).w, base.w + 19.);
    assert_eq!(
        [13., 28., 60.].map(|s| wireframe::icons::menu_size(true, s)),
        [10., 12., 24.]
    );
    set(&mut e, id, "text", Value::Text(String::new()));
    assign(&mut e, id);
    choice(&mut e, id, "icon_size", "32");
    assert_eq!(bounds(&e).w, 64.);
}
#[test]
fn button_fit_rotation_alignment_and_shared_exports() {
    let (mut e, id) = fixture(Kind::Button);
    let text_width = bounds(&e).w - 22.;
    assign(&mut e, id);
    choice(&mut e, id, "icon_size", "32");
    set(&mut e, id, "menu_icon", Value::Bool(true));
    let threshold = 32. + text_width + 10. + 6. + 3.;
    fixed(&mut e, threshold - 1., 32.);
    let s = e.rendered_scene(e.page).unwrap();
    assert!(
        !s.items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Icon { .. }))
    );
    fixed(&mut e, threshold, 32.);
    let s = e.rendered_scene(e.page).unwrap();
    assert_eq!(
        s.items
            .iter()
            .filter(|i| matches!(i.drawing, Drawing::Icon { .. }))
            .count(),
        1
    );
    fixed(&mut e, 160., 50.);
    choice(&mut e, id, "align", "right");
    choice(&mut e, id, "icon_rotation", "90");
    let s = e.rendered_scene(e.page).unwrap();
    let item = s
        .items
        .iter()
        .find(|i| matches!(i.drawing, Drawing::Icon { .. }))
        .unwrap();
    assert!(item.clip.is_some());
    if let Drawing::Icon {
        bounds,
        turns,
        color,
        ..
    } = &item.drawing
    {
        assert_eq!(*turns, 1);
        assert!(bounds.x > 100.);
        assert_eq!(bounds.y, 59.);
        assert_eq!(*color, [51, 51, 51, 255]);
    }
    let svg = s.svg();
    assert!(svg.contains("rotate(-90"));
    assert!(svg.contains("CC BY 4.0"));
    assert!(s.png().is_ok());
    choice(&mut e, id, "state", "disabled");
    let s = e.rendered_scene(e.page).unwrap();
    assert!(
        s.items
            .iter()
            .any(|i| matches!(&i.drawing,Drawing::Icon{color,..} if *color==[128,128,139,255]))
    );
    choice(&mut e, id, "state", "normal");
    set(&mut e, id, "fill", Value::Color([0, 0, 0, 255]));
    let s = e.rendered_scene(e.page).unwrap();
    assert!(
        s.items
            .iter()
            .any(|i| matches!(&i.drawing,Drawing::Icon{color,..} if *color==[255;4]))
    );
}
#[test]
fn dropdown_header_moves_menu_and_keeps_narrow_icons() {
    let (mut e, id) = fixture(Kind::Dropdown);
    set(
        &mut e,
        id,
        "text",
        Value::Text("Choose\nAlpha\nBravo".into()),
    );
    assign(&mut e, id);
    choice(&mut e, id, "icon_size", "32");
    let small = bounds(&e);
    let a = e.rendered_scene(e.page).unwrap();
    let glyph_y = |s: &wireframe::render::Scene| {
        s.items
            .iter()
            .filter_map(|i| match &i.drawing {
                Drawing::Glyph(g) if g.text == "A" => Some(g.baseline),
                _ => None,
            })
            .next()
            .unwrap()
    };
    assert!(
        a.items
            .iter()
            .any(|i| matches!(&i.drawing,Drawing::Icon{color,..} if *color==[51,51,51,255]))
    );
    let y = glyph_y(&a);
    choice(&mut e, id, "icon_size", "64");
    let big = bounds(&e);
    assert_eq!(big.w - small.w, 32.);
    assert_eq!(big.h - small.h, 32.);
    assert_eq!(glyph_y(&e.rendered_scene(e.page).unwrap()) - y, 32.);
    choice(&mut e, id, "state", "focused_closed");
    assert_eq!(bounds(&e).h, 64.);
    let s = e.rendered_scene(e.page).unwrap();
    assert!(
        s.items
            .iter()
            .any(|i| matches!(&i.drawing,Drawing::Icon{color,..} if *color==[151,189,240,255]))
    );
    choice(&mut e, id, "icon_size", "32");
    fixed(&mut e, 35., 32.);
    let s = e.rendered_scene(e.page).unwrap();
    assert!(
        s.items
            .iter()
            .any(|i| matches!(&i.drawing,Drawing::Icon{bounds,..} if bounds.w==32.))
    );
    choice(&mut e, id, "state", "disabled_closed");
    assert!(
        e.rendered_scene(e.page)
            .unwrap()
            .items
            .iter()
            .any(|i| matches!(&i.drawing,Drawing::Icon{color,..} if *color==[128,128,139,255]))
    );
}
#[test]
fn custom_control_icons_import_rotate_component_copy_and_reset() {
    let (mut e, id) = fixture(Kind::Button);
    let source_bytes = prepared("Source.png").bytes;
    let source = e
        .import_image_at(
            ImageTarget::Icon(id),
            prepared("Source.png"),
            ImportChoice::KeepBoth,
        )
        .unwrap();
    e.create_component("Action".into()).unwrap();
    let definitions = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    let local = e
        .import_image_at(
            ImageTarget::Icon(child),
            prepared("Local.png"),
            ImportChoice::KeepBoth,
        )
        .unwrap();
    assert_ne!(source, local);
    choice(&mut e, child, "icon_rotation", "90");
    assert_eq!(e.document.definitions, definitions);
    let s = e.rendered_scene(e.page).unwrap();
    let data = s
        .items
        .iter()
        .find_map(|i| match &i.drawing {
            Drawing::Image { data, .. } => Some(data),
            _ => None,
        })
        .unwrap();
    assert_eq!(data.pixels.size, [2, 4]);
    let tinted = wireframe::images::decoded_tinted(&source_bytes, [51; 3]).unwrap();
    let original = image::load_from_memory(&tinted.png).unwrap().to_rgba8();
    let actual = image::load_from_memory(&data.png).unwrap().to_rgba8();
    assert_eq!(actual, image::imageops::rotate270(&original));
    assert!(s.svg().contains("data:image/png;base64,"));
    assert!(!s.svg().contains("Font Awesome"));
    assert!(s.png().is_ok());
    e.leave();
    e.select(id, false);
    let copy = e.copy();
    assert_eq!(copy.assets.len(), 2);
    let (mut other, other_id) = fixture(Kind::Dropdown);
    other
        .import_image_at(
            ImageTarget::Icon(other_id),
            prepared("Collision.png"),
            ImportChoice::KeepBoth,
        )
        .unwrap();
    other.paste(&copy, 0.);
    assert!(other.error.is_none());
    let pasted = *other.selection.iter().next().unwrap();
    assert!(other.enter(pasted));
    let ch = other.nodes()[0].id;
    let transferred = asset(&other, ch);
    assert!(
        other
            .document
            .assets
            .iter()
            .any(|a| a.id == transferred && a.name == "Local.png")
    );
    assert_eq!(
        decode(&other.document.encode().unwrap()).unwrap(),
        other.document
    );
    assert!(e.enter(id));
    e.reset_local_changes(child).unwrap();
    assert_eq!(asset(&e, child), source);
    assert_eq!(e.document.definitions, definitions);
    let before = e.document.clone();
    assert!(
        e.import_image_at(
            ImageTarget::Icon(999999),
            prepared("Invalid.png"),
            ImportChoice::KeepBoth
        )
        .is_err()
    );
    assert_eq!(e.document, before);
    let mut invalid = e.document.clone();
    invalid.assets.retain(|a| a.id != source);
    assert!(invalid.validate().is_err());
}
