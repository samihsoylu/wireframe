// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::text_entry::layout::{Layout, shared_engine};

#[derive(Default)]
pub(super) struct PageInspector {
    draft: Option<Notes>,
    read: Option<ReadNotes>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum NotesOwner {
    Page(Id, Option<Id>),
    Source(Id),
}
struct Notes {
    owner: NotesOwner,
    text: String,
    rect: Rect,
    fresh: bool,
    error: Option<String>,
}
struct ReadNotes {
    text: String,
    width: f32,
    size: f32,
    scale: f32,
    layout: Layout,
    raster: Option<(Rect, egui::TextureHandle)>,
}

impl WireframeApp {
    fn notes_owner(&self) -> NotesOwner {
        if self.editor.in_source_workspace() {
            NotesOwner::Source(self.editor.surface_id())
        } else {
            NotesOwner::Page(
                self.editor.page,
                self.editor.active_alternate(self.editor.page),
            )
        }
    }
    /// Called before commands/widgets so a departing field cannot apply to the
    /// next page. The raw-input splitter has already delivered preceding text.
    pub(super) fn prepare_page_notes(&mut self, ctx: &egui::Context) {
        let Some(draft) = &self.page_inspector.draft else {
            return;
        };
        let departure = if self.present { !self.presentation_view.session.settings().notes } else { !self.show_inspector }
            || self.notes_owner() != draft.owner
            || (!self.present && !self.editor.selection.is_empty())
            || ctx.input(|i| i.viewport().close_requested() || i.events.iter().any(|e| {
                matches!(e, egui::Event::PointerButton { pos, pressed: true, .. } if !draft.rect.contains(*pos))
            }));
        if departure && !self.finish_page_notes() && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
    }
    pub(super) fn finish_page_notes(&mut self) -> bool {
        let Some(mut draft) = self.page_inspector.draft.take() else {
            return true;
        };
        let result = match draft.owner {
            NotesOwner::Page(page, alternate) => {
                self.editor.edit_page_notes(page, alternate, &draft.text)
            }
            NotesOwner::Source(surface) => self.editor.edit_source_notes(surface, &draft.text),
        };
        match result {
            Ok(()) => true,
            Err(error) => {
                draft.error = Some(error.clone());
                self.error = Some(error);
                self.page_inspector.draft = Some(draft);
                false
            }
        }
    }
    pub(super) fn notes_editing(&self) -> bool {
        self.page_inspector.draft.is_some()
    }
    pub(super) fn notes_shortcut(&mut self, event: &egui::Event) -> bool {
        if self.page_inspector.draft.is_none() || self.canvas_input_blocked() {
            return false;
        }
        if let egui::Event::Key {
            key,
            modifiers,
            pressed: true,
            ..
        } = event
        {
            if *key == Key::Escape && modifiers.is_none() {
                self.page_inspector.draft = None;
                return true;
            }
            if *key == Key::Enter && modifiers.matches_logically(Modifiers::COMMAND) {
                self.finish_page_notes();
                return true;
            }
        }
        false
    }
    fn begin_page_notes(&mut self, rect: Rect) {
        self.editor.commit();
        self.page_inspector.draft = Some(Notes {
            owner: self.notes_owner(),
            text: self
                .editor
                .document
                .surface_notes(self.editor.surface_id())
                .into(),
            rect,
            fresh: true,
            error: None,
        });
    }
    pub(super) fn page_inspector(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.y = 0.;
        let width = ui.available_width();
        egui::Frame::NONE
            .fill(chrome::panel_heading(ui, &self.menus))
            .inner_margin(egui::Margin::symmetric(16, 9))
            .show(ui, |ui| {
                ui.set_width((width - 32.).max(40.));
                let name = if self.editor.in_source_workspace() {
                    self.editor.current().name
                } else {
                    self.editor.page_data().name.clone()
                };
                ui_text::label_styled(
                    ui,
                    &name,
                    ui.available_width(),
                    ui_text::CaptionStyle::panel_heading(),
                );
            });
        let height = if self.editor.in_source_workspace() {
            let uses = self.editor.definition.map(|source| {
                self.editor
                    .component_usages(source)
                    .into_iter()
                    .map(|usage| usage.surface)
                    .collect::<HashSet<_>>()
                    .len()
            });
            let footer = uses.map_or(0., |count| {
                (44. + count.max(1) as f32 * 22.).min((ui.available_height() * 0.35).max(66.))
            });
            (ui.available_height() - footer - 8.).max(160.)
        } else {
            (ui.available_height() * 0.55).max(160.)
        };
        self.notes_region(ui, height);
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("page-alternates")
            .show(ui, |ui| {
                if self.editor.in_source_workspace() {
                    self.component_usage_panel(ui);
                } else {
                    self.versions_panel(ui);
                }
            });
    }
    pub(super) fn notes_region(&mut self, ui: &mut egui::Ui, height: f32) {
        let width = ui.available_width();
        let start = ui.cursor().min;
        let notes_rect = Rect::from_min_size(start, Vec2::new(width, height));
        ui.allocate_ui_with_layout(
            Vec2::new(width, height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_min_height(height);
                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(16, 10))
                    .show(ui, |ui| {
                        ui.strong("Notes");
                    });
                if let Some(mut draft) = self.page_inspector.draft.take() {
                    draft.rect = notes_rect;
                    let mut cancel = false;
                    let mut save = false;
                    let mut blur = false;
                    egui::Frame::NONE
                        .inner_margin(egui::Margin::symmetric(12, 4))
                        .show(ui, |ui| {
                            let salt = ("surface-notes", draft.owner);
                            if draft.fresh {
                                wireframe::text_entry::forget(
                                    ui.ctx(),
                                    ui.make_persistent_id(egui::Id::new(salt)),
                                );
                                draft.fresh = false;
                            }
                            let line = egui::TextStyle::Body.resolve(ui.style()).size * 1.4;
                            let rows = ((height - 100.) / line).floor().max(2.) as usize;
                            let response = ui.add(
                                TextInput::multiline(&mut draft.text)
                                    .id_salt(salt)
                                    .autofocus(true)
                                    .submit_on_command_enter(true)
                                    .desired_rows(rows)
                                    .desired_width(f32::INFINITY),
                            );
                            // Pointer-down on Cancel must not commit before its release.
                            // Outside clicks are handled before widgets; Tab is the
                            // remaining keyboard departure from the text field.
                            blur = response.lost_focus() && ui.input(|i| i.key_pressed(Key::Tab));
                            if let Some(error) = &draft.error {
                                ui.colored_label(Color32::from_rgb(180, 45, 45), error);
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    save = ui
                                        .add_sized([90., 24.], egui::Button::new("Save"))
                                        .on_hover_text("Save notes (⌘/Ctrl Enter)")
                                        .clicked();
                                    cancel = ui
                                        .add_sized([90., 24.], egui::Button::new("Cancel"))
                                        .clicked();
                                },
                            );
                        });
                    if !cancel {
                        self.page_inspector.draft = Some(draft);
                        if save || blur {
                            self.finish_page_notes();
                        }
                    }
                } else {
                    let notes = self
                        .editor
                        .document
                        .surface_notes(self.editor.surface_id())
                        .to_owned();
                    egui::Frame::NONE
                        .inner_margin(egui::Margin::symmetric(16, 20))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("notes-read-scroll")
                                .max_height((height - 82.).max(30.))
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    if self.read_page_notes(ui, &notes).clicked() {
                                        self.begin_page_notes(notes_rect);
                                    }
                                });
                        });
                }
            },
        );
    }
    fn read_page_notes(&mut self, ui: &mut egui::Ui, notes: &str) -> egui::Response {
        let text = if notes.is_empty() {
            "Click here to edit notes."
        } else {
            notes
        };
        let width = ui.available_width().max(1.);
        let size = egui::TextStyle::Body.resolve(ui.style()).size;
        let scale = ui.ctx().pixels_per_point();
        let engine = shared_engine(ui.ctx());
        let mut engine = engine.lock().unwrap_or_else(|e| e.into_inner());
        if !self.page_inspector.read.as_ref().is_some_and(|r| {
            r.text == text && r.width == width && r.size == size && r.scale == scale
        }) {
            match Layout::new(&mut engine, text, size, width, scale) {
                Ok(layout) => {
                    self.page_inspector.read = Some(ReadNotes {
                        text: text.into(),
                        width,
                        size,
                        scale,
                        layout,
                        raster: None,
                    })
                }
                Err(error) => return ui.colored_label(Color32::RED, error),
            }
        }
        let read = self.page_inspector.read.as_mut().unwrap();
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(width, read.layout.size.y.max(size * 1.4)),
            Sense::click(),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), text)
        });
        let visible = rect.intersect(ui.clip_rect());
        if visible.is_positive() {
            let viewport = visible.translate(-rect.min.to_vec2());
            if !read.raster.as_ref().is_some_and(|(r, _)| *r == viewport)
                && let Ok(image) = read
                    .layout
                    .raster(&mut engine, viewport, Color32::from_gray(30))
            {
                read.raster = Some((
                    viewport,
                    ui.ctx()
                        .load_texture("notes-read", image, egui::TextureOptions::LINEAR),
                ));
            }
            if let Some((r, texture)) = &read.raster {
                ui.painter().image(
                    texture.id(),
                    r.translate(rect.min.to_vec2()),
                    Rect::from_min_max(Pos2::ZERO, egui::pos2(1., 1.)),
                    Color32::WHITE,
                );
            }
        }
        response.on_hover_cursor(egui::CursorIcon::Text)
    }
}

#[cfg(test)]
mod tests;
