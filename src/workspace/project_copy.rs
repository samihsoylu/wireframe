// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Atomic Keep Both foundation for whole-project imports. Preview loading and
//! conflict decisions belong to callers; this path never replaces resources.
use super::*;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProjectCopyCounts {
    pub pages: usize,
    pub pages_replaced: usize,
    pub pages_reused: usize,
    pub libraries: usize,
    pub libraries_replaced: usize,
    pub libraries_reused: usize,
    pub assets_added: usize,
    pub assets_replaced: usize,
    pub assets_reused: usize,
}

pub struct PreparedProjectCopy {
    pub(super) base: Project,
    pub(super) after: Project,
    pub(super) pages: Vec<Id>,
    pub(super) counts: ProjectCopyCounts,
    pub(super) newly_orphaned_overrides: usize,
}
pub(super) struct ProjectCopyDraft {
    pub resources: HashMap<Id, Id>,
    pub base: Project,
    pub after: Project,
    pub pages: Vec<Id>,
    pub counts: ProjectCopyCounts,
}
impl ProjectCopyDraft {
    pub fn prepare(self) -> Result<PreparedProjectCopy, String> {
        // Validate both the final graph and the serialized file-size limit.
        self.after.encode()?;
        Ok(PreparedProjectCopy {
            base: self.base,
            after: self.after,
            pages: self.pages,
            counts: self.counts,
            newly_orphaned_overrides: 0,
        })
    }
}
impl PreparedProjectCopy {
    pub fn newly_orphaned_overrides(&self) -> usize {
        self.newly_orphaned_overrides
    }
    pub fn counts(&self) -> ProjectCopyCounts {
        self.counts
    }
    pub fn imported_pages(&self) -> &[Id] {
        &self.pages
    }
    pub fn preview(&self) -> &Project {
        &self.after
    }
    pub fn matches_destination(&self, project: &Project) -> bool {
        self.base == *project
    }
}

impl Editor {
    /// Copy selected live pages, all live component libraries and all assets.
    /// Selection is explicit: catalog policy can omit empty pages separately.
    /// Omitted ancestors promote their selected descendants; omitted link targets
    /// retain a fresh missing-page identity, never a coincident destination ID.
    pub fn prepare_project_copy(
        &self,
        source: &Project,
        source_origin: &str,
        selected_pages: &[Id],
    ) -> Result<PreparedProjectCopy, String> {
        draft_project_copy(&self.document, source, source_origin, selected_pages)?.prepare()
    }
    pub fn apply_project_copy(
        &mut self,
        prepared: PreparedProjectCopy,
    ) -> Result<ProjectCopyCounts, String> {
        if !prepared.matches_destination(&self.document) {
            return Err(
                "Project changed while import was being prepared; plan the import again".into(),
            );
        }
        if prepared.after == self.document {
            return Ok(prepared.counts);
        }
        self.component_change("Import project", |editor| {
            editor.document = prepared.after;
            Ok(prepared.counts)
        })
    }
}

