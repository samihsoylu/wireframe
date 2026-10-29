// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Whole-page transfer is distinct from control and library clipboards. Only
//! selected surfaces and their dependency closure cross the clipboard boundary.
use super::*;
use crate::page_tree::PageTree;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize)]
pub struct PageClipboard {
    project: Project,
    /// Original tree depths preserve discontiguous selection reconstruction.
    levels: Vec<usize>,
}

impl<'de> Deserialize<'de> for PageClipboard {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Payload {
            project: serde_json::Value,
            levels: Vec<usize>,
        }
        let payload = Payload::deserialize(deserializer)?;
        let bytes = serde_json::to_vec(&payload.project).map_err(serde::de::Error::custom)?;
        let project = crate::document::decode(&bytes).map_err(serde::de::Error::custom)?;
        Ok(Self {
            project,
            levels: payload.levels,
        })
    }
}

impl PageClipboard {
    pub fn project(&self) -> &Project {
        &self.project
    }
}

impl Clipboard {
    pub fn page_payload(&self) -> Result<&PageClipboard, String> {
        let pages = self
            .pages
            .as_ref()
            .ok_or("Clipboard does not contain pages")?;
        if !self.nodes.is_empty()
            || !self.definitions.is_empty()
            || !self.assets.is_empty()
            || !self.removed_assets.is_empty()
            || !self.libraries.is_empty()
            || self.has_sources()
            || self.resources.is_some()
        {
            return Err("Page clipboard contains mixed resource types".into());
        }
        valid_origin(
            self.origin
                .as_deref()
                .ok_or("Page clipboard has no origin")?,
        )?;
        if let Some(session) = &self.session {
            valid_origin(session)?;
        }
        pages.project.validate()?;
        if pages.levels.len() != pages.project.pages.len()
            || pages.levels.iter().any(|level| *level >= MAX_DEPTH)
        {
            return Err("Invalid page clipboard hierarchy".into());
        }
        let ids: Vec<_> = pages.project.pages.iter().map(|p| p.id).collect();
        if scoped_project(&pages.project, &ids)? != pages.project {
            return Err("Page clipboard contains unrelated resources".into());
        }
        Ok(pages)
    }

    pub fn pages_from_same_project(&self, editor: &Editor, origin: &str) -> bool {
        self.session.as_deref() == Some(editor.clipboard_session())
            || self.origin.as_deref() == Some(origin)
    }
}

impl Editor {
    /// Callers supply explicit navigator scope, including selected descendants.
    /// No unrelated page text, images, library sources or import paths are copied.
    pub fn copy_pages(&self, ids: &[Id], origin: &str) -> Result<Clipboard, String> {
        valid_origin(origin)?;
        self.document.validate()?;
        let project = scoped_project(&self.document, ids)?;
        let tree = PageTree::new(&self.document);
        let levels = project
            .pages
            .iter()
            .map(|p| tree.row(p.id).unwrap().depth)
            .collect();
        Ok(Clipboard {
            origin: Some(origin.into()),
            session: Some(self.clipboard_session().into()),
            pages: Some(PageClipboard { project, levels }),
            ..Clipboard::default()
        })
    }

