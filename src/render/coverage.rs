// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Pixel coverage of a filled triangle surface, preserving holes and overlaps.
mod cache;
pub use cache::paint;

use eframe::egui;
use i_overlay::{
    core::{fill_rule::FillRule, overlay_rule::OverlayRule},
    float::{simplify::SimplifyShape, single::SingleFloatOverlay},
};
use zeno::PathBuilder;

pub const TILE_SIZE: u32 = 256;
const PAD: i32 = 2;
const FIXED_GUARD: f64 = 32768.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileRect {
    pub origin: [i32; 2],
    pub size: [u32; 2],
}

/// One geometric union. Its input is retained for exact cache identity checks.
pub struct Silhouette {
    points: Vec<[f32; 2]>,
    indices: Vec<u32>,
    contours: Vec<Vec<[f64; 2]>>,
    bounds: [f64; 4],
}

impl Silhouette {
    pub fn new(points: &[[f32; 2]], indices: &[u32]) -> Result<Self, String> {
        if !indices.len().is_multiple_of(3)
            || points.iter().flatten().any(|v| !v.is_finite())
            || indices.iter().any(|&i| i as usize >= points.len())
        {
            return Err("Invalid outline geometry".into());
        }
        let triangles: Vec<Vec<[f64; 2]>> = indices
            .as_chunks::<3>()
            .0
            .iter()
            .filter_map(|t| {
                let mut p: Vec<_> = t
                    .iter()
                    .map(|&i| points[i as usize].map(f64::from))
                    .collect();
                let area = (p[1][0] - p[0][0]) * (p[2][1] - p[0][1])
                    - (p[1][1] - p[0][1]) * (p[2][0] - p[0][0]);
                if area == 0. {
                    return None;
                }
                if area < 0. {
                    p.swap(1, 2);
                }
                Some(p)
            })
            .collect();
        let contours: Vec<_> = triangles
            .simplify_shape_as::<i64>(FillRule::NonZero)
            .into_iter()
            .flatten()
            .collect();
        let mut bounds = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for p in contours.iter().flatten() {
            bounds[0] = bounds[0].min(p[0]);
            bounds[1] = bounds[1].min(p[1]);
            bounds[2] = bounds[2].max(p[0]);
            bounds[3] = bounds[3].max(p[1]);
        }
        Ok(Self {
            points: points.to_vec(),
            indices: indices.to_vec(),
            contours,
            bounds,
        })
    }

    fn storage_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.points.capacity() * std::mem::size_of::<[f32; 2]>()
            + self.indices.capacity() * std::mem::size_of::<u32>()
            + self.contours.capacity() * std::mem::size_of::<Vec<[f64; 2]>>()
            + self
                .contours
                .iter()
                .map(|c| c.capacity() * std::mem::size_of::<[f64; 2]>())
                .sum::<usize>()
    }

    /// Rasterize at physical-pixel scale. Only a bounded tile is allocated.
    /// The returned white pixels are premultiplied and may be tinted by the painter.
    pub fn raster(
        &self,
        scale: f64,
        offset: [f64; 2],
        tile: TileRect,
    ) -> Result<Option<egui::ColorImage>, String> {
        if !scale.is_finite()
            || scale <= 0.
            || offset.iter().any(|v| !v.is_finite())
            || tile.size.iter().any(|&v| v == 0 || v > TILE_SIZE)
        {
            return Err("Invalid outline raster coordinates".into());
        }
        if self.contours.is_empty() {
            return Ok(None);
        }
        let left = f64::from(tile.origin[0]) - f64::from(PAD);
        let top = f64::from(tile.origin[1]) - f64::from(PAD);
        let width = tile.size[0] + 2 * PAD as u32;
        let height = tile.size[1] + 2 * PAD as u32;
        let screen = |p: [f64; 2]| {
            [
                (p[0] * scale + offset[0] - left) * 2.,
                (p[1] * scale + offset[1] - top) * 2.,
            ]
        };
        let outside_guard = self.contours.iter().flatten().any(|&p| {
            screen(p)
                .iter()
                .any(|v| !v.is_finite() || v.abs() > FIXED_GUARD)
        });
        let clipped;
        let contours = if outside_guard {
            let doc = |x: f64, y: f64| [(x - offset[0]) / scale, (y - offset[1]) / scale];
            let clip = vec![
                doc(left, top),
                doc(left + f64::from(width), top),
                doc(left + f64::from(width), top + f64::from(height)),
                doc(left, top + f64::from(height)),
            ];
            if clip.iter().flatten().any(|v| !v.is_finite()) {
                return Err("Outline clipping exceeds the numeric range".into());
            }
            clipped = self
                .contours
                .overlay_as::<i64>(&clip, OverlayRule::Intersect, FillRule::NonZero)
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            &clipped
        } else {
            &self.contours
        };
        let mut path = Vec::<zeno::Command>::new();
        for contour in contours {
            let Some((&first, rest)) = contour.split_first() else {
                continue;
            };
            // Zeno uses unchecked fixed-point conversion. Enforce its safe input
            // range even after geometric clipping and floating-point conversion.
            if contour.iter().any(|&p| {
                screen(p)
                    .iter()
                    .any(|v| !v.is_finite() || v.abs() > FIXED_GUARD)
            }) {
                return Err("Outline coordinates exceed the raster range".into());
            }
            path.move_to(screen(first).map(|v| v as f32));
            for &p in rest {
                path.line_to(screen(p).map(|v| v as f32));
            }
            path.close();
        }
        if path.is_empty() {
            return Ok(None);
        }
        let stride = width as usize * 2;
        let mut alpha = vec![0u8; stride * height as usize * 2];
        zeno::Mask::new(&path)
            .size(width * 2, height * 2)
            .render_into(&mut alpha, None);
        let mut pixels = Vec::with_capacity(tile.size[0] as usize * tile.size[1] as usize);
        let mut any_ink = false;
        for y in 0..tile.size[1] as usize {
            for x in 0..tile.size[0] as usize {
                let i = (y + PAD as usize) * 2 * stride + (x + PAD as usize) * 2;
                let sum = u16::from(alpha[i])
                    + u16::from(alpha[i + 1])
                    + u16::from(alpha[i + stride])
                    + u16::from(alpha[i + stride + 1]);
                let a = ((sum + 2) / 4) as u8;
                any_ink |= a != 0;
                pixels.push(egui::Color32::from_rgba_premultiplied(a, a, a, a));
            }
        }
        Ok(any_ink.then(|| egui::ColorImage::new(tile.size.map(|v| v as usize), pixels)))
    }
}

#[cfg(test)]
mod tests;
