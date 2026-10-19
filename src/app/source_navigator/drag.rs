// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

pub(in crate::app) struct SourceDrag {
    point: Pos2,
    sources: Vec<Id>,
    session: String,
    surface: Id,
    owners: Vec<(Id, Id)>,
    frame: u64,
    moving: bool,
}
impl WireframeApp {
    fn move_source_resources(&mut self, sources: &[Id], library: Id) {
        if !self.finish_page_notes() {
            return;
        }
        self.end_pointer_gestures();
        match self.editor.move_sources_to_library(sources, library) {
            Ok(()) => {
                self.collapse_resource_selection();
                self.source_navigation.focus_to = self.editor.definition;
                self.source_navigation.reveal_library = Some(library);
            }
            Err(error) => self.error = Some(error),
        }
    }
    pub(super) fn source_move_menu(&mut self, ui: &mut egui::Ui, source: Id) {
        let sources: Vec<_> = self
            .resource_selection(ResourceRow::Source(source))
            .into_iter()
            .filter_map(ResourceRow::source)
            .collect();
        let choices: Vec<_> = self
            .editor
            .document
            .live_libraries()
            .filter(|l| {
                sources
                    .iter()
                    .any(|id| !l.sources.iter().any(|s| s.definition == *id))
            })
            .map(|l| (l.id, l.name.clone()))
            .collect();
        ui.add_enabled_ui(!choices.is_empty(), |ui| {
            ui.menu_button("Move to library", |ui| {
                for (id, name) in choices {
                    ui.push_id(id, |ui| {
                        if ui.button(name).clicked() {
                            self.move_source_resources(&sources, id);
                            ui.close();
                        }
                    });
                }
            });
        });
    }
    fn source_owners(&self) -> Vec<(Id, Id)> {
        self.editor
            .document
            .live_libraries()
            .flat_map(|l| l.sources.iter().map(move |s| (s.definition, l.id)))
            .collect()
    }
    pub(super) fn source_drag(&mut self, ui: &mut egui::Ui, rows: &[(ResourceRow, Rect)]) {
        let frame = ui.ctx().cumulative_frame_nr();
        let cancelled = !ui.is_enabled()
            || self.canvas_input_blocked()
            || ui.input(|i| {
                !i.focused
                    || i.key_pressed(Key::Escape)
                    || i.events
                        .iter()
                        .any(|e| matches!(e, egui::Event::PointerGone))
            });
        if cancelled {
            self.source_navigation.drag = None;
            return;
        }
        let owners = self.source_owners();
        if self.source_navigation.drag.as_ref().is_some_and(|d| {
            d.frame.saturating_add(1) < frame
                || d.owners != owners
                || d.session != self.editor.clipboard_session()
                || d.surface != self.editor.surface_id()
        }) {
            self.source_navigation.drag = None;
        }
        let threshold = ui.ctx().options(|o| o.input_options.max_click_dist);
        let mut release = None;
        for event in ui.input(|i| i.events.clone()) {
            match event {
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    ..
                } => {
                    self.source_navigation.drag = None;
                    if ui.ctx().layer_id_at(pos) != Some(ui.layer_id()) {
                        continue;
                    }
                    if let Some((ResourceRow::Source(source), _)) =
                        rows.iter().find(|(_, r)| r.contains(pos))
                    {
                        let sources = self
                            .resource_selection(ResourceRow::Source(*source))
                            .into_iter()
                            .filter_map(ResourceRow::source)
                            .collect();
                        self.source_navigation.drag = Some(SourceDrag {
                            point: pos,
                            sources,
                            session: self.editor.clipboard_session().into(),
                            surface: self.editor.surface_id(),
                            owners: owners.clone(),
                            frame,
                            moving: false,
                        });
                    }
                }
                egui::Event::PointerMoved(pos) => {
                    if let Some(d) = &mut self.source_navigation.drag {
                        d.moving |= pos.distance(d.point) > threshold;
                    }
                }
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    ..
                } => {
                    if let Some(d) = &mut self.source_navigation.drag {
                        d.moving |= pos.distance(d.point) > threshold;
                        release = Some(pos);
                    }
                }
                _ => {}
            }
        }
        let Some(drag) = &mut self.source_navigation.drag else {
            return;
        };
        drag.frame = frame;
        let point = release.or_else(|| ui.input(|i| i.pointer.interact_pos()));
        let target = point
            .filter(|p| ui.clip_rect().contains(*p))
            .and_then(|p| rows.iter().find(|(_, r)| r.contains(p)))
            .and_then(|(row, rect)| {
                let library = match row {
                    ResourceRow::Library(id) => Some(*id),
                    ResourceRow::Source(id) => {
                        owners.iter().find(|(s, _)| s == id).map(|(_, l)| *l)
                    }
                }?;
                drag.sources
                    .iter()
                    .any(|s| owners.iter().any(|(id, l)| id == s && *l != library))
                    .then_some((library, *rect))
            });
        if release.is_some() {
            let drag = self.source_navigation.drag.take().unwrap();
            if drag.moving
                && let Some((library, _)) = target
            {
                self.move_source_resources(&drag.sources, library);
            }
            return;
        }
        if !ui.input(|i| i.pointer.primary_down()) {
            self.source_navigation.drag = None;
            return;
        }
        if drag.moving {
            ui.ctx().set_cursor_icon(if target.is_some() {
                egui::CursorIcon::Grabbing
            } else {
                egui::CursorIcon::NotAllowed
            });
            if let Some(point) = point {
                let caption = if drag.sources.len() == 1 {
                    self.editor
                        .document
                        .definitions
                        .iter()
                        .find(|d| d.id == drag.sources[0])
                        .map(|d| d.name.clone())
                        .unwrap_or_else(|| "Component".into())
                } else {
                    format!("{} components", drag.sources.len())
                };
                let painter = ui.ctx().layer_painter(egui::LayerId::new(
                    egui::Order::Tooltip,
                    egui::Id::new("source-transfer-preview"),
                ));
                let galley = painter.layout_no_wrap(
                    caption,
                    egui::FontId::proportional(12.),
                    Color32::WHITE,
                );
                let rect = Rect::from_min_size(
                    point + Vec2::new(12., 14.),
                    galley.size() + Vec2::new(16., 10.),
                );
                painter.rect_filled(rect, 3., Color32::from_rgb(45, 90, 125));
                painter.galley(rect.min + Vec2::new(8., 5.), galley, Color32::WHITE);
                let viewport = ui.clip_rect();
                if viewport.contains(point) {
                    let speed = if point.y < viewport.top() + 24. {
                        240.
                    } else if point.y > viewport.bottom() - 24. {
                        -240.
                    } else {
                        0.
                    };
                    if speed != 0. {
                        ui.scroll_with_delta(Vec2::new(
                            0.,
                            speed * ui.input(|i| i.stable_dt.min(0.05)),
                        ));
                        ui.ctx().request_repaint();
                    }
                }
            }
            if let Some((_, rect)) = target {
                ui.painter().rect_stroke(
                    rect,
                    2.,
                    Stroke::new(2., Color32::from_rgb(35, 135, 210)),
                    egui::StrokeKind::Inside,
                );
            }
        }
    }
}
