// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Whole-library resources are distinct from control clipboard dependencies.
mod correspondence;
mod import;
use super::*;
pub use import::{
    LibraryImportAction, LibraryImportChoice, LibraryImportCounts, LibraryImportEntry,
    LibraryImportPlan, PreparedLibraryImport,
};
pub(super) use import::{ProjectImportContext, unresolved_overrides};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LibraryClipboard {
    pub project: String,
    pub requested: Vec<Id>,
    pub libraries: Vec<ComponentLibrary>,
    pub definitions: Vec<Definition>,
    pub assets: Vec<Asset>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeSet::is_empty")]
    pub removed_assets: std::collections::BTreeSet<Id>,
}
impl Clipboard {
    /// Validate both the envelope and resource graph before planning an import.
    pub fn library_resources(&self) -> Result<&LibraryClipboard, String> {
        let resources = self
            .resources
            .as_ref()
            .ok_or("Clipboard has no library resources")?;
        if !self.nodes.is_empty()
            || !self.definitions.is_empty()
            || !self.assets.is_empty()
            || !self.removed_assets.is_empty()
            || !self.libraries.is_empty()
            || self.has_sources()
            || self.pages.is_some()
        {
            return Err("Library resources cannot be mixed with control clipboard contents".into());
        }
        resources.validate()?;
        Ok(resources)
    }
}
impl Editor {
    /// Capture complete explicitly selected libraries and the transitive library/asset graph.
    /// Origins of earlier imports are not copied: this operation identifies the current project.
    pub fn copy_libraries(&self, requested: &[Id], project: &str) -> Result<Clipboard, String> {
        self.document.validate()?;
        validate_origin(project)?;
        let selected = checked_requested(requested, &self.document.libraries)?;
        let active_libraries: Vec<_> = self.document.live_libraries().cloned().collect();
        let active_definitions: Vec<_> = self.document.live_definitions().cloned().collect();
        let libraries = library_closure(&active_libraries, &active_definitions, selected)?;
        let mut definitions = HashSet::new();
        let mut assets = HashSet::new();
        let mut payload = LibraryClipboard {
            project: project.into(),
            requested: requested.to_vec(),
            libraries: vec![],
            definitions: vec![],
            assets: vec![],
            removed_assets: Default::default(),
        };
        for library in self
            .document
            .libraries
            .iter()
            .filter(|l| libraries.contains(&l.id))
        {
            let mut copy = library.clone();
            copy.imports.clear();
            definitions.extend(library.sources.iter().map(|s| s.definition));
            clipboard::references(&library.content.nodes, &mut HashSet::new(), &mut assets);
            payload.libraries.push(copy);
        }
        for definition in self
            .document
            .definitions
            .iter()
            .filter(|d| definitions.contains(&d.id))
        {
            clipboard::references(&definition.content.nodes, &mut HashSet::new(), &mut assets);
            payload.definitions.push(definition.clone());
        }
        payload.removed_assets = self
            .document
            .removed_assets
            .iter()
            .filter(|id| assets.contains(id))
            .copied()
            .collect();
        payload.assets = self
            .document
            .assets
            .iter()
            .filter(|a| assets.contains(&a.id))
            .cloned()
            .collect();
        payload.validate()?;
        Ok(Clipboard {
            resources: Some(payload),
            ..Clipboard::default()
        })
    }
}
impl LibraryClipboard {
    pub fn validate(&self) -> Result<(), String> {
        self.preview_project().map(|_| ())
    }
    /// Isolated validated resource graph for read-only previews. External page
    /// links are cleared; importing still uses the original clipboard payload.
    pub fn preview_project(&self) -> Result<Project, String> {
        validate_origin(&self.project)?;
        let requested = checked_requested(&self.requested, &self.libraries)?;
        if self
            .libraries
            .iter()
            .any(|l| l.trashed || !l.imports.is_empty())
        {
            return Err(
                "Clipboard libraries must not contain destination import correspondence".into(),
            );
        }
        let mut ids = HashSet::new();
        let mut maximum = 1;
        for id in self
            .libraries
            .iter()
            .map(|l| l.id)
            .chain(self.definitions.iter().map(|d| d.id))
            .chain(self.assets.iter().map(|a| a.id))
            .chain(self.removed_assets.iter().copied())
        {
            if id == 0 || !ids.insert(id) {
                return Err("Duplicate or zero clipboard resource ID".into());
            }
            maximum = maximum.max(id);
        }
        let mut assets = HashSet::new();
        let mut reserved = ids.clone();
        for nodes in self
            .libraries
            .iter()
            .map(|l| &l.content.nodes)
            .chain(self.definitions.iter().map(|d| &d.content.nodes))
        {
            maximum = maximum.max(maximum_node_id(nodes, 0)?);
            clipboard::references(nodes, &mut reserved, &mut assets);
        }
        if assets
            != self
                .assets
                .iter()
                .map(|a| a.id)
                .chain(self.removed_assets.iter().copied())
                .collect()
        {
            return Err("Clipboard assets must match the resource dependency graph".into());
        }
        let page = (1..)
            .find(|id| !reserved.contains(id))
            .ok_or("Clipboard ID space exhausted")?;
        maximum = maximum.max(page);
        let next_id = maximum
            .checked_add(1)
            .filter(|n| *n < u64::MAX)
            .ok_or("Clipboard ID space exhausted")?;
        let mut project = Project {
            libraries: self.libraries.clone(),
            definitions: self.definitions.clone(),
            assets: self.assets.clone(),
            removed_assets: self.removed_assets.clone(),
            next_id,
            font: None,
            ..Project::default()
        };
        project.pages[0].id = page;
        // Page links are external to this payload. Import planning decides whether they survive.
        for nodes in project
            .libraries
            .iter_mut()
            .map(|l| &mut l.content.nodes)
            .chain(project.definitions.iter_mut().map(|d| &mut d.content.nodes))
        {
            text_link_transfer::retain(nodes, &self.definitions, &HashSet::new())?;
            clipboard::strip_missing_links(nodes, &HashSet::new());
        }
        project.validate()?;
        if library_closure(&self.libraries, &self.definitions, requested)?.len()
            != self.libraries.len()
        {
            return Err("Clipboard contains unrelated libraries".into());
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > MAX_FILE_BYTES {
            return Err("Library clipboard exceeds the 64 MB limit".into());
        }
        Ok(project)
    }
}
fn validate_origin(project: &str) -> Result<(), String> {
    if project.is_empty() || project.len() > 512 || project.chars().any(char::is_control) {
        Err("Invalid clipboard project origin".into())
    } else {
        Ok(())
    }
}
fn checked_requested(
    requested: &[Id],
    libraries: &[ComponentLibrary],
) -> Result<HashSet<Id>, String> {
    let ids: HashSet<_> = requested.iter().copied().collect();
    if ids.is_empty()
        || ids.len() != requested.len()
        || ids
            .iter()
            .any(|id| !libraries.iter().any(|l| l.id == *id && !l.trashed))
    {
        return Err("Select distinct existing component libraries".into());
    }
    Ok(ids)
}
fn library_closure(
    libraries: &[ComponentLibrary],
    definitions: &[Definition],
    mut selected: HashSet<Id>,
) -> Result<HashSet<Id>, String> {
    let owners: HashMap<_, _> = libraries
        .iter()
        .flat_map(|l| l.sources.iter().map(move |s| (s.definition, l.id)))
        .collect();
    let definitions: HashMap<_, _> = definitions.iter().map(|d| (d.id, d)).collect();
    let mut visited = HashSet::new();
    while let Some(id) = selected.iter().find(|id| !visited.contains(*id)).copied() {
        visited.insert(id);
        let library = libraries
            .iter()
            .find(|l| l.id == id)
            .ok_or("Clipboard dependency library is missing")?;
        let mut referenced = HashSet::new();
        clipboard::references(&library.content.nodes, &mut referenced, &mut HashSet::new());
        for source in &library.sources {
            let definition = definitions
                .get(&source.definition)
                .ok_or("Clipboard source definition is missing")?;
            clipboard::references(
                &definition.content.nodes,
                &mut referenced,
                &mut HashSet::new(),
            );
        }
        for definition in referenced {
            if let Some(library) = owners.get(&definition) {
                selected.insert(*library);
            } else if definitions.contains_key(&definition) {
                return Err("Clipboard dependency has no owning library".into());
            }
            // A missing source remains an explicit missing reference, not fabricated contents.
        }
    }
    Ok(selected)
}
pub(super) fn maximum_node_id(nodes: &[Node], depth: usize) -> Result<Id, String> {
    if depth > MAX_DEPTH {
        return Err("Control hierarchy is too deep".into());
    }
    let mut maximum = 0;
    for node in nodes {
        maximum = maximum.max(node.id);
        match &node.content {
            NodeContent::Group { children } => {
                maximum = maximum.max(maximum_node_id(children, depth + 1)?)
            }
            NodeContent::Instance {
                definition,
                overrides,
            } => {
                maximum = maximum.max(*definition);
                for patch in overrides {
                    maximum = maximum.max(patch.path.iter().copied().max().unwrap_or(0));
                }
            }
            _ => {}
        }
    }
    Ok(maximum)
}
