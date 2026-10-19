// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Scoped child identities and paths shared by resource comparison and import.
use super::*;

pub(super) struct MatchContext<'a> {
    pub trusted: &'a HashSet<Id>,
    pub reused: &'a HashSet<Id>,
    pub pages: &'a HashSet<Id>,
    pub surfaces: &'a [(Id, Content)],
}

pub(super) struct Correspondence<'a> {
    definitions: &'a [Definition],
    ids: HashMap<(Id, NodePath), Id>,
    guards: HashMap<(Id, NodePath), Id>,
    unmapped: std::cell::RefCell<HashSet<Id>>,
}
impl<'a> Correspondence<'a> {
    pub fn build(
        payload: &'a LibraryClipboard,
        base: &Project,
        resources: &HashMap<Id, Id>,
        context: MatchContext<'_>,
        arena: &mut Project,
    ) -> Result<Self, String> {
        let MatchContext {
            trusted,
            reused,
            pages,
            surfaces,
        } = context;
        let mut result = Self {
            definitions: &payload.definitions,
            ids: HashMap::new(),
            guards: HashMap::new(),
            unmapped: std::cell::RefCell::new(HashSet::new()),
        };
        for definition in &payload.definitions {
            let existing = base
                .definitions
                .iter()
                .find(|d| d.id == resources[&definition.id]);
            let context = Alignment {
                incoming: &payload.definitions,
                existing: &base.definitions,
                resources,
                trusted: trusted.contains(&definition.id),
                pages,
            };
            context.align(
                definition.id,
                &definition.content.nodes,
                existing.map(|d| d.content.nodes.as_slice()),
                &mut vec![],
                &mut result.ids,
                arena,
            )?;
        }
        let mut orphans = HashMap::new();
        for nodes in payload
            .definitions
            .iter()
            .map(|d| &d.content.nodes)
            .chain(payload.libraries.iter().map(|l| &l.content.nodes))
            .chain(surfaces.iter().map(|(_, content)| &content.nodes))
        {
            orphan_scopes(nodes, &payload.definitions, &mut orphans);
        }
        let mut orphans: Vec<_> = orphans.into_iter().collect();
        orphans.sort_unstable_by_key(|(scope, _)| *scope);
        for (scope, mut paths) in orphans {
            paths.sort_unstable();
            paths.dedup();
            for path in paths {
                let (last, prefix) = path.split_last().unwrap();
                let mapped_capture = result.ids.iter().any(|((d, child), id)| {
                    *d == scope
                        && child.len() == path.len()
                        && child.starts_with(prefix)
                        && id == last
                });
                let capture = mapped_capture
                    || (reused.contains(&scope)
                        && could_capture(scope, &path, base, resources, &result.ids));
                if capture {
                    // Each missing prefix needs its own identity so distinct
                    // orphan edits cannot collide in the provisional graph.
                    // A guarded graph may inform decisions but never be applied.
                    result.guards.insert((scope, path), arena.allocate_id()?);
                }
            }
        }
        Ok(result)
    }
    pub fn unmapped_sources(&self) -> HashSet<Id> {
        self.unmapped.borrow().clone()
    }
    /// Rewrite paths while they still address incoming resource IDs. Resource
    /// references themselves are remapped by the normal graph transfer afterward.
    pub fn rewrite(&self, nodes: &mut [Node], scope: Option<Id>) {
        self.rewrite_at(nodes, scope, &mut vec![])
    }
    fn rewrite_at(&self, nodes: &mut [Node], scope: Option<Id>, prefix: &mut NodePath) {
        for node in nodes {
            prefix.push(node.id);
            if let Some(scope) = scope {
                node.id = self.ids[&(scope, prefix.clone())];
            }
            match &mut node.content {
                NodeContent::Group { children } => self.rewrite_at(children, scope, prefix),
                NodeContent::Instance {
                    definition,
                    overrides,
                } => {
                    for value in overrides {
                        value.path = self.path(*definition, &value.path);
                    }
                }
                _ => (),
            }
            prefix.pop();
        }
    }
    fn path(&self, definition: Id, path: &[Id]) -> NodePath {
        if path.is_empty() {
            return vec![];
        }
        let Some(source) = self.definitions.iter().find(|d| d.id == definition) else {
            // Explicit missing sources keep their paths for later reconnection.
            return path.to_vec();
        };
        let mut result = vec![];
        self.path_at(
            definition,
            &source.content.nodes,
            path,
            &mut vec![],
            &mut result,
        );
        result
    }
    fn path_at(
        &self,
        scope: Id,
        nodes: &[Node],
        path: &[Id],
        prefix: &mut NodePath,
        result: &mut NodePath,
    ) {
        if path.is_empty() {
            return;
        }
        let Some(node) = nodes.iter().find(|n| n.id == path[0]) else {
            let mut missing = prefix.clone();
            missing.push(path[0]);
            if let Some(guard) = self.guards.get(&(scope, missing)) {
                self.unmapped.borrow_mut().insert(scope);
                result.push(*guard);
                result.extend_from_slice(&path[1..]);
            } else {
                result.extend_from_slice(path);
            }
            return;
        };
        prefix.push(node.id);
        result.push(self.ids[&(scope, prefix.clone())]);
        if path.len() > 1 {
            match &node.content {
                NodeContent::Group { children } => {
                    self.path_at(scope, children, &path[1..], prefix, result)
                }
                NodeContent::Instance { definition, .. } => {
                    result.extend(self.path(*definition, &path[1..]))
                }
                NodeContent::Control { .. } => result.extend_from_slice(&path[1..]),
            }
        }
        prefix.pop();
    }
}
/// Find the literal source scope containing a missing path segment. Missing
/// source resources remain explicit placeholders and are handled separately.
fn missing_scope(
    scope: Id,
    nodes: &[Node],
    path: &[Id],
    definitions: &[Definition],
) -> Option<(Id, NodePath)> {
    let (&first, rest) = path.split_first()?;
    let Some(node) = nodes.iter().find(|n| n.id == first) else {
        return Some((scope, vec![first]));
    };
    match &node.content {
        NodeContent::Group { children } => {
            let (missing, mut prefix) = missing_scope(scope, children, rest, definitions)?;
            if missing == scope {
                prefix.insert(0, node.id);
            }
            Some((missing, prefix))
        }
        NodeContent::Instance { definition, .. } => {
            let source = definitions.iter().find(|d| d.id == *definition)?;
            missing_scope(*definition, &source.content.nodes, rest, definitions)
        }
        _ => None,
    }
}
fn orphan_scopes(nodes: &[Node], definitions: &[Definition], out: &mut HashMap<Id, Vec<NodePath>>) {
    for node in nodes {
        match &node.content {
            NodeContent::Group { children } => orphan_scopes(children, definitions, out),
            NodeContent::Instance {
                definition,
                overrides,
            } => {
                if let Some(source) = definitions.iter().find(|d| d.id == *definition) {
                    for value in overrides {
                        if let Some((scope, path)) = missing_scope(
                            *definition,
                            &source.content.nodes,
                            &value.path,
                            definitions,
                        ) {
                            out.entry(scope).or_default().push(path);
                        }
                    }
                }
            }
            _ => (),
        }
    }
}
fn could_capture(
    scope: Id,
    path: &[Id],
    base: &Project,
    resources: &HashMap<Id, Id>,
    ids: &HashMap<(Id, NodePath), Id>,
) -> bool {
    let Some(definition) = base
        .definitions
        .iter()
        .find(|d| Some(&d.id) == resources.get(&scope))
    else {
        return false;
    };
    let mut nodes = definition.content.nodes.as_slice();
    let mut prefix = vec![];
    for &id in path {
        prefix.push(id);
        let Some(mapped) = ids.get(&(scope, prefix.clone())) else {
            return nodes.iter().any(|n| n.id == id);
        };
        let Some(node) = nodes.iter().find(|n| n.id == *mapped) else {
            return false;
        };
        if let NodeContent::Group { children } = &node.content {
            nodes = children;
        } else {
            return false;
        }
    }
    false
}
struct Alignment<'a> {
    incoming: &'a [Definition],
    existing: &'a [Definition],
    resources: &'a HashMap<Id, Id>,
    trusted: bool,
    pages: &'a HashSet<Id>,
}
impl Alignment<'_> {
    fn align(
        &self,
        scope: Id,
        incoming: &[Node],
        existing: Option<&[Node]>,
        prefix: &mut NodePath,
        ids: &mut HashMap<(Id, NodePath), Id>,
        arena: &mut Project,
    ) -> Result<(), String> {
        let current = existing.unwrap_or_default();
        let mut matches = vec![None; incoming.len()];
        let mut used = HashSet::new();
        let left_kind: Vec<_> = incoming
            .iter()
            .map(|n| kind(n, Some(self.resources)))
            .collect();
        let right_kind: Vec<_> = current.iter().map(|n| kind(n, None)).collect();
        let left: Vec<_> = incoming
            .iter()
            .map(|n| canonical(n, self.incoming, Some((self.resources, self.pages))))
            .collect();
        let right: Vec<_> = current
            .iter()
            .map(|n| canonical(n, self.existing, None))
            .collect();
        if left.len() == right.len() && left.iter().all(Option::is_some) && left == right {
            for (i, entry) in matches.iter_mut().enumerate() {
                *entry = Some(i);
                used.insert(i);
            }
        }
        if self.trusted && used.is_empty() {
            let by_id: HashMap<_, _> = current.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
            for (i, n) in incoming.iter().enumerate() {
                if let Some(&j) = by_id.get(&n.id)
                    && left_kind[i] == right_kind[j]
                {
                    matches[i] = Some(j);
                    used.insert(j);
                }
            }
        }
        unique_matches(&left, &right, &mut matches, &mut used);
        // Identical repeated siblings are distinguishable by their retained order.
        let remaining_left: Vec<_> = (0..incoming.len())
            .filter(|i| matches[*i].is_none())
            .collect();
        let remaining_right: Vec<_> = (0..current.len()).filter(|j| !used.contains(j)).collect();
        if remaining_left.len() == remaining_right.len()
            && remaining_left
                .iter()
                .zip(&remaining_right)
                .all(|(i, j)| left[*i].is_some() && left[*i] == right[*j])
        {
            for (i, j) in remaining_left.into_iter().zip(remaining_right) {
                matches[i] = Some(j);
                used.insert(j);
            }
        }
        // A sole remaining control of a given kind can retain its identity across
        // a content edit. Multiple changed same-kind controls remain ambiguous.
        unique_matches(&left_kind, &right_kind, &mut matches, &mut used);
        for (i, node) in incoming.iter().enumerate() {
            prefix.push(node.id);
            let target = matches[i].map(|j| &current[j]);
            let id = if let Some(target) = target {
                target.id
            } else if existing.is_some() {
                arena.allocate_id()?
            } else {
                node.id
            };
            ids.insert((scope, prefix.clone()), id);
            if let NodeContent::Group { children } = &node.content {
                let other = target.and_then(|n| match &n.content {
                    NodeContent::Group { children } => Some(children.as_slice()),
                    _ => None,
                });
                self.align(scope, children, other, prefix, ids, arena)?;
            }
            prefix.pop();
        }
        Ok(())
    }
}
fn unique_matches(
    keys: &[Option<Vec<u8>>],
    current: &[Option<Vec<u8>>],
    matches: &mut [Option<usize>],
    used: &mut HashSet<usize>,
) {
    fn index(
        keys: &[Option<Vec<u8>>],
        eligible: impl Fn(usize) -> bool,
    ) -> HashMap<&[u8], Vec<usize>> {
        let mut out: HashMap<&[u8], Vec<usize>> = HashMap::new();
        for (i, key) in keys.iter().enumerate() {
            if eligible(i)
                && let Some(key) = key
            {
                out.entry(key).or_default().push(i);
            }
        }
        out
    }
    let left = index(keys, |i| matches[i].is_none());
    let right = index(current, |i| !used.contains(&i));
    for (key, entries) in left {
        if entries.len() == 1
            && let Some(other) = right.get(key)
            && other.len() == 1
        {
            matches[entries[0]] = Some(other[0]);
            used.insert(other[0]);
        }
    }
}
fn kind(node: &Node, resources: Option<&HashMap<Id, Id>>) -> Option<Vec<u8>> {
    let key = match &node.content {
        NodeContent::Control { kind, .. } => (0, kind.clone()),
        NodeContent::Group { .. } => (1, String::new()),
        NodeContent::Instance { definition, .. } => (
            2,
            resources
                .and_then(|m| m.get(definition))
                .unwrap_or(definition)
                .to_string(),
        ),
    };
    serde_json::to_vec(&key).ok()
}
fn canonical(
    node: &Node,
    definitions: &[Definition],
    resources: Option<(&HashMap<Id, Id>, &HashSet<Id>)>,
) -> Option<Vec<u8>> {
    fn normalize(node: &mut Node, definitions: &[Definition]) -> Option<()> {
        node.id = 0;
        match &mut node.content {
            NodeContent::Group { children } => {
                for n in children {
                    normalize(n, definitions)?;
                }
            }
            NodeContent::Instance {
                definition,
                overrides,
            } => {
                for o in overrides {
                    if !o.path.is_empty() {
                        let d = definitions.iter().find(|d| d.id == *definition)?;
                        o.path = structural(&d.content.nodes, &o.path, definitions)?;
                    }
                }
            }
            _ => (),
        }
        Some(())
    }
    let mut node = node.clone();
    if let Some((resources, pages)) = resources {
        text_link_transfer::rewrite(std::slice::from_mut(&mut node), definitions, &mut |id| {
            let id = resources.get(&id).copied().unwrap_or(id);
            pages.contains(&id).then_some(id)
        })
        .ok()?;
    }
    normalize(&mut node, definitions)?;
    if let Some((resources, pages)) = resources {
        project_copy::rewrite(std::slice::from_mut(&mut node), resources);
        clipboard::strip_missing_links(std::slice::from_mut(&mut node), pages);
    }
    serde_json::to_vec(&node).ok()
}
fn structural(nodes: &[Node], path: &[Id], definitions: &[Definition]) -> Option<NodePath> {
    if path.is_empty() {
        return Some(vec![]);
    }
    let i = nodes.iter().position(|n| n.id == path[0])?;
    let mut out = vec![i as Id + 1];
    if path.len() > 1 {
        let children = match &nodes[i].content {
            NodeContent::Group { children } => children,
            NodeContent::Instance { definition, .. } => {
                &definitions
                    .iter()
                    .find(|d| d.id == *definition)?
                    .content
                    .nodes
            }
            _ => return None,
        };
        out.extend(structural(children, &path[1..], definitions)?);
    }
    Some(out)
}
