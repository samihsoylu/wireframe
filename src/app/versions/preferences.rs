// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    io::Read,
    path::{Path, PathBuf},
};
use wireframe::{
    document::Id,
    storage,
    workspace::{CatalogCategory, Editor},
};
const MAX_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct View {
    pub page: Id,
    pub versions: BTreeMap<Id, Id>,
    #[serde(default = "crate::app::library::default_categories")]
    pub categories: Vec<CatalogCategory>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub collapsed: BTreeSet<Id>,
    #[serde(default = "default_thumbnails")]
    pub thumbnails: bool,
}
fn default_thumbnails() -> bool {
    true
}
impl View {
    pub fn capture(
        editor: &Editor,
        categories: &[CatalogCategory],
        collapsed: &HashSet<Id>,
        thumbnails: bool,
    ) -> Self {
        let tree = wireframe::page_tree::PageTree::new(&editor.document);
        Self {
            page: editor.page,
            categories: categories.to_vec(),
            versions: editor.version_selections().into_iter().collect(),
            thumbnails,
            collapsed: collapsed
                .iter()
                .copied()
                .filter(|id| tree.row(*id).is_some_and(|row| row.has_children))
                .collect(),
        }
    }
    pub fn restore(&self, editor: &mut Editor) {
        editor.restore_version_selections(
            self.versions
                .iter()
                .map(|(p, a)| (*p, *a))
                .collect::<HashMap<_, _>>(),
        );
        editor.switch_page(self.page);
    }
}
#[derive(Default)]
pub(super) struct Preferences {
    path: Option<PathBuf>,
    projects: BTreeMap<String, View>,
    unreadable: bool,
    last: Option<(String, View)>,
}
impl Preferences {
    pub fn enabled(&self) -> bool {
        self.path.is_some()
    }
    pub fn load(&mut self, root: &Path) -> Result<(), String> {
        self.path = Some(root.join("version-view-settings.json"));
        self.last = None;
        let result = (|| {
            let file = match std::fs::File::open(self.path.as_ref().unwrap()) {
                Ok(file) => file,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
                Err(e) => return Err(e.to_string()),
            };
            let mut bytes = vec![];
            file.take(MAX_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() as u64 > MAX_BYTES {
                return Err("Saved version choices exceed the size limit".into());
            }
            serde_json::from_slice(&bytes).map_err(|e| e.to_string())
        })();
        match result {
            Ok(projects) => {
                self.projects = projects;
                self.unreadable = false;
                Ok(())
            }
            Err(e) => {
                self.unreadable = true;
                Err(e)
            }
        }
    }
    pub fn restore(
        &mut self,
        key: &str,
        editor: &mut Editor,
    ) -> (Vec<CatalogCategory>, HashSet<Id>, bool) {
        if let Some(view) = self.projects.get(key) {
            view.restore(editor);
        }
        let mut categories = self
            .projects
            .get(key)
            .map(|view| view.categories.clone())
            .unwrap_or_else(crate::app::library::default_categories);
        crate::app::library::normalize_categories(&mut categories);
        let collapsed = self
            .projects
            .get(key)
            .map(|view| view.collapsed.iter().copied().collect())
            .unwrap_or_default();
        let thumbnails = self.projects.get(key).is_none_or(|view| view.thumbnails);
        let view = View::capture(editor, &categories, &collapsed, thumbnails);
        let collapsed = view.collapsed.iter().copied().collect();
        self.last = Some((key.into(), view));
        (categories, collapsed, thumbnails)
    }
    pub fn remember(
        &mut self,
        key: &str,
        editor: &Editor,
        categories: &[CatalogCategory],
        collapsed: &HashSet<Id>,
        thumbnails: bool,
    ) -> Result<(), String> {
        let view = View::capture(editor, categories, collapsed, thumbnails);
        if self
            .last
            .as_ref()
            .is_some_and(|(previous, state)| previous == key && *state == view)
        {
            return Ok(());
        }
        self.last = Some((key.into(), view.clone()));
        self.projects.insert(key.into(), view);
        if self.unreadable {
            return Err("Version choices are kept for this session. The unreadable settings file is unchanged.".into());
        }
        if let Some(path) = &self.path {
            let bytes = serde_json::to_vec_pretty(&self.projects).map_err(|e| e.to_string())?;
            if bytes.len() as u64 > MAX_BYTES {
                return Err(
                    "Version choices could not be remembered: settings are too large".into(),
                );
            }
            storage::write_atomic(path, &bytes)?;
        }
        Ok(())
    }
}
