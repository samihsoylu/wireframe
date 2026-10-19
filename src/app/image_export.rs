// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
mod preferences;
mod view;
use super::*;
use preferences::{Options, Range, Scope};
use wireframe::{
    export_plan::{self, Selection},
    raster::{
        self,
        job::{Destination, ExportJob, Outcome, Phase, Report, Status},
        names::{FileName, Format},
        plan::Request,
    },
    workspace::SceneScope,
};
#[derive(Default)]
pub(super) struct ImageExport {
    preferences: preferences::Preferences,
    pub(super) draft: Option<Draft>,
    pub job: Option<ExportJob>,
    status: Option<Status>,
    report: Option<Report>,
    show_report: bool,
    pub quit_after: bool,
    conflict_id: Option<u64>,
    apply_all: bool,
}
pub(super) struct Draft {
    options: Options,
    key: Option<String>,
    entries: Vec<export_plan::Entry>,
    current: Id,
    active: std::collections::BTreeSet<Id>,
    group: Vec<Id>,
    selected: Vec<Vec<Id>>,
    component: bool,
}
impl Draft {
    fn new(editor: &Editor, key: Option<String>, mut options: Options) -> Self {
        let current = editor.surface_id();
        let mut entries = export_plan::entries(&editor.document);
        if let Some(definition) = editor.definition {
            // A component workspace remains a valid current-page export.
            let name = editor.current().name;
            entries = vec![export_plan::Entry {
                id: definition,
                owner: definition,
                page_name: name,
                alternate_name: None,
            }];
            options.range = Range::Current;
        }
        options
            .chosen
            .retain(|id| entries.iter().any(|e| e.id == *id));
        let selected = editor
            .selection
            .iter()
            .map(|id| {
                let mut path = editor.context.clone();
                path.push(*id);
                path
            })
            .collect();
        let mut draft = Self {
            options,
            key,
            entries,
            current,
            active: if editor.definition.is_some() {
                [current].into()
            } else {
                editor
                    .document
                    .live_pages()
                    .map(|p| editor.active_surface(p.id))
                    .collect()
            },
            group: editor.context.clone(),
            selected,
            component: editor.definition.is_some(),
        };
        draft.reconcile_scope();
        draft
    }
    fn selection(&self) -> Selection {
        match self.options.range {
            Range::All => Selection::All,
            Range::Current => Selection::Current(self.current),
            Range::Chosen => Selection::Set(self.options.chosen.clone()),
        }
    }
    fn chosen(&self) -> Vec<&export_plan::Entry> {
        self.entries
            .iter()
            .filter(|e| match self.options.range {
                Range::All => true,
                Range::Current => e.id == self.current,
                Range::Chosen => self.options.chosen.contains(&e.id),
            })
            .collect()
    }
    fn is_current_only(&self) -> bool {
        let chosen = self.chosen();
        chosen.len() == 1 && chosen[0].id == self.current
    }
    fn reconcile_scope(&mut self) {
        if !self.is_current_only()
            || (self.options.scope == Scope::Group && self.group.is_empty())
            || (self.options.scope == Scope::Selection && self.selected.is_empty())
        {
            self.options.scope = Scope::Whole;
        }
    }
    fn request(&self) -> Result<Request, String> {
        if self.chosen().is_empty() {
            return Err("Choose at least one page to export".into());
        }
        let scope = match self.options.scope {
            Scope::Whole => SceneScope::Whole,
            Scope::Group if self.is_current_only() && !self.group.is_empty() => {
                SceneScope::Group(self.group.clone())
            }
            Scope::Selection if self.is_current_only() && !self.selected.is_empty() => {
                SceneScope::Selection(self.selected.clone())
            }
            _ => return Err("Choose a current group or selection to export".into()),
        };
        Ok(Request {
            selection: self.selection(),
            current: self.current,
            scope,
            image: raster::Options {
                resolution: if self.options.double {
                    raster::Resolution::Double
                } else {
                    raster::Resolution::Standard
                },
                transparent: self.options.transparent,
                ..Default::default()
            },
        })
    }
}
impl ImageExport {
    pub(super) fn blocks_input(&self) -> bool {
        self.draft.is_some()
            || self.show_report
            || matches!(
                self.status.as_ref().map(|s| &s.phase),
                Some(Phase::Conflict { .. })
            )
    }
}
impl WireframeApp {
    pub(super) fn load_image_export_preferences(&mut self, root: &std::path::Path) {
        if let Err(error) = self.image_export.preferences.load(root) {
            self.error = Some(format!(
                "Image export options couldn't be read. The file is unchanged.\n{error}"
            ));
        }
    }
    pub(super) fn open_image_export(&mut self) {
        if self.image_export.job.is_some() {
            return;
        }
        self.end_pointer_gestures();
        let key = match self.export_preferences_key() {
            Ok(key) => key,
            Err(error) => {
                self.error = Some(error);
                return;
            }
        };
        let options = self.image_export.preferences.get(key.as_deref());
        self.image_export.draft = Some(Draft::new(&self.editor, key, options));
    }
    pub(super) fn poll_image_export(&mut self, ctx: &egui::Context) {
        if self
            .image_export
            .job
            .as_ref()
            .is_some_and(ExportJob::is_finished)
        {
            let report = self.image_export.job.take().unwrap().finish();
            self.status = report_status(&report);
            let warnings = !report.reduced_pages.is_empty()
                || report.files.iter().any(|f| f.durability_warning.is_some());
            let completed = report.outcome == Outcome::Completed;
            self.image_export.show_report = !completed || warnings;
            self.image_export.status = None;
            self.image_export.report = Some(report);
            let quit = std::mem::take(&mut self.image_export.quit_after);
            if quit && completed && !warnings {
                self.request(Pending::Quit, ctx);
            }
        }
        self.image_export.status = self.image_export.job.as_ref().map(ExportJob::status);
        if self.image_export.job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }
    fn start_image_export(
        &mut self,
        request: Request,
        destination: Destination,
    ) -> Result<(), String> {
        if self.image_export.job.is_some() {
            return Err("An image export is already running".into());
        }
        if let Destination::File(path) = &destination {
            self.ensure_export_destination(path)?;
        }
        self.end_pointer_gestures();
        let snapshot = self.editor.export_snapshot();
        self.image_export.job = Some(ExportJob::start(snapshot, request, destination)?);
        self.image_export.draft = None;
        self.image_export.conflict_id = None;
        self.image_export.show_report = false;
        self.image_export.report = None;
        self.status = "Exporting images… You can keep editing.".into();
        Ok(())
    }
    fn choose_image_destination(&mut self) {
        let Some(draft) = &self.image_export.draft else {
            return;
        };
        let request = match draft.request() {
            Ok(request) => request,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        let chosen = draft.chosen();
        let count = chosen.len();
        let mut options = draft.options.clone();
        let key = draft.key.clone();
        let single_name = chosen
            .first()
            .map(|e| FileName::new(&e.file_stem(), Format::Png).text());
        // Remember explicit Export choices even if the subsequent save panel is cancelled.
        if let Err(error) = self
            .image_export
            .preferences
            .remember(key.as_deref(), options.clone())
        {
            self.error = Some(error);
        }
        let mut dialog = rfd::FileDialog::new();
        if let Some(folder) = &options.folder {
            dialog = dialog.set_directory(folder);
        }
        let destination = if count > 1 && !options.archive {
            self.menus
                .with_dialog(|| dialog.pick_folder())
                .map(|folder| {
                    options.folder = Some(folder.clone());
                    Destination::Folder(folder)
                })
        } else {
            let zip = count > 1;
            let ext = if zip { "zip" } else { "png" };
            let name = if zip {
                FileName::new(
                    options
                        .archive_stem
                        .as_deref()
                        .unwrap_or(&self.editor.document.title),
                    Format::Zip,
                )
                .text()
            } else {
                single_name.unwrap()
            };
            self.menus
                .with_dialog(|| {
                    dialog
                        .add_filter(if zip { "ZIP archive" } else { "PNG image" }, &[ext])
                        .set_file_name(name)
                        .save_file()
                })
                .map(|mut path| {
                    if !path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
                    {
                        path.set_extension(ext);
                    }
                    options.folder = path.parent().map(|p| p.to_owned());
                    if zip {
                        options.archive_stem =
                            path.file_stem().map(|s| s.to_string_lossy().into_owned());
                    }
                    Destination::File(path)
                })
        };
        if let Some(destination) = destination {
            if let Err(error) = self
                .image_export
                .preferences
                .remember(key.as_deref(), options.clone())
            {
                self.error = Some(error);
            }
            if let Some(draft) = &mut self.image_export.draft {
                draft.options = options;
            }
            if let Err(error) = self.start_image_export(request, destination) {
                self.error = Some(error);
            }
        }
    }
}
fn report_status(report: &Report) -> String {
    let prefix = match &report.outcome {
        Outcome::Completed => "Export finished",
        Outcome::Cancelled => "Export cancelled",
        Outcome::Failed(_) => "Export stopped",
        Outcome::Running => "Exporting",
    };
    format!(
        "{prefix}: {} image(s) exported, {} skipped, {} remaining.",
        report.exported_pages,
        report.skipped_pages,
        report.remaining_pages()
    )
}
#[cfg(test)]
mod tests;