    /// Remove only after the native clipboard publication barrier succeeds.
    /// Unlike permanent removal, all data remains in ordinary reversible history.
    pub fn cut_pages(&mut self, ids: &[Id]) -> Result<(), String> {
        let selected = checked_pages(&self.document, ids)?;
        let mut after = self.document.clone();
        trash::retain_pages(&mut after, &selected);
        let surfaces: HashSet<_> = after
            .pages
            .iter()
            .filter(|p| selected.contains(&p.id))
            .flat_map(|p| std::iter::once(p.id).chain(p.alternates.iter().map(|a| a.id)))
            .collect();
        after.pages.retain(|p| !selected.contains(&p.id));
        after.reviews.retain(|id, _| !surfaces.contains(id));
        after.page_imports.retain(|id, _| !selected.contains(id));
        after.removed_pages.extend(surfaces);
        // Retained children also lose a physically removed parent. Their own
        // content and availability are untouched.
        for page in &mut after.pages {
            if page.parent.is_some_and(|id| selected.contains(&id)) {
                page.parent = None;
            }
        }
        if after.live_pages().next().is_none() {
            let id = after.allocate_id()?;
            after.pages.push(Page {
                id,
                trashed: false,
                name: "Wireframe 1".into(),
                notes: String::new(),
                parent: None,
                content: Content::default(),
                alternates: vec![],
            });
        }
        after.encode()?;
        let next = if selected.contains(&self.page) {
            let position = self
                .document
                .pages
                .iter()
                .position(|p| p.id == self.page)
                .unwrap();
            self.document.pages[position + 1..]
                .iter()
                .find(|p| !p.trashed && !selected.contains(&p.id))
                .or_else(|| {
                    self.document.pages[..position]
                        .iter()
                        .rev()
                        .find(|p| !p.trashed && !selected.contains(&p.id))
                })
                .map_or(after.first_live_page().id, |p| p.id)
        } else {
            self.page
        };
        self.edit("Cut wireframes", |e| {
            e.document = after;
            if e.page != next {
                e.page = next;
                e.definition = None;
                e.library = None;
                e.context.clear();
                e.selection.clear();
            }
        });
        self.reconcile();
        Ok(())
    }

    /// Same-document resources keep their current contents. Pages receive new
    /// identities only when the old identity is occupied, including in Trash.
    pub fn prepare_same_project_page_paste(
        &self,
        copy: &Clipboard,
        origin: &str,
        selected: &[Id],
        collapsed: &HashSet<Id>,
    ) -> Result<PreparedProjectCopy, String> {
        valid_origin(origin)?;
        let payload = copy.page_payload()?;
        if !copy.pages_from_same_project(self, origin) {
            return Err("Pages from another project require import review".into());
        }
        let source = &payload.project;
        let mut after = self.document.clone();
        after.next_id = after.next_id.max(source.next_id);
        let occupied = resource_ids(&after);
        // Existing dependency ownership wins, even if its contents changed after
        // Copy. Missing libraries may bring only their selected source closure.
        let existing_libraries: HashSet<_> = after.libraries.iter().map(|l| l.id).collect();
        let skipped_definitions: HashSet<_> = source
            .libraries
            .iter()
            .filter(|l| existing_libraries.contains(&l.id))
            .flat_map(|l| l.sources.iter().map(|s| s.definition))
            .collect();
        for library in &source.libraries {
            if existing_libraries.contains(&library.id) {
                continue;
            }
            ensure_available(&occupied, library.id)?;
            after.libraries.push(library.clone());
        }
        for definition in &source.definitions {
            if after.definitions.iter().any(|d| d.id == definition.id)
                || skipped_definitions.contains(&definition.id)
            {
                continue;
            }
            ensure_available(&occupied, definition.id)?;
            after.definitions.push(definition.clone());
        }
        for asset in &source.assets {
            if after.assets.iter().any(|a| a.id == asset.id) {
                continue;
            }
            if !after.removed_assets.remove(&asset.id) {
                ensure_available(&occupied, asset.id)?;
            }
            after.assets.push(asset.clone());
        }
        for &id in &source.removed_assets {
            if !after.assets.iter().any(|a| a.id == id) {
                if !after.removed_assets.contains(&id) {
                    ensure_available(&occupied, id)?;
                }
                after.removed_assets.insert(id);
            }
        }
        if after.default_library.is_none() {
            let first = after.live_libraries().next().map(|l| l.id);
            after.default_library = first;
        }
        let mut version_map = HashMap::new();
        for version in &source.versions {
            let id = if let Some(v) = after.versions.iter().find(|v| v.id == version.id) {
                v.id
            } else if let Some(v) = after.versions.iter().find(|v| v.name == version.name) {
                v.id
            } else {
                let mut v = version.clone();
                if occupied.contains(&v.id) {
                    v.id = after.allocate_id()?;
                }
                let id = v.id;
                after.versions.push(v);
                id
            };
            version_map.insert(version.id, id);
        }
        let mut names: HashSet<_> = after.live_pages().map(|p| p.name.to_lowercase()).collect();
        let mut pages = vec![];
        let mut surfaces = HashMap::new();
        for page in &source.pages {
            let mut new = page.clone();
            new.id = surface_id(&mut after, &occupied, page.id)?;
            new.name = paste_name(&new.name, &mut names);
            surfaces.insert(page.id, new.id);
            for alternate in &mut new.alternates {
                let old = alternate.id;
                alternate.id = surface_id(&mut after, &occupied, old)?;
                alternate.version = alternate.version.map(|id| version_map[&id]);
                surfaces.insert(old, alternate.id);
            }
            pages.push(new.id);
            after.pages.push(new);
        }
        // Keep existing link targets, including self-links and copied siblings.
        // Only the owner key of a discussion moves to the copied surface.
        for (id, discussion) in &source.reviews {
            after.reviews.insert(surfaces[id], discussion.clone());
        }
        for &id in &source.removed_pages {
            if !after
                .pages
                .iter()
                .any(|p| p.id == id || p.alternates.iter().any(|a| a.id == id))
            {
                if !after.removed_pages.contains(&id) {
                    ensure_available(&occupied, id)?;
                }
                after.removed_pages.insert(id);
            }
        }
        let counts = ProjectCopyCounts {
            pages: pages.len(),
            ..Default::default()
        };
        place_pages(
            &self.document,
            &mut after,
            &pages,
            &payload.levels,
            selected,
            collapsed,
        )?;
        after.encode()?;
        Ok(PreparedProjectCopy {
            base: self.document.clone(),
            after,
            pages,
            counts,
            newly_orphaned_overrides: 0,
        })
    }

