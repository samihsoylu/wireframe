// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Editor {
    /// View preference, deliberately excluded from document snapshots/history.
    pub fn markup_visible(&self) -> bool {
        self.markup_visible
    }
    pub fn set_markup_visible(&mut self, visible: bool) {
        self.markup_visible = visible;
        self.retain_visible_selection();
    }
    pub(super) fn retain_visible_selection(&mut self) {
        if self.markup_visible {
            return;
        }
        let visible: HashSet<_> = self.current().controls.iter().map(|c| c.id).collect();
        self.selection.retain(|id| visible.contains(id));
        self.selection_order
            .retain(|id| self.selection.contains(id));
    }
}

/// New control identities reveal markup. Ordinary grouping only changes a
/// path, while an instance creates a distinct occurrence of shared children.
pub(super) fn has_added_markup(before: &Project, after: &Project, surface: Id) -> bool {
    fn instance_ids(project: &Project, content: &Content) -> HashSet<Id> {
        fn collect(nodes: &[Node], ids: &mut HashSet<Id>) {
            for node in nodes {
                match &node.content {
                    NodeContent::Instance { .. } => {
                        ids.insert(node.id);
                    }
                    NodeContent::Group { children } => collect(children, ids),
                    NodeContent::Control { .. } => {}
                }
            }
        }
        let mut ids = HashSet::new();
        collect(&content.nodes, &mut ids);
        for definition in &project.definitions {
            collect(&definition.content.nodes, &mut ids);
        }
        ids
    }
    fn identities(project: &Project, surface: Id, marked_only: bool) -> Option<HashSet<NodePath>> {
        fn walk(
            node: &Node,
            instances: &HashSet<Id>,
            scope: &mut NodePath,
            inherited: bool,
            marked_only: bool,
            result: &mut HashSet<NodePath>,
        ) {
            let marked = inherited || node.markup;
            scope.push(node.id);
            if !marked_only || marked {
                result.insert(scope.clone());
            }
            let instance = instances.contains(&node.id);
            if !instance {
                scope.pop();
            }
            if let NodeContent::Group { children } = &node.content {
                for child in children {
                    walk(child, instances, scope, marked, marked_only, result);
                }
            }
            if instance {
                scope.pop();
            }
        }
        let mut result = HashSet::new();
        if let Some((_, content)) = project.surface(surface) {
            let instances = instance_ids(project, content);
            for resolved in project.resolve_all(&content.nodes).ok()? {
                walk(
                    &resolved.root,
                    &instances,
                    &mut vec![],
                    false,
                    marked_only,
                    &mut result,
                );
            }
        }
        Some(result)
    }
    match (
        identities(before, surface, false),
        identities(after, surface, true),
    ) {
        (Some(before), Some(after)) => after.iter().any(|identity| !before.contains(identity)),
        _ => false,
    }
}
