// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Retain actual native faces without depending on a font name or file path.
use super::{FontData, FontFace, FontSource, Ink};
use core_foundation::{
    base::TCFType,
    dictionary::{CFDictionary, CFDictionaryRef},
    number::CFNumber,
};
use core_text::font::{CTFont, CTFontRef};
use std::sync::{Arc, Mutex, OnceLock, Weak};

const MAX_FONT_BYTES: usize = 32 * 1024 * 1024;
const MAX_TABLES: usize = 256;

// Share large fallback faces across live text spans without making their bytes
// globally resident. Native identity includes face, size and variation settings;
// document primary fonts never enter this cache. Dead entries are pruned on use.
type SharedFaces = Vec<(CTFont, Weak<FontFace>)>;
fn shared_faces() -> &'static Mutex<SharedFaces> {
    static FACES: OnceLock<Mutex<SharedFaces>> = OnceLock::new();
    FACES.get_or_init(|| Mutex::new(Vec::new()))
}
fn shared(font: &CTFont) -> Option<Arc<FontFace>> {
    let mut faces = shared_faces().lock().unwrap_or_else(|e| e.into_inner());
    faces.retain(|(_, face)| face.strong_count() > 0);
    faces
        .iter()
        .find(|(native, _)| native == font)
        .and_then(|(_, face)| face.upgrade())
}
fn share(font: &CTFont, face: &Arc<FontFace>) {
    let mut faces = shared_faces().lock().unwrap_or_else(|e| e.into_inner());
    faces.retain(|(_, face)| face.strong_count() > 0);
    if faces.len() >= 128 {
        faces.remove(0);
    }
    faces.push((font.clone(), Arc::downgrade(face)));
}

#[link(name = "CoreText", kind = "framework")]
unsafe extern "C" {
    fn CTFontCopyVariation(font: CTFontRef) -> CFDictionaryRef;
}

pub(super) struct Sources {
    primary: CTFont,
    data: Arc<[u8]>,
    faces: Vec<(CTFont, FontSource)>,
    remaining: usize,
}
impl Sources {
    pub(super) fn new(primary: CTFont, bytes: &[u8]) -> Self {
        Self {
            primary,
            data: Arc::from(bytes),
            faces: Vec::new(),
            remaining: MAX_FONT_BYTES.saturating_sub(bytes.len()),
        }
    }
    pub(super) fn get(&mut self, font: &CTFont, glyphs: &[super::Glyph]) -> FontSource {
        if let Some((_, source)) = self.faces.iter().find(|(native, _)| native == font) {
            return source.clone();
        }
        // Color bitmap fonts can be hundreds of MB. Their exact ink is already
        // retained; do not copy a full bitmap face merely to export that ink.
        if glyphs.iter().any(|g| matches!(&*g.ink, Ink::Bitmap { .. })) {
            return FontSource::InkOnly("Run contains native bitmap glyphs".into());
        }
        let fallback = font != &self.primary;
        if fallback && let Some(face) = shared(font) {
            let size = match &face.data {
                FontData::Tables { size, .. } => *size,
                FontData::Bytes(bytes) => bytes.len(),
            };
            if size <= self.remaining {
                self.remaining -= size;
                let source = FontSource::Embedded(face);
                self.faces.push((font.clone(), source.clone()));
                return source;
            }
        }
        let source = match self.capture(font) {
            Ok(face) => {
                let face = Arc::new(face);
                if fallback {
                    share(font, &face);
                }
                FontSource::Embedded(face)
            }
            Err(reason) => FontSource::InkOnly(reason),
        };
        self.faces.push((font.clone(), source.clone()));
        source
    }
    fn capture(&mut self, font: &CTFont) -> Result<FontFace, String> {
        let matrix = font.get_matrix();
        if [matrix.a, matrix.b, matrix.c, matrix.d, matrix.tx, matrix.ty]
            != [1., 0., 0., 1., 0., 0.]
        {
            return Err("Native font transform requires original ink".into());
        }
        let variations = variations(font)?;
        let data = if font == &self.primary {
            FontData::Bytes(self.data.clone())
        } else {
            let tags = font
                .get_available_font_tables()
                .ok_or("Font tables unavailable")?;
            if tags.is_empty() || tags.len() as usize > MAX_TABLES {
                return Err("Font table count exceeds limits".into());
            }
            let mut total = 12 + tags.len() as usize * 16;
            let mut tables = Vec::new();
            for tag in tags.iter() {
                // A detached face cannot retain a signature over the old file.
                if *tag == u32::from_be_bytes(*b"DSIG") {
                    continue;
                }
                let data = font.get_font_table(*tag).ok_or("Font table unavailable")?;
                total += (data.len() as usize + 3) & !3;
                if total > self.remaining {
                    return Err("Retained font data exceeds 32 MB per text span".into());
                }
                tables.push((*tag, data.bytes().to_vec()));
            }
            let head = tables
                .iter()
                .find(|(tag, _)| *tag == u32::from_be_bytes(*b"head"))
                .ok_or("Missing font header")?;
            if head.1.len() < 54 {
                return Err("Truncated font header".into());
            }
            self.remaining -= total;
            FontData::Tables {
                tables,
                size: total,
                packed: Default::default(),
            }
        };
        Ok(FontFace {
            data,
            index: 0,
            variations,
        })
    }
}

