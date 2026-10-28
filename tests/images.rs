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
fn cropped_rotation_preserves_center_pixels_asset_and_visible_reset_anchor() {
    let mut e = Editor::default();
    let id = add(&mut e, 50.);
    e.import_image(
        id,
        prepare("quadrants.png", 120, 80),
        ImportChoice::KeepBoth,
    )
    .unwrap();
    let asset = e.document.assets.clone();
    e.crop_image(
        id,
        Some(Crop {
            x: 0.,
            y: 0.,
            width: 0.5,
            height: 0.8,
        }),
    )
    .unwrap();
    let cropped = e.document.clone();
    e.rotate_image(id).unwrap();
    let (full, visible, crop) = e.image_geometry(id).unwrap();
    assert_eq!(visible, wireframe::model::Bounds::new(48., 52., 64., 60.));
    assert_eq!(full, wireframe::model::Bounds::new(48., -8., 80., 120.));
    assert_eq!(
        crop,
        Some(Crop {
            x: 0.,
            y: 0.5,
            width: 0.8,
            height: 0.5
        })
    );
    let scene = e.rendered_scene(e.page).unwrap();
    let data = scene
        .items
        .iter()
        .find_map(|i| match &i.drawing {
            Drawing::Image { data, .. } => Some(data),
            _ => None,
        })
        .unwrap();
    assert_eq!(data.pixels.size, [80, 120]);
    assert_eq!(
        data.pixels.pixels[10 * 80 + 10].to_array(),
        [0, 255, 0, 255]
    );
    assert_eq!(
        data.pixels.pixels[100 * 80 + 10].to_array(),
        [255, 0, 0, 255]
    );
    let raster = image::load_from_memory(&scene.png().unwrap())
        .unwrap()
        .to_rgba8();
    assert_eq!(raster.get_pixel(34, 34).0, [255, 0, 0, 255]);
    let rotated = e.document.clone();
    e.undo();
    assert_eq!(e.document, cropped);
    e.redo();
    assert_eq!(e.document, rotated);
    e.begin("Auto-size");
    e.auto_size_control(id).unwrap();
    e.commit();
    assert_eq!(size(&e, id), (80., 120.));
    assert_eq!(e.image_geometry(id).unwrap().2, crop);
    e.undo();
    e.crop_image(id, None).unwrap();
    assert_eq!(
        e.image_geometry(id).unwrap().1,
        wireframe::model::Bounds::new(48., 52., 80., 120.)
    );
    assert_eq!(e.document.assets, asset);
    e.undo();
    assert_eq!(e.document, rotated);
    let reopened = Editor::new(wireframe::document::decode(&e.document.encode().unwrap()).unwrap());
    assert_eq!(
        reopened.rendered_scene(reopened.page).unwrap().svg(),
        scene.svg()
    );
    for _ in 0..3 {
        e.rotate_image(id).unwrap();
    }
    assert_eq!(
        e.image_geometry(id).unwrap().1,
        wireframe::model::Bounds::new(50., 50., 60., 64.)
    );
}

#[test]
fn crop_remove_retains_visible_position_and_scaled_resize_keeps_normalized_crop() {
    let mut e = Editor::default();
    let id = add(&mut e, 50.);
    e.import_image(id, prepare("a.png", 120, 80), ImportChoice::KeepBoth)
        .unwrap();
    let crop = Crop {
        x: 0.25,
        y: 0.,
        width: 0.55,
        height: 0.8,
    };
    e.crop_image(id, Some(crop)).unwrap();
    assert_eq!(
        e.image_geometry(id).unwrap().1,
        wireframe::model::Bounds::new(80., 50., 66., 64.)
    );
    e.crop_image(id, None).unwrap();
    assert_eq!(
        e.image_geometry(id).unwrap().1,
        wireframe::model::Bounds::new(80., 50., 120., 80.)
    );
    e.undo();
    let mut c = e.current().controls[0].clone();
    c.bounds.w *= 2.;
    c.bounds.h *= 2.;
    e.begin("Resize crop");
    e.update_control(c);
    e.commit();
    let (full, visible, actual) = e.image_geometry(id).unwrap();
    assert_eq!(actual, Some(crop));
    assert_eq!(
        (visible.x, visible.y, visible.w, visible.h),
        (80., 50., 132., 128.)
    );
    assert_eq!((full.w, full.h), (240., 160.));
    e.begin("Auto-size");
    e.auto_size_control(id).unwrap();
    e.commit();
    assert_eq!(
        e.image_geometry(id).unwrap().1,
        wireframe::model::Bounds::new(80., 50., 66., 64.)
    );
}

