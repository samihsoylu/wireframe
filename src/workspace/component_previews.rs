// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Read-only, bounded previews of canonical fresh component instances.
use super::*;
use std::{collections::VecDeque, sync::Arc};

const MAX_ENTRIES: usize = 32;
const MAX_INPUT_BYTES: usize = 32 * 1024 * 1024;

#[derive(Default)]
pub struct ComponentPreviews {
    entries: VecDeque<Entry>,
    input_bytes: usize,
}
struct Entry {
    definition: Id,
    snapshot: Project,
    scene: Arc<crate::render::Scene>,
    bytes: usize,
}
struct Inputs<'a> {
    project: &'a Project,
    definitions: Vec<&'a Definition>,
    roots: Vec<(Id, Option<Crop>, bool)>,
    assets: Vec<&'a Asset>,
    removed_assets: std::collections::BTreeSet<Id>,
}
impl<'a> Inputs<'a> {
    fn collect(project: &'a Project, definition: Id) -> Result<Self, String> {
        let available: HashMap<_, _> = project.live_definitions().map(|d| (d.id, d)).collect();
        if !available.contains_key(&definition) {
            return Err("Component no longer exists".into());
        }
        let mut seen = HashSet::new();
        let mut pending = vec![definition];
        let mut assets = HashSet::new();
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            let Some(source) = available.get(&id) else {
                continue;
            };
            let mut dependencies = HashSet::new();
            clipboard::references(&source.content.nodes, &mut dependencies, &mut assets);
            pending.extend(dependencies);
        }
        let definitions: Vec<_> = project
            .live_definitions()
            .filter(|d| seen.contains(&d.id))
            .collect();
        let roots = definitions
            .iter()
            .map(|d| {
                let (crop, markup) = project.source_appearance(d.id);
                (d.id, crop, markup)
            })
            .collect();
        Ok(Self {
            project,
            definitions,
            roots,
            removed_assets: project
                .removed_assets
                .iter()
                .filter(|id| assets.contains(id))
                .copied()
                .collect(),
            assets: project
                .assets
                .iter()
                .filter(|asset| assets.contains(&asset.id))
                .collect(),
        })
    }
    fn matches(&self, snapshot: &Project) -> bool {
        self.removed_assets == snapshot.removed_assets
            && self.project.settings == snapshot.settings
            && self.project.font == snapshot.font
            && self.project.next_id == snapshot.pages[0].content.nodes[0].id
            && self.definitions.len() == snapshot.definitions.len()
            && self
                .definitions
                .iter()
                .zip(&snapshot.definitions)
                .all(|(a, b)| *a == b)
            && self
                .roots
                .iter()
                .all(|(id, crop, markup)| snapshot.source_appearance(*id) == (*crop, *markup))
            && self.assets.len() == snapshot.assets.len()
            && self
                .assets
                .iter()
                .zip(&snapshot.assets)
                .all(|(a, b)| *a == b)
    }
    fn snapshot(&self, definition: Id) -> Result<Project, String> {
        // Only the reachable component graph and referenced assets are copied.
        // Page contents, versions, notes, history and unrelated libraries never
        // enter a preview. This snapshot is private render input, not a file.
        let mut snapshot = Project {
            settings: self.project.settings.clone(),
            font: self.project.font.clone(),
            next_id: self.project.next_id,
            definitions: self.definitions.iter().map(|d| (*d).clone()).collect(),
            assets: self.assets.iter().map(|a| (*a).clone()).collect(),
            removed_assets: self.removed_assets.clone(),
            ..Project::default()
        };
        let instance = snapshot.allocate_id()?;
        let mut node = Node::group(instance, Point::default(), vec![]);
        node.content = NodeContent::Instance {
            definition,
            overrides: vec![],
        };
        snapshot.pages[0].content.nodes.push(node);
        // Retain only the owning placements needed for shared root appearance.
        // Existing identities avoid consuming extra ID space just to render.
        for library in self.project.live_libraries() {
            let sources: Vec<_> = library
                .sources
                .iter()
                .filter(|source| self.definitions.iter().any(|d| d.id == source.definition))
                .map(|source| ComponentSource {
                    definition: source.definition,
                    node: source.node,
                    notes: String::new(),
                })
                .collect();
            if sources.is_empty() {
                continue;
            }
            let nodes = library
                .content
                .nodes
                .iter()
                .filter(|node| sources.iter().any(|source| source.node == node.id))
                .cloned()
                .collect();
            snapshot.libraries.push(ComponentLibrary {
                id: library.id,
                trashed: false,
                name: String::new(),
                notes: String::new(),
                content: Content {
                    nodes,
                    paint_order: vec![],
                },
                sources,
                imports: vec![],
            });
        }
        snapshot.default_library = snapshot.libraries.first().map(|library| library.id);
        Ok(snapshot)
    }
}
impl ComponentPreviews {
    pub fn scene(
        &mut self,
        project: &Project,
        definition: Id,
    ) -> Result<Arc<crate::render::Scene>, String> {
        self.scene_with_clock(project, definition, crate::tablet::Clock::now)
    }
    /// Time is captured when a new preview is laid out, as on the canvas.
    pub fn scene_with_clock(
        &mut self,
        project: &Project,
        definition: Id,
        clock: fn() -> crate::tablet::Clock,
    ) -> Result<Arc<crate::render::Scene>, String> {
        let inputs = Inputs::collect(project, definition)?;
        if let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.definition == definition)
        {
            let entry = self.entries.remove(index).unwrap();
            if inputs.matches(&entry.snapshot) {
                let scene = entry.scene.clone();
                self.entries.push_back(entry);
                return Ok(scene);
            }
            self.input_bytes -= entry.bytes;
        }
        let snapshot = inputs.snapshot(definition)?;
        let editor = Editor::with_clock(snapshot.clone(), clock);
        let scene = Arc::new(editor.rendered_scene(editor.page)?);
        // The byte budget bounds retained input, including large asset buffers.
        // Oversized previews can render without displacing the entire cache.
        let bytes = serde_json::to_vec(&snapshot)
            .map_err(|e| e.to_string())?
            .len();
        if bytes <= MAX_INPUT_BYTES {
            while self.entries.len() >= MAX_ENTRIES || self.input_bytes + bytes > MAX_INPUT_BYTES {
                if let Some(entry) = self.entries.pop_front() {
                    self.input_bytes -= entry.bytes;
                }
            }
            self.input_bytes += bytes;
            self.entries.push_back(Entry {
                definition,
                snapshot,
                scene: scene.clone(),
                bytes,
            });
        }
        Ok(scene)
    }
}
