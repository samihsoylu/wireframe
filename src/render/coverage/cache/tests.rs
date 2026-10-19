use super::*;

fn shape(x: f32) -> Arc<Silhouette> {
    Arc::new(
        Silhouette::new(
            &[[x, 0.], [x + 16., 0.], [x + 16., 16.], [x, 16.]],
            &[0, 1, 2, 0, 2, 3],
        )
        .unwrap(),
    )
}

#[test]
fn geometry_cache_checks_collisions_and_bounds_retained_data() {
    let mut cache = Cache::default();
    let a = shape(0.);
    let b = shape(1.);
    let first = cache.insert_geometry(42, a.clone()).unwrap();
    assert!(cache.geometry(42, &b.points, &b.indices).is_none());
    let second = cache.insert_geometry(42, b.clone()).unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(
        cache.geometry(42, &a.points, &a.indices).unwrap().id,
        first.id
    );
    for i in 0..GEOMETRY_ENTRIES {
        cache
            .insert_geometry(i as u64, shape(i as f32 + 2.))
            .unwrap();
    }
    assert_eq!(cache.geometries.len(), GEOMETRY_ENTRIES);
    assert!(cache.geometry_bytes <= GEOMETRY_BYTES);
    let oversized = Arc::new(Silhouette {
        points: vec![[0., 0.]; GEOMETRY_BYTES / 8 + 1],
        indices: vec![],
        contours: vec![],
        bounds: [0.; 4],
    });
    cache.insert_geometry(99, oversized).unwrap();
    assert!(cache.geometries.is_empty());
    assert_eq!(cache.geometry_bytes, 0);
}

struct Frame {
    uploaded: bool,
    shapes: Vec<egui::epaint::ClippedShape>,
}

fn frame(
    ctx: &egui::Context,
    origin: Pos2,
    zoom: f32,
    density: f32,
    points: &[[f32; 2]],
    tint: [u8; 3],
) -> Frame {
    ctx.set_pixels_per_point(density);
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(500., 500.))),
            ..Default::default()
        },
        |_| {
            let painter = ctx
                .layer_painter(egui::LayerId::background())
                .with_clip_rect(Rect::from_min_max(Pos2::ZERO, egui::pos2(500., 500.)));
            paint(&painter, points, &[0, 1, 2, 0, 2, 3], tint, origin, zoom).unwrap();
        },
    );
    let uploaded = !out.textures_delta.set.is_empty();
    out.textures_delta.clear();
    Frame {
        uploaded,
        shapes: std::mem::take(&mut out.shapes),
    }
}

#[test]
fn translation_and_tint_reuse_textures_but_fraction_scale_and_geometry_invalidate() {
    let ctx = egui::Context::default();
    let a = shape(0.);
    let b = shape(1.);
    let first = frame(
        &ctx,
        egui::pos2(10.125, 10.),
        0.75,
        2.,
        &a.points,
        [0, 0, 0],
    );
    assert!(first.uploaded);
    let integer_shift = frame(
        &ctx,
        egui::pos2(15.125, 10.),
        0.75,
        2.,
        &a.points,
        [100, 50, 20],
    );
    assert!(!integer_shift.uploaded);
    assert!(frame(&ctx, egui::pos2(15.25, 10.), 0.75, 2., &a.points, [0, 0, 0]).uploaded);
    assert!(frame(&ctx, egui::pos2(15.25, 10.), 0.8, 2., &a.points, [0, 0, 0]).uploaded);
    assert!(frame(&ctx, egui::pos2(15.25, 10.), 0.8, 2., &b.points, [0, 0, 0]).uploaded);
    let output = frame(&ctx, egui::pos2(15.25, 10.), 0.8, 1.5, &b.points, [0, 0, 0]);
    assert!(output.uploaded);
    for clipped in output.shapes {
        if let egui::Shape::Mesh(mesh) = clipped.shape {
            let bounds = mesh.calc_bounds();
            for v in [bounds.min.x, bounds.min.y, bounds.max.x, bounds.max.y] {
                assert!((v * 1.5 - (v * 1.5).round()).abs() < 0.0001);
            }
        }
    }
}