#[test]
fn component_crops_can_inherit_clear_override_reset_and_detach() {
    let mut e = Editor::default();
    let id = add(&mut e, 50.);
    e.import_image(id, prepare("a.png", 120, 80), ImportChoice::KeepBoth)
        .unwrap();
    let crop = Crop {
        x: 0.25,
        y: 0.,
        width: 0.5,
        height: 0.8,
    };
    e.crop_image(id, Some(crop)).unwrap();
    e.select(id, false);
    let def = e.create_component("Picture".into()).unwrap();
    let source = e.document.definitions.clone();
    let other = e.insert_component(def, 400., 50.).unwrap();
    e.enter(id);
    let child = e.nodes()[0].id;
    e.crop_image(child, None).unwrap();
    assert_eq!(e.nodes()[0].crop, None);
    e.rotate_image(child).unwrap();
    let rotated = e.document.clone();
    assert_eq!(e.document.definitions, source);
    e.leave();
    e.enter(other);
    assert_eq!(e.nodes()[0].crop, Some(crop));
    e.leave();
    e.enter(id);
    let saved = wireframe::document::decode(&e.document.encode().unwrap()).unwrap();
    assert_eq!(saved, e.document);
    assert!(
        e.document
            .encode()
            .unwrap()
            .windows(5)
            .any(|v| v == b"clear")
    );
    e.reset_local_changes(child).unwrap();
    assert_eq!(e.nodes()[0].crop, Some(crop));
    e.undo();
    assert_eq!(e.document, rotated);
    e.crop_image(
        child,
        Some(Crop {
            x: 0.,
            y: 0.,
            width: 0.8,
            height: 0.5,
        }),
    )
    .unwrap();
    assert!(e.nodes()[0].crop.is_some());
    e.leave();
    let scene = e.rendered_scene(e.page).unwrap().svg();
    e.detach_component(id).unwrap();
    assert_eq!(e.rendered_scene(e.page).unwrap().svg(), scene);
}

#[test]
fn crop_validation_locks_and_legacy_patch_inheritance_are_preserved() {
    let mut e = Editor::default();
    let id = add(&mut e, 50.);
    e.import_image(id, prepare("a.png", 120, 80), ImportChoice::KeepBoth)
        .unwrap();
    let before = e.document.clone();
    assert!(
        e.crop_image(
            id,
            Some(Crop {
                x: 0.8,
                y: 0.,
                width: 0.5,
                height: 1.
            })
        )
        .is_err()
    );
    assert_eq!(e.document, before);
    let mut c = e.current().controls[0].clone();
    c.locked = true;
    e.update_control(c);
    let origin = e.nodes()[0].origin;
    e.rotate_image(id).unwrap();
    assert_eq!(e.nodes()[0].origin, origin);
    let before = e.document.clone();
    assert!(e.crop_image(id, None).is_err());
    assert_eq!(e.document, before);
    let patch: Patch = serde_json::from_str("{\"properties\":{}}").unwrap();
    assert_eq!(patch.crop, None);
    let region = CropOverride::Region(Crop {
        x: 0.,
        y: 0.,
        width: 0.5,
        height: 1.,
    });
    let patch = Patch {
        crop: Some(region),
        ..Patch::default()
    };
    assert_eq!(
        serde_json::from_slice::<Patch>(&serde_json::to_vec(&patch).unwrap()).unwrap(),
        patch
    );
}

