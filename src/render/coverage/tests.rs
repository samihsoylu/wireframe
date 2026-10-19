use super::*;

fn rectangle(x0: f32, y0: f32, x1: f32, y1: f32) -> (Vec<[f32; 2]>, Vec<u32>) {
    (
        vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]],
        vec![0, 1, 2, 0, 2, 3],
    )
}
fn alpha(s: &Silhouette, origin: [i32; 2], size: [u32; 2]) -> Vec<u8> {
    s.raster(1., [0., 0.], TileRect { origin, size })
        .unwrap()
        .map_or_else(
            || vec![0; (size[0] * size[1]) as usize],
            |im| im.pixels.iter().map(|p| p.a()).collect(),
        )
}

#[test]
fn fractional_union_preserves_area_without_duplicate_darkening() {
    let (mut points, mut indices) = rectangle(10.125, 2., 10.625, 30.);
    let first = Silhouette::new(&points, &indices).unwrap();
    let mut duplicated = indices.clone();
    duplicated.extend(indices.iter().rev());
    assert_eq!(
        alpha(&first, [0, 0], [32, 32]),
        alpha(
            &Silhouette::new(&points, &duplicated).unwrap(),
            [0, 0],
            [32, 32]
        )
    );
    let (b, bi) = rectangle(10.375, 2., 10.875, 30.);
    indices.extend(bi.into_iter().map(|i| i + 4));
    points.extend(b);
    let union = Silhouette::new(&points, &indices).unwrap();
    let pixels = alpha(&union, [0, 0], [32, 32]);
    assert!(pixels[16 * 32 + 10].abs_diff(191) <= 1); // exactly 0.75 pixel area
    assert_eq!(pixels[16 * 32 + 11], 0);
}

#[test]
fn holes_and_shared_diagonals_have_their_geometric_coverage() {
    let ring = Silhouette::new(
        &[
            [2., 2.],
            [30., 2.],
            [30., 30.],
            [2., 30.],
            [8., 8.],
            [24., 8.],
            [24., 24.],
            [8., 24.],
        ],
        &[
            0, 1, 4, 1, 5, 4, 1, 2, 5, 2, 6, 5, 2, 3, 6, 3, 7, 6, 3, 0, 7, 0, 4, 7,
        ],
    )
    .unwrap();
    let pixels = alpha(&ring, [0, 0], [32, 32]);
    assert_eq!(pixels[16 * 32 + 16], 0);
    assert_eq!(pixels[16 * 32 + 4], 255);
    let (p, i) = rectangle(2., 2., 30., 30.);
    let pixels = alpha(&Silhouette::new(&p, &i).unwrap(), [0, 0], [32, 32]);
    assert!((2..30).all(|n| pixels[n * 32 + n] == 255));
}

#[test]
fn tile_origin_does_not_move_fractional_geometry() {
    let s = Silhouette::new(
        &[[0.375, 0.125], [600.5, 12.25], [60.125, 32.75]],
        &[0, 1, 2],
    )
    .unwrap();
    let full = alpha(&s, [0, 0], [256, 64]);
    let shifted = alpha(&s, [64, 0], [128, 64]);
    for y in 0..64 {
        for x in 0..128 {
            assert!(full[y * 256 + x + 64].abs_diff(shifted[y * 128 + x]) <= 1);
        }
    }
}

#[test]
fn extreme_offscreen_shapes_are_clipped_before_fixed_point_conversion() {
    let (p, i) = rectangle(-1e9, -1e9, 1e9, 1e9);
    assert!(
        alpha(&Silhouette::new(&p, &i).unwrap(), [0, 0], [32, 32])
            .iter()
            .all(|&a| a == 255)
    );
    let pixels = alpha(&Silhouette::new(&p, &[0, 2, 3]).unwrap(), [0, 0], [32, 32]);
    for y in 0..32 {
        for x in 0..32 {
            let expected = if y == x {
                128
            } else if y > x {
                255
            } else {
                0
            };
            assert!(pixels[y * 32 + x].abs_diff(expected) <= 1);
        }
    }
}

#[test]
fn empty_invalid_and_oversized_inputs_do_not_reach_the_rasterizer() {
    assert!(Silhouette::new(&[[f32::NAN, 0.]], &[0, 0, 0]).is_err());
    assert!(Silhouette::new(&[[0., 0.]], &[0, 1, 2]).is_err());
    assert!(Silhouette::new(&[[0., 0.]], &[0, 0]).is_err());
    let empty = Silhouette::new(&[[0., 0.]], &[0, 0, 0]).unwrap();
    assert!(
        empty
            .raster(
                1.,
                [0., 0.],
                TileRect {
                    origin: [0, 0],
                    size: [16, 16]
                }
            )
            .unwrap()
            .is_none()
    );
    assert!(
        empty
            .raster(
                1.,
                [0., 0.],
                TileRect {
                    origin: [0, 0],
                    size: [257, 16]
                }
            )
            .is_err()
    );
    assert!(
        empty
            .raster(
                f64::INFINITY,
                [0., 0.],
                TileRect {
                    origin: [0, 0],
                    size: [16, 16]
                }
            )
            .is_err()
    );
}
