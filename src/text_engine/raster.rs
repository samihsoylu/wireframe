// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Rasterize the same positioned ink used by SVG exports, at the canvas scale.
use super::ShapedText;
use eframe::egui;
use egui::emath::GuiRounding as _;
use std::{collections::VecDeque, sync::Arc};

// Canvas text, visible page previews and the control library share this cache.
// Bound metadata independently while letting ordinary combined working sets stay warm.
const MAX_TEXTURE_ENTRIES: usize = 1024;
const MAX_TEXTURE_BYTES: usize = 32 * 1024 * 1024;

pub struct Raster {
    pub pixels: egui::ColorImage,
    /// Baseline-relative bounds in document points, including raster padding.
    pub bounds: egui::Rect,
}
impl ShapedText {
    pub fn raster(&self, color: [u8; 4], scale: f32) -> Result<Option<Raster>, String> {
        if !scale.is_finite() || scale <= 0. || scale > 32. {
            return Err("Invalid text raster scale".into());
        }
        let svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\">{}</svg>",
            self.svg(color)
        );
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default())
            .map_err(|e| e.to_string())?;
        let b = tree.root().abs_bounding_box();
        if b.width() <= 0. || b.height() <= 0. {
            return Ok(None);
        }
        let left = (b.x() * scale).floor() - 1.;
        let top = (b.y() * scale).floor() - 1.;
        let width = (b.right() * scale).ceil() - left + 1.;
        let height = (b.bottom() * scale).ceil() - top + 1.;
        if !width.is_finite()
            || !height.is_finite()
            || width > 16384.
            || height > 16384.
            || width * height > 8_388_608.
        {
            return Err("Text raster exceeds the rendering limit".into());
        }
        let mut pixmap = resvg::tiny_skia::Pixmap::new(width as u32, height as u32)
            .ok_or("Cannot allocate text raster")?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_row(scale, 0., 0., scale, -left, -top),
            &mut pixmap.as_mut(),
        );
        let pixels = pixmap
            .pixels()
            .iter()
            .map(|p| {
                egui::Color32::from_rgba_premultiplied(p.red(), p.green(), p.blue(), p.alpha())
            })
            .collect();
        Ok(Some(Raster {
            pixels: egui::ColorImage::new(
                [pixmap.width() as usize, pixmap.height() as usize],
                pixels,
            ),
            bounds: egui::Rect::from_min_size(
                egui::pos2(left / scale, top / scale),
                egui::vec2(width / scale, height / scale),
            ),
        }))
    }
}
#[derive(Clone)]
struct Entry {
    text: Arc<ShapedText>,
    color: [u8; 4],
    scale: f32,
    texture: egui::TextureHandle,
    bounds: egui::Rect,
    local_bytes: usize,
}
#[derive(Clone, Default)]
struct Textures(VecDeque<Entry>);
impl Textures {
    fn storage_bytes(&self) -> usize {
        let mut faces = Default::default();
        self.0
            .iter()
            .map(|entry| entry.local_bytes + entry.text.font_storage_bytes(&mut faces))
            .sum()
    }
}
/// Returns errors to the caller; no fallback reshaping occurs after positioning.
pub fn paint(
    painter: &egui::Painter,
    text: &Arc<ShapedText>,
    color: [u8; 4],
    baseline: egui::Pos2,
    zoom: f32,
) -> Result<(), String> {
    paint_rotated(painter, text, color, baseline, zoom, 0.)
}
/// Rotates already positioned ink without reshaping or changing the text cache.
pub fn paint_rotated(
    painter: &egui::Painter,
    text: &Arc<ShapedText>,
    color: [u8; 4],
    baseline: egui::Pos2,
    zoom: f32,
    rotation: f32,
) -> Result<(), String> {
    if !rotation.is_finite()
        || !zoom.is_finite()
        || zoom <= 0.
        || !baseline.x.is_finite()
        || !baseline.y.is_finite()
    {
        return Err("Invalid text paint coordinates".into());
    }
    let scale = (zoom * painter.ctx().pixels_per_point()).clamp(0.125, 32.);
    let key = egui::Id::new("wireframe-positioned-text-textures");
    let found = painter.ctx().data_mut(|d| {
        let cache = d.get_temp_mut_or_default::<Textures>(key);
        let index = cache
            .0
            .iter()
            .position(|e| Arc::ptr_eq(&e.text, text) && e.color == color && e.scale == scale)?;
        let entry = cache.0.remove(index)?;
        let found = (entry.texture.clone(), entry.bounds);
        cache.0.push_back(entry);
        Some(found)
    });
    let (texture, bounds) = if let Some(found) = found {
        found
    } else {
        let Some(raster) = text.raster(color, scale)? else {
            return Ok(());
        };
        let texture = painter.ctx().load_texture(
            "Positioned text",
            raster.pixels,
            egui::TextureOptions::LINEAR,
        );
        let bounds = raster.bounds;
        painter.ctx().data_mut(|d| {
            let cache = d.get_temp_mut_or_default::<Textures>(key);
            cache.0.push_back(Entry {
                local_bytes: texture.size()[0] * texture.size()[1] * 4 + text.local_storage_bytes(),
                text: text.clone(),
                color,
                scale,
                texture: texture.clone(),
                bounds,
            });
            while cache.0.len() > MAX_TEXTURE_ENTRIES || cache.storage_bytes() > MAX_TEXTURE_BYTES {
                cache.0.pop_front();
            }
        });
        (texture, bounds)
    };
    // The glyph texture is already antialiased at the physical display scale.
    // Align its origin so linear sampling does not blur it a second time.
    // Keep layout positions and export geometry unchanged.
    let rotate = egui::emath::Rot2::from_angle(rotation);
    let mut mesh = egui::Mesh::with_texture(texture.id());
    mesh.add_rect_with_uv(
        egui::Rect::from_min_max(
            (bounds.min.to_vec2() * zoom).to_pos2(),
            (bounds.max.to_vec2() * zoom).to_pos2(),
        ),
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1., 1.)),
        egui::Color32::WHITE,
    );
    let origin = baseline + rotate * (bounds.min.to_vec2() * zoom);
    let snap = if (rotation / std::f32::consts::FRAC_PI_2).fract().abs() < 0.00001 {
        origin.round_to_pixels(painter.ctx().pixels_per_point()) - origin
    } else {
        egui::Vec2::ZERO
    };
    for vertex in &mut mesh.vertices {
        vertex.pos = baseline + rotate * vertex.pos.to_vec2() + snap;
    }
    painter.add(egui::Shape::mesh(mesh));
    Ok(())
}

