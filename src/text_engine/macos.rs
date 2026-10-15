// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::{Glyph, Ink, MAX_INK_BYTES, MAX_TEXT_BYTES, Run, ShapedText};
use core_foundation::{
    attributed_string::CFMutableAttributedString,
    base::{CFRange, TCFType},
    string::CFString,
};
use core_graphics::{
    geometry::{CGAffineTransform, CGSize},
    path::{CGPath, CGPathElementType},
};
use core_text::{
    font::{self, CTFont},
    line::CTLine,
    run::CTRunRef,
    string_attributes::kCTFontAttributeName,
};
use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    fmt::Write as _,
    sync::Arc,
};

#[link(name = "CoreText", kind = "framework")]
unsafe extern "C" {
    fn CTRunGetStringRange(run: CTRunRef) -> CFRange;
    fn CTRunGetStatus(run: CTRunRef) -> u32;
    fn CTRunGetAdvances(run: CTRunRef, range: CFRange, out: *mut CGSize);
}

fn utf16_map(text: &str) -> Vec<Option<usize>> {
    let mut result = Vec::with_capacity(text.len() + 1);
    for (byte, ch) in text.char_indices() {
        result.push(Some(byte));
        if ch.len_utf16() == 2 {
            result.push(None);
        }
    }
    result.push(Some(text.len()));
    result
}
fn byte_at(map: &[Option<usize>], index: isize) -> Result<usize, String> {
    usize::try_from(index)
        .ok()
        .and_then(|i| map.get(i))
        .copied()
        .flatten()
        .ok_or_else(|| "Invalid native UTF-16 boundary".into())
}
fn outline(path: &CGPath) -> Result<Ink, String> {
    let output = RefCell::new(String::new());
    let invalid = std::cell::Cell::new(false);
    path.apply(&|element| {
        let points = element.points();
        let mut out = output.borrow_mut();
        if invalid.get() {
            return;
        }
        if points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) || out.len() > MAX_INK_BYTES
        {
            invalid.set(true);
            return;
        }
        match element.element_type {
            CGPathElementType::MoveToPoint => {
                let _ = write!(out, "M{} {}", points[0].x, -points[0].y);
            }
            CGPathElementType::AddLineToPoint => {
                let _ = write!(out, "L{} {}", points[0].x, -points[0].y);
            }
            CGPathElementType::AddQuadCurveToPoint => {
                let _ = write!(
                    out,
                    "Q{} {} {} {}",
                    points[0].x, -points[0].y, points[1].x, -points[1].y
                );
            }
            CGPathElementType::AddCurveToPoint => {
                let _ = write!(
                    out,
                    "C{} {} {} {} {} {}",
                    points[0].x, -points[0].y, points[1].x, -points[1].y, points[2].x, -points[2].y
                );
            }
            CGPathElementType::CloseSubpath => out.push('Z'),
        }
    });
    if invalid.get() || output.borrow().len() > MAX_INK_BYTES {
        return Err("Glyph outline exceeds rendering limits".into());
    }
    Ok(Ink::Outline(output.into_inner()))
}

