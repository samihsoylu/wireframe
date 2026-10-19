// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Styled, read-only notes. The editor continues to own the original markup.
use super::*;
use crate::rich_text::{Inline, RichText, Style, painting::parse_color};

struct Span {
    text: String,
    style: Style,
    icon: Option<&'static crate::icons::Icon>,
}

pub struct NotesLayout {
    layout: Layout,
    lines: Vec<LinePlacement>,
    icons: Vec<(&'static crate::icons::Icon, Rect, Color32)>,
}

impl NotesLayout {
    pub fn new(
        engine: &mut Engine,
        source: &str,
        size: f32,
        width: f32,
        scale: f32,
    ) -> Result<Self, String> {
        if source.len() > 1024 * 1024 {
            return Err("Notes exceed the 1 MiB layout limit".into());
        }
        if !size.is_finite()
            || !(1.0..=128.0).contains(&size)
            || !width.is_finite()
            || !(1.0..=16384.0).contains(&width)
            || !scale.is_finite()
            || !(0.5..=8.0).contains(&scale)
        {
            return Err("Invalid notes dimensions".into());
        }
        let spans = spans(source);
        let families = spans
            .iter()
            .map(|s| {
                engine.interface_family(
                    span_size(&s.style, size),
                    if s.style.bold {
                        InterfaceRole::Bold
                    } else {
                        InterfaceRole::Regular
                    },
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let base_family = engine.interface_family(size, InterfaceRole::Regular)?;
        let base_metrics =
            font_metrics(engine, size, InterfaceRole::Regular, base_family.as_deref())?;
        let mut metrics_cache = BTreeMap::new();
        let mut span_metrics = Vec::with_capacity(spans.len());
        for (span, family) in spans.iter().zip(&families) {
            let font_size = span_size(&span.style, size);
            let role = if span.style.bold {
                InterfaceRole::Bold
            } else {
                InterfaceRole::Regular
            };
            let metrics = match metrics_cache.entry((font_size.to_bits(), role)) {
                std::collections::btree_map::Entry::Occupied(entry) => *entry.get(),
                std::collections::btree_map::Entry::Vacant(entry) => {
                    *entry.insert(font_metrics(engine, font_size, role, family.as_deref())?)
                }
            };
            span_metrics.push(metrics);
        }
        let mut buffer = Buffer::new(
            &mut engine.fonts,
            Metrics::new(size * scale, base_metrics.0 * scale),
        );
        buffer.set_wrap(Wrap::WordOrGlyph);
        buffer.set_size(Some(width * scale), None);
        let attrs = spans.iter().enumerate().map(|(i, span)| {
            let color = ink(&span.style);
            let font_size = span_size(&span.style, size);
            let mut attrs = Attrs::new()
                .family(Family::Name(families[i].as_deref().unwrap_or("Ubuntu")))
                .weight(if families[i].is_some() {
                    Weight::NORMAL
                } else if span.style.bold {
                    Weight::BOLD
                } else {
                    Weight::LIGHT
                })
                .metadata(i)
                .metrics(Metrics::new(font_size * scale, span_metrics[i].0 * scale))
                .color(cosmic_text::Color::rgba(
                    color.r(),
                    color.g(),
                    color.b(),
                    color.a(),
                ));
            if span.style.italic {
                attrs = attrs.cache_key_flags(cosmic_text::CacheKeyFlags::FAKE_ITALIC);
            }
            if span.style.underline {
                attrs = attrs.underline(cosmic_text::UnderlineStyle::Single);
            }
            if span.style.strike {
                attrs = attrs.strikethrough();
            }
            (span.text.as_str(), attrs)
        });
        buffer.set_rich_text(attrs, &Attrs::new(), Shaping::Advanced, None);
        // A final line break reserves the following empty line in the read view.
        if spans.last().is_some_and(|span| span.text.ends_with('\n')) {
            buffer.lines.push(cosmic_text::BufferLine::new(
                "",
                cosmic_text::LineEnding::None,
                cosmic_text::AttrsList::new(&Attrs::new()),
                Shaping::Advanced,
            ));
        }
        buffer.shape_until_scroll(&mut engine.fonts, false);
        let mut height = 0.;
        let mut lines = Vec::new();
        let mut icons = Vec::new();
        for run in buffer.layout_runs() {
            let icons_only = !run.glyphs.is_empty()
                && run
                    .glyphs
                    .iter()
                    .all(|glyph| spans[glyph.metadata].icon.is_some());
            let (mut baseline, mut below) = run
                .glyphs
                .iter()
                .map(|glyph| span_metrics[glyph.metadata])
                .map(|(height, baseline)| (baseline, height - baseline))
                .reduce(|a, b| (a.0.max(b.0), a.1.max(b.1)))
                .unwrap_or((base_metrics.1, base_metrics.0 - base_metrics.1));
            if icons_only {
                // Attachment-only rows occupy their image extent without a
                // text descender. Spaces still count as ordinary text.
                baseline = run
                    .glyphs
                    .iter()
                    .map(|glyph| span_size(&spans[glyph.metadata].style, size))
                    .fold(0., f32::max);
                below = 0.;
            }
            let line = LinePlacement {
                top: height * scale,
                baseline: (height + baseline) * scale,
                height: (baseline + below) * scale,
            };
            height += baseline + below;
            for glyph in run.glyphs {
                let span = &spans[glyph.metadata];
                if let Some(icon) = span.icon {
                    let side = span_size(&span.style, size);
                    // The em-space reserves an inline box in the same shaped
                    // paragraph, including wrapping and bidirectional ordering.
                    let rect = Rect::from_center_size(
                        egui::pos2(
                            (glyph.x + glyph.w / 2.) / scale,
                            line.baseline / scale - side * if icons_only { 0.5 } else { 0.36 },
                        ),
                        Vec2::splat(side),
                    );
                    icons.push((icon, rect, ink(&span.style)));
                }
            }
            lines.push(line);
        }
        Ok(Self {
            layout: Layout {
                source: spans.iter().map(|s| s.text.as_str()).collect(),
                buffer,
                rows: Vec::new(),
                size: Vec2::new(
                    width,
                    if lines.is_empty() {
                        base_metrics.0
                    } else {
                        height
                    },
                ),
                scale,
            },
            lines,
            icons,
        })
    }

    pub fn size(&self) -> Vec2 {
        self.layout.size
    }

    pub fn raster(&mut self, engine: &mut Engine, viewport: Rect) -> Result<ColorImage, String> {
        self.layout
            .raster_with_lines(engine, viewport, Color32::BLACK, Some(&self.lines))
    }

    pub fn paint_icons(&self, painter: &egui::Painter, origin: Pos2) {
        for (icon, rect, color) in &self.icons {
            let rect = rect.translate(origin.to_vec2());
            if painter.clip_rect().intersects(rect) {
                icon.paint(painter, rect, *color);
            }
        }
    }
}

fn span_size(style: &Style, default: f32) -> f32 {
    style
        .size
        .filter(|s| (1..=512).contains(s))
        .map_or(default, f32::from)
}

fn ink(style: &Style) -> Color32 {
    if style.disabled {
        Color32::from_gray(179)
    } else if let Some(color) = style
        .color
        .as_deref()
        .map(|value| parse_color(value).unwrap_or(Color32::BLACK))
    {
        color
    } else if style.link {
        Color32::from_rgb(6, 123, 186)
    } else {
        Color32::BLACK
    }
}

fn spans(source: &str) -> Vec<Span> {
    let mut result = Vec::new();
    for item in RichText::parse(source, false).items {
        match item {
            Inline::Span { text, style } => split_icons(&text, style, &mut result),
            Inline::Break(crate::rich_text::Break::Hard) => result.push(Span {
                text: "\n".into(),
                style: Style::default(),
                icon: None,
            }),
            Inline::Break(crate::rich_text::Break::Soft) => {}
            Inline::Bullet => unreachable!("Notes do not parse bullet lists"),
        }
    }
    result
}

fn split_icons(text: &str, style: Style, output: &mut Vec<Span>) {
    let mut copied = 0;
    let mut search = 0;
    while let Some(offset) = text[search..].find(':') {
        let start = search + offset;
        search = start + 1;
        let Some(end) = text[search..].find(':').map(|i| search + i) else {
            break;
        };
        let Some(icon) = crate::icons::get(&text[search..end]) else {
            continue;
        };
        if start > copied {
            output.push(Span {
                text: text[copied..start].into(),
                style: style.clone(),
                icon: None,
            });
        }
        output.push(Span {
            text: "\u{2003}".into(),
            style: style.clone(),
            icon: Some(icon),
        });
        search = end + 1;
        copied = search;
    }
    if copied < text.len() {
        output.push(Span {
            text: text[copied..].into(),
            style,
            icon: None,
        });
    }
}

#[cfg(test)]
mod tests;

fn font_metrics(
    engine: &mut Engine,
    size: f32,
    role: InterfaceRole,
    family: Option<&str>,
) -> Result<(f32, f32), String> {
    if family.is_some()
        && let Some(metrics) = crate::fonts::interface::system_line_metrics(size, role)
    {
        return Ok(metrics);
    }
    let weight = if family.is_some() {
        Weight::NORMAL
    } else if role == InterfaceRole::Bold {
        Weight::BOLD
    } else {
        Weight::LIGHT
    };
    let id = engine
        .fonts
        .db()
        .query(&cosmic_text::fontdb::Query {
            families: &[Family::Name(family.unwrap_or("Ubuntu"))],
            weight,
            ..Default::default()
        })
        .ok_or("Notes font unavailable")?;
    let font = engine
        .fonts
        .get_font(id, weight)
        .ok_or("Notes font metrics unavailable")?;
    let metrics = font.metrics();
    let factor = size / f32::from(metrics.units_per_em);
    let baseline = (metrics.ascent * factor).ceil();
    let height =
        baseline + (-metrics.descent * factor).ceil() + (metrics.leading * factor).max(0.).ceil();
    if !height.is_finite()
        || !baseline.is_finite()
        || height <= 0.
        || baseline < 0.
        || baseline > height
    {
        return Err("Invalid notes font metrics".into());
    }
    Ok((height, baseline))
}
