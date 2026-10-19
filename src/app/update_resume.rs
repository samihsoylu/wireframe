// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! A one-use saved-project handoff, scoped to one application installation.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
struct Handoff {
    bundle: PathBuf,
    build: String,
    created: u64,
    project: PathBuf,
}

pub(super) struct Resume {
    path: PathBuf,
    bundle: PathBuf,
    build: String,
}
impl Resume {
    pub(super) fn new(path: PathBuf, bundle: PathBuf, build: String) -> Self {
        Self {
            path,
            bundle,
            build,
        }
    }

    pub(super) fn save(&self, project: &Path, now: u64) -> Result<(), String> {
        let project = std::path::absolute(project).map_err(|e| e.to_string())?;
        let handoff = Handoff {
            bundle: self.bundle.clone(),
            build: self.build.clone(),
            created: now,
            project,
        };
        let bytes = serde_json::to_vec(&handoff).map_err(|e| e.to_string())?;
        wireframe::storage::write_atomic(&self.path, &bytes)
    }

    pub(super) fn take(&self, explicit_open: bool, now: u64) -> Option<PathBuf> {
        let bytes = std::fs::read(&self.path).ok()?;
        // Consume even an obsolete or overridden handoff. Never reopen repeatedly.
        std::fs::remove_file(&self.path).ok()?;
        let saved: Handoff = serde_json::from_slice(&bytes).ok()?;
        (!explicit_open
            && saved.bundle == self.bundle
            && saved.build != self.build
            && now.checked_sub(saved.created).is_some_and(|age| age <= 600)
            && saved.project.is_absolute())
        .then_some(saved.project)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(root: &Path, build: &str) -> Resume {
        Resume::new(root.join("resume.json"), root.join("App.app"), build.into())
    }
    #[test]
    fn upgraded_build_consumes_unicode_project_once() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("Project café 世界.wireframe");
        fixture(dir.path(), "1").save(&project, 100).unwrap();
        let updated = fixture(dir.path(), "2");
        assert_eq!(updated.take(false, 101), Some(project));
        assert_eq!(updated.take(false, 102), None);
    }
    #[test]
    fn explicit_open_ordinary_launch_expired_and_other_install_ignore_handoff() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("saved.wireframe");
        for (build, explicit, time, other) in [
            ("2", true, 101, false),
            ("1", false, 101, false),
            ("2", false, 701, false),
            ("2", false, 99, false),
            ("2", false, 101, true),
        ] {
            fixture(dir.path(), "1").save(&project, 100).unwrap();
            let mut next = fixture(dir.path(), build);
            if other {
                next.bundle = dir.path().join("Other.app");
            }
            assert_eq!(next.take(explicit, time), None);
            assert!(!next.path.exists());
        }
    }
    #[test]
    fn malformed_handoff_does_not_open_and_failed_write_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let state = fixture(dir.path(), "2");
        std::fs::write(&state.path, b"incomplete").unwrap();
        assert_eq!(state.take(false, 100), None);
        std::fs::create_dir(&state.path).unwrap();
        assert!(
            state
                .save(&dir.path().join("saved.wireframe"), 100)
                .is_err()
        );
    }
}
