// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Resolution {
    pub root: Node,
    /// Obsolete paths remain stored, but cannot change an unrelated descendant.
    pub orphan_overrides: Vec<(NodePath, NodePath)>,
}
#[derive(Debug)]
pub struct ResolvedNode<'a> {
    pub path: NodePath,
    pub node: &'a Node,
    pub world_origin: Point,
    pub locked: bool,
}
impl Project {
    /// Expand inherited definitions first, then apply the outer instance's overrides.
    pub fn resolve(&self, node: &Node) -> Result<Resolution, String> {
        self.resolve_all(std::slice::from_ref(node))
            .map(|mut nodes| nodes.remove(0))
    }
    /// One budget spans the entire page, not just each individual instance.
    pub fn resolve_all(&self, nodes: &[Node]) -> Result<Vec<Resolution>, String> {
        let mut budget = MAX_NODES;
        nodes
            .iter()
            .map(|node| {
                let mut misses = vec![];
                let root =
                    self.expand(node, &mut vec![], &mut vec![], &mut budget, &mut misses, 0)?;
                Ok(Resolution {
                    root,
                    orphan_overrides: misses,
                })
            })
            .collect()
    }
    fn expand(
        &self,
        node: &Node,
        definitions: &mut Vec<Id>,
        path: &mut NodePath,
        budget: &mut usize,
        misses: &mut Vec<(NodePath, NodePath)>,
        depth: usize,
    ) -> Result<Node, String> {
        if depth > MAX_DEPTH {
            return Err("Component expansion is too deep".into());
        }
        *budget = budget
            .checked_sub(1)
            .ok_or("Component expansion exceeds the control limit")?;
        path.push(node.id);
        let mut result = Node {
            id: node.id,
            origin: node.origin,
            locked: node.locked,
            markup: node.markup,
            crop: node.crop,
            properties: node.properties.clone(),
            content: NodeContent::Group { children: vec![] },
        };
        result.content = match &node.content {
            NodeContent::Control { .. } => node.content.clone(),
            NodeContent::Group { children } => NodeContent::Group {
                children: children
                    .iter()
                    .map(|c| self.expand(c, definitions, path, budget, misses, depth + 1))
                    .collect::<Result<_, _>>()?,
            },
            NodeContent::Instance {
                definition,
                overrides,
            } => {
                if definitions.contains(definition) {
                    return Err("Cyclic component definition".into());
                }
                let source = self
                    .definitions
                    .iter()
                    .find(|d| d.id == *definition)
                    .ok_or("Missing component definition")?;
                if !source.content.paint_order.is_empty() {
                    return Err("Component definitions cannot use legacy paint order".into());
                }
                definitions.push(*definition);
                let mut children = source
                    .content
                    .nodes
                    .iter()
                    .map(|c| self.expand(c, definitions, path, budget, misses, depth + 1))
                    .collect::<Result<Vec<_>, _>>()?;
                definitions.pop();
                for patch in overrides {
                    if let Some(target) = find_mut(&mut children, &patch.path) {
                        target.properties.extend(patch.patch.properties.clone());
                        if let Some(origin) = patch.patch.origin {
                            target.origin = origin;
                        }
                        if let Some(size) = patch.patch.size {
                            match &mut target.content {
                                NodeContent::Control { size: actual, .. } => *actual = size,
                                _ => {
                                    return Err("A size override must target a leaf control".into());
                                }
                            }
                        }
                    } else {
                        misses.push((path.clone(), patch.path.clone()));
                    }
                }
                NodeContent::Group { children }
            }
        };
        path.pop();
        Ok(result)
    }
}
fn find_mut<'a>(nodes: &'a mut [Node], path: &[Id]) -> Option<&'a mut Node> {
    let (&id, remaining) = path.split_first()?;
    let node = nodes.iter_mut().find(|n| n.id == id)?;
    if remaining.is_empty() {
        return Some(node);
    }
    match &mut node.content {
        NodeContent::Group { children } => find_mut(children, remaining),
        _ => None,
    }
}
impl Content {
    /// Leaf traversal retains ancestry and local transforms, including migrated layering.
    /// Instances must first be expanded with Project::resolve.
    pub fn leaves(&self) -> Result<Vec<ResolvedNode<'_>>, String> {
        let mut leaves = vec![];
        collect(
            &self.nodes,
            Point::default(),
            false,
            &mut vec![],
            &mut leaves,
            0,
        )?;
        if self.paint_order.is_empty() {
            return Ok(leaves);
        }
        if self.paint_order.len() != leaves.len() {
            return Err("Incomplete paint order".into());
        }
        let mut by_path: BTreeMap<_, _> = leaves.into_iter().map(|n| (n.path.clone(), n)).collect();
        self.paint_order
            .iter()
            .map(|p| {
                by_path
                    .remove(p)
                    .ok_or_else(|| "Invalid paint-order path".into())
            })
            .collect()
    }
}
fn collect<'a>(
    nodes: &'a [Node],
    origin: Point,
    locked: bool,
    path: &mut NodePath,
    out: &mut Vec<ResolvedNode<'a>>,
    depth: usize,
) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("Control hierarchy is too deep".into());
    }
    for node in nodes {
        path.push(node.id);
        let world = node.origin.translated(origin);
        match &node.content {
            NodeContent::Control { .. } => out.push(ResolvedNode {
                path: path.clone(),
                node,
                world_origin: world,
                locked: locked || node.locked,
            }),
            NodeContent::Group { children } => {
                collect(children, world, locked || node.locked, path, out, depth + 1)?
            }
            NodeContent::Instance { .. } => {
                return Err("Expand component instances before traversing leaves".into());
            }
        }
        path.pop();
    }
    Ok(())
}
