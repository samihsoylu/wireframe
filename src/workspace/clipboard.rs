// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Clipboard {
    /// Qualified page-link identity; absent in older clipboard payloads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    /// Keeps links valid when this open document is first saved or renamed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    pub nodes: Vec<Node>,
    pub definitions: Vec<Definition>,
    pub assets: Vec<Asset>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeSet::is_empty")]
    pub removed_assets: std::collections::BTreeSet<Id>,

    #[serde(default)]
    pub libraries: Vec<ComponentLibrary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceOrigin>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<SourceOrigin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<super::library_transfer::LibraryClipboard>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<super::page_clipboard::PageClipboard>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SourceOrigin {
    pub project: String,
    pub definition: Id,
    pub node: Id,
}
impl Clipboard {
    pub fn has_sources(&self) -> bool {
        self.source.is_some() || !self.sources.is_empty()
    }
    fn source_origins(&self) -> impl Iterator<Item = &SourceOrigin> {
        self.source.iter().chain(&self.sources)
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
            && self.pages.is_none()
            && self
                .resources
                .as_ref()
                .is_none_or(|r| r.requested.is_empty())
    }
    pub fn len(&self) -> usize {
        self.pages.as_ref().map_or_else(
            || {
                self.resources
                    .as_ref()
                    .map_or(self.nodes.len(), |r| r.requested.len())
            },
            |p| p.project().pages.len(),
        )
    }
}
impl Editor {
    pub fn copy(&self) -> Clipboard {
        let mut nodes: Vec<Node> = self
            .nodes()
            .iter()
            .filter(|n| self.selection.contains(&n.id))
            .cloned()
            .collect();
        let parent = self.context_origin();
        for n in &mut nodes {
            n.clear_empty_component_markers();
            n.origin = n.origin.translated(parent);
        }
        self.clipboard_for_nodes(nodes)
    }
    pub fn copy_source(&self, definition: Id, origin: &str) -> Result<Clipboard, String> {
        self.copy_sources(&[definition], origin)
    }
    /// Same-library resources use a control clipboard; copying whole libraries
    /// is a distinct resource transfer.
    pub fn copy_sources(&self, definitions: &[Id], origin: &str) -> Result<Clipboard, String> {
        let mut nodes = Vec::new();
        let mut sources = Vec::new();
        let mut owner = None;
        let mut seen = HashSet::new();
        for &definition in definitions {
            if !seen.insert(definition) {
                return Err("Duplicate clipboard source".into());
            }
            let (library, source) = self
                .document
                .live_component_source(definition)
                .ok_or("Component source no longer exists")?;
            if owner.is_some_and(|id| id != library.id) {
                return Err("Select sources from one component library".into());
            }
            owner = Some(library.id);
            let node = library
                .content
                .nodes
                .iter()
                .find(|n| n.id == source.node)
                .ok_or("Component source placement is missing")?;
            nodes.push(self.source_group(library.id, node)?);
            sources.push(SourceOrigin {
                project: origin.into(),
                definition,
                node: node.id,
            });
        }
        if nodes.is_empty() {
            return Err("Select a component source".into());
        }
        self.source_clipboard(nodes, sources, origin)
    }
    pub fn copy_with_origin(&self, origin: &str) -> Result<Clipboard, String> {
        if !self.at_library_root() {
            if origin.is_empty() || origin.len() > 512 || origin.chars().any(char::is_control) {
                return Err("Invalid clipboard project origin".into());
            }
            let mut copy = self.copy();
            copy.origin = Some(origin.to_owned());
            return Ok(copy);
        }
        let library = self
            .document
            .libraries
            .iter()
            .find(|l| Some(l.id) == self.library)
            .ok_or("Component library no longer exists")?;
        let mut nodes = Vec::new();
        let mut sources = Vec::new();
        for node in library
            .content
            .nodes
            .iter()
            .filter(|n| self.selection.contains(&n.id))
        {
            nodes.push(self.source_group(library.id, node)?);
            if let Some(source) = library.sources.iter().find(|s| s.node == node.id) {
                sources.push(SourceOrigin {
                    project: origin.into(),
                    definition: source.definition,
                    node: node.id,
                });
            }
        }
        self.source_clipboard(nodes, sources, origin)
    }
    fn source_clipboard(
        &self,
        nodes: Vec<Node>,
        mut sources: Vec<SourceOrigin>,
        origin: &str,
    ) -> Result<Clipboard, String> {
        if origin.is_empty() || origin.len() > 512 || origin.chars().any(char::is_control) {
            return Err("Invalid clipboard project origin".into());
        }
        let mut copy = self.clipboard_for_nodes(nodes);
        copy.origin = Some(origin.to_owned());
        if copy.nodes.len() == 1 && sources.len() == 1 {
            // Retain the original single-source representation for older readers.
            copy.source = sources.pop();
        } else {
            copy.sources = sources;
        }
        Ok(copy)
    }
    pub(super) fn clipboard_for_nodes(&self, nodes: Vec<Node>) -> Clipboard {
        let mut copy = Clipboard {
            origin: Some(format!("session:{}", self.clipboard_session())),
            session: Some(self.clipboard_session().to_owned()),
            nodes,
            ..Clipboard::default()
        };
        let mut defs = HashSet::new();
        let mut assets = HashSet::new();
        references(&copy.nodes, &mut defs, &mut assets);
        loop {
            let add: Vec<_> = self
                .document
                .live_definitions()
                .filter(|d| {
                    defs.contains(&d.id) && !copy.definitions.iter().any(|old| old.id == d.id)
                })
                .cloned()
                .collect();
            if add.is_empty() {
                break;
            }
            for d in add {
                references(&d.content.nodes, &mut defs, &mut assets);
                copy.definitions.push(d);
            }
        }
        copy.removed_assets = self
            .document
            .removed_assets
            .iter()
            .filter(|id| assets.contains(id))
            .copied()
            .collect();
        copy.assets = self
            .document
            .assets
            .iter()
            .filter(|a| assets.contains(&a.id))
            .cloned()
            .collect();
        for library in self.document.live_libraries() {
            let sources: Vec<_> = library
                .sources
                .iter()
                .filter(|s| defs.contains(&s.definition))
                .cloned()
                .collect();
            if sources.is_empty() {
                continue;
            }
            let ids: HashSet<_> = sources.iter().map(|s| s.node).collect();
            let mut selected = library.clone();
            selected.sources = sources;
            selected.imports.clear();
            selected.content.nodes.retain(|n| ids.contains(&n.id));
            selected
                .content
                .paint_order
                .retain(|path| ids.contains(&path[0]));
            copy.libraries.push(selected);
        }
        copy
    }
    pub fn paste(&mut self, copy: &Clipboard, offset: f32) {
        self.paste_from(copy, offset, None);
    }
    /// Origin is supplied by the application file/session boundary. Missing or
    /// different origins can import contents but never retarget existing sources.
    pub fn paste_from(&mut self, copy: &Clipboard, offset: f32, origin: Option<&str>) {
        self.paste_from_at(copy, Point::new(offset, offset), origin);
    }
    pub fn paste_from_at(&mut self, copy: &Clipboard, offset: Point, origin: Option<&str>) {
        if copy.resources.is_some() || copy.pages.is_some() {
            self.error = Some("Resources require the resource import workflow".into());
            return;
        }
        if !self.structure_edit_allowed() {
            return;
        }
        if copy.nodes.is_empty() {
            return;
        }
        let result = self.component_change("Paste controls", |e| {
            let mut copy = copy.clone();
            let session_origin = format!("session:{}", e.clipboard_session());
            let target_origin = origin.unwrap_or(&session_origin);
            if copy
                .origin
                .as_ref()
                .is_some_and(|s| s.is_empty() || s.len() > 512 || s.chars().any(char::is_control))
            {
                return Err("Invalid clipboard project origin".into());
            }
            let same_project = copy.session.as_deref() == Some(e.clipboard_session())
                || copy.origin.as_deref().map_or_else(
                    || {
                        copy.has_sources()
                            && copy.source_origins().all(|s| s.project == target_origin)
                    },
                    |source| source == target_origin,
                );
            let pages: HashSet<_> = if same_project {
                e.document
                    .pages
                    .iter()
                    .map(|p| p.id)
                    .chain(e.document.removed_pages.iter().copied())
                    .collect()
            } else {
                HashSet::new()
            };
            text_link_transfer::retain(&mut copy.nodes, &copy.definitions, &pages)?;
            strip_missing_links(&mut copy.nodes, &pages);
            let text_definitions = copy.definitions.clone();
            for definition in &mut copy.definitions {
                text_link_transfer::retain(
                    &mut definition.content.nodes,
                    &text_definitions,
                    &pages,
                )?;
                strip_missing_links(&mut definition.content.nodes, &pages);
            }
            for library in &mut copy.libraries {
                text_link_transfer::retain(&mut library.content.nodes, &text_definitions, &pages)?;
                strip_missing_links(&mut library.content.nodes, &pages);
            }
            let source_clipboard = copy.has_sources();
            if copy.source.is_some() && (!copy.sources.is_empty() || copy.nodes.len() != 1) {
                return Err("Invalid source clipboard".into());
            }
            let mut source_nodes = HashSet::new();
            let mut source_definitions = HashSet::new();
            let mut reconnect = HashMap::new();
            for source in copy.source_origins() {
                if source.project.is_empty()
                    || source.project.len() > 512
                    || source.definition == 0
                    || source.node == 0
                    || !source_nodes.insert(source.node)
                    || !source_definitions.insert(source.definition)
                    || copy.nodes.iter().filter(|n| n.id == source.node).count() != 1
                    || !copy.nodes.iter().any(|n| {
                        n.id == source.node && matches!(n.content, NodeContent::Group { .. })
                    })
                {
                    return Err("Invalid source clipboard".into());
                }
                if Some(source.project.as_str()) == origin
                    && !e
                        .document
                        .definitions
                        .iter()
                        .any(|d| d.id == source.definition)
                {
                    reconnect.insert(source.node, source.definition);
                }
            }
            if source_clipboard {
                unlock_copied_nodes(&mut copy.nodes);
            }
            // A clipboard can outlive Undo to an earlier allocator state. Fresh
            // definitions must not overlap the missing identities being restored.
            if let Some(&maximum) = reconnect.values().max()
                && maximum >= e.document.next_id
            {
                e.document.next_id = maximum.checked_add(1).ok_or("Project ID space exhausted")?;
            }
            // A control clipboard carries only the source placements needed by
            // its definitions, never an editable library's unrelated contents.
            for library in &copy.libraries {
                if library.content.nodes.len() != library.sources.len()
                    || library
                        .content
                        .nodes
                        .iter()
                        .any(|node| !library.sources.iter().any(|source| source.node == node.id))
                {
                    return Err("Clipboard library contains unrelated controls".into());
                }
            }
            let mut refs = HashSet::new();
            let mut assets = HashSet::new();
            references(&copy.nodes, &mut refs, &mut assets);
            for d in &copy.definitions {
                references(&d.content.nodes, &mut refs, &mut assets);
            }
            let mut missing: Vec<_> = refs
                .into_iter()
                .filter(|id| !copy.definitions.iter().any(|d| d.id == *id))
                .collect();
            missing.sort_unstable();
            let reuse = (copy.removed_assets.is_empty() || same_project)
                && missing.is_empty()
                && copy.definitions.iter().all(|d| {
                    e.document.definition_available(d.id) && e.document.definitions.contains(d)
                })
                && copy.assets.iter().all(|a| e.document.assets.contains(a))
                && copy.removed_assets.is_subset(&e.document.removed_assets);
            let reuse = reuse
                && copy.definitions.iter().all(|definition| {
                    let appearance = copy
                        .libraries
                        .iter()
                        .find_map(|library| {
                            let source = library
                                .sources
                                .iter()
                                .find(|s| s.definition == definition.id)?;
                            library
                                .content
                                .nodes
                                .iter()
                                .find(|n| n.id == source.node)
                                .map(|node| (node.crop, node.markup))
                        })
                        .unwrap_or((None, false));
                    appearance == e.document.source_appearance(definition.id)
                });
            if reuse {
                copy.definitions.clear();
                copy.assets.clear();
                copy.removed_assets.clear();
                copy.libraries.clear();
            }
            let imported_sources = (!copy.definitions.is_empty()).then(|| e.snapshot());
            let mut map = HashMap::new();
            for id in copy
                .definitions
                .iter()
                .map(|d| d.id)
                .chain(copy.assets.iter().map(|a| a.id))
                .chain(copy.removed_assets.iter().copied())
                .chain(copy.libraries.iter().map(|l| l.id))
                .chain(missing)
            {
                if id == 0 || map.contains_key(&id) {
                    return Err("Duplicate or zero clipboard resource ID".into());
                }
                let next = e.document.allocate_id()?;
                map.insert(id, next);
            }
            for d in &mut copy.definitions {
                d.id = map[&d.id];
                remap(&mut d.content.nodes, &map);
            }
            copy.removed_assets = copy.removed_assets.iter().map(|id| map[id]).collect();
            for a in &mut copy.assets {
                a.id = map[&a.id];
            }
            for library in &mut copy.libraries {
                library.imports.clear();
                library.id = map[&library.id];
                remap(&mut library.content.nodes, &map);
                let mut placements = HashMap::new();
                for source in &mut library.sources {
                    let Some(definition) = map.get(&source.definition) else {
                        return Err("Clipboard source definition is missing".into());
                    };
                    source.definition = *definition;
                    if placements.contains_key(&source.node)
                        || !library.content.nodes.iter().any(|n| n.id == source.node)
                    {
                        return Err("Clipboard source placement is missing or duplicated".into());
                    }
                    placements.insert(source.node, e.document.allocate_id()?);
                }
                // Apply the map simultaneously: a fresh destination ID may
                // equal another placement's old, library-scoped ID.
                for node in &mut library.content.nodes {
                    node.id = placements[&node.id];
                }
                for path in &mut library.content.paint_order {
                    if let Some(id) = path.first_mut()
                        && let Some(mapped) = placements.get(id)
                    {
                        *id = *mapped;
                    }
                }
                for source in &mut library.sources {
                    source.node = placements[&source.node];
                }
            }
            remap(&mut copy.nodes, &map);
            let copied_maximum = maximum(&copy.nodes);
            let mut parent = e.context_origin();
            if source_clipboard && let Some(definition) = e.definition {
                let (library, source) = e
                    .document
                    .component_source(definition)
                    .ok_or("Component source no longer exists")?;
                let owner = library
                    .content
                    .nodes
                    .iter()
                    .find(|n| n.id == source.node)
                    .ok_or("Component source placement is missing")?;
                // A source clipboard uses library coordinates; focused source
                // editing uses coordinates relative to its owning placement.
                parent = parent.translated(owner.origin);
            }
            let mut selected = HashSet::new();
            let mut retarget = HashMap::new();
            let mut promoted_sources = Vec::new();
            let single_root = copy.nodes.len() == 1;
            for n in &mut copy.nodes {
                let original_id = n.id;
                let id = e.document.allocate_id()?;
                n.id = id;
                n.locked = false;
                n.origin.x += offset.x - parent.x;
                n.origin.y += offset.y - parent.y;
                if source_nodes.contains(&original_id) && e.at_library_root() {
                    let library = e.library.unwrap();
                    let name = match n.properties.get(GROUP_NAME) {
                        Some(Value::Text(name)) if !name.is_empty() => name.clone(),
                        _ => return Err("Source clipboard name is missing".into()),
                    };
                    let ids: HashSet<_> = e
                        .document
                        .libraries
                        .iter()
                        .find(|l| l.id == library)
                        .ok_or("Component library no longer exists")?
                        .sources
                        .iter()
                        .map(|s| s.definition)
                        .collect();
                    let names: HashSet<_> = e
                        .document
                        .definitions
                        .iter()
                        .filter(|d| ids.contains(&d.id))
                        .map(|d| d.name.to_lowercase())
                        .collect();
                    // Paste collision names reuse a trailing numeric suffix.
                    // Navigator Duplicate has its own separate copy-name policy.
                    let stem = name
                        .strip_suffix(')')
                        .and_then(|s| s.rsplit_once('('))
                        .filter(|(_, digits)| {
                            !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
                        })
                        .map(|(stem, _)| stem.to_owned())
                        .unwrap_or_else(|| format!("{name} "));
                    let mut unique = name.clone();
                    let mut suffix = 0;
                    while names.contains(&unique.to_lowercase()) {
                        suffix += 1;
                        unique = format!("{stem}({suffix})");
                    }
                    n.properties.insert(GROUP_NAME.into(), Value::Text(unique));
                    let (placement, definition) = e.promote_source(library, n.clone())?;
                    *n = placement;
                    promoted_sources.push(definition);
                    if let Some(&old) = reconnect.get(&original_id) {
                        retarget.insert(old, definition);
                    }
                }
                selected.insert(id);
            }
            // Scoped descendant IDs remain stable; advance the allocator above all imported IDs.
            fn maximum(nodes: &[Node]) -> Id {
                nodes
                    .iter()
                    .map(|n| match &n.content {
                        NodeContent::Group { children } => n.id.max(maximum(children)),
                        _ => n.id,
                    })
                    .max()
                    .unwrap_or(0)
            }
            let max = copied_maximum.max(maximum(&copy.nodes)).max(
                copy.definitions
                    .iter()
                    .map(|d| maximum(&d.content.nodes))
                    .max()
                    .unwrap_or(0),
            );
            if max >= e.document.next_id {
                let next = max.checked_add(1).ok_or("Project ID space exhausted")?;
                e.document.next_id = next;
            }
            // A restored source may also have arrived as another root's nested
            // dependency. Point that dependency at the promoted source, rather
            // than retaining an extra hidden owner/library for the same content.
            let replaced_dependencies: HashMap<_, _> = retarget
                .iter()
                .filter_map(|(old, new)| map.get(old).map(|imported| (*imported, *new)))
                .collect();
            if !replaced_dependencies.is_empty() {
                copy.definitions
                    .retain(|d| !replaced_dependencies.contains_key(&d.id));
                for library in &mut copy.libraries {
                    let removed: HashSet<_> = library
                        .sources
                        .iter()
                        .filter(|s| replaced_dependencies.contains_key(&s.definition))
                        .map(|s| s.node)
                        .collect();
                    library.sources.retain(|s| !removed.contains(&s.node));
                    library.content.nodes.retain(|n| !removed.contains(&n.id));
                    library
                        .content
                        .paint_order
                        .retain(|path| !removed.contains(&path[0]));
                }
                copy.libraries.retain(|l| !l.content.nodes.is_empty());
                retarget.extend(replaced_dependencies);
            }
            let had_order = !e.active_content().paint_order.is_empty();
            e.nodes_mut().extend(copy.nodes);
            e.document.definitions.extend(copy.definitions);
            e.document.assets.extend(copy.assets);
            e.document.removed_assets.extend(copy.removed_assets);
            if e.document.default_library.is_none() {
                e.document.default_library = copy.libraries.first().map(|l| l.id);
            }
            e.document.libraries.extend(copy.libraries);
            if !retarget.is_empty() {
                for page in &mut e.document.pages {
                    retarget_sources(&mut page.content.nodes, &retarget);
                    for alternate in &mut page.alternates {
                        retarget_sources(&mut alternate.content.nodes, &retarget);
                    }
                }
                for definition in &mut e.document.definitions {
                    retarget_sources(&mut definition.content.nodes, &retarget);
                }
                for library in &mut e.document.libraries {
                    retarget_sources(&mut library.content.nodes, &retarget);
                    for import in &mut library.imports {
                        for target in import.definitions.values_mut() {
                            if let Some(current) = retarget.get(target) {
                                *target = *current;
                            }
                        }
                    }
                }
            }
            e.selection = selected;
            if had_order {
                // New content must extend the stored leaf order, including group paths.
                let context = e.context.clone();
                let order = Content {
                    nodes: e.nodes().to_vec(),
                    paint_order: vec![],
                }
                .paint_paths()
                .map(|leaves| {
                    leaves
                        .into_iter()
                        .filter(|l| e.selection.contains(&l[0]))
                        .map(|l| {
                            let mut p = context.clone();
                            p.extend(l);
                            p
                        })
                        .collect::<Vec<_>>()
                });
                match order {
                    Ok(paths) => e.active_content_mut().paint_order.extend(paths),
                    Err(err) => return Err(err),
                }
            }
            if let Some(before) = imported_sources {
                // Normalize newly populated empty parents before validating ownership.
                e.repair_empty_order(&before)?;
                e.normalize_source_order()?;
                e.document.organize_components()?;
            }
            // Change the editing surface only after finishing the destination's
            // paint order. Source insertion opens the new resource and its notes.
            if single_root && let Some(&definition) = promoted_sources.first() {
                e.definition = Some(definition);
                e.context.clear();
                e.selection.clear();
            } else if source_clipboard && e.source_owner_locked() {
                e.selection.clear();
            }
            Ok(())
        });
        if let Err(error) = result {
            self.error = Some(error);
        }
    }
    pub fn paste_legacy(&mut self, controls: &[Control], offset: f32) {
        let max = controls
            .iter()
            .flat_map(|c| [c.id, c.group.unwrap_or(0)])
            .max()
            .unwrap_or(1);
        let mut doc = model::Document {
            next_id: max.saturating_add(1),
            ..model::Document::default()
        };
        doc.pages[0].controls = controls.to_vec();
        for c in &mut doc.pages[0].controls {
            c.link = None;
        }
        match Project::try_from(doc) {
            Ok(p) => self.paste(
                &Clipboard {
                    nodes: p.pages[0].content.nodes.clone(),
                    ..Clipboard::default()
                },
                offset,
            ),
            Err(err) => self.error = Some(err),
        }
    }
    pub fn duplicate(&mut self) {
        if self.at_library_root() {
            if let Err(error) = self.duplicate_library_selection() {
                self.error = Some(error);
            }
            return;
        }
        let mut copy = self.copy();
        unlock_copied_nodes(&mut copy.nodes);
        self.paste(&copy, if self.definition.is_some() { 10. } else { 20. });
    }
}
fn retarget_sources(nodes: &mut [Node], mapping: &HashMap<Id, Id>) {
    for node in nodes {
        match &mut node.content {
            NodeContent::Group { children } => retarget_sources(children, mapping),
            NodeContent::Instance { definition, .. } => {
                if let Some(&new) = mapping.get(definition) {
                    *definition = new;
                }
            }
            _ => {}
        }
    }
}
// Unlock the copied tree, never the shared definitions referenced by instances.
pub(super) fn unlock_copied_nodes(nodes: &mut [Node]) {
    for node in nodes {
        node.locked = false;
        if let NodeContent::Group { children } = &mut node.content {
            unlock_copied_nodes(children);
        }
    }
}
pub(super) fn references(nodes: &[Node], defs: &mut HashSet<Id>, assets: &mut HashSet<Id>) {
    fn props(p: &Properties, assets: &mut HashSet<Id>) {
        for v in p.values() {
            match v {
                Value::Asset(id) | Value::Icon(IconSource::Asset(id)) => {
                    assets.insert(*id);
                }
                Value::Icons(icons) => {
                    for icon in icons.iter().flatten() {
                        if let IconSource::Asset(id) = icon {
                            assets.insert(*id);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    for n in nodes {
        props(&n.properties, assets);
        match &n.content {
            NodeContent::Group { children } => references(children, defs, assets),
            NodeContent::Instance {
                definition,
                overrides,
            } => {
                defs.insert(*definition);
                for o in overrides {
                    props(&o.patch.properties, assets);
                }
            }
            _ => {}
        }
    }
}
pub(super) fn remap(nodes: &mut [Node], map: &HashMap<Id, Id>) {
    fn props(p: &mut Properties, map: &HashMap<Id, Id>) {
        for v in p.values_mut() {
            match v {
                Value::Asset(id) | Value::Icon(IconSource::Asset(id)) => {
                    if let Some(new) = map.get(id) {
                        *id = *new;
                    }
                }
                Value::Icons(icons) => {
                    for icon in icons.iter_mut().flatten() {
                        if let IconSource::Asset(id) = icon
                            && let Some(new) = map.get(id)
                        {
                            *id = *new;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    for n in nodes {
        props(&mut n.properties, map);
        match &mut n.content {
            NodeContent::Group { children } => remap(children, map),
            NodeContent::Instance {
                definition,
                overrides,
            } => {
                if let Some(new) = map.get(definition) {
                    *definition = *new;
                }
                for o in overrides {
                    props(&mut o.patch.properties, map);
                }
            }
            _ => {}
        }
    }
}
pub(super) fn strip_missing_links(nodes: &mut [Node], pages: &HashSet<Id>) {
    fn props(p: &mut Properties, pages: &HashSet<Id>) {
        for v in p.values_mut() {
            if let Value::Links(links) = v {
                for target in links {
                    if let Some(Link::Page(id)) = target
                        && !pages.contains(id)
                    {
                        *target = None;
                    }
                }
            }
            if let Value::Link(Link::Page(id)) = v
                && !pages.contains(id)
            {
                *v = Value::Null;
            }
        }
    }
    for n in nodes {
        props(&mut n.properties, pages);
        match &mut n.content {
            NodeContent::Group { children } => strip_missing_links(children, pages),
            NodeContent::Instance { overrides, .. } => {
                for o in overrides {
                    props(&mut o.patch.properties, pages);
                }
            }
            _ => {}
        }
    }
}
