// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Editor {
    pub fn rename_project(&mut self, name: &str) -> Result<(), String> {
        self.commit();
        if name.trim().is_empty() || name.len() > 10000 {
            return Err("Enter a project name of 1–10000 bytes".into());
        }
        let mut document = self.document.clone();
        document.title = name.into();
        document.encode()?;
        self.edit("Rename project", |e| e.document = document);
        Ok(())
    }

    /// Apply an explicitly owned notes draft without changing page/version selection.
    pub fn edit_page_notes(
        &mut self,
        page: Id,
        alternate: Option<Id>,
        notes: &str,
    ) -> Result<(), String> {
        self.commit();
        let mut document = self.document.clone();
        let owner = document
            .pages
            .iter_mut()
            .find(|p| p.id == page && !p.trashed)
            .ok_or("The notes page is no longer available")?;
        let value = if let Some(id) = alternate {
            &mut owner
                .alternates
                .iter_mut()
                .find(|a| a.id == id)
                .ok_or("The notes version is no longer available")?
                .notes
        } else {
            &mut owner.notes
        };
        *value = notes.into();
        document.encode()?;
        self.edit("Edit notes", |e| e.document = document);
        Ok(())
    }
    /// Selection produced by the current duplicate history entry. Used when
    /// newly restored copies reappear on Redo; ordinary navigation ignores it.
    pub fn duplicated_page_selection(&self) -> Vec<Id> {
        let Some(change) = self.cursor.checked_sub(1).and_then(|i| self.history.get(i)) else {
            return vec![];
        };
        if !matches!(
            change.name.as_str(),
            "Duplicate wireframe" | "Duplicate wireframes"
        ) || change.after.page != self.page
        {
            return vec![];
        }
        let existing: HashSet<_> = change.before.document.pages.iter().map(|p| p.id).collect();
        change
            .after
            .document
            .live_pages()
            .filter(|p| !existing.contains(&p.id))
            .map(|p| p.id)
            .collect()
    }
    pub fn rename_page(&mut self, page: Id, name: &str) -> Result<(), String> {
        self.commit();
        if name.trim().is_empty() || name.len() > 10000 {
            return Err("Enter a page name of 1–10000 bytes".into());
        }
        let mut document = self.document.clone();
        let target = document
            .pages
            .iter_mut()
            .find(|p| p.id == page && !p.trashed)
            .ok_or("Page no longer exists")?;
        if target.name == name {
            return Ok(());
        }
        target.name = name.into();
        document.encode()?;
        self.edit("Rename wireframe", |e| e.document = document);
        Ok(())
    }

    /// Clone one explicit page, without implicitly selecting descendants.
    pub fn duplicate_page(&mut self, page: Id) -> Result<Id, String> {
        Ok(self.duplicate_pages(&[page])?[0])
    }

    /// Duplicate an explicit selection as one atomic history operation.
    /// Input order is normalized to live tree order and duplicate IDs coalesce.
    pub fn duplicate_pages(&mut self, pages: &[Id]) -> Result<Vec<Id>, String> {
        self.commit();
        if pages.is_empty() {
            return Ok(vec![]);
        }
        let requested: HashSet<_> = pages.iter().copied().collect();
        let tree = crate::page_tree::PageTree::new(&self.document);
        let sources: Vec<_> = tree
            .rows
            .iter()
            .filter(|r| requested.contains(&r.id))
            .collect();
        if sources.len() != requested.len() {
            return Err("One or more pages are no longer available for this action".into());
        }
        let minimum = sources.iter().map(|r| r.depth).min().unwrap();
        let mut anchor = *sources.last().unwrap();
        while anchor.depth > minimum {
            anchor = tree.row(anchor.parent.unwrap()).unwrap();
        }
        let insertion = tree
            .subtree(anchor.id)
            .iter()
            .map(|r| r.position)
            .max()
            .unwrap()
            + 1;
        let mut document = self.document.clone();
        let original_len = document.pages.len();
        let mut copies = Vec::with_capacity(sources.len());
        let mut versions = Vec::with_capacity(sources.len());
        let mut parents = vec![];
        let mut previous_depth = sources[0].depth;
        let mut relative_depth = 0usize;
        for (index, row) in sources.iter().enumerate() {
            let source = self.document.live_page(row.id).unwrap();
            let mut copy = source.clone();
            copy.name = copy_name(&source.name, &document.pages);
            copy.id = document.allocate_id()?;
            let active = self.active_alternate(source.id);
            let mut selected_alternate = None;
            for alternate in &mut copy.alternates {
                let original = alternate.id;
                alternate.id = document.allocate_id()?;
                if active == Some(original) {
                    selected_alternate = Some(alternate.id);
                }
            }
            if sources.len() > 1 {
                if index > 0 {
                    relative_depth = match row.depth.cmp(&previous_depth) {
                        std::cmp::Ordering::Greater => {
                            (relative_depth + 1).min(4usize.saturating_sub(anchor.depth))
                        }
                        std::cmp::Ordering::Equal => relative_depth,
                        std::cmp::Ordering::Less => 0,
                    };
                }
                parents.truncate(relative_depth);
                copy.parent = parents.last().copied().or(anchor.parent);
                parents.push(copy.id);
                previous_depth = row.depth;
            } else {
                copy.parent = row.parent;
            }
            copies.push(copy.id);
            versions.push(selected_alternate);
            // Include preceding copies while allocating names and identities.
            document.pages.push(copy);
        }
        let added: Vec<_> = document.pages.drain(original_len..).collect();
        document.pages.splice(insertion..insertion, added);
        document.encode()?;
        // Undo returns to the last source. First use follows source alternates;
        // redo opens the first official copy through ordinary snapshot history.
        self.switch_page(sources.last().unwrap().id);
        self.edit(
            if copies.len() == 1 {
                "Duplicate wireframe"
            } else {
                "Duplicate wireframes"
            },
            |e| {
                e.document = document;
                e.page = copies[0];
                e.definition = None;
                e.library = None;
                e.context.clear();
                e.selection.clear();
            },
        );
        for (&id, version) in copies.iter().zip(versions) {
            if let Some(version) = version {
                self.active_versions.insert(id, version);
            } else {
                self.active_versions.remove(&id);
            }
        }
        Ok(copies)
    }
}

fn copy_name(name: &str, pages: &[Page]) -> String {
    let base = name
        .rsplit_once(" copy")
        .filter(|(_, suffix)| {
            suffix.is_empty()
                || suffix
                    .strip_prefix(' ')
                    .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        })
        .map_or(name, |(base, _)| base);
    let names: HashSet<_> = pages
        .iter()
        .filter(|p| !p.trashed)
        .map(|p| p.name.to_lowercase())
        .collect();
    let mut number = 1;
    loop {
        let candidate = if number == 1 {
            format!("{base} copy")
        } else {
            format!("{base} copy {number}")
        };
        if !names.contains(&candidate.to_lowercase()) {
            return candidate;
        }
        number += 1;
    }
}
