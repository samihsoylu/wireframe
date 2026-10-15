// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
#![cfg(target_os = "macos")]
use eframe::egui;
use std::sync::Arc;
use wireframe::{fonts, text_engine};

fn text_mesh(ctx: &egui::Context, output: &egui::FullOutput) -> egui::epaint::Mesh {
    ctx.tessellate(output.shapes.clone(), output.pixels_per_point)
        .into_iter()
        .find_map(|item| match item.primitive {
            egui::epaint::Primitive::Mesh(mesh)
                if mesh.texture_id != egui::TextureId::default() =>
            {
                Some(mesh)
            }
            _ => None,
        })
        .expect("Positioned text must paint a texture")
}

#[test]
fn canvas_text_maps_raster_pixels_to_physical_pixels_across_density_and_zoom() {
    let font = fonts::default_project_font();
    let text = Arc::new(text_engine::shape(font.faces()[0], "Planning notebook", 17.).unwrap());
    let ink = text.svg([51, 51, 51, 255]);
    let baseline = egui::pos2(80.25, 100.125);
    for density in [1., 1.25, 1.5, 2., 3.] {
        for zoom in [0.2, 0.75, 1., 1.2, 2., 4.] {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(density);
            let output = ctx.run_ui(Default::default(), |ui| {
                text_engine::paint(ui.painter(), &text, [51, 51, 51, 255], baseline, zoom).unwrap();
            });
            let mesh = text_mesh(&ctx, &output);
            let bounds = mesh.calc_bounds();
            let texture = output.textures_delta.set[&mesh.texture_id].last().unwrap();
            let size = texture.image.size();
            // One texel covers one display pixel, including fractional OS scales.
            assert!((bounds.width() * density - size[0] as f32).abs() < 0.002);
            assert!((bounds.height() * density - size[1] as f32).abs() < 0.002);
            for vertex in &mesh.vertices {
                for coordinate in [vertex.pos.x * density, vertex.pos.y * density] {
                    assert!((coordinate - coordinate.round()).abs() < 0.002);
                }
            }
            let raster = text
                .raster([51, 51, 51, 255], density * zoom)
                .unwrap()
                .unwrap();
            let unsnapped = baseline + raster.bounds.min.to_vec2() * zoom;
            let translation = (bounds.min - unsnapped) * density;
            assert!(translation.x.abs() <= 0.501 && translation.y.abs() <= 0.501);
            assert_eq!(text.svg([51, 51, 51, 255]), ink);
            output.drop_without_applying_deltas();
        }
    }
}

#[test]
fn panning_reuses_the_raster_without_reshaping_or_changing_its_scale() {
    let text = Arc::new(text_engine::shape(fonts::REGULAR, "ABC שלום 👩‍💻", 24.).unwrap());
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(2.);
    let mut first = None;
    for baseline in [egui::pos2(100.125, 100.25), egui::pos2(110.375, 120.125)] {
        let output = ctx.run_ui(Default::default(), |ui| {
            text_engine::paint(ui.painter(), &text, [51, 51, 51, 255], baseline, 1.).unwrap();
        });
        let mesh = text_mesh(&ctx, &output);
        if let Some((id, size)) = first {
            assert_eq!(mesh.texture_id, id);
            assert_eq!(mesh.calc_bounds().size(), size);
            assert!(!output.textures_delta.set.contains_key(&id));
        } else {
            first = Some((mesh.texture_id, mesh.calc_bounds().size()));
        }
        output.drop_without_applying_deltas();
    }
}
