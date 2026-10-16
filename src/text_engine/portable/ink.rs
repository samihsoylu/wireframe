// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Bound path storage and image dimensions before calling the color rasterizer.
mod png_budget;
use cosmic_text::Font;
use skrifa::{
    FontRef, GlyphId, MetadataProvider,
    instance::{LocationRef, Size},
    outline::{DrawSettings, OutlinePen},
    raw::TableProvider,
};
use std::fmt::Write;

const MAX_PATH: usize = 1024 * 1024;
const MAX_COMMANDS: usize = 65536;
const MAX_SIDE: f64 = 4096.;
const MAX_PIXELS: f64 = 4_194_304.;
const MAX_ENCODED: usize = 16 * 1024 * 1024;
const MAX_LAYERS: usize = 256;

pub(super) struct Pen {
    pub path: String,
    shear: f32,
    commands: usize,
    bounds: Option<[f32; 4]>,
    error: Option<&'static str>,
    limit: usize,
}
impl Pen {
    pub fn new(shear: f32, limit: usize) -> Self {
        Self {
            path: String::new(),
            shear,
            commands: 0,
            bounds: None,
            error: None,
            limit: limit.min(MAX_PATH),
        }
    }
    fn emit(&mut self, op: &str, points: &[(f32, f32)]) {
        if self.error.is_some() {
            return;
        }
        self.commands += 1;
        if self.commands > MAX_COMMANDS || self.path.len().saturating_add(160) > self.limit {
            self.error = Some("Glyph outline exceeds path budget");
            return;
        }
        let mut transformed = [(0., 0.); 3];
        for (i, &(x, y)) in points.iter().enumerate() {
            let x = x + y * self.shear;
            let y = -y;
            if !x.is_finite() || !y.is_finite() || x.abs() > 1_000_000. || y.abs() > 1_000_000. {
                self.error = Some("Glyph outline coordinates exceed limits");
                return;
            }
            transformed[i] = (x, y);
            match &mut self.bounds {
                Some(b) => {
                    b[0] = b[0].min(x);
                    b[1] = b[1].min(y);
                    b[2] = b[2].max(x);
                    b[3] = b[3].max(y)
                }
                None => self.bounds = Some([x, y, x, y]),
            }
        }
        self.path.push_str(op);
        for &(x, y) in &transformed[..points.len()] {
            let _ = write!(self.path, "{x} {y} ");
        }
    }
    pub fn check(&self) -> Result<(), String> {
        self.error.map_or(Ok(()), |s| Err(s.into()))
    }
}
impl OutlinePen for Pen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.emit("M", &[(x, y)])
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.emit("L", &[(x, y)])
    }
    fn quad_to(&mut self, x: f32, y: f32, x1: f32, y1: f32) {
        self.emit("Q", &[(x, y), (x1, y1)])
    }
    fn curve_to(&mut self, x: f32, y: f32, x1: f32, y1: f32, x2: f32, y2: f32) {
        self.emit("C", &[(x, y), (x1, y1), (x2, y2)])
    }
    fn close(&mut self) {
        self.emit("Z", &[])
    }
}
fn dimensions(w: f64, h: f64) -> Result<(), String> {
    if !w.is_finite()
        || !h.is_finite()
        || w <= 0.
        || h <= 0.
        || w > MAX_SIDE
        || h > MAX_SIDE
        || w * h > MAX_PIXELS
    {
        return Err("Color glyph dimensions exceed raster budget".into());
    }
    Ok(())
}
fn raw_len(w: u32, h: u32, bpp: u8, packed: bool, len: usize) -> Result<(), String> {
    dimensions(f64::from(w), f64::from(h))?;
    if ![1, 2, 4, 8, 32].contains(&bpp) {
        return Err("Unsupported bitmap bit depth".into());
    }
    let bits = u64::from(w) * u64::from(bpp);
    let expected = if packed {
        (bits * u64::from(h)).div_ceil(8)
    } else {
        bits.div_ceil(8) * u64::from(h)
    };
    if u64::try_from(len).ok() != Some(expected) {
        return Err("Color bitmap data length does not match its dimensions".into());
    }
    Ok(())
}
fn outline_bounds(
    font: &FontRef<'_>,
    glyph: GlyphId,
    size: f32,
    loc: LocationRef<'_>,
    shear: f32,
    remaining: &mut usize,
) -> Result<Option<[f32; 4]>, String> {
    let Some(outline) = font.outline_glyphs().get(glyph) else {
        return Ok(None);
    };
    let mut pen = Pen::new(shear, *remaining);
    outline
        .draw(DrawSettings::unhinted(Size::new(size), loc), &mut pen)
        .map_err(|e| e.to_string())?;
    pen.check()?;
    *remaining = remaining.saturating_sub(pen.path.len());
    Ok(pen.bounds)
}
fn check_bounds(bounds: Option<[f32; 4]>) -> Result<(), String> {
    if let Some([x0, y0, x1, y1]) = bounds {
        dimensions(
            f64::from((x1 - x0).ceil()) + 4.,
            f64::from((y1 - y0).ceil()) + 4.,
        )?
    }
    Ok(())
}
/// Returns false for ordinary outlines. Bitmap metadata is read by the same
/// strike selector that Swash will use; no bitmap is decoded by this function.
pub(super) fn preflight(
    resolved: &Font,
    font: &FontRef<'_>,
    id: u16,
    size: f32,
    loc: LocationRef<'_>,
    shear: f32,
) -> Result<bool, String> {
    let mut needs_image = false;
    let mut remaining = MAX_PATH;
    if font.table_data(skrifa::Tag::new(b"COLR")).is_some() {
        let colr = font.colr().map_err(|e| e.to_string())?;
        let bases = colr
            .base_glyph_records()
            .transpose()
            .map_err(|e| e.to_string())?
            .unwrap_or(&[]);
        if let Some(base) = bases.iter().find(|b| b.glyph_id().to_u16() == id) {
            let count = usize::from(base.num_layers());
            if count > MAX_LAYERS {
                return Err("Color glyph exceeds layer budget".into());
            }
            let layers = colr
                .layer_records()
                .transpose()
                .map_err(|e| e.to_string())?
                .ok_or("Missing color layers")?;
            let first = usize::from(base.first_layer_index());
            let layers = layers
                .get(first..first + count)
                .ok_or("Invalid color layer range")?;
            let mut union: Option<[f32; 4]> = None;
            for layer in layers {
                let bounds = outline_bounds(
                    font,
                    GlyphId::new(u32::from(layer.glyph_id().to_u16())),
                    size,
                    loc,
                    shear,
                    &mut remaining,
                )?;
                check_bounds(bounds)?;
                if let Some(b) = bounds {
                    match &mut union {
                        Some(u) => {
                            u[0] = u[0].min(b[0]);
                            u[1] = u[1].min(b[1]);
                            u[2] = u[2].max(b[2]);
                            u[3] = u[3].max(b[3]);
                        }
                        None => union = Some(b),
                    }
                }
            }
            check_bounds(union)?;
            needs_image = count > 0;
        }
    }
    use skrifa::bitmap::{BitmapData, BitmapFormat, BitmapStrikes};
    let bitmap_format = if font.table_data(skrifa::Tag::new(b"sbix")).is_some() {
        Some(BitmapFormat::Sbix)
    } else if font.table_data(skrifa::Tag::new(b"CBDT")).is_some() {
        Some(BitmapFormat::Cbdt)
    } else {
        None
    };
    if let Some(format) = bitmap_format {
        let strikes =
            BitmapStrikes::with_format(font, format).ok_or("Invalid color bitmap table")?;
        if strikes.len() > 256 {
            return Err("Color font exceeds strike budget".into());
        }
        let selected = resolved
            .as_swash()
            .color_strikes()
            .find_by_nearest_ppem(size as u16, id);
        if let Some(selected) = selected {
            let ppem = selected.ppem();
            if ppem == 0 {
                return Err("Invalid color bitmap size".into());
            }
            let mut found = false;
            for strike in strikes.iter().filter(|s| s.ppem() == f32::from(ppem)) {
                let Some(bitmap) = strike.get(GlyphId::new(u32::from(id))) else {
                    continue;
                };
                found = true;
                dimensions(f64::from(bitmap.width), f64::from(bitmap.height))?;
                let (w, h, data_len) = match &bitmap.data {
                    BitmapData::Png(data) => {
                        if data.len() < 24
                            || &data[..8] != b"\x89PNG\r\n\x1a\n"
                            || &data[12..16] != b"IHDR"
                        {
                            return Err("Invalid glyph PNG header".into());
                        }
                        (
                            u32::from_be_bytes(data[16..20].try_into().unwrap()),
                            u32::from_be_bytes(data[20..24].try_into().unwrap()),
                            data.len(),
                        )
                    }
                    BitmapData::Bgra(data) => {
                        raw_len(bitmap.width, bitmap.height, 32, true, data.len())?;
                        (bitmap.width, bitmap.height, data.len())
                    }
                    BitmapData::Mask(mask) => {
                        raw_len(
                            bitmap.width,
                            bitmap.height,
                            mask.bpp,
                            mask.is_packed,
                            mask.data.len(),
                        )?;
                        (bitmap.width, bitmap.height, mask.data.len())
                    }
                };
                dimensions(f64::from(w), f64::from(h))?;
                let scale = f64::from(size) / f64::from(ppem);
                dimensions((f64::from(w) * scale).ceil(), (f64::from(h) * scale).ceil())?;
                if data_len > MAX_ENCODED {
                    return Err("Encoded color glyph exceeds byte budget".into());
                }
                if let BitmapData::Png(data) = &bitmap.data {
                    png_budget::validate(data, w, h)?;
                }
                if !bitmap.inner_bearing_x.is_finite()
                    || !bitmap.inner_bearing_y.is_finite()
                    || (f64::from(bitmap.inner_bearing_x) * scale).abs() > 1_000_000.
                    || (f64::from(bitmap.inner_bearing_y) * scale).abs() > 1_000_000.
                {
                    return Err("Color glyph placement exceeds limits".into());
                }
            }
            if !found {
                return Err("Cannot validate the selected color bitmap".into());
            }
            needs_image = true;
        }
    }
    if needs_image {
        // The library renderer can fall back to the monochrome base outline if a
        // color source cannot be decoded. Bound that fallback before invoking it.
        check_bounds(outline_bounds(
            font,
            GlyphId::new(u32::from(id)),
            size,
            loc,
            shear,
            &mut remaining,
        )?)?;
    } else if font
        .color_glyphs()
        .get_with_format(
            GlyphId::new(u32::from(id)),
            skrifa::color::ColorGlyphFormat::ColrV1,
        )
        .is_some()
    {
        return Err("COLRv1 glyph paint is not supported by this adapter yet".into());
    }
    Ok(needs_image)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dimensions_are_checked_before_integer_conversion() {
        for (w, h) in [
            (f64::INFINITY, 1.),
            (f64::NAN, 1.),
            (-1., 2.),
            (4097., 1.),
            (4096., 4096.),
            (u32::MAX as f64, 1.),
        ] {
            assert!(dimensions(w, h).is_err())
        }
        assert!(dimensions(2048., 2048.).is_ok());
    }
    #[test]
    fn outline_storage_stops_growing_at_the_limit() {
        let mut pen = Pen::new(0., 1024);
        for _ in 0..10000 {
            pen.line_to(10., 20.);
        }
        assert!(pen.check().is_err());
        assert!(pen.path.len() <= 1024);
        let bytes = pen.path.len();
        pen.curve_to(1., 2., 3., 4., 5., 6.);
        assert_eq!(bytes, pen.path.len());
    }
    #[test]
    fn raw_bitmap_lengths_match_packed_and_row_aligned_buffers() {
        assert!(raw_len(2, 1, 32, true, 8).is_ok());
        for len in [7, 9] {
            assert!(raw_len(2, 1, 32, true, len).is_err());
        }
        assert!(raw_len(3, 3, 1, true, 2).is_ok());
        assert!(raw_len(3, 3, 1, false, 3).is_ok());
        assert!(raw_len(3, 3, 1, false, 2).is_err());
        assert!(raw_len(u32::MAX, u32::MAX, 32, true, 0).is_err());
    }
    #[test]
    fn invalid_outline_coordinates_never_enter_path() {
        let mut pen = Pen::new(0., MAX_PATH);
        pen.move_to(0., 0.);
        let path = pen.path.clone();
        pen.line_to(f32::INFINITY, 1.);
        assert!(pen.check().is_err());
        assert_eq!(pen.path, path);
    }
}
