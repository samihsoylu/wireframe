// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Positioned text and reusable ink, independent of platform font handles.
//!
//! Shape each formatting span separately. Consumers must retain these positions
//! rather than asking another text engine to shape the source again.
#[cfg(target_os = "macos")]
mod bitmap;
mod cache;
#[cfg(any(not(target_os = "macos"), test))]
mod portable;
mod raster;
pub(crate) mod sfnt;
pub use raster::{Raster, paint};
#[cfg(target_os = "macos")]
mod font_source;
#[cfg(target_os = "macos")]
mod macos;
pub use cache::shape_cached;
use std::{ops::Range, sync::Arc};

pub const MAX_TEXT_BYTES: usize = 64 * 1024;
#[cfg(target_os = "macos")]
const MAX_INK_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug)]
pub enum Ink {
    Outline(String),
    Bitmap { png: Vec<u8>, bounds: [f64; 4] },
    Empty,
}
#[derive(Clone, Debug)]
pub struct Glyph {
    pub id: u16,
    /// UTF-8 cluster range in the unchanged source; several glyphs can share it.
    pub source: Range<usize>,
    pub position: [f64; 2],
    pub advance: [f64; 2],
    pub ink: Arc<Ink>,
}
#[derive(Clone, Debug)]
pub struct Run {
    pub source: Range<usize>,
    pub font_name: String,
    /// Exact face behind these glyph IDs. Never substitute the primary face.
    pub font_source: FontSource,
    pub font_size: f64,
    pub right_to_left: bool,
    pub glyphs: Vec<Glyph>,
}
#[derive(Clone, Debug)]
pub enum FontSource {
    Embedded(Arc<FontFace>),
    /// Preserve the already shaped ink when a portable face is unavailable.
    InkOnly(String),
}
#[derive(Debug)]
pub struct FontFace {
    pub(crate) data: FontData,
    /// Native collection members are materialized as standalone faces at index 0.
    pub index: u32,
    /// OpenType axis tags and design-space coordinates, not normalized values.
    pub variations: Vec<([u8; 4], f32)>,
}
type FontTables = Vec<(u32, Vec<u8>)>;

