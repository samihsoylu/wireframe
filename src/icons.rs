// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Offline icon lookup, aspect fitting and shared vector/texture drawing.
//! Artwork is Font Awesome Free, separately licensed CC BY 4.0.
use crate::model::Bounds;
use eframe::egui::{self, Color32};
use serde::Deserialize;
use std::{collections::VecDeque, sync::LazyLock};

pub const ATTRIBUTION: &str = "Font Awesome Free 6.7.2 by Fonticons, Inc. — https://fontawesome.com — CC BY 4.0 https://creativecommons.org/licenses/by/4.0/ — path data unchanged";
#[derive(Debug, Deserialize)]
pub struct Icon {
    pub id: String,
    pub label: String,
    pub style: String,
    pub keywords: Vec<String>,
    pub width: u32,
    pub height: u32,
    pub paths: Vec<String>,
}
static CATALOG: LazyLock<Vec<Icon>> = LazyLock::new(|| {
    serde_json::from_str(include_str!(
        "../assets/icons/font-awesome-free/catalog.json"
    ))
    .expect("bundled icon catalog")
});
pub fn all() -> &'static [Icon] {
    &CATALOG
}
pub fn get(id: &str) -> Option<&'static Icon> {
    CATALOG
        .binary_search_by(|icon| icon.id.as_str().cmp(id))
        .ok()
        .map(|i| &CATALOG[i])
}
pub fn search(query: &str, style: &str) -> Vec<&'static Icon> {
    let query = query.to_lowercase();
    let terms: Vec<_> = query.split_whitespace().collect();
    let mut icons: Vec<_> = all()
        .iter()
        .filter(|icon| {
            (style.is_empty() || icon.style == style)
                && terms.iter().all(|term| {
                    icon.id.contains(term)
                        || icon.label.to_lowercase().contains(term)
                        || icon
                            .keywords
                            .iter()
                            .any(|word| word.to_lowercase().contains(term))
                })
        })
        .collect();
    // Put an exact name before semantic keyword matches.
    icons.sort_by_key(|icon| {
        !(icon.id == query
            || icon.id.strip_suffix(&format!("-{}", icon.style)) == Some(query.as_str())
            || icon.label.to_lowercase() == query)
    });
    icons
}
impl Icon {
    pub fn fit(&self, bounds: Bounds) -> Bounds {
        let scale = (bounds.w / self.width as f32).min(bounds.h / self.height as f32);
        let w = self.width as f32 * scale;
        let h = self.height as f32 * scale;
        Bounds::new(
            bounds.x + (bounds.w - w) / 2.,
            bounds.y + (bounds.h - h) / 2.,
            w,
            h,
        )
    }
    pub fn svg(&self, bounds: Bounds, color: [u8; 4]) -> String {
        let b = self.fit(bounds);
        let mut out = format!(
            "<g transform=\"translate({} {}) scale({} {})\" fill=\"{}\" fill-opacity=\"{}\">",
            b.x,
            b.y,
            b.w / self.width as f32,
            b.h / self.height as f32,
            crate::scene::color([color[0], color[1], color[2]]),
            color[3] as f32 / 255.
        );
        for path in &self.paths {
            out.push_str(&format!("<path d=\"{}\"/>", crate::scene::escape(path)));
        }
        out.push_str("</g>");
        out
    }
    /// White premultiplied pixels, colored by the painter without another cache entry.
    pub fn raster(&self, size: u32) -> Result<egui::ColorImage, String> {
        let size = size.clamp(1, 512);
        let max = self.width.max(self.height) as f32;
        let width = (size as f32 * self.width as f32 / max).ceil() as u32;
        let height = (size as f32 * self.height as f32 / max).ceil() as u32;
        let svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\">{}</svg>",
            self.svg(Bounds::new(0., 0., width as f32, height as f32), [255; 4])
        );
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default())
            .map_err(|e| e.to_string())?;
        let mut pixels =
            resvg::tiny_skia::Pixmap::new(width, height).ok_or("Icon allocation failed")?;
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixels.as_mut(),
        );
        Ok(egui::ColorImage::from_rgba_premultiplied(
            [width as usize, height as usize],
            pixels.data(),
        ))
    }
    pub fn paint(&self, painter: &egui::Painter, rect: egui::Rect, color: Color32) {
        self.paint_rotated(painter, rect, color, 0);
    }
    pub fn paint_rotated(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        color: Color32,
        turns: u8,
    ) {
        if !rect.is_positive() || !painter.clip_rect().intersects(rect) {
            return;
        }
        let size = ((rect.width().max(rect.height()) * painter.ctx().pixels_per_point()).ceil()
            as u32)
            .clamp(1, 512)
            .next_power_of_two();
        let key = egui::Id::new("wireframe-icon-textures");
        let found = painter.ctx().data_mut(|d| {
            let cache = d.get_temp_mut_or_default::<Textures>(key);
            let i = cache
                .0
                .iter()
                .position(|(id, s, _)| id == &self.id && *s == size)?;
            let entry = cache.0.remove(i)?;
            let texture = entry.2.clone();
            cache.0.push_back(entry);
            Some(texture)
        });
        let texture = found.or_else(|| {
            let pixels = self.raster(size).ok()?;
            let texture = painter.ctx().load_texture(
                format!("icon:{}:{size}", self.id),
                pixels,
                egui::TextureOptions::LINEAR,
            );
            painter.ctx().data_mut(|d| {
                let cache = d.get_temp_mut_or_default::<Textures>(key);
                cache.0.push_back((self.id.clone(), size, texture.clone()));
                while cache.0.len() > 128
                    || cache
                        .0
                        .iter()
                        .map(|(_, _, t)| t.size()[0] * t.size()[1] * 4)
                        .sum::<usize>()
                        > 16 * 1024 * 1024
                {
                    cache.0.pop_front();
                }
            });
            Some(texture)
        });
        if let Some(texture) = texture {
            let b = self.fit(Bounds::new(
                rect.min.x,
                rect.min.y,
                rect.width(),
                rect.height(),
            ));
            let mut mesh = egui::epaint::Mesh::with_texture(texture.id());
            mesh.add_rect_with_uv(
                egui::Rect::from_min_size(egui::pos2(b.x, b.y), egui::vec2(b.w, b.h)),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1., 1.)),
                color,
            );
            let rotation =
                egui::emath::Rot2::from_angle(-((turns % 4) as f32) * std::f32::consts::FRAC_PI_2);
            for vertex in &mut mesh.vertices {
                vertex.pos = rect.center() + rotation * (vertex.pos - rect.center());
            }
            painter.add(egui::Shape::mesh(mesh));
        }
    }
}
#[derive(Clone, Default)]
struct Textures(VecDeque<(String, u32, egui::TextureHandle)>);

/// An optional control icon keeps its size independently of text size.
pub struct Setting<'a> {
    pub source: &'a crate::document::IconSource,
    pub size: f32,
    pub turns: u8,
}
pub fn setting(props: &crate::document::Properties) -> Option<Setting<'_>> {
    use crate::document::Value;
    let Value::Icon(source) = props.get("icon")? else {
        return None;
    };
    let size = match props.get("icon_size") {
        Some(Value::Choice(s)) => s
            .parse::<u16>()
            .ok()
            .filter(|n| [16, 24, 32, 48, 64, 128].contains(n))
            .unwrap_or(24),
        _ => 24,
    };
    let turns = match props.get("icon_rotation") {
        Some(Value::Choice(s)) => match s.as_str() {
            "90" => 1,
            "180" => 2,
            "270" => 3,
            _ => 0,
        },
        _ => 0,
    };
    Some(Setting {
        source,
        size: size as f32,
        turns,
    })
}
pub fn menu_size(enabled: bool, font_size: f32) -> f32 {
    if !enabled {
        0.
    } else if font_size < 28. {
        10.
    } else if font_size < 60. {
        12.
    } else {
        24.
    }
}
