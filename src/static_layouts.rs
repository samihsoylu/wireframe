// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Sizing, toolbar composition and original map/rule geometry.
use crate::{document::Dimension, model::Bounds};
pub fn size(kind: &str, width: Dimension, height: Dimension) -> Option<(f32, f32)> {
    let fixed = |d, fallback| match d {
        Dimension::Auto => fallback,
        Dimension::Fixed(v) => v,
    };
    Some(match kind {
        "formatting_toolbar" => (fixed(width, 261.), 29.),
        "vertical_rule" => (10., fixed(height, 100.)),
        "map" => (fixed(width, 252.), fixed(height, 222.)),
        "modal_overlay" => (fixed(width, 318.), fixed(height, 240.)),
        _ => return None,
    })
}
pub const TOOLBAR_ICONS: [&str; 10] = [
    "bold-solid",
    "italic-solid",
    "underline-solid",
    "strikethrough-solid",
    "list-ul-solid",
    "list-ol-solid",
    "rotate-left-solid",
    "rotate-right-solid",
    "image-regular",
    "face-smile-regular",
];
#[derive(Debug, serde::Serialize)]
pub struct Toolbar {
    pub icons: Vec<(usize, Bounds)>,
    pub separators: Vec<Bounds>,
    pub combo: Option<Bounds>,
}
pub fn toolbar(width: f32) -> Toolbar {
    let origins = [3., 21., 39., 57., 129., 147., 170., 188., 211., 229.];
    let limits = [23., 41., 59., 77., 149., 167., 190., 208., 231., 249.];
    Toolbar {
        icons: origins
            .into_iter()
            .zip(limits)
            .enumerate()
            .filter(|(_, (_, limit))| width > *limit)
            .map(|(i, (x, _))| (i, Bounds::new(x, 7., 15., 15.)))
            .collect(),
        separators: [(165., 170.), (206., 211.)]
            .into_iter()
            .filter(|(_, limit)| width > *limit)
            .map(|(x, _)| Bounds::new(x, 7., 2., 15.))
            .collect(),
        combo: (width > 131.).then_some(Bounds::new(75., 4., 51., 21.)),
    }
}
#[derive(Debug, serde::Serialize)]
pub struct Rule {
    pub thickness: f32,
    pub dash: f32,
    pub gap: f32,
    pub segments: Vec<[f32; 2]>,
}
pub fn rule(height: f32, dotted: bool, sketch: bool) -> Rule {
    let (dash, gap) = if dotted {
        (1., 8.)
    } else if sketch {
        (12., 24.)
    } else {
        (6., 12.)
    };
    let mut segments = vec![];
    let mut start = 0.;
    while start < height {
        segments.push([start, (start + dash).min(height)]);
        start += dash + gap;
    }
    Rule {
        thickness: if sketch { 3. } else { 2. },
        dash,
        gap,
        segments,
    }
}
/// An independently drawn gently tapered line retains its endpoint proportions.
pub fn rule_solid(height: f32) -> Vec<[f32; 2]> {
    let map = |y: f32| {
        let ratio = (f64::from(height) / 17.).min(1.);
        let middle = (f64::from(height) - 17. * ratio).max(0.);
        (if y <= 8. {
            f64::from(y) * ratio
        } else if y >= 58. {
            8. * ratio + middle + f64::from(y - 58.) * ratio
        } else {
            8. * ratio + f64::from(y - 8.) / 50. * middle
        }) as f32
    };
    [
        [4.1, 0.],
        [5.2, 1.],
        [5.3, 8.],
        [5.5, 31.],
        [5.2, 58.],
        [4.5, 66.],
        [3.6, 67.],
        [3.8, 58.],
        [3.6, 31.],
        [3.7, 8.],
    ]
    .into_iter()
    .map(|[x, y]| [x, map(y)])
    .collect()
}
// Split convex blocks along independently designed road strips. Emitting the
// remaining blocks, rather than painting roads white, preserves transparency.
fn half_plane(polygon: &[[f32; 2]], a: [f32; 2], b: [f32; 2], inside: bool) -> Vec<[f32; 2]> {
    if polygon.is_empty() {
        return vec![];
    }
    let side = |p: [f32; 2]| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
    let mut out = vec![];
    for (p, q) in polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
    {
        let sp = side(*p);
        let sq = side(*q);
        let pin = if inside { sp >= 0. } else { sp <= 0. };
        let qin = if inside { sq >= 0. } else { sq <= 0. };
        if pin {
            out.push(*p);
        }
        if pin != qin {
            let t = sp / (sp - sq);
            out.push([p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]);
        }
    }
    out
}
pub fn map_blocks() -> Vec<Vec<[f32; 2]>> {
    let mut blocks = vec![vec![[0., 0.], [252., 0.], [252., 222.], [0., 222.]]];
    let roads = [
        ([60., -10.], [100., 240.], 6.),
        ([136., -10.], [179., 240.], 6.),
        ([-10., 75.], [265., 30.], 6.),
        ([-10., 163.], [265., 141.], 6.),
        ([30., 69.], [44., 135.], 6.),
        ([106., 60.], [118., 126.], 6.),
        ([69., 151.], [76., 235.], 5.),
        ([122., 149.], [129., 235.], 5.),
        ([225., 142.], [232., 235.], 5.),
        ([151., 81.], [264., 60.], 5.),
        ([158., 106.], [234., 92.], 5.),
    ];
    for (a, b, width) in roads {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let length = f32::hypot(dx, dy);
        let nx = -dy / length * width / 2.;
        let ny = dx / length * width / 2.;
        let cut = [
            [a[0] - nx, a[1] - ny],
            [b[0] - nx, b[1] - ny],
            [b[0] + nx, b[1] + ny],
            [a[0] + nx, a[1] + ny],
        ];
        let mut next = vec![];
        for block in blocks {
            let mut remainder = block;
            for i in 0..4 {
                let outside = half_plane(&remainder, cut[i], cut[(i + 1) % 4], false);
                if outside.len() >= 3 {
                    next.push(outside);
                }
                remainder = half_plane(&remainder, cut[i], cut[(i + 1) % 4], true);
                if remainder.is_empty() {
                    break;
                }
            }
        }
        blocks = next;
    }
    blocks
}
pub fn map_palette(sketch: bool) -> [[u8; 3]; 3] {
    if sketch {
        [[235, 233, 226], [188, 213, 172], [250, 240, 120]]
    } else {
        [[235, 233, 225], [189, 213, 172], [250, 240, 119]]
    }
}
