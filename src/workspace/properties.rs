// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Editor {
    fn property_context(&self) -> catalog::Context {
        catalog::Context {
            font_size: self.document.settings.font_size,
            sketch: self.document.settings.sketch,
            ..catalog::Context::default()
        }
    }
    /// Values come from the canonical node, not the temporary canvas projection.
    pub fn control_properties(
        &self,
        id: Id,
    ) -> Option<(&'static catalog::ControlSpec, Properties)> {
        let node = self.nodes().iter().find(|n| n.id == id)?;
        let NodeContent::Control { kind, .. } = &node.content else {
            return None;
        };
        let spec = catalog::get(kind)?;
        let effective = spec.effective(node, &self.property_context()).ok()?;
        Some((spec, effective))
    }
    /// Participates in the caller's begin/commit transaction for continuous input.
    pub fn set_control_property(&mut self, id: Id, key: &str, value: Value) -> Result<(), String> {
        let context = self.property_context();
        let node = self
            .nodes_mut()
            .iter_mut()
            .find(|n| n.id == id)
            .ok_or("Control no longer exists")?;
        if node.locked {
            return Err("Unlock the control before editing its properties".into());
        }
        let NodeContent::Control { kind, .. } = &node.content else {
            return Err("Select a control to edit properties".into());
        };
        let spec = catalog::get(kind).ok_or("Unknown control type")?;
        spec.set_property(node, key, value, &context)
            .map_err(|e| e.to_string())
    }
    pub fn reset_control_property(&mut self, id: Id, key: &str) -> Result<(), String> {
        let (spec, _) = self
            .control_properties(id)
            .ok_or("Select a known control")?;
        let node = self
            .nodes_mut()
            .iter_mut()
            .find(|n| n.id == id)
            .ok_or("Control no longer exists")?;
        if node.locked {
            return Err("Unlock the control before editing its properties".into());
        }
        spec.reset_property(node, key).map_err(|e| e.to_string())
    }

    pub fn auto_size_control(&mut self, id: Id) -> Result<(), String> {
        let (spec, _) = self
            .control_properties(id)
            .ok_or("Select a known control")?;
        let node = self
            .nodes_mut()
            .iter_mut()
            .find(|n| n.id == id)
            .ok_or("Control no longer exists")?;
        if node.locked {
            return Err("Unlock the control before resizing".into());
        }
        let NodeContent::Control { size, .. } = &mut node.content else {
            unreachable!()
        };
        let before = *size;
        use catalog::AutoSize;
        match spec.autosize {
            AutoSize::None => return Err("This control has no automatic size".into()),
            AutoSize::Width => size.width = Dimension::Auto,
            AutoSize::Height => size.height = Dimension::Auto,
            _ => *size = Size::default(),
        }
        if let Err(error) = self.layout_page(self.page) {
            if let Some(Node {
                content: NodeContent::Control { size, .. },
                ..
            }) = self.nodes_mut().iter_mut().find(|n| n.id == id)
            {
                *size = before;
            }
            return Err(error);
        }
        Ok(())
    }
}
