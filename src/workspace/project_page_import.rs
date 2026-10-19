// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Page and image decisions precede optional whole-project library review.
use super::*;
use std::collections::BTreeMap;
mod equivalence;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageImportChoice {
    UseNew,
    UseExisting,
    KeepBoth,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageImportEntry {
    pub incoming: Id,
    pub name: String,
    pub existing: Option<Id>,
}
pub struct PageImportPlan {
    base: Project,
    source: Project,
    origin: String,
    entries: Vec<PageImportEntry>,
    assets: Vec<AssetImportEntry>,
}
impl Editor {
    pub fn plan_project_page_import(
        &self,
        source: &Project,
        source_origin: &str,
        selected_pages: &[Id],
    ) -> Result<PageImportPlan, String> {
        // Validate inputs without imposing final limits on a temporary Keep Both
        // graph before decisions remove copies of reused/replaced pages.
        project_copy::draft_project_copy(&self.document, source, source_origin, selected_pages)?;
        let selected: HashSet<_> = selected_pages.iter().copied().collect();
        let mut claimed = HashSet::new();
        let mut entries: Vec<_> = source
            .live_pages()
            .filter(|p| selected.contains(&p.id))
            .map(|page| {
                let existing =
                    self.document
                        .live_pages()
                        .find(|target| {
                            !claimed.contains(&target.id)
                                && self.document.page_imports.get(&target.id).is_some_and(
                                    |records| {
                                        records.iter().any(|r| {
                                            r.project == source_origin && r.page == page.id
                                        })
                                    },
                                )
                        })
                        .map(|p| p.id);
                if let Some(id) = existing {
                    claimed.insert(id);
                }
                PageImportEntry {
                    incoming: page.id,
                    name: page.name.clone(),
                    existing,
                }
            })
            .collect();
        // Reserve origin matches before a weaker name match can steal a target.
        for entry in entries.iter_mut().filter(|e| e.existing.is_none()) {
            if let Some(page) = self
                .document
                .live_pages()
                .find(|p| !claimed.contains(&p.id) && p.name == entry.name)
            {
                entry.existing = Some(page.id);
                claimed.insert(page.id);
            }
        }
        Ok(PageImportPlan {
            base: self.document.clone(),
            source: source.clone(),
            origin: source_origin.into(),
            entries,
            assets: project_asset_import::entries(&self.document, source, source_origin),
        })
    }
}
impl PageImportPlan {
    pub fn entries(&self) -> &[PageImportEntry] {
        &self.entries
    }
    pub fn asset_entries(&self) -> &[AssetImportEntry] {
        &self.assets
    }
    pub fn matches_destination(&self, project: &Project) -> bool {
        self.base == *project
    }
    /// Every matched page needs an explicit decision. Dropping a plan cancels.
    pub fn prepare(
        &self,
        choices: &BTreeMap<Id, PageImportChoice>,
    ) -> Result<PreparedProjectCopy, String> {
        self.prepare_inner(choices, None)
    }
    /// Review images and pages together before one atomic project import.
    pub fn prepare_with_assets(
        &self,
        pages: &BTreeMap<Id, PageImportChoice>,
        assets: &BTreeMap<Id, AssetImportChoice>,
    ) -> Result<PreparedProjectCopy, String> {
        self.prepare_inner(
            pages,
            Some(project_asset_import::AssetReview {
                entries: &self.assets,
                choices: assets,
            }),
        )
    }
    pub fn review_libraries(
        &self,
        pages: &BTreeMap<Id, PageImportChoice>,
        assets: &BTreeMap<Id, AssetImportChoice>,
    ) -> Result<ProjectLibraryImportPlan, String> {
        let draft = self.draft_inner(
            pages,
            Some(project_asset_import::AssetReview {
                entries: &self.assets,
                choices: assets,
            }),
        )?;
        project_library_import::plan(draft, &self.source, &self.origin, pages)
    }
    fn prepare_inner(
        &self,
        choices: &BTreeMap<Id, PageImportChoice>,
        review: Option<project_asset_import::AssetReview<'_>>,
    ) -> Result<PreparedProjectCopy, String> {
        self.draft_inner(choices, review)?.prepare()
    }
    fn draft_inner(
        &self,
        choices: &BTreeMap<Id, PageImportChoice>,
        review: Option<project_asset_import::AssetReview<'_>>,
    ) -> Result<project_copy::ProjectCopyDraft, String> {
        if choices.keys().any(|id| {
            !self
                .entries
                .iter()
                .any(|e| e.incoming == *id && e.existing.is_some())
        }) {
            return Err("Choice does not identify a matching page".into());
        }
        if self
            .entries
            .iter()
            .any(|e| e.existing.is_some() && !choices.contains_key(&e.incoming))
        {
            return Err("Choose how to resolve every matching page".into());
        }
        let selected: Vec<_> = self.entries.iter().map(|e| e.incoming).collect();
        let mut draft = project_copy::draft_project_copy_with_assets(
            &self.base,
            &self.source,
            &self.origin,
            &selected,
            review,
        )?;
        let pairs: Vec<_> = self
            .entries
            .iter()
            .zip(draft.pages.iter().copied())
            .collect();
        let mut map = HashMap::new();
        for (entry, new) in &pairs {
            if let Some(existing) = entry.existing
                && choices[&entry.incoming] != PageImportChoice::KeepBoth
            {
                map.insert(*new, existing);
            }
        }
        // Rewrite only incoming content; existing controls keep their links.
        let text_definitions = draft.after.definitions.clone();
        for page in draft
            .after
            .pages
            .iter_mut()
            .filter(|p| draft.pages.contains(&p.id))
        {
            text_link_transfer::remap(&mut page.content.nodes, &text_definitions, &map)?;
            project_copy::rewrite(&mut page.content.nodes, &map);
            page.parent = page.parent.map(|id| map.get(&id).copied().unwrap_or(id));
            for alternate in &mut page.alternates {
                text_link_transfer::remap(&mut alternate.content.nodes, &text_definitions, &map)?;
                project_copy::rewrite(&mut alternate.content.nodes, &map);
            }
        }
        let existing_libraries: HashSet<_> = self.base.libraries.iter().map(|l| l.id).collect();
        let existing_definitions: HashSet<_> = self.base.definitions.iter().map(|d| d.id).collect();
        for library in draft
            .after
            .libraries
            .iter_mut()
            .filter(|l| !existing_libraries.contains(&l.id))
        {
            text_link_transfer::remap(&mut library.content.nodes, &text_definitions, &map)?;
            project_copy::rewrite(&mut library.content.nodes, &map);
        }
        for definition in draft
            .after
            .definitions
            .iter_mut()
            .filter(|d| !existing_definitions.contains(&d.id))
        {
            text_link_transfer::remap(&mut definition.content.nodes, &text_definitions, &map)?;
            project_copy::rewrite(&mut definition.content.nodes, &map);
        }
        for (entry, new) in pairs {
            let Some(&target) = map.get(&new) else {
                continue;
            };
            let source_page = self.source.live_page(entry.incoming).unwrap();
            let current = self.base.live_page(target).unwrap();
            let mut copied = draft
                .after
                .pages
                .remove(draft.after.pages.iter().position(|p| p.id == new).unwrap());
            draft.after.page_imports.remove(&new);
            let records = draft.after.page_imports.entry(target).or_default();
            let old = records
                .iter()
                .find(|r| r.project == self.origin && r.page == entry.incoming);
            let mut binding = old.cloned().unwrap_or(PageImport {
                project: self.origin.clone(),
                page: entry.incoming,
                alternates: BTreeMap::new(),
            });
            match choices[&entry.incoming] {
                PageImportChoice::UseNew => {
                    copied.id = target;
                    copied.name = current.name.clone();
                    copied.parent = current.parent;
                    let mut claimed = HashSet::new();
                    let reserved: HashSet<_> = source_page
                        .alternates
                        .iter()
                        .filter_map(|a| binding.alternates.get(&a.id).copied())
                        .collect();
                    for (incoming, alternate) in
                        source_page.alternates.iter().zip(&mut copied.alternates)
                    {
                        let known = binding
                            .alternates
                            .get(&incoming.id)
                            .copied()
                            .or_else(|| {
                                current
                                    .alternates
                                    .iter()
                                    .find(|a| {
                                        a.name == incoming.name
                                            && !claimed.contains(&a.id)
                                            && !reserved.contains(&a.id)
                                    })
                                    .map(|a| a.id)
                            })
                            .filter(|id| !claimed.contains(id));
                        if let Some(id) = known {
                            alternate.id = id;
                            draft.resources.insert(incoming.id, id);
                        }
                        claimed.insert(alternate.id);
                        binding.alternates.retain(|foreign, local| {
                            *foreign == incoming.id || *local != alternate.id
                        });
                        binding.alternates.insert(incoming.id, alternate.id);
                    }
                    // Replacement updates supplied branches; local branches
                    // omitted by the incoming project remain available.
                    let mut alternates = current.alternates.clone();
                    for alternate in copied.alternates {
                        if let Some(existing) = alternates.iter_mut().find(|a| a.id == alternate.id)
                        {
                            *existing = alternate;
                        } else {
                            alternates.push(alternate);
                        }
                    }
                    copied.alternates = alternates;
                    let index = draft
                        .after
                        .pages
                        .iter()
                        .position(|p| p.id == target)
                        .unwrap();
                    draft.after.pages[index] = copied;
                    draft.counts.pages_replaced += 1;
                }
                PageImportChoice::UseExisting => {
                    let mut claimed: HashSet<_> = binding.alternates.values().copied().collect();
                    for incoming in &source_page.alternates {
                        if !binding.alternates.contains_key(&incoming.id)
                            && let Some(alternate) = current
                                .alternates
                                .iter()
                                .find(|a| a.name == incoming.name && !claimed.contains(&a.id))
                        {
                            binding.alternates.insert(incoming.id, alternate.id);
                            claimed.insert(alternate.id);
                        }
                        if let Some(&target) = binding.alternates.get(&incoming.id) {
                            draft.resources.insert(incoming.id, target);
                        }
                    }
                    draft.counts.pages_reused += 1;
                }
                PageImportChoice::KeepBoth => unreachable!(),
            }
            records.retain(|r| r.project != self.origin || r.page != entry.incoming);
            records.push(binding);
            draft.counts.pages -= 1;
        }
        for page in &mut draft.pages {
            *page = map.get(page).copied().unwrap_or(*page);
        }
        // Reusing a page discards its provisional versions and broken-link
        // markers. Keep only those still needed by the accepted graph.
        let mut used_assets = HashSet::new();
        for content in project_copy::contents(&draft.after) {
            clipboard::references(&content.nodes, &mut HashSet::new(), &mut used_assets);
        }
        draft
            .after
            .removed_assets
            .retain(|id| self.base.removed_assets.contains(id) || used_assets.contains(id));
        let mut used_links = HashSet::new();
        for content in project_copy::contents(&draft.after) {
            project_copy::page_links(&content.nodes, &draft.after.definitions, &mut used_links)?;
        }
        draft
            .after
            .removed_pages
            .retain(|id| self.base.removed_pages.contains(id) || used_links.contains(id));
        let used_versions: HashSet<_> = draft
            .after
            .pages
            .iter()
            .flat_map(|p| p.alternates.iter().filter_map(|a| a.version))
            .collect();
        draft.after.versions.retain(|v| {
            self.base.versions.iter().any(|old| old.id == v.id) || used_versions.contains(&v.id)
        });
        for target in draft.resources.values_mut() {
            *target = map.get(target).copied().unwrap_or(*target);
        }
        // Apply discussion decisions only after final page/alternate bindings.
        // Omitted local alternates and Use Existing retain their local feedback.
        draft.after.reviews = self.base.reviews.clone();
        project_copy::import_reviews(
            &mut draft.after,
            &self.source,
            &draft.resources,
            self.entries
                .iter()
                .filter(|entry| {
                    choices.get(&entry.incoming) != Some(&PageImportChoice::UseExisting)
                })
                .map(|entry| entry.incoming),
        )?;
        used_links.extend(draft.after.review_links()?);
        // Review-only broken destinations also retain their missing identity.
        for target in &used_links {
            if !draft.after.review_surfaces().any(|id| id == *target)
                && draft.resources.values().any(|id| id == target)
            {
                draft.after.removed_pages.insert(*target);
            }
        }
        let allocated = draft.after.next_id;
        draft.after.next_id = self.base.next_id;
        if draft.after != self.base {
            draft.after.next_id = allocated;
        }
        Ok(draft)
    }
}
