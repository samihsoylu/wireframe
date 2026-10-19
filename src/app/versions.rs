// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
mod preferences;
use super::*;

#[derive(Default)]
pub(super) struct Versions {
    preferences: preferences::Preferences,
    pub(super) rename: Option<Rename>,
}
pub(super) struct Rename {
    page: Id,
    alternate: Id,
    name: String,
    focus: bool,
    error: Option<String>,
}
impl WireframeApp {
    pub(super) fn load_version_preferences(&mut self, root: &std::path::Path) {
        if let Err(error) = self.versions.preferences.load(root) {
            self.error = Some(format!(
                "Saved version choices couldn't be read. The settings file is unchanged.\n{error}"
            ));
        }
    }
    pub(super) fn restore_version_view(&mut self) {
        self.versions.rename = None;
        self.presentation_history.clear();
        self.categories = library::default_categories();
        let mut collapsed = HashSet::new();
        self.thumbnails = true;
        if let Some(key) = self.version_preferences_key() {
            (self.categories, collapsed, self.thumbnails) =
                self.versions.preferences.restore(&key, &mut self.editor);
        }
        self.navigator.restore(
            &wireframe::page_tree::PageTree::new(&self.editor.document),
            self.editor.page,
            collapsed,
        );
        self.restore_viewports();
    }
    pub(super) fn remember_version_view(&mut self) {
        if !self.versions.preferences.enabled() {
            return;
        }
        if let Some(key) = self.version_preferences_key()
            && let Err(error) = self.versions.preferences.remember(
                &key,
                &self.editor,
                &self.categories,
                &self.navigator.collapsed,
                self.thumbnails,
            )
        {
            self.status = error;
        }
    }
    fn select_page_version(&mut self, page: Id, alternate: Option<Id>) {
        if !self.finish_page_notes() {
            return;
        }
        self.end_pointer_gestures();
        if let Err(error) = self.editor.switch_version(page, alternate) {
            self.error = Some(error);
        }
    }
    pub(super) fn create_page_version(&mut self) {
        if !self.finish_page_notes() {
            return;
        }
        self.end_pointer_gestures();
        let mut number = 1;
        let name = loop {
            let name = format!("Alternate {number}");
            if !self
                .editor
                .document
                .live_pages()
                .any(|p| p.alternates.iter().any(|a| a.name == name))
            {
                break name;
            }
            number += 1;
        };
        if let Err(error) = self.editor.create_version(&name) {
            self.error = Some(error);
        }
    }
    pub(super) fn project_versions_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Versions", |ui| {
            let mut choice = None;
            if ui.button("Official versions").clicked() {
                choice = Some(None);
            }
            for version in self.editor.document.versions.clone() {
                if ui_text::selectable(ui, false, &version.name, 260.).clicked() {
                    choice = Some(Some(version.id));
                }
            }
            if let Some(version) = choice {
                self.end_pointer_gestures();
                if let Err(error) = self.editor.select_project_version(version) {
                    self.error = Some(error);
                }
                ui.close();
            }
        })
        .response
        .on_hover_text(
            "Choose a named version across pages. Pages without it keep their current version.",
        );
    }
    pub(super) fn page_versions_menu(&mut self, ui: &mut egui::Ui, page: Id) {
        let alternates = self
            .editor
            .document
            .live_pages()
            .find(|p| p.id == page)
            .map(|p| p.alternates.clone())
            .unwrap_or_default();
        let active = self.editor.active_alternate(page);
        if ui
            .selectable_label(active.is_none(), "Official version")
            .clicked()
        {
            self.select_page_version(page, None);
            ui.close();
        }
        for alt in alternates {
            if ui_text::selectable(ui, active == Some(alt.id), &alt.name, 260.).clicked() {
                self.select_page_version(page, Some(alt.id));
                ui.close();
            }
        }
    }
    pub(super) fn versions_panel(&mut self, ui: &mut egui::Ui) {
        let width = ui.available_width();
        ui.visuals_mut().selection.bg_fill = Color32::from_rgb(55, 155, 228);
        ui.visuals_mut().selection.stroke = Stroke::new(1., Color32::WHITE);
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(16, 10))
            .show(ui, |ui| {
                ui.set_width((width - 32.).max(40.));
                ui.horizontal(|ui| {
                    ui.strong("Alternate versions");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button("+")
                            .on_hover_text("Create an independent copy of this version")
                            .clicked()
                        {
                            self.create_page_version();
                        }
                    });
                });
            });
        let page = self.editor.page;
        let active = self.editor.active_alternate(page);
        if self.editor.page_data().alternates.is_empty() {
            return;
        }
        let fill = |selected| {
            if selected {
                Color32::from_rgb(55, 155, 228)
            } else {
                Color32::TRANSPARENT
            }
        };
        egui::Frame::NONE
            .fill(fill(active.is_none()))
            .inner_margin(egui::Margin::symmetric(16, 3))
            .show(ui, |ui| {
                if ui_text::selectable(
                    ui,
                    active.is_none(),
                    "Official version",
                    ui.available_width(),
                )
                .clicked()
                {
                    self.select_page_version(page, None);
                }
            });
        for alt in self.editor.page_data().alternates.clone() {
            egui::Frame::NONE
                .fill(fill(active == Some(alt.id)))
                .inner_margin(egui::Margin::symmetric(16, 3))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui_text::selectable(
                            ui,
                            active == Some(alt.id),
                            &alt.name,
                            (ui.available_width() - 40. - ui.spacing().item_spacing.x).max(40.),
                        )
                        .clicked()
                        {
                            self.select_page_version(page, Some(alt.id));
                        }
                        ui.menu_button("…", |ui| {
                            if ui.button("Rename…").clicked() {
                                self.end_pointer_gestures();
                                self.versions.rename = Some(Rename {
                                    page,
                                    alternate: alt.id,
                                    name: alt.name.clone(),
                                    focus: true,
                                    error: None,
                                });
                                ui.close();
                            }
                            if ui.button("Duplicate").clicked() {
                                self.select_page_version(page, Some(alt.id));
                                self.create_page_version();
                                ui.close();
                            }
                            if ui.button("Promote to official").clicked() {
                                self.select_page_version(page, Some(alt.id));
                                if let Err(error) = self.editor.promote_version(alt.id) {
                                    self.error = Some(error);
                                }
                                ui.close();
                            }
                            if ui.button("Merge with official").clicked() {
                                self.select_page_version(page, Some(alt.id));
                                if let Err(error) = self.editor.merge_version(alt.id) {
                                    self.error = Some(error);
                                }
                                ui.close();
                            }
                            if ui.button("Discard version").clicked() {
                                self.end_pointer_gestures();
                                if let Err(error) = self.editor.discard_version(alt.id) {
                                    self.error = Some(error);
                                }
                                ui.close();
                            }
                        })
                        .response
                        .on_hover_text(format!("Actions for {}", alt.name));
                    });
                });
        }
    }
    pub(super) fn version_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut draft) = self.versions.rename.take() else {
            return;
        };
        let mut apply = false;
        let mut cancel = false;
        let versions = self.editor.document.versions.clone();
        egui::Modal::new(egui::Id::new("rename-version")).show(ctx, |ui| {
            ui.set_width(360.);
            ui.heading("Rename version");
            let salt = ("version-name", draft.page, draft.alternate);
            if draft.focus {
                wireframe::text_entry::forget(ctx, ui.make_persistent_id(egui::Id::new(salt)));
                draft.focus = false;
            }
            ui.add(TextInput::singleline(&mut draft.name).id_salt(salt).autofocus(true)
                .select_all_on_open(true).desired_width(f32::INFINITY));
            if !versions.is_empty() {
                ui.menu_button("Existing names", |ui| {
                    for version in &versions {
                        if ui_text::selectable(ui, false, &version.name, 260.).clicked() { draft.name = version.name.clone(); ui.close(); }
                    }
                });
            }
            let source = self.editor.document.live_pages().find(|p| p.id == draft.page)
                .and_then(|p| p.alternates.iter().find(|a| a.id == draft.alternate));
            let target = versions.iter().find(|v| v.name == draft.name.trim());
            let moves = source.is_some_and(|a| target.is_some_and(|v| a.version != Some(v.id)));
            ui.small(if moves { "This moves this page's version to the existing name. Other pages keep their versions." }
                else { "A shared name changes on every page that uses it. Content and notes stay unchanged." });
            if let Some(error) = &draft.error { ui.colored_label(Color32::from_rgb(180,45,45), error); }
            ui.horizontal(|ui| {
                apply = ui.add_enabled(!draft.name.trim().is_empty(), egui::Button::new("Rename")).clicked();
                cancel = ui.button("Cancel").clicked();
            });
            apply |= !draft.name.trim().is_empty() && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
            cancel |= ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        });
        if apply {
            if self.editor.page != draft.page {
                draft.error =
                    Some("The selected page changed. Cancel and choose the version again.".into());
            } else {
                match self.editor.rename_version(draft.alternate, &draft.name) {
                    Ok(()) => return,
                    Err(error) => draft.error = Some(error),
                }
            }
        }
        if !cancel {
            self.versions.rename = Some(draft);
        }
    }
}
#[cfg(test)]
mod tests;
