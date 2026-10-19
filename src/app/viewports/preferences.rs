// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use wireframe::{model::Id, storage, viewport::View};

const MAX_BYTES: u64 = 4 * 1024 * 1024;
type Views = BTreeMap<Id, View>;

fn visible_by_default() -> bool {
    true
}
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct ProjectViews {
    views: Views,
    #[serde(default = "visible_by_default")]
    markup_visible: bool,
}
#[derive(Default)]
pub(super) struct Preferences {
    path: Option<PathBuf>,
    projects: BTreeMap<String, ProjectViews>,
    unreadable: bool,
    changed: Option<Instant>,
    pending_since: Option<Instant>,
    retry_after: Option<Instant>,
}

impl Preferences {
    pub fn load(&mut self, root: &Path) -> Result<(), String> {
        self.path = Some(root.join("viewport-settings.json"));
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
                return Err("Saved canvas views exceed the size limit".into());
            }
            // Deserialize through JSON values so numeric surface keys keep the
            // JSON map-key conversion in both the legacy and current formats.
            let stored: BTreeMap<String, serde_json::Value> =
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            let projects: BTreeMap<String, ProjectViews> = stored
                .into_iter()
                .map(|(key, value)| {
                    let project = if value.get("views").is_some() {
                        serde_json::from_value(value)
                    } else {
                        serde_json::from_value(value).map(|views| ProjectViews {
                            views,
                            markup_visible: true,
                        })
                    }
                    .map_err(|e| e.to_string())?;
                    Ok((key, project))
                })
                .collect::<Result<_, String>>()?;
            if projects
                .values()
                .flat_map(|project| project.views.values())
                .any(|view| {
                    !view.is_valid() || view.scroll.iter().any(|value| value.abs() > 1_000_000.)
                })
            {
                return Err("Saved canvas views contain invalid coordinates or scales".into());
            }
            Ok(projects)
        })();
        match result {
            Ok(projects) => {
                self.projects = projects;
                self.unreadable = false;
                Ok(())
            }
            Err(error) => {
                self.unreadable = true;
                Err(error)
            }
        }
    }

    pub fn get(&self, key: &str) -> Views {
        self.projects
            .get(key)
            .map(|project| project.views.clone())
            .unwrap_or_default()
    }

    pub fn markup_visible(&self, key: &str) -> bool {
        self.projects
            .get(key)
            .is_none_or(|project| project.markup_visible)
    }
    pub fn remember(&mut self, key: String, views: &Views, markup_visible: bool, now: Instant) {
        let saved = ProjectViews {
            views: views.clone(),
            markup_visible,
        };
        if self.path.is_none() || self.projects.get(&key) == Some(&saved) {
            return;
        }
        self.projects.insert(key, saved);
        self.changed = Some(now);
        self.pending_since.get_or_insert(now);
    }

    pub fn flush(&mut self, now: Instant, force: bool) -> Result<(), String> {
        let (Some(changed), Some(path)) = (self.changed, &self.path) else {
            return Ok(());
        };
        if !force
            && (self.retry_after.is_some_and(|retry| now < retry)
                || (now.duration_since(changed) < Duration::from_millis(500)
                    && now.duration_since(self.pending_since.unwrap_or(changed))
                        < Duration::from_secs(2)))
        {
            return Ok(());
        }
        self.retry_after = Some(now + Duration::from_secs(2));
        if self.unreadable {
            return Err("The unreadable settings file has been preserved".into());
        }
        let bytes = serde_json::to_vec(&self.projects).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("Saved canvas views exceed the size limit".into());
        }
        storage::write_atomic(path, &bytes)?;
        self.changed = None;
        self.pending_since = None;
        self.retry_after = None;
        Ok(())
    }
}
