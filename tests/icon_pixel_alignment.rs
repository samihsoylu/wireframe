// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use wireframe::icons;

#[test]
fn integer_pixel_pan_and_color_changes_reuse_the_icon_texture() {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(2.);
    let mut first = None;
    for (x, color) in [
        (60.25, Color32::BLACK),
        (70.25, Color32::from_rgba_unmultiplied(40, 90, 180, 128)),
    ] {
        let output = ctx.run_ui(Default::default(), |ui| {
            icons::get("star-regular").unwrap().paint(
                ui.painter(),
                Rect::from_min_size(Pos2::new(x, 80.125), Vec2::splat(24.)),
                color,
            );
        });
        let mesh = ctx
            .tessellate(output.shapes.clone(), output.pixels_per_point)
            .into_iter()
            .find_map(|item| match item.primitive {
                egui::epaint::Primitive::Mesh(m) if m.texture_id != egui::TextureId::default() => {
                    Some(m)
                }
                _ => None,
            })
            .unwrap();
        let reused =
            first.map(|id| id == mesh.texture_id && !output.textures_delta.set.contains_key(&id));
        first = Some(mesh.texture_id);
        let colors_preserved = mesh.vertices.iter().all(|v| v.color == color);
        output.drop_without_applying_deltas();
        assert!(colors_preserved);
        assert!(reused.unwrap_or(true));
    }
}

#[test]
fn oversized_icon_keeps_a_bounded_texture_and_invalid_geometry_does_not_paint() {
    let ctx = egui::Context::default();
    let output = ctx.run_ui(Default::default(), |ui| {
        let icon = icons::get("star-regular").unwrap();
        icon.paint(
            ui.painter(),
            Rect::from_min_size(Pos2::new(10., 10.), Vec2::splat(8000.)),
            Color32::BLACK,
        );
        icon.paint(
            ui.painter(),
            Rect::from_min_size(Pos2::new(f32::NAN, 10.), Vec2::splat(24.)),
            Color32::BLACK,
        );
        icon.paint(
            ui.painter(),
            Rect::from_min_size(Pos2::new(10., 10.), Vec2::splat(f32::INFINITY)),
            Color32::BLACK,
        );
    });
    let meshes: Vec<_> = ctx
        .tessellate(output.shapes.clone(), output.pixels_per_point)
        .into_iter()
        .filter_map(|item| match item.primitive {
            egui::epaint::Primitive::Mesh(m) if m.texture_id != egui::TextureId::default() => {
                Some(m)
            }
            _ => None,
        })
        .collect();
    let valid = meshes.len() == 1 && meshes[0].vertices.iter().all(|v| v.pos.is_finite());
    let size = output.textures_delta.set[&meshes[0].texture_id]
        .last()
        .unwrap()
        .image
        .size();
    output.drop_without_applying_deltas();
    assert!(valid);
    assert!(size[0] <= 512 && size[1] <= 512);
}

#[test]
fn icon_texture_preserves_svg_coverage_and_quarter_turns() {
    for id in [
        "0-solid",
        "star-regular",
        "circle-regular",
        "facebook-f-brands",
    ] {
        for turns in 0..4 {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(2.);
            let rect = Rect::from_min_size(Pos2::new(30.25, 40.125), Vec2::new(46., 32.));
            let icon = icons::get(id).unwrap();
            let output = ctx.run_ui(Default::default(), |ui| {
                icon.paint_rotated(ui.painter(), rect, Color32::WHITE, turns)
            });
            let mesh = ctx
                .tessellate(output.shapes.clone(), output.pixels_per_point)
                .into_iter()
                .find_map(|item| match item.primitive {
                    egui::epaint::Primitive::Mesh(m)
                        if m.texture_id != egui::TextureId::default() =>
                    {
                        Some(m)
                    }
                    _ => None,
                })
                .unwrap();
            let image = match &output.textures_delta.set[&mesh.texture_id]
                .last()
                .unwrap()
                .image
            {
                egui::ImageData::Color(image) => image.clone(),
            };
            let origin = (mesh.calc_bounds().min.to_vec2() * 2.).round();
            let mut actual = vec![0u8; 256 * 256];
            for y in 0..image.size[1] {
                for x in 0..image.size[0] {
                    actual[(origin.y as usize + y) * 256 + origin.x as usize + x] =
                        image.pixels[y * image.size[0] + x].a();
                }
            }
            output.drop_without_applying_deltas();
            // Independently render the exported paths into a whole canvas;
            // this oracle does not calculate a cropped texture placement.
            let b = wireframe::model::Bounds::new(
                rect.min.x * 2.,
                rect.min.y * 2.,
                rect.width() * 2.,
                rect.height() * 2.,
            );
            let svg = format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"256\" height=\"256\"><g transform=\"rotate({} {} {})\">{}</g></svg>",
                -i32::from(turns) * 90,
                b.x + b.w / 2.,
                b.y + b.h / 2.,
                icon.svg(b, [255; 4])
            );
            let tree = resvg::usvg::Tree::from_str(&svg, &Default::default()).unwrap();
            let mut expected = resvg::tiny_skia::Pixmap::new(256, 256).unwrap();
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::identity(),
                &mut expected.as_mut(),
            );
            let difference: Vec<_> = actual
                .iter()
                .zip(expected.data().as_chunks::<4>().0.iter())
                .map(|(a, b)| a.abs_diff(b[3]))
                .collect();
            assert!(
                difference.iter().all(|v| *v <= 1),
                "{id}, turn {turns}, max alpha difference {}",
                difference.iter().max().unwrap()
            );
        }
    }
}

#[test]
fn icon_pixels_map_one_to_one_at_fractional_positions_and_display_densities() {
    for density in [1., 1.25, 1.5, 2., 3., 5.] {
        for size in [
            12.,
            16.,
            24.,
            28.8,
            46.,
            96.,
            128. * wireframe::viewport::MAX_ZOOM,
        ] {
            for turns in 0..4 {
                let ctx = egui::Context::default();
                ctx.set_pixels_per_point(density);
                let output = ctx.run_ui(Default::default(), |ui| {
                    icons::get("arrow-right-solid").unwrap().paint_rotated(
                        ui.painter(),
                        Rect::from_min_size(Pos2::new(70.25, 90.125), Vec2::splat(size)),
                        Color32::BLACK,
                        turns,
                    );
                });
                let mesh = ctx
                    .tessellate(output.shapes.clone(), output.pixels_per_point)
                    .into_iter()
                    .find_map(|item| match item.primitive {
                        egui::epaint::Primitive::Mesh(mesh)
                            if mesh.texture_id != egui::TextureId::default() =>
                        {
                            Some(mesh)
                        }
                        _ => None,
                    })
                    .unwrap();
                let pixels = output.textures_delta.set[&mesh.texture_id]
                    .last()
                    .unwrap()
                    .image
                    .size();
                let bounds = mesh.calc_bounds();
                let size_error = (bounds.size() * density
                    - egui::vec2(pixels[0] as f32, pixels[1] as f32))
                .abs();
                let aligned = mesh.vertices.iter().all(|v| {
                    [v.pos.x * density, v.pos.y * density]
                        .iter()
                        .all(|p| (p - p.round()).abs() < 0.002)
                });
                output.drop_without_applying_deltas();
                assert!(
                    size_error.max_elem() < 0.002 && aligned,
                    "density {density}, size {size}, turns {turns}: {size_error:?}, aligned {aligned}"
                );
            }
        }
    }
}
