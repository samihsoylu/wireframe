// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Editor {
    pub fn can_duplicate_move(&self) -> bool {
        self.require_structure_edit().is_ok()
    }

    /// Rebuild a move from its transaction baseline. Offsets are per original
    /// movable root, so grid rounding and locked selections keep their scope.
    /// Switching duplication on/off never records or discards history.
    pub fn preview_move(&mut self, offsets: &[(Id, Point)], duplicate: bool) -> Result<(), String> {
        if self.pending.is_none() {
            return Err("The move was interrupted. Try dragging again.".into());
        }
        self.reset_preview();
        let duplicate = duplicate && self.can_duplicate_move();
        if let Some((name, _)) = &mut self.pending {
            *name = if duplicate {
                "Duplicate controls"
            } else {
                "Move"
            }
            .into();
        }
        if !duplicate {
            self.selected_mut(|c| {
                if let Some((_, offset)) = offsets.iter().find(|(id, _)| *id == c.id) {
                    c.bounds.x += offset.x;
                    c.bounds.y += offset.y;
                }
            });
            return Ok(());
        }
        let result = self.duplicate_move_roots(offsets);
        if result.is_err() {
            self.reset_preview();
        }
        result
    }

    fn duplicate_move_roots(&mut self, offsets: &[(Id, Point)]) -> Result<(), String> {
        let nodes = self.nodes();
        let mut copies = Vec::new();
        for node in nodes
            .iter()
            .filter(|n| self.selection.contains(&n.id) && !n.locked)
        {
            if let Some((_, offset)) = offsets.iter().find(|(id, _)| *id == node.id) {
                if !offset.x.is_finite() || !offset.y.is_finite() {
                    return Err("Invalid move offset".into());
                }
                let copy = if self.at_library_root() {
                    self.source_group(self.library.unwrap(), node)?
                } else {
                    node.clone()
                };
                copies.push((copy, *offset));
            }
        }
        drop(nodes);
        let order = self.leaf_order();
        let explicit_order = !self.active_content().paint_order.is_empty();
        let mut selected = HashSet::new();
        for (mut node, offset) in copies {
            clipboard::unlock_copied_nodes(std::slice::from_mut(&mut node));
            node.clear_empty_component_markers();
            let original_id = node.id;
            node.id = self.document.allocate_id()?;
            node.origin = node.origin.translated(offset);
            if self.at_library_root() && matches!(node.content, NodeContent::Group { .. }) {
                (node, _) = self.promote_source(self.library.unwrap(), node)?;
            }
            let id = node.id;
            self.nodes_mut().push(node);
            if explicit_order {
                let mut root = self.context.clone();
                root.push(original_id);
                let copied_order = order
                    .iter()
                    .filter(|p| p.starts_with(&root))
                    .map(|p| {
                        let mut path = p.clone();
                        path[root.len() - 1] = id;
                        path
                    })
                    .collect::<Vec<_>>();
                self.active_content_mut().paint_order.extend(copied_order);
            }
            selected.insert(id);
        }
        self.selection = selected;
        // Group leaves and owning source placements must be valid during the
        // preview as well as after release. Roll back the whole preview on error.
        let before = self.pending.as_ref().unwrap().1.clone();
        self.repair_empty_order(&before)?;
        self.normalize_source_order()?;
        self.document.validate()?;
        Ok(())
    }
}
