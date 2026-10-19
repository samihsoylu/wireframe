// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::arrows::{Arrow, Handle, Mode, Placement, node_geometry};

impl Editor {
    /// The caller owns begin/preview-reset/commit, so a continuous handle drag
    /// creates one history entry and cancellation restores the whole operation.
    pub fn move_arrow_handle(
        &mut self,
        id: Id,
        handle: Handle,
        target: Point,
        constrain: bool,
    ) -> Result<(), String> {
        self.change_arrow(id, |arrow, origin| {
            arrow.move_handle(origin, handle, target, constrain)
        })
    }
    pub fn snap_arrow_middle(&mut self, id: Id) -> Result<(), String> {
        self.change_arrow(id, |arrow, origin| arrow.snap_middle(origin))
    }
    pub(super) fn change_arrow_mode(&mut self, id: Id, value: Value) -> Result<(), String> {
        catalog::get("arrow")
            .unwrap()
            .property("shape")
            .unwrap()
            .validate(&value)
            .map_err(|e| e.to_string())?;
        let mode = if value == Value::Choice("lines".into()) {
            Mode::Lines
        } else {
            Mode::Curve
        };
        self.change_arrow(id, |arrow, origin| arrow.change_mode(origin, mode))
    }
    fn change_arrow(
        &mut self,
        id: Id,
        change: impl FnOnce(Arrow, Point) -> Placement,
    ) -> Result<(), String> {
        if self.context_position_locked() {
            return Err("Unlock the containing group before editing the arrow".into());
        }
        let before = self
            .nodes()
            .iter()
            .find(|n| n.id == id)
            .cloned()
            .ok_or("Arrow no longer exists")?;
        if before.locked {
            return Err("Unlock the arrow before editing it".into());
        }
        let (arrow, _) = node_geometry(&before).ok_or("Select an arrow")?;
        let mut after = before.clone();
        change(arrow, before.origin).apply(&mut after);
        // Validate the complete candidate before publishing either a node or
        // component patch. No partially updated points survive a failed edit.
        let spec = catalog::get("arrow").unwrap();
        spec.effective(&after, &self.property_context())
            .map_err(|e| e.to_string())?;
        let (_, size) = node_geometry(&after).unwrap();
        if ![size.x, size.y]
            .into_iter()
            .all(|v| v.is_finite() && (1. ..=100_000.).contains(&v))
        {
            return Err("Arrow exceeds the supported size".into());
        }
        if !after.origin.x.is_finite()
            || !after.origin.y.is_finite()
            || after.origin.x.abs() > 100_000.
            || after.origin.y.abs() > 100_000.
        {
            return Err("Arrow lies outside the supported canvas".into());
        }
        if self.editing_instance() {
            return self.write_local_node(id, &before, &after);
        }
        *self.nodes_mut().iter_mut().find(|n| n.id == id).unwrap() = after;
        Ok(())
    }
}
