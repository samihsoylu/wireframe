// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    model::Bounds,
    render::{Drawing, Item, Scene, mask::BottomMask},
    scene::Primitive,
};
fn item(drawing: Drawing) -> Item {
    Item {
        drawing,
        opacity: 1.,
        clip: None,
    }
}
fn rect(b: Bounds, color: u8) -> Drawing {
    Drawing::Primitive(Primitive::Rect {
        b,
        fill: [color; 3],
        stroke: false,
        radius: 0.,
    })
}
fn scene(items: Vec<Item>) -> Scene {
    Scene {
        title: "Faded control".into(),
        bounds: Bounds::new(0., 0., 100., 100.),
        items,
        links: vec![],
        font: None,
    }
}
fn sample(scene: &Scene, x: u32, y: u32) -> [u8; 4] {
    let png = wireframe::scene::png_svg(&scene.svg()).unwrap();
    let image = image::load_from_memory(&png).unwrap().into_rgba8();
    image.get_pixel(x + 24, y + 24).0
}
#[test]
fn control_opacity_is_applied_after_overlapping_parts_and_mask() {
    let b = Bounds::new(0., 0., 100., 100.);
    let control = scene(vec![
        item(Drawing::BeginOpacity(0.4)),
        item(Drawing::BeginMask(BottomMask {
            keep_above: true,
            bounds: b,
            edge: vec![[0., 60.], [100., 60.]],
        })),
        item(rect(b, 51)),
        item(rect(Bounds::new(5., 5., 90., 90.), 221)),
        item(Drawing::EndMask),
        item(Drawing::EndOpacity),
    ]);
    // The gray fill is composed first. Fading the border and fill separately
    // would make this pixel approximately 192 instead of 241.
    assert_eq!(sample(&control, 20, 20), [241, 241, 241, 255]);
    assert_eq!(sample(&control, 2, 20), [173, 173, 173, 255]);
    assert_eq!(sample(&control, 20, 80), [255; 4]);
    let mut transparent_part = control.clone();
    transparent_part.items[3].opacity = 0.5;
    let value = sample(&transparent_part, 20, 20)[0];
    assert!((206..=208).contains(&value));
    // A later opaque control must cover the faded control normally.
    let mut overlap = control;
    overlap
        .items
        .push(item(rect(Bounds::new(10., 10., 30., 30.), 80)));
    assert_eq!(sample(&overlap, 20, 20), [80, 80, 80, 255]);
}
#[test]
fn pdf_opacity_scopes_validate_nesting_and_unwind_on_error() {
    let page = |items| wireframe::pdf::Page {
        scene: scene(items),
        decoration: None,
        size: [150., 150.],
        origin: [0., 0.],
        scale: 1.,
        links: vec![],
    };
    let mask = || {
        Drawing::BeginMask(BottomMask {
            keep_above: true,
            bounds: Bounds::new(0., 0., 100., 100.),
            edge: vec![[0., 60.], [100., 60.]],
        })
    };
    assert!(
        wireframe::pdf::encode(
            "Opacity",
            &[page(vec![
                item(Drawing::BeginOpacity(0.4)),
                item(mask()),
                item(rect(Bounds::new(0., 0., 90., 90.), 221)),
                item(Drawing::EndMask),
                item(Drawing::EndOpacity)
            ])]
        )
        .is_ok()
    );
    for drawings in [
        vec![Drawing::EndOpacity],
        vec![Drawing::BeginOpacity(0.4)],
        vec![Drawing::BeginOpacity(f32::NAN)],
        vec![
            Drawing::BeginOpacity(0.4),
            mask(),
            Drawing::EndOpacity,
            Drawing::EndMask,
        ],
        vec![
            mask(),
            Drawing::BeginOpacity(0.4),
            Drawing::EndMask,
            Drawing::EndOpacity,
        ],
    ] {
        assert!(
            wireframe::pdf::encode("Invalid", &[page(drawings.into_iter().map(item).collect())])
                .is_err()
        );
    }
}
