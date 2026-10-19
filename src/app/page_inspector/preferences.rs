// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};
use wireframe::storage;

const MAX_BYTES: u64 = 4 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
struct Store {
    default_height: u16,
    projects: BTreeMap<String, u16>,
}
impl Default for Store {
    fn default() -> Self {
        Self {
            default_height: 200,
            projects: BTreeMap::new(),
        }
    }
}
#[derive(Default)]
pub(super) struct Preferences {
    path: Option<PathBuf>,
    store: Store,
    unreadable: bool,
}
impl Preferences {
    pub fn load(&mut self, root: &Path) -> Result<(), String> {
        self.path = Some(root.join("panel-settings.json"));
        let result = (|| {
            let file = match std::fs::File::open(self.path.as_ref().unwrap()) {
                Ok(file) => file,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(Store::default());
                }
                Err(error) => return Err(error.to_string()),
            };
            let mut bytes = Vec::new();
            file.take(MAX_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() as u64 > MAX_BYTES {
                return Err("Panel settings exceed the size limit".into());
            }
            let store: Store = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            if std::iter::once(&store.default_height)
                .chain(store.projects.values())
                .any(|height| !(130..=400).contains(height))
            {
                return Err("Panel settings contain an invalid height".into());
            }
            Ok(store)
        })();
        match result {
            Ok(store) => {
                self.store = store;
                self.unreadable = false;
                Ok(())
            }
            Err(error) => {
                self.unreadable = true;
                Err(error)
            }
        }
    }
    pub fn get(&self, key: Option<&str>) -> u16 {
        key.and_then(|key| self.store.projects.get(key))
            .copied()
            .unwrap_or(self.store.default_height)
    }
    pub fn remember(&mut self, key: Option<&str>, height: u16) -> Result<(), String> {
        if !(130..=400).contains(&height) {
            return Err("Panel height must be between 130 and 400".into());
        }
        self.store.default_height = height;
        if let Some(key) = key {
            self.store.projects.insert(key.into(), height);
        }
        if self.unreadable {
            return Err("The unreadable settings file is unchanged".into());
        }
        if let Some(path) = &self.path {
            let bytes = serde_json::to_vec(&self.store).map_err(|e| e.to_string())?;
            if bytes.len() as u64 > MAX_BYTES {
                return Err("Panel settings exceed the size limit".into());
            }
            storage::write_atomic(path, &bytes)?;
        }
        Ok(())
    }
}
