// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::images::Prepared;

#[derive(Clone, Copy)]
pub enum ImportChoice {
    KeepBoth,
    Replace,
    UseExisting,
}
impl Editor {
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
        self.editable_image(id)?;
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
            e.set_control_property(id, "image", Value::Asset(asset))?;
            // JSON byte arrays may exceed the file limit before the raw asset limit.
            e.document.encode()?;
            Ok(asset)
        })
    }
}
