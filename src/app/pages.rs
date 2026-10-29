// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(super) enum RenameOwner {
    Project,
    Page(Id),
    Library(Id),
    Source(Id),
}
pub(super) struct Rename {
    owner: RenameOwner,
    name: String,
    focus: bool,
    return_focus: Option<egui::Id>,
    error: Option<String>,
}
impl WireframeApp {
    fn rename_owner_name(&self, owner: RenameOwner) -> Option<&str> {
        match owner {
            RenameOwner::Project => Some(&self.editor.document.title),
            RenameOwner::Page(id) => self.editor.document.live_page(id).map(|p| p.name.as_str()),
            RenameOwner::Library(id) => self
                .editor
                .document
                .live_libraries()
                .find(|l| l.id == id)
                .map(|l| l.name.as_str()),
            RenameOwner::Source(id) => self
                .editor
                .document
                .live_definitions()
                .find(|d| d.id == id)
                .map(|d| d.name.as_str()),
        }
    }
    pub(super) fn begin_source_rename(&mut self, id: Id, library: bool) {
        if !self.finish_page_notes() {
            return;
        }
        let owner = if library {
            RenameOwner::Library(id)
        } else {
            RenameOwner::Source(id)
        };
        if let Some(name) = self.rename_owner_name(owner).map(str::to_owned) {
            self.end_pointer_gestures();
            self.page_rename = Some(Rename {
                owner,
                name,
                focus: true,
                return_focus: None,
                error: None,
            });
        }
    }
    pub(super) fn page_context_menu(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        owner: Id,
    ) {
        let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(response));
        if response.secondary_clicked() || (open && ui.input(|i| i.key_pressed(Key::Escape))) {
            // A right-click can clear another row's focus; Escape can also
            // clear newly acquired focus before the popup handles the event.
            // Return keyboard ownership to its row without changing selection.
            response.request_focus();
        }
        response.context_menu(|ui| self.page_actions_menu(ui, owner));
        if let Some(draft) = &mut self.page_rename
            && draft.owner == RenameOwner::Page(owner)
        {
            draft.return_focus = Some(response.id);
        }
    }
    pub(super) fn begin_page_rename(&mut self, ctx: &egui::Context, page: Id) {
        self.end_pointer_gestures();
        if let Some(target) = self.editor.document.live_pages().find(|p| p.id == page) {
            self.page_rename = Some(Rename {
                owner: RenameOwner::Page(page),
                name: target.name.clone(),
                focus: true,
                return_focus: ctx
                    .memory(|m| m.focused())
                    .filter(|id| self.navigator.focus_ids.contains(id)),
                error: None,
            });
        }
    }
    pub(super) fn begin_project_rename(&mut self) {
        self.end_pointer_gestures();
        self.page_rename = Some(Rename {
            owner: RenameOwner::Project,
            name: self.editor.document.title.clone(),
            focus: true,
            return_focus: None,
            error: None,
        });
    }
    pub(super) fn page_actions_menu(&mut self, ui: &mut egui::Ui, page: Id) {
        let allowed = !self.canvas_input_blocked() && self.image_crop.is_none();
        let tree = wireframe::page_tree::PageTree::new(&self.editor.document);
        self.navigator.sync(
            &tree,
            self.editor.page,
            self.editor.duplicated_page_selection(),
        );
        let targets = self.navigator.context(&tree, page);
        let single = targets.len() == 1;
        ui.add_enabled_ui(allowed, |ui| {
            if targets.len() > 1
                && self.navigator.selected.contains(&page)
                && ui.button("Deselect").clicked()
            {
                let active =
                    self.navigator
                        .select(&tree, page, self.editor.page, Modifiers::COMMAND);
                self.editor.switch_page(active);
                ui.close();
            }
            if ui
                .add_enabled(single, egui::Button::new("Rename…"))
                .clicked()
            {
                self.begin_page_rename(ui.ctx(), page);
                ui.close();
            }
            let duplicate = if single {
                "Duplicate".into()
            } else {
                format!("Duplicate {} items", targets.len())
            };
            for (label, cut) in [("Cut", true), ("Copy", false)] {
                if ui.button(label).clicked() {
                    self.copy_page_resources(ui.ctx(), &targets, cut);
                    ui.close();
                }
            }
            if ui.button("Paste").clicked() {
                self.request_page_paste(ui.ctx(), &targets);
                ui.close();
            }
            if ui.button(duplicate).clicked() {
                self.end_pointer_gestures();
                if let Err(error) = self.editor.duplicate_pages(&targets) {
                    self.error = Some(error);
                }
                ui.close();
            }
            ui.separator();
            ui.add_enabled_ui(single, |ui| self.page_versions_menu(ui, page));
            if ui
                .add_enabled(single, egui::Button::new("Create alternate version"))
                .clicked()
            {
                self.editor.switch_page(page);
                self.create_page_version();
                ui.close();
            }
            ui.separator();
            let trash = if single {
                "Move to Trash".into()
            } else {
                format!("Move {} items to Trash", targets.len())
            };
            if ui.button(trash).clicked() {
                self.move_pages_to_trash(&targets);
                ui.close();
            }
        });
    }
    fn return_page_rename_focus(&mut self, ctx: &egui::Context, draft: &Rename) {
        if let Some(id) = draft.return_focus {
            ctx.memory_mut(|m| m.request_focus(id));
            // Reassert after row construction while the closing modal's layer
            // still owns interaction for one pass.
            self.navigator.focus_to = Some(self.editor.page);
        }
    }
    pub(super) fn page_rename_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut draft) = self.page_rename.take() else {
            return;
        };
        let mut apply = false;
        let mut cancel = false;
        egui::Modal::new(egui::Id::new("rename-page")).show(ctx, |ui| {
            ui.set_width(360.);
            ui.heading(match draft.owner {
                RenameOwner::Project => "Rename project",
                RenameOwner::Page(_) => "Rename wireframe",
                RenameOwner::Library(_) => "Rename component library",
                RenameOwner::Source(_) => "Rename component source",
            });
            let salt = ("resource-name", draft.owner);
            if draft.focus {
                wireframe::text_entry::forget(ctx, ui.make_persistent_id(egui::Id::new(salt)));
                draft.focus = false;
            }
            ui.add(
                TextInput::singleline(&mut draft.name)
                    .id_salt(salt)
                    .autofocus(true)
                    .select_all_on_open(true)
                    .desired_width(f32::INFINITY),
            );
            let changed = self
                .rename_owner_name(draft.owner)
                .is_some_and(|name| name != draft.name);
            let valid = changed && !draft.name.trim().is_empty() && draft.name.len() <= 10000;
            if let Some(error) = &draft.error {
                ui.colored_label(Color32::from_rgb(180, 45, 45), error);
            }
            ui.horizontal(|ui| {
                cancel = ui.button("Cancel").clicked();
                apply = ui.add_enabled(valid, egui::Button::new("Rename")).clicked();
            });
            apply |= valid && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
            cancel |= ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        });
        if cancel {
            self.return_page_rename_focus(ctx, &draft);
            return;
        }
        if apply {
            let result = match draft.owner {
                RenameOwner::Project => self.editor.rename_project(&draft.name),
                RenameOwner::Page(page) => self.editor.rename_page(page, &draft.name),
                RenameOwner::Library(id) => self.editor.rename_library(id, &draft.name),
                RenameOwner::Source(id) => self.editor.rename_definition(id, draft.name.clone()),
            };
            match result {
                Ok(()) => {
                    self.return_page_rename_focus(ctx, &draft);
                    return;
                }
                Err(error) => draft.error = Some(error),
            }
        }
        self.page_rename = Some(draft);
    }
}

#[cfg(test)]
mod tests;
