// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Editor {
    pub fn surface_id(&self) -> Id {
        self.definition.unwrap_or(self.page)
    }
    pub(super) fn surface(&self, id: Id) -> Option<(&str, &Content)> {
        self.document
            .pages
            .iter()
            .find(|p| p.id == id)
            .map(|p| (p.name.as_str(), &p.content))
            .or_else(|| {
                self.document
                    .definitions
                    .iter()
                    .find(|d| d.id == id)
                    .map(|d| (d.name.as_str(), &d.content))
            })
    }
    pub(super) fn active_content(&self) -> &Content {
        self.surface(self.surface_id())
            .expect("Reconciled workspace")
            .1
    }
    pub(super) fn active_content_mut(&mut self) -> &mut Content {
        if let Some(id) = self.definition {
            &mut self
                .document
                .definitions
                .iter_mut()
                .find(|d| d.id == id)
                .expect("Reconciled component")
                .content
        } else {
            &mut self.current_mut().content
        }
    }
    pub(super) fn normalize_source_order(&mut self) -> Result<(), String> {
        if self.definition.is_some() && !self.active_content().paint_order.is_empty() {
            let c = self.active_content();
            let structural = Content {
                nodes: c.nodes.clone(),
                paint_order: vec![],
            }
            .paint_paths()?;
            if structural != c.paint_order {
                return Err("Component groups must keep their children together in layer order. Bring the selected layers together before grouping.".into());
            }
            self.active_content_mut().paint_order.clear();
        }
        Ok(())
    }
    pub(super) fn component_change<T>(
        &mut self,
        name: &str,
        f: impl FnOnce(&mut Self) -> Result<T, String>,
    ) -> Result<T, String> {
        self.commit();
        let before = self.snapshot();
        let result = f(self).and_then(|value| {
            self.repair_empty_order(&before)?;
            self.normalize_source_order()?;
            self.document.validate()?;
            Ok(value)
        });
        match result {
            Ok(value) => {
                self.record(name.into(), before);
                self.reconcile();
                Ok(value)
            }
            Err(error) => {
                self.restore(before);
                Err(error)
            }
        }
    }
    pub fn instance_definition(&self, id: Id) -> Option<Id> {
        self.nodes().iter().find(|n| n.id == id).and_then(|n| {
            if let NodeContent::Instance { definition, .. } = n.content {
                Some(definition)
            } else {
                None
            }
        })
    }
    pub fn missing_component(&self, id: Id) -> bool {
        self.instance_definition(id)
            .is_some_and(|definition| !self.document.definitions.iter().any(|d| d.id == definition))
            || self.nodes().iter().any(|n| {
                n.id == id && n.properties.get(MISSING_COMPONENT) == Some(&Value::Bool(true))
            })
    }
    pub fn empty_container(&self, id: Id) -> bool {
        self.nodes().iter().find(|n| n.id == id)
            .and_then(|n| self.document.resolve(n).ok())
            .is_some_and(|r| matches!(r.root.content, NodeContent::Group { children } if children.is_empty()))
    }
    pub fn delete_definition(&mut self, id: Id) -> Result<(), String> {
        self.component_change("Delete component", |e| {
            let index = e
                .document
                .definitions
                .iter()
                .position(|d| d.id == id)
                .ok_or("Component no longer exists")?;
            e.document.definitions.remove(index);
            e.reconcile();
            Ok(())
        })
    }
    pub fn edit_definition(&mut self, id: Id) -> Result<(), String> {
        self.commit();
        if !self.document.definitions.iter().any(|d| d.id == id) {
            return Err("Component no longer exists".into());
        }
        self.definition = Some(id);
        self.context.clear();
        self.selection.clear();
        Ok(())
    }
    pub fn rename_definition(&mut self, id: Id, name: String) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("Give the component a name".into());
        }
        self.component_change("Rename component", |e| {
            e.document
                .definitions
                .iter_mut()
                .find(|d| d.id == id)
                .ok_or("Component no longer exists")?
                .name = name.trim().into();
            Ok(())
        })
    }
    pub fn create_component(&mut self, name: String) -> Result<Id, String> {
        self.require_structure_edit()?;
        if name.trim().is_empty() {
            return Err("Give the component a name".into());
        }
        if self.selection.len() != 1 {
            return Err("Select one control or group to make a component".into());
        }
        let id = *self.selection.iter().next().unwrap();
        let selected = self
            .nodes()
            .iter()
            .find(|n| n.id == id)
            .ok_or("Control no longer exists")?
            .clone();
        if selected.locked {
            return Err("Unlock the control before making a component".into());
        }
        if matches!(selected.content, NodeContent::Instance { .. }) {
            return Err("This is already a component instance".into());
        }
        let mut path = self.context.clone();
        path.push(id);
        let order = self.leaf_order();
        let positions: Vec<_> = order
            .iter()
            .enumerate()
            .filter(|(_, p)| p.starts_with(&path))
            .map(|(i, _)| i)
            .collect();
        if positions.windows(2).any(|p| p[1] != p[0] + 1) {
            return Err("Bring this group's layers together before making a component".into());
        }
        let mut source = match &selected.content {
            NodeContent::Group { children } => Content {
                nodes: children.clone(),
                paint_order: vec![],
            },
            _ => {
                let mut child = selected.clone();
                child.origin = Point::default();
                Content {
                    nodes: vec![child],
                    paint_order: vec![],
                }
            }
        };
        if matches!(selected.content, NodeContent::Group { .. }) {
            let relative = order
                .iter()
                .filter(|p| p.starts_with(&path))
                .map(|p| p[path.len()..].to_vec())
                .collect::<Vec<_>>();
            if relative != source.paint_paths()? {
                return Err(
                    "Restore this group's structural layer order before making a component".into(),
                );
            }
        }
        self.component_change("Make component", |e| {
            let definition = e.document.allocate_id()?;
            // The selected node keeps its page identity and position.
            if !matches!(selected.content, NodeContent::Group { .. }) {
                source.nodes[0].id = e.document.allocate_id()?;
            }
            e.document.definitions.push(Definition {
                id: definition,
                name: name.trim().into(),
                content: source,
            });
            let node = e.nodes_mut().iter_mut().find(|n| n.id == id).unwrap();
            node.content = NodeContent::Instance {
                definition,
                overrides: vec![],
            };
            if !matches!(selected.content, NodeContent::Group { .. }) {
                node.crop = None;
                node.properties.clear();
            }
            if !e.active_content().paint_order.is_empty() {
                let mut inserted = false;
                e.active_content_mut().paint_order = order
                    .into_iter()
                    .filter_map(|p| {
                        if p.starts_with(&path) {
                            if inserted {
                                None
                            } else {
                                inserted = true;
                                Some(path.clone())
                            }
                        } else {
                            Some(p)
                        }
                    })
                    .collect();
            }
            Ok(definition)
        })
    }
    pub fn insert_component(&mut self, definition: Id, x: f32, y: f32) -> Result<Id, String> {
        self.require_structure_edit()?;
        if !x.is_finite() || !y.is_finite() {
            return Err("Invalid component position".into());
        }
        self.component_change("Insert component", |e| {
            if !e.document.definitions.iter().any(|d| d.id == definition) {
                return Err("Component no longer exists".into());
            }
            let id = e.document.allocate_id()?;
            let parent = e.context_origin();
            let mut n = Node::group(id, Point::new(x - parent.x, y - parent.y), vec![]);
            n.content = NodeContent::Instance {
                definition,
                overrides: vec![],
            };
            e.nodes_mut().push(n);
            e.append_order(id);
            e.selection = HashSet::from([id]);
            Ok(id)
        })
    }
    pub fn detach_component(&mut self, id: Id) -> Result<(), String> {
        self.require_structure_edit()?;
        self.component_change("Detach component", |e| {
            let nodes = e.nodes();
            let node = nodes
                .iter()
                .find(|n| n.id == id)
                .ok_or("Control no longer exists")?;
            if node.locked {
                return Err("Unlock the component before detaching it".into());
            }
            if !matches!(node.content, NodeContent::Instance { .. }) {
                return Err("Select a component instance".into());
            }
            let mut resolved = e.document.resolve(node)?.root;
            fn missing(n: &Node) -> bool {
                n.properties.get(MISSING_COMPONENT) == Some(&Value::Bool(true))
                    || matches!(&n.content, NodeContent::Group { children } if children.iter().any(missing))
            }
            if missing(&resolved) { return Err("Restore the missing component source before detaching".into()); }
            resolved.clear_empty_component_markers();
            let mut path = e.context.clone();
            path.push(id);
            let replacements = Content {
                nodes: vec![resolved.clone()],
                paint_order: vec![],
            }
            .paint_paths()?
            .into_iter()
            .map(|p| {
                let mut full = e.context.clone();
                full.extend(p);
                full
            })
            .collect::<Vec<_>>();
            *e.nodes_mut().iter_mut().find(|n| n.id == id).unwrap() = resolved;
            if !e.active_content().paint_order.is_empty() {
                e.active_content_mut().paint_order = e
                    .active_content()
                    .paint_order
                    .iter()
                    .flat_map(|p| {
                        if *p == path {
                            replacements.clone()
                        } else {
                            vec![p.clone()]
                        }
                    })
                    .collect();
            }
            Ok(())
        })
    }
}
