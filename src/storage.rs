// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

use crate::model::Document;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub trait DocumentStore {
    fn load(&self, path: &Path) -> Result<Document, String>;
    fn save(&self, path: &Path, document: &Document) -> Result<(), String>;
}
pub struct JsonStore;
impl DocumentStore for JsonStore {
    fn load(&self, path: &Path) -> Result<Document, String> {
        let size = fs::metadata(path).map_err(|e| e.to_string())?.len();
        if size > 32 * 1024 * 1024 {
            return Err("This document exceeds the 32 MB limit".into());
        }
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        let document: Document = serde_json::from_slice(&bytes)
            .map_err(|e| format!("Cannot read this Wireframe document: {e}"))?;
        document.validate()?;
        Ok(document)
    }
    fn save(&self, path: &Path, document: &Document) -> Result<(), String> {
        document.validate()?;
        let bytes = serde_json::to_vec_pretty(document).map_err(|e| e.to_string())?;
        if bytes.len() > 32 * 1024 * 1024 {
            return Err("This document exceeds the 32 MB limit".into());
        }
        write_atomic(path, &bytes)
    }
}
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp.write_all(bytes).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;
    // macOS save-panel grants can cover the file without directory read access.
    // Opening its parent can block on a permission prompt after a successful save.
    #[cfg(target_os = "linux")]
    fs::File::open(parent)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(())
}
pub fn recovery_path() -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("local", "Wireframe", "Wireframe")?;
    fs::create_dir_all(dirs.data_local_dir()).ok()?;
    Some(dirs.data_local_dir().join("recovery.wireframe"))
}

/// Preserve an unreadable checkpoint under a unique name before reusing its slot.
/// The caller must hold the recovery lock. Rename stays on the same filesystem;
/// the reserved destination cannot replace an existing user file.
pub fn preserve_unreadable_recovery(path: &Path) -> Result<PathBuf, String> {
    if !fs::symlink_metadata(path)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("The recovery path is not a regular file".into());
    }
    let parent = path.parent().unwrap_or(Path::new("."));
    let mut reserved = tempfile::Builder::new()
        .prefix("unreadable-recovery-")
        .suffix(".wireframe")
        .tempfile_in(parent)
        .map_err(|e| e.to_string())?
        .into_temp_path();
    fs::rename(path, &reserved).map_err(|e| e.to_string())?;
    // After the rename this is the user's original data, never temporary data.
    reserved.disable_cleanup(true);
    Ok(reserved.to_path_buf())
}

/// One process owns recovery at a time. The OS releases the lock on exit/crash.
pub fn acquire_recovery(path: &Path) -> Result<fs::File, String> {
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path.with_extension("lock"))
        .map_err(|e| e.to_string())?;
    lock.try_lock().map_err(|_| {
        "Recovery is in use by another Wireframe window. Save this project manually.".to_owned()
    })?;
    Ok(lock)
}

/// Storage boundary used by the desktop workspace and failure-injection tests.
pub trait ProjectRepository {
    fn load(&self, path: &Path) -> Result<crate::document::Project, String>;
    fn save(&self, path: &Path, project: &crate::document::Project) -> Result<(), String>;
}
/// Reads either document version and saves recursive version-2 projects.
pub struct ProjectStore;
impl ProjectStore {
    pub fn load(&self, path: &Path) -> Result<crate::document::Project, String> {
        use std::io::Read;
        let mut bytes = Vec::new();
        fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(crate::document::MAX_FILE_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        crate::document::decode(&bytes)
    }
    pub fn save(&self, path: &Path, project: &crate::document::Project) -> Result<(), String> {
        write_atomic(path, &project.encode()?)
    }
}

impl ProjectRepository for ProjectStore {
    fn load(&self, path: &Path) -> Result<crate::document::Project, String> {
        ProjectStore::load(self, path)
    }
    fn save(&self, path: &Path, project: &crate::document::Project) -> Result<(), String> {
        ProjectStore::save(self, path, project)
    }
}
