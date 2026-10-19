// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::borrow::Cow;

impl Editor {
    /// Markup is an appearance classification; position locks do not block it.
    pub fn set_markup(&mut self, id: Id, markup: bool) -> Result<(), String> {
        self.component_change("Change markup", |e| e.apply_markup(id, markup))
    }
    /// Mixed selections become markup; an entirely marked selection is cleared.
    pub fn selection_markup_target(&self) -> Option<bool> {
        if self.selection.is_empty() {
            return None;
        }
        let nodes = self.nodes();
        let mut selected = nodes
            .iter()
            .filter(|n| self.selection.contains(&n.id))
            .peekable();
        selected.peek()?;
        let mut any_plain = false;
        for node in selected {
            any_plain |= !self.document.resolve(node).ok()?.root.markup;
        }
        Some(any_plain)
    }
    pub fn toggle_selection_markup(&mut self) -> Result<(), String> {
        let Some(markup) = self.selection_markup_target() else {
            return Ok(());
        };
        let ids = self.ordered_selection();
        self.component_change("Change markup", |e| {
            for id in ids {
                e.apply_markup(id, markup)?;
            }
            Ok(())
        })
    }
    fn apply_markup(&mut self, id: Id, markup: bool) -> Result<(), String> {
        let before = self
            .nodes()
            .iter()
            .find(|n| n.id == id)
            .cloned()
            .ok_or("Control no longer exists")?;
        if self.editing_instance() {
            let mut after = before.clone();
            after.markup = markup;
            self.write_local_node(id, &before, &after)?;
        } else if matches!(before.content, NodeContent::Instance { .. })
            && self.owning_source(id).is_none()
        {
            let mut path = self.context.clone();
            path.push(id);
            self.change_patch(&path, &[], |patch| patch.markup = Some(markup))?;
            self.nodes_mut()
                .iter_mut()
                .find(|n| n.id == id)
                .unwrap()
                .markup = false;
        } else {
            self.nodes_mut()
                .iter_mut()
                .find(|n| n.id == id)
                .unwrap()
                .markup = markup;
        }
        Ok(())
    }
    /// First canonical instance crossed by the canvas context owns every local patch.
    fn instance_context(&self) -> Option<(NodePath, &Node)> {
        let mut nodes = self.active_content().nodes.as_slice();
        for (index, id) in self.context.iter().enumerate() {
            let node = nodes.iter().find(|n| n.id == *id)?;
            match &node.content {
                NodeContent::Instance { .. } => {
                    return Some((self.context[..=index].to_vec(), node));
                }
                NodeContent::Group { children } => nodes = children,
                _ => return None,
            }
        }
        None
    }
    pub fn editing_instance(&self) -> bool {
        self.instance_context().is_some()
    }
    pub(super) fn context_nodes(&self) -> Option<Cow<'_, [Node]>> {
        if let Some((path, instance)) = self.instance_context() {
            let resolved = self.document.resolve(instance).ok()?.root;
            let NodeContent::Group { children: nodes } = resolved.content else {
                return None;
            };
            children(&nodes, &self.context[path.len()..]).map(|c| Cow::Owned(c.to_vec()))
        } else {
            children(&self.active_content().nodes, &self.context).map(Cow::Borrowed)
        }
    }
    pub(super) fn require_structure_edit(&self) -> Result<(), String> {
        if self.editing_instance() {
            Err("Edit the component source to add, remove, group or reorder its contents.".into())
        } else if self.context_position_locked() {
            Err("Unlock the containing group before changing its structure".into())
        } else {
            Ok(())
        }
    }
    pub(super) fn structure_edit_allowed(&mut self) -> bool {
        match self.require_structure_edit() {
            Ok(()) => true,
            Err(error) => {
                self.error = Some(error);
                false
            }
        }
    }
    fn local_target(&self, id: Id) -> Result<(NodePath, NodePath, Node), String> {
        let (root, instance) = self
            .instance_context()
            .ok_or("Enter a component instance first")?;
        let mut path = self.context[root.len()..].to_vec();
        path.push(id);
        let mut inherited = instance.clone();
        if let NodeContent::Instance { overrides, .. } = &mut inherited.content {
            overrides.clear();
        }
        let inherited = self.document.resolve(&inherited)?.root;
        let node = find_node(&inherited, &path)
            .ok_or("Component child no longer exists")?
            .clone();
        Ok((root, path, node))
    }
    pub(super) fn set_local_lock(&mut self, id: Id, locked: bool) -> Result<(), String> {
        let (root, path, inherited) = self.local_target(id)?;
        self.change_patch(&root, &path, |p| {
            p.locked = (locked != inherited.locked).then_some(locked)
        })
    }
    pub(super) fn change_patch(
        &mut self,
        root: &[Id],
        path: &[Id],
        f: impl FnOnce(&mut Patch),
    ) -> Result<(), String> {
        let (&id, parent) = root.split_last().ok_or("Missing component path")?;
        let node = children_mut(&mut self.active_content_mut().nodes, parent)
            .and_then(|nodes| nodes.iter_mut().find(|n| n.id == id))
            .ok_or("Component no longer exists")?;
        let NodeContent::Instance { overrides, .. } = &mut node.content else {
            return Err("Component no longer exists".into());
        };
        let index = overrides
            .iter()
            .position(|p| p.path == path)
            .unwrap_or_else(|| {
                overrides.push(Override {
                    path: path.to_vec(),
                    patch: Patch::default(),
                });
                overrides.len() - 1
            });
        f(&mut overrides[index].patch);
        overrides.retain(|o| o.patch != Patch::default());
        Ok(())
    }
    pub fn explicit_control_properties(&self, id: Id) -> Properties {
        if let Some((root, instance)) = self.instance_context() {
            let mut path = self.context[root.len()..].to_vec();
            path.push(id);
            if let NodeContent::Instance { overrides, .. } = &instance.content {
                return overrides
                    .iter()
                    .find(|o| o.path == path)
                    .map(|o| o.patch.properties.clone())
                    .unwrap_or_default();
            }
        }
        self.nodes()
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.properties.clone())
            .unwrap_or_default()
    }
    pub(super) fn set_local_property(
        &mut self,
        id: Id,
        key: &str,
        value: Value,
    ) -> Result<(), String> {
        let (root, path, inherited) = self.local_target(id)?;
        let NodeContent::Control { kind, .. } = &inherited.content else {
            return Err("Select a control to edit properties".into());
        };
        let spec = catalog::get(kind).ok_or("Unknown control type")?;
        let effective = spec
            .effective(&inherited, &self.property_context())
            .map_err(|e| e.to_string())?;
        let mut properties = self.explicit_control_properties(id);
        spec.set_override(&mut properties, key, value, &effective)
            .map_err(|e| e.to_string())?;
        self.apply_local_text_anchor(id, &root, &path, &inherited, properties)
    }
    pub(super) fn reset_local_property(&mut self, id: Id, key: &str) -> Result<(), String> {
        let (root, path, inherited) = self.local_target(id)?;
        let NodeContent::Control { kind, .. } = &inherited.content else {
            return Err("Select a control to reset properties".into());
        };
        catalog::get(kind)
            .and_then(|s| s.property(key))
            .ok_or("Unknown control property")?;
        let mut properties = self.explicit_control_properties(id);
        properties.remove(key);
        if key == "state" {
            properties.remove("checked");
        }
        self.apply_local_text_anchor(id, &root, &path, &inherited, properties)
    }
    pub(super) fn reset_local_properties(&mut self, id: Id, keys: &[&str]) -> Result<(), String> {
        let (root, path, inherited) = self.local_target(id)?;
        let (spec, _) = self
            .control_properties(id)
            .ok_or("Select a known control")?;
        let mut properties = self.explicit_control_properties(id);
        let before = properties.len();
        for &key in keys {
            if spec.property(key).is_some() || key == "checked" && keys.contains(&"state") {
                properties.remove(key);
            }
        }
        if properties.len() == before {
            return Ok(());
        }
        // Resolve the whole section once so intermediate font/alignment states do
        // not move an automatic-width text anchor or round its position repeatedly.
        self.apply_local_text_anchor(id, &root, &path, &inherited, properties)
    }
    fn apply_local_text_anchor(
        &mut self,
        id: Id,
        root: &[Id],
        path: &[Id],
        inherited: &Node,
        properties: Properties,
    ) -> Result<(), String> {
        let before = self
            .nodes()
            .iter()
            .find(|n| n.id == id)
            .cloned()
            .ok_or("Control no longer exists")?;
        let mut after = before.clone();
        after.properties = inherited.properties.clone();
        after.properties.extend(properties.clone());
        self.preserve_text_anchor(&before, &mut after)?;
        self.change_patch(root, path, |patch| {
            patch.properties = properties;
            if after.origin != before.origin {
                patch.origin = (after.origin != inherited.origin).then_some(after.origin);
            }
        })
    }
    /// Update only changed geometry/properties, preserving unrelated and obsolete patches.
    pub(super) fn write_local_node(
        &mut self,
        id: Id,
        before: &Node,
        after: &Node,
    ) -> Result<(), String> {
        let (root, path, inherited) = self.local_target(id)?;
        let mut changes = vec![];
        collect_changes(before, after, &inherited, path, &mut changes);
        for (path, change) in changes {
            self.change_patch(&root, &path, |p| {
                if let Some(markup) = change.markup {
                    p.markup = markup;
                }
                if let Some(locked) = change.locked {
                    p.locked = locked;
                }
                if let Some(crop) = change.crop {
                    p.crop = crop;
                }
                if let Some(origin) = change.origin {
                    p.origin = origin;
                }
                if let Some(size) = change.size {
                    p.size = size;
                }
                for (key, value) in change.properties {
                    if let Some(v) = value {
                        p.properties.insert(key, v);
                    } else {
                        p.properties.remove(&key);
                    }
                }
            })?;
        }
        Ok(())
    }
    pub fn has_local_crop(&self, id: Id) -> bool {
        let Some((root, instance)) = self.instance_context() else {
            return false;
        };
        let mut path = self.context[root.len()..].to_vec();
        path.push(id);
        matches!(&instance.content, NodeContent::Instance { overrides, .. }
            if overrides.iter().any(|o| o.path == path && o.patch.crop.is_some()))
    }
    pub fn reset_local_crop(&mut self, id: Id) -> Result<(), String> {
        let (root, path, inherited) = self.local_target(id)?;
        if !self.has_local_crop(id) {
            return Ok(());
        }
        let (_, visible, _) = self.group_crop_geometry(id)?;
        self.component_change("Reset crop to source", |e| {
            e.change_patch(&root, &path, |p| p.crop = None)?;
            let (_, restored, _) = e.group_crop_geometry(id)?;
            let origin = e
                .nodes()
                .iter()
                .find(|n| n.id == id)
                .ok_or("Group no longer exists")?
                .origin
                .translated(Point::new(visible.x - restored.x, visible.y - restored.y));
            // Crop changes alter the visible corner without changing child geometry.
            e.change_patch(&root, &path, |p| {
                p.origin = (origin != inherited.origin).then_some(origin);
            })?;
            e.layout_page(e.surface_id())?;
            Ok(())
        })
    }
    pub fn reset_local_changes(&mut self, id: Id) -> Result<(), String> {
        let (root, path, _) = self.local_target(id)?;
        self.component_change("Reset local changes", |e| {
            let (&id, parent) = root.split_last().unwrap();
            let node = children_mut(&mut e.active_content_mut().nodes, parent)
                .unwrap()
                .iter_mut()
                .find(|n| n.id == id)
                .unwrap();
            if let NodeContent::Instance { overrides, .. } = &mut node.content {
                overrides.retain(|o| !o.path.starts_with(&path));
            }
            Ok(())
        })
    }
    pub fn reset_instance(&mut self, id: Id) -> Result<(), String> {
        self.require_structure_edit()?;
        if self.owning_source(id).is_some() {
            return Err("A source placement has no local instance changes".into());
        }
        if self.missing_component(id) {
            return Err("Restore the component source before resetting its local changes".into());
        }
        self.component_change("Reset component instance", |e| {
            let node = e
                .nodes_mut()
                .iter_mut()
                .find(|n| n.id == id)
                .ok_or("Component no longer exists")?;
            if node.locked {
                return Err("Unlock the component before resetting it".into());
            }
            if let NodeContent::Instance { overrides, .. } = &mut node.content {
                overrides.clear();
                node.crop = None;
                node.markup = false;
                Ok(())
            } else {
                Err("Select a component instance".into())
            }
        })
    }
}
fn find_node<'a>(root: &'a Node, path: &[Id]) -> Option<&'a Node> {
    if path.is_empty() {
        return Some(root);
    }
    let NodeContent::Group { children } = &root.content else {
        return None;
    };
    find_node(children.iter().find(|n| n.id == path[0])?, &path[1..])
}
#[derive(Default)]
struct Delta {
    markup: Option<Option<bool>>,
    locked: Option<Option<bool>>,
    crop: Option<Option<CropOverride>>,
    origin: Option<Option<Point>>,
    size: Option<Option<Size>>,
    properties: Vec<(String, Option<Value>)>,
}
fn collect_changes(
    before: &Node,
    after: &Node,
    inherited: &Node,
    path: NodePath,
    changes: &mut Vec<(NodePath, Delta)>,
) {
    let mut delta = Delta::default();
    if before.markup != after.markup {
        delta.markup = Some((after.markup != inherited.markup).then_some(after.markup));
    }
    if before.locked != after.locked {
        delta.locked = Some((after.locked != inherited.locked).then_some(after.locked));
    }
    if before.crop != after.crop {
        delta.crop = Some(
            (after.crop != inherited.crop)
                .then(|| after.crop.map_or(CropOverride::Clear, CropOverride::Region)),
        );
    }
    if before.origin != after.origin {
        delta.origin = Some((after.origin != inherited.origin).then_some(after.origin));
    }
    if let (
        NodeContent::Control { size: a, .. },
        NodeContent::Control { size: b, .. },
        NodeContent::Control { size: base, .. },
    ) = (&before.content, &after.content, &inherited.content)
        && a != b
    {
        delta.size = Some((b != base).then_some(*b));
    }
    for key in before
        .properties
        .keys()
        .chain(after.properties.keys())
        .collect::<HashSet<_>>()
    {
        if before.properties.get(key) != after.properties.get(key) {
            let value = after.properties.get(key);
            delta.properties.push((
                key.clone(),
                if value == inherited.properties.get(key) || (key == "checked" && value.is_none()) {
                    None
                } else {
                    Some(value.cloned().unwrap_or(Value::Null))
                },
            ));
        }
    }
    changes.push((path.clone(), delta));
    if let (
        NodeContent::Group { children: a },
        NodeContent::Group { children: b },
        NodeContent::Group { children: base },
    ) = (&before.content, &after.content, &inherited.content)
    {
        for child in b {
            if let (Some(old), Some(inherited)) = (
                a.iter().find(|n| n.id == child.id),
                base.iter().find(|n| n.id == child.id),
            ) {
                let mut path = path.clone();
                path.push(child.id);
                collect_changes(old, child, inherited, path, changes);
            }
        }
    }
}

