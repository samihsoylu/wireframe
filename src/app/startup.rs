// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Named-file preparation and session handoff run away from the UI thread.
mod menu;
use super::*;
use std::sync::mpsc::{self, Receiver, TryRecvError};

pub(super) enum Prepared {
    Persistent(Box<persistence::PreparedOpen>),
    Plain(Box<Project>),
    Current(Option<Box<Project>>),
    Committed {
        result: Result<Box<Prepared>, String>,
        discarded: bool,
    },
}
#[derive(Clone, Copy)]
enum Phase {
    Startup,
    MenuRead { discard: bool },
    Handoff,
}
pub(super) struct Opening {
    path: PathBuf,
    receiver: Receiver<Result<Prepared, String>>,
    phase: Phase,
    quit_after: bool,
    ctx: egui::Context,
}
impl Opening {
    fn spawn(
        path: PathBuf,
        ctx: egui::Context,
        read: impl FnOnce() -> Result<Prepared, String> + Send + 'static,
    ) -> Result<Self, String> {
        let (sender, receiver) = mpsc::channel();
        let wake = ctx.clone();
        std::thread::Builder::new()
            .name("wireframe-open".into())
            .spawn(move || {
                let result = read();
                // If cancelled, sending drops the candidate and releases its lock.
                let _ = sender.send(result);
                wake.request_repaint();
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            path,
            receiver,
            phase: Phase::Startup,
            quit_after: false,
            ctx,
        })
    }
}
impl WireframeApp {
    pub(super) fn start_startup_open(&mut self, path: PathBuf, ctx: &egui::Context) {
        self.dismiss_recoveries();
        let copies = self.startup_copies();
        let source = path.clone();
        match Opening::spawn(path, ctx.clone(), move || match copies {
            Some(copies) => persistence::PreparedOpen::read(&copies, &source)
                .map(Box::new)
                .map(Prepared::Persistent),
            None => ProjectStore
                .load(&source)
                .map(Box::new)
                .map(Prepared::Plain),
        }) {
            Ok(job) => {
                self.startup_open = Some(job);
                self.status = "Opening project…".into();
            }
            Err(error) => {
                self.error = Some(format!("Could not start opening the project: {error}"))
            }
        }
    }
    pub(super) fn cancel_startup_open(&mut self) {
        if self
            .startup_open
            .as_ref()
            .is_some_and(|job| matches!(job.phase, Phase::Handoff))
        {
            return;
        }
        self.startup_open = None;
        self.status = "Opening cancelled".into();
    }
    pub(super) fn poll_startup_open(&mut self) {
        let Some(job) = &self.startup_open else {
            return;
        };
        let result = match job.receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                Err("The file-opening worker stopped unexpectedly.".into())
            }
        };
        let job = self.startup_open.take().unwrap();
        if let Phase::MenuRead { discard } = job.phase {
            match result.and_then(|prepared| {
                Self::validate_open(&prepared)?;
                Ok(prepared)
            }) {
                Ok(prepared) => self.begin_open_handoff(job.path, prepared, discard, &job.ctx),
                Err(error) => {
                    if discard {
                        self.pending = Some(Pending::Open(job.path.clone()));
                    }
                    self.open_failed(&job.path, error);
                }
            }
            return;
        }
        let result = match result {
            Ok(Prepared::Committed { result, discarded }) => {
                self.complete_open_source(discarded);
                result.map(|prepared| *prepared)
            }
            other => other,
        };
        let accepted = result.and_then(|prepared| self.adopt_open(prepared, &job.path));
        if accepted.is_ok() {
            self.clear_recovery();
            self.refresh_recoveries(false);
        }
        if let Err(error) = accepted {
            self.open_failed(&job.path, error);
        }
        if job.quit_after {
            self.request(Pending::Quit, &job.ctx);
        }
    }
    fn adopt_open(&mut self, prepared: Prepared, path: &std::path::Path) -> Result<(), String> {
        match prepared {
            Prepared::Persistent(prepared) => self.accept_prepared_open(*prepared),
            Prepared::Plain(document) => {
                let candidate = Editor::new(*document);
                for page in candidate.document.live_pages() {
                    candidate.layout_page(page.id)?;
                }
                self.review.reset();
                self.editor = candidate;
                self.file = Some(path.to_owned());
                self.restore_version_view();
                self.status = "Project opened".into();
                Ok(())
            }
            _ => Err("The file-opening worker returned an incomplete project.".into()),
        }
    }
    fn open_failed(&mut self, path: &std::path::Path, error: String) {
        self.status = "Project could not be opened".into();
        self.error = Some(format!("Could not open {}.\n\n{error}", path.display()));
    }
    pub(super) fn startup_open_dialog(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.startup_open else {
            return;
        };
        let response = egui::Modal::new(egui::Id::new("startup-open")).show(ctx, |ui| {
            ui.set_width(340.);
            ui.heading("Opening project");
            ui.label(job.path.file_name().unwrap_or_default().to_string_lossy());
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(if matches!(job.phase, Phase::Handoff) {
                    "Finishing the current project…"
                } else {
                    "Waiting for the file…"
                });
            });
            ui.label("If your computer asks for file access, respond to that prompt to continue.");
            if matches!(job.phase, Phase::Handoff) {
                ui.label("The project change is being completed. Please wait.");
                false
            } else {
                ui.button("Cancel").clicked()
            }
        });
        if response.inner || response.should_close() {
            self.cancel_startup_open();
        }
        // Also detects unexpected worker disconnection without joining it.
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests;
