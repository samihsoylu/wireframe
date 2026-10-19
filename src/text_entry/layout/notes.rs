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
        let mut buffer = Buffer::new(
            &mut engine.fonts,
            Metrics::new(size * scale, size * 1.4 * scale),
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
                .metrics(Metrics::new(font_size * scale, font_size * 1.4 * scale))
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
        buffer.shape_until_scroll(&mut engine.fonts, false);
        let mut height = size * 1.4;
        let mut icons = Vec::new();
        for run in buffer.layout_runs() {
            height = height.max((run.line_top + run.line_height) / scale);
            for glyph in run.glyphs {
                let span = &spans[glyph.metadata];
                if let Some(icon) = span.icon {
                    let side = span_size(&span.style, size);
                    // The em-space reserves an inline box in the same shaped
                    // paragraph, including wrapping and bidirectional ordering.
                    let rect = Rect::from_center_size(
                        egui::pos2(
                            (glyph.x + glyph.w / 2.) / scale,
                            run.line_y / scale - side * 0.35,
                        ),
                        Vec2::splat(side),
                    );
                    icons.push((icon, rect, ink(&span.style)));
                }
            }
        }
        Ok(Self {
            layout: Layout {
                source: spans.iter().map(|s| s.text.as_str()).collect(),
                buffer,
                rows: Vec::new(),
                size: Vec2::new(width, height),
                scale,
            },
            icons,
        })
    }

    pub fn size(&self) -> Vec2 {
        self.layout.size
    }

    pub fn raster(&mut self, engine: &mut Engine, viewport: Rect) -> Result<ColorImage, String> {
        self.layout.raster(engine, viewport, Color32::BLACK)
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
            Inline::Break(_) => result.push(Span {
                text: "\n".into(),
                style: Style::default(),
                icon: None,
            }),
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
        if start > 0 && !text[..start].ends_with(char::is_whitespace) {
            continue;
        }
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