/// Materialization is temporary: only relative descendant deltas are stored back.
pub(super) fn transform_instance(
    project: &Project,
    node: &mut Node,
    layout: &crate::layout::LayoutNode,
    parent: Point,
    new_parent: Point,
    from: Bounds,
    to: Bounds,
) -> Result<(), String> {
    let before = project.resolve(node)?.root;
    let mut after = before.clone();
    super::transform(project, &mut after, layout, parent, new_parent, from, to)?;
    let mut source = node.clone();
    if let NodeContent::Instance { overrides, .. } = &mut source.content {
        overrides.clear();
    }
    let inherited = project.resolve(&source)?.root;
    let mut changes = vec![];
    collect_changes(&before, &after, &inherited, vec![], &mut changes);
    if from.w != to.w || from.h != to.h {
        freeze_resized_geometry(&after, &mut vec![], &mut changes);
    }
    let NodeContent::Instance { overrides, .. } = &mut node.content else {
        unreachable!()
    };
    for (path, delta) in changes.into_iter().filter(|(p, _)| !p.is_empty()) {
        if delta.locked.is_none()
            && delta.markup.is_none()
            && delta.origin.is_none()
            && delta.size.is_none()
            && delta.crop.is_none()
            && delta.properties.is_empty()
        {
            continue;
        }
        let index = overrides
            .iter()
            .position(|o| o.path == path)
            .unwrap_or_else(|| {
                overrides.push(Override {
                    path,
                    patch: Patch::default(),
                });
                overrides.len() - 1
            });
        let patch = &mut overrides[index].patch;
        if let Some(markup) = delta.markup {
            patch.markup = markup;
        }
        if let Some(locked) = delta.locked {
            patch.locked = locked;
        }
        if let Some(crop) = delta.crop {
            patch.crop = crop;
        }
        if let Some(origin) = delta.origin {
            patch.origin = origin;
        }
        if let Some(size) = delta.size {
            patch.size = size;
        }
        for (key, value) in delta.properties {
            if let Some(value) = value {
                patch.properties.insert(key, value);
            } else {
                patch.properties.remove(&key);
            }
        }
    }
    overrides.retain(|o| o.patch != Patch::default());
    node.origin = after.origin;
    Ok(())
}

fn freeze_resized_geometry(node: &Node, path: &mut NodePath, changes: &mut Vec<(NodePath, Delta)>) {
    if !path.is_empty() {
        let index = changes
            .iter()
            .position(|(p, _)| p == path)
            .unwrap_or_else(|| {
                changes.push((path.clone(), Delta::default()));
                changes.len() - 1
            });
        let delta = &mut changes[index].1;
        delta.origin = Some(Some(node.origin));
        if let NodeContent::Control { size, .. } = &node.content {
            delta.size = Some(Some(*size));
        }
    }
    if let NodeContent::Group { children } = &node.content {
        for child in children {
            path.push(child.id);
            freeze_resized_geometry(child, path, changes);
            path.pop();
        }
    }
}
