// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{fonts, text_engine};

#[test]
fn text_engine_rejects_unbounded_or_invalid_inputs() {
    for size in [0., -1., f64::NAN, f64::INFINITY, 513.] {
        assert!(text_engine::shape(fonts::REGULAR, "text", size).is_err());
    }
    assert!(text_engine::shape(&[], "text", 24.).is_err());
    assert!(
        text_engine::shape(
            fonts::REGULAR,
            &"a".repeat(text_engine::MAX_TEXT_BYTES + 1),
            24.
        )
        .is_err()
    );
    assert!(text_engine::shape(b"invalid font", "text", 24.).is_err());
}

#[test]
fn positioned_text_keeps_utf8_clusters_bidi_and_color_emoji_ink() {
    use text_engine::Ink;
    fn portable<T: Send + Sync>() {}
    portable::<text_engine::ShapedText>();
    for source in [
        "",
        "\u{2067}\u{2069}",
        "office",
        "a\u{301}\u{323}",
        "ABC שלום 123",
        "مرحبا ABC 123",
        "你好 😀 👍 👩‍💻",
    ] {
        let shaped = text_engine::shape(fonts::REGULAR, source, 24.).unwrap();
        assert_eq!(shaped.source, source);
        assert!(shaped.width.is_finite());
        for run in &shaped.runs {
            assert!(source.get(run.source.clone()).is_some());
            for glyph in &run.glyphs {
                assert!(source.get(glyph.source.clone()).is_some());
                assert!(!glyph.source.is_empty());
                assert!(
                    glyph
                        .position
                        .iter()
                        .chain(&glyph.advance)
                        .all(|v| v.is_finite())
                );
            }
        }
        if source.contains("שלום") || source.contains("مرحبا") {
            assert!(shaped.runs.iter().any(|r| r.right_to_left));
        }
        if source.contains('😀') {
            assert!(
                shaped
                    .runs
                    .iter()
                    .flat_map(|r| &r.glyphs)
                    .any(|g| matches!(&*g.ink, Ink::Bitmap { .. }))
            );
        }
    }
    let source = "👩‍💻";
    let joined = text_engine::shape(fonts::REGULAR, source, 24.).unwrap();
    let ink: Vec<_> = joined
        .runs
        .iter()
        .flat_map(|r| &r.glyphs)
        .filter(|g| !matches!(&*g.ink, Ink::Empty))
        .collect();
    assert_eq!(ink.len(), 1, "Joined emoji must have one visible glyph");
    assert_eq!(ink[0].source, 0..source.len());
    let text_engine::Ink::Bitmap { png, bounds } = &*ink[0].ink else {
        panic!("Expected color emoji")
    };
    let image = image::load_from_memory(png).unwrap().to_rgba8();
    assert_eq!(image.width(), (bounds[2] * 2.) as u32);
    assert_eq!(image.height(), (bounds[3] * 2.) as u32);
    assert!(
        image
            .pixels()
            .any(|p| p[3] > 0 && (p[0] != p[1] || p[1] != p[2]))
    );
}

#[test]
fn positioned_ink_exports_without_fonts_or_a_second_text_shaper() {
    let shaped = text_engine::shape(fonts::REGULAR, "ABC שלום 123 👩‍💻", 24.).unwrap();
    let content = shaped.svg([51, 51, 51, 255]);
    assert!(content.contains("<path"));
    assert!(content.contains("<image"));
    assert!(!content.contains("<text"));
    assert!(!content.contains("font-family"));
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"400\" height=\"70\"><g transform=\"translate(20 40)\">{content}</g></svg>"
    );
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).unwrap();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(400, 70).unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    assert!(pixmap.data().as_chunks::<4>().0.iter().any(|p| p[3] > 0));
}

