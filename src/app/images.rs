// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::{
    document::{Properties, Value},
    images::Prepared,
    workspace::ImportChoice,
};

impl WireframeApp {
    pub(super) fn image_properties(&mut self, ui: &mut egui::Ui, id: Id, props: &Properties) {
        let mut selected = match props.get("image") {
            Some(Value::Asset(id)) => Some(*id),
            _ => None,
        };
        let before = selected;
        let locked = self
            .editor
            .current()
            .controls
            .iter()
            .find(|c| c.id == id)
            .is_none_or(|c| c.locked);
        ui.add_space(10.);
        ui.add_enabled_ui(!locked, |ui| {
            ui.label("Image source");
            let name = selected
                .and_then(|id| self.editor.document.assets.iter().find(|a| a.id == id))
                .map(|a| a.name.as_str())
                .unwrap_or("Default placeholder");
            egui::ComboBox::from_id_salt("image-source")
                .selected_text(name)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected, None, "Default placeholder");
                    for a in &self.editor.document.assets {
                        if wireframe::images::dimensions(&a.bytes).is_ok() {
                            ui.selectable_value(&mut selected, Some(a.id), &a.name);
                        }
                    }
                });
            if selected != before
                && let Err(e) = self.editor.assign_image(id, selected)
            {
                self.error = Some(e);
            }
            if ui.button("Import image…").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Images", &["png", "jpg", "jpeg", "gif"])
                    .pick_file()
            {
                match Prepared::read(&path) {
                    Ok(image) => {
                        let conflict = self
                            .editor
                            .image_conflict(&image)
                            .and_then(|id| self.editor.document.assets.iter().find(|a| a.id == id));
                        if conflict.is_some_and(|a| a.bytes != image.bytes) {
                            self.image_import = Some((id, image));
                        } else if let Err(e) =
                            self.editor
                                .import_image(id, image, ImportChoice::UseExisting)
                        {
                            self.error = Some(e);
                        }
                    }
                    Err(e) => self.error = Some(e),
                }
            }
            if selected.is_some()
                && ui.button("Remove image").clicked()
                && let Err(e) = self.editor.assign_image(id, None)
            {
                self.error = Some(e);
            }
            if ui.button("Auto-size").clicked() {
                self.editor.begin("Auto-size image");
                if let Err(e) = self.editor.auto_size_control(id) {
                    self.error = Some(e);
                    self.editor.cancel();
                } else {
                    self.editor.commit();
                }
            }
            if self.editor.editing_instance()
                && self
                    .editor
                    .explicit_control_properties(id)
                    .contains_key("image")
                && ui.button("Reset image to source").clicked()
            {
                self.editor.begin("Reset image source");
                if let Err(e) = self.editor.reset_control_property(id, "image") {
                    self.error = Some(e);
                    self.editor.cancel();
                } else {
                    self.editor.commit();
                }
            }
            ui.small("Images are stored in this project.");
        });
    }
    pub(super) fn image_import_dialog(&mut self, ctx: &egui::Context) {
        let Some((_, image)) = &self.image_import else {
            return;
        };
        let name = image.name.clone();
        let mut choice = None;
        let mut cancel = false;
        egui::Modal::new(egui::Id::new("image-import-conflict")).show(ctx, |ui| {
            ui.heading("An image with this name already exists");
            ui.label(name);
            ui.label("Use new replaces the shared image everywhere in this project. Keep both adds a separate image.");
            ui.horizontal(|ui| {
                if ui.button("Use new").clicked() { choice = Some(ImportChoice::Replace); }
                if ui.button("Use existing").clicked() { choice = Some(ImportChoice::UseExisting); }
                if ui.button("Keep both").clicked() { choice = Some(ImportChoice::KeepBoth); }
                if ui.button("Cancel").clicked() { cancel = true; }
            });
        });
        if let Some(choice) = choice {
            let (id, image) = self.image_import.take().unwrap();
            if let Err(e) = self.editor.import_image(id, image, choice) {
                self.error = Some(e);
            }
        } else if cancel {
            self.image_import = None;
        }
    }
}
