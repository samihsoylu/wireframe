// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Rasterize the same positioned ink used by SVG exports, at the canvas scale.
use super::ShapedText;
use eframe::egui;
use egui::emath::GuiRounding as _;
use std::{collections::VecDeque, sync::Arc};

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
}
#[derive(Clone, Default)]
struct Textures(VecDeque<Entry>);
/// Returns errors to the caller; no fallback reshaping occurs after positioning.
pub fn paint(
    painter: &egui::Painter,
    text: &Arc<ShapedText>,
    color: [u8; 4],
    baseline: egui::Pos2,
    zoom: f32,
) -> Result<(), String> {
    if !zoom.is_finite() || zoom <= 0. || !baseline.x.is_finite() || !baseline.y.is_finite() {
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
                text: text.clone(),
                color,
                scale,
                texture: texture.clone(),
                bounds,
            });
            while cache.0.len() > 128
                || cache
                    .0
                    .iter()
                    .map(|e| e.texture.size()[0] * e.texture.size()[1] * 4 + e.text.storage_bytes())
                    .sum::<usize>()
                    > 32 * 1024 * 1024
            {
                cache.0.pop_front();
            }
        });
        (texture, bounds)
    };
    // The glyph texture is already antialiased at the physical display scale.
    // Align its origin so linear sampling does not blur it a second time.
    // Keep layout positions and export geometry unchanged.
    let origin =
        (baseline + bounds.min.to_vec2() * zoom).round_to_pixels(painter.ctx().pixels_per_point());
    painter.image(
        texture.id(),
        egui::Rect::from_min_size(origin, bounds.size() * zoom),
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1., 1.)),
        egui::Color32::WHITE,
    );
    Ok(())
}
