// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Compact editor chrome; command behavior stays in the editor and viewport modules.
mod icons;
#[cfg(test)]
mod tests;
use super::*;
use icons::{Icon, button};

impl WireframeApp {
    pub(super) fn toolbar(&mut self, ui: &mut egui::Ui) {
        self.begin_in_window_menus(ui.ctx());
        if self.present {
            self.presentation_toolbar(ui);
            return;
        }
        egui::Panel::top("toolbar")
            .frame(
                egui::Frame::NONE
                    .fill(chrome::TOOLBAR)
                    .inner_margin(egui::Margin::symmetric(10, 2)),
            )
            .show(ui, |ui| {
                let palette = chrome::toolbar_palette();
                *ui.visuals_mut() = egui::Visuals::dark();
                ui.visuals_mut().disabled_alpha = 0.4;
                ui.visuals_mut().selection.stroke.color = palette.selected;
                ui.visuals_mut().widgets.active.fg_stroke.color = palette.pressed;
                if self.image_crop.is_some() {
                    ui.disable();
                }
                ui.visuals_mut().override_text_color = Some(Color32::from_gray(235));
                ui.spacing_mut().item_spacing = Vec2::new(3., 0.);
                ui.spacing_mut().button_padding = Vec2::new(6., 2.);
                ui.spacing_mut().interact_size.y = 24.;
                self.project_bar(ui);
                self.canvas_bar(ui);
                self.editing_context_bar(ui);
            });
        egui::Panel::bottom("status").show(ui, |ui| {
            if self.image_crop.is_some() {
                ui.disable();
            }
            ui.horizontal(|ui| {
                ui.small(self.status_text());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.small(format!(
                        "{:.0}%  ·  {} controls  ·  {} selected",
                        self.zoom * 100.,
                        self.editor.current().controls.len(),
                        self.editor.selection.len()
                    ));
                });
            });
        });
    }

    fn project_bar(&mut self, ui: &mut egui::Ui) {
        let (row, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 26.), Sense::hover());
        let menu_width = if self.menus.native() { 0. } else { 210. };
        let style_width = 154.;
        let style_right = row.max.x - menu_width;
        let title = Rect::from_min_max(
            row.min,
            egui::pos2(style_right - style_width - 12., row.max.y),
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(title)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                if self.editor.dirty() {
                    ui.label("•").on_hover_text("Unsaved changes");
                }
                ui_text::label(ui, &self.editor.document.title, ui.available_width());
            },
        );
        let style = Rect::from_min_max(
            egui::pos2(style_right - style_width, row.min.y),
            egui::pos2(style_right, row.max.y),
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(style)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| self.style_switcher(ui),
        );
        if self.menus.native() {
            return;
        }
        let menu = Rect::from_min_max(egui::pos2(row.max.x - menu_width, row.min.y), row.max);
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(menu)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                self.application_menu(ui, "Help", |app, ui| {
                    app.menu_item(ui, menus::Command::Help);
                    app.menu_item(ui, menus::Command::About);
                });
                self.view_menu(ui);
                self.edit_menu(ui);
                self.file_menu(ui);
            },
        );
    }

    fn style_switcher(&mut self, ui: &mut egui::Ui) {
        let enabled = self.command_enabled(ui.ctx(), menus::Command::Sketch);
        ui.add_enabled_ui(enabled, |ui| {
            ui.spacing_mut().item_spacing.x = 2.;
            let sketch = self.editor.document.settings.sketch;
            for (label, value, hint) in [
                (
                    "Wireframe",
                    true,
                    "Hand-drawn style for all pages in this project",
                ),
                ("Mockup", false, "Clean style for all pages in this project"),
            ] {
                if ui
                    .selectable_label(sketch == value, label)
                    .on_hover_text(hint)
                    .clicked()
                    && sketch != value
                {
                    self.run_command(ui.ctx(), menus::Command::Sketch);
                }
            }
        });
    }

    fn application_menu(
        &mut self,
        ui: &mut egui::Ui,
        label: &str,
        contents: impl FnOnce(&mut Self, &mut egui::Ui),
    ) {
        let (button, popup) = egui::containers::menu::MenuButton::new(label).ui(ui, |ui| {
            self.menus.drawing_popup = true;
            contents(self, ui);
            self.menus.drawing_popup = false;
        });
        self.menus
            .record_in_window(&button, popup.map(|popup| popup.response.rect));
    }

    fn file_menu(&mut self, ui: &mut egui::Ui) {
        use menus::Command::*;
        self.application_menu(ui, "File", |app, ui| {
            for group in [
                &[NewPage, New, Open][..],
                &[Close, Save, SaveAs, RenameProject],
                &[AutomaticSave, Recover],
                &[ExportPdf, ExportImages, ExportSvg],
                &[Example, Sketch, Font],
                &[Quit],
            ] {
                for &command in group {
                    app.menu_item(ui, command);
                }
                ui.separator();
            }
        });
    }
    fn edit_menu(&mut self, ui: &mut egui::Ui) {
        use menus::Command::*;
        self.application_menu(ui, "Edit", |app, ui| {
            for group in [
                &[Undo, Redo][..],
                &[Cut, Copy, Paste, PasteControls, Delete],
                &[Duplicate, SelectAll],
                &[Group, Ungroup, AutoSize, TreatAsMarkup],
                &[SendToBack, BringToFront, SendBackward, BringForward],
            ] {
                for &command in group {
                    app.menu_item(ui, command);
                }
                ui.separator();
            }
            ui.menu_button("Align", |ui| {
                for &command in menus::Command::ALIGNMENT {
                    app.menu_item(ui, command);
                }
            });
        });
    }
    fn view_menu(&mut self, ui: &mut egui::Ui) {
        use menus::Command::*;
        self.application_menu(ui, "View", |app, ui| {
            for group in [
                &[Pages, Library, Comments, Inspector, ShowMarkup][..],
                &[Grid, Snap],
                &[ZoomIn, ZoomOut, Actual, Fit, AllActual, AllFit],
                &[Present],
            ] {
                for &command in group {
                    app.menu_item(ui, command);
                }
                ui.separator();
            }
        });
    }

    fn canvas_bar(&mut self, ui: &mut egui::Ui) {
        let (row, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 32.), Sense::hover());
        let source_workspace = self.editor.in_source_workspace();
        let has_trash = self.editor.retained_resources().next().is_some();
        let left_width = if source_workspace { 190. } else { 70. };
        let left = Rect::from_min_size(row.min, Vec2::new(left_width, row.height()));
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(left)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 10.;
                if source_workspace {
                    if icons::back_button(ui).clicked() {
                        self.back_to_wireframes();
                    }
                } else {
                    if button(
                        ui,
                        Icon::Pages,
                        "Pages",
                        "Show or hide pages",
                        true,
                        Some(self.show_navigator),
                    )
                    .clicked()
                    {
                        self.show_navigator = !self.show_navigator;
                    }
                    divider(ui);
                }
                let add = if self.editor.in_source_workspace() {
                    "New component library"
                } else {
                    "New wireframe"
                };
                let icon = if source_workspace {
                    Icon::NewLibrary
                } else {
                    Icon::Add
                };
                if button(ui, icon, add, add, true, None).clicked() {
                    if self.editor.in_source_workspace() {
                        self.new_component_library();
                    } else if self.finish_page_notes() {
                        self.editor.add_page();
                    }
                }
            },
        );
        let middle = Rect::from_center_size(
            row.center(),
            Vec2::new(if has_trash { 244. } else { 190. }, row.height()),
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(middle)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 10.;
                if button(
                    ui,
                    Icon::Undo,
                    "Undo",
                    self.editor.undo_name().unwrap_or("Nothing to undo"),
                    self.editor.undo_name().is_some(),
                    None,
                )
                .clicked()
                {
                    self.editor.undo();
                }
                if button(
                    ui,
                    Icon::Redo,
                    "Redo",
                    self.editor.redo_name().unwrap_or("Nothing to redo"),
                    self.editor.redo_name().is_some(),
                    None,
                )
                .clicked()
                {
                    self.editor.redo();
                }
                divider(ui);
                if button(
                    ui,
                    Icon::ZoomOut,
                    "Zoom out",
                    "Zoom out (⌘/Ctrl −)",
                    wireframe::viewport::next_zoom(self.zoom, false).is_some(),
                    None,
                )
                .clicked()
                {
                    self.zoom_step(false);
                }
                if button(
                    ui,
                    Icon::ZoomIn,
                    "Zoom in",
                    "Zoom in (⌘/Ctrl +)",
                    wireframe::viewport::next_zoom(self.zoom, true).is_some(),
                    None,
                )
                .clicked()
                {
                    self.zoom_step(true);
                }
                let actual = button(
                    ui,
                    Icon::ActualSize,
                    "Actual size",
                    "Actual size (⌘/Ctrl 1)
Double-click to reset all page views",
                    true,
                    None,
                );
                if actual.double_clicked() {
                    self.zoom_all(false);
                } else if actual.clicked() {
                    self.actual_size();
                }
                let fit = button(
                    ui,
                    Icon::Fit,
                    "Fit page",
                    "Fit page (⌘/Ctrl 0)
Double-click to fit all pages",
                    true,
                    None,
                );
                if fit.double_clicked() {
                    self.zoom_all(true);
                } else if fit.clicked() {
                    self.fit();
                }
                if has_trash {
                    divider(ui);
                    if button(ui, Icon::Trash, "Trash", "Show Trash", true, None).clicked() {
                        self.open_trash();
                    }
                }
            },
        );
        let right_width = if source_workspace { 185. } else { 265. };
        let right = Rect::from_min_max(egui::pos2(row.max.x - right_width, row.min.y), row.max);
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(right)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 10.;
                if !source_workspace {
                    if button(
                        ui,
                        Icon::Present,
                        "Present",
                        "Full screen presentation",
                        true,
                        None,
                    )
                    .clicked()
                    {
                        self.start_presentation();
                    }
                    divider(ui);
                }
                if button(
                    ui,
                    Icon::Inspector,
                    "Inspector",
                    "Show or hide inspector",
                    true,
                    Some(self.show_inspector && !self.review.open),
                )
                .clicked()
                {
                    self.toggle_review_inspector();
                }
                if !source_workspace
                    && button(
                        ui,
                        Icon::Comments,
                        "Comments",
                        "Show or hide comments",
                        true,
                        Some(self.review.open),
                    )
                    .clicked()
                {
                    self.review.open = !self.review.open;
                }
                if button(
                    ui,
                    Icon::Library,
                    "Library",
                    "Show or hide UI library",
                    true,
                    Some(self.show_library),
                )
                .clicked()
                {
                    self.show_library = !self.show_library;
                }
                ui.scope(|ui| {
                    ui.visuals_mut().extreme_bg_color = Color32::from_gray(125);
                    ui.visuals_mut().weak_text_color = Some(Color32::from_gray(240));
                    self.quick_add_ui(ui);
                });
            },
        );
    }

    fn editing_context_bar(&mut self, ui: &mut egui::Ui) {
        if !self.editor.editing_instance() && self.editor.context.is_empty() {
            return;
        }
        ui.horizontal(|ui| {
            ui.set_height(26.);
            if self.editor.editing_instance() {
                ui.strong("Editing this instance");
            }
            if !self.editor.context.is_empty()
                && ui
                    .button(if self.editor.editing_instance() {
                        "Exit component level"
                    } else {
                        "Exit group"
                    })
                    .clicked()
            {
                self.editor.leave();
            }
        });
    }
}

