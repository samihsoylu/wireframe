// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Retained-resource browsing and scoped permanent-removal confirmation.
use super::*;
use wireframe::workspace::RetainedResource;
#[cfg(test)]
mod tests;

pub(super) struct Trash {
    selected: HashSet<RetainedResource>,
    anchor: RetainedResource,
    focused: RetainedResource,
    focus: bool,
    error: Option<String>,
    confirmation: Option<RemovalConfirmation>,
}

struct RemovalConfirmation {
    resources: Vec<RetainedResource>,
    document: wireframe::document::Project,
    all: bool,
}

impl WireframeApp {
    pub(super) fn open_trash(&mut self) {
        if self.canvas_input_blocked() || self.image_crop.is_some() {
            return;
        }
        if !self.finish_page_notes() {
            return;
        }
        self.end_pointer_gestures();
        self.editor.commit();
        if let Some((resource, _)) = self.editor.retained_resources().next() {
            self.trash = Some(Trash {
                selected: HashSet::from([resource]),
                anchor: resource,
                focused: resource,
                focus: true,
                error: None,
                confirmation: None,
            });
        }
    }

    pub(super) fn move_asset_to_trash(&mut self, asset: Id) {
        if !self.finish_page_notes() {
            return;
        }
        self.end_pointer_gestures();
        match self.editor.trash_assets(&[asset]) {
            Ok(()) => self.status = "Image moved to Trash".into(),
            Err(error) => self.error = Some(error),
        }
    }

