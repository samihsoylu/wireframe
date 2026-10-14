// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Bounded image decoding and project-owned import data.
use eframe::egui;
mod filter;
use image::{DynamicImage, ImageFormat, ImageReader};
use std::{
    io::{Cursor, Read},
    path::Path,
    sync::OnceLock,
};

pub const MAX_BYTES: usize = 32 * 1024 * 1024;
const MAX_PIXELS: u64 = 64 * 1024 * 1024;
const IMPORT_PIXELS: f64 = 2. * 1024. * 1024.;

#[derive(Clone, Debug)]
pub struct Prepared {
    pub name: String,
    pub media_type: String,
    pub bytes: Vec<u8>,
}
pub struct Decoded {
    pub pixels: egui::ColorImage,
    pub png: Vec<u8>,
    texture: OnceLock<egui::TextureHandle>,
}
impl std::fmt::Debug for Decoded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Decoded")
            .field("size", &self.pixels.size)
            .finish()
    }
}
impl Decoded {
    pub fn texture(&self, ctx: &egui::Context) -> egui::TextureId {
        self.texture
            .get_or_init(|| {
                ctx.load_texture(
                    "Embedded image",
                    self.pixels.clone(),
                    egui::TextureOptions::LINEAR,
                )
            })
            .id()
    }
}
fn reader(bytes: &[u8]) -> Result<ImageReader<Cursor<&[u8]>>, String> {
    if bytes.len() > MAX_BYTES {
        return Err("This image exceeds the 32 MB limit".into());
    }
    let format = image::guess_format(bytes).map_err(|_| "Choose a PNG, JPEG or GIF image")?;
    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif
    ) {
        return Err("Choose a PNG, JPEG or GIF image".into());
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(32768);
    limits.max_image_height = Some(32768);
    limits.max_alloc = Some(MAX_PIXELS * 4);
    reader.limits(limits);
    Ok(reader)
}
pub fn dimensions(bytes: &[u8]) -> Result<(u32, u32), String> {
    let (w, h) = reader(bytes)?
        .into_dimensions()
        .map_err(|e| format!("Cannot read image: {e}"))?;
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err("This image exceeds the 64 megapixel decoding limit".into());
    }
    Ok((w, h))
}
fn decode(bytes: &[u8]) -> Result<DynamicImage, String> {
    dimensions(bytes)?;
    reader(bytes)?
        .decode()
        .map_err(|e| format!("Cannot decode image: {e}"))
}
fn png(image: &DynamicImage) -> Result<Vec<u8>, String> {
    let mut out = Cursor::new(Vec::new());
    image
        .write_to(&mut out, ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}
pub fn quarter_turns(props: &crate::document::Properties) -> u8 {
    match props.get("rotation") {
        Some(crate::document::Value::Number(v)) => ((v / 90.).round() as i32).rem_euclid(4) as u8,
        _ => 0,
    }
}
pub fn decoded(bytes: &[u8]) -> Result<Decoded, String> {
    decoded_oriented(bytes, 0)
}
pub fn decoded_oriented(bytes: &[u8], turns: u8) -> Result<Decoded, String> {
    decoded_styled(bytes, turns, false)
}
pub fn decoded_styled(bytes: &[u8], turns: u8, sketch: bool) -> Result<Decoded, String> {
    let image = decode(bytes)?;
    let image = if sketch {
        DynamicImage::ImageRgba8(filter::sketch(&image.to_rgba8())?)
    } else {
        image
    };
    let image = match turns % 4 {
        1 => image.rotate90(),
        2 => image.rotate180(),
        3 => image.rotate270(),
        _ => image,
    };
    let rgba = image.to_rgba8();
    Ok(Decoded {
        pixels: egui::ColorImage::from_rgba_unmultiplied(
            [rgba.width() as usize, rgba.height() as usize],
            rgba.as_raw(),
        ),
        png: png(&image)?,
        texture: OnceLock::new(),
    })
}
impl Prepared {
    pub fn read(path: &Path) -> Result<Self, String> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("Image");
        Self::from_bytes(name, &bytes)
    }
    pub fn from_bytes(name: &str, bytes: &[u8]) -> Result<Self, String> {
        let mut decoded = decode(bytes)?;
        let (w, h) = (decoded.width(), decoded.height());
        if f64::from(w) * f64::from(h) > IMPORT_PIXELS {
            let ratio = f64::from(w) / f64::from(h);
            let height = (IMPORT_PIXELS / ratio).sqrt().round().max(1.) as u32;
            let width = (f64::from(height) * ratio).round().max(1.) as u32;
            decoded = decoded.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
        }
        let alpha = decoded.to_rgba8().pixels().any(|p| p[3] < 255);
        let format = image::guess_format(bytes).map_err(|e| e.to_string())?;
        let resized = (decoded.width(), decoded.height()) != (w, h);
        let (bytes, media_type) = if alpha {
            (
                if format == ImageFormat::Png && !resized {
                    bytes.to_vec()
                } else {
                    png(&decoded)?
                },
                "image/png",
            )
        } else if format == ImageFormat::Jpeg && !resized {
            (bytes.to_vec(), "image/jpeg")
        } else {
            let mut encoded = Vec::new();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, 92)
                .encode_image(&decoded.to_rgb8())
                .map_err(|e| e.to_string())?;
            (encoded, "image/jpeg")
        };
        let name = name
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or("Image")
            .to_owned();
        Ok(Self {
            name,
            media_type: media_type.into(),
            bytes,
        })
    }
}
