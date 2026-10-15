// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::sync::Arc;
use wireframe::{
    model::Bounds,
    pdf::{self, Page},
    render::{Drawing, Item, Scene, mask::BottomMask},
    scene::Primitive,
};
fn page(items: Vec<Item>) -> Page {
    Page {
        decoration: None,
        scene: Scene {
            title: "Export".into(),
            bounds: Bounds::new(0., 0., 200., 100.),
            items,
            links: vec![],
            font: None,
        },
        size: [200., 100.],
        origin: [0., 0.],
        scale: 1.,
        links: vec![],
    }
}
fn item(drawing: Drawing) -> Item {
    Item {
        drawing,
        opacity: 0.5,
        clip: Some(Bounds::new(0., 0., 100., 100.)),
    }
}
fn mask() -> Drawing {
    Drawing::BeginMask(BottomMask {
        bounds: Bounds::new(0., 0., 100., 100.),
        edge: vec![[0., 80.], [50., 70.], [100., 90.]],
    })
}
#[test]
fn failed_pdf_drawing_unwinds_clips_opacity_and_masks_without_panicking() {
    let png=wireframe::scene::png_svg("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"2\" height=\"2\"><rect width=\"2\" height=\"2\" fill=\"red\"/></svg>").unwrap();
    let mut image = wireframe::images::decoded(&png).unwrap();
    image.png = b"invalid png".to_vec();
    let broken_bitmap = Drawing::Glyph(wireframe::render::Glyph {
        text: "😀".into(),
        x: 0.,
        baseline: 24.,
        size: 24.,
        bold: false,
        italic: false,
        color: [0, 0, 0, 255],
        advance: 24.,
        face: "canvas-regular".into(),
        face_scale: 1.,
        positioned: Some(Arc::new(wireframe::text_engine::ShapedText {
            source: "😀".into(),
            width: 24.,
            runs: vec![wireframe::text_engine::Run {
                source: 0..4,
                font_name: "Bitmap fixture".into(),
                font_source: wireframe::text_engine::FontSource::InkOnly("Fixture".into()),
                font_size: 24.,
                right_to_left: false,
                glyphs: vec![wireframe::text_engine::Glyph {
                    id: 1,
                    source: 0..4,
                    position: [0., 0.],
                    advance: [24., 0.],
                    ink: Arc::new(wireframe::text_engine::Ink::Bitmap {
                        png: b"invalid png".to_vec(),
                        bounds: [0., -24., 24., 24.],
                    }),
                }],
            }],
        })),
    });
    for drawing in [
        broken_bitmap,
        Drawing::Image {
            bounds: Bounds::new(0., 0., 30., 30.),
            data: Arc::new(image),
        },
        Drawing::Primitive(Primitive::Mesh {
            points: vec![[0., 0.]],
            indices: vec![0, 1, 2],
            fill: [0; 3],
        }),
    ] {
        let input = page(vec![item(mask()), item(drawing), item(Drawing::EndMask)]);
        assert!(pdf::encode("Failure", &[input]).is_err());
    }
    for items in [vec![item(Drawing::EndMask)], vec![item(mask())]] {
        assert!(pdf::encode("Unbalanced", &[page(items)]).is_err());
    }
    let bad = wireframe::render::Glyph {
        text: "Missing".into(),
        x: 0.,
        baseline: 20.,
        size: 16.,
        bold: false,
        italic: false,
        color: [0, 0, 0, 255],
        advance: 50.,
        face: "missing-font".into(),
        face_scale: 1.,
        positioned: None,
    };
    // Missing family resolution may fall back as SVG does, but invalid text
    // geometry must fail without leaving a marked-content section open.
    let mut bad = bad;
    bad.size = f32::NAN;
    assert!(
        pdf::encode(
            "Invalid text",
            &[page(vec![
                item(mask()),
                item(Drawing::Glyph(bad)),
                item(Drawing::EndMask)
            ])]
        )
        .is_err()
    );
}
#[test]
fn pdf_rejects_invalid_page_and_link_geometry() {
    assert!(pdf::encode("Empty", &[]).is_err());
    for value in [0., -1., f32::NAN, f32::INFINITY] {
        let mut p = page(vec![]);
        p.scale = value;
        assert!(pdf::encode("Scale", &[p]).is_err());
    }
    let mut p = page(vec![]);
    p.links.push(pdf::Link {
        bounds: Bounds::new(0., 0., 10., 10.),
        destination: pdf::Destination::Page(1),
    });
    assert!(pdf::encode("Missing page", &[p]).is_err());
}

#[test]
fn every_catalog_control_encodes_in_both_styles_without_mutating_the_project() {
    use wireframe::{catalog, document::Project, workspace::Editor};
    let mut pages = Vec::new();
    for sketch in [true, false] {
        for kind in wireframe::model::Kind::ALL {
            let mut project = Project::default();
            project.settings.sketch = sketch;
            let mut editor = Editor::new(project);
            editor.add(kind, 40., 50.);
            assert!(editor.error.is_none(), "{:?}: {:?}", kind, editor.error);
            let before = serde_json::to_vec(&editor.document).unwrap();
            let mut scene = editor.rendered_scene(1).unwrap();
            scene.title = format!("{kind:?}, sketch={sketch}");
            let input = Page {
                decoration: None,
                size: [scene.bounds.w + 48., scene.bounds.h + 48.],
                scene,
                origin: [24., 24.],
                scale: 1.,
                links: vec![],
            };
            assert_eq!(before, serde_json::to_vec(&editor.document).unwrap());
            pages.push(input);
        }
    }
    assert_eq!(pages.len(), catalog::CONTROLS.len() * 2);
    let result = pdf::encode("Control catalog", &pages);
    assert!(result.is_ok(), "{:?}", result.err());
}
