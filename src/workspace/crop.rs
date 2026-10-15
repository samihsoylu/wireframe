// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Editor {
    /// Keep all other artwork, ancestor masks and layer order, without a stale copy of the target.
    pub fn crop_background(&self, id: Id) -> Result<crate::render::Scene, String> {
        let mut path = self.context.clone();
        path.push(id);
        self.rendered_scene_excluding(self.surface_id(), Some(&path))
    }

    /// Group geometry includes descendant masks, but excludes ancestor masks.
    pub fn group_crop_geometry(&self, id: Id) -> Result<(Bounds, Bounds, Option<Crop>), String> {
        let node = self
            .nodes()
            .iter()
            .find(|n| n.id == id)
            .cloned()
            .ok_or("Select a group")?;
        let resolved = self.document.resolve(&node)?.root;
        if !matches!(&resolved.content, NodeContent::Group { children } if !children.is_empty()) {
            return Err("Select a nonempty group or component".into());
        }
        let mut path = self.context.clone();
        path.push(id);
        let layout = self.layout_page(self.surface_id())?;
        let n = view::find_layout(&layout.nodes, &path).ok_or("Group no longer exists")?;
        if n.locked {
            return Err("Unlock the group before cropping it".into());
        }
        let rect = |b: crate::layout::Rect| Bounds::new(b.x, b.y, b.width, b.height);
        if n.uncropped.width <= 0. || n.uncropped.height <= 0. {
            return Err("The group has no visible content to crop".into());
        }
        Ok((rect(n.uncropped), rect(n.bounds), resolved.crop))
    }

    /// A disposable scene of the complete target, preserving descendant clips and paint order.
    pub fn group_crop_preview(&self, id: Id) -> Result<crate::render::Scene, String> {
        self.group_crop_geometry(id)?;
        let node = self
            .nodes()
            .iter()
            .find(|n| n.id == id)
            .cloned()
            .ok_or("Group no longer exists")?;
        let mut root = self.document.resolve(&node)?.root;
        root.crop = None;
        root.origin = root.origin.translated(self.context_origin());
        let mut path = self.context.clone();
        path.push(id);
        let order = self
            .layout_page(self.surface_id())?
            .leaves()
            .into_iter()
            .filter(|(n, _)| n.path.starts_with(&path))
            .map(|(n, _)| n.path[self.context.len()..].to_vec())
            .collect();
        let mut document = self.document.clone();
        document.pages[0].name = self
            .surface(self.surface_id())
            .ok_or("Workspace no longer exists")?
            .0
            .into();
        document.pages[0].content = Content {
            nodes: vec![root],
            paint_order: order,
        };
        // The preview is never saved; other surfaces remain available for linked content.
        let mut preview = Editor::with_clock(document, self.clock);
        preview.date = self.date;
        preview.rendered_scene(preview.page)
    }

    /// Translate the underlying artwork and change its mask as one history entry.
    pub fn crop_group(
        &mut self,
        id: Id,
        crop: Option<Crop>,
        translation: Point,
    ) -> Result<(), String> {
        if crop.is_some_and(|c| !c.valid())
            || !translation.x.is_finite()
            || !translation.y.is_finite()
        {
            return Err("Choose a crop inside the group".into());
        }
        let (full, visible, _) = self.group_crop_geometry(id)?;
        let before = self
            .nodes()
            .iter()
            .find(|n| n.id == id)
            .cloned()
            .ok_or("Group no longer exists")?;
        let parent = self.context_origin();
        self.component_change(
            if crop.is_some() {
                "Crop group"
            } else {
                "Remove group crop"
            },
            |e| {
                let mut after = before.clone();
                after.crop = crop.filter(|c| *c != Crop::ALL);
                let delta = if crop.is_none() {
                    // A group's origin need not be the minimum of its children's bounds.
                    Point::new(
                        (visible.x - parent.x).max(0.) + parent.x - full.x,
                        (visible.y - parent.y).max(0.) + parent.y - full.y,
                    )
                } else {
                    translation
                };
                after.origin = after.origin.translated(delta);
                if e.editing_instance() {
                    e.write_local_node(id, &before, &after)?;
                } else {
                    *e.nodes_mut()
                        .iter_mut()
                        .find(|n| n.id == id)
                        .ok_or("Group no longer exists")? = after;
                }
                e.layout_page(e.surface_id())?;
                Ok(())
            },
        )
    }
}
