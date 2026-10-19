// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Fresh shared-image insertion and bounded catalog preview inputs.
use super::*;
use std::{collections::VecDeque, sync::Arc};

fn asset(project: &Project, id: Id) -> Result<&Asset, String> {
    project
        .live_asset(id)
        .ok_or_else(|| "Image no longer exists".into())
}
fn image_node(id: Id, source: Id, origin: Point) -> Node {
    let mut node = catalog::get("image").unwrap().create(id, origin);
    node.properties.insert("image".into(), Value::Asset(source));
    node
}
impl Editor {
    /// Insert a new image referencing shared bytes. Canvas overrides never
    /// become catalog defaults. Validate before finalizing unrelated edits.
    pub fn insert_catalog_asset(&mut self, source: Id, x: f32, y: f32) -> Result<Id, String> {
        self.require_structure_edit()?;
        crate::images::decoded(&asset(&self.document, source)?.bytes)?;
        if !x.is_finite() || !y.is_finite() {
            return Err("Choose a finite image position".into());
        }
        if self.pending.is_some() {
            let mut check = Editor::new(self.document.clone());
            check.page = self.page;
            check.active_versions = self.active_versions.clone();
            check.definition = self.definition;
            check.library = self.library;
            check.context = self.context.clone();
            check.insert_catalog_asset(source, x, y)?;
        }
        self.component_change("Add image", |editor| {
            let id = editor.document.allocate_id()?;
            let parent = editor.context_origin();
            editor.nodes_mut().push(image_node(
                id,
                source,
                Point::new(x - parent.x, y - parent.y),
            ));
            editor.append_order(id);
            editor.selection = HashSet::from([id]);
            Ok(id)
        })
    }
}

#[derive(Default)]
pub struct AssetPreviews {
    entries: VecDeque<Entry>,
    bytes: usize,
}
struct Entry {
    source: Id,
    bytes: Vec<u8>,
    weight: usize,
    settings: Settings,
    font: Option<Arc<crate::fonts::ProjectFont>>,
    scene: Arc<crate::render::Scene>,
}
impl AssetPreviews {
    /// Cache hits borrow the current asset; unrelated pages and assets never
    /// enter the comparison or snapshot. Names do not change image artwork.
    pub fn scene(
        &mut self,
        project: &Project,
        source: Id,
    ) -> Result<Arc<crate::render::Scene>, String> {
        self.for_asset(project, asset(project, source)?)
    }
    /// Read-only Trash artwork never changes canonical availability.
    pub fn retained_scene(
        &mut self,
        project: &Project,
        source: Id,
    ) -> Result<Arc<crate::render::Scene>, String> {
        let asset = project
            .assets
            .iter()
            .find(|a| a.id == source && a.trashed)
            .ok_or("Image is no longer in Trash")?;
        self.for_asset(project, asset)
    }
    fn for_asset(
        &mut self,
        project: &Project,
        asset: &Asset,
    ) -> Result<Arc<crate::render::Scene>, String> {
        let source = asset.id;
        if let Some(index) = self.entries.iter().position(|entry| {
            entry.source == source
                && entry.bytes == asset.bytes
                && entry.settings == project.settings
                && entry.font == project.font
        }) {
            let entry = self.entries.remove(index).unwrap();
            let scene = entry.scene.clone();
            self.entries.push_back(entry);
            return Ok(scene);
        }
        // Drop obsolete artwork before attempting to decode a replacement.
        self.entries.retain(|entry| entry.source != source);
        self.bytes = self.entries.iter().map(|entry| entry.weight).sum();
        // Account for encoded input, decoded pixels and the renderer's PNG
        // copy. This bounds retained image data, not total process/GPU memory.
        let weight = {
            let decoded = crate::images::decoded(&asset.bytes)?;
            asset.bytes.len() + decoded.png.len() + decoded.pixels.pixels.len() * 4
        };
        // Private render-only input uses fresh IDs, independent of allocator
        // exhaustion and all canvas control overrides in the user's document.
        let mut snapshot = Project {
            settings: project.settings.clone(),
            font: project.font.clone(),
            ..Project::default()
        };
        let mut image = asset.clone();
        image.trashed = false;
        image.id = snapshot.allocate_id()?;
        let node = image_node(snapshot.allocate_id()?, image.id, Point::default());
        snapshot.assets.push(image);
        snapshot.pages[0].content.nodes.push(node);
        let editor = Editor::new(snapshot);
        let scene = Arc::new(editor.rendered_scene(editor.page)?);
        const MAX_ENTRIES: usize = 32;
        const MAX_IMAGE_BYTES: usize = 64 * 1024 * 1024;
        if weight <= MAX_IMAGE_BYTES {
            while self.entries.len() >= MAX_ENTRIES || self.bytes + weight > MAX_IMAGE_BYTES {
                if let Some(entry) = self.entries.pop_front() {
                    self.bytes -= entry.weight;
                } else {
                    break;
                }
            }
            self.bytes += weight;
            self.entries.push_back(Entry {
                source,
                bytes: asset.bytes.clone(),
                weight,
                settings: project.settings.clone(),
                font: project.font.clone(),
                scene: scene.clone(),
            });
        }
        Ok(scene)
    }
}