    /// Foreign pages use the established complete page/image/library review.
    pub fn plan_page_clipboard_import(&self, copy: &Clipboard) -> Result<PageImportPlan, String> {
        let payload = copy.page_payload()?;
        self.plan_project_page_import(
            &payload.project,
            copy.origin.as_deref().unwrap(),
            &payload
                .project
                .pages
                .iter()
                .map(|p| p.id)
                .collect::<Vec<_>>(),
        )
    }

    /// After conflict review, place its resulting pages at the navigator target.
    /// Reused/replaced pages stay in place; newly imported pages follow scope.
    pub fn position_page_clipboard_import(
        &self,
        copy: &Clipboard,
        mut prepared: PreparedProjectCopy,
        selected: &[Id],
        collapsed: &HashSet<Id>,
    ) -> Result<PreparedProjectCopy, String> {
        let payload = copy.page_payload()?;
        if !prepared.matches_destination(&self.document) {
            return Err("Project changed while page paste was being prepared".into());
        }
        // Use Existing entries are omitted by the import plan. Recover source
        // depths through its page provenance rather than zipping unlike lists.
        let mut ids = vec![];
        let mut levels = vec![];
        let mut names: HashSet<_> = self
            .document
            .live_pages()
            .map(|p| p.name.to_lowercase())
            .collect();
        for (page, level) in payload.project.pages.iter().zip(&payload.levels) {
            if let Some(id) = prepared.pages.iter().find(|id| {
                self.document.live_page(**id).is_none()
                    && prepared.after.page_imports.get(id).is_some_and(|records| {
                        records.iter().any(|r| {
                            Some(r.project.as_str()) == copy.origin.as_deref() && r.page == page.id
                        })
                    })
            }) {
                ids.push(*id);
                levels.push(*level);
                prepared
                    .after
                    .pages
                    .iter_mut()
                    .find(|p| p.id == *id)
                    .unwrap()
                    .name = paste_name(&page.name, &mut names);
            }
        }
        if !ids.is_empty() {
            place_pages(
                &self.document,
                &mut prepared.after,
                &ids,
                &levels,
                selected,
                collapsed,
            )?;
        }
        prepared.after.encode()?;
        Ok(prepared)
    }

