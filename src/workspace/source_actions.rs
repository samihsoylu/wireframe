// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Editor {
    pub(super) fn at_library_root(&self) -> bool {
        self.library.is_some() && self.definition.is_none() && self.context.is_empty()
    }
    fn library_index(&self, library: Id) -> Result<usize, String> {
        self.document
            .libraries
            .iter()
            .position(|l| l.id == library && !l.trashed)
            .ok_or_else(|| "Component library no longer exists".into())
    }
    /// Materialize only the owning wrapper, retaining ordinary nested instances
    /// and canonical child locks instead of flattening the resolved drawing.
    pub(super) fn source_group(&self, library: Id, node: &Node) -> Result<Node, String> {
        let index = self.library_index(library)?;
        let Some(source) = self.document.libraries[index]
            .sources
            .iter()
            .find(|s| s.node == node.id)
        else {
            return Ok(node.clone());
        };
        let definition = self
            .document
            .definitions
            .iter()
            .find(|d| d.id == source.definition)
            .ok_or("Component source no longer exists")?;
        let mut group = node.clone();
        group.content = NodeContent::Group {
            children: definition.content.nodes.clone(),
        };
        group
            .properties
            .insert(GROUP_NAME.into(), Value::Text(definition.name.clone()));
        group
            .properties
            .insert(GROUP_NOTES.into(), Value::Text(source.notes.clone()));
        Ok(group)
    }
    /// Promote a literal top-level group with a fresh source identity. Its node
    /// identity is allocated by the caller; descendants retain scoped identities.
    pub(super) fn promote_source(
        &mut self,
        library: Id,
        mut group: Node,
    ) -> Result<(Node, Id), String> {
        let NodeContent::Group { children } =
            std::mem::replace(&mut group.content, NodeContent::Group { children: vec![] })
        else {
            return Err("Select a group to create a source".into());
        };
        let index = self.library_index(library)?;
        let name = match group.properties.remove(GROUP_NAME) {
            Some(Value::Text(name)) if !name.is_empty() => name,
            _ => "New component".into(),
        };
        let notes = match group.properties.remove(GROUP_NOTES) {
            Some(Value::Text(notes)) => notes,
            _ => String::new(),
        };
        // Links belong to page groups, not source roots.
        group.properties.remove("link");
        if !group.properties.is_empty() {
            return Err("This group's properties cannot be represented on a source root".into());
        }
        let definition = self.document.allocate_id()?;
        self.document.definitions.push(Definition {
            id: definition,
            name,
            content: Content {
                nodes: children,
                paint_order: vec![],
            },
        });
        self.document.libraries[index]
            .sources
            .push(ComponentSource {
                definition,
                node: group.id,
                notes,
            });
        group.content = NodeContent::Instance {
            definition,
            overrides: vec![],
        };
        Ok((group, definition))
    }
    fn remove_source_owners(&mut self, library: Id, nodes: &HashSet<Id>) -> Result<(), String> {
        let index = self.library_index(library)?;
        let definitions: HashSet<_> = self.document.libraries[index]
            .sources
            .iter()
            .filter(|s| nodes.contains(&s.node))
            .map(|s| s.definition)
            .collect();
        self.document.libraries[index]
            .sources
            .retain(|s| !nodes.contains(&s.node));
        self.document
            .definitions
            .retain(|d| !definitions.contains(&d.id));
        Ok(())
    }
    pub fn duplicate_source(&mut self, definition: Id) -> Result<(), String> {
        let (library, source) = self
            .document
            .component_source(definition)
            .ok_or("Component source no longer exists")?;
        let name = &self
            .document
            .definitions
            .iter()
            .find(|d| d.id == definition)
            .ok_or("Component source no longer exists")?
            .name;
        let source_ids: HashSet<_> = library.sources.iter().map(|s| s.definition).collect();
        let names: HashSet<_> = self
            .document
            .definitions
            .iter()
            .filter(|d| source_ids.contains(&d.id))
            .map(|d| d.name.as_str())
            .collect();
        let base = format!("{name} copy");
        let mut copy_name = base.clone();
        let mut suffix = 0;
        while names.contains(copy_name.as_str()) {
            suffix += 1;
            copy_name = format!("{base} {suffix}");
        }
        self.duplicate_library_nodes(library.id, &HashSet::from([source.node]), Some(copy_name))
    }
    pub(super) fn duplicate_library_selection(&mut self) -> Result<(), String> {
        self.duplicate_library_nodes(self.library.unwrap(), &self.selection.clone(), None)
    }
    fn duplicate_library_nodes(
        &mut self,
        library: Id,
        ids: &HashSet<Id>,
        copy_name: Option<String>,
    ) -> Result<(), String> {
        if ids.is_empty() {
            return Ok(());
        }
        let index = self.library_index(library)?;
        let selected: Vec<_> = self.document.libraries[index]
            .content
            .nodes
            .iter()
            .filter(|n| ids.contains(&n.id))
            .map(|n| self.source_group(library, n))
            .collect::<Result<_, _>>()?;
        if selected.len() != ids.len() {
            return Err("Source selection no longer exists".into());
        }
        self.component_change("Duplicate source controls", |e| {
            e.library = Some(library);
            e.definition = None;
            e.context.clear();
            e.selection.clear();
            let mut definitions = vec![];
            for mut node in selected {
                super::clipboard::unlock_copied_nodes(std::slice::from_mut(&mut node));
                if let Some(name) = &copy_name {
                    node.properties
                        .insert(GROUP_NAME.into(), Value::Text(name.clone()));
                }
                node.id = e.document.allocate_id()?;
                node.origin = node.origin.translated(Point::new(10., 10.));
                if matches!(node.content, NodeContent::Group { .. }) {
                    let (placement, definition) = e.promote_source(library, node)?;
                    node = placement;
                    definitions.push(definition);
                }
                let id = node.id;
                e.document.libraries[index].content.nodes.push(node);
                e.append_order(id);
                e.selection.insert(id);
            }
            if e.selection.len() == 1 && definitions.len() == 1 {
                e.definition = Some(definitions[0]);
                e.selection.clear();
            }
            Ok(())
        })
    }
    pub(super) fn group_library_selection(&mut self) -> Result<(), String> {
        let library = self.library.ok_or("Select a component library")?;
        let ids: HashSet<_> = self
            .nodes()
            .iter()
            .filter(|n| self.selection.contains(&n.id) && !n.locked)
            .map(|n| n.id)
            .collect();
        if ids.is_empty() {
            return Ok(());
        }
        let bounds = self
            .current()
            .controls
            .into_iter()
            .filter(|c| ids.contains(&c.id))
            .map(|c| c.bounds)
            .reduce(Bounds::union)
            .ok_or("Selected controls have no bounds")?;
        let origin = Point::new(bounds.x, bounds.y);
        let index = self.library_index(library)?;
        let first = self.document.libraries[index]
            .content
            .nodes
            .iter()
            .position(|n| ids.contains(&n.id))
            .unwrap();
        let children = self.document.libraries[index]
            .content
            .nodes
            .iter()
            .filter(|n| ids.contains(&n.id))
            .map(|n| {
                let mut child = self.source_group(library, n)?;
                child.origin.x -= origin.x;
                child.origin.y -= origin.y;
                Ok(child)
            })
            .collect::<Result<Vec<_>, String>>()?;
        self.component_change("Group source controls", |e| {
            let id = e.document.allocate_id()?;
            let group = Node::group(id, origin, children);
            e.remove_source_owners(library, &ids)?;
            let (placement, definition) = e.promote_source(library, group)?;
            let content = &mut e.document.libraries[index].content;
            content.nodes.retain(|n| !ids.contains(&n.id));
            content.nodes.insert(first, placement);
            let mut inserted = false;
            content.paint_order = content
                .paint_order
                .iter()
                .filter_map(|path| {
                    if ids.contains(&path[0]) {
                        if inserted {
                            None
                        } else {
                            inserted = true;
                            Some(vec![id])
                        }
                    } else {
                        Some(path.clone())
                    }
                })
                .collect();
            e.definition = Some(definition);
            e.selection.clear();
            Ok(())
        })
    }
    pub(super) fn ungroup_library_selection(&mut self) -> Result<(), String> {
        let library = self.library.ok_or("Select a component library")?;
        let index = self.library_index(library)?;
        let selected: HashSet<_> = self
            .nodes()
            .iter()
            .filter(|n| {
                self.selection.contains(&n.id)
                    && !n.locked
                    && (self.owning_source(n.id).is_some()
                        || matches!(n.content, NodeContent::Group { .. }))
            })
            .map(|n| n.id)
            .collect();
        if selected.is_empty() {
            return Ok(());
        }
        let nodes = self.document.libraries[index]
            .content
            .nodes
            .iter()
            .map(|n| {
                if selected.contains(&n.id) {
                    self.source_group(library, n)
                } else {
                    Ok(n.clone())
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.component_change("Ungroup source controls", |e| {
            e.remove_source_owners(library, &selected)?;
            let mut result = vec![];
            let mut replacements = HashMap::new();
            e.selection.retain(|id| !selected.contains(id));
            for node in nodes {
                if !selected.contains(&node.id) {
                    result.push(node);
                    continue;
                }
                let old = node.id;
                let mut inserted = vec![];
                for mut child in node.ungroup()? {
                    child.id = e.document.allocate_id()?;
                    if matches!(child.content, NodeContent::Group { .. }) {
                        child = e.promote_source(library, child)?.0;
                    }
                    inserted.push(vec![child.id]);
                    e.selection.insert(child.id);
                    result.push(child);
                }
                replacements.insert(old, inserted);
            }
            let content = &mut e.document.libraries[index].content;
            content.nodes = result;
            let mut replaced = HashSet::new();
            content.paint_order = content
                .paint_order
                .iter()
                .flat_map(|path| {
                    if let Some(paths) = replacements.get(&path[0]) {
                        if replaced.insert(path[0]) {
                            paths.clone()
                        } else {
                            vec![]
                        }
                    } else {
                        vec![path.clone()]
                    }
                })
                .collect();
            Ok(())
        })
    }
}
