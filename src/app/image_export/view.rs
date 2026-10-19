// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::raster::job::{Choice, Decision};

impl WireframeApp {
    pub(in crate::app) fn image_export_dialogs(&mut self, ctx: &egui::Context) {
        self.image_export_options(ctx);
        self.image_export_progress(ctx);
        self.image_export_report(ctx);
    }
    fn image_export_options(&mut self, ctx: &egui::Context) {
        let Some(draft) = &mut self.image_export.draft else {
            return;
        };
        let mut export = false;
        let mut cancel = false;
        egui::Modal::new(egui::Id::new("image-export-options")).show(ctx, |ui| {
            ui.set_width(450.);
            ui.heading("Export images");
            ui.horizontal(|ui| {
                ui.add_enabled_ui(!draft.component, |ui| {
                    ui.radio_value(&mut draft.options.range, Range::All, "All pages");
                });
                ui.radio_value(&mut draft.options.range, Range::Current, "Current page");
                ui.add_enabled_ui(!draft.component, |ui| {
                    if ui
                        .radio_value(&mut draft.options.range, Range::Chosen, "Choose pages")
                        .clicked()
                        && draft.options.chosen.is_empty()
                    {
                        draft.options.chosen.insert(draft.current);
                    }
                });
            });
            if draft.options.range == Range::Chosen {
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button("All").clicked() {
                        draft.options.chosen = draft.entries.iter().map(|e| e.id).collect();
                    }
                    if draft.entries.iter().any(|e| e.alternate_name.is_some())
                        && ui.small_button("Official versions").clicked()
                    {
                        draft.options.chosen = draft
                            .entries
                            .iter()
                            .filter(|e| e.alternate_name.is_none())
                            .map(|e| e.id)
                            .collect();
                    }
                    if ui
                        .small_button("Active")
                        .on_hover_text("The currently selected version of each page")
                        .clicked()
                    {
                        draft.options.chosen = draft.active.clone();
                    }
                    if ui.small_button("Current").clicked() {
                        draft.options.chosen = [draft.current].into();
                    }
                    if ui.small_button("None").clicked() {
                        draft.options.chosen.clear();
                    }
                });
                egui::ScrollArea::vertical()
                    .max_height(210.)
                    .show(ui, |ui| {
                        for entry in &draft.entries {
                            let mut selected = draft.options.chosen.contains(&entry.id);
                            if ui.checkbox(&mut selected, entry.display_name()).changed() {
                                if selected {
                                    draft.options.chosen.insert(entry.id);
                                } else {
                                    draft.options.chosen.remove(&entry.id);
                                }
                            }
                        }
                    });
            }
            draft.reconcile_scope();
            if draft.is_current_only() && (!draft.group.is_empty() || !draft.selected.is_empty()) {
                ui.separator();
                ui.label("Include");
                ui.radio_value(&mut draft.options.scope, Scope::Whole, "Whole page");
                if !draft.group.is_empty() {
                    ui.radio_value(
                        &mut draft.options.scope,
                        Scope::Group,
                        "Current group's contents",
                    );
                }
                if !draft.selected.is_empty() {
                    ui.radio_value(
                        &mut draft.options.scope,
                        Scope::Selection,
                        "Selected controls",
                    );
                }
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Resolution");
                ui.radio_value(&mut draft.options.double, false, "1×");
                ui.radio_value(&mut draft.options.double, true, "2×");
            });
            ui.horizontal(|ui| {
                ui.label("Background");
                ui.radio_value(&mut draft.options.transparent, false, "White");
                ui.radio_value(&mut draft.options.transparent, true, "Transparent");
            });
            let count = draft.chosen().len();
            if count > 1 {
                ui.checkbox(&mut draft.options.archive, "Save in one ZIP archive");
            }
            ui.small(format!("{count} page(s) selected"));
            ui.separator();
            ui.horizontal(|ui| {
                export = ui
                    .add_enabled(draft.request().is_ok(), egui::Button::new("Export…"))
                    .clicked();
                cancel = ui.button("Cancel").clicked();
            });
        });
        cancel |= ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        if cancel {
            self.image_export.draft = None;
        } else if export {
            self.choose_image_destination();
        }
    }
    fn image_export_progress(&mut self, ctx: &egui::Context) {
        let Some(status) = self.image_export.status.clone() else {
            return;
        };
        if let Phase::Conflict { id, path } = &status.phase {
            if self.image_export.conflict_id != Some(*id) {
                self.image_export.conflict_id = Some(*id);
                self.image_export.apply_all = false;
            }
            let mut choice = None;
            egui::Modal::new(egui::Id::new("image-export-conflict")).show(ctx, |ui| {
                ui.set_width(440.);
                ui.heading("A file already exists");
                ui.label(path.display().to_string());
                ui.label(
                    "Replace the existing file, skip this output, or keep both with a new name.",
                );
                ui.checkbox(
                    &mut self.image_export.apply_all,
                    "Apply to all conflicts in this export",
                );
                ui.horizontal(|ui| {
                    for (value, label) in [
                        (Choice::Replace, "Replace"),
                        (Choice::Skip, "Skip"),
                        (Choice::KeepBoth, "Keep Both"),
                        (Choice::Cancel, "Cancel export"),
                    ] {
                        if ui
                            .add_enabled(!status.cancel_requested, egui::Button::new(label))
                            .clicked()
                        {
                            choice = Some(value);
                        }
                    }
                });
                if status.cancel_requested {
                    ui.label("Cancelling…");
                }
            });
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                choice = Some(Choice::Cancel);
            }
            if let (Some(choice), Some(job)) = (choice, self.image_export.job.as_ref()) {
                if choice == Choice::Cancel {
                    job.cancel();
                } else {
                    job.decide(
                        *id,
                        Decision {
                            choice,
                            apply_to_all: self.image_export.apply_all,
                        },
                    );
                }
            }
            return;
        }
        egui::Window::new("Exporting images")
            .id(egui::Id::new("image-export-progress"))
            .anchor(Align2::RIGHT_BOTTOM, [-20., -20.])
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.set_width(290.);
                match &status.phase {
                    Phase::Preparing => {
                        ui.label("Preparing images…");
                    }
                    Phase::Rendering {
                        page,
                        name,
                        rows,
                        total_rows,
                    } => {
                        ui.label(format!(
                            "Page {} of {} · {name}",
                            page + 1,
                            status.total_pages
                        ));
                        ui.add(
                            egui::ProgressBar::new(*rows as f32 / (*total_rows).max(1) as f32)
                                .show_percentage(),
                        );
                    }
                    Phase::Publishing(path) => {
                        ui.label(format!(
                            "Saving {}…",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        ));
                    }
                    Phase::Finished => {
                        ui.label("Finishing…");
                    }
                    Phase::Conflict { .. } => {}
                }
                ui.small(format!(
                    "{} exported · {} skipped",
                    status.exported_pages, status.skipped_pages
                ));
                if status.cancel_requested {
                    ui.label("Cancelling… Completed files will be kept.");
                } else if ui.button("Cancel export").clicked()
                    && let Some(job) = &self.image_export.job
                {
                    job.cancel();
                }
                if self.image_export.quit_after {
                    ui.small("The window will close when this export finishes successfully.");
                }
            });
    }
    fn image_export_report(&mut self, ctx: &egui::Context) {
        if !self.image_export.show_report {
            return;
        }
        let Some(report) = &self.image_export.report else {
            return;
        };
        let mut close = false;
        egui::Modal::new(egui::Id::new("image-export-report")).show(ctx, |ui| {
            ui.set_width(450.);
            ui.heading(match report.outcome {
                Outcome::Completed => "Images exported",
                Outcome::Cancelled => "Export cancelled",
                _ => "Export stopped",
            });
            ui.label(report_status(report));
            if let Outcome::Failed(error) = &report.outcome {
                ui.label(error);
            }
            if !report.files.is_empty() {
                ui.label("Completed files have been kept.");
            }
            egui::ScrollArea::vertical()
                .max_height(220.)
                .show(ui, |ui| {
                    for page in &report.reduced_pages {
                        ui.label(format!(
                            "{} was reduced to {} × {} pixels to fit the image size limit.",
                            page.name, page.width, page.height
                        ));
                    }
                    for file in &report.files {
                        if let Some(warning) = &file.durability_warning {
                            ui.label(format!("{}: {warning}", file.path.display()));
                        }
                    }
                });
            close = ui.button("Close").clicked();
        });
        close |= ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        if close {
            self.image_export.show_report = false;
        }
    }
}
