// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{Node, Point, Project, Size, Value},
    raster::{self, Options, Resolution},
    render::Drawing,
    scene::Primitive,
    workspace::Editor,
};

fn frame(width: f32, height: f32) -> Editor {
    let mut doc = Project::default();
    doc.settings.sketch = true;
    let id = doc.allocate_id().unwrap();
    let mut node = Node::control(
        id,
        "browser",
        Point::new(20., 20.),
        Size::fixed(width, height),
    );
    node.properties
        .insert("text".into(), Value::Text(String::new()));
    doc.pages[0].content.nodes.push(node);
    Editor::new(doc)
}

#[test]
fn compressed_browser_bands_remain_inside_their_frame() {
    for (w, h) in [
        (1., 1.),
        (8., 8.),
        (66., 38.),
        (67., 39.),
        (254., 101.),
        (255., 102.),
        (450., 400.),
        (960., 700.),
    ] {
        let e = frame(w, h);
        let scene = e.rendered_scene(e.page).unwrap();
        assert_eq!((scene.bounds.w, scene.bounds.h), (w, h));
        let mut meshes = 0;
        for item in scene.items {
            if let Drawing::Primitive(Primitive::Mesh {
                points, indices, ..
            }) = item.drawing
            {
                meshes += 1;
                assert!(item.clip.is_some());
                assert!(points.iter().all(|p| p.iter().all(|x| x.is_finite())
                    && (19.999..=20. + w + 0.001).contains(&p[0])
                    && (19.999..=20. + h + 0.001).contains(&p[1])));
                for tri in indices.as_chunks::<3>().0 {
                    let [a, b, c] = [
                        points[tri[0] as usize],
                        points[tri[1] as usize],
                        points[tri[2] as usize],
                    ];
                    let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
                    assert!(area >= -0.0001, "{w} x {h}: {area}");
                }
            }
        }
        assert!(meshes >= 8);
    }
}

#[test]
fn transparent_browser_export_preserves_edge_coverage_and_solid_fills() {
    for (w, h) in [(450., 400.), (960., 700.)] {
        let e = frame(w, h);
        let scene = e.rendered_scene(e.page).unwrap();
        let options = Options {
            resolution: Resolution::Standard,
            transparent: true,
            ..Default::default()
        };
        let render = || {
            let mut bytes = Vec::new();
            raster::write_png(&scene, options, &mut bytes, |_, _| Ok(())).unwrap();
            bytes
        };
        let bytes = render();
        assert_eq!(
            bytes,
            render(),
            "Sketch contours must not change between exports"
        );
        let image = image::load_from_memory(&bytes).unwrap().to_rgba8();
        assert_eq!(image.dimensions(), (w as u32, h as u32));
        let alpha: Vec<_> = (10..w as u32 - 10)
            .map(|x| image.get_pixel(x, 0).0[3])
            .collect();
        assert!(alpha.iter().any(|&a| a > 0 && a < 255));
        assert!(
            alpha.iter().min() != alpha.iter().max(),
            "The edge must retain its bounded variation"
        );
        assert_eq!(image.get_pixel(w as u32 / 2, 20).0, [193, 193, 193, 255]);
        assert_eq!(image.get_pixel(w as u32 / 2, 120).0, [255; 4]);
        assert_eq!(
            image.get_pixel(w as u32 / 2, h as u32 - 15).0,
            [193, 193, 193, 255]
        );
    }
}
