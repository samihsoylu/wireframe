// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Read-only live page hierarchy, shared by navigation and page operations.
use crate::document::{Id, Project};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct PageRow {
    pub id: Id,
    pub parent: Option<Id>,
    pub depth: usize,
    pub position: usize,
    pub has_children: bool,
}

#[derive(Clone, Debug)]
pub struct PageTree {
    pub rows: Vec<PageRow>,
    indexes: HashMap<Id, usize>,
}

impl PageTree {
    pub fn new(project: &Project) -> Self {
        let mut seen = HashSet::new();
        let mut children: HashMap<Option<Id>, Vec<(Id, usize)>> = HashMap::new();
        for (position, page) in project.pages.iter().enumerate().filter(|(_, p)| !p.trashed) {
            // Hidden or later parents do not form an effective live hierarchy.
            // Preserve their stored identity; projection never edits the file.
            let parent = page.parent.filter(|id| seen.contains(id));
            children
                .entry(parent)
                .or_default()
                .push((page.id, position));
            seen.insert(page.id);
        }
        let mut rows = Vec::with_capacity(seen.len());
        let mut stack: Vec<_> = children
            .get(&None)
            .into_iter()
            .flatten()
            .rev()
            .map(|&(id, position)| (id, position, None, 0))
            .collect();
        while let Some((id, position, parent, depth)) = stack.pop() {
            let descendants = children.get(&Some(id));
            rows.push(PageRow {
                id,
                position,
                parent,
                depth,
                has_children: descendants.is_some(),
            });
            if let Some(descendants) = descendants {
                stack.extend(
                    descendants
                        .iter()
                        .rev()
                        .map(|&(child, position)| (child, position, Some(id), depth + 1)),
                );
            }
        }
        let indexes = rows.iter().enumerate().map(|(i, r)| (r.id, i)).collect();
        Self { rows, indexes }
    }

    pub fn row(&self, id: Id) -> Option<&PageRow> {
        self.indexes.get(&id).map(|&i| &self.rows[i])
    }

    /// The row plus its complete subtree, independent of expansion state.
    pub fn subtree(&self, id: Id) -> &[PageRow] {
        let Some(&start) = self.indexes.get(&id) else {
            return &[];
        };
        let end = self.rows[start + 1..]
            .iter()
            .position(|r| r.depth <= self.rows[start].depth)
            .map_or(self.rows.len(), |offset| start + 1 + offset);
        &self.rows[start..end]
    }

    pub fn visible(&self, collapsed: &HashSet<Id>) -> Vec<&PageRow> {
        let mut hidden_below = None;
        self.rows
            .iter()
            .filter(|row| {
                if hidden_below.is_some_and(|depth| row.depth > depth) {
                    return false;
                }
                hidden_below = collapsed.contains(&row.id).then_some(row.depth);
                true
            })
            .collect()
    }
}
