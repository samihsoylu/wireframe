// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::Settings;
use crate::storage;
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};
const MAX_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Default)]
pub struct Preferences {
    path: Option<PathBuf>,
    projects: BTreeMap<String, Settings>,
    unreadable: bool,
    pending: bool,
}
impl Preferences {
    pub fn load(&mut self, root: &Path) -> Result<(), String> {
        self.path = Some(root.join("presentation-settings.json"));
        self.projects.clear();
        self.pending = false;
        let result = (|| {
            let file = match std::fs::File::open(self.path.as_ref().unwrap()) {
                Ok(file) => file,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(BTreeMap::new());
                }
                Err(error) => return Err(error.to_string()),
            };
            let mut bytes = vec![];
            file.take(MAX_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() as u64 > MAX_BYTES {
                return Err("Presentation settings exceed the size limit".into());
            }
            let projects: BTreeMap<String, Settings> =
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            for settings in projects.values() {
                settings.validate()?;
            }
            Ok(projects)
        })();
        self.unreadable = result.is_err();
        self.projects = result?;
        Ok(())
    }
    pub fn get(&self, project: &str) -> Settings {
        self.projects.get(project).cloned().unwrap_or_default()
    }
    /// Retain the current session even when disk persistence fails. A subsequent
    /// call with the same value retries a failed atomic write.
    pub fn remember(&mut self, project: &str, settings: &Settings) -> Result<(), String> {
        settings.validate()?;
        if self.projects.get(project) != Some(settings) {
            self.projects.insert(project.into(), settings.clone());
            self.pending = true;
        }
        if !self.pending {
            return Ok(());
        }
        if self.unreadable {
            return Err("The unreadable presentation settings file has been preserved".into());
        }
        if let Some(path) = &self.path {
            let bytes = serde_json::to_vec(&self.projects).map_err(|e| e.to_string())?;
            if bytes.len() as u64 > MAX_BYTES {
                return Err("Presentation settings exceed the size limit".into());
            }
            storage::write_atomic(path, &bytes)?;
        }
        self.pending = false;
        Ok(())
    }
}
