// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Combined resource review remains isolated until one Import project action.
use super::*;
use std::collections::BTreeMap;

pub struct ProjectLibraryImportPlan {
    draft: project_copy::ProjectCopyDraft,
    libraries: Option<LibraryImportPlan>,
}
impl ProjectLibraryImportPlan {
    pub fn entries(&self) -> &[LibraryImportEntry] {
        self.libraries
            .as_ref()
            .map(|p| p.entries())
            .unwrap_or_default()
    }
    pub fn matches_destination(&self, document: &Project) -> bool {
        self.draft.base == *document
    }
    pub fn prepare(
        &self,
        choices: &BTreeMap<Id, LibraryImportChoice>,
    ) -> Result<PreparedProjectCopy, String> {
        let mut counts = self.draft.counts;
        let mut after = if let Some(plan) = &self.libraries {
            let prepared = plan.prepare(choices)?;
            counts.libraries = prepared.counts.added;
            counts.libraries_replaced = prepared.counts.replaced;
            counts.libraries_reused = prepared.counts.reused;
            prepared.after
        } else {
            if !choices.is_empty() {
                return Err("Choice does not identify a pending library conflict".into());
            }
            self.draft.after.clone()
        };
        let preferred_library = if self.draft.base.default_library.is_none()
            && let Some(preferred) = self.draft.after.default_library
            && let Some(record) = self
                .draft
                .after
                .libraries
                .iter()
                .find(|l| l.id == preferred)
                .and_then(|l| l.imports.first())
        {
            after
                .live_libraries()
                .find(|l| {
                    l.imports
                        .iter()
                        .any(|i| i.project == record.project && i.library == record.library)
                })
                .map(|l| l.id)
        } else {
            None
        };
        if let Some(target) = preferred_library {
            after.default_library = Some(target);
        }
        let mut used_assets = HashSet::new();
        for content in project_copy::contents(&after) {
            clipboard::references(&content.nodes, &mut HashSet::new(), &mut used_assets);
        }
        after
            .removed_assets
            .retain(|id| self.draft.base.removed_assets.contains(id) || used_assets.contains(id));
        let mut used_links = HashSet::new();
        for content in project_copy::contents(&after) {
            project_copy::page_links(&content.nodes, &after.definitions, &mut used_links)?;
        }
        used_links.extend(after.review_links()?);
        after
            .removed_pages
            .retain(|id| self.draft.base.removed_pages.contains(id) || used_links.contains(id));
        let allocated = after.next_id;
        after.next_id = self.draft.base.next_id;
        if after != self.draft.base {
            after.next_id = allocated;
        }
        // Final graph/file limits apply after every resource choice, not to the
        // provisional Keep Both graph used while identifying collisions.
        after.encode()?;
        let newly_orphaned_overrides = library_transfer::unresolved_overrides(&after)?
            .difference(&library_transfer::unresolved_overrides(&self.draft.base)?)
            .count();
        Ok(PreparedProjectCopy {
            base: self.draft.base.clone(),
            after,
            pages: self.draft.pages.clone(),
            counts,
            newly_orphaned_overrides,
        })
    }
}

pub(super) fn plan(
    draft: project_copy::ProjectCopyDraft,
    source: &Project,
    origin: &str,
    pages: &BTreeMap<Id, PageImportChoice>,
) -> Result<ProjectLibraryImportPlan, String> {
    if source.live_libraries().next().is_none() {
        return Ok(ProjectLibraryImportPlan {
            draft,
            libraries: None,
        });
    }
    let mut destination = draft.after.clone();
    // Remove provisional copies, retaining all destination libraries, including
    // retained resources and definitions referenced by existing local edits.
    destination
        .libraries
        .retain(|l| draft.base.libraries.iter().any(|old| old.id == l.id));
    destination
        .definitions
        .retain(|d| draft.base.definitions.iter().any(|old| old.id == d.id));
    destination.default_library = draft.base.default_library;
    let libraries: Vec<_> = source
        .live_libraries()
        .map(|l| {
            let mut library = l.clone();
            library.imports.clear();
            library
        })
        .collect();
    let mut payload = LibraryClipboard {
        project: origin.into(),
        requested: libraries.iter().map(|l| l.id).collect(),
        libraries,
        definitions: source.live_definitions().cloned().collect(),
        assets: source.assets.clone(),
        removed_assets: source.removed_assets.clone(),
    };
    // Whole-project assets already reside in the page/asset-decided draft.
    // The clipboard envelope must contain only library dependencies; unrelated
    // page images and unused project assets remain in the final draft/map.
    let mut library_assets = HashSet::new();
    for content in payload
        .libraries
        .iter()
        .map(|l| &l.content)
        .chain(payload.definitions.iter().map(|d| &d.content))
    {
        clipboard::references(&content.nodes, &mut HashSet::new(), &mut library_assets);
    }
    payload.assets.retain(|a| library_assets.contains(&a.id));
    payload
        .removed_assets
        .retain(|id| library_assets.contains(id));
    let mut surfaces = vec![];
    for page in source.live_pages() {
        let Some(&target) = draft.resources.get(&page.id) else {
            continue;
        };
        if !draft.pages.contains(&target)
            || pages.get(&page.id) == Some(&PageImportChoice::UseExisting)
        {
            continue;
        }
        surfaces.push((target, page.content.clone()));
        surfaces.extend(
            page.alternates
                .iter()
                .map(|a| (draft.resources[&a.id], a.content.clone())),
        );
    }
    let mut resources = draft.resources.clone();
    let mut definitions = HashSet::new();
    for content in project_copy::contents(source) {
        clipboard::references(&content.nodes, &mut definitions, &mut HashSet::new());
    }
    resources.retain(|id, _| {
        !definitions.contains(id)
            && !source.definitions.iter().any(|d| d.id == *id)
            && !source.libraries.iter().any(|l| l.id == *id)
    });
    let clipboard = Clipboard {
        resources: Some(payload),
        ..Clipboard::default()
    };
    let libraries = LibraryImportPlan::with_context(
        &destination,
        &clipboard,
        "project-import-destination",
        Some(library_transfer::ProjectImportContext {
            resources,
            surfaces,
        }),
    )?;
    Ok(ProjectLibraryImportPlan {
        draft,
        libraries: Some(libraries),
    })
}
