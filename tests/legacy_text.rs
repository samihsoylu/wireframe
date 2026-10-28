// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    catalog,
    document::{NodeContent, Point, Project, Size, Value},
    raster,
    render::{Drawing, Scene},
    scene::Primitive,
    workspace::Editor,
};

fn pixels(scene: &Scene, resolution: raster::Resolution) -> image::RgbaImage {
    let mut png = Vec::new();
    raster::write_png(
        scene,
        raster::Options {
            transparent: true,
            resolution,
            ..Default::default()
        },
        &mut png,
        |_, _| Ok(()),
    )
    .unwrap();
    image::load_from_memory(&png).unwrap().into_rgba8()
}

#[test]
fn legacy_rectangle_bold_exports_the_existing_canvas_overdraw_and_preserves_history() {
    for sketch in [false, true] {
        let mut project = Project::default();
        project.settings.sketch = sketch;
        let mut node = catalog::get("rectangle")
            .unwrap()
            .create(2, Point::new(20., 30.));
        if let NodeContent::Control { size, .. } = &mut node.content {
            *size = Size::fixed(210., 70.);
        }
        node.properties
            .insert("text".into(), Value::Text("Café caption".into()));
        node.properties
            .insert("font_size".into(), Value::Number(24.));
        project.pages[0].content.nodes.push(node);
        project.next_id = 3;
        let mut editor = Editor::new(project.clone());
        let before = editor.document.encode().unwrap();
        let normal = editor.rendered_scene(editor.page).unwrap();
        editor.begin("Bold caption");
        editor
            .set_control_property(2, "bold", Value::Bool(true))
            .unwrap();
        editor.commit();
        project.pages[0].content.nodes[0]
            .properties
            .insert("bold".into(), Value::Bool(true));
        let after = editor.document.encode().unwrap();
        assert_eq!(after, project.encode().unwrap());
        let bold = editor.rendered_scene(editor.page).unwrap();
        assert_eq!(editor.document.encode().unwrap(), after);

        // Independent reference: the unchanged native painter draws the ordinary
        // face twice, with the second copy 0.35 logical units to the right.
        let mut expected = normal.clone();
        let (index, caption) = expected
            .items
            .iter()
            .enumerate()
            .find(|(_, item)| matches!(item.drawing, Drawing::Primitive(Primitive::Text { .. })))
            .unwrap();
        let mut shadow = caption.clone();
        if let Drawing::Primitive(Primitive::Text { x, bold, .. }) = &mut shadow.drawing {
            assert!(!*bold);
            *x += 0.35;
        }
        expected.items.insert(index + 1, shadow);
        for resolution in [raster::Resolution::Standard, raster::Resolution::Double] {
            assert!(
                pixels(&normal, resolution) != pixels(&bold, resolution),
                "Bold must change exported ink"
            );
            assert!(
                pixels(&expected, resolution) == pixels(&bold, resolution),
                "Export must match native overdraw"
            );
            let mut clipped_expected = expected.clone();
            let mut clipped_bold = bold.clone();
            for scene in [&mut clipped_expected, &mut clipped_bold] {
                for item in &mut scene.items {
                    item.clip = Some(wireframe::model::Bounds::new(100., 30., 30., 70.));
                }
            }
            let a = pixels(&clipped_expected, resolution);
            let b = pixels(&clipped_bold, resolution);
            // A common SVG clip composites the two draws before clipping;
            // separate reference items round each intermediate color. Preserve
            // the exact visible extent and allow at most two 8-bit color steps.
            let scale = resolution.scale();
            for (x, y, p) in b.enumerate_pixels() {
                if !(80 * scale..110 * scale).contains(&x) {
                    assert_eq!(p.0[3], 0, "Caption ink must not escape its clip");
                }
                for (expected, actual) in a.get_pixel(x, y).0.into_iter().zip(p.0) {
                    assert!(
                        expected.abs_diff(actual) <= 2,
                        "Clip compositing changed materially"
                    );
                }
            }
        }
        editor.undo();
        assert_eq!(editor.document.encode().unwrap(), before);
        editor.redo();
        assert_eq!(editor.document.encode().unwrap(), after);
        let reopened = Editor::new(wireframe::document::decode(&after).unwrap());
        assert_eq!(
            pixels(
                &reopened.rendered_scene(reopened.page).unwrap(),
                raster::Resolution::Standard
            ),
            pixels(&bold, raster::Resolution::Standard)
        );
    }
}

