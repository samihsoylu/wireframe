// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use std::{io::Cursor, sync::Arc};
use wireframe::{
    document::*,
    images::{self, Prepared},
    model::Kind,
    render::Drawing,
    workspace::{Editor, ImageTarget, ImportChoice},
};
fn pixels() -> Vec<u8> {
    let colors = [
        [220, 40, 40, 255],
        [30, 180, 70, 255],
        [40, 80, 220, 255],
        [0, 0, 0, 255],
        [220, 40, 40, 128],
        [30, 180, 70, 64],
        [40, 80, 220, 192],
        [90, 150, 220, 0],
    ];
    let image = RgbaImage::from_fn(120, 80, |x, y| Rgba(colors[(x / 30 + y / 40 * 4) as usize]));
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut out, ImageFormat::Png)
        .unwrap();
    out.into_inner()
}
fn prepared(name: &str) -> Prepared {
    Prepared::from_bytes(name, &pixels()).unwrap()
}
fn fixture() -> (Editor, Id) {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    e.add(Kind::List, 50., 50.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
fn set(e: &mut Editor, id: Id, key: &str, value: Value) {
    e.begin("Edit");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
fn row_asset(e: &Editor, id: Id, row: usize) -> Id {
    let (_, p) = e.control_properties(id).unwrap();
    match &p["row_icons"] {
        Value::Icons(v) => match v[row] {
            Some(IconSource::Asset(id)) => id,
            _ => panic!(),
        },
        _ => panic!(),
    }
}
#[test]
fn tint_preserves_extent_alpha_and_linear_light_color() {
    let bytes = pixels();
    let gray = images::decoded_tinted(&bytes, [51; 3]).unwrap();
    let image = image::load_from_memory(&gray.png).unwrap().to_rgba8();
    assert_eq!(image.dimensions(), (120, 80));
    let expected = [
        [228, 128, 128, 255],
        [126, 198, 137, 255],
        [128, 140, 228, 255],
        [124, 124, 124, 255],
        [213, 96, 96, 128],
        [68, 171, 88, 64],
        [114, 128, 220, 192],
        [0, 0, 0, 0],
    ];
    for (i, expected) in expected.into_iter().enumerate() {
        let p = image
            .get_pixel((i % 4 * 30 + 15) as u32, (i / 4 * 40 + 20) as u32)
            .0;
        assert_eq!(p[3], expected[3]);
        for channel in 0..3 {
            assert!(p[channel].abs_diff(expected[channel]) <= 2, "{i}: {p:?}");
        }
    }
    let white = image::load_from_memory(&images::decoded_tinted(&bytes, [255; 3]).unwrap().png)
        .unwrap()
        .to_rgba8();
    assert_eq!(white.get_pixel(15, 20).0, [255; 4]);
    assert_eq!(white.get_pixel(105, 60).0, [0; 4]);
    let identity = image::load_from_memory(&images::decoded_tinted(&bytes, [0; 3]).unwrap().png)
        .unwrap()
        .to_rgba8();
    assert_eq!(identity.get_pixel(15, 20).0, [220, 40, 40, 255]);
    assert_eq!(bytes, pixels());
}
#[test]
fn custom_icon_import_replace_history_and_invalid_targets_are_atomic() {
    let (mut e, id) = fixture();
    let before = e.document.clone();
    let target = ImageTarget::ListRow {
        control: id,
        row: 0,
    };
    let asset = e
        .import_image_at(target, prepared("Palette.png"), ImportChoice::UseExisting)
        .unwrap();
    assert_eq!(row_asset(&e, id, 0), asset);
    assert_eq!(e.document.assets[0].bytes, pixels());
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    let assigned = e.document.clone();
    let mut replacement = prepared("Palette.png");
    replacement.bytes = images::decoded_tinted(&pixels(), [120, 20, 40])
        .unwrap()
        .png;
    e.import_image_at(target, replacement.clone(), ImportChoice::UseExisting)
        .unwrap();
    assert_eq!(e.document, assigned);
    let separate = e
        .import_image_at(target, replacement.clone(), ImportChoice::KeepBoth)
        .unwrap();
    assert_ne!(asset, separate);
    assert_eq!(e.document.assets.len(), 2);
    e.undo();
    assert_eq!(e.document, assigned);
    e.import_image_at(target, replacement, ImportChoice::Replace)
        .unwrap();
    assert_eq!(row_asset(&e, id, 0), asset);
    assert_ne!(e.document.assets[0].bytes, assigned.assets[0].bytes);
    e.undo();
    assert_eq!(e.document, assigned);
    assert!(
        e.import_image_at(
            ImageTarget::ListRow {
                control: id,
                row: 99
            },
            prepared("Bad.png"),
            ImportChoice::KeepBoth
        )
        .is_err()
    );
    let mut bad = prepared("Bad.png");
    bad.bytes = vec![0, 1, 2];
    assert!(
        e.import_image_at(target, bad, ImportChoice::KeepBoth)
            .is_err()
    );
    assert_eq!(e.document, assigned);
    e.select(id, false);
    e.lock(true);
    let locked = e.document.clone();
    assert!(
        e.import_image_at(target, prepared("Bad.png"), ImportChoice::KeepBoth)
            .is_err()
    );
    assert_eq!(locked, e.document);
    let mut invalid = assigned.clone();
    invalid.pages[0].content.nodes[0].properties.insert(
        "row_icons".into(),
        Value::Icons(vec![Some(IconSource::Asset(9999))]),
    );
    assert!(invalid.validate().is_err());
}
#[test]
fn custom_icons_copy_remap_component_overrides_and_storage_keep_assets() {
    let (mut e, id) = fixture();
    let asset = e
        .import_image_at(
            ImageTarget::ListRow {
                control: id,
                row: 0,
            },
            prepared("Source.png"),
            ImportChoice::KeepBoth,
        )
        .unwrap();
    e.create_component("List source".into()).unwrap();
    let source = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    let local = e
        .import_image_at(
            ImageTarget::ListRow {
                control: child,
                row: 1,
            },
            prepared("Local.png"),
            ImportChoice::KeepBoth,
        )
        .unwrap();
    assert_ne!(local, asset);
    assert_eq!(e.document.definitions, source);
    e.leave();
    e.select(id, false);
    let copy = e.copy();
    assert_eq!(copy.assets.len(), 2);
    let (mut other, other_id) = fixture();
    other
        .import_image_at(
            ImageTarget::ListRow {
                control: other_id,
                row: 0,
            },
            prepared("Collision.png"),
            ImportChoice::KeepBoth,
        )
        .unwrap();
    other.paste(&copy, 0.);
    assert!(other.error.is_none(), "{:?}", other.error);
    assert_eq!(other.document.assets.len(), 3);
    let pasted = *other.selection.iter().next().unwrap();
    assert!(other.enter(pasted));
    let pasted_child = other.nodes()[0].id;
    let remapped = row_asset(&other, pasted_child, 1);
    assert!(
        other
            .document
            .assets
            .iter()
            .any(|a| a.id == remapped && a.name == "Local.png")
    );
    assert_eq!(other.document.definitions.len(), 1);
    assert_eq!(
        decode(&other.document.encode().unwrap()).unwrap(),
        other.document
    );
    assert!(e.enter(id));
    e.reset_local_changes(child).unwrap();
    assert_eq!(row_asset(&e, child, 0), asset);
    let (_, props) = e.control_properties(child).unwrap();
    assert!(matches!(&props["row_icons"],Value::Icons(v) if v.len()==1));
    assert_eq!(e.document.definitions, source);
}
#[test]
fn custom_icon_scene_stretches_shares_variants_clips_and_exports() {
    let (mut e, id) = fixture();
    set(
        &mut e,
        id,
        "text",
        Value::Text("First\nSecond\nThird".into()),
    );
    set(&mut e, id, "font_size", Value::Number(24.));
    let asset = e
        .import_image_at(
            ImageTarget::ListRow {
                control: id,
                row: 0,
            },
            prepared("Palette.png"),
            ImportChoice::KeepBoth,
        )
        .unwrap();
    set(
        &mut e,
        id,
        "row_icons",
        Value::Icons(vec![Some(IconSource::Asset(asset)); 3]),
    );
    let scene = e.rendered_scene(e.page).unwrap();
    let images: Vec<_> = scene
        .items
        .iter()
        .filter_map(|i| match &i.drawing {
            Drawing::Image { bounds, data } => Some((bounds, data, i.clip)),
            _ => None,
        })
        .collect();
    assert_eq!(images.len(), 3);
    assert_eq!((images[0].0.w, images[0].0.h), (24., 24.));
    assert_eq!(images[0].1.pixels.size, [120, 80]);
    assert!(Arc::ptr_eq(images[0].1, images[1].1));
    assert!(images.iter().all(|i| i.2.is_some()));
    let svg = scene.svg();
    assert!(svg.contains("preserveAspectRatio=\"none\""));
    assert!(!svg.contains("Font Awesome"));
    assert!(scene.png().is_ok());
    set(&mut e, id, "alternate_fill", Value::Color([0, 0, 0, 255]));
    let changed = e.rendered_scene(e.page).unwrap();
    let data: Vec<_> = changed
        .items
        .iter()
        .filter_map(|i| match &i.drawing {
            Drawing::Image { data, .. } => Some(data),
            _ => None,
        })
        .collect();
    assert!(!Arc::ptr_eq(data[0], data[1]));
    assert!(Arc::ptr_eq(data[0], data[2]));
    assert_ne!(data[0].png, data[1].png);
    e.document
        .assets
        .iter_mut()
        .find(|a| a.id == asset)
        .unwrap()
        .bytes = vec![0, 1, 2];
    let missing = e.rendered_scene(e.page).unwrap();
    assert!(
        !missing
            .items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Image { .. }))
    );
    assert!(missing.png().is_ok());
}
