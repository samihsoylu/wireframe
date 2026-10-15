// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Pointer panel geometry shared by canvas and export.
use crate::model::Bounds;
use serde::Serialize;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize)]
pub enum Side {
    #[default]
    Top,
    Right,
    Bottom,
    Left,
}
impl Side {
    pub fn parse(value: &str) -> Self {
        match value {
            "right" => Self::Right,
            "bottom" => Self::Bottom,
            "left" => Self::Left,
            _ => Self::Top,
        }
    }
}
#[derive(Debug, Serialize)]
pub struct Layout {
    pub outline: Vec<[f32; 2]>,
    pub arrow_outline: Vec<[f32; 2]>,
    pub stroke: f32,
    pub draw_box: bool,
    pub draw_arrow: bool,
    pub label: Option<Bounds>,
}
pub fn layout(w: f32, h: f32, side: Side, position: u8, sketch: bool) -> Layout {
    let (radius, stroke, padding) = if sketch { (15., 3., 8.) } else { (16., 1., 5.) };
    let draw_box = w > 40. + 2. * radius && h > 40. + 2. * radius;
    let draw_arrow = w > 40. && h > 40.;
    let corner = if draw_box { radius } else { 0. };
    let inset = stroke / 2.;
    let left = if draw_arrow && side == Side::Left {
        20.
    } else {
        0.
    };
    let top = if draw_arrow && side == Side::Top {
        20.
    } else {
        0.
    };
    let right = w - if draw_arrow && side == Side::Right {
        20.
    } else {
        0.
    };
    let bottom = h - if draw_arrow && side == Side::Bottom {
        20.
    } else {
        0.
    };
    let total = if matches!(side, Side::Top | Side::Bottom) {
        w
    } else {
        h
    };
    let tip = ((total - 2. * corner - 40.) * f32::from(position.min(4)) / 4. + corner + 20.)
        .max(20. + stroke)
        .min(total - 20. - stroke);
    // Perimeter segments run clockwise; each edge can include its pointer.
    let mut outline = Vec::new();
    let corners = [
        (
            left + inset + corner,
            top + inset + corner,
            std::f32::consts::PI,
        ),
        (
            right - inset - corner,
            top + inset + corner,
            1.5 * std::f32::consts::PI,
        ),
        (right - inset - corner, bottom - inset - corner, 0.),
        (
            left + inset + corner,
            bottom - inset - corner,
            std::f32::consts::FRAC_PI_2,
        ),
    ];
    let arrow = match side {
        Side::Top => vec![
            [tip - 20. + inset, top + inset],
            [tip, inset],
            [tip + 20. - inset, top + inset],
        ],
        Side::Right => vec![
            [right - inset, tip - 20. + inset],
            [w - inset, tip - inset],
            [right - inset, tip + 20. - inset],
        ],
        Side::Bottom => vec![
            [tip + 20. - inset, bottom - inset],
            [tip, h - inset],
            [tip - 20. + inset, bottom - inset],
        ],
        Side::Left => vec![
            [left + inset, tip + 20. - inset],
            [stroke, tip],
            [left + inset, tip - 20. + inset],
        ],
    };
    for (edge, (x, y, start)) in corners.into_iter().enumerate() {
        let steps = if draw_box { 16 } else { 0 };
        for i in 0..=steps {
            let angle = start
                + if steps > 0 {
                    i as f32 / steps as f32 * std::f32::consts::FRAC_PI_2
                } else {
                    0.
                };
            outline.push([x + corner * angle.cos(), y + corner * angle.sin()]);
        }
        if draw_arrow && edge == side as usize {
            outline.extend(&arrow);
        }
    }
    // Small panels keep an independent arrow stroke, offset from the fill path.
    let arrow_outline = if draw_arrow && !draw_box {
        let t = stroke;
        let t2 = inset;
        match side {
            Side::Top => vec![
                [tip - 20. + t2, 20. + t],
                [tip, t],
                [tip + 20. + t2, 20. + t],
            ],
            Side::Bottom => vec![
                [tip - 20. + t2, h - 20. - t2],
                [tip, h - t2],
                [tip + 20. + t2, h - 20. - t2],
            ],
            Side::Left => vec![
                [20. + t, tip - 20. + t2],
                [t, tip],
                [20. + t, tip + 20. + t2],
            ],
            Side::Right => vec![
                [w - 20. - t2, tip - 20. + t2],
                [w - t2, tip],
                [w - 20. - t2, tip + 20. + t2],
            ],
        }
    } else {
        vec![]
    };
    let tw = right - left - 2. * padding;
    let th = bottom - top - radius * 4. / 3.;
    Layout {
        outline,
        arrow_outline,
        stroke,
        draw_box,
        draw_arrow,
        label: (tw > 0. && th > 0.).then_some(Bounds::new(
            left + padding,
            top + radius * 2. / 3.,
            tw,
            th,
        )),
    }
}
/// Ear clipping preserves the concave pointer junction without painting seams.
pub fn triangles(points: &[[f32; 2]]) -> Vec<u32> {
    fn cross(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f32 {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    }
    let mut ring: Vec<usize> = (0..points.len()).collect();
    let signed_area: f32 = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum();
    // Loaded dimensions can be thinner than the stroke inset, reversing winding.
    if signed_area < 0. {
        ring.reverse();
    }
    let mut indices = vec![];
    while ring.len() > 2 {
        let mut found = false;
        for i in 0..ring.len() {
            let a = ring[(i + ring.len() - 1) % ring.len()];
            let b = ring[i];
            let c = ring[(i + 1) % ring.len()];
            let area = cross(points[a], points[b], points[c]);
            if area.abs() < 0.00001 {
                ring.remove(i);
                found = true;
                break;
            }
            if area < 0. {
                continue;
            }
            let occupied = ring.iter().any(|&p| {
                p != a
                    && p != b
                    && p != c
                    && cross(points[a], points[b], points[p]) >= -0.00001
                    && cross(points[b], points[c], points[p]) >= -0.00001
                    && cross(points[c], points[a], points[p]) >= -0.00001
            });
            if !occupied {
                indices.extend([a as u32, b as u32, c as u32]);
                ring.remove(i);
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
    }
    indices
}