#[test]
fn sketch_filter_expands_extent_preserves_source_and_processes_premultiplied_alpha() {
    let input = image::RgbaImage::from_fn(120, 80, |x, y| {
        image::Rgba(match (x < 60, y < 40) {
            (true, true) => [220, 40, 40, 255],
            (false, true) => [30, 180, 70, 255],
            (true, false) => [40, 80, 220, 255],
            _ => [0, 0, 0, 0],
        })
    });
    let mut bytes = Cursor::new(vec![]);
    input.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    let data = wireframe::images::decoded_styled(bytes.get_ref(), 0, true).unwrap();
    assert_eq!(data.pixels.size, [124, 84]);
    let p = |x: usize, y: usize| data.pixels.pixels[y * 124 + x].to_array();
    assert_eq!(p(10, 10), [228, 228, 228, 255]);
    assert_eq!(p(90, 10), [255, 255, 255, 255]);
    assert_eq!(p(10, 60), [189, 189, 189, 255]);
    assert_eq!(p(90, 60), [0, 0, 0, 0]);
    assert_eq!(p(0, 0), [0, 0, 0, 0]);
    assert!((i16::from(p(60, 20)[0]) - 28).abs() <= 1); // darker side of a contrast edge
    assert_eq!(p(61, 20)[0], 0);
    let input = image::RgbaImage::from_pixel(24, 24, image::Rgba([200, 100, 30, 100]));
    let mut bytes = Cursor::new(vec![]);
    input.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    let result = wireframe::images::decoded_styled(bytes.get_ref(), 0, true).unwrap();
    let pixel = image::load_from_memory(&result.png)
        .unwrap()
        .to_rgba8()
        .get_pixel(10, 10)
        .0;
    assert!((i16::from(pixel[0]) - 128).abs() <= 1);
    assert!((i16::from(pixel[3]) - 200).abs() <= 1);
}

#[test]
fn empty_image_placeholder_uses_its_caption_formatting_and_style_frame() {
    use wireframe::{model::Bounds, scene::Primitive};
    for sketch in [false, true] {
        let mut e = Editor::default();
        e.document.settings.sketch = sketch;
        let id = add(&mut e, 50.);
        if let NodeContent::Control { size, .. } = &mut e.document.pages[0].content.nodes[0].content
        {
            *size = Size::fixed(240., 140.);
        }
        let mut e = Editor::new(e.document.clone());
        let before = e.document.clone();
        let blank = e.rendered_scene(e.page).unwrap();
        assert!(
            blank.items.iter().all(|item| !matches!(
                &item.drawing,
                Drawing::Glyph(_) | Drawing::Primitive(Primitive::Text { .. })
            )),
            "an empty Image has no synthetic caption"
        );
        assert!(blank.items.iter().any(|item| matches!(
            item.drawing,
            Drawing::Primitive(Primitive::Rect { b, fill: [255, 255, 255], stroke: false, .. })
                if b == Bounds::new(50., 50., 240., 140.)
        )));
        let (width, color) = if sketch {
            (3., [92, 92, 94])
        } else {
            (1., [51; 3])
        };
        for points in [
            vec![[55., 55.], [285., 185.]],
            vec![[285., 55.], [55., 185.]],
        ] {
            assert!(blank.items.iter().any(|item| matches!(
                &item.drawing,
                Drawing::Primitive(Primitive::Line { points: actual, width: w, color: c })
                    if *actual == points && *w == width && *c == color
            )));
        }
        e.begin("Format empty Image");
        for (key, value) in [
            ("text", Value::Text("Café caption".into())),
            ("font_size", Value::Number(24.)),
            ("bold", Value::Bool(true)),
            ("italic", Value::Bool(true)),
            ("underline", Value::Bool(true)),
            ("image_border", Value::Bool(true)),
        ] {
            e.set_control_property(id, key, value).unwrap();
        }
        e.commit();
        let formatted = e.document.clone();
        let scene = e.rendered_scene(e.page).unwrap();
        let glyphs: Vec<_> = scene
            .items
            .iter()
            .filter_map(|item| match &item.drawing {
                Drawing::Glyph(g) => Some((g, item.clip)),
                _ => None,
            })
            .collect();
        assert_eq!(
            glyphs
                .iter()
                .map(|(g, _)| g.text.as_str())
                .collect::<String>(),
            "Café caption"
        );
        assert!(glyphs.iter().all(|(g, clip)| g.size == 24.
            && g.bold
            && g.italic
            && *clip == Some(Bounds::new(50., 50., 240., 140.))));
        assert_eq!(
            e.document, formatted,
            "rendering must not mutate the project"
        );
        let svg = scene.svg();
        assert!(scene.png().is_ok());
        e.undo();
        assert_eq!(e.document, before);
        e.redo();
        assert_eq!(e.document, formatted);
        let reopened =
            Editor::new(wireframe::document::decode(&formatted.encode().unwrap()).unwrap());
        assert_eq!(reopened.rendered_scene(reopened.page).unwrap().svg(), svg);
    }
}