pub(super) fn shape(bytes: &[u8], text: &str, size: f64) -> Result<ShapedText, String> {
    let font = font::new_from_buffer(bytes)
        .map_err(|_| "Cannot load text font")?
        .clone_with_font_size(size);
    let map = utf16_map(text);
    let mut attributed = CFMutableAttributedString::new();
    attributed.replace_str(&CFString::new(text), CFRange::init(0, 0));
    // SAFETY: the font and attributed string are live; the range counts UTF-16 units.
    unsafe {
        attributed.set_attribute(
            CFRange::init(0, (map.len() - 1) as isize),
            kCTFontAttributeName,
            &font,
        );
    }
    let line = CTLine::new_with_attributed_string(attributed.as_concrete_TypeRef());
    let mut sources = super::font_source::Sources::new(font.clone(), bytes);
    let width = line.get_typographic_bounds().width;
    if !width.is_finite() || width < 0. {
        return Err("Invalid text advance".into());
    }
    let mut runs = Vec::new();
    let mut inks = HashMap::new();
    let mut ink_bytes = 0;
    let mut count = 0;
    for run in line.glyph_runs().iter() {
        // SAFETY: framework-owned static string and live run, borrowed for this call.
        let key = unsafe { CFString::wrap_under_get_rule(kCTFontAttributeName) };
        let font = run
            .attributes()
            .and_then(|attrs| attrs.find(key).and_then(|v| v.downcast::<CTFont>()))
            .ok_or("Missing text run font")?;
        let range = unsafe { CTRunGetStringRange(run.as_concrete_TypeRef()) };
        let end = range
            .location
            .checked_add(range.length)
            .ok_or("Invalid native range")?;
        let source = byte_at(&map, range.location)?..byte_at(&map, end)?;
        if source.start > source.end {
            return Err("Reversed text range".into());
        }
        let right_to_left = unsafe { CTRunGetStatus(run.as_concrete_TypeRef()) } & 1 != 0;
        let ids = run.glyphs();
        count += ids.len();
        if count > MAX_TEXT_BYTES * 2 {
            return Err("Too many shaped glyphs".into());
        }
        let positions = run.positions();
        let indices = run.string_indices();
        if ids.len() != positions.len() || ids.len() != indices.len() {
            return Err("Inconsistent native glyph arrays".into());
        }
        let mapped = indices
            .iter()
            .map(|i| byte_at(&map, *i))
            .collect::<Result<Vec<_>, _>>()?;
        if mapped.iter().any(|i| !source.contains(i)) {
            return Err("Glyph outside source range".into());
        }
        let mut boundaries: BTreeSet<_> = mapped.iter().copied().collect();
        boundaries.insert(source.end);
        let mut advances = vec![CGSize::new(0., 0.); ids.len()];
        // SAFETY: this buffer holds exactly the live run's glyph count, all requested by range 0,0.
        unsafe {
            CTRunGetAdvances(
                run.as_concrete_TypeRef(),
                CFRange::init(0, 0),
                advances.as_mut_ptr(),
            );
        }
        let mut glyphs = Vec::with_capacity(ids.len());
        let font_name = font.postscript_name();
        for (((&id, position), advance), start) in
            ids.iter().zip(positions.iter()).zip(&advances).zip(mapped)
        {
            let values = [position.x, position.y, advance.width, advance.height];
            if values.iter().any(|v| !v.is_finite()) {
                return Err("Non-finite glyph position".into());
            }
            let end = *boundaries
                .range((std::ops::Bound::Excluded(start), std::ops::Bound::Unbounded))
                .next()
                .ok_or("Missing cluster end")?;
            let key = (font_name.clone(), font.pt_size().to_bits(), id);
            let ink = if let Some(ink) = inks.get(&key) {
                Arc::clone(ink)
            } else {
                let ink = match font
                    .create_path_for_glyph(id, &CGAffineTransform::new(1., 0., 0., 1., 0., 0.))
                {
                    Ok(path) => outline(&path)?,
                    Err(_) => super::bitmap::render(&font, id)?,
                };
                let ink = Arc::new(ink);
                inks.insert(key, ink.clone());
                ink
            };
            ink_bytes += match &*ink {
                Ink::Outline(path) => path.len(),
                Ink::Bitmap { png, .. } => png.len(),
                Ink::Empty => 0,
            };
            if ink_bytes > MAX_INK_BYTES {
                return Err("Text ink exceeds 16 MB".into());
            }
            glyphs.push(Glyph {
                id,
                source: start..end,
                position: [position.x, -position.y],
                advance: [advance.width, -advance.height],
                ink,
            });
        }
        runs.push(Run {
            source,
            font_name,
            font_source: sources.get(&font, &glyphs),
            font_size: font.pt_size(),
            right_to_left,
            glyphs,
        });
    }
    Ok(ShapedText {
        source: text.to_owned(),
        width,
        runs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utf16_boundary_map_rejects_surrogate_interiors() {
        let map = utf16_map("a👩\u{200d}💻é");
        assert_eq!(
            map,
            vec![
                Some(0),
                Some(1),
                None,
                Some(5),
                Some(8),
                None,
                Some(12),
                Some(14)
            ]
        );
        for index in [-1, 2, 5, 8] {
            assert!(byte_at(&map, index).is_err());
        }
    }
}
