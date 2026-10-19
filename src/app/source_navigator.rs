// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Source-resource navigation is separate from canvas and page selection.
use super::*;
use std::collections::HashMap;
mod drag;
mod selection;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum ResourceRow {
    Library(Id),
    Source(Id),
}
impl ResourceRow {
    fn source(self) -> Option<Id> {
        match self {
            Self::Source(id) => Some(id),
            Self::Library(_) => None,
        }
    }
}
#[derive(Default)]
pub(super) struct SourceNavigation {
    pub(super) drag: Option<drag::SourceDrag>,
    reveal_library: Option<Id>,
    focus: HashMap<egui::Id, ResourceRow>,
    pub(super) selected: HashSet<ResourceRow>,
    anchor: Option<ResourceRow>,
    visible: Vec<ResourceRow>,
    surface: Option<ResourceRow>,
    pub(super) focus_to: Option<Id>,
    pub(super) release_focus: bool,
}
impl WireframeApp {
    pub(super) fn back_to_wireframes(&mut self) {
        if self.finish_page_notes() {
            self.end_pointer_gestures();
            self.editor.switch_page(self.editor.page);
            self.source_navigation = Default::default();
        }
    }
    pub(super) fn source_navigation_focused(&self, ctx: &egui::Context) -> bool {
        self.show_navigator
            && !self.source_navigation.release_focus
            && self.editor.in_source_workspace()
            && ctx
                .memory(|m| m.focused())
                .is_some_and(|id| self.source_navigation.focus.contains_key(&id))
    }
    pub(super) fn source_command_enabled(
        &self,
        ctx: &egui::Context,
        command: menus::Command,
    ) -> Option<bool> {
        if !self.source_navigation_focused(ctx) {
            return None;
        }
        let target = self.focused_resource_row(ctx);
        let source = target.and_then(ResourceRow::source);
        let valid_source =
            source.is_some_and(|id| self.editor.document.component_source(id).is_some());
        let selection = target
            .map(|row| self.resource_selection(row))
            .unwrap_or_default();
        let resource_transfer = self.selected_library_ids(&selection).is_some();
        use menus::Command::*;
        match command {
            AlignLeft | AlignCenter | AlignRight | AlignTop | AlignMiddle | AlignBottom
            | DistributeHorizontal | DistributeVertical | SpaceHorizontal | SpaceVertical
            | AutoSize => Some(false),
            Copy => Some(!selection.is_empty()),
            Duplicate => Some(valid_source && selection.len() == 1),
            Delete => Some(valid_source && !resource_transfer),
            Cut => Some(!selection.is_empty()),
            Paste => Some(true),
            PasteControls => Some(!self.clipboard.is_empty()),
            // Whole-library clipboard and resource selection are separate actions.
            SelectAll | Group | Ungroup | SendToBack | BringToFront | SendBackward
            | BringForward => Some(false),
            _ => None,
        }
    }
    pub(super) fn run_source_command(
        &mut self,
        ctx: &egui::Context,
        command: menus::Command,
    ) -> bool {
        let Some(enabled) = self.source_command_enabled(ctx, command) else {
            return false;
        };
        if !enabled {
            return true;
        }
        let target = self.focused_resource_row(ctx);
        let source = target.and_then(ResourceRow::source);
        self.end_pointer_gestures();
        let result = match command {
            menus::Command::Duplicate => self.duplicate_source_resource(source.unwrap()),
            menus::Command::Delete => self
                .editor
                .delete_definitions(&self.source_resource_selection(source.unwrap())),
            menus::Command::Copy | menus::Command::Cut => {
                self.copy_selected_resource(ctx, target.unwrap(), command == menus::Command::Cut)
            }
            menus::Command::Paste => {
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                Ok(())
            }
            menus::Command::PasteControls => {
                self.paste_clipboard(&self.clipboard.clone());
                Ok(())
            }
            _ => unreachable!(),
        };
        if let Err(error) = result {
            self.error = Some(error);
        } else if command == menus::Command::Delete {
            if let Some(id) = ctx.memory(|m| m.focused()) {
                ctx.memory_mut(|m| m.surrender_focus(id));
            }
            self.source_navigation = Default::default();
        }
        true
    }
    fn focused_resource_row(&self, ctx: &egui::Context) -> Option<ResourceRow> {
        let row = ctx
            .memory(|m| m.focused())
            .and_then(|id| self.source_navigation.focus.get(&id).copied())?;
        // Consecutive commands can precede the next navigator frame. A freshly
        // duplicated source owns that pending focus, while library rows remain explicit.
        Some(match row {
            ResourceRow::Source(_) => self
                .source_navigation
                .focus_to
                .map(ResourceRow::Source)
                .unwrap_or(row),
            ResourceRow::Library(_) => row,
        })
    }
    fn duplicate_source_resource(&mut self, source: Id) -> Result<(), String> {
        self.end_pointer_gestures();
        self.editor.duplicate_source(source)?;
        self.source_navigation.focus_to = self.editor.definition;
        Ok(())
    }
    pub(super) fn new_component_library(&mut self) {
        if !self.finish_page_notes() {
            return;
        }
        self.end_pointer_gestures();
        let name = format!(
            "Component library {}",
            self.editor.document.libraries.len() + 1
        );
        if let Err(error) = self.editor.create_library(&name) {
            self.error = Some(error);
        }
    }
    fn new_empty_source(&mut self, library: Id) {
        if !self.finish_page_notes() {
            return;
        }
        self.end_pointer_gestures();
        if let Err(error) = self.editor.create_empty_source(
            library,
            "New component",
            wireframe::document::Point { x: 6., y: 6. },
        ) {
            self.error = Some(error);
        }
    }
    pub(super) fn source_navigator(&mut self, ui: &mut egui::Ui) {
        if self.source_navigation.release_focus {
            if let Some(id) = ui.ctx().memory(|m| m.focused())
                && self.source_navigation.focus.contains_key(&id)
            {
                ui.ctx().memory_mut(|m| m.surrender_focus(id));
            }
            self.source_navigation.release_focus = false;
        }
        self.source_navigation.focus.clear();
        self.sync_resource_selection();
        let mut visible = Vec::new();
        let mut drag_rows = Vec::new();
        let width = ui.available_width();
        egui::Frame::NONE
            .fill(chrome::panel_heading(ui, &self.menus))
            .inner_margin(egui::Margin::symmetric(12, 9))
            .show(ui, |ui| {
                ui.set_width((width - 24.).max(0.));
                ui.label(
                    egui::RichText::new("Components").font(wireframe::fonts::panel_heading_font()),
                );
            });
        ui.separator();
        let libraries: Vec<_> = self
            .editor
            .document
            .live_libraries()
            .map(|l| {
                let sources: Vec<_> = l
                    .sources
                    .iter()
                    .filter_map(|s| {
                        self.editor
                            .document
                            .definitions
                            .iter()
                            .find(|d| d.id == s.definition)
                            .map(|d| (d.id, d.name.clone()))
                    })
                    .collect();
                (l.id, l.name.clone(), sources)
            })
            .collect();
        egui::ScrollArea::vertical()
            .id_salt("source-navigator")
            .show(ui, |ui| {
                for (library, name, sources) in libraries {
                    visible.push(ResourceRow::Library(library));
                    let selected = self
                        .source_navigation
                        .selected
                        .contains(&ResourceRow::Library(library));
                    let mut state =
                        egui::collapsing_header::CollapsingState::load_with_default_open(
                            ui.ctx(),
                            ui.id().with(("source-library", library)),
                            true,
                        );
                    if self.source_navigation.reveal_library == Some(library) {
                        state.set_open(true);
                        self.source_navigation.reveal_library = None;
                    }
                    state
                        .show_header(ui, |ui| {
                            let response = ui_text::selectable(
                                ui,
                                selected,
                                &name,
                                (ui.available_width() - 40.).max(0.),
                            );
                            drag_rows.push((
                                ResourceRow::Library(library),
                                response.rect.intersect(ui.clip_rect()),
                            ));
                            self.source_navigation
                                .focus
                                .insert(response.id, ResourceRow::Library(library));
                            if response.clicked() && self.finish_page_notes() {
                                response.request_focus();
                                self.end_pointer_gestures();
                                if let Err(error) = self.select_resource_row(
                                    ResourceRow::Library(library),
                                    activation::click_modifiers(ui),
                                ) {
                                    self.error = Some(error);
                                }
                            }
                            response.context_menu(|ui| {
                                for (label, cut) in [("Copy", false), ("Cut", true)] {
                                    if ui.button(label).clicked() {
                                        if self.finish_page_notes() {
                                            self.end_pointer_gestures();
                                            if let Err(error) = self.copy_selected_resource(
                                                ui.ctx(),
                                                ResourceRow::Library(library),
                                                cut,
                                            ) {
                                                self.error = Some(error);
                                            }
                                        }
                                        ui.close();
                                    }
                                }
                                if ui
                                    .add_enabled(
                                        self.source_navigation.selected.len() <= 1,
                                        egui::Button::new("Rename…"),
                                    )
                                    .clicked()
                                {
                                    self.begin_source_rename(library, true);
                                    ui.close();
                                }
                                if ui.button("New component").clicked() {
                                    self.new_empty_source(library);
                                    ui.close();
                                }
                                if ui.button("Paste").clicked() {
                                    if self.finish_page_notes() {
                                        match self.editor.edit_library(library) {
                                            Ok(()) => ui.ctx().send_viewport_cmd(
                                                egui::ViewportCommand::RequestPaste,
                                            ),
                                            Err(error) => self.error = Some(error),
                                        }
                                    }
                                    ui.close();
                                }
                                ui.separator();
                                let count = self.library_trash_selection(library).len();
                                let label = if count == 1 {
                                    "Move Library to Trash".into()
                                } else {
                                    format!("Move {count} Libraries to Trash")
                                };
                                if ui
                                    .add_enabled(count > 0, egui::Button::new(label))
                                    .clicked()
                                {
                                    self.move_library_to_trash(library);
                                    ui.close();
                                }
                            });
                            if ui
                                .small_button("+")
                                .on_hover_text("New component")
                                .clicked()
                            {
                                self.new_empty_source(library);
                            }
                        })
                        .body(|ui| {
                            for (source, name) in sources {
                                visible.push(ResourceRow::Source(source));
                                ui.push_id(("source-row", source), |ui| {
                                    let response = ui_text::selectable(
                                        ui,
                                        self.source_navigation
                                            .selected
                                            .contains(&ResourceRow::Source(source)),
                                        &name,
                                        ui.available_width(),
                                    );
                                    drag_rows.push((
                                        ResourceRow::Source(source),
                                        response.rect.intersect(ui.clip_rect()),
                                    ));
                                    self.source_navigation
                                        .focus
                                        .insert(response.id, ResourceRow::Source(source));
                                    if self.source_navigation.focus_to == Some(source) {
                                        response.request_focus();
                                        self.source_navigation.selected =
                                            HashSet::from([ResourceRow::Source(source)]);
                                        self.source_navigation.anchor =
                                            Some(ResourceRow::Source(source));
                                        self.source_navigation.surface =
                                            Some(ResourceRow::Source(source));
                                        self.source_navigation.focus_to = None;
                                    }
                                    if response.clicked() && self.finish_page_notes() {
                                        response.request_focus();
                                        self.end_pointer_gestures();
                                        if let Err(error) = self.select_source_row(
                                            source,
                                            activation::click_modifiers(ui),
                                        ) {
                                            self.error = Some(error);
                                        }
                                    }
                                    response.context_menu(|ui| {
                                        if ui
                                            .add_enabled(
                                                self.source_navigation.selected.len() <= 1,
                                                egui::Button::new("Rename…"),
                                            )
                                            .clicked()
                                        {
                                            self.begin_source_rename(source, false);
                                            ui.close();
                                        }
                                        self.source_move_menu(ui, source);
                                        // Context Duplicate targets this row, even in a range.
                                        if ui.button("Duplicate").clicked() {
                                            if self.finish_page_notes()
                                                && let Err(error) =
                                                    self.duplicate_source_resource(source)
                                            {
                                                self.error = Some(error);
                                            }
                                            ui.close();
                                        }
                                        for (label, cut) in [("Copy", false), ("Cut", true)] {
                                            if ui.button(label).clicked() {
                                                if self.finish_page_notes() {
                                                    self.end_pointer_gestures();
                                                    if let Err(error) = self.copy_source_resource(
                                                        ui.ctx(),
                                                        source,
                                                        cut,
                                                    ) {
                                                        self.error = Some(error);
                                                    }
                                                }
                                                ui.close();
                                            }
                                        }
                                        if ui.button("Paste").clicked() {
                                            if self.finish_page_notes() {
                                                match self.editor.edit_library_source(source) {
                                                    Ok(()) => ui.ctx().send_viewport_cmd(
                                                        egui::ViewportCommand::RequestPaste,
                                                    ),
                                                    Err(error) => self.error = Some(error),
                                                }
                                            }
                                            ui.close();
                                        }
                                        let count = self.source_resource_selection(source).len();
                                        let label = if count == 1 {
                                            "Delete component source".into()
                                        } else {
                                            format!("Delete {count} component sources")
                                        };
                                        if ui
                                            .add_enabled(count > 0, egui::Button::new(label))
                                            .clicked()
                                        {
                                            if self.finish_page_notes()
                                                && let Err(error) = self.editor.delete_definitions(
                                                    &self.source_resource_selection(source),
                                                )
                                            {
                                                self.error = Some(error);
                                            }
                                            ui.close();
                                        }
                                    });
                                });
                            }
                        });
                }
                self.source_drag(ui, &drag_rows);
            });
        self.source_navigation.visible = visible;
    }
    pub(super) fn component_usage_panel(&mut self, ui: &mut egui::Ui) {
        let Some(source) = self.editor.definition else {
            return;
        };
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(16, 10))
            .show(ui, |ui| {
                ui.strong("Used in");
                let uses = self.editor.component_usages(source);
                if uses.is_empty() {
                    ui.weak("This component is not used yet.");
                }
                // One destination link per surface; multiple instances share that destination.
                let mut seen = HashSet::new();
                for usage in uses {
                    if !seen.insert(usage.surface) {
                        continue;
                    }
                    let label = self
                        .editor
                        .document
                        .surface(usage.surface)
                        .map(|(name, _)| name.to_owned())
                        .unwrap_or_else(|| "Component".into());
                    if ui.link(label).clicked() && self.finish_page_notes() {
                        self.end_pointer_gestures();
                        if let Err(error) = self.editor.visit_component_usage(source, &usage) {
                            self.error = Some(error);
                        }
                    }
                }
            });
    }
}

#[cfg(test)]
mod tests;
