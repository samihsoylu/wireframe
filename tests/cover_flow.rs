// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::io::Cursor;
mod support;
use wireframe::{
    cover_flow,
    document::*,
    images::Prepared,
    model::{Bounds, Kind},
    render::Drawing,
    workspace::{Editor, ImportChoice},
};
fn fixture() -> (Editor, Id) {
    let mut e = Editor::default();
    e.add(Kind::CoverFlow, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
fn image() -> Prepared {
    let mut img = image::RgbaImage::from_fn(160, 100, |x, y| {
        image::Rgba(match (x < 80, y < 50) {
            (true, true) => [240, 40, 30, 255],
            (false, true) => [30, 210, 70, 255],
            (true, false) => [30, 70, 240, 255],
            _ => [250, 190, 20, 255],
        })
    });
    // A transparent sample keeps the importer on its lossless PNG path.
    img.put_pixel(0, 0, image::Rgba([0, 0, 0, 0]));
    let mut bytes = Cursor::new(Vec::new());
    img.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    Prepared::from_bytes("Quadrants.png", &bytes.into_inner()).unwrap()
}
#[test]
fn defaults_and_scrollbar_boundaries() {
    let (e, _) = fixture();
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(20., 30., 493., 198.)
    );
    let a = cover_flow::layout(493., 198., true);
    assert_eq!(a.frame, Bounds::new(0., 0., 493., 182.));
    assert_eq!(a.scrollbar, Some(Bounds::new(62., 182., 370., 16.)));
    assert!((a.image.w - 152.83).abs() < 0.001);
    assert!((a.image.h - 141.96).abs() < 0.001);
    assert!(cover_flow::layout(493., 16., true).scrollbar.is_none());
    assert_eq!(cover_flow::layout(493., 17., true).frame.h, 1.);
    for (w, thumb) in [(60., false), (61., true)] {
        let bar = cover_flow::layout(w, 50., true).scrollbar.unwrap();
        assert_eq!(
            wireframe::linear_controls::scrollbar(false, bar.w, 0.5, false)
                .thumb
                .is_some(),
            thumb
        );
    }
    let bg = wireframe::linear_controls::scrollbar(false, 1., 0., false)
        .background
        .unwrap();
    assert_eq!(bg, Bounds::new(2., 2., 0., 12.));
    let sketch_bg = wireframe::linear_controls::scrollbar(false, 1., 0., true)
        .background
        .unwrap();
    assert_eq!(sketch_bg, Bounds::new(1., 1., 1., 14.));
}
#[test]
fn image_rotation_keeps_frame_and_asset_and_roundtrips_history() {
    let (mut e, id) = fixture();
    e.import_image(id, image(), ImportChoice::KeepBoth).unwrap();
    let assets = e.document.assets.clone();
    let before = e.document.clone();
    let bounds = e.selected_views()[0].bounds;
    e.rotate_image(id).unwrap();
    assert_eq!(e.selected_views()[0].bounds, bounds);
    assert_eq!(
        e.control_properties(id).unwrap().1["rotation"],
        Value::Number(270.)
    );
    assert_eq!(e.document.assets, assets);
    let scene = e.rendered_scene(e.page).unwrap();
    let Drawing::Image { bounds: ib, data } = &scene.items[0].drawing else {
        panic!("image must precede outline")
    };
    assert!((ib.x - 190.085).abs() < 0.001);
    assert_eq!(data.pixels.size, [100, 160]);
    assert_eq!(
        image::load_from_memory(&data.png)
            .unwrap()
            .to_rgba8()
            .get_pixel(10, 10)
            .0,
        [30, 210, 70, 255]
    );
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), after);
    assert!(e.crop_image(id, Some(Crop::ALL)).is_err());
}
#[test]
fn removal_filter_and_missing_asset_preserve_frame_without_warning_text() {
    let (mut e, id) = fixture();
    let empty = e.rendered_scene(e.page).unwrap().svg();
    e.import_image(id, image(), ImportChoice::KeepBoth).unwrap();
    e.begin("Filter");
    e.set_control_property(id, "sketch_filter", Value::Bool(true))
        .unwrap();
    e.commit();
    let original = e.document.clone();
    let scene = e.rendered_scene(e.page).unwrap();
    let Drawing::Image { data, .. } = &scene.items[0].drawing else {
        panic!()
    };
    let px = data.pixels.pixels[30 * 164 + 40].to_array();
    assert_eq!(px[0], px[1]);
    assert_eq!(px[1], px[2]);
    e.assign_image(id, None).unwrap();
    assert_eq!(e.rendered_scene(e.page).unwrap().svg(), empty);
    e.undo();
    assert_eq!(e.document, original);
    e.redo();
    assert_eq!(e.rendered_scene(e.page).unwrap().svg(), empty);
    e.undo();
    e.document.assets.clear();
    assert_eq!(e.rendered_scene(e.page).unwrap().svg(), empty);
}
#[test]
fn image_and_frame_share_clipping_and_exports_in_both_styles() {
    for sketch in [false, true] {
        for (w, h) in [
            (1., 1.),
            (60., 50.),
            (61., 50.),
            (493., 16.),
            (493., 17.),
            (493., 198.),
            (800., 100.),
        ] {
            let (mut e, id) = fixture();
            e.document.settings.sketch = sketch;
            e.import_image(id, image(), ImportChoice::KeepBoth).unwrap();
            e.set_control_property(id, "vertical_scrollbar", Value::Bool(true))
                .unwrap();
            support::stored_size(&mut e, w, h);
            let scene = e.rendered_scene(e.page).unwrap();
            assert!(scene.links.is_empty());
            for item in &scene.items {
                let c = item.clip.unwrap();
                assert!(
                    c.x >= 20.
                        && c.y >= 30.
                        && c.x + c.w <= 20. + w + 0.001
                        && c.y + c.h <= 30. + h + 0.001
                );
            }
            let png = image::load_from_memory(&scene.png().unwrap()).unwrap();
            assert_eq!((png.width(), png.height()), (w as u32 + 48, h as u32 + 48));
            assert!(scene.svg().contains("data:image/png;base64,"));
        }
    }
}
#[test]
fn component_image_edits_are_local_and_locked_import_is_rejected() {
    let (mut e, id) = fixture();
    e.import_image(id, image(), ImportChoice::KeepBoth).unwrap();
    e.create_component("Carousel".into()).unwrap();
    let definitions = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    e.select(child, false);
    e.rotate_image(child).unwrap();
    e.assign_image(child, None).unwrap();
    assert_eq!(e.document.definitions, definitions);
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
    let (mut e, child) = fixture();
    e.lock(true);
    let before = e.document.clone();
    assert!(
        e.import_image(child, image(), ImportChoice::KeepBoth)
            .is_err()
    );
    assert_eq!(before, e.document);
}
