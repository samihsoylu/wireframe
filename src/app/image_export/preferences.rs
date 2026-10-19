// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
};
use wireframe::{document::Id, storage};
const MAX_BYTES: u64 = 4 * 1024 * 1024;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Range {
    #[default]
    All,
    Current,
    Chosen,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Scope {
    #[default]
    Whole,
    Group,
    Selection,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Options {
    pub range: Range,
    pub chosen: BTreeSet<Id>,
    pub scope: Scope,
    pub double: bool,
    pub transparent: bool,
    pub archive: bool,
    pub folder: Option<PathBuf>,
    pub archive_stem: Option<String>,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            range: Range::All,
            chosen: BTreeSet::new(),
            scope: Scope::Whole,
            double: true,
            transparent: false,
            archive: true,
            folder: None,
            archive_stem: None,
        }
    }
}
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Store {
    defaults: Options,
    projects: BTreeMap<String, Options>,
}
#[derive(Default)]
pub(super) struct Preferences {
    path: Option<PathBuf>,
    store: Store,
    unreadable: bool,
}
impl Preferences {
    pub fn load(&mut self, root: &Path) -> Result<(), String> {
        self.path = Some(root.join("image-export-settings.json"));
        let result = (|| {
            let file = match std::fs::File::open(self.path.as_ref().unwrap()) {
                Ok(file) => file,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Store::default()),
                Err(e) => return Err(e.to_string()),
            };
            let mut bytes = vec![];
            file.take(MAX_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() as u64 > MAX_BYTES {
                return Err("Image export settings exceed the size limit".into());
            }
            serde_json::from_slice(&bytes).map_err(|e| e.to_string())
        })();
        match result {
            Ok(store) => {
                self.store = store;
                self.unreadable = false;
                Ok(())
            }
            Err(e) => {
                self.unreadable = true;
                Err(e)
            }
        }
    }
    pub fn get(&self, key: Option<&str>) -> Options {
        key.and_then(|key| self.store.projects.get(key))
            .unwrap_or(&self.store.defaults)
            .clone()
    }
    pub fn remember(&mut self, key: Option<&str>, options: Options) -> Result<(), String> {
        if let Some(key) = key {
            self.store.projects.insert(key.into(), options.clone());
        }
        self.store.defaults = options;
        // Numeric IDs are local to a project; never reuse them through defaults.
        self.store.defaults.chosen.clear();
        if self.unreadable {
            return Err("Export options are kept for this session. The unreadable settings file is unchanged.".into());
        }
        if let Some(path) = &self.path {
            let bytes = serde_json::to_vec_pretty(&self.store).map_err(|e| e.to_string())?;
            if bytes.len() as u64 > MAX_BYTES {
                return Err(
                    "Export options could not be remembered: settings are too large".into(),
                );
            }
            storage::write_atomic(path, &bytes)?;
        }
        Ok(())
    }
}
