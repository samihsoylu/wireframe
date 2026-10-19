// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

pub struct AppBarIconSlot {
    pub index: usize,
    pub label: String,
    pub icon: Option<IconSource>,
}

impl Editor {
    fn app_bar_collections(&self, ids: &[Id]) -> Option<Vec<Vec<Option<IconSource>>>> {
        if ids.is_empty() {
            return None;
        }
        ids.iter()
            .map(|id| {
                let (spec, values) = self.control_properties(*id)?;
                if spec.id != "app_bar" {
                    return None;
                }
                Some(match values.get("icons") {
                    Some(Value::Icons(icons)) => icons.clone(),
                    _ => vec![],
                })
            })
            .collect()
    }

    /// Selection order determines the displayed icon; labels retain missing slots.
    pub fn app_bar_icon_slots(&self, ids: &[Id]) -> Option<Vec<AppBarIconSlot>> {
        let collections = self.app_bar_collections(ids)?;
        Some(
            (0..collections.iter().map(Vec::len).max().unwrap_or(0))
                .map(|index| {
                    let label = collections
                        .iter()
                        .map(|icons| match icons.get(index).and_then(Option::as_ref) {
                            Some(IconSource::Catalog(id)) => crate::icons::get(id)
                                .map_or("Unavailable icon".into(), |icon| icon.label.clone()),
                            Some(IconSource::Asset(id)) => self
                                .document
                                .live_asset(*id)
                                .map_or("Unavailable image".into(), |asset| asset.name.clone()),
                            None => "No Icon".into(),
                        })
                        .collect::<Vec<_>>()
                        .join(" / ");
                    AppBarIconSlot {
                        index,
                        label,
                        icon: collections
                            .iter()
                            .find_map(|icons| icons.get(index).cloned().flatten()),
                    }
                })
                .collect(),
        )
    }

    fn validate_app_bar_icon(&self, icon: &Option<IconSource>) -> Result<(), String> {
        match icon {
            Some(IconSource::Catalog(id)) if crate::icons::get(id).is_none() => {
                Err("Icon no longer exists".into())
            }
            Some(IconSource::Asset(id)) if self.document.live_asset(*id).is_none() => {
                Err("Image no longer exists".into())
            }
            _ => Ok(()),
        }
    }

    fn write_app_bar_collections(
        &mut self,
        ids: &[Id],
        collections: Vec<Vec<Option<IconSource>>>,
    ) -> Result<(), String> {
        let before = self.snapshot();
        for (&id, icons) in ids.iter().zip(collections) {
            if self
                .control_properties(id)
                .is_some_and(|(_, p)| p.get("icons") == Some(&Value::Icons(icons.clone())))
            {
                continue;
            }
            if let Err(error) = self.set_control_property(id, "icons", Value::Icons(icons)) {
                self.restore(before);
                return Err(error);
            }
        }
        Ok(())
    }

    /// Compact edits retain every other slot. Missing replacement slots append.
    pub fn set_app_bar_icon(
        &mut self,
        ids: &[Id],
        index: usize,
        icon: Option<IconSource>,
    ) -> Result<(), String> {
        let mut collections = self.app_bar_collections(ids).ok_or("Select app bars")?;
        if index > collections.iter().map(Vec::len).max().unwrap_or(0) {
            return Err("Icon position no longer exists".into());
        }
        self.validate_app_bar_icon(&icon)?;
        for icons in &mut collections {
            if let Some(icon) = &icon {
                if index < icons.len() {
                    icons[index] = Some(icon.clone());
                } else {
                    icons.push(Some(icon.clone()));
                }
            } else if index < icons.len() {
                icons.remove(index);
            }
        }
        self.write_app_bar_collections(ids, collections)
    }

    /// Full-dialog edits share the aggregate collection and retain destination roles.
    pub fn replace_app_bar_icons(
        &mut self,
        ids: &[Id],
        icons: Vec<Option<IconSource>>,
    ) -> Result<(), String> {
        self.app_bar_collections(ids).ok_or("Select app bars")?;
        for icon in &icons {
            self.validate_app_bar_icon(icon)?;
        }
        let icons: Vec<_> = icons.into_iter().flatten().map(Some).collect();
        self.write_app_bar_collections(ids, vec![icons; ids.len()])
    }
}