#[test]
fn texture_and_empty_tile_entries_are_bounded() {
    let ctx = egui::Context::default();
    let mut cache = Cache::default();
    let make_key = |n| TileKey {
        geometry: n,
        scale: 1,
        phase: [0, 0],
        rect: TileRect {
            origin: [0, 0],
            size: [256, 256],
        },
    };
    for n in 0..130 {
        let texture = ctx.load_texture(
            "coverage budget test",
            egui::ColorImage::filled([256, 256], Color32::WHITE),
            egui::TextureOptions::NEAREST,
        );
        cache.insert_tile(make_key(n), Some(texture));
    }
    assert_eq!(cache.texture_bytes, TEXTURE_BYTES);
    assert!(cache.tile(make_key(0)).is_none());
    for n in 130..(TILE_ENTRIES as u64 + 131) {
        cache.insert_tile(make_key(n), None);
    }
    assert_eq!(cache.tiles.len(), TILE_ENTRIES);
    assert_eq!(cache.texture_bytes, 0);
    assert!(matches!(
        cache.tile(make_key(TILE_ENTRIES as u64 + 130)),
        Some(None)
    ));
}

#[test]
fn existing_mask_preserves_texture_coordinates_and_item_opacity() {
    let ctx = egui::Context::default();
    let a = shape(0.);
    let mut out = ctx.run_ui(Default::default(), |_| {
        let mut painter = ctx
            .layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("coverage-mask-test"),
            ))
            .with_clip_rect(Rect::from_min_max(Pos2::ZERO, egui::pos2(64., 64.)));
        painter.multiply_opacity(0.5);
        let start = ctx.graphics(|g| g.get(painter.layer_id()).map_or(0, |l| l.next_idx().0));
        paint(
            &painter,
            &a.points,
            &a.indices,
            [160, 80, 40],
            egui::pos2(10., 10.),
            1.,
        )
        .unwrap();
        crate::render::mask::finish(
            &painter,
            start,
            &crate::render::mask::BottomMask {
                keep_above: true,
                bounds: crate::model::Bounds::new(0., 0., 16., 8.),
                edge: vec![[0., 8.], [16., 8.]],
            },
            egui::pos2(10., 10.),
            1.,
        );
    });
    let mut vertices = 0;
    for shape in &out.shapes {
        if let egui::Shape::Mesh(mesh) = &shape.shape {
            if mesh.texture_id == egui::TextureId::default() {
                continue;
            }
            for v in &mesh.vertices {
                vertices += 1;
                assert!((10. - 0.001..=26. + 0.001).contains(&v.pos.x));
                assert!((10. - 0.001..=18. + 0.001).contains(&v.pos.y));
                assert!((v.uv.x - (v.pos.x - 9.) / 18.).abs() < 0.0001);
                assert!((v.uv.y - (v.pos.y - 9.) / 18.).abs() < 0.0001);
                assert!(v.color.a().abs_diff(128) <= 1);
            }
        }
    }
    out.textures_delta.clear();
    assert!(vertices > 0);
}

#[test]
fn dense_small_outlines_reuse_coverage_at_two_scales_within_byte_budgets() {
    let ctx = egui::Context::default();
    let points: Vec<_> = (0..1260)
        .map(|i| {
            let x = (i % 35) as f32 * 10.;
            let y = (i / 35) as f32 * 10.;
            [[x, y], [x + 8., y], [x + 8., y + 8.], [x, y + 8.]]
        })
        .collect();
    let mut previous = None;
    for pass in 0..3 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(500., 500.))),
                ..Default::default()
            },
            |_| {
                let painter = ctx.layer_painter(egui::LayerId::background());
                for (origin, zoom) in [(Pos2::ZERO, 0.25), (egui::pos2(200., 200.), 0.5)] {
                    for vertices in &points {
                        paint(
                            &painter,
                            vertices,
                            &[0, 1, 2, 0, 2, 3],
                            [24, 48, 80],
                            origin,
                            zoom,
                        )
                        .unwrap();
                    }
                }
            },
        );
        let uploaded = !output.textures_delta.set.is_empty();
        let shapes = std::mem::take(&mut output.shapes);
        output.textures_delta.clear();
        output.drop_without_applying_deltas();
        if pass > 0 {
            assert!(!uploaded, "unchanged dense outlines re-uploaded coverage");
            assert_eq!(previous.as_ref().unwrap(), &shapes);
        }
        previous = Some(shapes);
        ctx.data_mut(|d| {
            let cache = d.get_temp_mut_or_default::<Cache>(cache_id());
            assert!(cache.geometry_bytes <= GEOMETRY_BYTES);
            assert!(cache.texture_bytes <= TEXTURE_BYTES);
        });
    }
}
