// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Context-local cache: font identity is retained so replacement cannot reuse stale ink.
use super::ShapedText;
use eframe::egui;
use std::{collections::VecDeque, sync::Arc};

const MAX_ENTRIES: usize = 128;
const MAX_BYTES: usize = 32 * 1024 * 1024;
#[derive(Clone)]
struct Entry {
    font: Arc<egui::FontData>,
    size: f32,
    text: Arc<ShapedText>,
    bytes: usize,
}
#[derive(Clone, Default)]
struct Cache {
    entries: VecDeque<Entry>,
    bytes: usize,
}
impl Cache {
    fn get(
        &mut self,
        font: &Arc<egui::FontData>,
        text: &str,
        size: f32,
    ) -> Option<Arc<ShapedText>> {
        let index = self
            .entries
            .iter()
            .position(|e| Arc::ptr_eq(&e.font, font) && e.size == size && e.text.source == text)?;
        let entry = self.entries.remove(index)?;
        let result = entry.text.clone();
        self.entries.push_back(entry);
        Some(result)
    }
    fn recount(&mut self) {
        let mut faces = Default::default();
        self.bytes = self
            .entries
            .iter()
            .map(|entry| entry.bytes + entry.text.font_storage_bytes(&mut faces))
            .sum();
    }
    fn insert(&mut self, entry: Entry) {
        if entry.bytes + entry.text.font_storage_bytes(&mut Default::default()) > MAX_BYTES {
            return;
        }
        self.entries.push_back(entry);
        self.recount();
        while self.entries.len() > MAX_ENTRIES || self.bytes > MAX_BYTES {
            if let Some(old) = self.entries.pop_front() {
                drop(old);
                self.recount();
            }
        }
    }
}
/// Returns None for a family without an installed canvas face.
/// Backend errors remain errors rather than silently changing the renderer.
pub fn shape_cached(
    ctx: &egui::Context,
    font_id: &egui::FontId,
    text: &str,
) -> Result<Option<Arc<ShapedText>>, String> {
    let name = font_id.family.to_string();
    if !name.starts_with("canvas-") {
        return Ok(None);
    }
    let Some(font) = ctx.fonts_mut(|f| f.definitions().font_data.get(&name).cloned()) else {
        return Ok(None);
    };
    let key = egui::Id::new("wireframe-positioned-text");
    if let Some(found) = ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Cache>(key)
            .get(&font, text, font_id.size)
    }) {
        return Ok(Some(found));
    }
    // Do not hold an egui font or data lock while invoking the platform engine.
    let shaped = Arc::new(super::shape(&font.font, text, f64::from(font_id.size))?);
    // Count repeated ink conservatively: an SVG consumer may duplicate that ink.
    let bytes = font.font.len() + shaped.local_storage_bytes();
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Cache>(key).insert(Entry {
            font,
            size: font_id.size,
            text: shaped.clone(),
            bytes,
        })
    });
    Ok(Some(shaped))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_evicts_by_bytes_and_count_and_promotes_hits() {
        let font = Arc::new(egui::FontData::from_static(crate::fonts::REGULAR));
        let make = |source: String, bytes| Entry {
            font: font.clone(),
            size: 24.,
            text: Arc::new(ShapedText {
                source,
                width: 0.,
                runs: vec![],
            }),
            bytes,
        };
        let mut cache = Cache::default();
        cache.insert(make("first".into(), MAX_BYTES / 2));
        cache.insert(make("second".into(), MAX_BYTES / 2));
        assert!(cache.get(&font, "first", 24.).is_some());
        cache.insert(make("third".into(), 1));
        assert!(cache.get(&font, "second", 24.).is_none());
        assert!(cache.get(&font, "first", 24.).is_some());
        cache.insert(make("oversized".into(), MAX_BYTES + 1));
        assert!(cache.get(&font, "oversized", 24.).is_none());
        for i in 0..MAX_ENTRIES + 1 {
            cache.insert(make(i.to_string(), 1));
        }
        assert_eq!(cache.entries.len(), MAX_ENTRIES);
        assert_eq!(cache.bytes, MAX_ENTRIES);
        assert!(cache.get(&font, "0", 24.).is_none());
    }
    #[test]
    fn shared_large_faces_survive_multiple_entries_and_distinct_faces_evict() {
        use super::super::{FontData, FontFace, storage_tests};
        let font = Arc::new(egui::FontData::from_static(crate::fonts::REGULAR));
        let face = || {
            Arc::new(FontFace {
                data: FontData::Bytes(Arc::from(vec![0; 20 * 1024 * 1024])),
                index: 0,
                variations: vec![],
            })
        };
        let shared = face();
        let mut cache = Cache::default();
        for i in 0..8 {
            let text = Arc::new(storage_tests::text(&i.to_string(), shared.clone()));
            let bytes = text.local_storage_bytes() + font.font.len();
            cache.insert(Entry {
                font: font.clone(),
                size: 17.,
                text,
                bytes,
            });
        }
        assert_eq!(cache.entries.len(), 8);
        assert!(cache.bytes < MAX_BYTES);
        assert!(cache.get(&font, "0", 17.).is_some());
        let text = Arc::new(storage_tests::text("other", face()));
        cache.insert(Entry {
            font: font.clone(),
            size: 17.,
            bytes: text.local_storage_bytes() + font.font.len(),
            text,
        });
        assert_eq!(cache.entries.len(), 1);
        assert!(cache.bytes < MAX_BYTES);
        assert!(cache.get(&font, "0", 17.).is_none());
    }
}
