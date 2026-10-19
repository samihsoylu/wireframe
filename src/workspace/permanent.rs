// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Permanent resource removal keeps broken references and unrelated history.
use super::*;
use std::collections::BTreeSet;

impl Editor {
    /// Remove only an explicitly confirmed set of retained pages.
    pub fn permanently_remove_pages(&mut self, pages: &[Id]) -> Result<(), String> {
        self.permanently_remove_resources(
            &pages
                .iter()
                .map(|&id| RetainedResource::Page(id))
                .collect::<Vec<_>>(),
        )
    }

    /// The caller owns confirmation of this exact retained-resource set.
    /// Stale selections fail before changing content, pending edits or history.
    pub fn permanently_remove_resources(
        &mut self,
        resources: &[RetainedResource],
    ) -> Result<(), String> {
        let requested: HashSet<_> = resources.iter().copied().collect();
        if requested.is_empty()
            || requested.len() != resources.len()
            || requested.iter().any(|resource| match resource {
                RetainedResource::Page(id) => {
                    !self.document.pages.iter().any(|p| p.id == *id && p.trashed)
                }
                RetainedResource::Asset(id) => !self
                    .document
                    .assets
                    .iter()
                    .any(|a| a.id == *id && a.trashed),
                RetainedResource::Library(id) => !self
                    .document
                    .libraries
                    .iter()
                    .any(|l| l.id == *id && l.trashed),
            })
        {
            return Err("Select distinct resources still in Trash".into());
        }
        let mut removed = self.document.removed_pages.clone();
        let mut libraries = HashSet::new();
        let mut assets = self.document.removed_assets.clone();
        for resource in requested {
            match resource {
                RetainedResource::Page(id) => {
                    removed.insert(id);
                }
                RetainedResource::Asset(id) => {
                    assets.insert(id);
                }
                RetainedResource::Library(id) => {
                    libraries.insert(id);
                }
            }
        }
        let definitions: HashSet<_> = self
            .document
            .libraries
            .iter()
            .filter(|library| libraries.contains(&library.id))
            .flat_map(|library| library.sources.iter().map(|source| source.definition))
            .collect();
        let next_id = self.document.next_id;
        let mut document = self.document.clone();
        remove_content(
            &mut document,
            &removed,
            &libraries,
            &definitions,
            &assets,
            next_id,
        );
        document.encode()?;

        // Prepare every historical projection before applying anything. Neither
        // Undo nor Redo may restore removed content from an older full snapshot.
        let mut history = self.history.clone();
        for change in &mut history {
            for snapshot in [&mut change.before, &mut change.after] {
                remove_content(
                    &mut snapshot.document,
                    &removed,
                    &libraries,
                    &definitions,
                    &assets,
                    next_id,
                );
            }
            if change.before.document.live_pages().next().is_none()
                || change.after.document.live_pages().next().is_none()
            {
                // This operation requires a page that no longer exists. Consume
                // it as a failed operation without replacing the current page or
                // copying its content into an incompatible historical graph.
                change.invalidated = true;
                change.before.document = document.clone();
                change.after.document = document.clone();
            }
            change.before.document.encode()?;
            change.after.document.encode()?;
            let mut before = change.before.document.clone();
            before.next_id = change.after.document.next_id;
            let no_remaining_change = before == change.after.document;
            change.invalidated |= if let Some(page_history) = &mut change.page_history {
                page_history.remove_deleted_targets(&removed, &libraries, &assets)
            } else {
                no_remaining_change
            };
            change.bytes = serde_json::to_vec(&change.before.document)
                .map_err(|e| e.to_string())?
                .len()
                + serde_json::to_vec(&change.after.document)
                    .map_err(|e| e.to_string())?
                    .len();
        }
        let mut pending = self.pending.clone();
        if let Some((_, snapshot)) = &mut pending {
            remove_content(
                &mut snapshot.document,
                &removed,
                &libraries,
                &definitions,
                &assets,
                next_id,
            );
            if snapshot.document.live_pages().next().is_none() {
                snapshot.document = document.clone();
            }
            snapshot.document.encode()?;
        }
        let mut cursor = self.cursor;
        let mut bytes: usize = history.iter().map(|change| change.bytes).sum();
        while history.len() > 1 && bytes > 64 * 1024 * 1024 {
            let index = if cursor > 0 { 0 } else { history.len() - 1 };
            bytes = bytes.saturating_sub(history.remove(index).bytes);
            cursor = cursor.saturating_sub(1);
        }
        self.cursor = cursor;
        self.document = document;
        self.history = history;
        self.pending = pending;
        self.commit();
        self.reconcile();
        Ok(())
    }
}

fn remove_content(
    document: &mut Project,
    removed: &BTreeSet<Id>,
    libraries: &HashSet<Id>,
    removed_definitions: &HashSet<Id>,
    assets: &BTreeSet<Id>,
    next_id: Id,
) {
    document.assets.retain(|asset| !assets.contains(&asset.id));
    document.asset_imports.retain(|id, _| !assets.contains(id));
    document.removed_assets.extend(assets);
    document
        .page_imports
        .retain(|page, _| !removed.contains(page));
    // Each historical library owns its source definitions in that snapshot.
    // Keep consumers unchanged so they resolve as missing components.
    let mut definitions: HashSet<_> = document
        .libraries
        .iter()
        .filter(|library| libraries.contains(&library.id))
        .flat_map(|library| library.sources.iter().map(|source| source.definition))
        .collect();
    definitions.extend(removed_definitions);
    document
        .libraries
        .retain(|library| !libraries.contains(&library.id));
    for library in &mut document.libraries {
        let source_nodes: HashSet<_> = library
            .sources
            .iter()
            .filter(|source| definitions.contains(&source.definition))
            .map(|source| source.node)
            .collect();
        library
            .sources
            .retain(|source| !definitions.contains(&source.definition));
        library
            .content
            .nodes
            .retain(|node| !source_nodes.contains(&node.id));
        library
            .content
            .paint_order
            .retain(|path| !source_nodes.contains(&path[0]));
    }
    document
        .definitions
        .retain(|definition| !definitions.contains(&definition.id));
    if document
        .default_library
        .is_some_and(|id| libraries.contains(&id))
    {
        document.default_library = None;
    }
    document.removed_pages.extend(removed);
    document.pages.retain(|p| !removed.contains(&p.id));
    document.remove_unowned_reviews();
    for page in &mut document.pages {
        if page.parent.is_some_and(|id| removed.contains(&id)) {
            page.parent = None;
        }
    }
    let used: HashSet<_> = document
        .pages
        .iter()
        .flat_map(|p| &p.alternates)
        .filter_map(|a| a.version)
        .collect();
    document
        .versions
        .retain(|version| used.contains(&version.id));
    document.next_id = document.next_id.max(next_id);
}
