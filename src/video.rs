// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Region-based sizing for a static video placeholder.
use crate::model::Bounds;
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
pub struct Region {
    pub column: usize,
    pub row: usize,
    pub source: Bounds,
    pub destination: Bounds,
}
#[derive(Clone, Debug, Serialize)]
pub struct Layout {
    pub frame: Bounds,
    pub background: Bounds,
    pub regions: Vec<Region>,
    pub ink: [u8; 3],
    pub progress: [u8; 3],
}
pub fn layout(width: f32, height: f32, sketch: bool) -> Layout {
    let (left, right, top, bottom) = if sketch {
        (146., 74., 7., 43.)
    } else {
        (137., 71., 1., 39.)
    };
    // Double precision preserves the collapsed-center decision at small integer sizes.
    fn axis(total: f64, before: f64, after: f64) -> ([f64; 3], [f64; 3], bool) {
        let ratio = (total / (before + after)).min(1.);
        let a = before * ratio;
        let b = after * ratio;
        let middle = (total - (a + b)).max(0.);
        ([0., a, a + middle], [a, middle, b], middle == 0.)
    }
    let (xs, widths, flat_x) = axis(width as f64, left, right);
    let (ys, heights, flat_y) = axis(height as f64, top, bottom);
    let source_x = [0., left, 300. - right];
    let source_y = [0., top, 200. - bottom];
    let source_w = [left, 300. - left - right, right];
    let source_h = [top, 200. - top - bottom, bottom];
    let mut regions = Vec::new();
    for row in 0..3 {
        for column in 0..3 {
            if sketch && row == 1 && column == 1 {
                continue;
            }
            let corner = row != 1 && column != 1;
            let ox = if corner && flat_x { 0.5 } else { 0. };
            let oy = if corner && flat_y { 0.5 } else { 0. };
            let x = xs[column]
                - if column == 1 {
                    0.25
                } else if column == 2 {
                    ox
                } else {
                    0.
                };
            let y = ys[row]
                - if row == 1 {
                    0.25
                } else if row == 2 {
                    oy
                } else {
                    0.
                };
            let w = widths[column] + if column == 1 { 0.5 } else { ox };
            let h = heights[row] + if row == 1 { 0.5 } else { oy };
            regions.push(Region {
                column,
                row,
                source: Bounds::new(
                    source_x[column] as f32,
                    source_y[row] as f32,
                    source_w[column] as f32,
                    source_h[row] as f32,
                ),
                destination: Bounds::new(x as f32, y as f32, w as f32, h as f32),
            });
        }
    }
    Layout {
        frame: Bounds::new(0., 0., width, height),
        background: Bounds::new(1., 1., (width - 2.).max(1.), (height - 2.).max(1.)),
        regions,
        ink: if sketch { [0; 3] } else { [103; 3] },
        progress: if sketch { [205, 204, 204] } else { [152; 3] },
    }
}
