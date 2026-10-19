// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

use super::{Affinity, Caret, Selection};
use crate::fonts::InterfaceRole;
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, Weight, Wrap};
use eframe::egui::{self, Color32, ColorImage, Pos2, Rect, Vec2};
use std::{
    collections::BTreeMap,
    ops::Range,
    sync::{Arc, Mutex},
};
use unicode_segmentation::UnicodeSegmentation;

/// Reuse the UI font/shaping engine for editable fields and read-only text.
pub fn shared_engine(ctx: &egui::Context) -> Arc<Mutex<Engine>> {
    let id = egui::Id::new("unicode-input-engine");
    ctx.data_mut(|d| d.get_temp::<Arc<Mutex<Engine>>>(id))
        .unwrap_or_else(|| {
            let engine = Arc::new(Mutex::new(Engine::default()));
            ctx.data_mut(|d| d.insert_temp(id, engine.clone()));
            engine
        })
}

pub struct Engine {
    fonts: FontSystem,
    raster: SwashCache,
    interface_bases: BTreeMap<InterfaceRole, cosmic_text::fontdb::ID>,
    interface_instances: BTreeMap<(InterfaceRole, u32), String>,
}

impl Default for Engine {
    fn default() -> Self {
        let mut fonts = FontSystem::new();
        // Use the existing UI face, with system fallback for other scripts.
        if let Some(data) = egui::FontDefinitions::default()
            .font_data
            .get("Ubuntu-Light")
        {
            fonts.db_mut().load_font_data(data.font.to_vec());
        }
        let mut interface_bases = BTreeMap::new();
        for role in [
            InterfaceRole::Regular,
            InterfaceRole::Medium,
            InterfaceRole::Bold,
        ] {
            if let Some(font) = crate::fonts::interface::system_font(role) {
                let ids = fonts
                    .db_mut()
                    .load_font_source(cosmic_text::fontdb::Source::Binary(font.bytes.clone()));
                if let Some(id) = ids.into_iter().find(|id| {
                    fonts
                        .db()
                        .face(*id)
                        .is_some_and(|face| face.index == font.index)
                }) {
                    interface_bases.insert(role, id);
                }
            }
        }
        Self {
            fonts,
            raster: SwashCache::new(),
            interface_bases,
            interface_instances: BTreeMap::new(),
        }
    }
}

impl Engine {
    fn interface_family(
        &mut self,
        size: f32,
        role: InterfaceRole,
    ) -> Result<Option<String>, String> {
        let Some(&base) = self.interface_bases.get(&role) else {
            return Ok(None);
        };
        let key = (role, size.to_bits());
        if let Some(family) = self.interface_instances.get(&key) {
            return Ok(Some(family.clone()));
        }
        if self.interface_instances.len() >= 128 {
            return Err("Too many interface font sizes".into());
        }
        let font =
            crate::fonts::interface::system_font(role).ok_or("Interface font unavailable")?;
        let family = format!("wireframe-interface-{role:?}-{:08x}", size.to_bits());
        self.fonts
            .add_font_instance(base, family.clone(), font.axes_at(size), size)
            .ok_or("Could not create interface font instance")?;
        self.interface_instances.insert(key, family.clone());
        Ok(Some(family))
    }
}

#[derive(Clone, Debug)]
pub struct Cell {
    pub source: Range<usize>,
    pub rect: Rect,
    pub rtl: bool,
}

