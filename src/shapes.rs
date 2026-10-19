// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Geometric shape contours; saved dimensions are independent of orientation.
use std::f32::consts::{PI, TAU};

pub struct Geometry {
    pub outline: Vec<[f32; 2]>,
    pub thickness: f32,
    pub dashed: bool,
    pub tear: Option<(Vec<[f32; 2]>, bool)>,
}
pub fn geometry(shape: &str, rotation: f32, w: f32, h: f32, sketch: bool) -> Geometry {
    let thickness = if w < 10. || h < 10. {
        0.
    } else if sketch {
        (w.min(h) / 40.).clamp(2., 4.).floor()
    } else {
        1.
    };
    let t = thickness / 2.;
    let left = t;
    let right = w - t;
    let top = t;
    let bottom = h - t;
    let mut tear = None;
    let mut outline = match shape {
        "ellipse" => (0..96)
            .map(|i| {
                let a = i as f32 * TAU / 96.;
                [
                    w / 2. + (w / 2. - t) * a.cos(),
                    h / 2. + (h / 2. - t) * a.sin(),
                ]
            })
            .collect(),
        "diamond" => vec![
            [w / 2., top],
            [right, h / 2.],
            [w / 2., bottom],
            [left, h / 2.],
        ],
        "parallelogram" => vec![
            [w * 0.17, top],
            [right, top],
            [w * 0.83, bottom],
            [left, bottom],
        ],
        "triangle" => match (rotation / 90.).round() as u32 % 4 {
            1 => vec![[left, h / 2.], [right, top], [right, bottom]],
            2 => vec![[left, top], [right, top], [w / 2., bottom]],
            3 => vec![[left, top], [right, h / 2.], [left, bottom]],
            _ => vec![[w / 2., top], [right, bottom], [left, bottom]],
        },
        "star" => {
            // Original five-point contour, oriented with its leading point right.
            let raw: Vec<_> = (0..10)
                .map(|i| {
                    let a = -TAU / 5. + i as f32 * PI / 5.;
                    let r = if i % 2 == 0 { 1. } else { 0.48 };
                    [r * a.cos(), r * a.sin()]
                })
                .collect();
            let min_x = raw.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
            let max_x = raw.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max);
            let min_y = raw.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
            let max_y = raw.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max);
            raw.into_iter()
                .map(|p| {
                    [
                        thickness + (p[0] - min_x) / (max_x - min_x) * (w - 2. * thickness),
                        thickness + (p[1] - min_y) / (max_y - min_y) * (h - 2. * thickness),
                    ]
                })
                .collect()
        }
        "rounded_rectangle" | "dotted_rectangle" => {
            let r = if shape == "dotted_rectangle" {
                10.
            } else {
                (w / 8.).min(h / 8.).min(10.)
            };
            let r = r.min((w - thickness) / 2.).min((h - thickness) / 2.);
            let mut p = Vec::new();
            for (cx, cy, start) in [
                (right - r, top + r, -PI / 2.),
                (right - r, bottom - r, 0.),
                (left + r, bottom - r, PI / 2.),
                (left + r, top + r, PI),
            ] {
                for i in 0..=12 {
                    let a = start + i as f32 * PI / 24.;
                    p.push([cx + r * a.cos(), cy + r * a.sin()]);
                }
            }
            p
        }
        "torn_bottom" | "torn_top" if w >= 65. && h >= 40. => {
            let above = shape == "torn_bottom";
            let baseline = if above { h - t - 19. } else { t + 19. };
            let edge: Vec<_> = crate::browser::torn_edge(w - thickness, baseline, thickness)
                .into_iter()
                .map(|p| [p[0] + t, p[1]])
                .collect();
            let mask_edge = crate::browser::torn_edge(w, baseline, 0.);
            tear = Some((mask_edge, above));
            if above {
                let mut p = vec![[left, top], [right, top]];
                p.extend(edge.into_iter().rev());
                p
            } else {
                let mut p = edge;
                p.extend([[right, bottom], [left, bottom]]);
                p
            }
        }
        _ => vec![[left, top], [right, top], [right, bottom], [left, bottom]],
    };
    if sketch {
        // Small deterministic ink variation; no random state or imported paths.
        for p in &mut outline {
            let u = p[0] / w;
            let v = p[1] / h;
            p[0] += 0.8 * u * (1. - u) * (v * TAU).sin();
            p[1] += 0.8 * v * (1. - v) * (u * TAU).sin();
        }
    }
    Geometry {
        outline,
        thickness,
        dashed: shape == "dotted_rectangle",
        tear,
    }
}
