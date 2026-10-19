// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::collections::BTreeMap;
use wireframe::workspace::{
    AssetImportAction, AssetImportChoice, LibraryImportAction, LibraryImportChoice,
    PageImportChoice, PageImportPlan, PreparedProjectCopy, ProjectLibraryImportPlan,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum Resource {
    Asset,
    Page,
    Library,
}
#[derive(Clone)]
pub(in crate::app) struct Conflict {
    pub kind: Resource,
    pub incoming: Id,
    pub existing: Id,
    pub name: String,
    pub image_availability: Option<(bool, bool)>,
}
#[derive(Clone, Copy)]
pub(in crate::app) enum Choice {
    New,
    Existing,
    Both,
}
pub(in crate::app) struct Comparison {
    kind: Resource,
    incoming: Id,
    pub scenes: [Result<Arc<Scene>, String>; 2],
}
pub(in crate::app) struct Review {
    plan: PageImportPlan,
    pages: BTreeMap<Id, PageImportChoice>,
    automatic_pages: Vec<Id>,
    assets: BTreeMap<Id, AssetImportChoice>,
    libraries: BTreeMap<Id, LibraryImportChoice>,
    library_plan: Option<ProjectLibraryImportPlan>,
    pub prepared: Option<PreparedProjectCopy>,
    pub all_remaining: bool,
    pub error: Option<String>,
    comparison_editor: Option<Editor>,
    pub pair: Option<Comparison>,
}
impl Review {
    pub fn new(destination: &Editor, snapshot: &Snapshot) -> Result<Self, String> {
        snapshot.verify_source()?;
        let plan = destination.plan_project_page_import(
            &snapshot.project,
            &snapshot.origin,
            &snapshot.import_pages(),
        )?;
        Ok(Self::from_plan(plan))
    }
    pub fn from_plan(plan: PageImportPlan) -> Self {
        let mut review = Self {
            plan,
            pages: BTreeMap::new(),
            automatic_pages: vec![],
            assets: BTreeMap::new(),
            libraries: BTreeMap::new(),
            library_plan: None,
            prepared: None,
            all_remaining: false,
            error: None,
            pair: None,
            comparison_editor: None,
        };
        review.advance();
        review
    }
    pub fn conflicts(&self) -> Vec<Conflict> {
        let assets: Vec<_> = self
            .plan
            .asset_entries()
            .iter()
            .filter(|e| {
                e.action == AssetImportAction::Conflict && !self.assets.contains_key(&e.incoming)
            })
            .map(|e| Conflict {
                kind: Resource::Asset,
                image_availability: Some((e.existing_trashed.unwrap_or(false), e.incoming_trashed)),
                incoming: e.incoming,
                existing: e.existing.unwrap(),
                name: e.name.clone(),
            })
            .collect();
        if !assets.is_empty() {
            return assets;
        }
        let pages: Vec<_> = self
            .plan
            .entries()
            .iter()
            .filter(|e| e.existing.is_some() && !self.pages.contains_key(&e.incoming))
            .map(|e| Conflict {
                kind: Resource::Page,
                image_availability: None,
                incoming: e.incoming,
                existing: e.existing.unwrap(),
                name: e.name.clone(),
            })
            .collect();
        if !pages.is_empty() {
            return pages;
        }
        self.library_plan
            .iter()
            .flat_map(|p| p.entries())
            .filter(|e| {
                e.action == LibraryImportAction::Conflict
                    && !self.libraries.contains_key(&e.incoming)
            })
            .map(|e| Conflict {
                kind: Resource::Library,
                image_availability: None,
                incoming: e.incoming,
                existing: e.existing.unwrap(),
                name: e.name.clone(),
            })
            .collect()
    }
    pub fn choose(&mut self, choice: Choice) {
        let conflicts = self.conflicts();
        let count = if self.all_remaining {
            conflicts.len()
        } else {
            1
        };
        for conflict in conflicts.into_iter().take(count) {
            match conflict.kind {
                Resource::Asset => {
                    self.assets.insert(
                        conflict.incoming,
                        match choice {
                            Choice::New => AssetImportChoice::UseNew,
                            Choice::Existing => AssetImportChoice::UseExisting,
                            Choice::Both => AssetImportChoice::KeepBoth,
                        },
                    );
                }
                Resource::Page => {
                    self.library_plan = None;
                    self.pages.insert(
                        conflict.incoming,
                        match choice {
                            Choice::New => PageImportChoice::UseNew,
                            Choice::Existing => PageImportChoice::UseExisting,
                            Choice::Both => PageImportChoice::KeepBoth,
                        },
                    );
                }
                Resource::Library => {
                    self.libraries.insert(
                        conflict.incoming,
                        match choice {
                            Choice::New => LibraryImportChoice::UseNew,
                            Choice::Existing => LibraryImportChoice::UseExisting,
                            Choice::Both => LibraryImportChoice::KeepBoth,
                        },
                    );
                }
            }
        }
        self.all_remaining = false;
        self.pair = None;
        self.advance();
    }
    fn advance(&mut self) {
        self.prepared = None;
        self.error = None;
        for id in self.automatic_pages.drain(..) {
            self.pages.remove(&id);
        }
        // Comparison is optional; an invalid provisional graph must retain
        // explicit review and its existing validation/recovery paths.
        self.automatic_pages = self
            .plan
            .equivalent_pages(&self.pages, &self.assets, &self.libraries)
            .unwrap_or_default();
        for &id in &self.automatic_pages {
            self.pages.insert(id, PageImportChoice::UseExisting);
        }
        if self.conflicts().iter().any(|c| c.kind != Resource::Library) {
            return;
        }
        match self.plan.review_libraries(&self.pages, &self.assets) {
            Ok(p) => self.library_plan = Some(p),
            Err(e) => {
                self.error = Some(e);
                return;
            }
        }
        self.libraries.retain(|incoming, _| {
            self.library_plan
                .as_ref()
                .unwrap()
                .entries()
                .iter()
                .any(|e| e.incoming == *incoming && e.action == LibraryImportAction::Conflict)
        });
        if !self.conflicts().is_empty() {
            return;
        }
        match self.library_plan.as_ref().unwrap().prepare(&self.libraries) {
            Ok(p) => self.prepared = Some(p),
            Err(e) => self.error = Some(e),
        }
    }
    pub fn valid_destination(&self, editor: &Editor) -> bool {
        self.plan.matches_destination(&editor.document)
    }
    pub fn take_prepared(
        &mut self,
        editor: &Editor,
        snapshot: &Snapshot,
    ) -> Result<PreparedProjectCopy, String> {
        snapshot.verify_source()?;
        self.take_ready(editor)
    }
    pub fn take_ready(&mut self, editor: &Editor) -> Result<PreparedProjectCopy, String> {
        if !self.valid_destination(editor) {
            return Err("The destination project changed. Review the import again.".into());
        }
        self.prepared
            .take()
            .ok_or_else(|| "Finish reviewing every conflict before importing".into())
    }
    pub fn preview_pair(&mut self, destination: &Editor, incoming: &Editor) {
        let Some(c) = self.conflicts().first().cloned() else {
            return;
        };
        if self
            .pair
            .as_ref()
            .is_some_and(|p| p.kind == c.kind && p.incoming == c.incoming)
        {
            return;
        }
        let incoming = self.comparison_editor.get_or_insert_with(|| {
            let mut project = incoming.document.clone();
            project.settings = destination.document.settings.clone();
            project.font = destination.document.font.clone();
            Editor::new(project)
        });
        let mut images = previews::LibraryPreviews::default();
        let mut render = |editor: &Editor, id: Id| {
            if c.kind == Resource::Asset {
                if editor
                    .document
                    .assets
                    .iter()
                    .any(|a| a.id == id && a.trashed)
                {
                    images.retained_asset_scene(&editor.document, id)
                } else {
                    images.item_scene(CatalogItem::Asset(id), &editor.document)
                }
            } else {
                editor.rendered_scene(id).map(std::sync::Arc::new)
            }
        };
        self.pair = Some(Comparison {
            kind: c.kind,
            incoming: c.incoming,
            scenes: [
                render(destination, c.existing),
                render(incoming, c.incoming),
            ],
        });
    }
}