#[test]
fn unavailable_image_retains_caption_layout_and_its_failure_status() {
    for sketch in [false, true] {
        let mut e = Editor::default();
        e.document.settings.sketch = sketch;
        e.document.settings.font_size = 20.;
        let id = add(&mut e, 50.);
        if let NodeContent::Control { size, .. } = &mut e.document.pages[0].content.nodes[0].content
        {
            *size = Size::fixed(240., 140.);
        }
        let mut e = Editor::new(e.document.clone());
        e.import_image(id, prepare("caption.png", 24, 14), ImportChoice::KeepBoth)
            .unwrap();
        e.set_control_property(id, "text", Value::Text("*Café* caption".into()))
            .unwrap();
        e.set_control_property(id, "font_size", Value::Number(24.))
            .unwrap();
        let caption = |scene: &wireframe::render::Scene| {
            scene
                .items
                .iter()
                .filter_map(|item| match &item.drawing {
                    Drawing::Glyph(g) if g.size == 24. => Some((
                        g.text.clone(),
                        g.x,
                        g.baseline,
                        g.size,
                        g.bold,
                        g.italic,
                        g.face.clone(),
                        item.clip,
                    )),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let loaded_caption = caption(&e.rendered_scene(e.page).unwrap());
        assert!(!loaded_caption.is_empty());
        for source in ["corrupt", "trashed", "removed"] {
            let mut project = e.document.clone();
            match source {
                "corrupt" => project.assets[0].bytes = b"not a PNG".to_vec(),
                "trashed" => project.assets[0].trashed = true,
                "removed" => {
                    let asset = project.assets.remove(0);
                    project.removed_assets.insert(asset.id);
                }
                _ => unreachable!(),
            }
            let editor = Editor::new(project.clone());
            let scene = editor.rendered_scene(editor.page).unwrap();
            assert_eq!(
                caption(&scene),
                loaded_caption,
                "caption layout changed for {source}"
            );
            let status_glyphs: Vec<_> = scene
                .items
                .iter()
                .filter_map(|item| match &item.drawing {
                    Drawing::Glyph(g) if g.size != 24. => Some(g),
                    _ => None,
                })
                .collect();
            let status = status_glyphs
                .iter()
                .map(|g| g.text.as_str())
                .collect::<String>();
            assert_eq!(status, "Image unavailable", "missing status for {source}");
            assert!(
                status_glyphs
                    .iter()
                    .all(|g| g.size == 20. && !g.bold && !g.italic && g.color == [0, 0, 0, 255]),
                "status must use the project default style for {source}"
            );
            assert!(
                scene
                    .items
                    .iter()
                    .all(|item| !matches!(item.drawing, Drawing::Image { .. }))
            );
            assert_eq!(editor.document, project);
        }
    }
}

#[test]
fn image_style_frame_crop_text_and_local_reset_share_canvas_export_data() {
    let spec = wireframe::catalog::get("image").unwrap();
    for key in ["image_border", "sketch_filter", "text"] {
        assert!(spec.property(key).unwrap().visible);
    }
    let mut e = Editor::default();
    let id = add(&mut e, 50.);
    e.import_image(id, prepare("picture.png", 120, 80), ImportChoice::KeepBoth)
        .unwrap();
    e.crop_image(
        id,
        Some(Crop {
            x: 0.25,
            y: 0.,
            width: 0.5,
            height: 0.8,
        }),
    )
    .unwrap();
    let assets = e.document.assets.clone();
    let before = e.document.clone();
    e.begin("Style image");
    e.set_control_property(id, "image_border", Value::Bool(true))
        .unwrap();
    e.set_control_property(id, "sketch_filter", Value::Bool(true))
        .unwrap();
    e.set_control_property(id, "text", Value::Text("*Photo*".into()))
        .unwrap();
    e.commit();
    let scene = e.rendered_scene(e.page).unwrap();
    let (b, data) = scene
        .items
        .iter()
        .find_map(|i| match &i.drawing {
            Drawing::Image { bounds, data } => Some((bounds, data)),
            _ => None,
        })
        .unwrap();
    assert_eq!(data.pixels.size, [124, 84]);
    // Visible crop is (80,50,60,64), inset to (85,55,50,54).
    assert_eq!((b.x, b.y, b.w, b.h), (60., 55., 100., 67.5));
    let clip = scene
        .items
        .iter()
        .find(|i| matches!(i.drawing, Drawing::Image { .. }))
        .unwrap()
        .clip
        .unwrap();
    assert_eq!((clip.x, clip.y, clip.w, clip.h), (85., 55., 50., 54.));
    let label: String = scene
        .items
        .iter()
        .filter_map(|i| match &i.drawing {
            Drawing::Glyph(g) => Some(g.text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(label, "Photo");
    assert!(
        scene
            .items
            .iter()
            .filter_map(|i| match &i.drawing {
                Drawing::Glyph(g) => Some(g),
                _ => None,
            })
            .all(|g| g.bold)
    );
    assert!(scene.svg().contains("data:image/png;base64,"));
    assert!(scene.png().is_ok());
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document.assets, assets);
    let svg = e.rendered_scene(e.page).unwrap().svg();
    let reopened = Editor::new(wireframe::document::decode(&e.document.encode().unwrap()).unwrap());
    assert_eq!(reopened.rendered_scene(reopened.page).unwrap().svg(), svg);
    // An ancestor clip hides part of the frame without creating a new border.
    let mut clipped = e.document.clone();
    let children = std::mem::take(&mut clipped.pages[0].content.nodes);
    let group_id = clipped.allocate_id().unwrap();
    let mut group = Node::group(group_id, Point::default(), children);
    group.crop = Some(Crop {
        x: 0.5,
        y: 0.,
        width: 0.5,
        height: 1.,
    });
    clipped.pages[0].content.nodes.push(group);
    let clipped = Editor::new(clipped);
    let layer = &clipped.layers(clipped.page).unwrap()[0];
    assert_eq!((layer.frame.x, layer.frame.w), (80., 60.));
    let clipped_scene = clipped.rendered_scene(clipped.page).unwrap();
    let item = clipped_scene
        .items
        .iter()
        .find(|i| matches!(i.drawing, Drawing::Image { .. }))
        .unwrap();
    let Drawing::Image { bounds, .. } = item.drawing else {
        unreachable!()
    };
    assert_eq!((bounds.x, bounds.w), (60., 100.));
    let clip = item.clip.unwrap();
    assert_eq!((clip.x, clip.w), (110., 25.));
    e.select(id, false);
    let def = e.create_component("Photo".into()).unwrap();
    let other = e.insert_component(def, 300., 50.).unwrap();
    let source = e.document.definitions.clone();
    e.enter(id);
    let child = e.nodes()[0].id;
    e.begin("Local style");
    e.set_control_property(child, "sketch_filter", Value::Bool(false))
        .unwrap();
    e.set_control_property(child, "image_border", Value::Bool(false))
        .unwrap();
    e.commit();
    assert_eq!(e.document.definitions, source);
    e.leave();
    e.enter(other);
    assert_eq!(
        e.control_properties(e.nodes()[0].id).unwrap().1["sketch_filter"],
        Value::Bool(true)
    );
    e.leave();
    e.enter(id);
    e.reset_local_changes(child).unwrap();
    assert_eq!(
        e.control_properties(child).unwrap().1["sketch_filter"],
        Value::Bool(true)
    );
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
    let origin = e.nodes().iter().find(|n| n.id == id).unwrap().origin;
    e.import_image(id, prepare("test.png", 3, 2), ImportChoice::KeepBoth)
        .unwrap();
    assert!(e.own_locked(id));
    assert_eq!(
        e.nodes().iter().find(|n| n.id == id).unwrap().origin,
        origin
    );
    let edited = e.document.clone();
    e.move_by(10., 10.);
    e.delete();
    assert_eq!(e.document, edited);
}
