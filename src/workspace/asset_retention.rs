// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Recoverable shared-image availability, independent of control source edits.
use super::*;

impl Editor {
    pub fn trash_assets(&mut self, ids: &[Id]) -> Result<(), String> {
        self.change_asset_retention(ids, true)
    }

    pub fn restore_assets(&mut self, ids: &[Id]) -> Result<(), String> {
        self.change_asset_retention(ids, false)
    }

    fn change_asset_retention(&mut self, ids: &[Id], trashed: bool) -> Result<(), String> {
        let requested: HashSet<_> = ids.iter().copied().collect();
        if requested.is_empty()
            || requested.len() != ids.len()
            || requested.iter().any(|id| {
                !self
                    .document
                    .assets
                    .iter()
                    .any(|asset| asset.id == *id && asset.trashed != trashed)
            })
        {
            return Err("Select distinct images still available for this action".into());
        }
        // Preflight serialization before committing an unrelated pending edit.
        // All references, bytes, geometry and identities remain intact.
        let mut after = self.document.clone();
        for asset in &mut after.assets {
            if requested.contains(&asset.id) {
                asset.trashed = trashed;
            }
        }
        after.encode()?;
        self.commit();
        let before = self.snapshot();
        let mut history = trash::PageHistory::assets(ids.to_vec(), trashed);
        self.apply_retained_transition(&mut history, false)?;
        self.record_with_page_history(
            if trashed {
                "Move images to Trash"
            } else {
                "Restore images"
            }
            .into(),
            before,
            Some(history),
        );
        Ok(())
    }
}
