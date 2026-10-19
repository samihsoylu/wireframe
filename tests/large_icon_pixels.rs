// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use wireframe::{icons, model::Bounds};

#[test]
fn large_icons_preserve_vector_coverage_across_rotation_and_clipping() {
    for id in [
        "star-regular",
        "check-solid",
        "arrow-right-solid",
        "facebook-f-brands",
    ] {
        for turns in 0..4 {
            let density = 2.;
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(density);
            let rect = Rect::from_min_size(Pos2::new(30.25, 20.125), Vec2::new(340., 300.));
            let clip = Rect::from_min_max(Pos2::new(70.25, 70.25), Pos2::new(360.25, 320.25));
            let icon = icons::get(id).unwrap();
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(450., 400.))),
                    ..Default::default()
                },
                |ui| {
                    icon.paint_rotated(
                        &ui.painter().with_clip_rect(clip),
                        rect,
                        Color32::WHITE,
                        turns,
                    )
                },
            );
            let meshes: Vec<_> = ctx
                .tessellate(output.shapes.clone(), density)
                .into_iter()
                .filter_map(|p| match p.primitive {
                    egui::epaint::Primitive::Mesh(m)
                        if m.texture_id != egui::TextureId::default() =>
                    {
                        Some(m)
                    }
                    _ => None,
                })
                .collect();
            let mut actual = vec![0u8; 900 * 800];
            let mut exact_mapping = true;
            let mut bounded = true;
            for mesh in &meshes {
                let egui::ImageData::Color(image) = &output.textures_delta.set[&mesh.texture_id]
                    .last()
                    .unwrap()
                    .image;
                bounded &= image.size.iter().all(|s| *s <= 2048);
                let b = mesh.calc_bounds();
                let uv = Rect::from_points(&mesh.vertices.iter().map(|v| v.uv).collect::<Vec<_>>());
                let source =
                    uv.min.to_vec2() * Vec2::new(image.size[0] as f32, image.size[1] as f32);
                let source_size = uv.size() * Vec2::new(image.size[0] as f32, image.size[1] as f32);
                let size = b.size() * density;
                let origin = b.min.to_vec2() * density;
                exact_mapping &= (size - source_size).abs().max_elem() < 0.002
                    && (origin - origin.round()).abs().max_elem() < 0.002;
                if !exact_mapping {
                    continue;
                }
                for y in 0..size.y.round() as usize {
                    for x in 0..size.x.round() as usize {
                        let dx = origin.x.round() as i32 + x as i32;
                        let dy = origin.y.round() as i32 + y as i32;
                        if (0..900).contains(&dx) && (0..800).contains(&dy) {
                            actual[dy as usize * 900 + dx as usize] =
                                image.pixels[(source.y.round() as usize + y) * image.size[0]
                                    + source.x.round() as usize
                                    + x]
                                    .a();
                        }
                    }
                }
            }
            let nonempty = !meshes.is_empty();
            output.drop_without_applying_deltas();
            assert!(
                nonempty && bounded && exact_mapping,
                "{id}/{turns}: visible large icons must use bounded, one-to-one textures"
            );
            let b = Bounds::new(
                rect.min.x * density,
                rect.min.y * density,
                rect.width() * density,
                rect.height() * density,
            );
            let svg = format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"900\" height=\"800\"><g transform=\"rotate({} {} {})\">{}</g></svg>",
                -i32::from(turns) * 90,
                b.x + b.w / 2.,
                b.y + b.h / 2.,
                icon.svg(b, [255; 4])
            );
            let tree = resvg::usvg::Tree::from_str(&svg, &Default::default()).unwrap();
            let mut expected = resvg::tiny_skia::Pixmap::new(900, 800).unwrap();
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::identity(),
                &mut expected.as_mut(),
            );
            let mut max_difference = 0;
            // Compare the interior of the fractional clip. GPU scissor rounding
            // at the clip boundary is independent of the vector coverage here.
            for y in (clip.min.y * density).ceil() as usize..(clip.max.y * density).floor() as usize
            {
                for x in
                    (clip.min.x * density).ceil() as usize..(clip.max.x * density).floor() as usize
                {
                    max_difference = max_difference
                        .max(actual[y * 900 + x].abs_diff(expected.data()[(y * 900 + x) * 4 + 3]));
                }
            }
            assert!(
                max_difference <= 1,
                "{id}/{turns}: max alpha difference {max_difference}"
            );
        }
    }
}

#[test]
fn large_icon_whole_pixel_pan_and_color_reuse_the_complete_raster() {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(2.);
    let mut first = None;
    for (x, color) in [
        (40.25, Color32::BLACK),
        (50.25, Color32::from_rgba_unmultiplied(80, 40, 120, 128)),
    ] {
        let output = ctx.run_ui(Default::default(), |ui| {
            icons::get("star-regular").unwrap().paint_rotated(
                ui.painter(),
                Rect::from_min_size(Pos2::new(x, 50.125), Vec2::splat(384.)),
                color,
                3,
            )
        });
        let meshes: Vec<_> = ctx
            .tessellate(output.shapes.clone(), output.pixels_per_point)
            .into_iter()
            .filter_map(|p| match p.primitive {
                egui::epaint::Primitive::Mesh(m) if m.texture_id != egui::TextureId::default() => {
                    Some(m)
                }
                _ => None,
            })
            .collect();
        assert_eq!(meshes.len(), 1);
        let mesh = &meshes[0];
        let reused =
            first.map(|id| id == mesh.texture_id && !output.textures_delta.set.contains_key(&id));
        first = Some(mesh.texture_id);
        let colors = mesh.vertices.iter().all(|v| v.color == color);
        output.drop_without_applying_deltas();
        assert!(colors && reused.unwrap_or(true));
    }
}
