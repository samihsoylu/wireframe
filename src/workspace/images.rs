// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::images::Prepared;

#[derive(Clone, Copy)]
pub enum ImageTarget {
    Control(Id),
    Icon(Id),
    ListRow { control: Id, row: usize },
    AppBarIcon { control: Id, index: usize },
}
#[derive(Clone)]
pub enum ImageImportTarget {
    Single(ImageTarget),
    Controls(Vec<Id>),
    AppBarIcons { controls: Vec<Id>, index: usize },
}
impl From<ImageTarget> for ImageImportTarget {
    fn from(target: ImageTarget) -> Self {
        Self::Single(target)
    }
}
#[derive(Clone, Copy)]
pub enum ImportChoice {
    KeepBoth,
    Replace,
    UseExisting,
}
impl Editor {
    /// Splitting adds a sibling, so it is available in source content, not an instance override.
    pub fn image_split_geometry(&self, id: Id) -> Result<(Bounds, Bounds, Option<Crop>), String> {
        self.require_structure_edit()?;
        if self.position_locked(id) {
            return Err("Unlock the image before splitting it".into());
        }
        self.image_geometry(id)
    }

    /// Cut the current visible rectangle horizontally, retaining shared artwork.
    pub fn split_image(&mut self, id: Id, fraction: f32) -> Result<Id, String> {
        if !fraction.is_finite() || fraction <= 0. || fraction >= 1. {
            return Err("Choose a split point inside the image".into());
        }
        let (full, _, crop) = self.image_split_geometry(id)?;
        let crop = crop.unwrap_or(Crop::ALL);
        let before = self
            .nodes()
            .iter()
            .find(|n| n.id == id)
            .cloned()
            .ok_or("Image no longer exists")?;
        if self.pending.is_some() {
            let mut check = Editor::new(self.document.clone());
            check.page = self.page;
            check.active_versions = self.active_versions.clone();
            check.definition = self.definition;
            check.library = self.library;
            check.context = self.context.clone();
            check.split_image(id, fraction)?;
        }
        self.component_change("Split image", |e| {
            let new_id = e.document.allocate_id()?;
            let mut upper = before.clone();
            let mut lower = before.clone();
            let height = crop.height * fraction;
            upper.crop = Some(Crop { height, ..crop });
            lower.crop = Some(Crop {
                y: crop.y + height,
                height: crop.height - height,
                ..crop
            });
            lower.id = new_id;
            // The lower crop's offset already accounts for the upper piece.
            // Move the uncropped artwork only by the visible gap.
            lower.origin.y += 20.;
            for node in [&mut upper, &mut lower] {
                if let NodeContent::Control { size, .. } = &mut node.content {
                    *size = Size::fixed(full.w, full.h);
                }
            }
            let nodes = e.nodes_mut();
            let index = nodes
                .iter()
                .position(|n| n.id == id)
                .ok_or("Image no longer exists")?;
            nodes[index] = upper;
            nodes.insert(index + 1, lower);
            let mut path = e.context.clone();
            path.push(id);
            let order = &mut e.active_content_mut().paint_order;
            if !order.is_empty() {
                let index = order
                    .iter()
                    .position(|p| p == &path)
                    .ok_or("Image layer no longer exists")?;
                *path.last_mut().unwrap() = new_id;
                order.insert(index + 1, path);
            }
            e.layout_page(e.surface_id())?;
            e.selection = HashSet::from([new_id]);
            Ok(new_id)
        })
    }