#[test]
fn both_svg_writers_keep_distinct_bold_references_and_one_caption_source() {
    use wireframe::{
        model::{Bounds, Control, Kind},
        render::Item,
    };
    for sketch in [false, true] {
        let mut left = Control::new(2, Kind::Rectangle, 20., 30.);
        left.text = "Left & label".into();
        left.bold = true;
        left.bounds.w = 160.;
        let mut right = left.clone();
        right.id = 3;
        right.bounds.x = 220.;
        right.text = "Right label".into();
        let layers = [(left, None), (right, None)];
        let scene = Scene {
            title: "Two captions".into(),
            bounds: layers
                .iter()
                .map(|(c, _)| c.bounds)
                .reduce(Bounds::union)
                .unwrap(),
            items: layers
                .iter()
                .flat_map(|(control, _)| {
                    wireframe::scene::control_scene(control, sketch)
                        .into_iter()
                        .map(|p| Item {
                            drawing: Drawing::Primitive(p),
                            opacity: 1.,
                            clip: None,
                        })
                })
                .collect(),
            links: vec![],
            font: None,
        };
        let legacy = wireframe::scene::svg_layers("Two captions", &layers, sketch);
        let canonical = scene.svg();
        for svg in [&legacy, &canonical] {
            assert_eq!(svg.matches("Left &amp; label</text>").count(), 1);
            assert_eq!(svg.matches("Right label</text>").count(), 1);
            let ids: Vec<_> = svg
                .split(" id=\"")
                .skip(1)
                .map(|s| s.split('"').next().unwrap())
                .collect();
            assert_eq!(ids.len(), 2);
            assert_eq!(
                ids.iter().collect::<std::collections::BTreeSet<_>>().len(),
                2
            );
            for id in ids {
                assert_eq!(svg.matches(&format!("href=\"#{id}\"")).count(), 1);
            }
        }
        let decoded = |svg: &str| {
            image::load_from_memory(&wireframe::scene::png_svg(svg).unwrap())
                .unwrap()
                .into_rgba8()
        };
        assert!(decoded(&legacy) == decoded(&canonical));
    }
}

#[test]
fn standalone_legacy_captions_embed_the_native_proportional_fallback_faces() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use wireframe::{
        model::{Control, Kind},
        render::Item,
    };
    let mut control = Control::new(2, Kind::Rectangle, 20., 30.);
    control.text = "Café caption".into();
    let scene = Scene {
        title: "Portable caption".into(),
        bounds: control.bounds,
        items: wireframe::scene::control_scene(&control, false)
            .into_iter()
            .map(|p| Item {
                drawing: Drawing::Primitive(p),
                opacity: 1.,
                clip: None,
            })
            .collect(),
        links: vec![],
        font: None,
    };
    let definitions = eframe::egui::FontDefinitions::default();
    let native_faces = &definitions.families[&eframe::egui::FontFamily::Proportional];
    for svg in [
        scene.svg(),
        wireframe::scene::svg_layers("Portable caption", &[(control, None)], false),
    ] {
        for name in native_faces {
            let family = format!("Wireframe {name}");
            assert!(
                svg.contains(&format!("@font-face{{font-family:'{family}';")),
                "Missing embedded native face {name}"
            );
            assert!(
                svg.contains(&STANDARD.encode(&definitions.font_data[name].font)),
                "Embedded font bytes must match the canvas fallback"
            );
        }
        let requested: Vec<_> = native_faces
            .iter()
            .map(|n| format!("Wireframe {n}"))
            .collect();
        assert!(
            svg.contains(&format!("font-family=\"{}\"", requested.join(","))),
            "Legacy captions must request the embedded aliases in native fallback order"
        );
        assert!(
            svg.contains("UBUNTU FONT LICENCE Version 1.0"),
            "Bundled fallback font notices must accompany the bytes"
        );
    }
}
