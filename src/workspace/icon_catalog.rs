// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Individual catalog choices save ordinary icon controls.
use super::*;
use std::sync::LazyLock;

pub(super) fn entries() -> &'static [CatalogEntry] {
    static ENTRIES: LazyLock<Vec<CatalogEntry>> = LazyLock::new(|| {
        let mut entries: Vec<_> = crate::icons::all()
            .iter()
            .map(|icon| CatalogEntry {
                item: CatalogItem::Icon(&icon.id),
                name: icon.catalog_name(),
                enabled: true,
            })
            .collect();
        entries.sort_by_cached_key(|entry| entry.name.to_lowercase());
        entries
    });
    &ENTRIES
}

impl Editor {
    pub fn insert_catalog_icon(&mut self, source: &str, x: f32, y: f32) -> Result<Id, String> {
        self.require_structure_edit()?;
        crate::icons::get(source).ok_or("Icon no longer exists")?;
        if !x.is_finite() || !y.is_finite() {
            return Err("Choose a finite icon position".into());
        }
        if self.pending.is_some() {
            let mut check = Editor::new(self.document.clone());
            check.page = self.page;
            check.active_versions = self.active_versions.clone();
            check.definition = self.definition;
            check.library = self.library;
            check.context = self.context.clone();
            check.insert_catalog_icon(source, x, y)?;
        }
        self.component_change("Add icon", |editor| {
            let id = editor.document.allocate_id()?;
            let parent = editor.context_origin();
            let mut node = catalog::get("icon")
                .unwrap()
                .create(id, Point::new(x - parent.x, y - parent.y));
            node.properties.insert(
                "icon".into(),
                Value::Icon(IconSource::Catalog(source.into())),
            );
            node.properties
                .insert("icon_size".into(), Value::Choice("24".into()));
            editor.nodes_mut().push(node);
            editor.append_order(id);
            editor.selection = HashSet::from([id]);
            Ok(id)
        })
    }
}
