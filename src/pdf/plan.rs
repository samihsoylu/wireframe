// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Project selection, page geometry and document decoration, independent of UI and I/O.
use super::{Destination, Link, Page};
use crate::{
    document::{Id, Project},
    layout::{EguiMeasurer, TextMeasurer, TextStyle},
    model::Bounds,
    render::{Alignment, Drawing, Item, Scene, TextBox},
    rich_text::{RichText, painting::PaintOptions},
    scene::Primitive,
    workspace::Editor,
};
use eframe::egui;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq)]
pub enum Selection {
    All,
    Current(Id),
    Set(BTreeSet<Id>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paper {
    A3,
    A4,
    A5,
    Letter,
    Legal,
    Tabloid,
}
impl Paper {
    pub const ALL: [Self; 6] = [
        Self::A3,
        Self::A4,
        Self::A5,
        Self::Letter,
        Self::Legal,
        Self::Tabloid,
    ];
    pub fn points(self) -> [f32; 2] {
        match self {
            Self::A3 => [841.8898, 1190.5511],
            Self::A4 => [595.2756, 841.8898],
            Self::A5 => [419.5276, 595.2756],
            Self::Letter => [612., 792.],
            Self::Legal => [612., 1008.],
            Self::Tabloid => [792., 1224.],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Portrait,
    Landscape,
    BestFit,
}
#[derive(Clone, Debug)]
pub struct Options {
    pub selection: Selection,
    pub include_alternates: bool,
    pub paper: Option<Paper>,
    pub orientation: Orientation,
    pub margins: bool,
    pub names: bool,
    pub numbers: bool,
    pub notes: bool,
    pub hints: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            selection: Selection::All,
            include_alternates: false,
            paper: None,
            orientation: Orientation::BestFit,
            margins: true,
            names: false,
            numbers: false,
            notes: false,
            hints: false,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Entry {
    pub id: Id,
    pub owner: Id,
    pub name: String,
    pub alternate: bool,
}
pub fn entries(project: &Project) -> Vec<Entry> {
    project
        .pages
        .iter()
        .flat_map(|p| {
            std::iter::once(Entry {
                id: p.id,
                owner: p.id,
                name: p.name.clone(),
                alternate: false,
            })
            .chain(p.alternates.iter().map(|a| Entry {
                id: a.id,
                owner: p.id,
                name: format!("{} — {}", p.name, a.name),
                alternate: true,
            }))
        })
        .collect()
}
fn selected(project: &Project, options: &Options) -> Result<Vec<Entry>, String> {
    let all = entries(project);
    let requested: Vec<_> = match &options.selection {
        Selection::All => vec![],
        Selection::Current(id) => vec![*id],
        Selection::Set(ids) => ids.iter().copied().collect(),
    };
    if requested.iter().any(|id| !all.iter().any(|e| e.id == *id)) {
        return Err("An export page is no longer available. Choose the pages again.".into());
    }
    let result: Vec<_> = all
        .into_iter()
        .filter(|e| match &options.selection {
            Selection::All => !e.alternate || options.include_alternates,
            Selection::Current(id) => e.id == *id,
            Selection::Set(ids) => ids.contains(&e.id),
        })
        .collect();
    if result.is_empty() || result.len() > 1000 {
        return Err("Choose between 1 and 1000 pages to export.".into());
    }
    Ok(result)
}
fn target(target: &str, chosen: &[Entry]) -> Option<Destination> {
    if target.starts_with("https://") || target.starts_with("http://") {
        return Some(Destination::Web(target.into()));
    }
    let id = target.strip_prefix("page:")?.parse::<Id>().ok()?;
    chosen
        .iter()
        .position(|e| e.id == id)
        .or_else(|| chosen.iter().position(|e| e.owner == id))
        .map(Destination::Page)
}
fn empty_scene(width: f32, height: f32) -> Scene {
    Scene {
        title: String::new(),
        bounds: Bounds::new(0., 0., width, height),
        items: vec![],
        links: vec![],
        font: None,
    }
}
fn text(
    scene: &mut Scene,
    fonts: &EguiMeasurer<'_>,
    value: &str,
    bounds: Bounds,
    size: f32,
    align: Alignment,
    truncate: bool,
) -> Result<(), String> {
    scene.text(
        &RichText::plain(value),
        TextStyle {
            size,
            bold: false,
            italic: false,
        },
        TextBox {
            bounds,
            align,
            middle: false,
            wrap: !truncate,
            soft_breaks: true,
            truncate,
            clip: Some(bounds),
        },
        fonts,
        PaintOptions::default(),
    )
}
/// Render an immutable project snapshot. May run entirely on an export worker.
pub fn plan(project: &Project, options: &Options) -> Result<Vec<Page>, String> {
    project.validate()?;
    let chosen = selected(project, options)?;
    let mut snapshot = project.clone();
    snapshot.pages = chosen
        .iter()
        .map(|entry| {
            let owner = project.pages.iter().find(|p| p.id == entry.owner).unwrap();
            let content = if entry.alternate {
                owner
                    .alternates
                    .iter()
                    .find(|a| a.id == entry.id)
                    .unwrap()
                    .content
                    .clone()
            } else {
                owner.content.clone()
            };
            crate::document::Page {
                id: entry.id,
                name: entry.name.clone(),
                notes: owner.notes.clone(),
                parent: None,
                content,
                alternates: vec![],
            }
        })
        .collect();
    let editor = Editor::new(snapshot);
    let ctx = egui::Context::default();
    crate::fonts::install(&ctx);
    let mut result = Err("Could not initialize PDF text layout".into());
    let output = ctx.run_ui(egui::RawInput::default(), |ui| {
        let fonts = EguiMeasurer {
            context: ui.ctx(),
            regular: egui::FontFamily::Proportional,
            bold: egui::FontFamily::Proportional,
        };
        result = layout_pages(project, options, &chosen, &editor, &fonts);
    });
    output.drop_without_applying_deltas();
    result
}

fn layout_pages(
    project: &Project,
    options: &Options,
    chosen: &[Entry],
    editor: &Editor,
    fonts: &EguiMeasurer<'_>,
) -> Result<Vec<Page>, String> {
    let mut pages = Vec::new();
    for (index, entry) in chosen.iter().enumerate() {
        let scene = editor.rendered_scene(entry.id)?;
        let margin = if options.paper.is_some() || options.margins {
            10.
        } else {
            0.
        };
        let header = if options.names || options.numbers {
            26.
        } else {
            0.
        };
        let mut size = options.paper.map_or(
            [
                scene.bounds.w.max(1.) + 2. * margin,
                scene.bounds.h.max(1.) + 2. * margin,
            ],
            Paper::points,
        );
        if options.paper.is_some()
            && (options.orientation == Orientation::Landscape
                || options.orientation == Orientation::BestFit && scene.bounds.w > scene.bounds.h)
        {
            size.swap(0, 1);
        }
        let width = (size[0] - 2. * margin).max(1.);
        let owner = project.pages.iter().find(|p| p.id == entry.owner).unwrap();
        let note = if options.notes && !owner.notes.trim().is_empty() {
            owner.notes.as_str()
        } else {
            ""
        };
        let note_style = TextStyle {
            size: 12.,
            bold: false,
            italic: false,
        };
        let header_style = TextStyle {
            size: 13.,
            bold: false,
            italic: false,
        };
        let note_height = if note.is_empty() {
            0.
        } else {
            fonts.measure(note, note_style, Some(width)).height
        };
        let gap = if note.is_empty() { 0. } else { 10. };
        let scale = if options.paper.is_some() {
            let height = size[1] - 2. * margin - header - gap - note_height;
            if height < 1. {
                return Err(format!(
                    "Notes for ‘{}’ do not fit on this paper. Choose Content sized pages, larger paper, or turn off Notes.",
                    entry.name
                ));
            }
            (width / scene.bounds.w.max(1.))
                .min(height / scene.bounds.h.max(1.))
                .min(1.)
        } else {
            size[1] += header + gap + note_height;
            1.
        };
        let origin = [margin, margin + header];
        let mut decoration = empty_scene(size[0], size[1]);
        if header > 0. {
            let counter = if options.numbers {
                format!("{} / {}", index + 1, chosen.len())
            } else {
                String::new()
            };
            let counter_width = if counter.is_empty() {
                0.
            } else {
                fonts.measure(&counter, header_style, None).width.min(width)
            };
            if options.names {
                let available =
                    (width - counter_width - if counter.is_empty() { 0. } else { 12. }).max(1.);
                text(
                    &mut decoration,
                    fonts,
                    &entry.name,
                    Bounds::new(margin, margin, available, 20.),
                    13.,
                    Alignment::Left,
                    true,
                )?;
            }
            if !counter.is_empty() {
                text(
                    &mut decoration,
                    fonts,
                    &counter,
                    Bounds::new(margin + width - counter_width, margin, counter_width, 20.),
                    13.,
                    Alignment::Right,
                    true,
                )?;
            }
        }
        if !note.is_empty() {
            text(
                &mut decoration,
                fonts,
                note,
                Bounds::new(
                    margin,
                    origin[1] + scene.bounds.h.max(1.) * scale + gap,
                    width,
                    note_height,
                ),
                12.,
                Alignment::Left,
                false,
            )?;
        }
        let links: Vec<_> = scene
            .links
            .iter()
            .filter_map(|l| {
                target(&l.target, chosen).map(|destination| Link {
                    bounds: l.bounds,
                    destination,
                })
            })
            .collect();
        if options.hints {
            let mut marked = BTreeSet::new();
            for link in &scene.links {
                if target(&link.target, chosen).is_none() {
                    continue;
                }
                let b = if options.paper.is_some() {
                    let b = link.hint;
                    let key = (
                        link.target.as_str(),
                        [b.x.to_bits(), b.y.to_bits(), b.w.to_bits(), b.h.to_bits()],
                    );
                    if !marked.insert(key) {
                        continue;
                    }
                    b
                } else {
                    link.bounds
                };
                let b = Bounds::new(
                    origin[0] + (b.x - scene.bounds.x) * scale,
                    origin[1] + (b.y - scene.bounds.y) * scale,
                    b.w * scale,
                    b.h * scale,
                );
                let drawing = if options.paper.is_none() {
                    Primitive::Rect {
                        b,
                        fill: [0, 102, 204],
                        stroke: false,
                        radius: 0.,
                    }
                } else {
                    let width = 8_f32.min(b.w);
                    let half = 3_f32.min(b.h / 2.);
                    let x = b.x + b.w;
                    let y = b.y + b.h - half;
                    Primitive::Line {
                        points: vec![
                            [x - width, y],
                            [x, y],
                            [x - half, y - half],
                            [x, y],
                            [x - half, y + half],
                        ],
                        color: [153; 3],
                        width: 1.,
                    }
                };
                decoration.items.push(Item {
                    drawing: Drawing::Primitive(drawing),
                    opacity: if options.paper.is_none() { 0.15 } else { 1. },
                    clip: Some(Bounds::new(0., 0., size[0], size[1])),
                });
            }
        }
        pages.push(Page {
            scene,
            decoration: Some(decoration),
            size,
            origin,
            scale,
            links,
        });
    }
    Ok(pages)
}
