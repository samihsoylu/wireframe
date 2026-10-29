// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::pdf::{self, Orientation, Paper, Selection};
impl WireframeApp {
    pub(super) fn poll_pdf_export(&mut self, ctx: &egui::Context) {
        if self
            .pdf_job
            .as_ref()
            .is_some_and(pdf::ExportJob::is_finished)
        {
            let quit = std::mem::take(&mut self.quit_after_pdf);
            match self.pdf_job.take().unwrap().finish() {
                Ok(path) => {
                    self.status = format!("Exported {}", path.display());
                    if quit {
                        self.request(Pending::Quit, ctx);
                    }
                }
                Err(error) => {
                    self.status = "PDF export failed".into();
                    self.error = Some(error);
                }
            }
        }
        if self.pdf_job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
    pub(super) fn pdf_export_dialog(&mut self, ctx: &egui::Context) {
        let Some(options) = &mut self.pdf_options else {
            return;
        };
        let mut cancel = false;
        let mut export = false;
        let entries = pdf::entries(&self.editor.document);
        egui::Modal::new(egui::Id::new("pdf-export-options")).show(ctx, |ui| {
            ui.set_width(430.);
            ui.heading("Export PDF");
            ui.horizontal(|ui| {
                ui.radio_value(&mut options.selection, Selection::All, "All pages");
                ui.radio_value(
                    &mut options.selection,
                    Selection::Current(self.editor.active_surface(self.editor.page)),
                    "Current page",
                );
                if ui
                    .radio(
                        matches!(options.selection, Selection::Set(_)),
                        "Choose pages",
                    )
                    .clicked()
                    && !matches!(options.selection, Selection::Set(_))
                {
                    options.selection =
                        Selection::Set([self.editor.active_surface(self.editor.page)].into());
                }
            });
            if matches!(options.selection, Selection::All) {
                ui.checkbox(
                    &mut options.include_alternates,
                    "Include alternate versions",
                );
            }
            if let Selection::Set(ids) = &mut options.selection {
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button("All").clicked() {
                        *ids = entries.iter().map(|e| e.id).collect();
                    }
                    if ui.small_button("Official versions").clicked() {
                        *ids = entries
                            .iter()
                            .filter(|e| !e.alternate)
                            .map(|e| e.id)
                            .collect();
                    }
                    if ui.small_button("Active").clicked() {
                        *ids = self
                            .editor
                            .document
                            .live_pages()
                            .map(|p| self.editor.active_surface(p.id))
                            .collect();
                    }
                    if ui.small_button("Current").clicked() {
                        *ids = [self.editor.active_surface(self.editor.page)].into();
                    }
                    if ui.small_button("None").clicked() {
                        ids.clear();
                    }
                });
                egui::ScrollArea::vertical()
                    .max_height(180.)
                    .show(ui, |ui| {
                        for entry in &entries {
                            let mut selected = ids.contains(&entry.id);
                            if ui.checkbox(&mut selected, &entry.name).changed() {
                                if selected {
                                    ids.insert(entry.id);
                                } else {
                                    ids.remove(&entry.id);
                                }
                            }
                        }
                    });
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Page size");
                egui::ComboBox::from_id_salt("pdf-paper")
                    .selected_text(
                        options
                            .paper
                            .map_or("Content sized".into(), |p| format!("{p:?}")),
                    )
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut options.paper, None, "Content sized");
                        for paper in Paper::ALL {
                            ui.selectable_value(
                                &mut options.paper,
                                Some(paper),
                                format!("{paper:?}"),
                            );
                        }
                    });
            });
            if options.paper.is_none() {
                ui.checkbox(&mut options.margins, "Add margins");
            } else {
                ui.horizontal(|ui| {
                    ui.label("Orientation");
                    for (value, label) in [
                        (Orientation::BestFit, "Best fit"),
                        (Orientation::Portrait, "Portrait"),
                        (Orientation::Landscape, "Landscape"),
                    ] {
                        ui.radio_value(&mut options.orientation, value, label);
                    }
                });
                ui.small("Large wireframes shrink to fit; small wireframes keep their size.");
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.checkbox(&mut options.names, "Page names");
                ui.checkbox(&mut options.numbers, "Page numbers");
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut options.notes, "Notes");
                ui.checkbox(&mut options.hints, "Highlight links");
            });
            ui.small("Links to pages outside this export are omitted.");
            ui.separator();
            ui.horizontal(|ui| {
                let enabled = !matches!(&options.selection,Selection::Set(ids) if ids.is_empty());
                export = ui
                    .add_enabled(enabled, egui::Button::new("Export…"))
                    .clicked();
                cancel = ui.button("Cancel").clicked();
            });
        });
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            cancel = true;
        }
        if cancel {
            self.pdf_options = None;
        } else if export {
            let options = self.pdf_options.as_ref().unwrap().clone();
            if let Some(mut path) = self.menus.with_dialog(|| {
                rfd::FileDialog::new()
                    .add_filter("PDF document", &["pdf"])
                    .set_file_name(format!("{}.pdf", self.editor.document.title))
                    .save_file()
            }) {
                if !path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
                {
                    path.set_extension("pdf");
                }
                if let Err(error) = self.ensure_export_destination(&path) {
                    self.error = Some(error);
                    return;
                }
                self.end_pointer_gestures();
                match pdf::ExportJob::start(self.editor.document.clone(), options, path) {
                    Ok(job) => {
                        self.pdf_job = Some(job);
                        self.pdf_options = None;
                        self.status = "Exporting PDF… You can keep editing.".into();
                    }
                    Err(error) => self.error = Some(error),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_export_reports_failure_and_cancels_deferred_quit() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("output.pdf");
        std::fs::write(&path, b"prior output").unwrap();
        app.pdf_job = Some(
            pdf::ExportJob::start(
                app.editor.document.clone(),
                pdf::Options {
                    selection: Selection::Set(Default::default()),
                    ..Default::default()
                },
                path.clone(),
            )
            .unwrap(),
        );
        app.request(Pending::Quit, &ctx);
        assert!(app.quit_after_pdf);
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !app.pdf_job.as_ref().unwrap().is_finished() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        app.poll_pdf_export(&ctx);
        assert_eq!(app.status, "PDF export failed");
        assert!(app.error.as_ref().unwrap().contains("Choose between"));
        assert!(
            !app.error
                .as_ref()
                .unwrap()
                .contains("Couldn't save the PDF")
        );
        assert!(!app.quit_after_pdf);
        assert!(!app.quit_allowed);
        assert!(app.pending.is_none());
        assert!(app.pdf_job.is_none());
        assert_eq!(std::fs::read(path).unwrap(), b"prior output");
    }

    #[test]
    fn pdf_write_failure_has_exact_details_preserves_output_work_and_history() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Existing output.pdf");
        std::fs::create_dir(&path).unwrap();
        let sentinel = path.join("keep.txt");
        std::fs::write(&sentinel, b"existing output contents").unwrap();
        let raw = wireframe::storage::write_atomic(&path, b"not published").unwrap_err();
        app.editor.add(Kind::Label, 20., 30.);
        let edited = app.editor.document.clone();
        app.editor.undo();
        let baseline = app.editor.document.clone();
        let redo = app.editor.redo_name().map(str::to_owned);
        app.pdf_job = Some(
            pdf::ExportJob::start(
                app.editor.document.clone(),
                pdf::Options::default(),
                path.clone(),
            )
            .unwrap(),
        );
        app.request(Pending::Quit, &ctx);
        assert!(app.quit_after_pdf);
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !app.pdf_job.as_ref().unwrap().is_finished() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(2));
        }
        app.poll_pdf_export(&ctx);
        let error = app.error.as_deref().unwrap();
        let (primary, details) = error.split_once("\n\nDetails: ").unwrap();
        assert!(primary.contains("Couldn't save the PDF"));
        assert!(primary.contains("Choose a different folder"));
        assert_eq!(details, raw);
        assert_eq!(app.status, "PDF export failed");
        assert!(!app.quit_after_pdf && !app.quit_allowed && app.pending.is_none());
        assert_eq!(app.editor.document, baseline);
        assert_eq!(app.editor.redo_name(), redo.as_deref());
        app.editor.redo();
        assert_eq!(app.editor.document, edited);
        assert!(path.is_dir());
        assert_eq!(
            std::fs::read(sentinel).unwrap(),
            b"existing output contents"
        );
    }
}