    /// Full and visible rectangles in workspace coordinates, plus the effective crop.
    pub fn image_geometry(&self, id: Id) -> Result<(Bounds, Bounds, Option<Crop>), String> {
        self.editable_image_source(id)?;
        if self
            .control_properties(id)
            .is_none_or(|(spec, _)| spec.id != "image")
        {
            return Err("Cropping requires an image control".into());
        }
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
    fn write_image_node(&mut self, before: &Node, mut after: Node) -> Result<(), String> {
        if self.position_locked(before.id) {
            after.origin = before.origin;
        }
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
        if self.own_locked(id) {
            return Err("Unlock the image before cropping it".into());
        }
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
    pub fn selected_image_ids(&self) -> Option<Vec<Id>> {
        let ids = self.ordered_selection();
        self.validate_image_targets(&ids).ok().map(|()| ids)
    }
    fn validate_image_targets(&self, ids: &[Id]) -> Result<(), String> {
        if ids.is_empty() {
            return Err("Select image controls".into());
        }
        let mut seen = HashSet::new();
        for &id in ids {
            if !seen.insert(id) {
                return Err("Image targets must be unique".into());
            }
            self.editable_image_source(id)?;
        }
        Ok(())
    }
    pub fn rotate_image(&mut self, id: Id) -> Result<(), String> {
        self.rotate_images(&[id])
    }
    pub fn rotate_images(&mut self, ids: &[Id]) -> Result<(), String> {
        self.validate_image_targets(ids)?;
        self.component_change("Rotate image", |e| {
            for &id in ids {
                e.rotate_image_value(id)?;
            }
            Ok(())
        })
    }
    pub(super) fn rotate_image_value(&mut self, id: Id) -> Result<(), String> {
        self.editable_image_source(id)?;
        if let Some((spec, props)) = self.control_properties(id)
            && spec.id == "cover_flow"
        {
            let turns = (crate::images::quarter_turns(&props) + 3) % 4;
            return self.set_control_property(
                id,
                "rotation",
                Value::Number(f64::from(turns) * 90.),
            );
        }
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
        self.write_image_node(&before, after)
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
                .live_asset(*id)
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
    fn editable_image_source(&self, id: Id) -> Result<(), String> {
        if self
            .control_properties(id)
            .is_none_or(|(spec, _)| !matches!(spec.id, "image" | "cover_flow"))
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
            .live_assets()
            .find(|a| {
                a.name.eq_ignore_ascii_case(&image.name) || stem(&a.name) == stem(&image.name)
            })
            .map(|a| a.id)
    }
    pub fn assign_image(&mut self, id: Id, asset: Option<Id>) -> Result<(), String> {
        self.assign_images(&[id], asset)
    }
    pub fn assign_images(&mut self, ids: &[Id], asset: Option<Id>) -> Result<(), String> {
        self.validate_image_targets(ids)?;
        if asset.is_some_and(|id| self.document.live_asset(id).is_none()) {
            return Err("Image resource no longer exists".into());
        }
        let value = asset.map_or(Value::Null, Value::Asset);
        self.component_change("Change image", |e| {
            for &id in ids {
                if e.control_properties(id)
                    .is_some_and(|(_, p)| p.get("image") == Some(&value))
                {
                    continue;
                }
                e.set_control_property(id, "image", value.clone())?;
            }
            Ok(())
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
        target: impl Into<ImageImportTarget>,
        image: Prepared,
        choice: ImportChoice,
    ) -> Result<Id, String> {
        self.import_image_options(target.into(), image, choice, None)
    }
    pub fn import_web_image(
        &mut self,
        ids: Vec<Id>,
        image: Prepared,
        choice: ImportChoice,
        sketch: bool,
    ) -> Result<Id, String> {
        self.validate_image_targets(&ids)?;
        self.import_image_options(
            ImageImportTarget::Controls(ids),
            image,
            choice,
            Some(sketch),
        )
    }
    fn import_image_options(
        &mut self,
        target: ImageImportTarget,
        image: Prepared,
        choice: ImportChoice,
        sketch: Option<bool>,
    ) -> Result<Id, String> {
        match target.clone() {
            ImageImportTarget::Single(ImageTarget::Control(id)) => {
                self.editable_image_source(id)?
            }
            ImageImportTarget::Controls(ids) => self.validate_image_targets(&ids)?,
            ImageImportTarget::AppBarIcons { controls, index } => {
                let slots = self
                    .app_bar_icon_slots(&controls)
                    .ok_or("Select app bars")?;
                if index > slots.len() {
                    return Err("Icon position no longer exists".into());
                }
            }
            ImageImportTarget::Single(ImageTarget::Icon(id)) => {
                let (spec, _) = self
                    .control_properties(id)
                    .ok_or("Control no longer exists")?;
                if !matches!(
                    spec.id,
                    "button"
                        | "dropdown"
                        | "icon"
                        | "icon_label"
                        | "circle_button"
                        | "pointed_button"
                        | "multiline_button"
                ) || self
                    .current()
                    .controls
                    .iter()
                    .find(|c| c.id == id)
                    .is_none()
                {
                    return Err("Select a control with an icon".into());
                }
            }
            ImageImportTarget::Single(ImageTarget::AppBarIcon { control, index }) => {
                let (spec, props) = self
                    .control_properties(control)
                    .ok_or("App bar no longer exists")?;
                let count = match props.get("icons") {
                    Some(Value::Icons(v)) => v.len(),
                    _ => 0,
                };
                if spec.id != "app_bar"
                    || index > count
                    || self.nodes().iter().find(|n| n.id == control).is_none()
                {
                    return Err("Select a app bar and an existing icon position".into());
                }
            }
            ImageImportTarget::Single(ImageTarget::ListRow { control, row }) => {
                let (spec, props) = self
                    .control_properties(control)
                    .ok_or("List no longer exists")?;
                if spec.id != "list"
                    || self
                        .current()
                        .controls
                        .iter()
                        .find(|c| c.id == control)
                        .is_none()
                {
                    return Err("Select a List".into());
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
                    trashed: false,
                    id,
                    name,
                    media_type: image.media_type,
                    bytes: image.bytes,
                });
                id
            };
            match target {
                ImageImportTarget::AppBarIcons { controls, index } => {
                    e.set_app_bar_icon(&controls, index, Some(IconSource::Asset(asset)))?;
                }
                ImageImportTarget::Controls(ids) => {
                    for id in ids {
                        if let Some(sketch) = sketch {
                            e.set_control_property(id, "sketch_filter", Value::Bool(sketch))?;
                        }
                        if e.control_properties(id)
                            .is_some_and(|(_, p)| p.get("image") == Some(&Value::Asset(asset)))
                        {
                            continue;
                        }
                        e.set_control_property(id, "image", Value::Asset(asset))?;
                    }
                }
                ImageImportTarget::Single(ImageTarget::Icon(id)) => {
                    e.set_control_property(id, "icon", Value::Icon(IconSource::Asset(asset)))?;
                }
                ImageImportTarget::Single(ImageTarget::Control(id)) => {
                    e.set_control_property(id, "image", Value::Asset(asset))?
                }
                ImageImportTarget::Single(ImageTarget::AppBarIcon { control, index }) => {
                    let (_, props) = e
                        .control_properties(control)
                        .ok_or("App bar no longer exists")?;
                    let mut icons = match props.get("icons") {
                        Some(Value::Icons(v)) => v.clone(),
                        _ => vec![],
                    };
                    if index == icons.len() {
                        icons.push(Some(IconSource::Asset(asset)));
                    } else {
                        icons[index] = Some(IconSource::Asset(asset));
                    }
                    e.set_control_property(control, "icons", Value::Icons(icons))?;
                }
                ImageImportTarget::Single(ImageTarget::ListRow { control, row }) => {
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
