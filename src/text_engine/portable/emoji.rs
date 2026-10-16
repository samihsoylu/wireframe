// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use unicode_segmentation::UnicodeSegmentation;
#[path = "emoji_data.rs"]
mod data;
fn prefers_emoji(grapheme: &str) -> bool {
    let mut previous = None;
    let mut default_emoji = false;
    for c in grapheme.chars() {
        if previous.is_some_and(|p| data::VARIATIONS.binary_search(&(p as u32)).is_ok()) {
            match c {
                '\u{fe0e}' => return false,
                '\u{fe0f}' => return true,
                _ => {}
            }
        }
        let code = c as u32;
        let index = data::PRESENTATION.partition_point(|&(lo, _)| lo <= code);
        default_emoji |= index > 0 && code <= data::PRESENTATION[index - 1].1;
        previous = Some(c);
    }
    default_emoji
        || (grapheme.ends_with('\u{20e3}')
            && grapheme
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit() || matches!(c, '#' | '*')))
}

pub(super) fn family(db: &fontdb::Database) -> Option<&'static str> {
    // Only select an installed face; normal fallback remains available if the
    // platform has no preferred emoji family or lacks a particular sequence.
    ["Apple Color Emoji", "Segoe UI Emoji", "Noto Color Emoji"]
        .into_iter()
        .find(|name| {
            db.query(&fontdb::Query {
                families: &[fontdb::Family::Name(name)],
                ..Default::default()
            })
            .is_some()
        })
}
pub(super) fn spans<'a>(
    text: &'a str,
    base: Attrs<'a>,
    emoji: Option<&'static str>,
) -> Vec<(&'a str, Attrs<'a>)> {
    let Some(family) = emoji else {
        return vec![(text, base)];
    };
    let mut spans = Vec::new();
    let mut start = 0;
    let mut previous = None;
    for (offset, g) in text.grapheme_indices(true) {
        let mode = prefers_emoji(g);
        if previous.is_some_and(|old| old != mode) {
            spans.push((
                &text[start..offset],
                attributes(base.clone(), family, previous.unwrap()),
            ));
            start = offset;
        }
        previous = Some(mode);
    }
    if start < text.len() {
        spans.push((
            &text[start..],
            attributes(base, family, previous.unwrap_or(false)),
        ));
    }
    spans
}
fn attributes<'a>(base: Attrs<'a>, family: &'static str, emoji: bool) -> Attrs<'a> {
    if emoji {
        base.family(Family::Name(family))
            .weight(fontdb::Weight::NORMAL)
            .style(fontdb::Style::Normal)
    } else {
        base
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presentation_respects_valid_selectors_and_leaves_plain_text_alone() {
        for s in [
            "😀",
            "😀\u{fe0e}",
            "☀\u{fe0f}",
            "©\u{fe0f}",
            "1\u{20e3}",
            "1\u{fe0f}\u{20e3}",
            "👍🏽",
            "🇳🇱",
            "👩‍💻",
        ] {
            assert!(prefers_emoji(s), "{s:?}");
        }
        for s in [
            "A",
            "1",
            "#",
            "©",
            "©\u{fe0e}",
            "☀",
            "☀\u{fe0e}",
            "A\u{fe0f}",
        ] {
            assert!(!prefers_emoji(s), "{s:?}");
        }
    }
    #[test]
    fn spans_preserve_source_graphemes_and_coalesce_equal_presentation() {
        let source = "Office a\u{301}\u{323} مرحبا 👩‍💻😀 end";
        let base = Attrs::new()
            .family(Family::Name("Primary"))
            .weight(fontdb::Weight::BOLD)
            .style(fontdb::Style::Italic);
        let result = spans(source, base.clone(), Some("Noto Color Emoji"));
        assert_eq!(result.len(), 3);
        assert_eq!(result.iter().map(|(s, _)| *s).collect::<String>(), source);
        assert_eq!(result[1].0, "👩‍💻😀");
        assert_eq!(result[1].1.weight, fontdb::Weight::NORMAL);
        assert_eq!(result[1].1.style, fontdb::Style::Normal);
        assert_eq!(result[0].1.weight, fontdb::Weight::BOLD);
        assert_eq!(spans(source, base, None).len(), 1);
    }
    #[test]
    fn installed_emoji_face_shapes_selectors_without_losing_source_clusters() {
        let mut engine = Engine::new();
        if family(engine.fonts.db()).is_none() {
            return;
        }
        for source in [
            "😀",
            "😀\u{fe0e}",
            "😀\u{fe0f}",
            "☀\u{fe0f}",
            "©\u{fe0f}",
            "❤\u{fe0f}",
            "👩‍💻",
            "1\u{fe0f}\u{20e3}",
        ] {
            let shaped = engine.shape(crate::fonts::REGULAR, source, 17.).unwrap();
            assert_eq!(shaped.source, source);
            assert!(shaped.width > 0.);
            assert!(
                shaped
                    .runs
                    .iter()
                    .flat_map(|r| &r.glyphs)
                    .any(|g| matches!(&*g.ink, Ink::Bitmap { .. })),
                "{source:?}"
            );
            assert!(
                shaped
                    .runs
                    .iter()
                    .flat_map(|r| &r.glyphs)
                    .all(|g| source.get(g.source.clone()).is_some())
            );
        }
    }
}
