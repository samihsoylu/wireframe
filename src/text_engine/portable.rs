// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Positioned shaping and detached ink for Windows and Linux.
use super::*;
use cosmic_text::{
    Attrs, Buffer, CacheKey, CacheKeyFlags, Family, FontSystem, Metrics, Shaping, SwashCache,
    SwashContent, Wrap, fontdb,
};
use skrifa::{MetadataProvider, instance::Size, outline::DrawSettings};
use std::collections::{HashMap, VecDeque};

// Document fonts remain selectable by their explicit family, but must not
// participate in the final system fallback sweep for a different document.
const PRIMARY_MARKER: &str = "Wireframe document primary";
struct DocumentFallback {
    forbidden: Vec<&'static str>,
}
impl DocumentFallback {
    fn new() -> Self {
        use cosmic_text::Fallback;
        let mut forbidden = cosmic_text::PlatformFallback.forbidden_fallback().to_vec();
        forbidden.push(PRIMARY_MARKER);
        Self { forbidden }
    }
}
impl cosmic_text::Fallback for DocumentFallback {
    fn common_fallback(&self) -> &[&'static str] {
        cosmic_text::PlatformFallback.common_fallback()
    }
    fn forbidden_fallback(&self) -> &[&'static str] {
        &self.forbidden
    }
    fn script_fallback(&self, script: unicode_script::Script, locale: &str) -> &[&'static str] {
        cosmic_text::PlatformFallback.script_fallback(script, locale)
    }
}
fn font_system(locale: String, db: fontdb::Database) -> FontSystem {
    FontSystem::new_with_locale_and_db_and_fallback(locale, db, DocumentFallback::new())
}
mod emoji;
mod ink;
use ink::Pen;
const MAX_PRIMARIES: usize = 8;
struct Primary {
    bytes: Arc<[u8]>,
    id: fontdb::ID,
    family: String,
    weight: fontdb::Weight,
    style: fontdb::Style,
}
struct Engine {
    #[cfg(test)]
    raster_calls: usize,
    fonts: FontSystem,
    raster: SwashCache,
    primaries: VecDeque<Primary>,
    family: String,
    next_family: u64,
    primary_id: Option<fontdb::ID>,
    weight: fontdb::Weight,
    style: fontdb::Style,
    shared: HashMap<(fontdb::ID, u16), std::sync::Weak<FontFace>>,
}
impl Engine {
    fn new() -> Self {
        let (locale, db) = FontSystem::new().into_locale_and_db();
        Self {
            #[cfg(test)]
            raster_calls: 0,
            fonts: font_system(locale, db),
            raster: SwashCache::new(),
            primaries: VecDeque::new(),
            family: String::new(),
            next_family: 0,
            primary_id: None,
            weight: fontdb::Weight::NORMAL,
            style: fontdb::Style::Normal,
            shared: HashMap::new(),
        }
    }
    fn primary(&mut self, bytes: &[u8]) -> Result<(), String> {
        if let Some(index) = self.primaries.iter().position(|p| &*p.bytes == bytes) {
            let primary = self.primaries.remove(index).unwrap();
            self.primary_id = Some(primary.id);
            self.family = primary.family.clone();
            self.weight = primary.weight;
            self.style = primary.style;
            self.primaries.push_back(primary);
            return Ok(());
        }
        let mut db = fontdb::Database::new();
        db.load_font_data(bytes.to_vec());
        let mut face = db.faces().next().ok_or("Invalid primary font")?.clone();
        if db.faces().count() != 1 {
            return Err("Choose one standalone primary font face".into());
        }
        if self.primaries.len() == MAX_PRIMARIES {
            let old = self.primaries.pop_front().unwrap();
            self.fonts.db_mut().remove_face(old.id);
            // Removing a DB face does not clear cosmic-text's resolved-font or
            // shaping caches. Rebuild from the surviving DB to release them;
            // this does not rescan the filesystem or invalidate surviving IDs.
            self.fonts = font_system(self.fonts.locale().to_owned(), self.fonts.db().clone());
            self.shared.retain(|(id, _), _| *id != old.id);
            self.raster = SwashCache::new();
        }
        self.family = format!("Portable Primary {}", self.next_family);
        self.next_family += 1;
        face.families = vec![
            (self.family.clone(), fontdb::Language::English_UnitedStates),
            (
                PRIMARY_MARKER.into(),
                fontdb::Language::English_UnitedStates,
            ),
        ];
        self.weight = face.weight;
        self.style = face.style;
        let id = self.fonts.db_mut().push_face_info(face);
        self.primary_id = Some(id);
        self.primaries.push_back(Primary {
            bytes: Arc::from(bytes),
            id,
            family: self.family.clone(),
            weight: self.weight,
            style: self.style,
        });
        Ok(())
    }
    fn shape(&mut self, bytes: &[u8], text: &str, size: f64) -> Result<ShapedText, String> {
        self.primary(bytes)?;
        if text.contains(['\n', '\r']) {
            return Err("Text shaping requires a single format line".into());
        }
        let mut buffer = Buffer::new(
            &mut self.fonts,
            Metrics::new(size as f32, size as f32 * 1.2),
        );
        buffer.set_wrap(Wrap::None);
        buffer.set_size(None, None);
        let base = Attrs::new()
            .family(Family::Name(&self.family))
            .weight(self.weight)
            .style(self.style);
        let spans = emoji::spans(text, base.clone(), emoji::family(self.fonts.db()));
        buffer.set_rich_text(spans, &base, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut self.fonts, false);
        self.shared.retain(|_, face| face.strong_count() > 0);
        let mut out = ShapedText {
            source: text.into(),
            width: 0.,
            runs: vec![],
        };
        let mut faces: HashMap<(fontdb::ID, u16), FontSource> = HashMap::new();
        let mut remaining = 32 * 1024 * 1024usize;
        let mut ink_bytes = 0usize;
        for row in buffer.layout_runs() {
            out.width = f64::from(row.line_w);
            let ordered: Vec<_> = if row.rtl {
                row.glyphs.iter().rev().collect()
            } else {
                row.glyphs.iter().collect()
            };
            for g in ordered {
                if text.get(g.start..g.end).is_none() || g.start >= g.end {
                    return Err("Invalid UTF-8 cluster".into());
                }
                if g.glyph_id == 0 {
                    return Err(format!(
                        "No installed font covers {:?}",
                        &text[g.start..g.end]
                    ));
                }
                let synthetic = g.cache_key_flags.contains(CacheKeyFlags::FAKE_ITALIC);
                let info = self
                    .fonts
                    .db()
                    .face(g.font_id)
                    .ok_or("Resolved face vanished")?
                    .clone();
                let key = (g.font_id, g.font_weight.0);
                let resolved = self
                    .fonts
                    .get_font(g.font_id, g.font_weight)
                    .ok_or("Resolved font data unavailable")?;
                let font = skrifa::FontRef::from_index(resolved.data(), info.index)
                    .map_err(|e| e.to_string())?;
                let loc = font
                    .axes()
                    .location([(skrifa::Tag::new(b"wght"), g.font_weight.0 as f32)]);
                let (cache, _, _) = CacheKey::new(
                    g.font_id,
                    g.glyph_id,
                    g.font_size * 2.,
                    (0., 0.),
                    g.font_weight,
                    g.cache_key_flags | CacheKeyFlags::DISABLE_HINTING,
                );
                let shear = if synthetic {
                    14_f32.to_radians().tan()
                } else {
                    0.
                };
                let image = if ink::preflight(
                    &resolved,
                    &font,
                    g.glyph_id,
                    g.font_size * 2.,
                    (&loc).into(),
                    shear,
                )? {
                    #[cfg(test)]
                    {
                        self.raster_calls += 1;
                    }
                    self.raster.get_image_uncached(&mut self.fonts, cache)
                } else {
                    None
                };
                let ink = if let Some(img) = image
                    .as_ref()
                    .filter(|i| matches!(i.content, SwashContent::Color))
                {
                    if img.data.len() > 16 * 1024 * 1024 {
                        return Err("Color glyph exceeds image budget".into());
                    }
                    let mut png = vec![];
                    {
                        let mut enc =
                            png::Encoder::new(&mut png, img.placement.width, img.placement.height);
                        enc.set_color(png::ColorType::Rgba);
                        enc.set_depth(png::BitDepth::Eight);
                        let mut writer = enc.write_header().map_err(|e| e.to_string())?;
                        writer
                            .write_image_data(&img.data)
                            .map_err(|e| e.to_string())?;
                    }
                    Ink::Bitmap {
                        png,
                        bounds: [
                            img.placement.left as f64 / 2.,
                            -(img.placement.top as f64) / 2.,
                            img.placement.width as f64 / 2.,
                            img.placement.height as f64 / 2.,
                        ],
                    }
                } else if let Some(outline) = font
                    .outline_glyphs()
                    .get(skrifa::GlyphId::new(u32::from(g.glyph_id)))
                {
                    let mut pen =
                        Pen::new(shear, (16 * 1024 * 1024usize).saturating_sub(ink_bytes));
                    outline
                        .draw(
                            DrawSettings::unhinted(Size::new(g.font_size), &loc),
                            &mut pen,
                        )
                        .map_err(|e| e.to_string())?;
                    pen.check()?;
                    if pen.path.is_empty() {
                        Ink::Empty
                    } else {
                        Ink::Outline(pen.path)
                    }
                } else if text[g.start..g.end].chars().all(char::is_whitespace)
                    || (g.w == 0.
                        && font
                            .charmap()
                            .map(' ')
                            .is_some_and(|id| id.to_u32() == u32::from(g.glyph_id)))
                {
                    // Shapers can retain an unsupported variation selector as
                    // the font's space glyph with zero advance. Its cluster may
                    // include the visible base, so inspecting source text alone
                    // cannot identify this intentionally invisible glyph.
                    Ink::Empty
                } else {
                    return Err(format!(
                        "No portable ink for {} glyph {}",
                        info.post_script_name, g.glyph_id
                    ));
                };
                ink_bytes += match &ink {
                    Ink::Outline(p) => p.len(),
                    Ink::Bitmap { png, .. } => png.len(),
                    Ink::Empty => 0,
                };
                if ink_bytes > 16 * 1024 * 1024 {
                    return Err("Shaped ink exceeds budget".into());
                }
                let face = if matches!(&ink, Ink::Bitmap { .. }) {
                    FontSource::InkOnly(
                        "Color glyph ink retained without copying bitmap font".into(),
                    )
                } else if synthetic {
                    FontSource::InkOnly("Synthetic italic requires transformed ink".into())
                } else if let Some(f) = faces.get(&key) {
                    f.clone()
                } else if let Some(f) = self.shared.get(&key).and_then(std::sync::Weak::upgrade) {
                    let size = match &f.data {
                        FontData::Bytes(b) => b.len(),
                        FontData::Tables { size, .. } => *size,
                    };
                    if size <= remaining {
                        remaining -= size;
                        let source = FontSource::Embedded(f);
                        faces.insert(key, source.clone());
                        source
                    } else {
                        FontSource::InkOnly(
                            "Retained font data exceeds 32 MB per span; exact ink retained".into(),
                        )
                    }
                } else {
                    let data = if Some(g.font_id) == self.primary_id {
                        remaining = remaining.saturating_sub(bytes.len());
                        Ok(FontData::Bytes(Arc::from(bytes)))
                    } else {
                        capture(&font, &mut remaining)
                    };
                    let source = match data {
                        Ok(data) => FontSource::Embedded(Arc::new(FontFace {
                            data,
                            index: 0,
                            variations: vec![(*b"wght", g.font_weight.0 as f32)],
                        })),
                        Err(reason) => FontSource::InkOnly(reason),
                    };
                    if let FontSource::Embedded(face) = &source {
                        if self.shared.len() >= 128
                            && let Some(key) = self.shared.keys().next().copied()
                        {
                            self.shared.remove(&key);
                        }
                        self.shared.insert(key, Arc::downgrade(face));
                    }
                    faces.insert(key, source.clone());
                    source
                };
                let glyph = Glyph {
                    id: g.glyph_id,
                    source: g.start..g.end,
                    position: [
                        f64::from(g.x + g.font_size * g.x_offset),
                        f64::from(g.y - g.font_size * g.y_offset),
                    ],
                    advance: [f64::from(g.w), 0.],
                    ink: Arc::new(ink),
                };
                let rtl = g.level.is_rtl();
                if let Some(last) = out.runs.last_mut().filter(|r| {
                    r.right_to_left == rtl
                        && match (&r.font_source, &face) {
                            (FontSource::Embedded(a), FontSource::Embedded(b)) => Arc::ptr_eq(a, b),
                            _ => false,
                        }
                }) {
                    last.source.start = last.source.start.min(g.start);
                    last.source.end = last.source.end.max(g.end);
                    last.glyphs.push(glyph)
                } else {
                    out.runs.push(Run {
                        source: g.start..g.end,
                        font_name: info.post_script_name.clone(),
                        font_source: face,
                        font_size: f64::from(g.font_size),
                        right_to_left: rtl,
                        glyphs: vec![glyph],
                    });
                }
            }
        }
        Ok(out)
    }
}
#[cfg(not(target_os = "macos"))]
pub(super) fn shape(font: &[u8], text: &str, size: f64) -> Result<ShapedText, String> {
    use std::sync::{Mutex, OnceLock};
    static ENGINE: OnceLock<Mutex<Engine>> = OnceLock::new();
    ENGINE
        .get_or_init(|| Mutex::new(Engine::new()))
        .lock()
        .map_err(|_| "Text font lock poisoned".to_string())?
        .shape(font, text, size)
}

