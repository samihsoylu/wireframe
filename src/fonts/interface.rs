// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Host-owned interface fonts. These bytes never enter project documents.
use std::sync::{Arc, OnceLock};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InterfaceRole {
    #[default]
    Regular,
    Medium,
    Bold,
}

impl InterfaceRole {
    pub fn family(self) -> eframe::egui::FontFamily {
        match self {
            Self::Regular => eframe::egui::FontFamily::Proportional,
            Self::Medium => eframe::egui::FontFamily::Name("interface-medium".into()),
            Self::Bold => eframe::egui::FontFamily::Name("interface-bold".into()),
        }
    }
}

pub(crate) struct InterfaceFont {
    pub bytes: Arc<Vec<u8>>,
    pub index: u32,
    pub axes: Vec<([u8; 4], f32)>,
    #[cfg(target_os = "macos")]
    source: std::path::PathBuf,
}

pub(crate) fn system_font(role: InterfaceRole) -> Option<&'static InterfaceFont> {
    static REGULAR: OnceLock<Option<InterfaceFont>> = OnceLock::new();
    static MEDIUM: OnceLock<Option<InterfaceFont>> = OnceLock::new();
    static BOLD: OnceLock<Option<InterfaceFont>> = OnceLock::new();
    match role {
        InterfaceRole::Regular => REGULAR.get_or_init(|| load(role)),
        InterfaceRole::Medium => MEDIUM.get_or_init(|| load(role)),
        InterfaceRole::Bold => BOLD.get_or_init(|| load(role)),
    }
    .as_ref()
}

#[cfg(not(target_os = "macos"))]
fn load(_role: InterfaceRole) -> Option<InterfaceFont> {
    None
}

#[cfg(target_os = "macos")]
fn load(role: InterfaceRole) -> Option<InterfaceFont> {
    use core_foundation::{
        base::TCFType,
        dictionary::{CFDictionary, CFDictionaryRef},
        number::CFNumber,
    };
    use core_text::font::{CTFont, CTFontRef, kCTFontSystemFontType};
    unsafe extern "C" {
        fn CTFontCreateUIFontForLanguage(
            kind: u32,
            size: f64,
            language: *const std::ffi::c_void,
        ) -> CTFontRef;
        fn CTFontCopyVariation(font: CTFontRef) -> CFDictionaryRef;
    }
    // Request the host's font rather than assuming a file name or family.
    let font = match role {
        InterfaceRole::Regular => {
            let raw = unsafe {
                CTFontCreateUIFontForLanguage(kCTFontSystemFontType, 13., std::ptr::null())
            };
            if raw.is_null() {
                return None;
            }
            unsafe { CTFont::wrap_under_create_rule(raw) }
        }
        InterfaceRole::Medium | InterfaceRole::Bold => {
            use objc2::{rc::Retained, runtime::AnyObject};
            use objc2_app_kit::{NSFont, NSFontWeightBold, NSFontWeightMedium};
            let weight = unsafe {
                if role == InterfaceRole::Bold {
                    NSFontWeightBold
                } else {
                    NSFontWeightMedium
                }
            };
            let native = NSFont::systemFontOfSize_weight(13., weight);
            // NSFont and CTFont are toll-free bridged. Retain through the get rule
            // so this CTFont remains valid after the Objective-C wrapper drops.
            let raw = Retained::as_ptr(&native).cast::<AnyObject>().cast();
            unsafe { CTFont::wrap_under_get_rule(raw) }
        }
    };
    let path = font.url()?.to_path()?;
    let metadata = std::fs::metadata(&path).ok()?;
    if metadata.len() > 32 * 1024 * 1024 {
        return None;
    }
    let bytes = if role != InterfaceRole::Regular {
        system_font(InterfaceRole::Regular)
            .filter(|regular| regular.source == path)
            .map(|regular| regular.bytes.clone())
    } else {
        None
    }
    .or_else(|| std::fs::read(&path).ok().map(Arc::new))?;
    let mut db = cosmic_text::fontdb::Database::new();
    let ids = db.load_font_source(cosmic_text::fontdb::Source::Binary(bytes.clone()));
    let only_face = (ids.len() == 1)
        .then(|| db.face(ids[0]).map(|face| face.index))
        .flatten();
    let index = ids
        .into_iter()
        .find_map(|id| {
            let face = db.face(id)?;
            (face.post_script_name == font.postscript_name()).then_some(face.index)
        })
        .or(only_face)?;
    let mut axes = Vec::new();
    let raw = unsafe { CTFontCopyVariation(font.as_concrete_TypeRef()) };
    if !raw.is_null() {
        let dict: CFDictionary<CFNumber, CFNumber> =
            unsafe { CFDictionary::wrap_under_create_rule(raw) };
        let (keys, values) = dict.get_keys_and_values();
        for (key, value) in keys.into_iter().zip(values) {
            let key = unsafe { CFNumber::wrap_under_get_rule(key.cast()) };
            let value = unsafe { CFNumber::wrap_under_get_rule(value.cast()) };
            let tag = u32::try_from(key.to_i64()?).ok()?.to_be_bytes();
            let value = value.to_f64()? as f32;
            if !value.is_finite() {
                return None;
            }
            axes.push((tag, value));
        }
    }
    Some(InterfaceFont {
        bytes,
        index,
        axes,
        source: path,
    })
}

impl InterfaceFont {
    pub fn axes_at(&self, size: f32) -> Vec<([u8; 4], f32)> {
        let mut axes = self.axes.clone();
        axes.retain(|(tag, _)| tag != b"opsz");
        axes.push((*b"opsz", size));
        axes
    }
}

/// Host text-system line metrics, in logical points. Canvas metrics are separate.
pub(crate) fn system_line_metrics(size: f32, role: InterfaceRole) -> Option<(f32, f32)> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{
            NSFont, NSFontWeightBold, NSFontWeightMedium, NSFontWeightRegular, NSLayoutManager,
        };
        let weight = unsafe {
            match role {
                InterfaceRole::Regular => NSFontWeightRegular,
                InterfaceRole::Medium => NSFontWeightMedium,
                InterfaceRole::Bold => NSFontWeightBold,
            }
        };
        let font = NSFont::systemFontOfSize_weight(f64::from(size), weight);
        let layout = NSLayoutManager::new();
        let height = layout.defaultLineHeightForFont(&font) as f32;
        let baseline = layout.defaultBaselineOffsetForFont(&font) as f32;
        (height.is_finite()
            && baseline.is_finite()
            && height > 0.
            && baseline >= 0.
            && baseline <= height)
            .then_some((height, baseline))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (size, role);
        None
    }
}
