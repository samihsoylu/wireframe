// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Editor {
    pub fn active_alternate(&self, page: Id) -> Option<Id> {
        self.active_versions.get(&page).copied().filter(|id| {
            self.document
                .live_pages()
                .any(|p| p.id == page && p.alternates.iter().any(|a| a.id == *id))
        })
    }
    pub fn active_surface(&self, page: Id) -> Id {
        self.active_alternate(page).unwrap_or(page)
    }
    pub fn version_selections(&self) -> HashMap<Id, Id> {
        self.document
            .live_pages()
            .filter_map(|p| self.active_alternate(p.id).map(|a| (p.id, a)))
            .collect()
    }
    pub fn restore_version_selections(&mut self, choices: HashMap<Id, Id>) {
        self.commit();
        self.active_versions = choices;
        self.context.clear();
        self.selection.clear();
        self.reconcile();
    }
    pub fn switch_version(&mut self, page: Id, alternate: Option<Id>) -> Result<(), String> {
        let owner = self
            .document
            .live_pages()
            .find(|p| p.id == page)
            .ok_or("Page no longer exists")?;
        if alternate.is_some_and(|id| !owner.alternates.iter().any(|a| a.id == id)) {
            return Err("Version no longer exists on this page".into());
        }
        self.commit();
        if let Some(id) = alternate {
            self.active_versions.insert(page, id);
        } else {
            self.active_versions.remove(&page);
        }
        self.switch_page(page);
        Ok(())
    }
    /// None selects official everywhere; unmatched pages retain their selection.
    pub fn select_project_version(&mut self, version: Option<Id>) -> Result<(), String> {
        if version.is_some_and(|id| !self.document.versions.iter().any(|v| v.id == id)) {
            return Err("Named version no longer exists".into());
        }
        self.commit();
        if let Some(id) = version {
            for page in self.document.live_pages() {
                if let Some(alt) = page.alternates.iter().find(|a| a.version == Some(id)) {
                    self.active_versions.insert(page.id, alt.id);
                }
            }
        } else {
            self.active_versions.clear();
        }
        self.context.clear();
        self.selection.clear();
        Ok(())
    }
    /// Participates in the caller's notes editing transaction.
    pub fn set_notes(&mut self, notes: String) {
        if let Some(id) = self.definition {
            if let Some(source) = self
                .document
                .libraries
                .iter_mut()
                .flat_map(|l| &mut l.sources)
                .find(|s| s.definition == id)
            {
                source.notes = notes;
            }
            return;
        }
        if let Some(id) = self.library {
            if let Some(library) = self.document.libraries.iter_mut().find(|l| l.id == id) {
                library.notes = notes;
            }
            return;
        }
        if let Some(id) = self.active_alternate(self.page) {
            self.current_mut()
                .alternates
                .iter_mut()
                .find(|a| a.id == id)
                .unwrap()
                .notes = notes;
        } else {
            self.current_mut().notes = notes;
        }
    }
    fn version_name(name: &str) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() || name.len() > 10000 || name.chars().any(char::is_control) {
            return Err("Enter a version name of 1–10000 bytes without control characters".into());
        }
        Ok(name.to_owned())
    }
    fn named_version(&mut self, name: &str) -> Result<Id, String> {
        if let Some(v) = self.document.versions.iter().find(|v| v.name == name) {
            return Ok(v.id);
        }
        let id = self.document.allocate_id()?;
        self.document.versions.push(NamedVersion {
            id,
            name: name.into(),
        });
        Ok(id)
    }
    fn prune_versions(&mut self) {
        let used: HashSet<_> = self
            .document
            .pages
            .iter()
            .flat_map(|p| p.alternates.iter().filter_map(|a| a.version))
            .collect();
        self.document.versions.retain(|v| used.contains(&v.id));
    }
    pub fn create_version(&mut self, name: &str) -> Result<Id, String> {
        if self.in_source_workspace() {
            return Err("Select a page to create a version".into());
        }
        let name = Self::version_name(name)?;
        self.component_change("Create version", |e| {
            if e.page_data().alternates.iter().any(|a| a.name == name) {
                return Err("This page already has a version with that name".into());
            }
            let content = e.active_content().clone();
            let notes = e.document.surface_notes(e.surface_id()).to_owned();
            let version = e.named_version(&name)?;
            let id = e.document.allocate_id()?;
            e.current_mut().alternates.push(Alternate {
                id,
                name,
                version: Some(version),
                notes,
                content,
            });
            e.active_versions.insert(e.page, id);
            e.context.clear();
            e.selection.clear();
            Ok(id)
        })
    }
    pub fn rename_version(&mut self, alternate: Id, name: &str) -> Result<(), String> {
        let name = Self::version_name(name)?;
        self.component_change("Rename version", |e| {
            let alt = e
                .page_data()
                .alternates
                .iter()
                .find(|a| a.id == alternate)
                .ok_or("Version no longer exists on this page")?
                .clone();
            if alt.name == name {
                return Ok(());
            }
            let target = e
                .document
                .versions
                .iter()
                .find(|v| v.name == name)
                .map(|v| v.id);
            match (target, alt.version) {
                (Some(_), _) | (None, None) => {
                    if e.page_data()
                        .alternates
                        .iter()
                        .any(|a| a.id != alternate && a.name == name)
                    {
                        return Err("This page already has a version with that name".into());
                    }
                    let version = e.named_version(&name)?;
                    let alt = e
                        .current_mut()
                        .alternates
                        .iter_mut()
                        .find(|a| a.id == alternate)
                        .unwrap();
                    alt.version = Some(version);
                    alt.name = name;
                }
                (None, Some(id)) => {
                    // Shared rename must also avoid colliding with an isolated legacy label.
                    if e.document.pages.iter().any(|p| {
                        p.alternates.iter().any(|a| a.version == Some(id))
                            && p.alternates
                                .iter()
                                .any(|a| a.version != Some(id) && a.name == name)
                    }) {
                        return Err("A page already has an isolated version with that name".into());
                    }
                    e.document
                        .versions
                        .iter_mut()
                        .find(|v| v.id == id)
                        .unwrap()
                        .name = name.clone();
                    for page in &mut e.document.pages {
                        for alt in &mut page.alternates {
                            if alt.version == Some(id) {
                                alt.name = name.clone();
                            }
                        }
                    }
                }
            }
            e.prune_versions();
            Ok(())
        })
    }
    pub fn discard_version(&mut self, alternate: Id) -> Result<(), String> {
        self.component_change("Discard version", |e| {
            let page = e.current_mut();
            let index = page
                .alternates
                .iter()
                .position(|a| a.id == alternate)
                .ok_or("Version no longer exists on this page")?;
            page.alternates.remove(index);
            e.document.reviews.remove(&alternate);
            e.prune_versions();
            e.reconcile();
            Ok(())
        })
    }
    pub fn promote_version(&mut self, alternate: Id) -> Result<(), String> {
        self.component_change("Promote version", |e| {
            let alt = e
                .page_data()
                .alternates
                .iter()
                .find(|a| a.id == alternate)
                .ok_or("Version no longer exists on this page")?
                .clone();
            let mut name = "Former official".to_owned();
            let mut suffix = 2;
            while e.document.versions.iter().any(|v| v.name == name)
                || e.document
                    .pages
                    .iter()
                    .any(|p| p.alternates.iter().any(|a| a.name == name))
            {
                name = format!("Former official {suffix}");
                suffix += 1;
            }
            let version = e.named_version(&name)?;
            let id = e.document.allocate_id()?;
            let page = e.current_mut();
            page.alternates.push(Alternate {
                id,
                name,
                version: Some(version),
                notes: std::mem::replace(&mut page.notes, alt.notes),
                content: std::mem::replace(&mut page.content, alt.content),
            });
            Ok(())
        })
    }
    pub fn merge_version(&mut self, alternate: Id) -> Result<(), String> {
        let mut alt = self
            .page_data()
            .alternates
            .iter()
            .find(|a| a.id == alternate)
            .ok_or("Version no longer exists on this page")?
            .clone();
        let offset = self
            .rendered_page(self.page)?
            .controls
            .iter()
            .map(|c| c.bounds)
            .reduce(Bounds::union)
            .map_or(0., |b| b.w)
            + 20.;
        let previous = self.snapshot();
        self.switch_version(self.page, None)?;
        // The document history starts on Official, so merge Undo remains on Official.
        let result = self.component_change("Merge version", |e| {
            let mut order = e.page_data().content.paint_paths()?;
            let mut imported_order = alt.content.paint_paths()?;
            let mut identities = HashMap::new();
            for node in &mut alt.content.nodes {
                let id = e.document.allocate_id()?;
                identities.insert(node.id, id);
                node.id = id;
                node.origin.x += offset;
            }
            for path in &mut imported_order {
                path[0] = identities[&path[0]];
            }
            order.extend(imported_order);
            e.selection = identities.values().copied().collect();
            let page = e.current_mut();
            let needs_order =
                !page.content.paint_order.is_empty() || !alt.content.paint_order.is_empty();
            page.content.nodes.extend(alt.content.nodes);
            if needs_order {
                page.content.paint_order = order;
            }
            if !alt.notes.is_empty() {
                if !page.notes.is_empty() {
                    page.notes.push_str("\n--\n");
                }
                page.notes.push_str(&alt.notes);
            }
            Ok(())
        });
        if result.is_err() {
            self.restore(previous);
        }
        result
    }
}
