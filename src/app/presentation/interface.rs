// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::collections::BTreeSet;
use wireframe::presentation::{PageExtent, Preferences, Session, Settings, centered_view};

const SAVE_FAILURE_NOTICE: &str =
    "Couldn't save presentation settings. Your choices still work until you close Wireframe.";

#[derive(Default)]
pub(crate) struct State {
    pub session: Session,
    pub preferences: Preferences,
    save_error: Option<String>,
    pub settings_open: bool,
    pub exclusions: Option<BTreeSet<Id>>,
    pub(super) thumbnails: bool,
    key: Option<String>,
    fullscreen_prior: Option<bool>,
    document: Option<Project>,
    extents: Vec<PageExtent>,
    framed: Option<(Id, Settings, Vec2)>,
    press: Option<(Pos2, Vec2, f64)>,
    hints: hints::Hints,
}
impl WireframeApp {
    pub(in crate::app) fn initialize_presentation(&mut self) {
        self.presentation_view.key = match self.export_preferences_key() {
            Ok(key) => key,
            Err(error) => {
                self.status = error;
                None
            }
        };
        let settings = self
            .presentation_view
            .key
            .as_deref()
            .map(|key| self.presentation_view.preferences.get(key))
            .unwrap_or_default();
        self.presentation_view.session = Session::new(settings).unwrap_or_default();
        self.presentation_view.settings_open = true;
        self.presentation_view.exclusions = None;
        self.presentation_view.framed = None;
        self.presentation_view.document = None;
        self.presentation_view.press = None;
        self.presentation_view.hints = hints::Hints::default();
    }
    pub(in crate::app) fn remember_presentation(&mut self) {
        if let Some(key) = &self.presentation_view.key {
            self.presentation_view.save_error = self
                .presentation_view
                .preferences
                .remember(key, self.presentation_view.session.settings())
                .err();
        }
    }
    pub(in crate::app) fn presentation_save_status(&self) -> Option<&'static str> {
        self.presentation_view
            .save_error
            .as_ref()
            .map(|_| SAVE_FAILURE_NOTICE)
    }
    fn presentation_save_notice(&self, ui: &mut egui::Ui) {
        let Some(error) = &self.presentation_view.save_error else {
            return;
        };
        ui.label(SAVE_FAILURE_NOTICE);
        ui.collapsing("Details", |ui| {
            egui::ScrollArea::vertical().max_height(84.).show(ui, |ui| {
                ui.add(egui::Label::new(error).wrap());
            });
        });
    }
    pub(in crate::app) fn presentation_toolbar(&mut self, ui: &mut egui::Ui) {
        if self.presentation_view.fullscreen_prior.is_none() {
            self.presentation_view.fullscreen_prior =
                Some(ui.input(|i| i.viewport().fullscreen.unwrap_or(false)));
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        }
        if !self.presentation_view.settings_open && self.presentation_view.save_error.is_some() {
            egui::Panel::bottom("presentation-save-notice")
                .frame(
                    egui::Frame::NONE
                        .fill(chrome::PANEL)
                        .inner_margin(egui::Margin::symmetric(12, 8)),
                )
                .show(ui, |ui| self.presentation_save_notice(ui));
        }
        let settings = self.presentation_view.session.settings().clone();
        if settings.navigator {
            let id = egui::Id::new((
                "presentation-navigator",
                &self.presentation_view.key,
                settings.navigator_width.to_bits(),
            ));
            let fast_resize = chrome::prepare_panel_resize(
                ui,
                id,
                chrome::PanelEdge::Left,
                130. ..=600.,
                !self.presentation_view.settings_open,
            );
            let panel = egui::Panel::left(id)
                .default_size(settings.navigator_width)
                .resizable(true)
                .size_range(130. ..=600.)
                .frame(egui::Frame::NONE.fill(chrome::PANEL))
                .show(ui, |ui| {
                    if self.presentation_view.settings_open {
                        ui.disable();
                    }
                    self.presentation_navigator(ui);
                });
            let width = panel.response.rect.width();
            if (fast_resize || resized(ui, id)) && width != settings.navigator_width {
                let _ = self.presentation_view.session.set_panel_sizes(
                    width,
                    settings.side_width,
                    settings.notes_height,
                );
            }
        }
        if settings.notes || settings.comments {
            let id = egui::Id::new((
                "presentation-info",
                &self.presentation_view.key,
                settings.side_width.to_bits(),
            ));
            let fast_resize = chrome::prepare_panel_resize(
                ui,
                id,
                chrome::PanelEdge::Right,
                180. ..=650.,
                !self.presentation_view.settings_open,
            );
            let panel = egui::Panel::right(id)
                .default_size(settings.side_width)
                .resizable(true)
                .size_range(180. ..=650.)
                .frame(egui::Frame::NONE.fill(chrome::PANEL))
                .show(ui, |ui| {
                    if self.presentation_view.settings_open {
                        ui.disable();
                    }
                    ui.spacing_mut().item_spacing.y = 0.;
                    if settings.notes {
                        if settings.comments {
                            let id = egui::Id::new((
                                "presentation-notes",
                                &self.presentation_view.key,
                                settings.notes_height.to_bits(),
                            ));
                            let notes_max = (ui.available_height() - 340.).clamp(120., 700.);
                            let fast_resize = chrome::prepare_panel_resize(
                                ui,
                                id,
                                chrome::PanelEdge::Top,
                                120. ..=notes_max,
                                !self.presentation_view.settings_open,
                            );
                            let panel = egui::Panel::top(id)
                                .default_size(settings.notes_height.min(notes_max))
                                .resizable(true)
                                .size_range(120. ..=notes_max)
                                .frame(egui::Frame::NONE.fill(chrome::PANEL))
                                .show(ui, |ui| self.presentation_notes(ui));
                            if fast_resize || resized(ui, id) {
                                let now = self.presentation_view.session.settings().clone();
                                let _ = self.presentation_view.session.set_panel_sizes(
                                    now.navigator_width,
                                    now.side_width,
                                    panel.response.rect.height(),
                                );
                            }
                        } else {
                            self.presentation_notes(ui);
                        }
                    }
                    if settings.comments {
                        self.review_contents(ui);
                    }
                });
            if fast_resize || resized(ui, id) {
                let now = self.presentation_view.session.settings().clone();
                let _ = self.presentation_view.session.set_panel_sizes(
                    now.navigator_width,
                    panel.response.rect.width(),
                    now.notes_height,
                );
            }
        }
    }
    fn presentation_notes(&mut self, ui: &mut egui::Ui) {
        let heading = chrome::PRESENTATION_HEADING;
        let name = self.editor.page_data().name.clone();
        chrome::panel_header(ui, &name, heading, 24., |ui| {
            if chrome::close_button(ui, "Hide Notes", "Hide Notes (N)").clicked()
                && self.finish_page_notes()
            {
                self.presentation_view.session.toggle(Aid::Notes);
            }
        });
        self.notes_region(ui, ui.available_height().max(110.));
    }
    pub(super) fn presentation_versions(&mut self, ui: &mut egui::Ui) {
        for (forward, label) in [
            (false, "Previous including versions"),
            (true, "Next including versions"),
        ] {
            if ui
                .add_enabled(
                    self.adjacent_destination(forward, true).is_some(),
                    egui::Button::new(label),
                )
                .clicked()
            {
                self.navigate_adjacent(forward, true);
                ui.close();
            }
        }
        ui.separator();
        let page = self.editor.page;
        let choices: Vec<_> = std::iter::once((None, "Official version".to_owned()))
            .chain(
                self.editor
                    .page_data()
                    .alternates
                    .iter()
                    .map(|a| (Some(a.id), a.name.clone())),
            )
            .collect();
        for (id, name) in choices {
            if ui
                .selectable_label(self.editor.active_alternate(page) == id, name)
                .clicked()
            {
                self.visit_destination(Destination {
                    page,
                    alternate: id,
                });
                ui.close();
            }
        }
    }
    pub(in crate::app) fn presentation_dialog(&mut self, ctx: &egui::Context) {
        if !self.present {
            if let Some(fullscreen) = self.presentation_view.fullscreen_prior.take() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(fullscreen));
            }
            return;
        }
        if !self.presentation_view.settings_open {
            return;
        }
        if self.presentation_view.exclusions.is_some() {
            self.presentation_centering_dialog(ctx);
            return;
        }
        egui::Modal::new(egui::Id::new("presentation-settings")).show(ctx, |ui| {
            ui.set_width(460.);
            ui.heading("Full Screen Presentation Settings");
            ui.separator();
            ui.horizontal(|ui| {
                for (preset, label) in [
                    (Preset::Review, "Review"),
                    (Preset::UserTest, "User Test"),
                    (Preset::Custom, "Custom"),
                ] {
                    if ui
                        .selectable_label(
                            self.presentation_view.session.settings().preset == preset,
                            label,
                        )
                        .clicked()
                    {
                        self.presentation_view.session.select(preset);
                    }
                }
            });
            ui.add_space(12.);
            ui.columns(2, |cols| {
                for (col, items) in [
                    [
                        (Aid::Navigator, "Navigator", "J"),
                        (Aid::Notes, "Notes", "N"),
                        (Aid::Comments, "Comments", "C"),
                    ],
                    [
                        (Aid::LinkHints, "Link Hints", "L"),
                        (Aid::BigCursor, "Big Arrow Cursor", "B"),
                        (Aid::Markup, "Markup", "M"),
                    ],
                ]
                .into_iter()
                .enumerate()
                {
                    cols[col].horizontal(|ui| {
                        ui.strong("Show / Hide");
                        ui.label("Shortcut");
                    });
                    for (aid, label, key) in items {
                        cols[col].horizontal(|ui| {
                            let mut enabled = self.presentation_view.session.settings().aid(aid);
                            if ui.checkbox(&mut enabled, label).changed() {
                                self.presentation_view.session.set_aid(aid, enabled);
                            }
                            ui.label(key);
                        });
                    }
                }
            });
            ui.horizontal(|ui| {
                let label = if self.presentation_view.session.settings().zoom == Zoom::Fit {
                    "1:1 Zoom"
                } else {
                    "Fit Zoom"
                };
                if ui.button(label).on_hover_text("Toggle zoom (Z)").clicked() {
                    self.presentation_view.session.toggle_zoom();
                }
                if ui.link("Advanced…").clicked() {
                    self.presentation_view.exclusions =
                        Some(self.presentation_view.session.settings().excluded.clone());
                }
            });
            if self.presentation_view.save_error.is_some() {
                ui.add_space(12.);
                self.presentation_save_notice(ui);
            } else {
                ui.add_space(65.);
            }
            ui.separator();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close Settings").clicked() {
                    self.presentation_view.settings_open = false;
                }
            });
        });
    }
    fn presentation_centering_dialog(&mut self, ctx: &egui::Context) {
        let mut excluded = self.presentation_view.exclusions.take().unwrap_or_default();
        let mut close = false;
        egui::Modal::new(egui::Id::new("presentation-centering")).show(ctx, |ui| {
            ui.set_width(460.);
            ui.heading("Project Centering Rules");
            self.presentation_save_notice(ui);
            ui.label(
                "Included pages share a common canvas size. Excluded pages center individually.",
            );
            let pages: Vec<_> = self
                .editor
                .document
                .live_pages()
                .flat_map(|p| {
                    std::iter::once((p.id, p.name.clone())).chain(
                        p.alternates
                            .iter()
                            .map(|a| (a.id, format!("{} — {}", p.name, a.name))),
                    )
                })
                .collect();
            ui.horizontal(|ui| {
                if ui.button("Select All").clicked() {
                    excluded.clear();
                }
                if ui.button("Select None").clicked() {
                    excluded.extend(pages.iter().map(|p| p.0));
                }
            });
            egui::ScrollArea::vertical()
                .max_height(320.)
                .show(ui, |ui| {
                    for (id, name) in pages {
                        let mut included = !excluded.contains(&id);
                        if ui.checkbox(&mut included, name).changed() {
                            if included {
                                excluded.remove(&id);
                            } else {
                                excluded.insert(id);
                            }
                        }
                    }
                });
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    close = true;
                }
                if ui.button("OK").clicked() {
                    match self
                        .presentation_view
                        .session
                        .set_excluded(excluded.clone())
                    {
                        Ok(()) => close = true,
                        Err(e) => self.error = Some(e),
                    }
                }
            });
        });
        if !close {
            self.presentation_view.exclusions = Some(excluded);
        }
    }
    pub(in crate::app) fn presentation_canvas(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        painter: &egui::Painter,
    ) {
        let settings = self.presentation_view.session.settings().clone();
        let surface = self.editor.surface_id();
        if self.presentation_view.document.as_ref() != Some(&self.editor.document) {
            let artwork_changed = self.presentation_view.document.as_ref().is_none_or(|old| {
                let mut artwork = self.editor.document.clone();
                artwork.reviews = old.reviews.clone();
                &artwork != old
            });
            if artwork_changed {
                match self.editor.presentation_extents() {
                    Ok(extents) => {
                        self.presentation_view.extents = extents;
                        self.presentation_view.framed = None;
                    }
                    Err(error) => {
                        self.status = error;
                        return;
                    }
                }
            }
            self.presentation_view.document = Some(self.editor.document.clone());
        }
        let size = response.rect.size();
        if self.presentation_view.framed.as_ref() != Some(&(surface, settings.clone(), size)) {
            match centered_view(
                &settings,
                &self.presentation_view.extents,
                surface,
                size.into(),
            ) {
                Ok(view) => {
                    self.presentation_view.press = None;
                    self.presentation_view.hints = hints::Hints::default();
                    self.zoom = view.zoom;
                    self.pan = Vec2::from(view.offset);
                    self.presentation_view.framed = Some((surface, settings.clone(), size));
                }
                Err(error) => {
                    self.status = error;
                    return;
                }
            }
        }
        let Ok(scene) = self.editor.presentation_scene(surface, settings.markup) else {
            return;
        };
        let review_capture = self.review_pointer(ui);
        let blocked = review_capture
            || self.presentation_view.settings_open
            || self.canvas_input_blocked()
            || !ui.input(|i| i.focused);
        let pointer = response
            .hover_pos()
            .or_else(|| response.interact_pointer_pos());
        let target = pointer.and_then(|p| {
            let p = self.world(p);
            scene.hit_link(p.x, p.y).map(str::to_owned)
        });
        let pressed_position = ui.input(|i| {
            i.events.iter().find_map(|event| {
                if let egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    ..
                } = event
                    && response.rect.contains(*pos)
                {
                    Some(*pos)
                } else {
                    None
                }
            })
        });
        let pressed_position =
            pressed_position.filter(|pos| ui.ctx().layer_id_at(*pos) == Some(response.layer_id));
        let pressed_target = pressed_position.and_then(|pos| {
            let point = self.world(pos);
            scene.hit_link(point.x, point.y).map(str::to_owned)
        });
        let hints_enabled = settings.link_hints
            && !blocked
            && !egui::Popup::is_any_open(ui.ctx())
            && !wireframe::text_entry::is_focused(ui.ctx())
            && !ui.ctx().text_edit_focused();
        let hints = self.presentation_view.hints.update(
            hints_enabled,
            ui.input(|i| i.modifiers.alt),
            pressed_position.is_some() && pressed_target.is_none(),
            ui.input(|i| i.time),
        );
        if !blocked {
            // Presentation pan is independent of document gestures and Undo.
            self.viewport_input(ui.ctx());
            let time = ui.input(|i| i.time);
            if pressed_target.is_none()
                && let Some(pos) = pressed_position
            {
                self.presentation_view.press = Some((pos, self.pan, time));
            }
            if let Some((start, pan, pressed)) = self.presentation_view.press {
                if ui.input(|i| i.pointer.primary_down()) {
                    if time - pressed >= 0.15 {
                        if let Some(pos) = pointer {
                            let delta = pos - start;
                            self.pan = pan + Vec2::new(delta.x.trunc(), delta.y.trunc());
                        }
                    } else {
                        ui.ctx().request_repaint_after(Duration::from_millis(150));
                    }
                } else {
                    self.presentation_view.press = None;
                }
            }
            if let Some(target) = pressed_target {
                self.follow_target(&target, ui.ctx());
                self.presentation_view.press = None;
                ui.ctx().request_repaint();
            }
            response.context_menu(|ui| {
                if ui.button("Presentation settings…").clicked() {
                    self.presentation_view.settings_open = true;
                    ui.close();
                }
                if ui.button("Back to editor").clicked() && self.finish_page_notes() {
                    self.present = false;
                    ui.close();
                }
            });
        } else {
            self.presentation_view.press = None;
        }
        Self::paint_scene(painter, &scene, self.canvas_rect.min + self.pan, self.zoom);
        if let Some(hints) = hints {
            if hints.animating {
                ui.ctx().request_repaint();
            }
            let mut painted = BTreeSet::new();
            for link in &scene.links {
                // Occlusion may split one hit region into several pieces that
                // retain the same visible hint. Paint that hint only once.
                let rect = self.screen_rect(link.hint);
                let key = [rect.min.x, rect.min.y, rect.max.x, rect.max.y].map(f32::to_bits);
                if painted.insert(key) {
                    let broken = link.target.strip_prefix("page:").is_some_and(|id| {
                        id.parse::<Id>()
                            .ok()
                            .is_none_or(|id| !self.editor.document.live_pages().any(|p| p.id == id))
                    });
                    let color = hints.color(broken);
                    if let Some(quad) = link.quad {
                        painter.add(egui::Shape::convex_polygon(
                            quad.map(|p| self.screen(p)).to_vec(),
                            color,
                            egui::Stroke::NONE,
                        ));
                    } else {
                        painter.rect_filled(rect, 0., color);
                    }
                }
            }
        }
        self.paint_review_markers(painter);
        if !blocked
            && !egui::Popup::is_any_open(ui.ctx())
            && let Some(pos) = response.hover_pos()
        {
            if settings.big_cursor {
                ui.ctx().set_cursor_icon(egui::CursorIcon::None);
                let held = self
                    .presentation_view
                    .press
                    .is_some_and(|(_, _, time)| ui.input(|i| i.time) - time >= 0.15);
                self.paint_presentation_cursor(painter, pos, target.is_some(), held, scene.bounds);
            } else if target.is_some() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
    }
    fn paint_presentation_cursor(
        &self,
        painter: &egui::Painter,
        pos: Pos2,
        link: bool,
        held: bool,
        bounds: Bounds,
    ) {
        let (points, angle) = if link {
            (
                vec![
                    [32.5, 0.],
                    [39., 4.],
                    [39., 30.],
                    [46., 26.],
                    [53., 32.],
                    [60., 32.],
                    [65., 40.],
                    [60., 68.],
                    [49., 84.],
                    [20., 84.],
                    [5., 60.],
                    [0., 44.],
                    [8., 39.],
                    [24., 54.],
                    [24., 7.],
                ],
                0.,
            )
        } else if held {
            (
                vec![
                    [8., 5.],
                    [15., 3.],
                    [19., 24.],
                    [22., 1.],
                    [29., 0.],
                    [34., 24.],
                    [38., 3.],
                    [45., 5.],
                    [46., 28.],
                    [53., 13.],
                    [61., 16.],
                    [59., 43.],
                    [66., 37.],
                    [66., 48.],
                    [49., 67.],
                    [21., 67.],
                    [4., 44.],
                    [0., 30.],
                    [7., 25.],
                    [16., 38.],
                ],
                0.,
            )
        } else {
            let center = self.screen([bounds.x + bounds.w / 2., bounds.y + bounds.h / 2.]);
            let delta = center - pos;
            (
                vec![
                    [50., 0.],
                    [100., 90.],
                    [69., 90.],
                    [69., 231.],
                    [31., 231.],
                    [31., 90.],
                    [0., 90.],
                ],
                delta.y.atan2(delta.x) + std::f32::consts::FRAC_PI_2,
            )
        };
        let half = if link {
            32.5
        } else if held {
            33.
        } else {
            50.
        };
        let rotate = egui::emath::Rot2::from_angle(angle);
        let points: Vec<_> = points
            .into_iter()
            .map(|p| pos + rotate * Vec2::new(p[0] - half, p[1]))
            .collect();
        let coordinates: Vec<[f32; 2]> = points.iter().map(|p| [p.x, p.y]).collect();
        let mut mesh = egui::Mesh::default();
        for point in &points {
            mesh.colored_vertex(*point, Color32::from_rgb(0, 159, 222));
        }
        mesh.indices = wireframe::popover::triangles(&coordinates);
        painter.add(mesh);
        painter.add(egui::epaint::PathShape {
            points,
            closed: true,
            fill: Color32::TRANSPARENT,
            stroke: egui::Stroke::new(4., Color32::BLACK).into(),
        });
    }
}

fn resized(ui: &egui::Ui, id: egui::Id) -> bool {
    // The vendored panel uses a separate resize response, not its contents response.
    ui.read_response(id.with("__resize"))
        .is_some_and(|r| r.drag_stopped())
}