#[test]
fn shaped_runs_retain_primary_and_actual_fallback_faces() {
    use text_engine::FontSource;
    let source = "Office e\u{301} שלום مرحبا 你好 👩‍💻";
    let shaped = text_engine::shape(fonts::REGULAR, source, 24.).unwrap();
    let mut primary = 0;
    let mut fallback = 0;
    let mut bitmap = 0;
    for run in &shaped.runs {
        match &run.font_source {
            FontSource::Embedded(face) => {
                let parsed = skrifa::FontRef::from_index(face.data().unwrap(), face.index).unwrap();
                use skrifa::MetadataProvider;
                let count = parsed
                    .metrics(
                        skrifa::instance::Size::unscaled(),
                        skrifa::instance::LocationRef::default(),
                    )
                    .glyph_count;
                assert!(run.glyphs.iter().all(|g| g.id < count));
                if run.font_name == "ComicNeue-Regular" {
                    primary += 1;
                    assert_eq!(face.data().unwrap(), fonts::REGULAR);
                } else {
                    fallback += 1;
                    assert_ne!(face.data().unwrap(), fonts::REGULAR);
                }
                let mut db = resvg::usvg::fontdb::Database::new();
                db.load_font_data(face.data().unwrap().to_vec());
                assert_eq!(
                    db.faces()
                        .nth(face.index as usize)
                        .unwrap()
                        .post_script_name,
                    run.font_name
                );
            }
            FontSource::InkOnly(reason) => {
                assert!(
                    run.glyphs
                        .iter()
                        .any(|g| matches!(&*g.ink, text_engine::Ink::Bitmap { .. })),
                    "Unexpected unavailable vector face {}: {reason}",
                    run.font_name
                );
                bitmap += 1;
            }
        }
    }
    assert!(primary > 0 && fallback >= 3 && bitmap > 0);
    // Several labels must share a live fallback face, not each retain a full
    // multi-megabyte copy of the same system font.
    let second = text_engine::shape(fonts::REGULAR, "世界 שלום مرحبا", 24.).unwrap();
    for run in &second.runs {
        if run.font_name == "ComicNeue-Regular" {
            continue;
        }
        let FontSource::Embedded(face) = &run.font_source else {
            panic!("Expected vector face")
        };
        let first = shaped
            .runs
            .iter()
            .find(|r| r.font_name == run.font_name)
            .unwrap();
        let FontSource::Embedded(original) = &first.font_source else {
            panic!()
        };
        assert!(std::sync::Arc::ptr_eq(face, original));
    }
}

#[test]
fn positioned_text_cache_survives_reuse_but_invalidates_replaced_fonts() {
    use eframe::egui;
    use std::sync::Arc;
    let ctx = egui::Context::default();
    fonts::install(&ctx);
    let font_id = egui::FontId::new(24., fonts::family(false, false));
    let mut original = None;
    ctx.run_ui(Default::default(), |_| {
        let first = text_engine::shape_cached(&ctx, &font_id, "Wide text")
            .unwrap()
            .unwrap();
        let second = text_engine::shape_cached(&ctx, &font_id, "Wide text")
            .unwrap()
            .unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        original = Some(first);
        assert!(
            text_engine::shape_cached(&ctx, &egui::FontId::proportional(24.), "a")
                .unwrap()
                .is_none()
        );
    })
    .drop_without_applying_deltas();
    let definitions = egui::FontDefinitions::default();
    let bytes = definitions.font_data["Ubuntu-Light"].font.to_vec();
    let font = fonts::ProjectFont::new(
        "Test family".into(),
        std::array::from_fn(|_| bytes.clone()),
        String::new(),
    )
    .unwrap();
    fonts::install_project(&ctx, Some(&font));
    ctx.run_ui(Default::default(), |_| {
        let replacement = text_engine::shape_cached(&ctx, &font_id, "Wide text")
            .unwrap()
            .unwrap();
        let first = original.as_ref().unwrap();
        assert!(!Arc::ptr_eq(first, &replacement));
        assert_ne!(first.width, replacement.width);
        assert_ne!(first.svg([0, 0, 0, 255]), replacement.svg([0, 0, 0, 255]));
    })
    .drop_without_applying_deltas();
}

#[test]
fn canvas_raster_matches_positioned_svg_at_multiple_scales() {
    let text = text_engine::shape(fonts::REGULAR, "ABC שלום 123 👩‍💻", 24.).unwrap();
    for scale in [0.5, 1., 2., 3.] {
        let raster = text.raster([51, 51, 51, 255], scale).unwrap().unwrap();
        assert!(raster.bounds.min.y < 0.);
        assert!(raster.pixels.pixels.iter().any(|p| p.a() > 0));
        let svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\"><g transform=\"matrix({scale} 0 0 {scale} {} {})\">{}</g></svg>",
            raster.pixels.size[0],
            raster.pixels.size[1],
            -raster.bounds.min.x * scale,
            -raster.bounds.min.y * scale,
            text.svg([51, 51, 51, 255])
        );
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).unwrap();
        let mut expected = resvg::tiny_skia::Pixmap::new(
            raster.pixels.size[0] as u32,
            raster.pixels.size[1] as u32,
        )
        .unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut expected.as_mut(),
        );
        for (a, b) in raster.pixels.pixels.iter().zip(expected.pixels()) {
            assert_eq!(a.to_array(), [b.red(), b.green(), b.blue(), b.alpha()]);
        }
    }
    for scale in [0., -1., f32::NAN, f32::INFINITY, 33.] {
        assert!(text.raster([0, 0, 0, 255], scale).is_err());
    }
    assert!(
        text_engine::shape(fonts::REGULAR, "   ", 24.)
            .unwrap()
            .raster([0, 0, 0, 255], 1.)
            .unwrap()
            .is_none()
    );
}
