// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::{io::Cursor, sync::Arc};
use wireframe::{
    document::*,
    images::Prepared,
    model::Kind,
    render::Drawing,
    workspace::{Editor, ImportChoice},
};

fn png(w: u32, h: u32, alpha: bool) -> Vec<u8> {
    let img = image::RgbaImage::from_fn(w, h, |x, y| {
        image::Rgba(if x < w / 2 {
            [255, 0, 0, 255]
        } else if y < h / 2 {
            [0, 255, 0, 255]
        } else {
            [0, 0, 255, if alpha { 0 } else { 255 }]
        })
    });
    let mut bytes = Cursor::new(Vec::new());
    img.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
}
fn add(e: &mut Editor, x: f32) -> Id {
    e.add(Kind::Image, x, 50.);
    *e.selection.iter().next().unwrap()
}
fn prepare(name: &str, w: u32, h: u32) -> Prepared {
    Prepared::from_bytes(name, &png(w, h, true)).unwrap()
}
fn size(e: &Editor, id: Id) -> (f32, f32) {
    let c = e
        .current()
        .controls
        .into_iter()
        .find(|c| c.id == id)
        .unwrap();
    (c.bounds.w, c.bounds.h)
}

#[test]
fn embedded_alpha_renders_in_shared_scene_and_exports_and_survives_source_removal() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("alpha.png");
    let bytes = png(120, 80, true);
    std::fs::write(&file, &bytes).unwrap();
    let mut e = Editor::default();
    let id = add(&mut e, 50.);
    assert_eq!(size(&e, id), (77., 79.));
    let asset = e
        .import_image(id, Prepared::read(&file).unwrap(), ImportChoice::KeepBoth)
        .unwrap();
    assert_eq!(size(&e, id), (120., 80.));
    assert_eq!(e.document.assets[0].bytes, bytes);
    e.select(id, false);
    e.duplicate();
    let scene = e.rendered_scene(e.page).unwrap();
    let images: Vec<_> = scene
        .items
        .iter()
        .filter_map(|i| {
            if let Drawing::Image { data, .. } = &i.drawing {
                Some(data)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(images.len(), 2);
    assert!(Arc::ptr_eq(images[0], images[1]));
    assert_eq!(images[0].pixels.pixels[70 * 120 + 100].a(), 0);
    assert!(scene.svg().contains("data:image/png;base64,"));
    assert!(!scene.svg().contains(file.to_str().unwrap()));
    let raster = image::load_from_memory(&scene.png().unwrap()).unwrap();
    assert!(raster.width() > 120);
    let pixels = raster.to_rgba8();
    assert_eq!(pixels.get_pixel(34, 34).0, [255, 0, 0, 255]);
    assert_eq!(pixels.get_pixel(124, 34).0, [0, 255, 0, 255]);
    assert_eq!(pixels.get_pixel(154, 114).0, [255, 255, 255, 255]);
    let saved = e.document.encode().unwrap();
    std::fs::remove_file(file).unwrap();
    let reopened = Editor::new(wireframe::document::decode(&saved).unwrap());
    assert_eq!(reopened.document.assets[0].id, asset);
    assert_eq!(
        reopened.rendered_scene(reopened.page).unwrap().svg(),
        scene.svg()
    );
}

#[test]
fn source_assignment_clear_shared_replace_and_history_preserve_identity() {
    let mut e = Editor::default();
    let first = add(&mut e, 50.);
    let second = add(&mut e, 250.);
    let original = e
        .import_image(first, prepare("photo.png", 120, 80), ImportChoice::KeepBoth)
        .unwrap();
    e.assign_image(second, Some(original)).unwrap();
    let before = e.document.clone();
    let same = e
        .import_image(second, prepare("photo.png", 80, 120), ImportChoice::Replace)
        .unwrap();
    assert_eq!(same, original);
    assert_eq!(e.document.assets.len(), 1);
    assert_eq!(size(&e, first), (80., 120.));
    assert_eq!(size(&e, second), (80., 120.));
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    let replaced = e.document.clone();
    e.assign_image(second, None).unwrap();
    assert_eq!(size(&e, second), (77., 79.));
    assert_eq!(e.document.assets, replaced.assets);
    e.undo();
    assert_eq!(e.document, replaced);
    let other = e
        .import_image(
            second,
            prepare("photo.png", 120, 80),
            ImportChoice::KeepBoth,
        )
        .unwrap();
    assert_ne!(other, original);
    assert_eq!(size(&e, first), (80., 120.));
    assert_eq!(size(&e, second), (120., 80.));
    let count = e.document.assets.len();
    e.import_image(
        second,
        prepare("photo.png", 40, 30),
        ImportChoice::UseExisting,
    )
    .unwrap();
    assert_eq!(e.document.assets.len(), count);
    assert_eq!(size(&e, second), (80., 120.));
}

#[test]
fn fixed_size_and_component_local_sources_remain_independent() {
    let mut e = Editor::default();
    let first = add(&mut e, 50.);
    let a = e
        .import_image(first, prepare("a.png", 120, 80), ImportChoice::KeepBoth)
        .unwrap();
    let mut c = e.current().controls[0].clone();
    c.bounds.w = 240.;
    c.bounds.h = 160.;
    e.begin("Resize image");
    e.update_control(c);
    e.commit();
    e.import_image(first, prepare("a.png", 80, 120), ImportChoice::Replace)
        .unwrap();
    assert_eq!(size(&e, first), (240., 160.));
    e.select(first, false);
    let def = e.create_component("Picture".into()).unwrap();
    let second = e.insert_component(def, 400., 50.).unwrap();
    e.enter(first);
    let child = e.nodes()[0].id;
    let b = e
        .import_image(child, prepare("b.png", 90, 60), ImportChoice::KeepBoth)
        .unwrap();
    assert_ne!(a, b);
    assert_eq!(
        e.control_properties(child).unwrap().1.get("image"),
        Some(&Value::Asset(b))
    );
    e.leave();
    e.enter(second);
    let child = e.nodes()[0].id;
    assert_eq!(
        e.control_properties(child).unwrap().1.get("image"),
        Some(&Value::Asset(a))
    );
    e.leave();
    e.enter(first);
    let child = e.nodes()[0].id;
    e.assign_image(child, None).unwrap();
    assert_eq!(
        e.control_properties(child).unwrap().1.get("image"),
        Some(&Value::Null)
    );
    e.undo();
    assert_eq!(
        e.control_properties(child).unwrap().1.get("image"),
        Some(&Value::Asset(b))
    );
}

#[test]
fn formats_are_identified_by_bytes_and_preprocessing_respects_alpha_and_size() {
    let alpha = png(120, 80, true);
    let prepared = Prepared::from_bytes("/private/source/strange.jpg", &alpha).unwrap();
    assert_eq!(prepared.name, "strange.jpg");
    assert_eq!(prepared.bytes, alpha);
    assert_eq!(prepared.media_type, "image/png");
    let opaque = Prepared::from_bytes("opaque.png", &png(80, 120, false)).unwrap();
    assert_eq!(
        image::guess_format(&opaque.bytes).unwrap(),
        image::ImageFormat::Jpeg
    );
    assert_eq!(opaque.media_type, "image/jpeg");
    let jpeg = Prepared::from_bytes("jpeg.data", &opaque.bytes).unwrap();
    assert_eq!(jpeg.bytes, opaque.bytes);
    let mut gif = Vec::new();
    {
        let mut enc = image::codecs::gif::GifEncoder::new(&mut gif);
        enc.encode_frame(image::Frame::new(image::RgbaImage::from_pixel(
            4,
            3,
            image::Rgba([2, 3, 4, 0]),
        )))
        .unwrap();
    }
    let gif = Prepared::from_bytes("animated.gif", &gif).unwrap();
    assert_eq!(gif.media_type, "image/png");
    assert_eq!(wireframe::images::dimensions(&gif.bytes).unwrap(), (4, 3));
    let large = Prepared::from_bytes("large.png", &png(2000, 1500, true)).unwrap();
    let (w, h) = wireframe::images::dimensions(&large.bytes).unwrap();
    assert_eq!((w, h), (1672, 1254));
}

#[test]
fn invalid_imports_and_locked_controls_do_not_mutate_document_or_history() {
    assert!(Prepared::from_bytes("bad.png", b"not an image").is_err());
    let mut e = Editor::default();
    let id = add(&mut e, 0.);
    let before = e.document.clone();
    let undo = e.undo_name().map(str::to_owned);
    let bad = Prepared {
        name: "broken.png".into(),
        media_type: "image/png".into(),
        bytes: vec![1, 2, 3],
    };
    assert!(e.import_image(id, bad, ImportChoice::KeepBoth).is_err());
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), undo.as_deref());
    let invalid_name = prepare(&"x".repeat(10001), 3, 2);
    assert!(
        e.import_image(id, invalid_name, ImportChoice::KeepBoth)
            .is_err()
    );
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), undo.as_deref());
    assert!(e.assign_image(id, Some(999)).is_err());
    assert_eq!(e.document, before);
    e.select(id, false);
    e.lock(true);
    let locked = e.document.clone();
    assert!(
        e.import_image(id, prepare("test.png", 3, 2), ImportChoice::KeepBoth)
            .is_err()
    );
    assert_eq!(e.document, locked);
}
