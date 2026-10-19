// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::Icon;
use crate::model::Bounds;
use eframe::egui::{self, Color32, Pos2, Rect, TextureHandle, Vec2};
use std::collections::VecDeque;

#[derive(Debug)]
pub(super) struct Geometry {
    paths: Vec<resvg::tiny_skia::Path>,
    bounds: Bounds,
}

const MAX_RASTER: u32 = 2048;
const FALLBACK_RASTER: u32 = 512;
const MAX_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, PartialEq, Eq)]
enum RasterKey {
    Exact {
        size: [u32; 2],
        bounds: [u32; 4],
        turns: u8,
    },
    Scaled(u32),
}

struct Placement {
    rect: Rect,
    bounds: Bounds,
    size: [u32; 2],
    turns: u8,
}
impl Placement {
    fn new(icon: &Icon, rect: Rect, density: f32, turns: u8) -> Option<Self> {
        let density = f64::from(density);
        let width = f64::from(rect.width()) * density;
        let height = f64::from(rect.height()) * density;
        let mut extent = [width, height];
        if turns % 2 == 1 {
            extent.swap(0, 1);
        }
        let center = [
            f64::from(rect.center().x) * density,
            f64::from(rect.center().y) * density,
        ];
        // Include transparent padding so antialiasing at a view-box edge is
        // retained. Fractional phase stays in the raster, not in texture sampling.
        let mut low = std::array::from_fn::<_, 2, _>(|i| (center[i] - extent[i] / 2.).floor() - 1.);
        let mut high = std::array::from_fn::<_, 2, _>(|i| (center[i] + extent[i] / 2.).ceil() + 1.);
        // The rasterizer clips Bezier control points, not just visible ink.
        // Keep their full bounds inside the raster to retain the same curve
        // subdivision and coverage as an unclipped vector render.
        if let Some(b) = icon.path_bounds() {
            let scale = (width / f64::from(icon.width)).min(height / f64::from(icon.height));
            for x in [b.x, b.x + b.w] {
                for y in [b.y, b.y + b.h] {
                    let x = (f64::from(x) - f64::from(icon.width) / 2.) * scale;
                    let y = (f64::from(y) - f64::from(icon.height) / 2.) * scale;
                    let delta = match turns % 4 {
                        0 => [x, y],
                        1 => [y, -x],
                        2 => [-x, -y],
                        _ => [-y, x],
                    };
                    for i in 0..2 {
                        low[i] = low[i].min((center[i] + delta[i]).floor() - 1.);
                        high[i] = high[i].max((center[i] + delta[i]).ceil() + 1.);
                    }
                }
            }
        }
        let size = std::array::from_fn::<_, 2, _>(|i| high[i] - low[i]);
        if size
            .iter()
            .any(|v| !v.is_finite() || *v < 1. || *v > f64::from(MAX_RASTER))
        {
            return None;
        }
        Some(Self {
            rect: Rect::from_min_max(
                Pos2::new((low[0] / density) as f32, (low[1] / density) as f32),
                Pos2::new((high[0] / density) as f32, (high[1] / density) as f32),
            ),
            bounds: Bounds::new(
                (f64::from(rect.min.x) * density - low[0]) as f32,
                (f64::from(rect.min.y) * density - low[1]) as f32,
                width as f32,
                height as f32,
            ),
            size: size.map(|v| v as u32),
            turns: turns % 4,
        })
    }
    fn key(&self) -> RasterKey {
        let b = self.bounds;
        RasterKey::Exact {
            size: self.size,
            bounds: [b.x, b.y, b.w, b.h].map(f32::to_bits),
            turns: self.turns,
        }
    }
    fn raster(&self, icon: &Icon) -> Result<egui::ColorImage, String> {
        let [width, height] = self.size;
        let b = self.bounds;
        // Use the same SVG transform parser as the exported-path renderer. It
        // preserves decimal and rotation rounding without reparsing path data.
        let rotation: svgtypes::Transform = format!(
            "rotate({} {} {})",
            -i32::from(self.turns) * 90,
            b.x + b.w / 2.,
            b.y + b.h / 2.,
        )
        .parse()
        .map_err(|e: svgtypes::Error| e.to_string())?;
        let rotation = resvg::tiny_skia::Transform::from_row(
            rotation.a as f32,
            rotation.b as f32,
            rotation.c as f32,
            rotation.d as f32,
            rotation.e as f32,
            rotation.f as f32,
        );
        let fit = icon.fit(b);
        let transform = rotation.pre_concat(resvg::tiny_skia::Transform::from_row(
            fit.w / icon.width as f32,
            0.,
            0.,
            fit.h / icon.height as f32,
            fit.x,
            fit.y,
        ));
        let geometry = icon.geometry().ok_or("Icon geometry unavailable")?;
        let mut pixels =
            resvg::tiny_skia::Pixmap::new(width, height).ok_or("Icon allocation failed")?;
        let mut paint = resvg::tiny_skia::Paint::default();
        paint.set_color_rgba8(255, 255, 255, 255);
        paint.anti_alias = true;
        for path in &geometry.paths {
            pixels.fill_path(
                path,
                &paint,
                resvg::tiny_skia::FillRule::Winding,
                transform,
                None,
            );
        }
        Ok(egui::ColorImage::from_rgba_premultiplied(
            [width as usize, height as usize],
            pixels.data(),
        ))
    }
}

