// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
impl Editor {
    /// Prepare an editor session. Placeholder conversion is a committed operation,
    /// so cancelling the subsequent text draft leaves the conversion undoable.
    pub fn prepare_text_edit(&mut self, id: Id) -> Result<Option<(Id, String)>, String> {
        let Some(view) = self.current().controls.into_iter().find(|c| c.id == id) else {
            return Ok(None);
        };
        if view.locked || view.group.is_some() || !view.kind.text_editable() {
            return Ok(None);
        }
        let target = match view.kind {
            Kind::TextBlock => Some("paragraph"),
            Kind::TextLine => Some("subtitle"),
            _ => None,
        };
        if let Some(target) = target {
            self.require_structure_edit()?;
            self.edit("Convert to text", |e| {
                if let Some(node) = e.nodes_mut().iter_mut().find(|n| n.id == id) {
                    // Retain identity, origin, ordering, markup and crop. References to
                    // this object remain valid throughout conversion and undo/redo.
                    node.content = NodeContent::Control {
                        kind: target.into(),
                        size: Size {
                            width: Dimension::Fixed(view.bounds.w),
                            height: if target == "subtitle" {
                                Dimension::Auto
                            } else {
                                Dimension::Fixed(view.bounds.h)
                            },
                        },
                    };
                    node.properties.remove("text");
                }
            });
        }
        Ok(self
            .current()
            .controls
            .into_iter()
            .find(|c| c.id == id)
            .map(|c| (id, c.text)))
    }
}