#[cfg(test)]
mod storage_tests {
    use super::super::{FontData, FontFace, storage_tests::text};
    use super::*;
    #[test]
    fn visible_text_working_set_reuses_textures_across_frames() {
        let ctx = egui::Context::default();
        let shared = Arc::new(FontFace {
            data: FontData::Bytes(Arc::from(vec![0; 1024])),
            index: 0,
            variations: vec![],
        });
        // The canvas, visible page previews and control library share this cache.
        // Keep a modest combined working set warm without changing its artwork.
        let texts: Vec<_> = (0..256)
            .map(|i| Arc::new(text(&format!("Visible caption {i}"), shared.clone())))
            .collect();
        for frame in 0..3 {
            let mut output = ctx.run_ui(Default::default(), |_| {
                let painter = ctx.layer_painter(egui::LayerId::background());
                for shaped in &texts {
                    paint(&painter, shaped, [0, 0, 0, 255], egui::pos2(0., 20.), 1.).unwrap();
                }
            });
            let uploads = output.textures_delta.set.len();
            let frees = output.textures_delta.free.len();
            output.textures_delta.clear();
            output.drop_without_applying_deltas();
            if frame > 0 {
                assert_eq!(uploads, 0, "stationary text uploaded again");
                assert_eq!(frees, 0, "visible text was evicted");
            }
        }
    }
    #[test]
    fn texture_cache_shares_large_faces_and_evicts_distinct_faces() {
        let ctx = egui::Context::default();
        let face = || {
            Arc::new(FontFace {
                data: FontData::Bytes(Arc::from(vec![0; 20 * 1024 * 1024])),
                index: 0,
                variations: vec![],
            })
        };
        let shared = face();
        ctx.run_ui(Default::default(), |_| {
            let painter = ctx.layer_painter(egui::LayerId::background());
            for i in 0..8 {
                paint(
                    &painter,
                    &Arc::new(text(&i.to_string(), shared.clone())),
                    [0, 0, 0, 255],
                    egui::pos2(0., 20.),
                    1.,
                )
                .unwrap();
            }
            let key = egui::Id::new("wireframe-positioned-text-textures");
            ctx.data_mut(|d| {
                let cache = d.get_temp_mut_or_default::<Textures>(key);
                assert_eq!(cache.0.len(), 8);
                assert!(cache.storage_bytes() < 32 * 1024 * 1024);
            });
            paint(
                &painter,
                &Arc::new(text("distinct", face())),
                [0, 0, 0, 255],
                egui::pos2(0., 20.),
                1.,
            )
            .unwrap();
            ctx.data_mut(|d| {
                let cache = d.get_temp_mut_or_default::<Textures>(key);
                assert_eq!(cache.0.len(), 1);
                assert!(cache.storage_bytes() < 32 * 1024 * 1024);
            });
        })
        .drop_without_applying_deltas();
    }
}
