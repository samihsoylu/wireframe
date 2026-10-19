// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Editor {
    pub fn trash_libraries(&mut self, ids: &[Id]) -> Result<(), String> {
        self.change_library_retention(ids, true)
    }
    pub fn restore_libraries(&mut self, ids: &[Id]) -> Result<(), String> {
        self.change_library_retention(ids, false)
    }
    fn change_library_retention(&mut self, ids: &[Id], trashed: bool) -> Result<(), String> {
        let selected = self.checked_library_transition(ids, !trashed)?;
        self.component_change(
            if trashed {
                "Move libraries to Trash"
            } else {
                "Restore libraries"
            },
            |e| {
                for library in &mut e.document.libraries {
                    if selected.contains(&library.id) {
                        library.trashed = trashed;
                    }
                }
                e.reconcile();
                Ok(())
            },
        )
    }
    /// Call only after successful clipboard publication. Unlike Trash, Cut
    /// removes physical source ownership while keeping consumers unresolved.
    pub fn cut_libraries(&mut self, ids: &[Id]) -> Result<(), String> {
        let selected = self.checked_library_transition(ids, false)?;
        self.component_change("Cut libraries", |e| {
            let definitions: HashSet<_> = e
                .document
                .libraries
                .iter()
                .filter(|l| selected.contains(&l.id))
                .flat_map(|l| l.sources.iter().map(|s| s.definition))
                .collect();
            e.document.libraries.retain(|l| !selected.contains(&l.id));
            e.document
                .definitions
                .retain(|d| !definitions.contains(&d.id));
            if e.document
                .default_library
                .is_some_and(|id| selected.contains(&id))
            {
                e.document.default_library = None;
            }
            e.reconcile();
            Ok(())
        })
    }
    fn checked_library_transition(
        &self,
        ids: &[Id],
        from_trashed: bool,
    ) -> Result<HashSet<Id>, String> {
        let selected: HashSet<_> = ids.iter().copied().collect();
        if selected.is_empty()
            || selected.len() != ids.len()
            || selected.iter().any(|id| {
                !self
                    .document
                    .libraries
                    .iter()
                    .any(|l| l.id == *id && l.trashed == from_trashed)
            })
        {
            return Err("Select distinct available component libraries".into());
        }
        Ok(selected)
    }
}