    pub(super) fn move_pages_to_trash(&mut self, pages: &[Id]) {
        // Ordinary outside-pointer acceptance runs before page widgets. If a
        // different activation leaves text provisional, do not invalidate it.
        if self.text_edit.is_some() {
            self.status = "Finish editing text, then try moving to Trash again.".into();
            return;
        }
        self.end_pointer_gestures();
        match self.editor.trash_pages(pages) {
            Ok(()) => {
                self.status = if pages.len() == 1 {
                    "Wireframe moved to Trash".into()
                } else {
                    format!("{} wireframes moved to Trash", pages.len())
                };
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub(super) fn move_library_to_trash(&mut self, library: Id) {
        let libraries = self.library_trash_selection(library);
        if !self.finish_page_notes() {
            return;
        }
        self.end_pointer_gestures();
        match self.editor.trash_libraries(&libraries) {
            Ok(()) => {
                self.source_navigation = Default::default();
                self.status = if libraries.len() == 1 {
                    "Component library moved to Trash".into()
                } else {
                    format!("{} component libraries moved to Trash", libraries.len())
                };
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub(super) fn trash_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut draft) = self.trash.take() else {
            return;
        };
        let resources: Vec<_> = self
            .editor
            .retained_resources()
            .map(|(resource, name)| (resource, name.to_owned()))
            .collect();
        if resources.is_empty() {
            return;
        }
        draft
            .selected
            .retain(|id| resources.iter().any(|p| p.0 == *id));
        if !resources.iter().any(|p| p.0 == draft.anchor) {
            draft.anchor = resources[0].0;
        }
        if !resources.iter().any(|p| p.0 == draft.focused) {
            draft.focused = resources[0].0;
            draft.focus = true;
        }
        let mut close = false;
        let mut restore = false;
        let mut remove = None;
        let confirming = draft.confirmation.is_some();
        let width = (ctx.content_rect().width() - 48.).clamp(280., 860.);
        let body_height = (ctx.content_rect().height() - 190.).clamp(120., 460.);
        let columns = ((width - 32.) / 272.).floor().max(1.) as usize;
        egui::Modal::new(egui::Id::new("page-trash"))
            .frame(egui::Frame::NONE.fill(Color32::WHITE).corner_radius(10))
            .show(ctx, |ui| {
                ui.set_width(width);
                if confirming {
                    ui.disable();
                }
                egui::Frame::NONE
                    .fill(Color32::from_gray(225))
                    .inner_margin(16)
                    .show(ui, |ui| {
                        ui.set_width(width - 32.);
                        ui.vertical_centered(|ui| {
                            ui.heading("Trash");
                        });
                    });
                let select_all =
                    !confirming && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::A));
                if select_all {
                    draft.selected = resources.iter().map(|p| p.0).collect();
                }
                let movement = ui.input_mut(|i| {
                    if confirming {
                        return None;
                    }
                    for (key, delta) in [
                        (Key::ArrowLeft, -1),
                        (Key::ArrowRight, 1),
                        (Key::ArrowUp, -(columns as isize)),
                        (Key::ArrowDown, columns as isize),
                    ] {
                        if i.consume_key(Modifiers::SHIFT, key) {
                            return Some((delta, true));
                        }
                        if i.consume_key(Modifiers::NONE, key) {
                            return Some((delta, false));
                        }
                    }
                    None
                });
                if let Some((delta, extend)) = movement {
                    let index = resources
                        .iter()
                        .position(|p| p.0 == draft.focused)
                        .unwrap_or(0);
                    let next =
                        (index as isize + delta).clamp(0, resources.len() as isize - 1) as usize;
                    if !extend {
                        draft.anchor = resources[next].0;
                    }
                    let anchor = resources
                        .iter()
                        .position(|p| p.0 == draft.anchor)
                        .unwrap_or(next);
                    draft.focused = resources[next].0;
                    draft.selected = resources[anchor.min(next)..=anchor.max(next)]
                        .iter()
                        .map(|p| p.0)
                        .collect();
                    draft.focus = true;
                }
                egui::Frame::NONE.inner_margin(24).show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("trash-items")
                        .auto_shrink([false, false])
                        .max_height(body_height)
                        .min_scrolled_height(body_height)
                        .show(ui, |ui| {
                            egui::Grid::new("trash-grid")
                                .spacing(Vec2::splat(16.))
                                .show(ui, |ui| {
                                    for (index, (id, name)) in resources.iter().enumerate() {
                                        let selected = draft.selected.contains(id);
                                        let card_width =
                                            ((width - 48. - 16. * (columns - 1) as f32)
                                                / columns as f32)
                                                .min(256.);
                                        let (rect, response) = ui.allocate_exact_size(
                                            Vec2::new(card_width, 198.),
                                            Sense::click(),
                                        );
                                        response.widget_info(|| {
                                            egui::WidgetInfo::selected(
                                                egui::WidgetType::SelectableLabel,
                                                true,
                                                selected,
                                                match id {
                                                    RetainedResource::Page(_) => {
                                                        format!("{name} — Wireframe")
                                                    }
                                                    RetainedResource::Asset(_) => {
                                                        format!("{name} — Image")
                                                    }
                                                    RetainedResource::Library(_) => {
                                                        format!("{name} — Component library")
                                                    }
                                                },
                                            )
                                        });
                                        if !confirming && draft.focus && *id == draft.focused {
                                            response.request_focus();
                                            response.scroll_to_me(Some(egui::Align::Center));
                                            draft.focus = false;
                                        }
                                        if response.has_focus() {
                                            // The grid owns arrow movement; egui must not
                                            // move focus a second time after selection.
                                            ui.memory_mut(|m| {
                                                m.set_focus_lock_filter(
                                                    response.id,
                                                    egui::EventFilter {
                                                        horizontal_arrows: true,
                                                        vertical_arrows: true,
                                                        ..Default::default()
                                                    },
                                                );
                                            });
                                        }
                                        if response.clicked() {
                                            draft.focused = *id;
                                            let modifiers = ui.input(|i| {
                                                i.events
                                                    .iter()
                                                    .rev()
                                                    .find_map(|event| match event {
                                                        egui::Event::PointerButton {
                                                            button: egui::PointerButton::Primary,
                                                            pressed: false,
                                                            modifiers,
                                                            ..
                                                        } => Some(*modifiers),
                                                        _ => None,
                                                    })
                                                    .unwrap_or(i.modifiers)
                                            });
                                            if modifiers.shift {
                                                let from = resources
                                                    .iter()
                                                    .position(|p| p.0 == draft.anchor)
                                                    .unwrap_or(index);
                                                draft.selected = resources
                                                    [from.min(index)..=from.max(index)]
                                                    .iter()
                                                    .map(|p| p.0)
                                                    .collect();
                                            } else if modifiers.command {
                                                if !draft.selected.remove(id) {
                                                    draft.selected.insert(*id);
                                                }
                                                draft.anchor = *id;
                                            } else {
                                                draft.selected = HashSet::from([*id]);
                                                draft.anchor = *id;
                                            }
                                        }
                                        if ui.is_rect_visible(rect) {
                                            ui.painter().add(
                                                egui::epaint::Shadow {
                                                    offset: [0, 1],
                                                    blur: 5,
                                                    spread: 0,
                                                    color: Color32::from_black_alpha(45),
                                                }
                                                .as_shape(rect, 0),
                                            );
                                            ui.painter().rect_filled(rect, 0, Color32::WHITE);
                                            let preview = Rect::from_min_max(
                                                rect.min + Vec2::splat(12.),
                                                rect.max - Vec2::new(12., 36.),
                                            );
                                            let scene = match *id {
                                                RetainedResource::Asset(asset) => {
                                                    self.previews.retained_asset_scene(
                                                        &self.editor.document,
                                                        asset,
                                                    )
                                                }
                                                _ => self
                                                    .editor
                                                    .retained_preview(id.id())
                                                    .map(std::sync::Arc::new),
                                            };
                                            match scene {
                                                Ok(scene) => {
                                                    let bounds = scene.bounds;
                                                    let scale = (preview.width()
                                                        / bounds.w.max(1.))
                                                    .min(preview.height() / bounds.h.max(1.));
                                                    let origin = preview.center()
                                                        - Vec2::new(bounds.w, bounds.h) * scale
                                                            / 2.
                                                        - Vec2::new(bounds.x, bounds.y) * scale;
                                                    Self::paint_scene(
                                                        &ui.painter().with_clip_rect(preview),
                                                        &scene,
                                                        origin,
                                                        scale,
                                                    );
                                                }
                                                Err(_) => {
                                                    ui.painter().text(
                                                        preview.center(),
                                                        egui::Align2::CENTER_CENTER,
                                                        "Preview unavailable",
                                                        egui::FontId::proportional(12.),
                                                        Color32::DARK_GRAY,
                                                    );
                                                }
                                            }
                                            ui.painter().rect_stroke(
                                                rect,
                                                0,
                                                Stroke::new(
                                                    if selected || response.has_focus() {
                                                        2.
                                                    } else {
                                                        1.
                                                    },
                                                    if selected || response.has_focus() {
                                                        Color32::from_rgb(65, 155, 221)
                                                    } else {
                                                        Color32::LIGHT_GRAY
                                                    },
                                                ),
                                                StrokeKind::Inside,
                                            );
                                            ui_text::paint(
                                                ui,
                                                Rect::from_min_max(
                                                    egui::pos2(
                                                        rect.left() + 8.,
                                                        rect.bottom() - 32.,
                                                    ),
                                                    rect.max - Vec2::splat(8.),
                                                ),
                                                name,
                                                14.,
                                                Color32::from_gray(40),
                                                true,
                                            );
                                        }
                                        if (index + 1) % columns == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        });
                });
                if let Some(error) = &draft.error {
                    ui.colored_label(Color32::DARK_RED, error);
                }
                egui::Frame::NONE
                    .fill(Color32::from_gray(225))
                    .inner_margin(12)
                    .show(ui, |ui| {
                        ui.set_width(width - 24.);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Empty Trash").clicked() {
                                remove = Some(true);
                            }
                            if ui
                                .add_enabled(
                                    !draft.selected.is_empty(),
                                    egui::Button::new("Delete Permanently"),
                                )
                                .clicked()
                            {
                                remove = Some(false);
                            }
                            restore = ui
                                .add_enabled(
                                    !draft.selected.is_empty(),
                                    egui::Button::new("Restore"),
                                )
                                .clicked();
                            close = ui.button("Close").clicked();
                        });
                    });
                close |=
                    !confirming && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
            });
        if close {
            return;
        }
        if let Some(all) = remove {
            draft.confirmation = Some(RemovalConfirmation {
                resources: resources
                    .iter()
                    .filter(|(id, _)| all || draft.selected.contains(id))
                    .map(|(id, _)| *id)
                    .collect(),
                document: self.editor.document.clone(),
                all,
            });
        }
        self.permanent_trash_confirmation(ctx, &mut draft);
        if restore {
            let ids: Vec<_> = resources
                .iter()
                .filter(|p| draft.selected.contains(&p.0))
                .map(|p| p.0)
                .collect();
            match self.editor.restore_resources(&ids) {
                Ok(()) => {
                    let old_index = resources
                        .iter()
                        .position(|p| p.0 == draft.focused)
                        .unwrap_or(0);
                    let remaining: Vec<_> = self
                        .editor
                        .retained_resources()
                        .map(|(resource, _)| resource)
                        .collect();
                    draft.selected.retain(|id| remaining.contains(id));
                    if !remaining.is_empty() {
                        if !remaining.contains(&draft.focused) {
                            draft.focused = remaining[old_index.min(remaining.len() - 1)];
                        }
                        draft.anchor = draft.focused;
                        draft.selected.insert(draft.focused);
                        draft.focus = true;
                    }
                    draft.error = None;
                    self.status = "Restored from Trash".into();
                }
                Err(error) => draft.error = Some(error),
            }
        }
        if self.editor.retained_resources().next().is_some() || draft.confirmation.is_some() {
            self.trash = Some(draft);
        }
    }

