// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

//! Bounded, shaped captions with ordinary label/button accessibility semantics.
use crate::text_entry::layout::{Engine, Layout, shared_engine};
use eframe::egui::{
    self, Color32, ColorImage, Pos2, Rect, Response, Sense, TextureHandle, Ui, Vec2,
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use unicode_segmentation::UnicodeSegmentation;

const MAX_GRAPHEMES: usize = 4096;
const MAX_ENTRIES: usize = 128;
const MAX_BYTES: usize = 16 * 1024 * 1024;

pub struct PreparedCaption {
    pub displayed: String,
    pub truncated: bool,
    pub image: ColorImage,
    pub ink_width: f32,
}

fn display_source(source: &str) -> (String, bool) {
    let mut end = 0;
    for (i, g) in source.grapheme_indices(true).take(MAX_GRAPHEMES) {
        if i + g.len() > 64 * 1024 {
            break;
        }
        end = i + g.len();
    }
    (
        source[..end]
            .replace("\r\n", " ")
            .replace(['\r', '\n'], " "),
        end < source.len(),
    )
}

pub fn prepare(
    engine: &mut Engine,
    source: &str,
    size: f32,
    width: f32,
    scale: f32,
    color: Color32,
) -> Result<PreparedCaption, String> {
    let (source, capped) = display_source(source);
    prepare_display(engine, &source, capped, size, width, scale, color)
}

fn prepare_display(
    engine: &mut Engine,
    source: &str,
    capped: bool,
    size: f32,
    width: f32,
    scale: f32,
    color: Color32,
) -> Result<PreparedCaption, String> {
    let mut displayed = source.to_owned();
    let mut layout = Layout::with_wrap(engine, &displayed, size, width, scale, false)?;
    let measure = |l: &Layout| l.rows.first().map_or(0., |r| r.rect.width());
    let truncated = capped || measure(&layout) > width;
    if truncated {
        let ends: Vec<_> = std::iter::once(0)
            .chain(source.grapheme_indices(true).map(|(i, g)| i + g.len()))
            .collect();
        let (mut lo, mut hi) = (0, ends.len());
        // Re-shape each candidate so ligatures, joining and bidi remain valid.
        while lo + 1 < hi {
            let mid = (lo + hi) / 2;
            let candidate = format!("{}…", &source[..ends[mid]]);
            let trial = Layout::with_wrap(engine, &candidate, size, width, scale, false)?;
            if measure(&trial) <= width {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        displayed = format!("{}…", &source[..ends[lo]]);
        layout = Layout::with_wrap(engine, &displayed, size, width, scale, false)?;
    }
    let ink_width = measure(&layout).min(width);
    let image = layout.raster(
        engine,
        Rect::from_min_size(Pos2::ZERO, Vec2::new(width, size * 1.4)),
        color,
    )?;
    Ok(PreparedCaption {
        displayed,
        truncated,
        image,
        ink_width,
    })
}

#[derive(Clone, PartialEq)]
struct Key {
    source: String,
    capped: bool,
    size: f32,
    width: f32,
    scale: f32,
    color: Color32,
}
struct Entry {
    key: Key,
    texture: TextureHandle,
    ink_width: f32,
    truncated: bool,
    bytes: usize,
}
#[derive(Default)]
struct Cache {
    entries: VecDeque<Entry>,
    bytes: usize,
}

/// Paint a single-line caption inside a fixed rectangle. The caller retains
/// interaction and semantic ownership; shaping never creates an input field.
pub fn paint(ui: &Ui, rect: Rect, source: &str, size: f32, color: Color32, centered: bool) -> bool {
    if rect.width() < 1. || rect.height() < 1. || !ui.is_rect_visible(rect) {
        return false;
    }
    let (source, capped) = display_source(source);
    let key = Key {
        source,
        capped,
        size,
        width: rect.width().min(16384.),
        scale: ui.ctx().pixels_per_point(),
        color,
    };
    let id = egui::Id::new("shaped-caption-cache");
    let cache = ui
        .ctx()
        .data_mut(|d| d.get_temp::<Arc<Mutex<Cache>>>(id))
        .unwrap_or_else(|| {
            let cache = Arc::new(Mutex::new(Cache::default()));
            ui.ctx().data_mut(|d| d.insert_temp(id, cache.clone()));
            cache
        });
    let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
    let hit = cache
        .entries
        .iter()
        .position(|e| e.key == key)
        .map(|i| cache.entries.remove(i).unwrap());
    let entry = if let Some(entry) = hit {
        entry
    } else {
        let engine = shared_engine(ui.ctx());
        let mut engine = engine.lock().unwrap_or_else(|e| e.into_inner());
        let Ok(caption) = prepare_display(
            &mut engine,
            &key.source,
            key.capped,
            key.size,
            key.width,
            key.scale,
            key.color,
        ) else {
            return true;
        };
        let bytes = caption.image.pixels.len() * 4 + key.source.len();
        let texture = ui
            .ctx()
            .load_texture("caption", caption.image, egui::TextureOptions::LINEAR);
        cache.bytes += bytes;
        Entry {
            key,
            texture,
            ink_width: caption.ink_width,
            truncated: caption.truncated,
            bytes,
        }
    };
    let x = rect.left()
        + if centered {
            (rect.width() - entry.ink_width).max(0.) / 2.
        } else {
            0.
        };
    let target = Rect::from_min_size(
        Pos2::new(x, rect.center().y - size * 0.7),
        Vec2::new(entry.key.width, size * 1.4),
    );
    ui.painter()
        .with_clip_rect(rect.intersect(ui.clip_rect()))
        .image(
            entry.texture.id(),
            target,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
            Color32::WHITE,
        );
    let truncated = entry.truncated;
    cache.entries.push_back(entry);
    while cache.entries.len() > MAX_ENTRIES || cache.bytes > MAX_BYTES {
        if let Some(old) = cache.entries.pop_front() {
            cache.bytes -= old.bytes;
        } else {
            break;
        }
    }
    truncated
}

pub fn label(ui: &mut Ui, source: &str, width: f32) -> Response {
    let size = egui::TextStyle::Body.resolve(ui.style()).size;
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(width.max(0.).min(ui.available_width()), size * 1.4),
        Sense::hover(),
    );
    paint(ui, rect, source, size, ui.visuals().text_color(), false);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), source)
    });
    response
}

