// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl WireframeApp {
    fn active_resource_row(&self) -> Option<ResourceRow> {
        self.editor
            .definition
            .map(ResourceRow::Source)
            .or_else(|| self.editor.library.map(ResourceRow::Library))
    }
    fn resource_rows(&self) -> Vec<ResourceRow> {
        self.editor
            .document
            .live_libraries()
            .flat_map(|l| {
                std::iter::once(ResourceRow::Library(l.id))
                    .chain(l.sources.iter().map(|s| ResourceRow::Source(s.definition)))
            })
            .collect()
    }
    pub(in crate::app) fn collapse_resource_selection(&mut self) {
        let active = self.active_resource_row();
        self.source_navigation.selected = active.into_iter().collect();
        self.source_navigation.anchor = active;
        self.source_navigation.surface = active;
    }
    pub(super) fn sync_resource_selection(&mut self) {
        let rows = self.resource_rows();
        let active = self.active_resource_row();
        if self.source_navigation.surface != active {
            self.collapse_resource_selection();
        }
        self.source_navigation
            .selected
            .retain(|row| rows.contains(row));
        if self
            .source_navigation
            .anchor
            .is_some_and(|row| !rows.contains(&row))
        {
            self.source_navigation.anchor = active;
        }
    }
    pub(super) fn resource_selection(&self, target: ResourceRow) -> Vec<ResourceRow> {
        let rows = self.resource_rows();
        if !rows.contains(&target) {
            return vec![];
        }
        if self.source_navigation.selected.contains(&target)
            && self.source_navigation.surface == self.active_resource_row()
        {
            rows.into_iter()
                .filter(|row| self.source_navigation.selected.contains(row))
                .collect()
        } else {
            vec![target]
        }
    }
    /// Library context removal expands only an all-heading selection. A mixed
    /// selection still removes the explicit context library, not source owners.
    pub(in crate::app) fn library_trash_selection(&self, library: Id) -> Vec<Id> {
        let rows = self.resource_selection(ResourceRow::Library(library));
        if rows
            .iter()
            .all(|row| matches!(row, ResourceRow::Library(_)))
        {
            rows.into_iter()
                .filter_map(|row| match row {
                    ResourceRow::Library(id) => Some(id),
                    ResourceRow::Source(_) => None,
                })
                .collect()
        } else {
            vec![library]
        }
    }
    /// Whole libraries are requested when any heading or sources from several
    /// owners are selected. Document order deduplicates mixed parent/child rows.
    pub(super) fn selected_library_ids(&self, rows: &[ResourceRow]) -> Option<Vec<Id>> {
        let owners: HashSet<_> = rows
            .iter()
            .filter_map(|row| match row {
                ResourceRow::Library(id) => Some(*id),
                ResourceRow::Source(id) => self
                    .editor
                    .document
                    .component_source(*id)
                    .map(|(l, _)| l.id),
            })
            .collect();
        if owners.len() > 1
            || rows
                .iter()
                .any(|row| matches!(row, ResourceRow::Library(_)))
        {
            Some(
                self.editor
                    .document
                    .libraries
                    .iter()
                    .filter(|l| owners.contains(&l.id))
                    .map(|l| l.id)
                    .collect(),
            )
        } else {
            None
        }
    }
    pub(in crate::app) fn copy_selected_resource(
        &mut self,
        ctx: &egui::Context,
        target: ResourceRow,
        cut: bool,
    ) -> Result<(), String> {
        let rows = self.resource_selection(target);
        if rows.is_empty() {
            return Err("Component resource no longer exists".into());
        }
        if let Some(libraries) = self.selected_library_ids(&rows) {
            if cut {
                let copy = self
                    .editor
                    .copy_libraries(&libraries, &self.clipboard_origin())?;
                return self.request_cut(ctx, copy, clipboard::CutScope::Libraries(libraries));
            }
            self.copy_library_resources(ctx, &libraries)
        } else {
            let sources: Vec<_> = rows.into_iter().filter_map(ResourceRow::source).collect();
            self.copy_source_resources(ctx, &sources, cut)
        }
    }
    pub(in crate::app) fn source_resource_selection(&self, source: Id) -> Vec<Id> {
        self.resource_selection(ResourceRow::Source(source))
            .into_iter()
            .filter_map(ResourceRow::source)
            .collect()
    }
    pub(super) fn select_source_row(
        &mut self,
        source: Id,
        modifiers: egui::Modifiers,
    ) -> Result<(), String> {
        self.select_resource_row(ResourceRow::Source(source), modifiers)
    }
    pub(super) fn select_resource_row(
        &mut self,
        row: ResourceRow,
        modifiers: egui::Modifiers,
    ) -> Result<(), String> {
        let all = self.resource_rows();
        if !all.contains(&row) {
            return Err("Component resource no longer exists".into());
        }
        self.sync_resource_selection();
        let mut order: Vec<_> = self
            .source_navigation
            .visible
            .iter()
            .copied()
            .filter(|r| all.contains(r))
            .collect();
        if !order.contains(&row) {
            order = all.clone();
        }
        let previous = self.active_resource_row();
        let selected = &mut self.source_navigation.selected;
        if !modifiers.shift && !modifiers.command {
            selected.clear();
            selected.insert(row);
            self.source_navigation.anchor = Some(row);
        } else if modifiers.shift {
            let anchor = self
                .source_navigation
                .anchor
                .filter(|a| order.contains(a))
                .unwrap_or(row);
            let start = order.iter().position(|r| *r == anchor).unwrap();
            let end = order.iter().position(|r| *r == row).unwrap();
            if !modifiers.command {
                selected.clear();
            }
            selected.extend(order[start.min(end)..=start.max(end)].iter().copied());
            self.source_navigation.anchor = Some(anchor);
        } else {
            if !selected.remove(&row) {
                selected.insert(row);
            }
            self.source_navigation.anchor = Some(row);
        }
        selected.retain(|r| all.contains(r));
        let active = previous
            .filter(|r| selected.contains(r))
            .or_else(|| all.iter().find(|r| selected.contains(r)).copied());
        match active {
            Some(ResourceRow::Source(id)) => self.editor.edit_library_source(id)?,
            Some(ResourceRow::Library(id)) => self.editor.edit_library(id)?,
            None => {
                let owner = match row {
                    ResourceRow::Library(id) => id,
                    ResourceRow::Source(id) => {
                        self.editor.document.component_source(id).unwrap().0.id
                    }
                };
                self.editor.edit_library(owner)?;
            }
        }
        self.source_navigation.surface = self.active_resource_row();
        Ok(())
    }
}