impl Cell {
    fn leading(&self) -> f32 {
        if self.rtl {
            self.rect.right()
        } else {
            self.rect.left()
        }
    }
    fn trailing(&self) -> f32 {
        if self.rtl {
            self.rect.left()
        } else {
            self.rect.right()
        }
    }
    fn start(&self) -> Caret {
        Caret {
            byte: self.source.start,
            affinity: Affinity::Downstream,
        }
    }
    fn end(&self) -> Caret {
        Caret {
            byte: self.source.end,
            affinity: Affinity::Upstream,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Row {
    pub source: Range<usize>,
    pub rect: Rect,
    pub cells: Vec<Cell>,
    pub ending: Range<usize>,
}

#[derive(Clone, Debug)]
pub struct Layout {
    source: String,
    buffer: Buffer,
    pub rows: Vec<Row>,
    pub size: Vec2,
    scale: f32,
}

impl Layout {
    pub fn new(
        engine: &mut Engine,
        source: &str,
        font_size: f32,
        width: f32,
        scale: f32,
    ) -> Result<Self, String> {
        Self::with_wrap(engine, source, font_size, width, scale, true)
    }

    pub fn with_wrap(
        engine: &mut Engine,
        source: &str,
        font_size: f32,
        width: f32,
        scale: f32,
        wrap: bool,
    ) -> Result<Self, String> {
        Self::with_style(
            engine,
            source,
            font_size,
            width,
            scale,
            wrap,
            InterfaceRole::Regular,
        )
    }

    pub fn with_style(
        engine: &mut Engine,
        source: &str,
        font_size: f32,
        width: f32,
        scale: f32,
        wrap: bool,
        role: InterfaceRole,
    ) -> Result<Self, String> {
        if source.len() > 1024 * 1024 {
            return Err("Text exceeds the editor's 1 MiB layout limit".into());
        }
        if !font_size.is_finite()
            || !(1.0..=128.0).contains(&font_size)
            || !width.is_finite()
            || width <= 0.0
            || width > 16384.0
            || !scale.is_finite()
            || !(0.5..=8.0).contains(&scale)
        {
            return Err("Invalid text editor dimensions".into());
        }
        let interface_family = engine.interface_family(font_size, role)?;
        let mut buffer = Buffer::new(
            &mut engine.fonts,
            Metrics::new(font_size * scale, font_size * 1.4 * scale),
        );
        buffer.set_wrap(if wrap { Wrap::WordOrGlyph } else { Wrap::None });
        buffer.set_size(wrap.then_some(width * scale), None);
        buffer.set_text(
            source,
            &Attrs::new()
                .family(Family::Name(
                    interface_family.as_deref().unwrap_or("Ubuntu"),
                ))
                .weight(if interface_family.is_some() {
                    Weight::NORMAL
                } else {
                    Weight::LIGHT
                }),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut engine.fonts, false);
        let mut offset = 0;
        let lines: Vec<_> = buffer
            .lines
            .iter()
            .map(|line| {
                let start = offset;
                offset += line.text().len() + line.ending().as_str().len();
                (start, start + line.text().len(), offset)
            })
            .collect();
        let reconstructed: String = buffer
            .lines
            .iter()
            .flat_map(|line| [line.text(), line.ending().as_str()])
            .collect();
        if reconstructed != source {
            return Err("Text layout changed the source".into());
        }
        let boundaries: Vec<_> = source
            .grapheme_indices(true)
            .map(|(i, g)| i..i + g.len())
            .collect();
        let mut rows = Vec::new();
        let mut row_lines = Vec::new();
        for run in buffer.layout_runs() {
            let (start, end, after) = lines[run.line_i];
            row_lines.push((start, end, after, run.rtl));
            let top = run.line_top / scale;
            let bottom = top + run.line_height / scale;
            let mut cells: BTreeMap<usize, Cell> = BTreeMap::new();
            for glyph in run.glyphs {
                let lo = start + glyph.start;
                let hi = start + glyph.end;
                let first = boundaries.partition_point(|r| r.end <= lo);
                let last = boundaries.partition_point(|r| r.start < hi);
                if first >= last {
                    continue;
                }
                let part_width = glyph.w / (last - first) as f32;
                for (part, range) in boundaries[first..last].iter().enumerate() {
                    let visual_part = if glyph.level.is_rtl() {
                        last - first - 1 - part
                    } else {
                        part
                    };
                    let left = (glyph.x + visual_part as f32 * part_width) / scale;
                    let rect = Rect::from_min_max(
                        Pos2::new(left, top),
                        Pos2::new(left + part_width / scale, bottom),
                    );
                    cells
                        .entry(range.start)
                        .and_modify(|cell| cell.rect = cell.rect.union(rect))
                        .or_insert(Cell {
                            source: range.clone(),
                            rect,
                            rtl: glyph.level.is_rtl(),
                        });
                }
            }
            let logical_start = cells.values().next().map_or(start, |c| c.source.start);
            let logical_end = cells.values().next_back().map_or(start, |c| c.source.end);
            let mut cells: Vec<_> = cells.into_values().collect();
            cells.sort_by(|a, b| {
                a.rect
                    .left()
                    .total_cmp(&b.rect.left())
                    .then(a.source.start.cmp(&b.source.start))
            });
            rows.push(Row {
                source: logical_start..logical_end,
                rect: Rect::from_min_max(Pos2::new(0., top), Pos2::new(run.line_w / scale, bottom)),
                cells,
                ending: if logical_end == end {
                    end..after
                } else {
                    logical_end..logical_end
                },
            });
        }
        // A soft wrap can trim whitespace from the glyph stream. Retain its
        // original range at the line edge; never derive source length from ink.
        for i in 0..rows.len() {
            let (start, end, after, rtl) = row_lines[i];
            let lo = if i == 0 || row_lines[i - 1].0 != start {
                start
            } else {
                rows[i].source.start
            };
            let hi = if i + 1 < rows.len() && row_lines[i + 1].0 == start {
                rows[i + 1].source.start
            } else {
                end
            };
            let row = &mut rows[i];
            let first = boundaries.partition_point(|r| r.end <= lo);
            let last = boundaries.partition_point(|r| r.start < hi);
            let mut source_positions: BTreeMap<_, _> = row
                .cells
                .iter()
                .map(|c| (c.source.start, c.trailing()))
                .collect();
            for range in &boundaries[first..last] {
                if !source_positions.contains_key(&range.start) {
                    let x = source_positions.range(..range.start).next_back().map_or(
                        if rtl {
                            row.rect.left()
                        } else {
                            row.rect.right()
                        },
                        |(_, x)| *x,
                    );
                    row.cells.push(Cell {
                        source: range.clone(),
                        rect: Rect::from_min_max(
                            Pos2::new(x, row.rect.top()),
                            Pos2::new(x, row.rect.bottom()),
                        ),
                        rtl,
                    });
                    source_positions.insert(range.start, x);
                }
            }
            row.cells.sort_by(|a, b| {
                a.rect
                    .left()
                    .total_cmp(&b.rect.left())
                    .then(a.source.start.cmp(&b.source.start))
            });
            row.source = lo..hi;
            row.ending = if hi == end { end..after } else { hi..hi };
        }
        let height = rows.last().map_or(font_size * 1.4, |r| r.rect.bottom());
        let content_width = rows.iter().map(|r| r.rect.right()).fold(width, f32::max);
        Ok(Self {
            source: source.into(),
            buffer,
            rows,
            size: Vec2::new(content_width, height),
            scale,
        })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// Return the requested side of a bidi or wrapped boundary, falling back
    /// to the other side only where there is no adjacent source grapheme.
    pub fn caret_rect(&self, caret: Caret) -> Option<Rect> {
        let mut fallback = None;
        for row in &self.rows {
            for cell in &row.cells {
                for (candidate, x) in [
                    (cell.start(), cell.leading()),
                    (cell.end(), cell.trailing()),
                ] {
                    if candidate.byte == caret.byte {
                        let rect = Rect::from_min_max(
                            Pos2::new(x, row.rect.top()),
                            Pos2::new(x, row.rect.bottom()),
                        );
                        if candidate.affinity == caret.affinity {
                            return Some(rect);
                        }
                        fallback = Some(rect);
                    }
                }
            }
            if row.cells.is_empty() && row.source.start == caret.byte {
                fallback = Some(Rect::from_min_max(
                    row.rect.min,
                    Pos2::new(row.rect.left(), row.rect.bottom()),
                ));
            }
        }
        fallback
    }

    pub fn hit(&self, point: Pos2) -> Caret {
        let Some(row) = self.rows.iter().min_by(|a, b| {
            vertical_distance(a.rect, point.y).total_cmp(&vertical_distance(b.rect, point.y))
        }) else {
            return Caret::new(0);
        };
        Self::hit_row(row, point.x)
    }

    fn hit_row(row: &Row, x: f32) -> Caret {
        let stops = || {
            row.cells
                .iter()
                .flat_map(|c| [(c.start(), c.leading()), (c.end(), c.trailing())])
        };
        if let (Some(left), Some(right)) = (
            stops().min_by(|a, b| a.1.total_cmp(&b.1)),
            stops().max_by(|a, b| a.1.total_cmp(&b.1)),
        ) {
            if x < left.1 {
                return left.0;
            }
            if x > right.1 {
                return right.0;
            }
        }
        if let Some(cell) = row
            .cells
            .iter()
            .find(|c| x >= c.rect.left() && x < c.rect.right())
        {
            return if (x < cell.rect.center().x) != cell.rtl {
                cell.start()
            } else {
                cell.end()
            };
        }
        row.cells
            .iter()
            .flat_map(|c| [(c.start(), c.leading()), (c.end(), c.trailing())])
            .min_by(|a, b| (a.1 - x).abs().total_cmp(&(b.1 - x).abs()))
            .map_or(Caret::new(row.source.start), |v| v.0)
    }

    pub fn selection_rects(&self, selection: Selection) -> Vec<Rect> {
        let selected = selection.range();
        if selected.is_empty() {
            return Vec::new();
        }
        let mut rects: Vec<Rect> = Vec::new();
        for row in &self.rows {
            for cell in &row.cells {
                if cell.source.start < selected.end && cell.source.end > selected.start {
                    if let Some(last) = rects.last_mut()
                        && last.top() == cell.rect.top()
                        && cell.rect.left() <= last.right() + 0.1
                    {
                        *last = last.union(cell.rect);
                    } else {
                        rects.push(cell.rect);
                    }
                }
            }
            if !row.ending.is_empty()
                && selected.start < row.ending.end
                && selected.end > row.ending.start
            {
                let x = self
                    .caret_rect(Caret::new(row.ending.start))
                    .map_or(row.rect.right(), |r| r.left());
                rects.push(Rect::from_min_size(
                    Pos2::new(x, row.rect.top()),
                    Vec2::new(row.rect.height() * 0.3, row.rect.height()),
                ));
            }
        }
        rects
    }

    pub fn horizontal(&self, caret: Caret, right: bool) -> Caret {
        let Some(pos) = self.caret_rect(caret) else {
            return caret;
        };
        let Some(index) = self.rows.iter().position(|r| r.rect.top() == pos.top()) else {
            return caret;
        };
        let row = &self.rows[index];
        let target = row
            .cells
            .iter()
            .flat_map(|c| [(c.start(), c.leading()), (c.end(), c.trailing())])
            .filter(|(_, x)| {
                if right {
                    *x > pos.left() + 0.01
                } else {
                    *x < pos.left() - 0.01
                }
            })
            .min_by(|a, b| {
                (a.1 - pos.left())
                    .abs()
                    .total_cmp(&(b.1 - pos.left()).abs())
            });
        if let Some((caret, _)) = target {
            return caret;
        }
        let next = if right {
            index.checked_add(1)
        } else {
            index.checked_sub(1)
        };
        next.and_then(|i| self.rows.get(i)).map_or(caret, |r| {
            Self::hit_row(r, if right { -f32::MAX } else { f32::MAX })
        })
    }

    pub fn selection_edge(&self, selection: Selection, right: bool) -> Caret {
        let range = selection.range();
        let cells = self
            .rows
            .iter()
            .flat_map(|r| &r.cells)
            .filter(|c| c.source.start < range.end && c.source.end > range.start);
        let edge = cells.min_by(|a, b| {
            let order = a.rect.top().total_cmp(&b.rect.top()).then_with(|| {
                if right {
                    a.rect.right().total_cmp(&b.rect.right())
                } else {
                    a.rect.left().total_cmp(&b.rect.left())
                }
            });
            if right { order.reverse() } else { order }
        });
        edge.map_or(selection.focus, |c| {
            if right != c.rtl { c.end() } else { c.start() }
        })
    }

    pub fn vertical(&self, caret: Caret, down: bool, preferred_x: f32) -> Caret {
        let Some(pos) = self.caret_rect(caret) else {
            return caret;
        };
        let Some(index) = self.rows.iter().position(|r| r.rect.top() == pos.top()) else {
            return caret;
        };
        let next = if down {
            index.checked_add(1)
        } else {
            index.checked_sub(1)
        };
        next.and_then(|i| self.rows.get(i))
            .map_or(caret, |r| Self::hit_row(r, preferred_x))
    }

    /// Rasterize only the requested viewport. Geometry and raster use the same
    /// physical-scale buffer; selection and caret are painted separately.
    pub fn raster(
        &mut self,
        engine: &mut Engine,
        viewport: Rect,
        color: Color32,
    ) -> Result<ColorImage, String> {
        if !viewport.is_finite() || viewport.width() <= 0. || viewport.height() <= 0. {
            return Err("Invalid text viewport".into());
        }
        let width = (viewport.width() * self.scale).ceil() as usize;
        let height = (viewport.height() * self.scale).ceil() as usize;
        if width > 16384 || height > 16384 || width.saturating_mul(height) > 8 * 1024 * 1024 {
            return Err("Text viewport exceeds raster limit".into());
        }
        let mut image = ColorImage::filled([width, height], Color32::TRANSPARENT);
        let left = (viewport.left() * self.scale).round() as i32;
        let top = (viewport.top() * self.scale).round() as i32;
        self.buffer.draw(
            &mut engine.fonts,
            &mut engine.raster,
            cosmic_text::Color::rgba(color.r(), color.g(), color.b(), color.a()),
            |x, y, w, h, c| {
                let [r, g, b, a] = c.as_rgba();
                let ink = Color32::from_rgba_unmultiplied(r, g, b, a);
                for py in (y - top).max(0)..(y - top + h as i32).min(height as i32) {
                    for px in (x - left).max(0)..(x - left + w as i32).min(width as i32) {
                        let pixel = &mut image.pixels[py as usize * width + px as usize];
                        *pixel = ink + pixel.gamma_multiply(1. - a as f32 / 255.);
                    }
                }
            },
        );
        Ok(image)
    }
}

fn vertical_distance(rect: Rect, y: f32) -> f32 {
    if y < rect.top() {
        rect.top() - y
    } else if y > rect.bottom() {
        y - rect.bottom()
    } else {
        0.
    }
}
