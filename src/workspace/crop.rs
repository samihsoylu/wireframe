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
        if node.locked {
            return Err("Unlock the group before cropping it".into());
        }
        let rect = |b: crate::layout::Rect| Bounds::new(b.x, b.y, b.width, b.height);
        if n.uncropped.width <= 0. || n.uncropped.height <= 0. {
            return Err("The group has no visible content to crop".into());
        }
        Ok((rect(n.uncropped), rect(n.bounds), resolved.crop))
    }

    /// Crop UI geometry follows the visible group frame. Existing crop fractions
    /// are inverted from that frame; canonical document geometry stays intact.
    pub fn group_crop_edit_geometry(
        &self,
        id: Id,
    ) -> Result<(Bounds, Bounds, Option<Crop>), String> {
        let geometry = self.group_crop_view_geometry(id)?;
        if !self.markup_visible && (geometry.1.w <= 0. || geometry.1.h <= 0.) {
            return Err("The group has no visible content to crop".into());
        }
        Ok(geometry)
    }

    // Removal must also work when the current mask clips every plain child.
    fn group_crop_view_geometry(&self, id: Id) -> Result<(Bounds, Bounds, Option<Crop>), String> {
        let geometry = self.group_crop_geometry(id)?;
        if self.markup_visible {
            return Ok(geometry);
        }
        let mut path = self.context.clone();
        path.push(id);
        let layout = self.layout_page(self.surface_id())?;
        let b = view::find_layout(&layout.nodes, &path)
            .ok_or("Group no longer exists")?
            .without_markup;
        let visible = Bounds::new(b.x, b.y, b.width, b.height);
        let full = geometry.2.map_or(visible, |c| {
            Bounds::new(
                (b.x - c.x * b.width / c.width).round(),
                (b.y - c.y * b.height / c.height).round(),
                (b.width / c.width).round(),
                (b.height / c.height).round(),
            )
        });
        Ok((full, visible, geometry.2))
    }

    /// Apply crop fractions to the complete group while keeping the preview's
    /// chosen corner. Hidden descendants participate in the saved mask.
    pub fn crop_group_at(&mut self, id: Id, crop: Crop, origin: Point) -> Result<(), String> {
        let (full, _, _) = self.group_crop_geometry(id)?;
        let parent = self.context_origin();
        let translation = Point::new(
            origin.x - parent.x - (full.x - parent.x + crop.x * full.w).round(),
            origin.y - parent.y - (full.y - parent.y + crop.y * full.h).round(),
        );
        self.crop_group(id, Some(crop), translation)
    }

    /// A disposable scene of the target, preserving descendant clips, paint
    /// order and editor markup visibility while removing the target's crop.
    pub fn group_crop_preview(&self, id: Id) -> Result<crate::render::Scene, String> {
        self.group_crop_geometry(id)?;
        self.crop_preview(id)
    }

    pub fn image_split_preview(&self, id: Id) -> Result<crate::render::Scene, String> {
        self.image_split_geometry(id)?;
        self.crop_preview(id)
    }

    fn crop_preview(&self, id: Id) -> Result<crate::render::Scene, String> {
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
        preview.markup_visible = self.markup_visible;
        preview.export_scene(preview.page, &SceneScope::Whole)
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
        let (full, visible, existing_crop) = self.group_crop_geometry(id)?;
        let reset = if crop.is_none() && existing_crop.is_some() && !self.markup_visible {
            self.group_crop_view_geometry(id)?.0
        } else {
            visible
        };
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
                        (reset.x - parent.x).max(0.) + parent.x - full.x,
                        (reset.y - parent.y).max(0.) + parent.y - full.y,
                    )
                } else {
                    translation
                };
                after.origin = after.origin.translated(delta);
                if e.editing_instance() {
                    e.write_local_node(id, &before, &after)?;
                } else if matches!(before.content, NodeContent::Instance { .. })
                    && e.owning_source(id).is_none()
                {
                    let mut path = e.context.clone();
                    path.push(id);
                    e.change_patch(&path, &[], |patch| {
                        patch.crop =
                            Some(after.crop.map_or(CropOverride::Clear, CropOverride::Region));
                    })?;
                    let node = e.nodes_mut().iter_mut().find(|n| n.id == id).unwrap();
                    node.crop = None;
                    node.origin = after.origin;
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
