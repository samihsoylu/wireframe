// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Offline catalog and immutable snapshots; no working-copy or save ownership.
use crate::document::{self, Id, Project};
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
};
mod starters;

#[derive(Clone)]
pub enum Source {
    Starter(usize),
    Local(PathBuf),
}
#[derive(Clone)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub category: String,
    pub description: String,
    pub source: Source,
}
pub struct Catalog {
    pub entries: Vec<Entry>,
    pub category: String,
    pub query: String,
}
impl Default for Catalog {
    fn default() -> Self {
        Self {
            entries: starters::entries(),
            category: "All".into(),
            query: String::new(),
        }
    }
}
impl Catalog {
    pub fn categories(&self) -> Vec<String> {
        let mut names: Vec<_> = self.entries.iter().map(|e| e.category.clone()).collect();
        names.sort();
        names.dedup();
        names.insert(0, "All".into());
        names
    }
    pub fn results(&self) -> Vec<usize> {
        let query = self.query.trim().to_lowercase();
        let mut results: Vec<_> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                if query.is_empty() {
                    self.category == "All" || self.category == e.category
                } else {
                    [&e.title, &e.category, &e.description]
                        .iter()
                        .any(|s| s.to_lowercase().contains(&query))
                }
            })
            .map(|(i, _)| i)
            .collect();
        results.sort_by_key(|i| {
            let e = &self.entries[*i];
            (
                !query.is_empty() && !e.title.to_lowercase().contains(&query),
                e.title.to_lowercase(),
                e.id.clone(),
            )
        });
        results
    }
    pub fn add_local(&mut self, snapshot: &Snapshot) -> Result<usize, String> {
        let path = snapshot.path.clone().ok_or("This is not a local project")?;
        let entry = Entry {
            id: snapshot.origin.clone(),
            title: snapshot.project.title.clone(),
            category: "Local projects".into(),
            description: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
            source: Source::Local(path),
        };
        if let Some(i) = self.entries.iter().position(|e| e.id == entry.id) {
            self.entries[i] = entry;
            Ok(i)
        } else {
            self.entries.push(entry);
            Ok(self.entries.len() - 1)
        }
    }
}
impl Entry {
    pub fn load(&self) -> Result<Snapshot, String> {
        match &self.source {
            Source::Starter(index) => Ok(Snapshot {
                project: starters::project(*index)?,
                origin: self.id.clone(),
                path: None,
                digest: None,
            }),
            Source::Local(path) => Snapshot::local(path),
        }
    }
}
#[derive(Clone)]
pub struct Snapshot {
    pub project: Project,
    pub origin: String,
    path: Option<PathBuf>,
    digest: Option<[u8; 32]>,
}
impl Snapshot {
    pub fn local(path: &Path) -> Result<Self, String> {
        let path = path
            .canonicalize()
            .map_err(|e| format!("Cannot open project: {e}"))?;
        let bytes = read(&path)?;
        let project = document::decode(&bytes)?;
        let digest = Some(Sha256::digest(&bytes).into());
        let origin = format!(
            "local-project:{}",
            Sha256::digest(path.as_os_str().as_encoded_bytes())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        Ok(Self {
            project,
            origin,
            path: Some(path),
            digest,
        })
    }
    /// Compare actual bytes, not modification times. The accepted snapshot is
    /// immutable even if an external writer changes the file after this check.
    pub fn verify_source(&self) -> Result<(), String> {
        if let Some(path) = &self.path {
            let current: [u8; 32] = Sha256::digest(read(path)?).into();
            if Some(current) != self.digest {
                return Err(
                    "The source project changed. Reload its preview and review the import again."
                        .into(),
                );
            }
        }
        Ok(())
    }
    pub fn initial_resource(&self) -> Option<Id> {
        if self.project.live_pages().count() == 1
            && let Some(library) = self.project.live_libraries().next()
        {
            return Some(library.id);
        }
        self.project
            .live_pages()
            .next()
            .map(|p| p.id)
            .or_else(|| self.project.live_libraries().next().map(|l| l.id))
    }
    pub fn import_pages(&self) -> Vec<Id> {
        self.project
            .live_pages()
            .filter(|p| {
                !p.content.nodes.is_empty()
                    || !p.notes.is_empty()
                    || self.project.has_review_comments(p.id)
                    || p.alternates.iter().any(|a| {
                        !a.content.nodes.is_empty()
                            || !a.notes.is_empty()
                            || self.project.has_review_comments(a.id)
                    })
            })
            .map(|p| p.id)
            .collect()
    }
}
fn read(path: &Path) -> Result<Vec<u8>, String> {
    let mut bytes = vec![];
    std::fs::File::open(path)
        .map_err(|e| format!("Cannot read source project: {e}"))?
        .take(document::MAX_FILE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > document::MAX_FILE_BYTES {
        return Err("The source project exceeds the 64 MB limit".into());
    }
    Ok(bytes)
}
