// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! One immutable export on a background thread; destination changes only after encoding.
use super::Options;
use crate::document::Project;
use std::{path::PathBuf, thread::JoinHandle};
pub struct ExportJob {
    handle: JoinHandle<Result<PathBuf, String>>,
}
impl ExportJob {
    pub fn start(project: Project, options: Options, path: PathBuf) -> Result<Self, String> {
        let handle = std::thread::Builder::new()
            .name("pdf-export".into())
            .spawn(move || {
                let pages = super::plan(&project, &options)?;
                let bytes = super::encode(&project.title, &pages)?;
                crate::storage::write_atomic(&path, &bytes).map_err(|error| {
                    format!(
                        "Couldn't save the PDF. Choose a different folder and try exporting again. If it still fails, check your available disk space.\n\nDetails: {error}"
                    )
                })?;
                Ok(path)
            })
            .map_err(|e| format!("Cannot start PDF export: {e}"))?;
        Ok(Self { handle })
    }
    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }
    pub fn finish(self) -> Result<PathBuf, String> {
        self.handle
            .join()
            .map_err(|_| "PDF export stopped unexpectedly".to_owned())?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupted_pdf_job_remains_distinct_from_destination_write_failure() {
        let job = ExportJob {
            handle: std::thread::spawn(|| panic!("injected export interruption")),
        };
        assert_eq!(job.finish().unwrap_err(), "PDF export stopped unexpectedly");
    }
}
