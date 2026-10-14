// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::images::Prepared;

#[derive(Clone, Copy)]
pub enum ImageTarget {
    Control(Id),
    Icon(Id),
    ListRow { control: Id, row: usize },
}
#[derive(Clone, Copy)]
pub enum ImportChoice {
    KeepBoth,
    Replace,
    UseExisting,
}
impl Editor {
    /// Full and visible rectangles in workspace coordinates, plus the effective crop.
    pub fn image_geometry(&self, id: Id) -> Result<(Bounds, Bounds, Option<Crop>), String> {
        self.editable_image(id)?;
        let mut path = self.context.clone();
        path.push(id);
        let layout = self.layout_page(self.surface_id())?;
        let n = view::find_layout(&layout.nodes, &path).ok_or("Image no longer exists")?;
        let rect = |b: crate::layout::Rect| Bounds::new(b.x, b.y, b.width, b.height);
        let crop = self
            .nodes()
            .iter()
            .find(|n| n.id == id)
            .and_then(|n| n.crop);
        Ok((rect(n.uncropped), rect(n.bounds), crop))
    }
    fn write_image_node(&mut self, before: &Node, after: Node) -> Result<(), String> {
        if self.editing_instance() {
            self.write_local_node(before.id, before, &after)?;
        } else {
            *self
                .nodes_mut()
                .iter_mut()
                .find(|n| n.id == before.id)
                .ok_or("Image no longer exists")? = after;
        }
        self.layout_page(self.surface_id())?;
        self.document.validate()?;
        Ok(())
    }
    pub fn crop_image(&mut self, id: Id, crop: Option<Crop>) -> Result<(), String> {
        if crop.is_some_and(|c| !c.valid()) {
            return Err("Choose a crop inside the image".into());
        }
        let (full, visible, _) = self.image_geometry(id)?;
        let before = self
            .nodes()
            .iter()
            .find(|n| n.id == id)
            .cloned()
            .ok_or("Image no longer exists")?;
        let parent = self.context_origin();
        self.component_change(
            if crop.is_some() {
                "Crop image"
            } else {
                "Remove image crop"
            },
            |e| {
                let mut after = before.clone();
                after.crop = crop.filter(|c| *c != Crop::ALL);
                if crop.is_none() {
                    after.origin = Point {
                        x: visible.x - parent.x,
                        y: visible.y - parent.y,
                    };
                }
                if let NodeContent::Control { size, .. } = &mut after.content {
                    *size = Size::fixed(full.w, full.h);
                }
                e.write_image_node(&before, after)
            },
        )
    }
    pub fn rotate_image(&mut self, id: Id) -> Result<(), String> {
        let (full, visible, crop) = self.image_geometry(id)?;
        let before = self
            .nodes()
            .iter()
            .find(|n| n.id == id)
            .cloned()
            .ok_or("Image no longer exists")?;
        let (_, props) = self.control_properties(id).ok_or("Select an image")?;
        let turns = (crate::images::quarter_turns(&props) + 3) % 4;
        let parent = self.context_origin();
        self.component_change("Rotate image", |e| {
            let mut after = before.clone();
            let crop = crop.map(|c| Crop {
                x: c.y,
                y: (1. - c.x - c.width).max(0.),
                width: c.height,
                height: c.width,
            });
            let c = crop.unwrap_or(Crop::ALL);
            let dx = (visible.w / 2.).round();
            let dy = (visible.h / 2.).round();
            after.origin = Point {
                x: visible.x + dx - dy - c.x * full.h - parent.x,
                y: visible.y + dy - (visible.w - dx) - c.y * full.w - parent.y,
            };
            after.crop = crop;
            after
                .properties
                .insert("rotation".into(), Value::Number(f64::from(turns) * 90.));
            if let NodeContent::Control { size, .. } = &mut after.content {
                *size = Size::fixed(full.h, full.w);
            }
            e.write_image_node(&before, after)
        })
    }
    pub(super) fn auto_size_image(&mut self, id: Id) -> Result<(), String> {
        let (_, visible, crop) = self.image_geometry(id)?;
        let before = self
            .nodes()
            .iter()
            .find(|n| n.id == id)
            .cloned()
            .ok_or("Image no longer exists")?;
        let (_, props) = self.control_properties(id).ok_or("Select an image")?;
        let dims = match props.get("image") {
            Some(Value::Asset(id)) => self
                .document
                .assets
                .iter()
                .find(|a| a.id == *id)
                .and_then(|a| crate::images::dimensions(&a.bytes).ok()),
            _ => None,
        }
        .unwrap_or((77, 79));
        let quarter = crate::images::quarter_turns(&props) % 2 == 1;
        let (w, h) = if quarter {
            (dims.1 as f32, dims.0 as f32)
        } else {
            (dims.0 as f32, dims.1 as f32)
        };
        let c = crop.unwrap_or(Crop::ALL);
        let (w, h) = if quarter {
            (w / c.width, h / c.height)
        } else {
            (w, h)
        };
        let parent = self.context_origin();
        let mut after = before.clone();
        after.origin = Point {
            x: visible.x - c.x * w - parent.x,
            y: visible.y - c.y * h - parent.y,
        };
        if let NodeContent::Control { size, .. } = &mut after.content {
            *size = if quarter && crop.is_some() {
                Size::fixed(w, h)
            } else {
                Size::default()
            };
        }
        self.write_image_node(&before, after)
    }
    fn editable_image(&self, id: Id) -> Result<(), String> {
        let c = self
            .current()
            .controls
            .into_iter()
            .find(|c| c.id == id)
            .ok_or("Select an image")?;
        if c.locked {
            return Err("Unlock the image before changing its source".into());
        }
        if self
            .control_properties(id)
            .is_none_or(|(spec, _)| spec.id != "image")
        {
            return Err("Select an image control".into());
        }
        Ok(())
    }
    pub fn image_conflict(&self, image: &Prepared) -> Option<Id> {
        let stem = |name: &str| {
            std::path::Path::new(name)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
        };
        self.document
            .assets
            .iter()
            .find(|a| {
                a.name.eq_ignore_ascii_case(&image.name) || stem(&a.name) == stem(&image.name)
            })
            .map(|a| a.id)
    }
    pub fn assign_image(&mut self, id: Id, asset: Option<Id>) -> Result<(), String> {
        self.editable_image(id)?;
        if asset.is_some_and(|id| !self.document.assets.iter().any(|a| a.id == id)) {
            return Err("Image resource no longer exists".into());
        }
        self.component_change("Change image", |e| {
            e.set_control_property(id, "image", asset.map_or(Value::Null, Value::Asset))
        })
    }
    pub fn import_image(
        &mut self,
        id: Id,
        image: Prepared,
        choice: ImportChoice,
    ) -> Result<Id, String> {
        self.import_image_at(ImageTarget::Control(id), image, choice)
    }
    pub fn import_image_at(
        &mut self,
        target: ImageTarget,
        image: Prepared,
        choice: ImportChoice,
    ) -> Result<Id, String> {
        match target {
            ImageTarget::Control(id) => self.editable_image(id)?,
            ImageTarget::Icon(id) => {
                let (spec, _) = self
                    .control_properties(id)
                    .ok_or("Control no longer exists")?;
                if !matches!(spec.id, "button" | "dropdown" | "icon" | "icon_label")
                    || self
                        .current()
                        .controls
                        .iter()
                        .find(|c| c.id == id)
                        .is_none_or(|c| c.locked)
                {
                    return Err("Select an unlocked control with an icon".into());
                }
            }
            ImageTarget::ListRow { control, row } => {
                let (spec, props) = self
                    .control_properties(control)
                    .ok_or("List no longer exists")?;
                if spec.id != "list"
                    || self
                        .current()
                        .controls
                        .iter()
                        .find(|c| c.id == control)
                        .is_none_or(|c| c.locked)
                {
                    return Err("Select an unlocked List".into());
                }
                let rows = match props.get("text") {
                    Some(Value::Text(text)) => text.split('\n').count(),
                    _ => 0,
                };
                if row >= rows {
                    return Err("List row no longer exists".into());
                }
            }
        }
        crate::images::decoded(&image.bytes)?;
        self.component_change("Import image", |e| {
            let conflict = e.image_conflict(&image);
            let same = conflict.filter(|id| {
                e.document
                    .assets
                    .iter()
                    .any(|a| a.id == *id && a.bytes == image.bytes)
            });
            let asset = if let Some(id) = same {
                id
            } else if let Some(id) = conflict
                .filter(|_| matches!(choice, ImportChoice::UseExisting | ImportChoice::Replace))
            {
                if matches!(choice, ImportChoice::Replace) {
                    let a = e.document.assets.iter_mut().find(|a| a.id == id).unwrap();
                    a.bytes = image.bytes;
                    a.media_type = image.media_type;
                }
                id
            } else {
                let id = e.document.allocate_id()?;
                let mut name = image.name.clone();
                let mut suffix = 2;
                while e
                    .document
                    .assets
                    .iter()
                    .any(|a| a.name.eq_ignore_ascii_case(&name))
                {
                    name = format!("{} ({suffix})", image.name);
                    suffix += 1;
                }
                e.document.assets.push(Asset {
                    id,
                    name,
                    media_type: image.media_type,
                    bytes: image.bytes,
                });
                id
            };
            match target {
                ImageTarget::Icon(id) => {
                    e.set_control_property(id, "icon", Value::Icon(IconSource::Asset(asset)))?;
                }
                ImageTarget::Control(id) => {
                    e.set_control_property(id, "image", Value::Asset(asset))?
                }
                ImageTarget::ListRow { control, row } => {
                    let (_, props) = e
                        .control_properties(control)
                        .ok_or("List no longer exists")?;
                    let mut icons = match props.get("row_icons") {
                        Some(Value::Icons(v)) => v.clone(),
                        _ => vec![],
                    };
                    if icons.len() <= row {
                        icons.resize(row + 1, None);
                    }
                    icons[row] = Some(IconSource::Asset(asset));
                    e.set_control_property(control, "row_icons", Value::Icons(icons))?;
                }
            }
            // JSON byte arrays may exceed the file limit before the raw asset limit.
            e.document.encode()?;
            Ok(asset)
        })
    }
}
