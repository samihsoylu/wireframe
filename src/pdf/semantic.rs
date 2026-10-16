// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use crate::text_engine::ShapedText;
use krilla::{
    geom::Point,
    num::NormalizedF32,
    paint::Fill,
    surface::Surface,
    text::{Font, GlyphId, KrillaGlyph},
};
use std::{collections::HashMap, ops::Range, sync::Arc};
pub(super) struct Cluster {
    pub range: Range<usize>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub size: f32,
    pub rtl: bool,
}
pub(super) fn clusters(shaped: &ShapedText, unavailable: &[bool]) -> Vec<Cluster> {
    let mut ranges: Vec<_> = shaped
        .runs
        .iter()
        .enumerate()
        .flat_map(|(i, r)| {
            r.glyphs.iter().map(move |g| {
                (
                    g.source.clone(),
                    unavailable[i],
                    g.position,
                    g.advance,
                    r.font_size,
                    r.right_to_left,
                )
            })
        })
        .collect();
    ranges.sort_by_key(|(r, ..)| (r.start, r.end));
    let mut merged: Vec<(Cluster, bool, f64)> = vec![];
    for (range, needed, pos, advance, size, rtl) in ranges {
        let right = pos[0] + advance[0];
        if let Some((last, flag, edge)) = merged
            .last_mut()
            .filter(|(r, ..)| range.start < r.range.end)
        {
            last.range.end = last.range.end.max(range.end);
            last.x = last.x.min(pos[0] as f32);
            *edge = edge.max(right);
            last.width = (*edge - last.x as f64).max(0.01) as f32;
            *flag |= needed;
        } else {
            merged.push((
                Cluster {
                    range,
                    x: pos[0] as f32,
                    y: pos[1] as f32,
                    width: advance[0].max(0.01) as f32,
                    size: size as f32,
                    rtl,
                },
                needed,
                right,
            ));
        }
    }
    merged
        .into_iter()
        .filter(|(_, needed, _)| *needed)
        .map(|(c, ..)| c)
        .collect()
}
pub(super) fn index(clusters: &[Cluster], range: &Range<usize>) -> Option<usize> {
    let i = clusters.partition_point(|c| c.range.start <= range.start);
    (i > 0 && range.start < clusters[i - 1].range.end).then(|| i - 1)
}
#[derive(Default)]
pub(super) struct Fonts {
    fonts: HashMap<Vec<(char, u16)>, Font>,
}
struct Atom {
    range: Range<usize>,
    x: f32,
    y: f32,
    width: f32,
    size: f32,
    ch: char,
}
impl Fonts {
    pub fn draw(
        &mut self,
        surface: &mut Surface<'_>,
        source: &str,
        clusters: &[Cluster],
        indices: &[usize],
        x: f32,
        y: f32,
    ) -> Result<(), String> {
        let mut atoms = vec![];
        for &i in indices {
            let c = &clusters[i];
            if ![c.x, c.y, c.width, c.size].iter().all(|v| v.is_finite())
                || c.width <= 0.
                || c.size <= 0.
            {
                return Err("Invalid PDF semantic text geometry".into());
            }
            let text = source
                .get(c.range.clone())
                .ok_or("Invalid PDF semantic source range")?;
            let count = text.chars().count();
            let width = c.width / count.max(1) as f32;
            let mut local: Vec<_> = text
                .char_indices()
                .enumerate()
                .map(|(j, (offset, ch))| Atom {
                    range: c.range.start + offset..c.range.start + offset + ch.len_utf8(),
                    x: c.x + if c.rtl { count - j - 1 } else { j } as f32 * width,
                    y: c.y,
                    width,
                    size: c.size,
                    ch,
                })
                .collect();
            if c.rtl {
                local.reverse();
            }
            atoms.extend(local);
        }
        for group in atoms.chunk_by(|a, b| a.size == b.size) {
            for chunk in group.chunks(512) {
                let key: Vec<_> = chunk
                    .iter()
                    .map(|a| {
                        (
                            a.ch,
                            (a.width / a.size * 1000.).round().clamp(1., 65535.) as u16,
                        )
                    })
                    .collect();
                let next = self.fonts.len();
                let font = if let Some(f) = self.fonts.get(&key) {
                    f.clone()
                } else {
                    let f = carrier(next, &key)?;
                    self.fonts.insert(key, f.clone());
                    f
                };
                let size = chunk[0].size;
                let mut pen = 0.;
                let glyphs: Vec<_> = chunk
                    .iter()
                    .enumerate()
                    .map(|(i, a)| {
                        let glyph = KrillaGlyph {
                            glyph_id: GlyphId::new(i as u32 + 1),
                            text_range: a.range.clone(),
                            x_advance: a.width / size,
                            y_advance: 0.,
                            x_offset: (a.x - pen) / size,
                            y_offset: a.y / size,
                            location: None,
                        };
                        pen += a.width;
                        glyph
                    })
                    .collect();
                surface.set_fill(Some(Fill {
                    opacity: NormalizedF32::ZERO,
                    ..Default::default()
                }));
                surface.set_stroke(None);
                let start = chunk.iter().map(|a| a.range.start).min().unwrap();
                let end = chunk.iter().map(|a| a.range.end).max().unwrap();
                let exact = chunk.iter().map(|a| a.range.len()).sum::<usize>() == end - start;
                surface.start_tagged(krilla::tagging::ContentTag::Span(
                    krilla::tagging::SpanTag::empty()
                        .with_actual_text(exact.then(|| &source[start..end])),
                ));
                surface.draw_glyphs(Point::from_xy(x, y), &glyphs, font, source, size, false);
                surface.end_tagged();
            }
        }
        Ok(())
    }
}
fn carrier(index: usize, key: &[(char, u16)]) -> Result<Font, String> {
    let source = skrifa::FontRef::new(include_bytes!("../../assets/pdf/semantic.ttf"))
        .map_err(|e| e.to_string())?;
    let mut tables = vec![];
    for r in source.table_directory.table_records() {
        let tag = r.tag();
        let bytes = if tag == skrifa::Tag::new(b"name") {
            let name = format!("WireframePDFSemantic{index}");
            let utf16: Vec<u8> = name.encode_utf16().flat_map(u16::to_be_bytes).collect();
            let mut bytes = vec![];
            for word in [0u16, 1, 18, 3, 1, 0x409, 6, utf16.len() as u16, 0] {
                bytes.extend_from_slice(&word.to_be_bytes());
            }
            bytes.extend(utf16);
            bytes
        } else if tag == skrifa::Tag::new(b"glyf") {
            let mut bytes = vec![];
            for i in 0..513 {
                let w = if i > 0 && i <= key.len() {
                    key[i - 1].1.min(32767) as i16
                } else {
                    1000
                };
                for n in [1i16, 0, -200, w, 800, 3, 0] {
                    bytes.extend_from_slice(&n.to_be_bytes());
                }
                bytes.extend_from_slice(&[1; 4]);
                for n in [0, w, 0, -w, -200, 0, 1000, 0] {
                    bytes.extend_from_slice(&n.to_be_bytes());
                }
            }
            bytes
        } else if tag == skrifa::Tag::new(b"loca") {
            (0..514u16).flat_map(|i| (i * 17).to_be_bytes()).collect()
        } else if tag == skrifa::Tag::new(b"cmap") {
            let mut chars = std::collections::BTreeMap::new();
            for (i, (ch, _)) in key.iter().enumerate() {
                chars.entry(*ch as u32).or_insert(i as u32 + 1);
            }
            let mut bytes = vec![];
            for n in [0u16, 1, 3, 10] {
                bytes.extend_from_slice(&n.to_be_bytes());
            }
            bytes.extend_from_slice(&12u32.to_be_bytes());
            bytes.extend_from_slice(&12u16.to_be_bytes());
            bytes.extend_from_slice(&0u16.to_be_bytes());
            for n in [16 + 12 * chars.len() as u32, 0, chars.len() as u32] {
                bytes.extend_from_slice(&n.to_be_bytes());
            }
            for (cp, id) in chars {
                for n in [cp, cp, id] {
                    bytes.extend_from_slice(&n.to_be_bytes());
                }
            }
            bytes
        } else if tag == skrifa::Tag::new(b"head") {
            let mut bytes = source.table_data(tag).unwrap().as_bytes().to_vec();
            bytes[40..42].copy_from_slice(
                &(key
                    .iter()
                    .map(|x| x.1.min(32767))
                    .max()
                    .unwrap_or(1000)
                    .max(1000) as i16)
                    .to_be_bytes(),
            );
            bytes[50..52].copy_from_slice(&0i16.to_be_bytes());
            bytes
        } else if tag == skrifa::Tag::new(b"hmtx") {
            let mut bytes = vec![];
            for i in 0..513 {
                let width = if i > 0 && i <= key.len() {
                    key[i - 1].1
                } else {
                    1000
                };
                bytes.extend_from_slice(&width.to_be_bytes());
                bytes.extend_from_slice(&0i16.to_be_bytes());
            }
            bytes
        } else if tag == skrifa::Tag::new(b"hhea") {
            let mut bytes = source
                .table_data(tag)
                .ok_or("Carrier header missing")?
                .as_bytes()
                .to_vec();
            bytes[10..12].copy_from_slice(
                &key.iter()
                    .map(|x| x.1)
                    .max()
                    .unwrap_or(1000)
                    .max(1000)
                    .to_be_bytes(),
            );
            bytes[34..36].copy_from_slice(&513u16.to_be_bytes());
            bytes
        } else {
            source
                .table_data(tag)
                .ok_or("Carrier table missing")?
                .as_bytes()
                .to_vec()
        };
        tables.push((u32::from_be_bytes(tag.to_be_bytes()), bytes));
    }
    let bytes = crate::text_engine::sfnt::pack(tables, 128 * 1024)?;
    Font::new(Arc::new(bytes).into(), 0).ok_or_else(|| "Invalid semantic carrier".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text_engine::{FontSource, Glyph, Ink, Run};
    fn sample() -> ShapedText {
        let glyph = |range, x, advance| Glyph {
            id: 1,
            source: range,
            position: [x, 0.],
            advance: [advance, 0.],
            ink: Arc::new(Ink::Empty),
        };
        let run = |glyphs| Run {
            source: 0..7,
            font_name: "Fixture".into(),
            font_source: FontSource::InkOnly("Fixture".into()),
            font_size: 17.,
            right_to_left: false,
            glyphs,
        };
        ShapedText {
            source: "😀\u{fe0f}".into(),
            width: 17.,
            runs: vec![
                run(vec![glyph(0..7, 0., 17.)]),
                run(vec![glyph(0..7, 17., 0.)]),
            ],
        }
    }
    #[test]
    fn overlapping_ink_and_embedded_empty_glyphs_have_one_semantic_cluster() {
        let s = sample();
        let c = clusters(&s, &[true, false]);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].range, 0..7);
        assert_eq!(c[0].width, 17.);
        assert_eq!(index(&c, &(0..7)), Some(0));
        assert!(clusters(&s, &[false, false]).is_empty());
    }
    #[test]
    fn carrier_fonts_keep_unicode_and_width_mappings_and_unique_chunk_names() {
        let key = [('😀', 1000), ('\u{200d}', 250), ('א', 500)];
        let font = carrier(0, &key).unwrap();
        let other = carrier(1, &key).unwrap();
        assert_ne!(font, other);
        let mut surface_doc = krilla::Document::new();
        let mut page = surface_doc.start_page();
        {
            let mut surface = page.surface();
            let mut fonts = Fonts::default();
            let source = "a".repeat(1025);
            let c = Cluster {
                range: 0..source.len(),
                x: 0.,
                y: 0.,
                width: 100.,
                size: 17.,
                rtl: false,
            };
            fonts
                .draw(&mut surface, &source, &[c], &[0], 0., 20.)
                .unwrap();
            assert_eq!(fonts.fonts.len(), 2);
        }
        page.finish();
        assert!(surface_doc.finish().is_ok());
    }
    #[test]
    fn invalid_semantic_geometry_and_source_return_errors() {
        let mut doc = krilla::Document::new();
        let mut page = doc.start_page();
        let mut surface = page.surface();
        let mut fonts = Fonts::default();
        let mut c = Cluster {
            range: 0..4,
            x: 0.,
            y: 0.,
            width: 10.,
            size: 0.,
            rtl: false,
        };
        assert!(fonts.draw(&mut surface, "😀", &[c], &[0], 0., 0.).is_err());
        c = Cluster {
            range: 1..4,
            x: 0.,
            y: 0.,
            width: 10.,
            size: 17.,
            rtl: false,
        };
        assert!(fonts.draw(&mut surface, "😀", &[c], &[0], 0., 0.).is_err());
    }
}
