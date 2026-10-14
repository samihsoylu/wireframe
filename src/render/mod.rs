// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Shared positioned drawing data for the canvas and vector/raster exports.
mod button_bar;
mod choice;
mod circle_button;
mod control_icons;
mod dropdown;
mod forms;
mod images;
mod list;
mod multiline_button;
mod pointed_button;
mod rasters;
mod scrollbar;
mod search;
mod standalone_icons;
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
    Icon {
        bounds: Bounds,
        id: String,
        turns: u8,
        color: [u8; 4],
    },
    Image {
        bounds: Bounds,
        data: std::sync::Arc<crate::images::Decoded>,
    },
}
#[derive(Clone, Debug)]
pub struct Item {
    pub opacity: f32,
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
        assets: &[crate::document::Asset],
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
        let mut rasters = rasters::Rasters::new(assets);
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
            let image = if layer.control.kind == Kind::Image && layer.control.group.is_none() {
                if let Some(Value::Asset(id)) = layer.properties.get("image") {
                    let turns = crate::images::quarter_turns(&layer.properties);
                    let filter = boolean(&layer.properties, "sketch_filter");
                    rasters.get(
                        *id,
                        rasters::Style::Image {
                            turns,
                            sketch: filter,
                        },
                    )
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(data) = image {
                scene.items.push(Item {
                    opacity: 1.,
                    drawing: Drawing::Image {
                        bounds: images::raster_bounds(layer),
                        data,
                    },
                    clip: images::raster_clip(layer),
                });
                scene.image_decoration(layer, sketch, fonts)?;
            } else {
                scene.control(layer, sketch, fonts, &mut rasters)?;
                if layer.control.kind == Kind::Image
                    && matches!(layer.properties.get("image"), Some(Value::Asset(_)))
                {
                    scene.primitive(
                        Primitive::Text {
                            x: layer.control.bounds.x + layer.control.bounds.w / 2.,
                            y: layer.control.bounds.y + layer.control.bounds.h / 2.,
                            text: "Image unavailable".into(),
                            size: 13.,
                            bold: false,
                            color: [130, 35, 35],
                            center: true,
                        },
                        layer.clip,
                    );
                }
            }
            if scene.items.len() + scene.links.len() > 1_000_000 {
                return Err("This page exceeds the drawing complexity limit".into());
            }
        }
        Ok(scene)
    }
    fn rectangle_frame(&mut self, b: Bounds, color: [u8; 3], width: f32, clip: Option<Bounds>) {
        let inset = (width / 2.).min(b.w / 2.).min(b.h / 2.);
        self.primitive(
            Primitive::Line {
                points: vec![
                    [b.x + inset, b.y + inset],
                    [b.x + b.w - inset, b.y + inset],
                    [b.x + b.w - inset, b.y + b.h - inset],
                    [b.x + inset, b.y + b.h - inset],
                    [b.x + inset, b.y + inset],
                ],
                color,
                width,
            },
            clip,
        );
    }
    fn primitive(&mut self, p: Primitive, clip: Option<Bounds>) {
        self.items.push(Item {
            opacity: 1.,
            drawing: Drawing::Primitive(p),
            clip,
        });
    }
    fn control(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
        rasters: &mut rasters::Rasters<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let b = c.bounds;
        let missing =
            layer.properties.get(crate::document::MISSING_COMPONENT) == Some(&Value::Bool(true));
        if missing || c.group.is_some() {
            self.primitive(
                Primitive::Rect {
                    b,
                    fill: [255; 3],
                    stroke: true,
                    radius: 0.,
                },
                layer.clip,
            );
            for points in [
                vec![[b.x, b.y], [b.x + b.w, b.y + b.h]],
                vec![[b.x + b.w, b.y], [b.x, b.y + b.h]],
            ] {
                self.primitive(
                    Primitive::Line {
                        points,
                        color: [90; 3],
                        width: 2.,
                    },
                    layer.clip,
                );
            }
            self.primitive(
                Primitive::Rect {
                    b: Bounds::new(b.x + 4., b.y + b.h / 2. - 12., b.w - 8., 24.),
                    fill: [255; 3],
                    stroke: false,
                    radius: 3.,
                },
                layer.clip,
            );
            self.primitive(
                Primitive::Text {
                    x: b.x + b.w / 2.,
                    y: b.y + b.h / 2. + 5.,
                    text: if missing {
                        "Component not found"
                    } else if layer.properties.get(crate::document::EMPTY_COMPONENT)
                        == Some(&Value::Bool(true))
                    {
                        "Empty component"
                    } else {
                        "Empty group"
                    }
                    .into(),
                    size: 13.,
                    bold: false,
                    color: [44, 48, 55],
                    center: true,
                },
                layer.clip,
            );
            return Ok(());
        }
        if c.kind == Kind::Dropdown {
            return self.dropdown_control(layer, sketch, fonts, rasters);
        }
        if c.kind == Kind::Search {
            return self.search_control(layer, sketch, fonts);
        }
        if matches!(c.kind, Kind::Icon | Kind::IconLabel) {
            return self.standalone_icon(layer, fonts, rasters);
        }
        if c.kind == Kind::CircleButton {
            return self.circle_button(layer, sketch, fonts, rasters);
        }
        if c.kind == Kind::ButtonBar {
            return self.button_bar(layer, sketch, fonts);
        }
        if c.kind == Kind::MultilineButton {
            return self.multiline_button(layer, sketch, fonts, rasters);
        }
        if c.kind == Kind::PointedButton {
            return self.pointed_button(layer, sketch, fonts, rasters);
        }
        if c.kind == Kind::Button {
            return self.button_control(layer, sketch, fonts, rasters);
        }
        if c.kind == Kind::List {
            return self.list_control(layer, sketch, fonts, rasters);
        }
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
        if matches!(c.kind, Kind::Checkbox | Kind::Radio) {
            self.choice_control(layer);
        } else if matches!(c.kind, Kind::Button | Kind::TextInput | Kind::TextArea) {
            self.form_control(layer, sketch);
        } else if !text_only {
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
        let mut placement = crate::layout::text_placement(kind, sketch);
        if c.kind == Kind::TextInput
            && matches!(layer.properties.get("border"), Some(Value::Choice(s)) if s == "line")
        {
            placement.left = 0.;
        }
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(&layer.properties, "italic"),
        };
        let wrap = placement.wrap;
        let scrollbar =
            c.kind == Kind::TextArea && boolean(&layer.properties, "vertical_scrollbar");
        let area = Bounds::new(
            b.x + placement.left,
            b.y + placement.top,
            (b.w - placement.left - placement.right - if scrollbar { 16. } else { 0. }).max(1.),
            (b.h - placement.top - placement.bottom).max(1.),
        );
        let parsed = RichText::parse(&c.text, c.kind == Kind::Paragraph);
        let mut options = paint_options(&layer.properties);
        if c.kind == Kind::Button {
            forms::button_text_colors(layer, sketch, &mut options);
        }
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
                truncate: matches!(
                    c.kind,
                    Kind::Checkbox | Kind::Radio | Kind::Button | Kind::TextInput
                ),
                clip: clip_with(layer.clip, b),
            },
            fonts,
            options,
        )?;
        if c.kind == Kind::TextArea {
            self.form_border(layer, sketch);
        }
        if scrollbar {
            let inset = if sketch { 2. } else { 0. };
            self.vertical_scrollbar(
                Bounds::new(b.x + b.w - inset - 16., b.y + inset, 16., b.h - 2. * inset),
                number(&layer.properties, "value", 0.),
                matches!(layer.properties.get("state"), Some(Value::Choice(s)) if s == "disabled"),
                sketch,
                clip_with(layer.clip, b),
            );
        }
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
            (area.bounds.h - painted.size.y) / 2.
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
                row.size.x.ceil()
            }
        };
        let mut byte = 0;
        let mut section = 0;
        for (row_index, row) in galley.rows.iter().enumerate() {
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
                let baseline = dy + painted.rows[row_index].baseline;
                self.items.push(Item {
                    opacity: 1.,
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
            let row = galley.rows.get(link.row);
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
    let link = rgb(p, "link_color", [0, 102, 204]);
    PaintOptions {
        ink: if matches!(p.get("state"),Some(Value::Choice(s))if s.contains("disabled")) {
            Color32::from_rgb(128, 128, 139)
        } else {
            Color32::from_rgb(color[0], color[1], color[2])
        },
        link: if matches!(p.get("state"),Some(Value::Choice(s))if s.contains("disabled")) {
            Color32::from_rgb(128, 128, 139)
        } else {
            Color32::from_rgb(link[0], link[1], link[2])
        },
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
