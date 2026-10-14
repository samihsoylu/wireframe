// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
impl WireframeApp {
    pub(super) fn components_sidebar(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Components")
            .default_open(true)
            .show(ui, |ui| {
                let definitions = self
                    .editor
                    .document
                    .definitions
                    .iter()
                    .map(|d| (d.id, d.name.clone()))
                    .collect::<Vec<_>>();
                if definitions.is_empty() {
                    ui.small(
                        "Select a control or group, then choose Make component in the inspector.",
                    );
                }
                egui::ScrollArea::vertical()
                    .id_salt("components-list")
                    .max_height(200.)
                    .show(ui, |ui| {
                        for (id, name) in definitions {
                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_label(self.editor.definition == Some(id), &name)
                                    .on_hover_text("Edit this component's source")
                                    .clicked()
                                    && let Err(error) = self.editor.edit_definition(id)
                                {
                                    self.error = Some(error);
                                }
                                if ui
                                    .small_button("+")
                                    .on_hover_text(format!("Insert {name}"))
                                    .clicked()
                                {
                                    let n = self.editor.current().controls.len();
                                    if let Err(error) = self.editor.insert_component(
                                        id,
                                        50. + (n % 8) as f32 * 20.,
                                        50. + (n % 8) as f32 * 24.,
                                    ) {
                                        self.error = Some(error);
                                    }
                                }
                            });
                        }
                    });
            });
        ui.separator();
    }
    pub(super) fn component_source_inspector(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.editor.definition else {
            return;
        };
        ui.heading("Component source");
        ui.label("Changes here update every instance. Local overrides keep their values.");
        let mut name = self.editor.current().name;
        ui.label("Name");
        let edit = ui.add(TextInput::singleline(&mut name));
        if edit.changed() {
            self.editor.begin("Rename component");
            if let Some(d) = self
                .editor
                .document
                .definitions
                .iter_mut()
                .find(|d| d.id == id)
            {
                d.name = name.clone();
            }
        }
        if edit.lost_focus() {
            if name.trim().is_empty() {
                self.editor.cancel();
                self.error = Some("Give the component a name".into());
            } else {
                self.editor.commit();
            }
        }
        ui.separator();
        ui.small(
            "Deleting this source leaves missing component placeholders. Undo restores the source.",
        );
        if ui.button("Delete component source").clicked()
            && let Err(error) = self.editor.delete_definition(id)
        {
            self.error = Some(error);
        }
        if ui.button("Back to wireframe").clicked() {
            self.editor.switch_page(self.editor.page);
        }
    }
    pub(super) fn component_actions(&mut self, ui: &mut egui::Ui) {
        if self.editor.selection.len() != 1 {
            return;
        }
        let id = *self.editor.selection.iter().next().unwrap();
        ui.separator();
        if self.editor.missing_component(id) {
            ui.label("Component source is missing. Its reference and local changes are preserved.");
            ui.small("Source editing and detaching are unavailable while the source is missing.");
            return;
        }
        if self.editor.editing_instance() {
            ui.small("Local changes affect only this instance.");
            if ui.button("Reset local changes").clicked()
                && let Err(error) = self.editor.reset_local_changes(id)
            {
                self.error = Some(error);
            }
            return;
        }
        if let Some(definition) = self.editor.instance_definition(id) {
            if ui.button("Edit this instance").clicked() {
                self.editor.enter(id);
            }
            if ui.button("Reset instance overrides").clicked()
                && let Err(error) = self.editor.reset_instance(id)
            {
                self.error = Some(error);
            }
            if ui.button("Edit component source").clicked()
                && let Err(error) = self.editor.edit_definition(definition)
            {
                self.error = Some(error);
            }
            if ui
                .button("Detach component")
                .on_hover_text("Turn this instance into an ordinary group with its current values")
                .clicked()
                && let Err(error) = self.editor.detach_component(id)
            {
                self.error = Some(error);
            }
        } else if ui.button("Make component").clicked() {
            let name = format!("Component {}", self.editor.document.definitions.len() + 1);
            if let Err(error) = self.editor.create_component(name) {
                self.error = Some(error);
            }
        }
    }
}