pub fn selectable(ui: &mut Ui, selected: bool, source: &str, width: f32) -> Response {
    let size = egui::TextStyle::Body.resolve(ui.style()).size;
    let padding = ui.spacing().button_padding;
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(
            width.max(0.).min(ui.available_width()),
            (size * 1.4 + padding.y * 2.).max(ui.spacing().interact_size.y),
        ),
        Sense::click(),
    );
    let visuals = ui.style().interact_selectable(&response, selected);
    if selected || response.hovered() || response.has_focus() {
        ui.painter().rect(
            rect,
            visuals.corner_radius,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
    }
    paint(
        ui,
        rect.shrink2(padding),
        source,
        size,
        visuals.text_color(),
        false,
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            source,
        )
    });
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_caption_changes_evict_old_textures() {
        let ctx = egui::Context::default();
        for i in 0..140 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(700., 100.))),
                    ..Default::default()
                },
                |ui| {
                    label(ui, &format!("Caption {i} שלום"), 600.);
                },
            );
            output.textures_delta.clear();
        }
        let cache = ctx
            .data_mut(|d| d.get_temp::<Arc<Mutex<Cache>>>(egui::Id::new("shaped-caption-cache")))
            .unwrap();
        let cache = cache.lock().unwrap();
        assert!(cache.entries.len() <= MAX_ENTRIES && cache.bytes <= MAX_BYTES);
        assert!(
            !cache
                .entries
                .iter()
                .any(|e| e.key.source == "Caption 0 שלום")
        );
        assert!(
            cache
                .entries
                .iter()
                .any(|e| e.key.source == "Caption 139 שלום")
        );
        assert_eq!(
            cache.bytes,
            cache.entries.iter().map(|e| e.bytes).sum::<usize>()
        );
    }
}
