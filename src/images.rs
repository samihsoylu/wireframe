// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Bounded image decoding and project-owned import data.
use eframe::egui;
mod filter;
mod tint;
pub mod web;
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
// Keep actionable import constraints distinct from codec/IO diagnostics without
// exposing a new error type or changing the backend's String-returning API.
enum ReadFailure {
    Guidance(&'static str),
    Diagnostic(String),
}
impl ReadFailure {
    fn diagnostic(self) -> String {
        match self {
            Self::Guidance(message) => message.into(),
            Self::Diagnostic(message) => message,
        }
    }
    fn local_import_message(self) -> String {
        match self {
            Self::Guidance(message) => message.into(),
            Self::Diagnostic(error) => format!(
                "Couldn't import this image. Check that you can open the file, then try again or choose another image.\n\nDetails: {error}"
            ),
        }
    }
}
fn reader(bytes: &[u8]) -> Result<ImageReader<Cursor<&[u8]>>, ReadFailure> {
    if bytes.len() > MAX_BYTES {
        return Err(ReadFailure::Guidance("This image exceeds the 32 MB limit"));
    }
    let format = image::guess_format(bytes)
        .map_err(|_| ReadFailure::Guidance("Choose a PNG, JPEG or GIF image"))?;
    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif
    ) {
        return Err(ReadFailure::Guidance("Choose a PNG, JPEG or GIF image"));
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
    image_dimensions(bytes).map_err(ReadFailure::diagnostic)
}
fn image_dimensions(bytes: &[u8]) -> Result<(u32, u32), ReadFailure> {
    let (w, h) = reader(bytes)?
        .into_dimensions()
        .map_err(|e| ReadFailure::Diagnostic(format!("Cannot read image: {e}")))?;
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err(ReadFailure::Guidance(
            "This image exceeds the 64 megapixel decoding limit",
        ));
    }
    Ok((w, h))
}
fn decode(bytes: &[u8]) -> Result<DynamicImage, String> {
    decode_image(bytes).map_err(ReadFailure::diagnostic)
}
fn decode_image(bytes: &[u8]) -> Result<DynamicImage, ReadFailure> {
    image_dimensions(bytes)?;
    reader(bytes)?
        .decode()
        .map_err(|e| ReadFailure::Diagnostic(format!("Cannot decode image: {e}")))
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
/// Tint in linear light before the scene stretches the image to its icon square.
pub fn decoded_tinted(bytes: &[u8], color: [u8; 3]) -> Result<Decoded, String> {
    decoded_tinted_rotated(bytes, color, 0)
}
pub fn decoded_tinted_rotated(bytes: &[u8], color: [u8; 3], turns: u8) -> Result<Decoded, String> {
    let mut rgba = decode(bytes)?.to_rgba8();
    tint::apply(&mut rgba, color);
    let rgba = match turns % 4 {
        1 => image::imageops::rotate270(&rgba),
        2 => image::imageops::rotate180(&rgba),
        3 => image::imageops::rotate90(&rgba),
        _ => rgba,
    };
    from_rgba(rgba)
}
pub fn thumbnail(bytes: &[u8]) -> Result<Decoded, String> {
    from_rgba(decode(bytes)?.thumbnail(64, 64).to_rgba8())
}
fn from_rgba(rgba: image::RgbaImage) -> Result<Decoded, String> {
    Ok(Decoded {
        pixels: egui::ColorImage::from_rgba_unmultiplied(
            [rgba.width() as usize, rgba.height() as usize],
            rgba.as_raw(),
        ),
        png: png(&DynamicImage::ImageRgba8(rgba))?,
        texture: OnceLock::new(),
    })
}
impl Prepared {
    pub fn read(path: &Path) -> Result<Self, String> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|e| ReadFailure::Diagnostic(e.to_string()).local_import_message())?
            .take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| ReadFailure::Diagnostic(e.to_string()).local_import_message())?;
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("Image");
        Self::prepare(name, &bytes).map_err(ReadFailure::local_import_message)
    }
    pub fn from_bytes(name: &str, bytes: &[u8]) -> Result<Self, String> {
        Self::prepare(name, bytes).map_err(ReadFailure::diagnostic)
    }
    fn prepare(name: &str, bytes: &[u8]) -> Result<Self, ReadFailure> {
        let mut decoded = decode_image(bytes)?;
        let (w, h) = (decoded.width(), decoded.height());
        if f64::from(w) * f64::from(h) > IMPORT_PIXELS {
            let ratio = f64::from(w) / f64::from(h);
            let height = (IMPORT_PIXELS / ratio).sqrt().round().max(1.) as u32;
            let width = (f64::from(height) * ratio).round().max(1.) as u32;
            decoded = decoded.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
        }
        let alpha = decoded.to_rgba8().pixels().any(|p| p[3] < 255);
        let format =
            image::guess_format(bytes).map_err(|e| ReadFailure::Diagnostic(e.to_string()))?;
        let resized = (decoded.width(), decoded.height()) != (w, h);
        let (bytes, media_type) = if alpha {
            (
                if format == ImageFormat::Png && !resized {
                    bytes.to_vec()
                } else {
                    png(&decoded).map_err(ReadFailure::Diagnostic)?
                },
                "image/png",
            )
        } else if format == ImageFormat::Jpeg && !resized {
            (bytes.to_vec(), "image/jpeg")
        } else {
            let mut encoded = Vec::new();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, 92)
                .encode_image(&decoded.to_rgb8())
                .map_err(|e| ReadFailure::Diagnostic(e.to_string()))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_import_keeps_actionable_format_size_and_pixel_limit_messages() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Unsupported.png");
        std::fs::write(&path, b"not an image").unwrap();
        assert_eq!(
            Prepared::read(&path).unwrap_err(),
            "Choose a PNG, JPEG or GIF image"
        );

        let path = dir.path().join("Oversized.png");
        std::fs::File::create(&path)
            .unwrap()
            .set_len(MAX_BYTES as u64 + 1)
            .unwrap();
        assert_eq!(
            Prepared::read(&path).unwrap_err(),
            "This image exceeds the 32 MB limit"
        );

        // Header-only payload: dimensions are checked before any large decode.
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 10000, 7000);
            encoder.set_color(png::ColorType::Grayscale);
            let mut writer = encoder.write_header().unwrap();
            writer
                .write_chunk(
                    png::chunk::IDAT,
                    &[0x78, 0x9c, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01],
                )
                .unwrap();
        }
        assert_eq!(
            dimensions(&bytes).unwrap_err(),
            "This image exceeds the 64 megapixel decoding limit"
        );
        let path = dir.path().join("Too many pixels.png");
        std::fs::write(&path, bytes).unwrap();
        assert_eq!(
            Prepared::read(&path).unwrap_err(),
            "This image exceeds the 64 megapixel decoding limit"
        );
    }
}