    fn permanent_trash_confirmation(&mut self, ctx: &egui::Context, draft: &mut Trash) {
        let Some(confirmation) = &draft.confirmation else {
            return;
        };
        let unchanged = self.editor.document == confirmation.document;
        let title = if confirmation.all {
            "Empty Trash"
        } else {
            "Delete Permanently"
        };
        let action = if confirmation.all {
            "Empty Trash"
        } else {
            "Delete"
        };
        let count = confirmation.resources.len();
        let message = if confirmation.all {
            format!(
                "Empty the Trash and permanently delete all {count} items? This operation cannot be undone."
            )
        } else if count == 1 {
            "Permanently delete the selected item? This operation cannot be undone.".into()
        } else {
            format!(
                "Permanently delete these {count} selected items? This operation cannot be undone."
            )
        };
        let mut cancel = false;
        let mut accept = false;
        egui::Modal::new(egui::Id::new("permanent-trash-confirmation")).show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 64.).clamp(240., 420.));
            ui.heading(title);
            ui.label(message);
            if !unchanged {
                ui.colored_label(
                    Color32::DARK_RED,
                    "The project changed. Cancel and review the Trash again.",
                );
            }
            ui.horizontal(|ui| {
                cancel = ui.button("Cancel").clicked();
                accept = ui
                    .add_enabled(unchanged, egui::Button::new(action))
                    .clicked();
            });
            cancel |= ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        });
        if cancel {
            draft.confirmation = None;
            draft.focus = true;
        } else if accept {
            let ids = confirmation.resources.clone();
            // Check again at application time. Empty Trash uses its captured
            // complete set and never silently expands to newly retained items.
            if self.editor.document != confirmation.document {
                return;
            }
            match self.editor.permanently_remove_resources(&ids) {
                Ok(()) => {
                    draft.confirmation = None;
                    draft.error = None;
                    draft.selected.clear();
                    if let Some((id, _)) = self.editor.retained_resources().next() {
                        draft.selected.insert(id);
                        draft.anchor = id;
                        draft.focused = id;
                        draft.focus = true;
                    }
                    self.source_navigation = Default::default();
                    self.status = if count == 1 {
                        "Item permanently deleted".into()
                    } else {
                        format!("{count} items permanently deleted")
                    };
                }
                Err(error) => {
                    draft.confirmation = None;
                    draft.error = Some(error);
                }
            }
        }
    }
}
