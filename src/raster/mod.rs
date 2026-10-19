// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Streaming image output from the canonical scene, independent of UI and file publication.
pub mod archive;
pub mod job;
pub mod names;
pub mod plan;
pub mod publication;
use crate::{model::Bounds, render::Scene};
use std::io::Write;

pub const MAX_DIMENSION: u32 = 20_000;
const TILE_WIDTH: u32 = 512;
const BAND_HEIGHT: u32 = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution {
    Standard,
    Double,
}
impl Resolution {
    pub fn scale(self) -> u32 {
        match self {
            Self::Standard => 1,
            Self::Double => 2,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundsRounding {
    Page,
    Selection,
}
#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub resolution: Resolution,
    pub transparent: bool,
    pub rounding: BoundsRounding,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            resolution: Resolution::Double,
            transparent: false,
            rounding: BoundsRounding::Page,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    pub width: u32,
    pub height: u32,
    /// Effective document-to-pixel scale, including any dimension limit.
    pub scale: f32,
    pub reduced: bool,
}
impl Geometry {
    pub fn new(bounds: Bounds, options: Options) -> Result<Self, String> {
        if [bounds.x, bounds.y, bounds.w, bounds.h]
            .iter()
            .any(|v| !v.is_finite())
            || bounds.w < 0.
            || bounds.h < 0.
            || !(bounds.x + bounds.w).is_finite()
            || !(bounds.y + bounds.h).is_finite()
        {
            return Err("Invalid image export bounds".into());
        }
        let round = |v: f32| match options.rounding {
            BoundsRounding::Page => f64::from(v).ceil().max(1.),
            BoundsRounding::Selection => f64::from(v).round().max(1.),
        };
        let width = round(bounds.w);
        let height = round(bounds.h);
        let density = f64::from(options.resolution.scale());
        let reduction = (f64::from(MAX_DIMENSION) / (width.max(height) * density)).min(1.);
        // Round logical dimensions before applying the chosen pixel density.
        let pixels = |v: f64| ((v * reduction).round().max(1.) * density) as u32;
        let scale = (density * reduction) as f32;
        if scale <= 0. || !scale.is_finite() {
            return Err("Image export scale is outside supported bounds".into());
        }
        Ok(Self {
            width: pixels(width).min(MAX_DIMENSION),
            height: pixels(height).min(MAX_DIMENSION),
            scale,
            reduced: reduction < 1.,
        })
    }
    fn viewport(self, bounds: Bounds) -> Bounds {
        Bounds::new(
            bounds.x,
            bounds.y,
            self.width as f32 / self.scale,
            self.height as f32 / self.scale,
        )
    }
}

/// Write a complete PNG without retaining the encoded image or a full-size
/// pixel buffer. The callback runs before preparation, between tiles and after
/// completed bands. Returning an error cancels encoding; callers must discard
/// the incomplete temporary destination instead of publishing it.
///
/// Working pixels use at most one 128-row band (10.24 MB at maximum width),
/// one 512-by-128 output tile and regional intermediate layers no larger than
/// that tile. Geometry and clipping retain the canonical logical canvas, so
/// partitioning output does not change curve coverage or shader coordinates.
/// Raster edges/scanlines and nested clip buffers use additional bounded space.
/// Scene/font/image resources have their own document validation limits.
pub fn write_png(
    scene: &Scene,
    options: Options,
    output: impl Write,
    mut checkpoint: impl FnMut(u32, u32) -> Result<(), String>,
) -> Result<Geometry, String> {
    let geometry = Geometry::new(scene.bounds, options)?;
    checkpoint(0, geometry.height)?;
    let svg = scene.svg_viewport(
        geometry.viewport(scene.bounds),
        geometry.width as f32,
        geometry.height as f32,
        !options.transparent,
    );
    let tree = crate::scene::export_tree(&svg, scene.font.as_deref())?;
    checkpoint(0, geometry.height)?;
    let mut encoder = png::Encoder::new(output, geometry.width, geometry.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .add_itxt_chunk(
            "Attribution".into(),
            crate::fonts::notices(scene.font.as_deref()).into(),
        )
        .map_err(|e| e.to_string())?;
    if scene
        .items
        .iter()
        .any(|i| matches!(i.drawing, crate::render::Drawing::Icon { .. }))
    {
        encoder
            .add_itxt_chunk("Icon attribution".into(), crate::icons::ATTRIBUTION.into())
            .map_err(|e| e.to_string())?;
    }
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    {
        let mut stream = writer.stream_writer().map_err(|e| e.to_string())?;
        let mut band = vec![0; geometry.width as usize * BAND_HEIGHT as usize * 4];
        for y in (0..geometry.height).step_by(BAND_HEIGHT as usize) {
            let height = BAND_HEIGHT.min(geometry.height - y);
            for x in (0..geometry.width).step_by(TILE_WIDTH as usize) {
                checkpoint(y, geometry.height)?;
                let width = TILE_WIDTH.min(geometry.width - x);
                let tile = resvg::render_region(
                    &tree,
                    resvg::tiny_skia::Transform::identity(),
                    resvg::tiny_skia::IntRect::from_xywh(x as i32, y as i32, width, height)
                        .ok_or("Invalid image export region")?,
                    || checkpoint(y, geometry.height),
                )?;
                for row in 0..height as usize {
                    let dst = (row * geometry.width as usize + x as usize) * 4;
                    let src = row * width as usize;
                    for (pixel, rgba) in tile.pixels()[src..src + width as usize].iter().zip(
                        band[dst..dst + width as usize * 4]
                            .as_chunks_mut::<4>()
                            .0
                            .iter_mut(),
                    ) {
                        let pixel = pixel.demultiply();
                        rgba.copy_from_slice(&[
                            pixel.red(),
                            pixel.green(),
                            pixel.blue(),
                            pixel.alpha(),
                        ]);
                    }
                }
            }
            stream
                .write_all(&band[..geometry.width as usize * height as usize * 4])
                .map_err(|e| format!("Cannot write PNG image: {e}"))?;
            checkpoint(y + height, geometry.height)?;
        }
        stream
            .finish()
            .map_err(|e| format!("Cannot finish PNG image: {e}"))?;
    }
    writer
        .finish()
        .map_err(|e| format!("Cannot finish PNG file: {e}"))?;
    Ok(geometry)
}

#[cfg(test)]
mod tests;