fn divider(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(10., 20.), Sense::hover());
    ui.painter().line_segment(
        [rect.center_top(), rect.center_bottom()],
        Stroke::new(1., Color32::from_gray(95)),
    );
}

pub(super) fn quick_input(ui: &mut egui::Ui, value: &mut String) -> egui::Response {
    let height = egui::TextStyle::Body.resolve(ui.style()).size * 1.4 + 4.;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(125., height), Sense::hover());
    ui.painter()
        .rect_filled(rect, 2., ui.visuals().extreme_bg_color);
    let icon_rect = Rect::from_min_size(rect.min, Vec2::new(22., height));
    let icon = ui.interact(
        icon_rect,
        ui.make_persistent_id("quick-add-search"),
        Sense::click(),
    );
    icon.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "Focus Quick Add")
    });
    icons::search(ui, icon_rect);
    let response = ui
        .scope_builder(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_max(rect.min + Vec2::new(22., 0.), rect.max))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.style_mut()
                    .text_styles
                    .insert(egui::TextStyle::Body, FontId::proportional(13.));
                ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::NONE;
                ui.visuals_mut().widgets.hovered.bg_stroke = Stroke::NONE;
                ui.visuals_mut().widgets.active.bg_stroke = Stroke::NONE;
                ui.visuals_mut().selection.stroke = Stroke::NONE;
                ui.add(
                    TextInput::singleline(value)
                        .id_salt("quick-add")
                        .hint_text("Quick Add")
                        .desired_width(103.),
                )
            },
        )
        .inner;
    if icon.clicked() {
        response.request_focus();
    }
    if response.has_focus() || icon.has_focus() || response.hovered() || icon.hovered() {
        ui.painter().rect_stroke(
            rect,
            2.,
            ui.visuals().selection.stroke,
            egui::StrokeKind::Inside,
        );
    }
    response
}