    pub fn apply_page_paste(&mut self, prepared: PreparedProjectCopy) -> Result<Vec<Id>, String> {
        if !prepared.matches_destination(&self.document) {
            return Err("Project changed while page paste was being prepared".into());
        }
        prepared.after.encode()?;
        let pages = prepared.pages;
        self.edit("Paste wireframes", |e| e.document = prepared.after);
        self.reconcile();
        Ok(pages)
    }
}

fn valid_origin(origin: &str) -> Result<(), String> {
    if origin.is_empty() || origin.len() > 512 || origin.chars().any(char::is_control) {
        Err("Invalid clipboard project origin".into())
    } else {
        Ok(())
    }
}

fn checked_pages(project: &Project, ids: &[Id]) -> Result<HashSet<Id>, String> {
    let selected: HashSet<_> = ids.iter().copied().collect();
    if selected.is_empty()
        || selected.len() != ids.len()
        || selected.iter().any(|id| project.live_page(*id).is_none())
    {
        return Err("Select distinct live pages".into());
    }
    Ok(selected)
}

fn scoped_project(source: &Project, ids: &[Id]) -> Result<Project, String> {
    let selected = checked_pages(source, ids)?;
    let tree = PageTree::new(source);
    let mut pages = vec![];
    for row in tree.rows.iter().filter(|r| selected.contains(&r.id)) {
        let mut page = source.live_page(row.id).unwrap().clone();
        let mut parent = row.parent;
        while let Some(id) = parent.filter(|id| !selected.contains(id)) {
            parent = tree.row(id).and_then(|r| r.parent);
        }
        page.parent = parent;
        pages.push(page);
    }
    let nodes: Vec<_> = pages
        .iter()
        .flat_map(|p| std::iter::once(&p.content).chain(p.alternates.iter().map(|a| &a.content)))
        .flat_map(|c| c.nodes.iter().cloned())
        .collect();
    let dependency = Editor::new(source.clone()).clipboard_for_nodes(nodes);
    let mut project = Project {
        title: source.title.clone(),
        next_id: source.next_id,
        settings: source.settings.clone(),
        font: source.font.clone(),
        pages,
        definitions: dependency.definitions,
        libraries: dependency.libraries,
        assets: dependency.assets,
        removed_assets: dependency.removed_assets,
        ..Project::default()
    };
    // Retained images remain missing in the destination, without exporting their
    // private retained bytes merely because a page still references their ID.
    project
        .removed_assets
        .extend(project.assets.iter().filter(|a| a.trashed).map(|a| a.id));
    project.assets.retain(|a| !a.trashed);
    project.default_library = project
        .libraries
        .iter()
        .find(|l| Some(l.id) == source.default_library)
        .or_else(|| project.libraries.first())
        .map(|l| l.id);
    let surfaces: HashSet<_> = project
        .pages
        .iter()
        .flat_map(|p| std::iter::once(p.id).chain(p.alternates.iter().map(|a| a.id)))
        .collect();
    project.reviews = source
        .reviews
        .iter()
        .filter(|(id, _)| surfaces.contains(id))
        .map(|(id, value)| (*id, value.clone()))
        .collect();
    let versions: HashSet<_> = project
        .pages
        .iter()
        .flat_map(|p| &p.alternates)
        .filter_map(|a| a.version)
        .collect();
    project.versions = source
        .versions
        .iter()
        .filter(|v| versions.contains(&v.id))
        .cloned()
        .collect();
    let mut links = HashSet::new();
    for content in project_copy::contents(&project) {
        project_copy::page_links(&content.nodes, &project.definitions, &mut links)?;
    }
    for discussion in project.reviews.values() {
        for comment in discussion.comments.values() {
            links.extend(comment.body.surface_references()?);
        }
    }
    project
        .removed_pages
        .extend(links.into_iter().filter(|id| !surfaces.contains(id)));
    project.encode()?;
    Ok(project)
}