/// Font tables are retained during shaping, but serialization is deferred until
/// an exporter requests bytes. Canvas editing never computes whole-font checksums.
// Both positioned backends construct detached font sources.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
#[derive(Debug)]
pub(crate) enum FontData {
    Bytes(Arc<[u8]>),
    Tables {
        tables: std::sync::Mutex<Option<FontTables>>,
        size: usize,
        reserved: usize,
        packed: std::sync::OnceLock<Result<Box<[u8]>, String>>,
    },
}
impl FontData {
    fn tables(tables: FontTables, size: usize) -> Self {
        let retained = tables.capacity() * std::mem::size_of::<(u32, Vec<u8>)>()
            + tables
                .iter()
                .map(|(_, bytes)| bytes.capacity())
                .sum::<usize>();
        Self::Tables {
            tables: std::sync::Mutex::new(Some(tables)),
            size,
            reserved: retained.max(size),
            packed: Default::default(),
        }
    }
}
impl FontFace {
    pub fn data(&self) -> Result<&[u8], &str> {
        match &self.data {
            FontData::Bytes(bytes) => Ok(bytes),
            FontData::Tables {
                tables,
                size,
                packed,
                ..
            } => packed
                .get_or_init(|| {
                    // Transfer ownership once. The packer consumes and drops input tables;
                    // steady storage is tables OR packed bytes, never both.
                    let tables = tables
                        .lock()
                        .map_err(|_| "Font tables lock poisoned")?
                        .take()
                        .ok_or("Font tables already consumed")?;
                    let bytes = sfnt::pack(tables, *size)?;
                    skrifa::FontRef::new(&bytes)
                        .map_err(|_| "Invalid detached font face".to_string())?;
                    Ok(bytes.into_boxed_slice())
                })
                .as_deref()
                .map_err(String::as_str),
        }
    }
    fn storage_bytes(&self) -> usize {
        match &self.data {
            FontData::Bytes(bytes) => bytes.len(),
            // Stable reservation covers either steady representation. Packing
            // has a temporary output allocation outside the retained-cache cap.
            FontData::Tables { reserved, .. } => *reserved,
        }
    }
}
#[derive(Clone, Debug)]
pub struct ShapedText {
    pub source: String,
    pub width: f64,
    pub runs: Vec<Run>,
}
impl ShapedText {
    /// Cache accounting shares face storage by Arc identity, but conservatively
    /// counts repeated ink because consumers may materialize each occurrence.
    #[cfg(test)]
    pub(crate) fn storage_bytes(&self) -> usize {
        self.local_storage_bytes() + self.font_storage_bytes(&mut Default::default())
    }
    fn font_storage_bytes(&self, seen: &mut std::collections::HashSet<usize>) -> usize {
        self.runs
            .iter()
            .map(|run| match &run.font_source {
                FontSource::Embedded(face) if seen.insert(Arc::as_ptr(face) as usize) => {
                    std::mem::size_of::<FontFace>()
                        + face.storage_bytes()
                        + face.variations.capacity() * std::mem::size_of::<([u8; 4], f32)>()
                }
                _ => 0,
            })
            .sum()
    }
    fn local_storage_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.source.capacity()
            + self.runs.capacity() * std::mem::size_of::<Run>()
            + self
                .runs
                .iter()
                .map(|run| {
                    run.font_name.capacity()
                        + match &run.font_source {
                            FontSource::InkOnly(reason) => reason.capacity(),
                            _ => 0,
                        }
                        + run.glyphs.capacity() * std::mem::size_of::<Glyph>()
                        + run
                            .glyphs
                            .iter()
                            .map(|glyph| {
                                std::mem::size_of::<Ink>()
                                    + match &*glyph.ink {
                                        Ink::Outline(path) => path.capacity(),
                                        Ink::Bitmap { png, .. } => png.capacity(),
                                        Ink::Empty => 0,
                                    }
                            })
                            .sum::<usize>()
                })
                .sum::<usize>()
    }
    /// Baseline-relative SVG content. Outlines and bitmap positions are already
    /// shaped, so export does not require the original fonts or a second shaper.
    pub fn svg(&self, color: [u8; 4]) -> String {
        use base64::Engine as _;
        use std::fmt::Write as _;
        let mut out = format!(
            "<g fill=\"rgb({},{},{})\" opacity=\"{}\">",
            color[0],
            color[1],
            color[2],
            f64::from(color[3]) / 255.
        );
        for glyph in self.runs.iter().flat_map(|r| &r.glyphs) {
            let [x, y] = glyph.position;
            match &*glyph.ink {
                Ink::Outline(path) => {
                    let _ = write!(out, "<path transform=\"translate({x} {y})\" d=\"{path}\"/>");
                }
                Ink::Bitmap {
                    png,
                    bounds: [left, top, width, height],
                } => {
                    let encoded = base64::engine::general_purpose::STANDARD.encode(png);
                    let _ = write!(
                        out,
                        "<image x=\"{}\" y=\"{}\" width=\"{width}\" height=\"{height}\" href=\"data:image/png;base64,{encoded}\"/>",
                        x + left,
                        y + top
                    );
                }
                Ink::Empty => {}
            }
        }
        out.push_str("</g>");
        out
    }
}

/// Shape a single format span with an explicit, document-owned primary font.
/// System fallback fonts are resolved by the platform backend for missing glyphs.
/// macOS uses native shaping; Windows and Linux use the portable adapter.
pub fn shape(font: &[u8], text: &str, size: f64) -> Result<ShapedText, String> {
    if text.len() > MAX_TEXT_BYTES {
        return Err("Text span exceeds 64 KB".into());
    }
    if !size.is_finite() || size <= 0. || size > 512. {
        return Err("Text size must be greater than zero and at most 512".into());
    }
    if font.is_empty() || font.len() > crate::fonts::MAX_FACE_BYTES {
        return Err("Font must contain 1 byte to 4 MB".into());
    }
    #[cfg(target_os = "macos")]
    {
        macos::shape(font, text, size)
    }
    #[cfg(not(target_os = "macos"))]
    {
        portable::shape(font, text, size)
    }
}

#[cfg(test)]
mod storage_tests;
