// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Positioned text and reusable ink, independent of platform font handles.
//!
//! Shape each formatting span separately. Consumers must retain these positions
//! rather than asking another text engine to shape the source again.
#[cfg(target_os = "macos")]
mod bitmap;
mod cache;
mod raster;
pub use raster::{Raster, paint};
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
    pub font_size: f64,
    pub right_to_left: bool,
    pub glyphs: Vec<Glyph>,
}
#[derive(Clone, Debug)]
pub struct ShapedText {
    pub source: String,
    pub width: f64,
    pub runs: Vec<Run>,
}
impl ShapedText {
    /// Conservative resident/output size for bounded caches. Shared ink is
    /// counted per glyph because rendering/export can materialize each copy.
    pub(crate) fn storage_bytes(&self) -> usize {
        self.source.len()
            + self
                .runs
                .iter()
                .map(|run| {
                    std::mem::size_of::<Run>()
                        + run.font_name.len()
                        + run
                            .glyphs
                            .iter()
                            .map(|glyph| {
                                std::mem::size_of::<Glyph>()
                                    + match &*glyph.ink {
                                        Ink::Outline(path) => path.len(),
                                        Ink::Bitmap { png, .. } => png.len(),
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
/// Unsupported platforms return an explicit error until their adapter is ready.
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
        Err("Positioned text is not available on this platform yet".into())
    }
}
