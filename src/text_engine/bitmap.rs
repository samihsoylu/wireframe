// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::Ink;
use core_graphics::{
    base::{kCGBitmapByteOrder32Big, kCGImageAlphaPremultipliedLast},
    color_space::CGColorSpace,
    context::CGContext,
    geometry::CGPoint,
};
use core_text::{font::CTFont, font_descriptor::kCTFontOrientationHorizontal};

/// A glyph without an outline may be a color bitmap or an invisible control.
/// Render at 2x while retaining the original font size and its shaped advance.
pub(super) fn render(font: &CTFont, glyph: u16) -> Result<Ink, String> {
    let rect = font.get_bounding_rects_for_glyphs(kCTFontOrientationHorizontal, &[glyph]);
    let values = [
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
    ];
    if values.iter().any(|v| !v.is_finite()) {
        return Err("Non-finite glyph bounds".into());
    }
    if rect.size.width <= 0. || rect.size.height <= 0. {
        return Ok(Ink::Empty);
    }
    let left = rect.origin.x.floor() - 1.;
    let top = (-rect.origin.y - rect.size.height).floor() - 1.;
    let width = (rect.origin.x + rect.size.width).ceil() - left + 1.;
    let height = (-rect.origin.y).ceil() - top + 1.;
    let scale = 2.;
    if width > 2048. || height > 2048. || width * height * scale * scale > 4_194_304. {
        return Err("Glyph bitmap exceeds the rendering limit".into());
    }
    let pixels_w = (width * scale) as usize;
    let pixels_h = (height * scale) as usize;
    let mut ctx = CGContext::create_bitmap_context(
        None,
        pixels_w,
        pixels_h,
        8,
        pixels_w * 4,
        &CGColorSpace::create_device_rgb(),
        kCGImageAlphaPremultipliedLast | kCGBitmapByteOrder32Big,
    );
    ctx.scale(scale, scale);
    ctx.set_rgb_fill_color(0., 0., 0., 1.);
    font.draw_glyphs(&[glyph], &[CGPoint::new(-left, height + top)], ctx.clone());
    let bytes = ctx.data();
    if !bytes.as_chunks::<4>().0.iter().any(|p| p[3] != 0) {
        return Ok(Ink::Empty);
    }
    for pixel in bytes.as_chunks_mut::<4>().0 {
        let alpha = u32::from(pixel[3]);
        for channel in &mut pixel[..3] {
            *channel = (u32::from(*channel) * 255 + alpha / 2)
                .checked_div(alpha)
                .unwrap_or(0)
                .min(255) as u8;
        }
    }
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, pixels_w as u32, pixels_h as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .map_err(|e| e.to_string())?
            .write_image_data(bytes)
            .map_err(|e| e.to_string())?;
    }
    Ok(Ink::Bitmap {
        png,
        bounds: [left, top, width, height],
    })
}
