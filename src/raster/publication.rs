// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Complete temporary outputs and explicit destination publication decisions.
use super::names::{FileName, collision_key};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

pub struct PendingOutput {
    temp: tempfile::NamedTempFile,
    target: PathBuf,
}
#[derive(Clone, Debug)]
pub struct SavedOutput {
    pub path: PathBuf,
    /// The file was published, but its directory durability could not be confirmed.
    pub durability_warning: Option<String>,
}
pub enum Attempt {
    Saved(SavedOutput),
    Conflict(PendingOutput),
}
impl PendingOutput {
    pub fn prepare(
        target: PathBuf,
        encode: impl FnOnce(&mut File) -> Result<(), String>,
    ) -> Result<Self, String> {
        if target.file_name().is_none() {
            return Err("Choose a destination filename".into());
        }
        let mut temp = tempfile::Builder::new()
            .prefix(".wireframe-export-")
            .tempfile_in(parent(&target))
            .map_err(|e| format!("Cannot create temporary export: {e}"))?;
        encode(temp.as_file_mut())?;
        temp.flush()
            .and_then(|_| temp.as_file().sync_all())
            .map_err(|e| format!("Cannot finish temporary export: {e}"))?;
        Ok(Self { temp, target })
    }
    pub fn target(&self) -> &Path {
        &self.target
    }
    /// A newly appearing destination is a conflict, never implicit permission
    /// to replace it. The completed temporary file survives for the next choice.
    pub fn publish_new(self) -> Result<Attempt, String> {
        let Self { temp, target } = self;
        match temp.persist_noclobber(&target) {
            Ok(file) => {
                drop(file);
                Ok(Attempt::Saved(saved(target)))
            }
            Err(error) => {
                let exists = error.error.kind() == io::ErrorKind::AlreadyExists
                    || fs::symlink_metadata(&target).is_ok();
                if exists {
                    Ok(Attempt::Conflict(Self {
                        temp: error.file,
                        target,
                    }))
                } else {
                    Err(format!("Cannot publish export: {}", error.error))
                }
            }
        }
    }
    /// Call only after an explicit Replace decision for this destination (or
    /// an applicable Replace-all decision). Replacement is atomic at the path;
    /// it is not a compare-and-swap against another program's concurrent edits.
    pub fn replace(self) -> Result<SavedOutput, String> {
        match fs::symlink_metadata(&self.target) {
            Ok(metadata) if !metadata.file_type().is_file() => {
                return Err("Cannot replace a folder or symbolic link with an export".into());
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("Cannot inspect export destination: {error}")),
        }
        let Self { temp, target } = self;
        temp.persist(&target)
            .map_err(|e| format!("Cannot replace export: {}", e.error))?;
        Ok(saved(target))
    }
    /// Keep the existing destination and all planned names. Exclusive publish
    /// is retried if another writer takes a candidate after directory scanning.
    pub fn keep_both(
        mut self,
        base: &FileName,
        planned: &HashSet<String>,
        mut checkpoint: impl FnMut() -> Result<(), String>,
    ) -> Result<SavedOutput, String> {
        let directory = parent(&self.target).to_path_buf();
        checkpoint()?;
        let mut occupied = planned.clone();
        for entry in
            fs::read_dir(&directory).map_err(|e| format!("Cannot inspect export folder: {e}"))?
        {
            checkpoint()?;
            let entry = entry.map_err(|e| format!("Cannot inspect export folder: {e}"))?;
            if let Some(name) = entry.file_name().to_str() {
                occupied.insert(collision_key(name));
            }
        }
        let mut number = 1u64;
        loop {
            checkpoint()?;
            let candidate = base.numbered(number);
            if !occupied.contains(&candidate.key()) {
                self.target = directory.join(candidate.text());
                match self.publish_new()? {
                    Attempt::Saved(output) => return Ok(output),
                    Attempt::Conflict(pending) => {
                        self = pending;
                        occupied.insert(candidate.key());
                    }
                }
            }
            number = number
                .checked_add(1)
                .ok_or("Cannot assign a Keep Both filename")?;
        }
    }
    /// Dropping a pending output implements Skip or Cancel without publishing.
    pub fn discard(self) -> Result<(), String> {
        self.temp
            .close()
            .map_err(|e| format!("Cannot remove temporary export: {e}"))
    }
}
fn parent(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}
fn saved(path: PathBuf) -> SavedOutput {
    // Follow the existing storage policy: macOS save-panel grants can cover a
    // file without granting directory-read access. Do not provoke a new prompt.
    #[cfg(target_os = "linux")]
    let durability_warning = File::open(parent(&path))
        .and_then(|dir| dir.sync_all())
        .err()
        .map(|e| format!("Export saved, but folder synchronization failed: {e}"));
    #[cfg(not(target_os = "linux"))]
    let durability_warning = None;
    SavedOutput {
        path,
        durability_warning,
    }
}