fn resource_ids(project: &Project) -> HashSet<Id> {
    project
        .pages
        .iter()
        .flat_map(|p| std::iter::once(p.id).chain(p.alternates.iter().map(|a| a.id)))
        .chain(project.versions.iter().map(|v| v.id))
        .chain(project.definitions.iter().map(|d| d.id))
        .chain(project.libraries.iter().map(|l| l.id))
        .chain(project.assets.iter().map(|a| a.id))
        .chain(project.removed_assets.iter().copied())
        .collect()
}
fn ensure_available(occupied: &HashSet<Id>, id: Id) -> Result<(), String> {
    if occupied.contains(&id) {
        Err("Clipboard dependency identity is occupied by another resource".into())
    } else {
        Ok(())
    }
}
fn surface_id(after: &mut Project, occupied: &HashSet<Id>, id: Id) -> Result<Id, String> {
    if occupied.contains(&id) {
        after.allocate_id()
    } else {
        after.removed_pages.remove(&id);
        Ok(id)
    }
}
fn paste_name(name: &str, names: &mut HashSet<String>) -> String {
    if names.insert(name.to_lowercase()) {
        return name.into();
    }
    let numbered_base = name
        .strip_suffix(')')
        .and_then(|s| s.rsplit_once('('))
        .filter(|(_, number)| !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
        .map(|(base, _)| base);
    for n in 1.. {
        let candidate =
            numbered_base.map_or_else(|| format!("{name} ({n})"), |base| format!("{base}({n})"));
        if names.insert(candidate.to_lowercase()) {
            return candidate;
        }
    }
    unreachable!()
}

fn place_pages(
    base: &Project,
    after: &mut Project,
    ids: &[Id],
    levels: &[usize],
    selected: &[Id],
    collapsed: &HashSet<Id>,
) -> Result<(), String> {
    checked_pages(base, selected)?;
    let tree = PageTree::new(base);
    let selection: HashSet<_> = selected.iter().copied().collect();
    let last = tree
        .rows
        .iter()
        .rev()
        .find(|r| selection.contains(&r.id))
        .unwrap();
    let mut anchor = last;
    if ids.len() > 1 && !last.has_children {
        let mut current = last;
        while let Some(parent) = current.parent.and_then(|id| tree.row(id)) {
            if !selection.contains(&parent.id) {
                break;
            }
            if collapsed.contains(&parent.id) {
                anchor = parent;
            }
            current = parent;
        }
    }
    let child = last.has_children && anchor.id == last.id;
    let parent = if child {
        Some(anchor.id)
    } else {
        anchor.parent
    };
    let base_depth = if child {
        anchor.depth + 1
    } else {
        anchor.depth
    };
    let insert_before = if child {
        tree.rows
            .iter()
            .find(|r| r.parent == Some(anchor.id))
            .map(|r| r.id)
    } else {
        let end = tree.subtree(anchor.id).last().unwrap().id;
        tree.rows
            .iter()
            .skip_while(|r| r.id != end)
            .nth(1)
            .map(|r| r.id)
    };
    let mut copies = vec![];
    for id in ids {
        let position = after
            .pages
            .iter()
            .position(|p| p.id == *id)
            .ok_or("Pasted page no longer exists")?;
        copies.push(after.pages.remove(position));
    }
    let minimum = *levels.iter().min().ok_or("No pages to place")?;
    let mut adjusted = minimum;
    let mut previous_level = 0;
    let mut last_at_level = HashMap::new();
    let mut parents = HashMap::new();
    for i in 0..copies.len() {
        if i > 0 {
            adjusted = match levels[i].cmp(&levels[i - 1]) {
                std::cmp::Ordering::Greater => adjusted + 1,
                std::cmp::Ordering::Equal => adjusted,
                std::cmp::Ordering::Less => minimum,
            };
        }
        let level = (adjusted + base_depth).min(4);
        copies[i].parent = if i == 0 {
            parent
        } else if level > previous_level {
            Some(copies[i - 1].id)
        } else {
            parents[&last_at_level[&level]]
        };
        last_at_level.insert(level, copies[i].id);
        parents.insert(copies[i].id, copies[i].parent);
        previous_level = level;
    }
    let insertion = insert_before
        .and_then(|id| after.pages.iter().position(|p| p.id == id))
        .unwrap_or(after.pages.len());
    after.pages.splice(insertion..insertion, copies);
    Ok(())
}