fn capture(font: &skrifa::FontRef<'_>, remaining: &mut usize) -> Result<FontData, String> {
    let records = font.table_directory.table_records();
    if records.is_empty() || records.len() > 256 {
        return Err("Invalid font table count".into());
    }
    let mut size = 12 + 16 * records.len();
    for record in records {
        if record.tag() != skrifa::Tag::new(b"DSIG") {
            size = size
                .checked_add((record.length() as usize + 3) & !3)
                .ok_or("Font size overflow")?;
        }
    }
    if size > *remaining {
        return Err("Retained font data exceeds 32 MB per span; exact ink retained".into());
    }
    let mut tables = vec![];
    for record in records {
        let tag = record.tag();
        if tag == skrifa::Tag::new(b"DSIG") {
            continue;
        }
        let data = font.table_data(tag).ok_or("Missing font table")?;
        tables.push((
            u32::from_be_bytes(tag.to_be_bytes()),
            data.as_bytes().to_vec(),
        ));
    }
    *remaining -= size;
    Ok(FontData::tables(tables, size))
}

#[cfg(test)]
mod resource_tests {
    use super::*;
    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/text")
                .join(format!("{name}.ttf")),
        )
        .unwrap()
    }
    #[test]
    fn ordinary_outlines_do_not_invoke_the_image_rasterizer() {
        let mut engine = Engine::new();
        let font = crate::fonts::default_project_font();
        let shaped = engine
            .shape(font.faces()[0], "Planning notebook — Café déjà vu", 17.)
            .unwrap();
        assert!(!shaped.runs.is_empty());
        assert_eq!(engine.raster_calls, 0);
        assert!(
            shaped
                .runs
                .iter()
                .flat_map(|r| &r.glyphs)
                .all(|g| !matches!(&*g.ink, Ink::Bitmap { .. }))
        );
    }
    #[test]
    fn valid_bitmap_and_color_layers_retain_colored_pixels() {
        let mut engine = Engine::new();
        for name in [
            "valid-sbix",
            "valid-colr",
            "valid-fragmented",
            "valid-interlaced",
        ] {
            let before = engine.raster_calls;
            let shaped = engine.shape(&fixture(name), "A", 17.).unwrap();
            assert_eq!(engine.raster_calls, before + 1);
            let images: Vec<_> = shaped
                .runs
                .iter()
                .flat_map(|r| &r.glyphs)
                .filter_map(|g| {
                    if let Ink::Bitmap { png, .. } = &*g.ink {
                        Some(png)
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(images.len(), 1);
            let rgba = image::load_from_memory(images[0]).unwrap().to_rgba8();
            assert!(
                rgba.pixels()
                    .any(|p| p[3] > 0 && (p[0] != p[1] || p[1] != p[2]))
            );
        }
    }
    #[test]
    fn oversized_bitmap_headers_are_rejected_before_decode() {
        let mut engine = Engine::new();
        for name in ["decoded-oversize", "scaled-oversize", "overflow-header"] {
            let bytes = fixture(name);
            crate::fonts::ProjectFont::inspect_face(&bytes).unwrap();
            let error = engine.shape(&bytes, "A", 17.).unwrap_err();
            assert!(error.contains("raster budget"), "{name}: {error}");
            assert_eq!(engine.raster_calls, 0);
        }
    }
    #[test]
    fn overlong_png_stream_is_rejected_before_the_image_rasterizer() {
        let mut engine = Engine::new();
        let bytes = fixture("overlong-png");
        crate::fonts::ProjectFont::inspect_face(&bytes).unwrap();
        let error = engine.shape(&bytes, "A", 17.).unwrap_err();
        assert!(error.contains("decoded budget"), "{error}");
        assert_eq!(engine.raster_calls, 0);
    }
    #[test]
    fn oversized_color_outlines_and_layer_lists_are_rejected_before_rasterization() {
        let mut engine = Engine::new();
        for name in ["colr-oversize", "colr-layers"] {
            let bytes = fixture(name);
            crate::fonts::ProjectFont::inspect_face(&bytes).unwrap();
            let error = engine.shape(&bytes, "A", 17.).unwrap_err();
            assert!(error.contains("budget"), "{name}: {error}");
            assert_eq!(engine.raster_calls, 0);
        }
    }

    #[test]
    fn portable_variable_defaults_match_static_geometry() {
        let mut engine = Engine::new();
        for name in ["default", "heavy", "condensed", "display"] {
            let fixture = |kind| {
                std::fs::read(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join(format!("tests/fixtures/text/variable/{name}-{kind}.ttf")),
                )
                .unwrap()
            };
            let variable = fixture("variable");
            let fixed = fixture("static");
            for size in [8., 17., 64., 512.] {
                for source in ["AB VA", "A V B A"] {
                    let a = engine.shape(&variable, source, size).unwrap();
                    let b = engine.shape(&fixed, source, size).unwrap();
                    assert_eq!(a.width, b.width, "{name} {size}");
                    let glyphs = |s: &ShapedText| {
                        s.runs
                            .iter()
                            .flat_map(|r| &r.glyphs)
                            .map(|g| {
                                let path = match &*g.ink {
                                    Ink::Outline(p) => p.clone(),
                                    Ink::Empty => String::new(),
                                    _ => panic!("Unexpected bitmap"),
                                };
                                (g.source.clone(), g.position, g.advance, path)
                            })
                            .collect::<Vec<_>>()
                    };
                    assert_eq!(glyphs(&a), glyphs(&b), "{name} {size} {source}");
                }
            }
        }
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    #[test]
    fn repeated_primary_style_switches_reuse_resolved_fonts() {
        let project = crate::fonts::default_project_font();
        let mut engine = Engine::new();
        let mut seen = std::collections::HashSet::new();
        let mut retained = Vec::new();
        for _ in 0..16 {
            for bytes in project.faces() {
                engine.primary(bytes).unwrap();
                let id = engine.primary_id.unwrap();
                seen.insert(id);
                let font = engine.fonts.get_font(id, engine.weight).unwrap();
                retained.push(Arc::downgrade(&font));
            }
        }
        println!(
            "primary_ids={}, retained_resolved_handles={}",
            seen.len(),
            retained.iter().filter(|f| f.strong_count() > 0).count()
        );
        assert_eq!(
            seen.len(),
            4,
            "Four document styles must reuse four registered primary faces"
        );
    }
    #[test]
    fn primary_eviction_releases_resolved_cache_and_keeps_live_output() {
        let project = crate::fonts::default_project_font();
        let mut engine = Engine::new();
        let held = engine.shape(project.faces()[0], "Planning", 17.).unwrap();
        let svg = held.svg([0, 0, 0, 255]);
        let first_id = engine.primary_id.unwrap();
        let first = Arc::downgrade(&engine.fonts.get_font(first_id, engine.weight).unwrap());
        let db_faces = engine.fonts.db().faces().count();
        for i in 0..MAX_PRIMARIES {
            // Trailing bytes create distinct document identities without changing
            // valid glyph tables. The font boundary permits these small inputs.
            let mut bytes = project.faces()[0].to_vec();
            bytes.extend_from_slice(&(i as u64).to_le_bytes());
            engine.primary(&bytes).unwrap();
            engine
                .fonts
                .get_font(engine.primary_id.unwrap(), engine.weight)
                .unwrap();
        }
        assert!(
            first.upgrade().is_none(),
            "Evicted resolved font must be released"
        );
        assert!(engine.fonts.db().face(first_id).is_none());
        assert_eq!(engine.primaries.len(), MAX_PRIMARIES);
        assert_eq!(
            engine.fonts.db().faces().count(),
            db_faces - 1 + MAX_PRIMARIES
        );
        assert_eq!(held.svg([0, 0, 0, 255]), svg);
        for run in held.runs {
            if let FontSource::Embedded(face) = run.font_source {
                assert!(face.data().is_ok());
            }
        }
    }
    #[test]
    fn previous_document_primaries_never_become_system_fallbacks() {
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/text/private-primary.ttf"),
        )
        .unwrap();
        let mut engine = Engine::new();
        assert!(
            engine
                .shape(crate::fonts::REGULAR, "\u{10ffff}", 17.)
                .is_err()
        );
        assert!(engine.shape(&bytes, "\u{10ffff}", 17.).is_ok());
        assert!(
            engine
                .shape(crate::fonts::REGULAR, "\u{10ffff}", 17.)
                .is_err(),
            "Earlier primary must not leak into fallback policy"
        );
    }
    #[test]
    fn primary_lru_promotes_hits_and_rejects_invalid_replacement_without_eviction() {
        let mut engine = Engine::new();
        let base = crate::fonts::REGULAR;
        let mut variants = Vec::new();
        let mut ids = Vec::new();
        for i in 0..MAX_PRIMARIES {
            let mut bytes = base.to_vec();
            bytes.extend_from_slice(&(i as u64).to_le_bytes());
            engine.primary(&bytes).unwrap();
            ids.push(engine.primary_id.unwrap());
            variants.push(bytes);
        }
        engine.primary(&variants[0]).unwrap();
        let active = engine.primary_id;
        assert!(engine.primary(b"invalid").is_err());
        assert_eq!(engine.primary_id, active);
        assert_eq!(engine.primaries.len(), MAX_PRIMARIES);
        engine.primary(base).unwrap();
        assert!(engine.fonts.db().face(ids[0]).is_some());
        assert!(engine.fonts.db().face(ids[1]).is_none());
        for i in MAX_PRIMARIES..64 {
            let mut bytes = base.to_vec();
            bytes.extend_from_slice(&(i as u64).to_le_bytes());
            engine.primary(&bytes).unwrap();
            assert_eq!(engine.primaries.len(), MAX_PRIMARIES);
        }
    }
}