pub(super) fn draft_project_copy(
    base: &Project,
    source: &Project,
    source_origin: &str,
    selected_pages: &[Id],
) -> Result<ProjectCopyDraft, String> {
    draft_project_copy_with_assets(base, source, source_origin, selected_pages, None)
}
pub(super) fn draft_project_copy_with_assets(
    base: &Project,
    source: &Project,
    source_origin: &str,
    selected_pages: &[Id],
    review: Option<project_asset_import::AssetReview<'_>>,
) -> Result<ProjectCopyDraft, String> {
    base.validate()?;
    source.validate()?;
    if source_origin.is_empty()
        || source_origin.len() > 512
        || source_origin.chars().any(char::is_control)
    {
        return Err("Invalid source project identity".into());
    }
    let selected: HashSet<_> = selected_pages.iter().copied().collect();
    if selected.len() != selected_pages.len()
        || selected.iter().any(|id| source.live_page(*id).is_none())
    {
        return Err("Choose distinct live pages from the source project".into());
    }
    let mut after = base.clone();
    // Scoped node IDs and orphan override paths stay intact. Reserve them
    // before allocating resources so later edits cannot capture old paths.
    for project in [base, source] {
        for content in contents(project) {
            let maximum = library_transfer::maximum_node_id(&content.nodes, 0)?;
            after.next_id = after
                .next_id
                .max(maximum.checked_add(1).ok_or("Project ID space exhausted")?);
        }
    }
    let mut map = HashMap::new();
    let mut counts = ProjectCopyCounts::default();
    for page in source.live_pages().filter(|p| selected.contains(&p.id)) {
        map.insert(page.id, after.allocate_id()?);
        for alternate in &page.alternates {
            map.insert(alternate.id, after.allocate_id()?);
        }
    }
    for library in source.live_libraries() {
        map.insert(library.id, after.allocate_id()?);
    }
    for definition in source.live_definitions() {
        map.insert(definition.id, after.allocate_id()?);
    }
    map.extend(project_asset_import::stage(
        &mut after,
        source,
        source_origin,
        review,
        &mut counts,
    )?);
    for version in &source.versions {
        if !source
            .live_pages()
            .filter(|p| selected.contains(&p.id))
            .any(|p| p.alternates.iter().any(|a| a.version == Some(version.id)))
        {
            continue;
        }
        let id = if let Some(existing) = after.versions.iter().find(|v| v.name == version.name) {
            existing.id
        } else {
            let mut copy = version.clone();
            copy.id = after.allocate_id()?;
            let id = copy.id;
            after.versions.push(copy);
            id
        };
        map.insert(version.id, id);
    }
    // References to unavailable resources need fresh reserved identities too.
    let imported_contents: Vec<_> = source
        .live_pages()
        .filter(|p| selected.contains(&p.id))
        .flat_map(|p| std::iter::once(&p.content).chain(p.alternates.iter().map(|a| &a.content)))
        .chain(source.live_libraries().map(|l| &l.content))
        .chain(source.live_definitions().map(|d| &d.content))
        .collect();
    let mut definitions = HashSet::new();
    let mut links = HashSet::new();
    for content in imported_contents {
        clipboard::references(&content.nodes, &mut definitions, &mut HashSet::new());
        page_links(&content.nodes, &source.definitions, &mut links)?;
    }
    for page in source.live_pages().filter(|p| selected.contains(&p.id)) {
        for surface in std::iter::once(page.id).chain(page.alternates.iter().map(|a| a.id)) {
            if let Some(discussion) = source.reviews.get(&surface) {
                for comment in discussion.comments.values() {
                    links.extend(comment.body.surface_references()?);
                }
            }
        }
    }
    let mut missing_definitions: Vec<_> = definitions
        .into_iter()
        .filter(|id| !map.contains_key(id))
        .collect();
    missing_definitions.sort_unstable();
    for id in missing_definitions {
        map.insert(id, after.allocate_id()?);
    }
    let mut missing_pages: Vec<_> = links
        .into_iter()
        .filter(|id| !map.contains_key(id))
        .collect();
    missing_pages.sort_unstable();
    for id in missing_pages {
        let target = after.allocate_id()?;
        map.insert(id, target);
        after.removed_pages.insert(target);
    }
    let mut names: HashSet<_> = after.pages.iter().map(|p| p.name.to_lowercase()).collect();
    let mut imported_pages = vec![];
    for page in source.live_pages().filter(|p| selected.contains(&p.id)) {
        let mut copy = page.clone();
        copy.id = map[&page.id];
        copy.name = unique_name(&page.name, &mut names);
        let mut parent = page.parent;
        while parent.is_some_and(|id| !selected.contains(&id)) {
            parent = source
                .pages
                .iter()
                .find(|p| Some(p.id) == parent)
                .and_then(|p| p.parent);
        }
        copy.parent = parent.map(|id| map[&id]);
        text_link_transfer::remap(&mut copy.content.nodes, &source.definitions, &map)?;
        rewrite(&mut copy.content.nodes, &map);
        for alternate in &mut copy.alternates {
            alternate.id = map[&alternate.id];
            alternate.version = alternate.version.map(|id| map[&id]);
            text_link_transfer::remap(&mut alternate.content.nodes, &source.definitions, &map)?;
            rewrite(&mut alternate.content.nodes, &map);
        }
        after.page_imports.insert(
            copy.id,
            vec![PageImport {
                project: source_origin.into(),
                page: page.id,
                alternates: page.alternates.iter().map(|a| (a.id, map[&a.id])).collect(),
            }],
        );
        imported_pages.push(copy.id);
        after.pages.push(copy);
        counts.pages += 1;
    }
    for definition in source.live_definitions() {
        let mut copy = definition.clone();
        copy.id = map[&copy.id];
        text_link_transfer::remap(&mut copy.content.nodes, &source.definitions, &map)?;
        rewrite(&mut copy.content.nodes, &map);
        after.definitions.push(copy);
    }
    let mut names: HashSet<_> = after
        .libraries
        .iter()
        .map(|l| l.name.to_lowercase())
        .collect();
    for library in source.live_libraries() {
        let mut copy = library.clone();
        copy.id = map[&copy.id];
        copy.name = unique_name(&copy.name, &mut names);
        text_link_transfer::remap(&mut copy.content.nodes, &source.definitions, &map)?;
        rewrite(&mut copy.content.nodes, &map);
        let mut bindings = BTreeMap::new();
        for source in &mut copy.sources {
            let original = source.definition;
            source.definition = map[&original];
            bindings.insert(original, source.definition);
        }
        copy.imports = vec![LibraryImport {
            project: source_origin.into(),
            library: library.id,
            definitions: bindings,
        }];
        if after.default_library.is_none()
            && (source.default_library == Some(library.id) || source.default_library.is_none())
        {
            after.default_library = Some(copy.id);
        }
        after.libraries.push(copy);
        counts.libraries += 1;
    }
    if after.default_library.is_none() && counts.libraries > 0 {
        let first = after.live_libraries().next().map(|l| l.id);
        after.default_library = first;
    }
    import_reviews(&mut after, source, &map, selected.iter().copied())?;
    // An empty selection from a truly empty project is a no-op.
    let allocated = after.next_id;
    after.next_id = base.next_id;
    if after != *base {
        after.next_id = allocated;
    }
    Ok(ProjectCopyDraft {
        resources: map,
        base: base.clone(),
        after,
        pages: imported_pages,
        counts,
    })
}

