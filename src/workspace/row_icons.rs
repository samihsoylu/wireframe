// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

#[derive(Clone, Debug)]
pub struct ListRowIcon {
    pub row: usize,
    pub label: String,
    pub icon: Option<IconSource>,
    pub targets: Vec<Id>,
}

impl Editor {
    /// Icon rows include list headers; absent rows never create new list content.
    pub fn list_row_icons(&self, ids: &[Id]) -> Option<Vec<ListRowIcon>> {
        if ids.is_empty() {
            return None;
        }
        let nodes = self.nodes();
        let by_id: HashMap<_, _> = nodes.iter().map(|node| (node.id, node)).collect();
        let context = self.property_context();
        let mut controls = vec![];
        for &id in ids {
            let node = by_id.get(&id)?;
            let NodeContent::Control { kind, .. } = &node.content else {
                return None;
            };
            if kind != "list" {
                return None;
            }
            let values = catalog::get(kind)?.effective(node, &context).ok()?;
            let labels = match values.get("text") {
                Some(Value::Text(text)) => text.split('\n').map(str::to_owned).collect::<Vec<_>>(),
                _ => return None,
            };
            let icons = match values.get("row_icons") {
                Some(Value::Icons(icons)) => icons.clone(),
                _ => vec![],
            };
            controls.push((id, labels, icons));
        }
        Some(
            (0..controls
                .iter()
                .map(|(_, labels, _)| labels.len())
                .max()
                .unwrap_or(0))
                .map(|row| {
                    let label = controls
                        .iter()
                        .map(|(_, labels, _)| labels.get(row).map_or("-", String::as_str))
                        .collect::<Vec<_>>()
                        .join(" / ");
                    ListRowIcon {
                        row,
                        label: crate::rich_text::RichText::parse(&label, false).display_text(false),
                        icon: controls
                            .iter()
                            .filter(|(_, labels, _)| row < labels.len())
                            .find_map(|(_, _, icons)| icons.get(row).cloned().flatten()),
                        targets: controls
                            .iter()
                            .filter(|(_, labels, _)| row < labels.len())
                            .map(|(id, _, _)| *id)
                            .collect(),
                    }
                })
                .collect(),
        )
    }

    pub fn set_list_row_icon(
        &mut self,
        ids: &[Id],
        row: usize,
        icon: Option<IconSource>,
    ) -> Result<(), String> {
        let targets = self
            .list_row_icons(ids)
            .and_then(|rows| rows.into_iter().find(|entry| entry.row == row))
            .ok_or("Select lists containing this row")?
            .targets;
        match &icon {
            Some(IconSource::Catalog(id)) if crate::icons::get(id).is_none() => {
                return Err("Icon no longer exists".into());
            }
            Some(IconSource::Asset(id)) if self.document.live_asset(*id).is_none() => {
                return Err("Image no longer exists".into());
            }
            _ => {}
        }
        let before = self.snapshot();
        for id in targets {
            let (_, values) = self.control_properties(id).ok_or("List no longer exists")?;
            let mut icons = match values.get("row_icons") {
                Some(Value::Icons(icons)) => icons.clone(),
                _ => vec![],
            };
            if icons.get(row).cloned().flatten() == icon {
                continue;
            }
            icons.resize(icons.len().max(row + 1), None);
            icons[row] = icon.clone();
            if let Err(error) = self.set_control_property(id, "row_icons", Value::Icons(icons)) {
                self.restore(before);
                return Err(error);
            }
        }
        Ok(())
    }
}
