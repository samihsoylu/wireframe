// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Shared positioned drawing data for the canvas and vector/raster exports.
mod svg;
mod table;
use crate::{
    document::{Properties, Value},
    layout::{EguiMeasurer, TextStyle},
    model::{Bounds, Kind},
    rich_text::{RichText, painting::PaintOptions},
    scene::Primitive,
    workspace::Layer,
};
use eframe::egui::{self, Color32};
#[derive(Clone, Debug)]
pub struct Glyph {
    pub text: String,
    pub x: f32,
    pub baseline: f32,
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
    pub color: [u8; 4],
    pub advance: f32,
}
#[derive(Clone, Debug)]
pub enum Drawing {
    Primitive(Primitive),
    Glyph(Glyph),
}
#[derive(Clone, Debug)]
pub struct Item {
    pub drawing: Drawing,
    pub clip: Option<Bounds>,
}
#[derive(Clone, Debug)]
pub struct LinkRegion {
    pub target: String,
    pub bounds: Bounds,
}
#[derive(Clone, Debug)]
pub struct Scene {
    pub title: String,
    pub bounds: Bounds,
    pub items: Vec<Item>,
    pub links: Vec<LinkRegion>,
}
#[derive(Clone, Copy)]
pub struct TextBox {
    pub bounds: Bounds,
    pub align: Alignment,
    pub middle: bool,
    pub wrap: bool,
    pub soft_breaks: bool,
    pub truncate: bool,
    pub clip: Option<Bounds>,
}
#[derive(Clone, Copy, Default)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}
impl Alignment {
    fn offset(self, space: f32, width: f32) -> f32 {
        match self {
            Self::Left => 0.,
            Self::Center => (space - width) / 2.,
            Self::Right => space - width,
        }
    }
}
impl Scene {
    pub fn build(
        title: &str,
        layers: &[Layer],
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<Self, String> {
        let bounds = layers
            .iter()
            .map(|l| {
                l.clip
                    .map_or(l.control.bounds, |b| intersect(l.control.bounds, b))
            })
            .reduce(Bounds::union)
            .unwrap_or(Bounds::new(0., 0., 800., 600.));
        let mut scene = Self {
            title: title.into(),
            bounds,
            items: vec![],
            links: vec![],
        };
        for layer in layers {
            let visible = layer.clip.map_or(layer.control.bounds, |clip| {
                intersect(layer.control.bounds, clip)
            });
            if visible.w > 0. && visible.h > 0. {
                // Later controls own their visible hit area, including blank space.
                // Split covered links so canvas and exported anchors agree.
                scene.links = scene
                    .links
                    .drain(..)
                    .flat_map(|link| {
                        subtract(link.bounds, visible)
                            .into_iter()
                            .map(move |bounds| LinkRegion {
                                target: link.target.clone(),
                                bounds,
                            })
                    })
                    .collect();
            }
            scene.control(layer, sketch, fonts)?;
            if scene.items.len() + scene.links.len() > 1_000_000 {
                return Err("This page exceeds the drawing complexity limit".into());
            }
        }
        Ok(scene)
    }
    fn primitive(&mut self, p: Primitive, clip: Option<Bounds>) {
        self.items.push(Item {
            drawing: Drawing::Primitive(p),
            clip,
        });
    }
    fn control(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let b = c.bounds;
        if c.kind == Kind::Table {
            return self.table(layer, sketch, fonts);
        }
        let supported = matches!(
            c.kind,
            Kind::Label
                | Kind::Heading
                | Kind::Link
                | Kind::Paragraph
                | Kind::TextArea
                | Kind::Button
                | Kind::TextInput
                | Kind::Dropdown
                | Kind::Search
                | Kind::Checkbox
                | Kind::Radio
        );
        if !supported {
            for p in crate::scene::control_scene(c, sketch) {
                self.primitive(p, layer.clip);
            }
            return Ok(());
        }
        let text_only = matches!(
            c.kind,
            Kind::Label | Kind::Heading | Kind::Link | Kind::Paragraph
        );
        if !text_only {
            let mut background = c.clone();
            background.text.clear();
            for p in crate::scene::control_scene(&background, sketch) {
                if !matches!(p, Primitive::Text { .. }) {
                    self.primitive(p, layer.clip);
                }
            }
        }
        let kind = match c.kind {
            Kind::Button => "button",
            Kind::TextArea => "text_area",
            Kind::TextInput => "text_input",
            Kind::Dropdown => "dropdown",
            Kind::Search => "search",
            Kind::Checkbox => "checkbox",
            Kind::Radio => "radio",
            Kind::Paragraph => "paragraph",
            _ => "label",
        };
        let placement = crate::layout::text_placement(kind);
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(&layer.properties, "italic"),
        };
        let wrap = placement.wrap;
        let area = Bounds::new(
            b.x + placement.left,
            b.y + placement.top,
            (b.w - placement.left - placement.right).max(1.),
            (b.h - placement.top - placement.bottom).max(1.),
        );
        let parsed = RichText::parse(&c.text, c.kind == Kind::Paragraph);
        let mut options = paint_options(&layer.properties);
        options.underline |= c.kind == Kind::Link;
        self.text(
            &parsed,
            style,
            TextBox {
                bounds: area,
                align: alignment(&layer.properties),
                middle: placement.middle,
                wrap,
                soft_breaks: wrap,
                truncate: false,
                clip: clip_with(layer.clip, b),
            },
            fonts,
            options,
        )?;
        Ok(())
    }
    fn text(
        &mut self,
        text: &RichText,
        style: TextStyle,
        area: TextBox,
        fonts: &EguiMeasurer<'_>,
        options: PaintOptions,
    ) -> Result<(), String> {
        let painted = fonts.rich_galley_with(
            text,
            style,
            area.wrap.then_some(area.bounds.w),
            area.soft_breaks,
            options,
        );
        let galley = &painted.galley;
        let vertical = if area.middle {
            (area.bounds.h - galley.size().y) / 2.
        } else {
            0.
        };
        let visible_width = |row: &egui::epaint::text::PlacedRow| {
            if area.truncate {
                row.glyphs
                    .iter()
                    .take_while(|g| g.pos.x + g.advance_width <= area.bounds.w)
                    .last()
                    .map_or(0., |g| g.pos.x + g.advance_width)
            } else {
                row.size.x
            }
        };
        let mut byte = 0;
        let mut section = 0;
        for row in &galley.rows {
            let width = visible_width(row);
            let dx = area.bounds.x + area.align.offset(area.bounds.w, width);
            let dy = area.bounds.y + vertical;
            for glyph in &row.glyphs {
                while section + 1 < galley.job.sections.len()
                    && byte >= galley.job.sections[section].byte_range.end.0
                {
                    section += 1;
                }
                let Some(format) = galley.job.sections.get(section).map(|s| &s.format) else {
                    continue;
                };
                byte += glyph.chr.len_utf8();
                if area.truncate && glyph.pos.x + glyph.advance_width > width {
                    continue;
                }
                let family = format.font_id.family.to_string();
                let x = dx + row.pos.x + glyph.pos.x;
                let baseline = dy + row.pos.y + glyph.pos.y;
                self.items.push(Item {
                    drawing: Drawing::Glyph(Glyph {
                        text: glyph.chr.to_string(),
                        x,
                        baseline,
                        size: format.font_id.size,
                        bold: family.contains("bold"),
                        italic: family.contains("italic") || format.italics,
                        color: format.color.to_array(),
                        advance: glyph.advance_width,
                    }),
                    clip: area.clip,
                });
                for (stroke, y) in [
                    (format.underline, baseline + style.size * 0.12),
                    (format.strikethrough, baseline - format.font_id.size * 0.3),
                ] {
                    if stroke.width > 0. {
                        self.primitive(
                            Primitive::Line {
                                points: vec![[x, y], [x + glyph.advance_width, y]],
                                color: [stroke.color.r(), stroke.color.g(), stroke.color.b()],
                                width: stroke.width,
                            },
                            area.clip,
                        );
                    }
                }
            }
            if row.ends_with_newline {
                byte += 1;
            }
        }
        for link in painted.links {
            let row = galley.rows.iter().find(|r| {
                link.rect.center().y >= r.pos.y && link.rect.center().y <= r.pos.y + r.size.y
            });
            let dx = row.map_or(0., |r| area.align.offset(area.bounds.w, visible_width(r)));
            let mut bounds = Bounds::new(
                area.bounds.x + dx + link.rect.min.x,
                area.bounds.y + vertical + link.rect.min.y,
                link.rect.width(),
                link.rect.height(),
            );
            if area.truncate
                && let Some(row) = row
            {
                bounds = intersect(
                    bounds,
                    Bounds::new(area.bounds.x + dx, bounds.y, visible_width(row), bounds.h),
                );
            }
            if let Some(clip) = area.clip {
                bounds = intersect(bounds, clip);
            }
            if bounds.w > 0. && bounds.h > 0. {
                self.links.push(LinkRegion {
                    target: link.target,
                    bounds,
                });
            }
        }
        if self.items.len() > 1_000_000 {
            return Err("This page exceeds the drawing complexity limit".into());
        }
        Ok(())
    }
    pub fn hit_link(&self, x: f32, y: f32) -> Option<&str> {
        self.links
            .iter()
            .rev()
            .find(|l| l.bounds.contains(x, y))
            .map(|l| l.target.as_str())
    }
}
fn subtract(bounds: Bounds, cover: Bounds) -> Vec<Bounds> {
    let overlap = intersect(bounds, cover);
    if overlap.w <= 0. || overlap.h <= 0. {
        return vec![bounds];
    }
    [
        Bounds::new(bounds.x, bounds.y, bounds.w, overlap.y - bounds.y),
        Bounds::new(
            bounds.x,
            overlap.y + overlap.h,
            bounds.w,
            bounds.y + bounds.h - overlap.y - overlap.h,
        ),
        Bounds::new(bounds.x, overlap.y, overlap.x - bounds.x, overlap.h),
        Bounds::new(
            overlap.x + overlap.w,
            overlap.y,
            bounds.x + bounds.w - overlap.x - overlap.w,
            overlap.h,
        ),
    ]
    .into_iter()
    .filter(|b| b.w > 0. && b.h > 0.)
    .collect()
}
pub fn intersect(a: Bounds, b: Bounds) -> Bounds {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    Bounds::new(
        x,
        y,
        ((a.x + a.w).min(b.x + b.w) - x).max(0.),
        ((a.y + a.h).min(b.y + b.h) - y).max(0.),
    )
}
fn clip_with(clip: Option<Bounds>, b: Bounds) -> Option<Bounds> {
    Some(clip.map_or(b, |c| intersect(c, b)))
}
fn boolean(p: &Properties, key: &str) -> bool {
    matches!(p.get(key), Some(Value::Bool(true)))
}
fn number(p: &Properties, key: &str, default: f32) -> f32 {
    match p.get(key) {
        Some(Value::Number(v)) => *v as f32,
        _ => default,
    }
}
fn rgb(p: &Properties, key: &str, default: [u8; 3]) -> [u8; 3] {
    match p.get(key) {
        Some(Value::Color(v)) => [v[0], v[1], v[2]],
        _ => default,
    }
}
fn alignment(p: &Properties) -> Alignment {
    match p.get("align") {
        Some(Value::Choice(v)) if v == "center" => Alignment::Center,
        Some(Value::Choice(v)) if v == "right" => Alignment::Right,
        _ => Alignment::Left,
    }
}
fn paint_options(p: &Properties) -> PaintOptions {
    let color = rgb(p, "text_color", [0, 0, 0]);
    PaintOptions {
        ink: if matches!(p.get("state"),Some(Value::Choice(s))if s.contains("disabled")) {
            Color32::from_gray(153)
        } else {
            Color32::from_rgb(color[0], color[1], color[2])
        },
        link: Color32::from_rgb(0, 102, 204),
        underline: boolean(p, "underline"),
    }
}
/// Draw positioned glyphs using this canvas's own font atlas. Cached atlas coordinates
/// from the measurement context are intentionally never reused.
pub fn paint_glyph(painter: &egui::Painter, g: &Glyph, origin: egui::Pos2, scale: f32) {
    let color = Color32::from_rgba_unmultiplied(g.color[0], g.color[1], g.color[2], g.color[3]);
    let font = egui::FontId::new(g.size * scale, crate::fonts::family(g.bold, g.italic));
    let galley = painter.layout_no_wrap(g.text.clone(), font, color);
    let offset = galley
        .rows
        .first()
        .and_then(|r| {
            r.glyphs
                .first()
                .map(|glyph| r.pos.to_vec2() + glyph.pos.to_vec2())
        })
        .unwrap_or_default();
    painter.galley(
        origin + egui::vec2(g.x, g.baseline) * scale - offset,
        galley,
        color,
    );
}