pub(super) fn contents(project: &Project) -> impl Iterator<Item = &Content> {
    project
        .pages
        .iter()
        .flat_map(|p| std::iter::once(&p.content).chain(p.alternates.iter().map(|a| &a.content)))
        .chain(project.definitions.iter().map(|d| &d.content))
        .chain(project.libraries.iter().map(|l| &l.content))
}

/// Each discussion owns its parent/marker namespace, so copying the complete
/// surface preserves those links without risking collisions with another page.
pub(super) fn import_reviews(
    after: &mut Project,
    source: &Project,
    map: &HashMap<Id, Id>,
    pages: impl Iterator<Item = Id>,
) -> Result<(), String> {
    for id in pages {
        let Some(page) = source.live_page(id) else {
            continue;
        };
        for surface in std::iter::once(page.id).chain(page.alternates.iter().map(|a| a.id)) {
            let Some(&target) = map.get(&surface) else {
                continue;
            };
            after.reviews.remove(&target);
            if let Some(discussion) = source.reviews.get(&surface) {
                let mut imported = discussion.clone();
                for comment in imported.comments.values_mut() {
                    comment.imported = true;
                    comment.body.text =
                        crate::rich_text::rewrite_link_targets(&comment.body.text, |target| {
                            target
                                .strip_prefix("surface:")
                                .and_then(|id| id.parse::<Id>().ok())
                                .and_then(|id| map.get(&id))
                                .map(|id| format!("surface:{id}"))
                        })?;
                }
                after.reviews.insert(target, imported);
            }
        }
    }
    Ok(())
}
pub(super) fn unique_name(name: &str, occupied: &mut HashSet<String>) -> String {
    if occupied.insert(name.to_lowercase()) {
        return name.into();
    }
    for suffix in 2.. {
        let next = format!("{name} ({suffix})");
        if occupied.insert(next.to_lowercase()) {
            return next;
        }
    }
    unreachable!()
}
fn properties(nodes: &mut [Node], f: &mut impl FnMut(&mut Properties)) {
    for node in nodes {
        f(&mut node.properties);
        match &mut node.content {
            NodeContent::Group { children } => properties(children, f),
            NodeContent::Instance { overrides, .. } => {
                for item in overrides {
                    f(&mut item.patch.properties);
                }
            }
            _ => {}
        }
    }
}
pub(super) fn page_links(
    nodes: &[Node],
    definitions: &[Definition],
    links: &mut HashSet<Id>,
) -> Result<(), String> {
    text_link_transfer::collect(nodes, definitions, links)?;
    typed_page_links(nodes, links);
    Ok(())
}
fn typed_page_links(nodes: &[Node], links: &mut HashSet<Id>) {
    fn collect(properties: &Properties, links: &mut HashSet<Id>) {
        for value in properties.values() {
            match value {
                Value::Link(Link::Page(id)) => {
                    links.insert(*id);
                }
                Value::Links(values) => {
                    for value in values {
                        if let Some(Link::Page(id)) = value {
                            links.insert(*id);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    for node in nodes {
        collect(&node.properties, links);
        match &node.content {
            NodeContent::Group { children } => typed_page_links(children, links),
            NodeContent::Instance { overrides, .. } => {
                for item in overrides {
                    collect(&item.patch.properties, links);
                }
            }
            _ => {}
        }
    }
}
pub(super) fn rewrite(nodes: &mut [Node], map: &HashMap<Id, Id>) {
    clipboard::remap(nodes, map);
    properties(nodes, &mut |properties| {
        for value in properties.values_mut() {
            match value {
                Value::Link(Link::Page(id)) => {
                    if let Some(target) = map.get(id) {
                        *id = *target;
                    }
                }
                Value::Links(values) => {
                    for value in values {
                        if let Some(Link::Page(id)) = value
                            && let Some(target) = map.get(id)
                        {
                            *id = *target;
                        }
                    }
                }
                _ => {}
            }
        }
    });
}
