// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Original deterministic chrome contours, independent of text and layout.
use super::*;
use std::f32::consts::PI;

use super::outlines::axis;

fn mesh(b: Bounds, a: &[[f32; 2]], z: &[[f32; 2]], fill: [u8; 3]) -> Primitive {
    let n = a.len();
    let points: Vec<_> = a
        .iter()
        .chain(z)
        .map(|p| {
            [
                b.x + axis(p[0], 450., b.w, 162., 93.),
                b.y + axis(p[1], 400., b.h, 77., 34.),
            ]
        })
        .collect();
    let mut indices = Vec::with_capacity((n - 1) * 6);
    for i in 0..n - 1 {
        for mut t in [[i, i + 1, n + i], [i + 1, n + i + 1, n + i]] {
            let [a, b, c] = t.map(|j| points[j]);
            if (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]) < 0. {
                t.swap(1, 2);
            }
            indices.extend(t.map(|j| j as u32));
        }
    }
    Primitive::Mesh {
        points,
        indices,
        fill,
    }
}

fn horizontal(y: impl Fn(f32) -> f32, left: f32, right: f32) -> Vec<[f32; 2]> {
    (0..=32)
        .map(|i| {
            let t = i as f32 / 32.;
            [left + (right - left) * t, y(t)]
        })
        .collect()
}

pub(super) fn sketch(b: Bounds) -> Vec<Primitive> {
    let wave = |t: f32| (PI * t).sin();
    let top = horizontal(|t| 0.7 + 0.4 * wave(t), 0.6, 449.4);
    let top_inner = horizontal(|t| 3.0 + 0.15 * wave(t), 2.8, 447.2);
    let bottom = horizontal(|t| 399.4 - 0.3 * wave(t), 0.6, 449.4);
    let bottom_inner = horizontal(|t| 397.0 - 0.15 * wave(t), 2.8, 447.2);
    let header_top = horizontal(|t| 71.7 - 0.7 * t, 0.6, 449.4);
    let header_bottom = horizontal(|t| 74.5 - 0.8 * t, 0.6, 449.4);
    let footer_top = horizontal(|t| 371.2 - wave(t) + 2.0 * t.powi(6), 0.6, 449.4);
    let footer_bottom = horizontal(|t| 374.0 - wave(t) + 2.0 * t.powi(6), 0.6, 449.4);
    let header_fill_top = horizontal(|_| 2.5, 2.0, 448.0);
    let header_fill_bottom = horizontal(|t| 72.2 - 0.7 * t, 2.0, 448.0);
    let footer_fill_top = horizontal(|t| 373.5 - wave(t) + 2.0 * t.powi(6), 2.0, 448.0);
    let footer_fill_bottom = horizontal(|_| 397.5, 2.0, 448.0);
    let mut out = vec![
        mesh(b, &header_fill_top, &header_fill_bottom, [193; 3]),
        mesh(b, &footer_fill_top, &footer_fill_bottom, [193; 3]),
        mesh(b, &top, &top_inner, [0; 3]),
        mesh(b, &bottom_inner, &bottom, [0; 3]),
        mesh(b, &header_top, &header_bottom, [0; 3]),
        mesh(b, &footer_top, &footer_bottom, [0; 3]),
    ];
    for right in [false, true] {
        let outer: Vec<_> = (0..=32)
            .map(|i| {
                let t = i as f32 / 32.;
                [
                    if right {
                        449.4 - 0.15 * wave(t)
                    } else {
                        0.6 + 0.15 * wave(t)
                    },
                    0.7 + 398.7 * t,
                ]
            })
            .collect();
        let inner: Vec<_> = (0..=32)
            .map(|i| {
                let t = i as f32 / 32.;
                [
                    if right {
                        447.2 - 0.15 * wave(t)
                    } else {
                        2.8 + 0.15 * wave(t)
                    },
                    3.0 + 394.0 * t,
                ]
            })
            .collect();
        out.push(mesh(b, &outer, &inner, [0; 3]));
    }
    out
}
