// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Whole-library review owns choices until one complete import can be applied.
use super::*;
use std::collections::BTreeMap;
use wireframe::workspace::{
    LibraryImportAction, LibraryImportChoice as Choice, LibraryImportPlan, PreparedLibraryImport,
};
mod preview;

pub(super) struct Review {
    clipboard: Clipboard,
    plan: LibraryImportPlan,
    choices: BTreeMap<Id, Choice>,
    all_remaining: bool,
    prepared: Option<PreparedLibraryImport>,
    error: Option<String>,
    preview_editor: Option<Result<Editor, String>>,
    preview: Option<(Id, preview::Pair)>,
}
impl Review {
    fn new(clipboard: Clipboard, plan: LibraryImportPlan) -> Self {
        Self {
            clipboard,
            plan,
            choices: BTreeMap::new(),
            all_remaining: false,
            prepared: None,
            error: None,
            preview_editor: None,
            preview: None,
        }
    }
    fn remaining(&self) -> impl Iterator<Item = &wireframe::workspace::LibraryImportEntry> {
        self.plan.entries().iter().filter(|e| {
            e.action == LibraryImportAction::Conflict && !self.choices.contains_key(&e.incoming)
        })
    }
    fn reset_choices(&mut self) {
        self.choices.clear();
        self.prepared = None;
        self.all_remaining = false;
        self.error = None;
    }
}
impl WireframeApp {
    pub(super) fn copy_library_resources(
        &mut self,
        ctx: &egui::Context,
        libraries: &[Id],
    ) -> Result<(), String> {
        let copy = self
            .editor
            .copy_libraries(libraries, &self.clipboard_origin())?;
        self.publish_clipboard(ctx, copy)?;
        self.status = format!(
            "Copied {} component {}",
            libraries.len(),
            if libraries.len() == 1 {
                "library"
            } else {
                "libraries"
            }
        );
        Ok(())
    }
    pub(super) fn start_library_import(&mut self, copy: &Clipboard) {
        // A second clipboard event must not replace an in-flight review.
        if self.library_import.is_some() || !self.finish_page_notes() {
            return;
        }
        self.end_pointer_gestures();
        match self
            .editor
            .plan_library_import(copy, &self.clipboard_origin())
        {
            Ok(plan) => {
                self.library_import = Some(Review::new(copy.clone(), plan));
                self.prepare_library_choices();
            }
            Err(error) => self.error = Some(error),
        }
    }
    fn refresh_library_review(&mut self) -> bool {
        let Some(review) = &self.library_import else {
            return false;
        };
        let origin = self.clipboard_origin();
        if review
            .plan
            .matches_destination(&self.editor.document, &origin)
        {
            return true;
        }
        let copy = review.clipboard.clone();
        match self.editor.plan_library_import(&copy, &origin) {
            Ok(plan) => {
                let mut next = Review::new(copy, plan);
                next.error =
                    Some("The project changed. Review the import again before continuing.".into());
                self.library_import = Some(next);
            }
            Err(error) => {
                self.library_import = None;
                self.error = Some(error);
            }
        }
        false
    }
    fn choose_library_import(&mut self, choice: Choice) {
        if !self.refresh_library_review() {
            return;
        }
        let review = self.library_import.as_mut().unwrap();
        let ids: Vec<_> = review.remaining().map(|e| e.incoming).collect();
        let count = if review.all_remaining { ids.len() } else { 1 };
        for id in ids.into_iter().take(count) {
            review.choices.insert(id, choice);
        }
        self.prepare_library_choices();
    }
    fn prepare_library_choices(&mut self) {
        if !self.refresh_library_review() {
            return;
        }
        let review = self.library_import.as_mut().unwrap();
        if review.remaining().next().is_some() {
            return;
        }
        match review.plan.prepare(&review.choices) {
            Ok(prepared) => {
                let affected = prepared.newly_orphaned_overrides();
                review.prepared = Some(prepared);
                if affected == 0 {
                    self.apply_reviewed_libraries();
                }
            }
            Err(error) => review.error = Some(error),
        }
    }
    fn apply_reviewed_libraries(&mut self) {
        if !self.refresh_library_review() {
            return;
        }
        let Some(mut review) = self.library_import.take() else {
            return;
        };
        let Some(prepared) = review.prepared.take() else {
            self.library_import = Some(review);
            return;
        };
        match self
            .editor
            .apply_library_import(prepared, &self.clipboard_origin())
        {
            Ok(counts) => {
                self.collapse_resource_selection();
                self.status = format!(
                    "Component libraries: {} added, {} replaced, {} reused",
                    counts.added, counts.replaced, counts.reused
                );
            }
            Err(error) => self.error = Some(error),
        }
    }
    pub(super) fn library_import_dialog(&mut self, ctx: &egui::Context) {
        if self.library_import.is_none() {
            return;
        }
        self.refresh_library_review();
        let Some(review) = self.library_import.as_mut() else {
            return;
        };
        let mut choice = None;
        let mut cancel = false;
        let mut retry = false;
        let mut apply = false;
        let entry = review.remaining().next().cloned();
        if let Some(entry) = &entry {
            review.prepare_preview(&self.editor, entry);
        }
        let can_review = review
            .plan
            .entries()
            .iter()
            .any(|entry| entry.action == LibraryImportAction::Conflict);
        egui::Modal::new(egui::Id::new("library-import-review"))
            .frame(egui::Frame::NONE.corner_radius(12).fill(Color32::from_gray(221)).inner_margin(16))
            .show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 64.).clamp(280., 900.));
            if let Some(error) = &review.error {
                ui.heading("Review component library import");
                ui.label(error);
                retry = ui.button("Review choices").clicked();
            } else if let Some(prepared) = &review.prepared {
                ui.heading("Review unresolved component edits");
                ui.label(format!("{} additional component edits cannot be displayed after this import because their target controls are missing. Their values remain stored. Repeated uses are counted separately.", prepared.newly_orphaned_overrides()));
                if can_review {
                    ui.label("Review your choices to keep both libraries and preserve existing components, or import with these changes.");
                }
                ui.horizontal(|ui| {
                    if can_review { retry = ui.button("Review choices").clicked(); }
                    apply = ui.button("Import with changes").clicked();
                });
            } else if let Some(entry) = &entry {
                ui.label(format!("The library “{}” already exists in this project. What would you like to do?", entry.name));
                if let Some((_, pair)) = &review.preview { pair.show(ui); }
                if entry.has_unmapped_overrides {
                    ui.label("Some incoming edits refer to missing controls. Keep Both can preserve them without attaching them to different controls.");
                }
                let remaining = review.remaining().count() - 1;
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    if remaining > 0 {
                        ui.checkbox(&mut review.all_remaining, format!("Apply to remaining {remaining} {}", if remaining == 1 { "conflict" } else { "conflicts" }));
                    }
                    if ui.button("Cancel Paste Libraries").clicked() { cancel = true; }
                    for (label, value, help) in [("Keep Both", Choice::KeepBoth, "Add a separate library"), ("Use Existing", Choice::UseExisting, "Keep the existing library and its contents"), ("Use New", Choice::UseNew, "Replace the existing library with the incoming contents")] {
                        let button = if value == Choice::UseNew {
                            egui::Button::new(egui::RichText::new(label).color(Color32::WHITE)).fill(ACCENT)
                        } else { egui::Button::new(label) };
                        if ui.add(button).on_hover_text(help).clicked() { choice = Some(value); }
                    }
                });
            }
            if entry.is_none() || review.error.is_some() || review.prepared.is_some() {
                cancel |= ui.button("Cancel Paste Libraries").clicked();
            }
            cancel |= ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        });
        if cancel {
            self.library_import = None;
        } else if retry {
            self.library_import.as_mut().unwrap().reset_choices();
            self.prepare_library_choices();
        } else if let Some(choice) = choice {
            self.choose_library_import(choice);
        } else if apply {
            self.apply_reviewed_libraries();
        }
    }
}

#[cfg(test)]
mod tests;
