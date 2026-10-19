// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::working_copy::WorkingCopies;

impl WireframeApp {
    pub(in crate::app) fn start_menu_open(
        &mut self,
        path: PathBuf,
        discard: bool,
        ctx: &egui::Context,
    ) {
        self.dismiss_recoveries();
        self.remember_version_view();
        self.remember_viewports(true);
        self.remember_presentation();
        self.font_import = None;
        let copies = self.startup_copies();
        let current = self.open_source_id();
        let source = path.clone();
        let read = move || {
            // The current session remains owned and editable after cancellation.
            // Resolve aliases with exactly the ownership key, away from the UI.
            if copies.is_some()
                && current
                    .as_ref()
                    .is_some_and(|id| WorkingCopies::named_id(&source).is_ok_and(|key| &key == id))
            {
                let validated = if discard {
                    Some(Box::new(ProjectStore.load(&source)?))
                } else {
                    None
                };
                return Ok(Prepared::Current(validated));
            }
            // Explicit Discard must not destroy current work for a missing or
            // invalid destination, even if that destination has a recovery copy.
            if discard {
                ProjectStore.load(&source)?;
            }
            match copies {
                Some(copies) => persistence::PreparedOpen::read(&copies, &source)
                    .map(Box::new)
                    .map(Prepared::Persistent),
                None => ProjectStore
                    .load(&source)
                    .map(Box::new)
                    .map(Prepared::Plain),
            }
        };
        match Opening::spawn(path.clone(), ctx.clone(), read) {
            Ok(mut job) => {
                job.phase = Phase::MenuRead { discard };
                self.startup_open = Some(job);
                self.status = "Opening project…".into();
            }
            Err(error) => self.open_failed(&path, error),
        }
    }

    pub(super) fn validate_open(prepared: &Prepared) -> Result<(), String> {
        let document = match prepared {
            Prepared::Persistent(p) => Some(p.copy.project()),
            Prepared::Plain(p) | Prepared::Current(Some(p)) => Some(p.as_ref()),
            Prepared::Current(None) => None,
            Prepared::Committed { .. } => return Err("Unexpected completed open request.".into()),
        };
        if let Some(document) = document {
            let editor = Editor::new(document.clone());
            for page in editor.document.live_pages() {
                editor.layout_page(page.id)?;
            }
        }
        Ok(())
    }

    pub(super) fn begin_open_handoff(
        &mut self,
        path: PathBuf,
        prepared: Prepared,
        discard: bool,
        ctx: &egui::Context,
    ) {
        let copies = self.startup_copies();
        let source = self.take_open_source();
        let target = path.clone();
        match Opening::spawn(path.clone(), ctx.clone(), move || {
            let mut discarded = false;
            let result = (|| {
                if let Some(source) = source {
                    source.finish(discard)?;
                    discarded = discard;
                }
                // Reopening the same identity cannot acquire a second lock until
                // the previous session has finished. Other candidates are already
                // held and validated before any explicit source discard.
                let prepared = if matches!(prepared, Prepared::Current(_)) {
                    persistence::PreparedOpen::read(
                        &copies.expect("current persistent owner"),
                        &target,
                    )
                    .map(Box::new)
                    .map(Prepared::Persistent)?
                } else {
                    prepared
                };
                Ok(Box::new(prepared))
            })();
            Ok(Prepared::Committed { result, discarded })
        }) {
            Ok(mut job) => {
                job.phase = Phase::Handoff;
                self.startup_open = Some(job);
                self.status = "Finishing project change…".into();
            }
            Err(error) => self.open_failed(&path, error),
        }
    }

    pub(in crate::app) fn cancel_open_for_quit(&mut self) -> bool {
        if let Some(job) = &mut self.startup_open
            && matches!(job.phase, Phase::Handoff)
        {
            // The worker owns the source session during this final handoff.
            // Quit follows its result; it must not race a second owner or discard.
            job.quit_after = true;
            return false;
        }
        self.cancel_startup_open();
        true
    }
}

#[cfg(test)]
mod tests;