fn variations(font: &CTFont) -> Result<Vec<([u8; 4], f32)>, String> {
    // SAFETY: live CTFont; Copy returns an owned dictionary whose keys and values
    // are CFNumbers per CTFontCopyVariation's API contract.
    let raw = unsafe { CTFontCopyVariation(font.as_concrete_TypeRef()) };
    if raw.is_null() {
        return Ok(Vec::new());
    }
    let values: CFDictionary<CFNumber, CFNumber> =
        unsafe { CFDictionary::wrap_under_create_rule(raw) };
    if values.len() > 64 {
        return Err("Too many font variation axes".into());
    }
    let (keys, coordinates) = values.get_keys_and_values();
    let mut out = Vec::new();
    for (key, value) in keys.into_iter().zip(coordinates) {
        // SAFETY: both numbers are owned by the live dictionary above.
        let key = unsafe { CFNumber::wrap_under_get_rule(key.cast()) };
        let value = unsafe { CFNumber::wrap_under_get_rule(value.cast()) };
        let tag = key
            .to_i64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or("Invalid font variation tag")?;
        let coordinate = value.to_f64().ok_or("Invalid font variation coordinate")? as f32;
        if !coordinate.is_finite() {
            return Err("Non-finite font variation coordinate".into());
        }
        out.push((tag.to_be_bytes(), coordinate));
    }
    out.sort_by_key(|(tag, _)| *tag);
    Ok(out)
}

