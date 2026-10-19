// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::{Silhouette, TILE_SIZE, TileRect};
use eframe::egui::{self, Color32, Pos2, Rect, TextureHandle};
use std::{
    collections::{VecDeque, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    sync::Arc,
};

const GEOMETRY_BYTES: usize = 8 * 1024 * 1024;
const TEXTURE_BYTES: usize = 32 * 1024 * 1024;
// Dense thumbnails contain many tiny outlines. Keep them reusable within the
// byte budgets instead of evicting a full page because of a low entry count.
// Entry caps still bound metadata and empty tile records independently of bytes.
const GEOMETRY_ENTRIES: usize = 4096;
const TILE_ENTRIES: usize = 4096;

#[derive(Clone)]
struct GeometryEntry {
    id: u64,
    hash: u64,
    data: Arc<Silhouette>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct TileKey {
    geometry: u64,
    scale: u64,
    phase: [u64; 2],
    rect: TileRect,
}
#[derive(Clone)]
struct TileEntry {
    key: TileKey,
    texture: Option<TextureHandle>,
    bytes: usize,
}
#[derive(Clone, Default)]
struct Cache {
    geometries: VecDeque<GeometryEntry>,
    tiles: VecDeque<TileEntry>,
    geometry_bytes: usize,
    texture_bytes: usize,
    next_id: u64,
}
fn cache_id() -> egui::Id {
    egui::Id::new("wireframe-outline-coverage")
}
fn geometry_hash(points: &[[f32; 2]], indices: &[u32]) -> u64 {
    let mut h = DefaultHasher::new();
    points.len().hash(&mut h);
    for v in points.iter().flatten() {
        (if *v == 0. { 0 } else { v.to_bits() }).hash(&mut h);
    }
    indices.hash(&mut h);
    h.finish()
}
impl Cache {
    fn geometry(
        &mut self,
        hash: u64,
        points: &[[f32; 2]],
        indices: &[u32],
    ) -> Option<GeometryEntry> {
        let i = self
            .geometries
            .iter()
            .position(|e| e.hash == hash && e.data.points == points && e.data.indices == indices)?;
        let e = self.geometries.remove(i)?;
        self.geometries.push_back(e.clone());
        Some(e)
    }
    fn insert_geometry(
        &mut self,
        hash: u64,
        data: Arc<Silhouette>,
    ) -> Result<GeometryEntry, String> {
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or("Outline cache identity exhausted")?;
        let e = GeometryEntry {
            id: self.next_id,
            hash,
            data,
        };
        self.geometry_bytes += e.data.storage_bytes();
        self.geometries.push_back(e.clone());
        while self.geometry_bytes > GEOMETRY_BYTES || self.geometries.len() > GEOMETRY_ENTRIES {
            if let Some(old) = self.geometries.pop_front() {
                self.geometry_bytes -= old.data.storage_bytes();
            }
        }
        Ok(e)
    }
    fn tile(&mut self, key: TileKey) -> Option<Option<TextureHandle>> {
        let i = self.tiles.iter().position(|e| e.key == key)?;
        let e = self.tiles.remove(i)?;
        let result = e.texture.clone();
        self.tiles.push_back(e);
        Some(result)
    }
    fn insert_tile(&mut self, key: TileKey, texture: Option<TextureHandle>) {
        let bytes = texture
            .as_ref()
            .map_or(0, |t| t.size()[0] * t.size()[1] * 4);
        self.texture_bytes += bytes;
        self.tiles.push_back(TileEntry {
            key,
            texture,
            bytes,
        });
        // The glow backend paints the frame before processing texture frees.
        // Evicted handles remain usable by shapes already emitted this frame.
        while self.texture_bytes > TEXTURE_BYTES || self.tiles.len() > TILE_ENTRIES {
            if let Some(old) = self.tiles.pop_front() {
                self.texture_bytes -= old.bytes;
            }
        }
    }
}

/// Draw a filled mesh with subpixel coverage and bounded retained resources.
pub fn paint(
    painter: &egui::Painter,
    points: &[[f32; 2]],
    indices: &[u32],
    fill: [u8; 3],
    origin: Pos2,
    zoom: f32,
) -> Result<(), String> {
    let density = f64::from(painter.ctx().pixels_per_point());
    let scale = f64::from(zoom) * density;
    let physical_origin = [f64::from(origin.x) * density, f64::from(origin.y) * density];
    if !scale.is_finite() || scale <= 0. || physical_origin.iter().any(|v| !v.is_finite()) {
        return Err("Invalid outline paint coordinates".into());
    }
    let hash = geometry_hash(points, indices);
    let found = painter.ctx().data_mut(|d| {
        d.get_temp_mut_or_default::<Cache>(cache_id())
            .geometry(hash, points, indices)
    });
    let geometry = if let Some(found) = found {
        found
    } else {
        let data = Arc::new(Silhouette::new(points, indices)?);
        painter.ctx().data_mut(|d| {
            d.get_temp_mut_or_default::<Cache>(cache_id())
                .insert_geometry(hash, data)
        })?
    };
    if geometry.data.contours.is_empty() {
        return Ok(());
    }
    let anchor = physical_origin.map(f64::floor);
    let phase = std::array::from_fn::<_, 2, _>(|i| physical_origin[i] - anchor[i]);
    let b = geometry.data.bounds;
    let start = [
        (b[0] * scale + phase[0]).floor() - 1.,
        (b[1] * scale + phase[1]).floor() - 1.,
    ];
    let end = [
        (b[2] * scale + phase[0]).ceil() + 1.,
        (b[3] * scale + phase[1]).ceil() + 1.,
    ];
    if start
        .iter()
        .chain(&end)
        .any(|v| !v.is_finite() || *v < f64::from(i32::MIN) + 4. || *v > f64::from(i32::MAX) - 4.)
    {
        return Err("Outline extent exceeds the tile range".into());
    }
    let clip = painter.clip_rect();
    let clip_start = [
        f64::from(clip.min.x) * density - anchor[0],
        f64::from(clip.min.y) * density - anchor[1],
    ];
    let clip_end = [
        f64::from(clip.max.x) * density - anchor[0],
        f64::from(clip.max.y) * density - anchor[1],
    ];
    if clip_start.iter().chain(&clip_end).any(|v| !v.is_finite()) {
        return Err("Invalid outline viewport".into());
    }
    let visible_start =
        std::array::from_fn::<_, 2, _>(|i| start[i].max(clip_start[i].floor() - 1.));
    let visible_end = std::array::from_fn::<_, 2, _>(|i| end[i].min(clip_end[i].ceil() + 1.));
    if (0..2).any(|i| visible_end[i] <= visible_start[i]) {
        return Ok(());
    }
    let first = std::array::from_fn::<_, 2, _>(|i| {
        ((visible_start[i] - start[i]) / f64::from(TILE_SIZE)).floor() as i64
    });
    let last = std::array::from_fn::<_, 2, _>(|i| {
        ((visible_end[i] - start[i]) / f64::from(TILE_SIZE)).ceil() as i64
    });
    let mut images = Vec::new();
    for y in first[1]..last[1] {
        for x in first[0]..last[0] {
            let pos = [
                start[0] as i64 + x * i64::from(TILE_SIZE),
                start[1] as i64 + y * i64::from(TILE_SIZE),
            ];
            let rect = TileRect {
                origin: pos.map(|v| v as i32),
                size: std::array::from_fn(|i| {
                    (end[i] as i64 - pos[i]).min(i64::from(TILE_SIZE)) as u32
                }),
            };
            let key = TileKey {
                geometry: geometry.id,
                scale: scale.to_bits(),
                phase: phase.map(f64::to_bits),
                rect,
            };
            let found = painter
                .ctx()
                .data_mut(|d| d.get_temp_mut_or_default::<Cache>(cache_id()).tile(key));
            let texture = if let Some(found) = found {
                found
            } else {
                let pixels = geometry.data.raster(scale, phase, rect)?;
                let texture = pixels.map(|pixels| {
                    painter.ctx().load_texture(
                        "Outline coverage",
                        pixels,
                        egui::TextureOptions::NEAREST,
                    )
                });
                painter.ctx().data_mut(|d| {
                    d.get_temp_mut_or_default::<Cache>(cache_id())
                        .insert_tile(key, texture.clone())
                });
                texture
            };
            if let Some(texture) = texture {
                let pos = Pos2::new(
                    ((anchor[0] + f64::from(rect.origin[0])) / density) as f32,
                    ((anchor[1] + f64::from(rect.origin[1])) / density) as f32,
                );
                images.push((
                    texture,
                    Rect::from_min_size(
                        pos,
                        egui::vec2(
                            rect.size[0] as f32 / density as f32,
                            rect.size[1] as f32 / density as f32,
                        ),
                    ),
                ));
            }
        }
    }
    // Emit only after all tiles succeed, so the caller can use a fallback without
    // painting any successful partial tiles twice.
    for (texture, rect) in images {
        painter.image(
            texture.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, egui::pos2(1., 1.)),
            Color32::from_rgb(fill[0], fill[1], fill[2]),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests;
