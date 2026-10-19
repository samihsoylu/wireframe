// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Editor {
    /// Source ownership is outside the focused definition's layout tree.
    /// This lock restricts geometry/removal, not insertion or content edits.
    pub(super) fn source_owner_locked(&self) -> bool {
        self.definition
            .and_then(|id| self.document.component_source(id))
            .and_then(|(library, source)| {
                library.content.nodes.iter().find(|n| n.id == source.node)
            })
            .is_some_and(|node| node.locked)
    }

    /// The checkbox edits only this node's lock, never its ancestor's state.
    pub fn own_locked(&self, id: Id) -> bool {
        self.nodes()
            .iter()
            .find(|n| n.id == id)
            .is_some_and(|n| n.locked)
    }

    pub fn position_locked(&self, id: Id) -> bool {
        if self.source_owner_locked() {
            return true;
        }
        let mut path = self.context.clone();
        path.push(id);
        self.layout_page(self.surface_id())
            .ok()
            .and_then(|layout| view::find_layout(&layout.nodes, &path).map(|n| n.locked))
            .unwrap_or(true)
    }

    pub(super) fn context_position_locked(&self) -> bool {
        if self.context.is_empty() {
            return false;
        }
        self.layout_page(self.surface_id())
            .ok()
            .and_then(|layout| view::find_layout(&layout.nodes, &self.context).map(|n| n.locked))
            .unwrap_or(true)
    }

    pub fn lock(&mut self, locked: bool) {
        if let Err(error) = self.component_change(if locked { "Lock" } else { "Unlock" }, |e| {
            for id in e.selection.clone() {
                if e.editing_instance() {
                    e.set_local_lock(id, locked)?;
                } else if let Some(node) = e.nodes_mut().iter_mut().find(|n| n.id == id) {
                    node.locked = locked;
                }
            }
            Ok(())
        }) {
            self.error = Some(error);
        }
    }

    /// A selection badge operates on one node, never the other selected nodes
    /// or a containing group's lock. History keeps that node selected.
    pub fn unlock_selected_control(&mut self, id: Id) {
        if self.selection.contains(&id) && self.own_locked(id) {
            self.select(id, false);
            self.lock(false);
        }
    }
}
