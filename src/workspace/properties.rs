// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Editor {
    /// Whole-control, group and instance targets share the saved Link model.
    pub fn node_link(&self, id: Id) -> Option<Link> {
        self.nodes()
            .iter()
            .find(|n| n.id == id)
            .and_then(|n| match n.properties.get("link") {
                Some(Value::Link(link)) => Some(link.clone()),
                _ => None,
            })
    }
    /// Participates in the caller's begin/commit transaction, including local overrides.
    pub fn set_node_link(&mut self, id: Id, target: Option<Link>) -> Result<(), String> {
        if target.as_ref().is_some_and(|link| !link.valid()) {
            return Err("Enter a valid web address or choose a page".into());
        }
        let before = self
            .nodes()
            .iter()
            .find(|n| n.id == id)
            .cloned()
            .ok_or("Control no longer exists")?;
        let mut after = before.clone();
        after.properties.insert(
            "link".into(),
            target.map(Value::Link).unwrap_or(Value::Null),
        );
        if self.editing_instance() {
            return self.write_local_node(id, &before, &after);
        }
        *self.nodes_mut().iter_mut().find(|n| n.id == id).unwrap() = after;
        Ok(())
    }
    pub(super) fn property_context(&self) -> catalog::Context {
        catalog::Context::default().with_settings(&self.document.settings)
    }
    /// Values come from the canonical node, not the temporary canvas projection.
    pub fn control_properties(
        &self,
        id: Id,
    ) -> Option<(&'static catalog::ControlSpec, Properties)> {
        let nodes = self.nodes();
        let node = nodes.iter().find(|n| n.id == id)?;
        let NodeContent::Control { kind, .. } = &node.content else {
            return None;
        };
        let spec = catalog::get(kind)?;
        let effective = spec.effective(node, &self.property_context()).ok()?;
        Some((spec, effective))
    }
    pub(super) fn structured_property_baseline(&self, id: Id) -> Option<Properties> {
        let kind = self.control_properties(id)?.0.id;
        if !matches!(
            kind,
            "list"
                | "button_bar"
                | "breadcrumbs"
                | "link_bar"
                | "tabs"
                | "vertical_tabs"
                | "checkbox_group"
                | "radio_group"
                | "menu"
                | "menu_bar"
                | "mobile_menu"
                | "tree"
                | "accordion"
                | "alert"
                | "app_bar"
                | "site_map"
        ) {
            return None;
        }
        let snapshot = self.pending.as_ref().map(|(_, s)| s).filter(|s| {
            s.page == self.page
                && s.definition == self.definition
                && s.alternate == self.active_alternate(self.page)
                && s.context == self.context
        });
        let project = snapshot.map_or(&self.document, |s| &s.document);
        let surface = self.surface_id();
        let content = project.surface(surface)?.1;
        let resolved = project
            .resolve_all(&content.nodes)
            .ok()?
            .into_iter()
            .map(|r| r.root)
            .collect::<Vec<_>>();
        let nodes = children(&resolved, &self.context)?;
        let node = nodes.iter().find(|n| n.id == id)?;
        if !matches!(&node.content,NodeContent::Control{kind,..} if matches!(kind.as_str(), "list" | "button_bar" | "breadcrumbs" | "link_bar" | "tabs" | "vertical_tabs" | "checkbox_group" | "radio_group" | "menu" | "menu_bar" | "mobile_menu" | "tree" | "accordion" | "alert" | "app_bar" | "site_map"))
        {
            return None;
        }
        catalog::get(kind)?
            .effective(node, &self.property_context())
            .ok()
    }
    /// Participates in the caller's begin/commit transaction for continuous input.
    pub fn set_control_property(&mut self, id: Id, key: &str, value: Value) -> Result<(), String> {
        if key == "shape"
            && self
                .control_properties(id)
                .is_some_and(|(spec, _)| spec.id == "arrow")
        {
            return self.change_arrow_mode(id, value);
        }
        if key == "icons"
            && self
                .control_properties(id)
                .is_some_and(|(spec, _)| spec.id == "app_bar")
        {
            let baseline = self
                .structured_property_baseline(id)
                .ok_or("Control no longer exists")?;
            let Value::Icons(icons) = &value else {
                return Err("Custom icons must be an icon collection".into());
            };
            if icons.len() > 9_998 {
                return Err("Too many app-bar icons".into());
            }
            let before = self
                .nodes()
                .iter()
                .find(|n| n.id == id)
                .cloned()
                .ok_or("Control no longer exists")?;
            let mut edited = before.clone();
            let spec = catalog::get("app_bar").unwrap();
            spec.set_property(&mut edited, key, value.clone(), &self.property_context())
                .map_err(|e| e.to_string())?;
            if let Some(Value::Links(links)) = baseline.get("row_links")
                && !links.is_empty()
            {
                let old = match baseline.get("icons") {
                    Some(Value::Icons(v)) => v.as_slice(),
                    _ => &[],
                };
                edited.properties.insert(
                    "row_links".into(),
                    Value::Links(crate::app_bar::remap_links(old, icons, links)),
                );
            }
            if self.editing_instance() {
                return self.write_local_node(id, &before, &edited);
            }
            *self.nodes_mut().iter_mut().find(|n| n.id == id).unwrap() = edited;
            return Ok(());
        }
        if key == "text"
            && self
                .control_properties(id)
                .is_some_and(|(spec, _)| spec.id != "app_bar")
            && let Some(baseline) = self.structured_property_baseline(id)
        {
            let Value::Text(text) = value else {
                return Err("Control text must be text".into());
            };
            let before = self
                .nodes()
                .iter()
                .find(|n| n.id == id)
                .cloned()
                .ok_or("Control no longer exists")?;
            let mut edited = before.clone();
            if matches!(&edited.content,NodeContent::Control {kind,..} if matches!(kind.as_str(),"tabs"|"vertical_tabs"))
            {
                crate::tabs::apply_text(&mut edited, &text, &baseline)?;
            } else if matches!(&edited.content,NodeContent::Control {kind,..} if matches!(kind.as_str(),"breadcrumbs"|"link_bar"))
            {
                crate::navigation::apply_text(&mut edited, &text, &baseline)?;
            } else if matches!(&edited.content,NodeContent::Control {kind,..} if matches!(kind.as_str(),"menu"|"menu_bar"))
            {
                crate::menu::apply_text(&mut edited, &text, &baseline)?;
            } else if matches!(&edited.content,NodeContent::Control {kind,..} if kind=="mobile_menu")
            {
                crate::mobile_menu::apply_text(&mut edited, &text, &baseline)?;
            } else if matches!(&edited.content,NodeContent::Control {kind,..} if kind=="accordion")
            {
                crate::accordion::apply_text(&mut edited, &text, &baseline)?;
            } else if matches!(&edited.content,NodeContent::Control {kind,..} if kind=="alert") {
                crate::alert::apply_text(&mut edited, &text, &baseline)?;
            } else if matches!(&edited.content,NodeContent::Control {kind,..} if kind=="site_map") {
                crate::site_map::apply_text(&mut edited, &text, &baseline)?;
            } else if matches!(&edited.content,NodeContent::Control {kind,..} if kind=="tree") {
                crate::tree::apply_text(&mut edited, &text, &baseline)?;
            } else if matches!(&edited.content,NodeContent::Control {kind,..} if kind=="button_bar")
            {
                crate::button_bar::apply_text(&mut edited, &text, &baseline)?;
            } else if matches!(&edited.content,NodeContent::Control {kind,..} if matches!(kind.as_str(),"checkbox_group"|"radio_group"))
            {
                crate::choice_group::apply_text(&mut edited, &text, &baseline)?;
            } else {
                crate::list::apply_text(&mut edited, &text, &baseline)?;
            }
            if self.editing_instance() {
                return self.write_local_node(id, &before, &edited);
            }
            *self.nodes_mut().iter_mut().find(|n| n.id == id).unwrap() = edited;
            return Ok(());
        }
        if self.editing_instance() {
            return self.set_local_property(id, key, value);
        }
        let context = self.property_context();
        let before = self
            .nodes()
            .iter()
            .find(|n| n.id == id)
            .cloned()
            .ok_or("Control no longer exists")?;
        let mut edited = before.clone();
        let NodeContent::Control { kind, .. } = &edited.content else {
            return Err("Select a control to edit properties".into());
        };
        let spec = catalog::get(kind).ok_or("Unknown control type")?;
        spec.set_property(&mut edited, key, value, &context)
            .map_err(|e| e.to_string())?;
        self.preserve_text_anchor(&before, &mut edited)?;
        *self.nodes_mut().iter_mut().find(|n| n.id == id).unwrap() = edited;
        Ok(())
    }
    pub fn reset_control_property(&mut self, id: Id, key: &str) -> Result<(), String> {
        if key == "shape"
            && self
                .control_properties(id)
                .is_some_and(|(spec, _)| spec.id == "arrow")
        {
            let snapshot = self.snapshot();
            let result = (|| {
                self.reset_property_value(id, key)?;
                let mode = self
                    .control_properties(id)
                    .ok_or("Arrow no longer exists")?
                    .1["shape"]
                    .clone();
                self.change_arrow_mode(id, mode)
            })();
            if result.is_err() {
                self.restore(snapshot);
            }
            return result;
        }
        if key == "icons"
            && let Some((spec, before)) = self.control_properties(id)
            && spec.id == "app_bar"
        {
            let snapshot = self.snapshot();
            let result = (|| {
                self.reset_property_value(id, key)?;
                let (_, after) = self
                    .control_properties(id)
                    .ok_or("Control no longer exists")?;
                if let Some(Value::Links(links)) = before.get("row_links")
                    && !links.is_empty()
                {
                    let old = match before.get("icons") {
                        Some(Value::Icons(v)) => v.as_slice(),
                        _ => &[],
                    };
                    let new = match after.get("icons") {
                        Some(Value::Icons(v)) => v.as_slice(),
                        _ => &[],
                    };
                    self.set_control_property(
                        id,
                        "row_links",
                        Value::Links(crate::app_bar::remap_links(old, new, links)),
                    )?;
                }
                Ok(())
            })();
            if result.is_err() {
                self.restore(snapshot);
            }
            return result;
        }
        self.reset_property_value(id, key)
    }
    fn reset_property_value(&mut self, id: Id, key: &str) -> Result<(), String> {
        if self.editing_instance() {
            return self.reset_local_property(id, key);
        }
        let (spec, _) = self
            .control_properties(id)
            .ok_or("Select a known control")?;
        let node = self
            .nodes_mut()
            .iter_mut()
            .find(|n| n.id == id)
            .ok_or("Control no longer exists")?;
        let before = node.clone();
        let mut edited = before.clone();
        spec.reset_property(&mut edited, key)
            .map_err(|e| e.to_string())?;
        self.preserve_text_anchor(&before, &mut edited)?;
        *self.nodes_mut().iter_mut().find(|n| n.id == id).unwrap() = edited;
        Ok(())
    }

    /// Replace diagram labels and destinations together inside the caller's transaction.
    pub fn populate_site_map(&mut self, id: Id) -> Result<(), String> {
        let (spec, _) = self.control_properties(id).ok_or("Select a site map")?;
        if spec.id != "site_map" {
            return Err("Select a site map".into());
        }
        let before = self
            .nodes()
            .iter()
            .find(|n| n.id == id)
            .cloned()
            .ok_or("Control no longer exists")?;
        let mut edited = before.clone();
        let (text, links) = crate::site_map::from_project(&self.document);
        spec.set_property(
            &mut edited,
            "text",
            Value::Text(text),
            &self.property_context(),
        )
        .map_err(|e| e.to_string())?;
        spec.set_property(
            &mut edited,
            "row_links",
            Value::Links(links),
            &self.property_context(),
        )
        .map_err(|e| e.to_string())?;
        if self.editing_instance() {
            return self.write_local_node(id, &before, &edited);
        }
        *self.nodes_mut().iter_mut().find(|n| n.id == id).unwrap() = edited;
        if let Err(e) = self.layout_page(self.surface_id()) {
            *self.nodes_mut().iter_mut().find(|n| n.id == id).unwrap() = before;
            return Err(e);
        }
        Ok(())
    }
    pub fn auto_size_control(&mut self, id: Id) -> Result<(), String> {
        let (spec, _) = self
            .control_properties(id)
            .ok_or("Select a known control")?;
        if spec.id == "image" {
            return self.auto_size_image(id);
        }
        let mut edited = self
            .nodes()
            .iter()
            .find(|n| n.id == id)
            .cloned()
            .ok_or("Control no longer exists")?;
        let original = edited.clone();
        let node = &mut edited;
        let NodeContent::Control { size, .. } = &mut node.content else {
            unreachable!()
        };
        use catalog::AutoSize;
        match spec.autosize {
            AutoSize::None => return Err("This control has no automatic size".into()),
            AutoSize::Width => size.width = Dimension::Auto,
            AutoSize::Height => size.height = Dimension::Auto,
            _ => *size = Size::default(),
        }
        self.preserve_text_anchor(&original, &mut edited)?;
        if self.editing_instance() {
            return self.write_local_node(id, &original, &edited);
        }
        *self.nodes_mut().iter_mut().find(|n| n.id == id).unwrap() = edited;
        if let Err(error) = self.layout_page(self.surface_id()) {
            *self.nodes_mut().iter_mut().find(|n| n.id == id).unwrap() = original;
            return Err(error);
        }
        Ok(())
    }
}
