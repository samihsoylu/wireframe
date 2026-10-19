// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::{
    render::{Drawing, Item, mask::BottomMask},
    scene::Primitive,
};

fn scene() -> Scene {
    let b = Bounds::new(-17.25, 21.5, 1100., 380.);
    let item = |drawing| Item {
        drawing,
        opacity: 1.,
        clip: None,
    };
    let rect = |b, fill, radius| {
        item(Drawing::Primitive(Primitive::Rect {
            b,
            fill,
            radius,
            stroke: true,
        }))
    };
    Scene {
        title: "Masked overlapping artwork".into(),
        bounds: b,
        font: None,
        links: vec![],
        items: vec![
            item(Drawing::BeginOpacity(0.4)),
            item(Drawing::BeginMask(BottomMask {
                keep_above: true,
                bounds: b,
                edge: vec![
                    [-17.25, 350.],
                    [480., 249.75],
                    [680.25, 379.],
                    [1082.75, 300.],
                ],
            })),
            rect(b, [30, 100, 220], 41.25),
            rect(
                Bounds::new(483.5, 142.75, 190.5, 200.25),
                [250, 80, 10],
                31.75,
            ),
            item(Drawing::EndMask),
            item(Drawing::EndOpacity),
            item(Drawing::Icon {
                bounds: Bounds::new(470.5, 120.25, 170., 200.),
                id: "heart".into(),
                turns: 1,
                color: [90, 30, 120, 177],
            }),
        ],
    }
}
fn encode(scene: &Scene, options: Options) -> Vec<u8> {
    let mut bytes = vec![];
    write_png(scene, options, &mut bytes, |_, _| Ok(())).unwrap();
    bytes
}
fn full_frame(scene: &Scene, options: Options) -> image::RgbaImage {
    let geometry = Geometry::new(scene.bounds, options).unwrap();
    let svg = scene.svg_viewport(
        geometry.viewport(scene.bounds),
        geometry.width as f32,
        geometry.height as f32,
        !options.transparent,
    );
    let bytes = crate::scene::png_svg_with_fonts(&svg, None, scene.font.as_deref()).unwrap();
    image::load_from_memory(&bytes).unwrap().into_rgba8()
}
#[test]
fn dimensions_preserve_density_rounding_and_proportional_limit() {
    let options = Options::default();
    assert_eq!(
        Geometry::new(Bounds::new(-30., 60., 10000., 10000.), options).unwrap(),
        Geometry {
            width: 20000,
            height: 20000,
            scale: 2.,
            reduced: false
        }
    );
    let reduced = Geometry::new(Bounds::new(0., 0., 30000., 10000.), options).unwrap();
    assert_eq!((reduced.width, reduced.height), (20000, 6666));
    assert!(reduced.reduced && (reduced.scale - 2. / 3.).abs() < 0.00001);
    let b = Bounds::new(1.25, -9.5, 10.25, 4.4);
    assert_eq!(Geometry::new(b, options).unwrap().width, 22);
    assert_eq!(
        Geometry::new(
            b,
            Options {
                rounding: BoundsRounding::Selection,
                ..options
            }
        )
        .unwrap()
        .width,
        20
    );
    assert_eq!(
        Geometry::new(Bounds::new(0., 0., 0., 0.), options)
            .unwrap()
            .width,
        2
    );
    for b in [
        Bounds::new(f32::NAN, 0., 1., 1.),
        Bounds::new(0., 0., -1., 1.),
        Bounds::new(0., 0., f32::INFINITY, 1.),
    ] {
        assert!(Geometry::new(b, options).is_err());
    }
}
#[test]
fn tiled_png_matches_full_frame_across_mask_opacity_and_tile_boundaries() {
    let scene = scene();
    for transparent in [false, true] {
        for resolution in [Resolution::Standard, Resolution::Double] {
            let options = Options {
                transparent,
                resolution,
                ..Default::default()
            };
            let tiled = image::load_from_memory(&encode(&scene, options))
                .unwrap()
                .into_rgba8();
            let full = full_frame(&scene, options);
            assert_eq!(tiled.dimensions(), full.dimensions());
            let max_delta = tiled
                .as_raw()
                .iter()
                .zip(full.as_raw())
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            assert!(
                max_delta <= 1,
                "{transparent} {resolution:?}: maximum pixel difference {max_delta}"
            );
            if transparent {
                assert_eq!(tiled.get_pixel(100, 10).0[3], 102);
            } else {
                assert!(tiled.pixels().all(|p| p.0[3] == 255));
            }
        }
    }
}
#[test]
fn ungrouped_curves_keep_antialiasing_across_tiles() {
    let mut scene = scene();
    scene
        .items
        .retain(|item| matches!(item.drawing, Drawing::Primitive(_)));
    let options = Options {
        transparent: true,
        ..Default::default()
    };
    let tiled = image::load_from_memory(&encode(&scene, options))
        .unwrap()
        .into_rgba8();
    let full = full_frame(&scene, options);
    let delta = tiled
        .as_raw()
        .iter()
        .zip(full.as_raw())
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap();
    if delta > 1 {
        let differences: Vec<_> = tiled
            .enumerate_pixels()
            .filter_map(|(x, y, p)| {
                let q = full.get_pixel(x, y);
                let d =
                    p.0.iter()
                        .zip(q.0)
                        .map(|(a, b)| a.abs_diff(b))
                        .max()
                        .unwrap();
                (d > 1).then_some((x, y, p.0, q.0, d))
            })
            .take(30)
            .collect();
        eprintln!("Ungrouped differences: {differences:?}");
    }
    assert!(delta <= 1, "Ungrouped curve pixel difference {delta}");
}
#[test]
fn text_uses_the_canonical_scene_in_both_resolutions() {
    let mut editor = crate::workspace::Editor::default();
    editor.add(crate::model::Kind::TextArea, 490., 120.);
    editor.selected_mut(|c| {
        c.bounds = Bounds::new(490., 120., 750., 260.);
        c.text = "Planning a project — café & layout.\nSecond line with a few words.".into();
        c.font_size = 21.;
    });
    let scene = editor.rendered_scene(editor.page).unwrap();
    for resolution in [Resolution::Standard, Resolution::Double] {
        let options = Options {
            resolution,
            ..Default::default()
        };
        let tiled = image::load_from_memory(&encode(&scene, options))
            .unwrap()
            .into_rgba8();
        let full = full_frame(&scene, options);
        let max_delta = tiled
            .as_raw()
            .iter()
            .zip(full.as_raw())
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        assert!(max_delta <= 1, "Text pixel difference {max_delta}");
    }
}
#[test]
fn wide_output_streams_past_the_previous_dimension_limit() {
    let mut scene = scene();
    scene.bounds = Bounds::new(0., 0., 10000., 70.);
    scene.items = vec![Item {
        drawing: Drawing::Primitive(Primitive::Rect {
            b: Bounds::new(9940., 10., 40., 40.),
            fill: [30, 100, 220],
            stroke: false,
            radius: 20.,
        }),
        opacity: 1.,
        clip: None,
    }];
    let options = Options::default();
    let bytes = encode(&scene, options);
    let image = image::load_from_memory(&bytes).unwrap().into_rgba8();
    assert_eq!(image.dimensions(), (20000, 140));
    assert_eq!(image.get_pixel(0, 0).0, [255; 4]);
    assert_eq!(image.get_pixel(19920, 60).0, [30, 100, 220, 255]);
    assert!(image.pixels().any(|p| p.0[0] > 30 && p.0[0] < 255));
    scene.bounds = Bounds::new(9940., 10., 40., 40.);
    let expected = full_frame(&scene, options);
    let cropped = image::imageops::crop_imm(&image, 19880, 20, 80, 80).to_image();
    let delta = cropped
        .as_raw()
        .iter()
        .zip(expected.as_raw())
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap();
    assert!(delta <= 1, "Far-edge curve pixel difference {delta}");
}
#[test]
fn cancellation_and_write_failure_cannot_return_success() {
    let mut bytes = vec![];
    let mut checkpoints = vec![];
    let result = write_png(&scene(), Options::default(), &mut bytes, |row, total| {
        checkpoints.push((row, total));
        if row >= BAND_HEIGHT {
            Err("Cancelled".into())
        } else {
            Ok(())
        }
    });
    assert_eq!(result.unwrap_err(), "Cancelled");
    assert!(checkpoints.iter().any(|(row, _)| *row == BAND_HEIGHT));
    assert!(image::load_from_memory(&bytes).is_err());
    struct Fail;
    impl Write for Fail {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("destination unavailable"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    assert!(
        write_png(&scene(), Options::default(), Fail, |_, _| Ok(()))
            .unwrap_err()
            .contains("destination unavailable")
    );
    let mut empty = vec![];
    assert!(
        write_png(&scene(), Options::default(), &mut empty, |_, _| Err(
            "Cancelled".into()
        ))
        .is_err()
    );
    assert!(empty.is_empty());
}
#[test]
fn legacy_svg_and_png_keep_their_existing_framing() {
    let scene = scene();
    let old = image::load_from_memory(&scene.png().unwrap()).unwrap();
    assert_eq!((old.width(), old.height()), (1148, 428));
    let tight = image::load_from_memory(&encode(
        &scene,
        Options {
            resolution: Resolution::Standard,
            ..Default::default()
        },
    ))
    .unwrap();
    assert_eq!((tight.width(), tight.height()), (1100, 380));
}
