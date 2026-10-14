// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

pub const MAX_FACE_BYTES: usize = 4 * 1024 * 1024;

/// Immutable, validated project resource. Cloned documents share it through Arc.
/// Face order is regular, bold, italic, bold italic; no system font lookup is used.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(try_from = "EncodedFont")]
pub struct ProjectFont {
    name: String,
    faces: [Vec<u8>; 4],
    notices: String,
}

#[derive(Serialize, Deserialize)]
struct EncodedFont {
    name: String,
    faces: [String; 4],
    #[serde(default)]
    notices: String,
}

impl ProjectFont {
    pub fn new(name: String, faces: [Vec<u8>; 4], notices: String) -> Result<Self, String> {
        if name.trim().is_empty() || name.len() > 200 || name.chars().any(char::is_control) {
            return Err("Enter a font name of 1–200 bytes without control characters".into());
        }
        if notices.len() > 1_000_000 {
            return Err("Font notices exceed 1 MB".into());
        }
        for face in &faces {
            Self::inspect_face(face)?;
        }
        Ok(Self {
            name,
            faces,
            notices,
        })
    }
    /// Validate both rendering engines before a font can enter a document.
    pub fn inspect_face(bytes: &[u8]) -> Result<String, String> {
        if bytes.is_empty() || bytes.len() > MAX_FACE_BYTES {
            return Err("Each font file must contain 1 byte to 4 MB".into());
        }
        // A file maps to exactly one explicit face. Collections need an index chooser.
        if bytes.starts_with(b"ttcf") {
            return Err("Choose a single TTF or OTF face, not a font collection".into());
        }
        use skrifa::MetadataProvider;
        let font = skrifa::FontRef::new(bytes).map_err(|_| "Cannot read this font file")?;
        let metrics = font.metrics(
            skrifa::instance::Size::unscaled(),
            skrifa::instance::LocationRef::default(),
        );
        if metrics.units_per_em == 0 || font.outline_glyphs().iter().next().is_none() {
            return Err("This font has no usable outline glyphs".into());
        }
        let mut db = resvg::usvg::fontdb::Database::new();
        db.load_font_data(bytes.to_vec());
        let face = db.faces().next().ok_or("Cannot use this font in exports")?;
        Ok(face
            .families
            .first()
            .ok_or("Font family name is missing")?
            .0
            .clone())
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn notices(&self) -> &str {
        &self.notices
    }
    pub fn faces(&self) -> [&[u8]; 4] {
        self.faces.each_ref().map(Vec::as_slice)
    }
}

impl TryFrom<EncodedFont> for ProjectFont {
    type Error = String;
    fn try_from(raw: EncodedFont) -> Result<Self, String> {
        let mut faces = [const { Vec::new() }; 4];
        for (target, encoded) in faces.iter_mut().zip(raw.faces) {
            if encoded.len() > MAX_FACE_BYTES.div_ceil(3) * 4 {
                return Err("Embedded font face exceeds 4 MB".into());
            }
            *target = STANDARD
                .decode(encoded)
                .map_err(|_| "Invalid embedded font encoding")?;
        }
        Self::new(raw.name, faces, raw.notices)
    }
}
impl Serialize for ProjectFont {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        EncodedFont {
            name: self.name.clone(),
            faces: self.faces.each_ref().map(|f| STANDARD.encode(f)),
            notices: self.notices.clone(),
        }
        .serialize(serializer)
    }
}