#[derive(Clone)]
struct Entry {
    id: String,
    key: RasterKey,
    texture: TextureHandle,
}
#[derive(Clone, Default)]
struct Textures {
    entries: VecDeque<Entry>,
    bytes: usize,
}
impl Textures {
    fn get(&mut self, id: &str, key: &RasterKey) -> Option<TextureHandle> {
        let index = self
            .entries
            .iter()
            .position(|e| e.id == id && e.key == *key)?;
        let entry = self.entries.remove(index)?;
        let texture = entry.texture.clone();
        self.entries.push_back(entry);
        Some(texture)
    }
    fn insert(&mut self, entry: Entry) {
        self.bytes += entry.texture.size()[0] * entry.texture.size()[1] * 4;
        self.entries.push_back(entry);
        while self.entries.len() > 128 || self.bytes > MAX_BYTES {
            if let Some(old) = self.entries.pop_front() {
                self.bytes -= old.texture.size()[0] * old.texture.size()[1] * 4;
            }
        }
    }
}

impl Icon {
    fn path_bounds(&self) -> Option<Bounds> {
        self.geometry().map(|geometry| geometry.bounds)
    }

    fn geometry(&self) -> Option<&Geometry> {
        self.geometry
            .get_or_init(|| {
                let svg = format!(
                    "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\">{}</svg>",
                    self.width,
                    self.height,
                    self.svg(
                        Bounds::new(0., 0., self.width as f32, self.height as f32),
                        [255; 4]
                    ),
                );
                let tree = resvg::usvg::Tree::from_str(&svg, &Default::default()).ok()?;
                fn collect(
                    group: &resvg::usvg::Group,
                    bounds: &mut Rect,
                    paths: &mut Vec<resvg::tiny_skia::Path>,
                ) -> Option<()> {
                    for node in group.children() {
                        match node {
                            resvg::usvg::Node::Group(group) => collect(group, bounds, paths)?,
                            resvg::usvg::Node::Path(path) => {
                                // Catalog SVG generation produces opaque, nonzero
                                // white fills only. Do not silently simplify other paint.
                                let fill = path.fill()?;
                                if path.stroke().is_some()
                                    || fill.rule() != resvg::usvg::FillRule::NonZero
                                    || fill.opacity().get() != 1.
                                    || !matches!(fill.paint(), resvg::usvg::Paint::Color(c) if c.red == 255 && c.green == 255 && c.blue == 255)
                                {
                                    return None;
                                }
                                let path = path.data().clone().transform(path.abs_transform())?;
                                let b = path.bounds();
                                *bounds = bounds.union(Rect::from_min_max(
                                    Pos2::new(b.left(), b.top()),
                                    Pos2::new(b.right(), b.bottom()),
                                ));
                                paths.push(path);
                            }
                            _ => return None,
                        }
                    }
                    Some(())
                }
                let mut bounds = Rect::NOTHING;
                let mut paths = Vec::new();
                collect(tree.root(), &mut bounds, &mut paths)?;
                bounds.is_finite().then(|| Geometry {
                    paths,
                    bounds: Bounds::new(bounds.min.x, bounds.min.y, bounds.width(), bounds.height()),
                })
            })
            .as_ref()
    }
    pub fn paint(&self, painter: &egui::Painter, rect: Rect, color: Color32) {
        self.paint_rotated(painter, rect, color, 0);
    }
    pub fn paint_rotated(&self, painter: &egui::Painter, rect: Rect, color: Color32, turns: u8) {
        let density = painter.ctx().pixels_per_point();
        if !rect.is_positive() || !rect.is_finite() || !density.is_finite() || density <= 0. {
            return;
        }
        let placement = Placement::new(self, rect, density, turns);
        let scale_size = ((rect.width().max(rect.height()) * density).ceil() as u32)
            .clamp(1, FALLBACK_RASTER)
            .next_power_of_two();
        let key = placement
            .as_ref()
            .map_or(RasterKey::Scaled(scale_size), Placement::key);
        let visible = placement.as_ref().map_or_else(
            || {
                let b = self.fit(Bounds::new(
                    rect.min.x,
                    rect.min.y,
                    rect.width(),
                    rect.height(),
                ));
                let size = if turns % 2 == 1 {
                    Vec2::new(b.h, b.w)
                } else {
                    Vec2::new(b.w, b.h)
                };
                Rect::from_center_size(rect.center(), size)
            },
            |p| p.rect,
        );
        if !painter.clip_rect().intersects(visible) {
            return;
        }
        let cache_id = egui::Id::new("wireframe-icon-textures");
        let found = painter.ctx().data_mut(|d| {
            d.get_temp_mut_or_default::<Textures>(cache_id)
                .get(&self.id, &key)
        });
        let texture = found.or_else(|| {
            let pixels = match &placement {
                Some(p) => p.raster(self),
                None => self.raster(scale_size),
            }
            .ok()?;
            let options = if placement.is_some() {
                egui::TextureOptions::NEAREST
            } else {
                egui::TextureOptions::LINEAR
            };
            let texture = painter
                .ctx()
                .load_texture(format!("icon:{}", self.id), pixels, options);
            painter.ctx().data_mut(|d| {
                d.get_temp_mut_or_default::<Textures>(cache_id)
                    .insert(Entry {
                        id: self.id.clone(),
                        key,
                        texture: texture.clone(),
                    })
            });
            Some(texture)
        });
        let Some(texture) = texture else {
            return;
        };
        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.));
        if let Some(placement) = placement {
            painter.image(texture.id(), placement.rect, uv, color);
        } else {
            // Retain bounded rendering for unusually large on-screen icons.
            // These still use filtered scaling; ordinary sizes map one-to-one.
            let b = self.fit(Bounds::new(
                rect.min.x,
                rect.min.y,
                rect.width(),
                rect.height(),
            ));
            let mut mesh = egui::epaint::Mesh::with_texture(texture.id());
            mesh.add_rect_with_uv(
                Rect::from_min_size(Pos2::new(b.x, b.y), Vec2::new(b.w, b.h)),
                uv,
                color,
            );
            let rotation =
                egui::emath::Rot2::from_angle(-f32::from(turns % 4) * std::f32::consts::FRAC_PI_2);
            for vertex in &mut mesh.vertices {
                vertex.pos = rect.center() + rotation * (vertex.pos - rect.center());
            }
            painter.add(egui::Shape::mesh(mesh));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_catalog_icons_have_supported_reusable_geometry() {
        let mut path_count = 0;
        let mut payload_bytes = 0;
        for icon in crate::icons::all() {
            let geometry = icon.geometry().expect(&icon.id);
            assert!(!geometry.paths.is_empty(), "{}", icon.id);
            for path in &geometry.paths {
                path_count += 1;
                payload_bytes +=
                    std::mem::size_of_val(path.points()) + std::mem::size_of_val(path.verbs());
            }
        }
        eprintln!(
            "Reusable catalog geometry: {} icons, {path_count} paths, {payload_bytes} point/verb payload bytes (excludes allocation/object overhead)",
            crate::icons::all().len()
        );
    }

    #[test]
    fn reused_paths_match_svg_rasterization_exactly() {
        for id in [
            "star-regular",
            "check-solid",
            "arrow-right-solid",
            "facebook-f-brands",
            "0-solid",
            "circle-regular",
            "address-book-regular",
            "apple-brands",
        ] {
            let icon = crate::icons::get(id).unwrap();
            for size in [24., 46., 128., 384.] {
                for turns in 0..4 {
                    for phase in [0.125, 0.21875] {
                        let placement = Placement::new(
                            icon,
                            Rect::from_min_size(
                                Pos2::new(60. + phase, 60.25),
                                Vec2::new(size, size * 0.9),
                            ),
                            2.,
                            turns,
                        )
                        .unwrap();
                        let [width, height] = placement.size;
                        let b = placement.bounds;
                        let svg = format!(
                            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\"><g transform=\"rotate({} {} {})\">{}</g></svg>",
                            -i32::from(turns) * 90,
                            b.x + b.w / 2.,
                            b.y + b.h / 2.,
                            icon.svg(b, [255; 4])
                        );
                        let tree = resvg::usvg::Tree::from_str(&svg, &Default::default()).unwrap();
                        let mut expected = resvg::tiny_skia::Pixmap::new(width, height).unwrap();
                        resvg::render(
                            &tree,
                            resvg::tiny_skia::Transform::identity(),
                            &mut expected.as_mut(),
                        );
                        let actual = placement.raster(icon).unwrap();
                        assert!(
                            actual
                                .pixels
                                .iter()
                                .zip(expected.data().as_chunks::<4>().0.iter())
                                .all(|(a, b)| a.to_array() == *b),
                            "{id}: size {size}, turn {turns}, phase {phase}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn placement_preserves_pixel_phase_and_bounds_large_allocations() {
        let rect = Rect::from_min_size(Pos2::new(100.25, 80.125), Vec2::splat(24.));
        let first =
            Placement::new(crate::icons::get("star-regular").unwrap(), rect, 2., 0).unwrap();
        let moved = Placement::new(
            crate::icons::get("star-regular").unwrap(),
            rect.translate(Vec2::new(10., 20.)),
            2.,
            0,
        )
        .unwrap();
        assert!(first.key() == moved.key());
        assert_eq!(moved.rect.min - first.rect.min, Vec2::new(10., 20.));
        assert!(
            first.key()
                != Placement::new(
                    crate::icons::get("star-regular").unwrap(),
                    rect.translate(Vec2::new(0.25, 0.)),
                    2.,
                    0
                )
                .unwrap()
                .key()
        );
        assert!(
            first.key()
                != Placement::new(crate::icons::get("star-regular").unwrap(), rect, 2., 1)
                    .unwrap()
                    .key()
        );
        assert!(
            Placement::new(
                crate::icons::get("star-regular").unwrap(),
                Rect::from_min_size(Pos2::ZERO, Vec2::splat(2000.)),
                2.,
                0
            )
            .is_none()
        );
    }

    #[test]
    fn texture_cache_evicts_by_bytes_and_entries_and_promotes_hits() {
        let ctx = egui::Context::default();
        let mut cache = Textures::default();
        let texture = ctx.load_texture(
            "small",
            egui::ColorImage::filled([2, 2], Color32::WHITE),
            egui::TextureOptions::NEAREST,
        );
        for i in 0..128 {
            cache.insert(Entry {
                id: i.to_string(),
                key: RasterKey::Scaled(2),
                texture: texture.clone(),
            });
        }
        assert!(cache.get("0", &RasterKey::Scaled(2)).is_some());
        cache.insert(Entry {
            id: "128".into(),
            key: RasterKey::Scaled(2),
            texture,
        });
        assert!(cache.get("0", &RasterKey::Scaled(2)).is_some());
        assert!(cache.get("1", &RasterKey::Scaled(2)).is_none());
        assert_eq!(cache.entries.len(), 128);
        let large = ctx.load_texture(
            "large",
            egui::ColorImage::filled([512, 512], Color32::WHITE),
            egui::TextureOptions::NEAREST,
        );
        for i in 0..20 {
            cache.insert(Entry {
                id: format!("large-{i}"),
                key: RasterKey::Scaled(512),
                texture: large.clone(),
            });
        }
        assert_eq!(cache.bytes, MAX_BYTES);
        assert_eq!(cache.entries.len(), 16);
        assert!(cache.get("large-0", &RasterKey::Scaled(512)).is_none());
        assert!(cache.get("large-19", &RasterKey::Scaled(512)).is_some());
    }
}