fn checksum(bytes: &[u8]) -> u32 {
    bytes.chunks(4).fold(0u32, |sum, chunk| {
        let mut word = [0; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

/// Original SFNT serialization from detached tables. The head checksum is
/// calculated with adjustment zero; the final whole-font sum is 0xB1B0AFBA.
pub(super) fn pack(mut tables: Vec<(u32, Vec<u8>)>, limit: usize) -> Result<Vec<u8>, String> {
    if tables.is_empty() || tables.len() > MAX_TABLES {
        return Err("Invalid font table count".into());
    }
    tables.sort_by_key(|(tag, _)| *tag);
    if tables.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err("Duplicate font table".into());
    }
    let head_tag = u32::from_be_bytes(*b"head");
    let head = tables
        .iter_mut()
        .find(|(tag, _)| *tag == head_tag)
        .ok_or("Missing font header")?;
    if head.1.len() < 54 {
        return Err("Truncated font header".into());
    }
    head.1[8..12].fill(0);
    let total = tables
        .iter()
        .try_fold(12 + tables.len() * 16, |size, (_, data)| {
            size.checked_add(data.len().checked_add(3)? & !3)
        })
        .filter(|n| *n <= limit && *n <= u32::MAX as usize)
        .ok_or("Detached font exceeds size limit")?;
    let cff = tables
        .iter()
        .any(|(tag, _)| [*b"CFF ", *b"CFF2"].contains(&tag.to_be_bytes()));
    let mut out = vec![0; 12 + tables.len() * 16];
    out.reserve_exact(total - out.len());
    out[..4].copy_from_slice(if cff { b"OTTO" } else { &[0, 1, 0, 0] });
    let count = tables.len() as u16;
    let selector = count.ilog2() as u16;
    let search = (1u16 << selector) * 16;
    for (offset, value) in [
        (4, count),
        (6, search),
        (8, selector),
        (10, count * 16 - search),
    ] {
        out[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }
    let mut head_offset = 0;
    for (index, (tag, data)) in tables.into_iter().enumerate() {
        let offset = out.len();
        if tag == head_tag {
            head_offset = offset;
        }
        let entry = 12 + index * 16;
        for (i, value) in [tag, checksum(&data), offset as u32, data.len() as u32]
            .into_iter()
            .enumerate()
        {
            out[entry + i * 4..entry + i * 4 + 4].copy_from_slice(&value.to_be_bytes());
        }
        out.extend_from_slice(&data);
        out.resize((out.len() + 3) & !3, 0);
    }
    let adjustment = 0xB1B0AFBAu32.wrapping_sub(checksum(&out));
    out[head_offset + 8..head_offset + 12].copy_from_slice(&adjustment.to_be_bytes());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    unsafe extern "C" {
        static kCTFontVariationAxisIdentifierKey: core_foundation::string::CFStringRef;
        static kCTFontVariationAxisMaximumValueKey: core_foundation::string::CFStringRef;
    }
    #[test]
    fn detached_tables_have_valid_checksums_alignment_and_bounds() {
        let native = core_text::font::new_from_buffer(crate::fonts::REGULAR).unwrap();
        let tables: Vec<_> = native
            .get_available_font_tables()
            .unwrap()
            .iter()
            .map(|tag| (*tag, native.get_font_table(*tag).unwrap().bytes().to_vec()))
            .collect();
        let packed = pack(tables.clone(), MAX_FONT_BYTES).unwrap();
        assert_eq!(checksum(&packed), 0xB1B0AFBA);
        assert!(skrifa::FontRef::new(&packed).is_ok());
        let read = |offset| u32::from_be_bytes(packed[offset..offset + 4].try_into().unwrap());
        let count = u16::from_be_bytes(packed[4..6].try_into().unwrap()) as usize;
        let mut previous = 0;
        for entry in (12..12 + 16 * count).step_by(16) {
            let tag = read(entry);
            assert!(tag > previous);
            previous = tag;
            let offset = read(entry + 8) as usize;
            let len = read(entry + 12) as usize;
            assert_eq!(offset % 4, 0);
            let mut data = packed[offset..offset + len].to_vec();
            if tag == u32::from_be_bytes(*b"head") {
                data[8..12].fill(0);
            }
            assert_eq!(checksum(&data), read(entry + 4));
        }
        assert!(pack(tables.clone(), packed.len() - 1).is_err());
        let mut duplicate = tables.clone();
        duplicate.push(tables[0].clone());
        assert!(pack(duplicate, MAX_FONT_BYTES).is_err());
        assert!(pack(vec![], MAX_FONT_BYTES).is_err());
        assert!(pack(vec![(u32::from_be_bytes(*b"head"), vec![0; 12])], 1024).is_err());
    }

    #[test]
    fn captures_nondefault_native_variation_coordinates() {
        use core_foundation::string::CFString;
        use core_text::{font, font_descriptor};
        let native = font::new_ui_font_for_language(font::kCTFontSystemDetailFontType, 24., None);
        let axes = native.get_variation_axes().expect("System variable font");
        let mut coordinates = Vec::new();
        for axis in axes.iter() {
            // SAFETY: framework-owned static keys.
            let tag = axis
                .find(unsafe { kCTFontVariationAxisIdentifierKey })
                .unwrap()
                .downcast::<CFNumber>()
                .unwrap()
                .to_i64()
                .unwrap();
            let max = axis
                .find(unsafe { kCTFontVariationAxisMaximumValueKey })
                .unwrap()
                .downcast::<CFNumber>()
                .unwrap()
                .to_f64()
                .unwrap();
            coordinates.push((CFNumber::from(tag), CFNumber::from(max)));
        }
        let values = CFDictionary::from_CFType_pairs(&coordinates);
        let key =
            unsafe { CFString::wrap_under_get_rule(font_descriptor::kCTFontVariationAttribute) };
        let attrs = CFDictionary::from_CFType_pairs(&[(key, values)]);
        let desc = native
            .copy_descriptor()
            .create_copy_with_attributes(attrs.to_untyped())
            .unwrap();
        let varied = font::new_from_descriptor(&desc, 24.);
        let primary = font::new_from_buffer(crate::fonts::REGULAR).unwrap();
        let mut sources = Sources::new(primary, crate::fonts::REGULAR);
        let FontSource::Embedded(face) = sources.get(&varied, &[]) else {
            panic!("Expected portable variable face")
        };
        let FontData::Tables { packed, .. } = &face.data else {
            panic!("Expected detached tables")
        };
        assert!(
            packed.get().is_none(),
            "Shaping must not serialize the face"
        );
        let reserved = face.storage_bytes();
        assert!(!face.variations.is_empty());
        for (tag, value) in &face.variations {
            assert!(coordinates.iter().any(|(k, v)| k.to_i64()
                == Some(i64::from(u32::from_be_bytes(*tag)))
                && v.to_f64().unwrap() as f32 == *value));
        }
        assert_eq!(face.index, 0);
        assert_eq!(checksum(face.data().unwrap()), 0xB1B0AFBA);
        assert!(packed.get().is_some());
        assert_eq!(
            face.storage_bytes(),
            reserved,
            "Cache accounting must reserve lazy bytes"
        );
        assert!(reserved >= 2 * face.data().unwrap().len());
        let FontSource::Embedded(again) = sources.get(&varied, &[]) else {
            panic!()
        };
        assert!(Arc::ptr_eq(&face, &again));
        sources.remaining = 0;
        let other = font::new_from_name("Helvetica", 24.).unwrap();
        assert!(matches!(sources.get(&other, &[]), FontSource::InkOnly(_)));
    }
}
