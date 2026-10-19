// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Ordered page selection shared by document and image exports.
use crate::document::{Id, Page, Project};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selection {
    All,
    Current(Id),
    Set(BTreeSet<Id>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: Id,
    pub owner: Id,
    pub page_name: String,
    pub alternate_name: Option<String>,
}
impl Entry {
    pub fn display_name(&self) -> String {
        self.alternate_name.as_ref().map_or_else(
            || self.page_name.clone(),
            |name| format!("{} — {name}", self.page_name),
        )
    }
    pub fn file_stem(&self) -> String {
        self.alternate_name.as_ref().map_or_else(
            || self.page_name.clone(),
            |name| format!("{} ({name})", self.page_name),
        )
    }
    /// Materialize one version without pruning or rearranging its controls.
    pub fn page(&self, project: &Project) -> Result<Page, String> {
        let owner = project
            .live_pages()
            .find(|p| p.id == self.owner)
            .ok_or("An export page is no longer available")?;
        let content = if self.alternate_name.is_some() {
            owner
                .alternates
                .iter()
                .find(|a| a.id == self.id)
                .ok_or("An export version is no longer available")?
                .content
                .clone()
        } else {
            if self.id != owner.id {
                return Err("Invalid export page identity".into());
            }
            owner.content.clone()
        };
        Ok(Page {
            trashed: false,
            id: self.id,
            name: self.display_name(),
            notes: project.surface_notes(self.id).to_owned(),
            parent: None,
            content,
            alternates: vec![],
        })
    }
}
pub fn entries(project: &Project) -> Vec<Entry> {
    project
        .live_pages()
        .flat_map(|p| {
            std::iter::once(Entry {
                id: p.id,
                owner: p.id,
                page_name: p.name.clone(),
                alternate_name: None,
            })
            .chain(p.alternates.iter().map(|a| Entry {
                id: a.id,
                owner: p.id,
                page_name: p.name.clone(),
                alternate_name: Some(a.name.clone()),
            }))
        })
        .collect()
}
/// Preserve document order even when a set was chosen in a different order.
/// The alternate flag applies only to All; explicit choices remain explicit.
pub fn select(
    project: &Project,
    selection: &Selection,
    include_alternates: bool,
) -> Result<Vec<Entry>, String> {
    let all = entries(project);
    let available: BTreeSet<_> = all.iter().map(|e| e.id).collect();
    let stale = match selection {
        Selection::All => false,
        Selection::Current(id) => !available.contains(id),
        Selection::Set(ids) => !ids.is_subset(&available),
    };
    if stale {
        return Err("An export page is no longer available. Choose the pages again.".into());
    }
    let chosen: Vec<_> = all
        .into_iter()
        .filter(|e| match selection {
            Selection::All => include_alternates || e.alternate_name.is_none(),
            Selection::Current(id) => e.id == *id,
            Selection::Set(ids) => ids.contains(&e.id),
        })
        .collect();
    if chosen.is_empty() {
        return Err("Choose at least one page to export.".into());
    }
    Ok(chosen)
}
