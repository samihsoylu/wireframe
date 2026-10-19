// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Page ordering changes metadata only, in one validated history transaction.
use super::*;
use crate::page_tree::PageTree;

struct Order {
    ids: Vec<Id>,
    parents: HashMap<Id, Option<Id>>,
}

impl Order {
    fn new(project: &Project, selected: &[Id]) -> Result<(Self, HashSet<Id>), String> {
        let tree = PageTree::new(project);
        let selected: HashSet<_> = selected.iter().copied().collect();
        if selected.iter().any(|&id| tree.row(id).is_none()) {
            return Err("One or more pages are no longer available for this action".into());
        }
        Ok((
            Self {
                ids: tree.rows.iter().map(|r| r.id).collect(),
                parents: tree.rows.iter().map(|r| (r.id, r.parent)).collect(),
            },
            selected,
        ))
    }

    fn depth(&self, id: Id) -> usize {
        let mut depth = 0;
        let mut parent = self.parents[&id];
        while let Some(id) = parent {
            depth += 1;
            parent = self.parents[&id];
        }
        depth
    }

    fn preceding(&self, index: usize, allowed: impl Fn(Id) -> bool) -> Option<Id> {
        // Prefer a preceding peer, then a shallower ancestor-level candidate.
        (0..=self.depth(self.ids[index])).rev().find_map(|depth| {
            self.ids[..index]
                .iter()
                .rev()
                .copied()
                .find(|&id| allowed(id) && self.depth(id) == depth)
        })
    }

    fn reorder(&mut self, selected: &HashSet<Id>, target: usize) {
        let insertion = target
            - self.ids[..target]
                .iter()
                .filter(|id| selected.contains(id))
                .count();
        let moving: Vec<_> = self
            .ids
            .iter()
            .copied()
            .filter(|id| selected.contains(id))
            .collect();
        for &id in &moving {
            let index = self.ids.iter().position(|&i| i == id).unwrap();
            let replacement = self.preceding(index, |i| !selected.contains(&i));
            for (&child, parent) in &mut self.parents {
                if *parent == Some(id) && !selected.contains(&child) {
                    *parent = replacement;
                }
            }
            if let Some(parent) = self.parents[&id].filter(|p| !selected.contains(p)) {
                let index = self.ids.iter().position(|&i| i == parent).unwrap();
                let replacement = self.preceding(index, |i| selected.contains(&i));
                self.parents.insert(id, replacement);
            }
        }
        self.ids.retain(|id| !selected.contains(id));
        let parent = self.ids.get(insertion).and_then(|id| self.parents[id]);
        for &id in &moving {
            if self.parents[&id].is_none() {
                self.parents.insert(id, parent);
            }
        }
        self.ids.splice(insertion..insertion, moving);
    }

    fn change_level(&mut self, selected: &HashSet<Id>, indent: bool) {
        // Eligibility uses the original tree, processing shallower rows first.
        let mut eligible = Vec::new();
        let mut scheduled = HashSet::new();
        for depth in 0..5 {
            for (index, &id) in self.ids.iter().enumerate().skip(1) {
                if !selected.contains(&id) || self.depth(id) != depth {
                    continue;
                }
                let previous = self.ids[index - 1];
                let allowed = if indent {
                    depth < 4
                        && (depth != self.depth(previous) + 1 || scheduled.contains(&previous))
                } else {
                    depth > 0
                };
                if allowed {
                    eligible.push((index, id));
                    scheduled.insert(id);
                }
            }
        }
        for (index, id) in eligible {
            if indent {
                let depth = self.depth(id);
                if depth == self.depth(self.ids[index - 1]) + 1 {
                    continue;
                }
                let parent = self.ids[..index]
                    .iter()
                    .rev()
                    .copied()
                    .find(|&i| self.depth(i) == depth);
                for p in self.parents.values_mut() {
                    if *p == Some(id) {
                        *p = parent;
                    }
                }
                self.parents.insert(id, parent);
            } else if let Some(parent) = self.parents[&id] {
                let grandparent = self.parents[&parent];
                for &sibling in &self.ids[index + 1..] {
                    if self.parents[&sibling] == Some(parent) {
                        self.parents.insert(sibling, Some(id));
                    }
                }
                self.parents.insert(id, grandparent);
            }
        }
    }

    fn apply(self, editor: &mut Editor, label: &str) -> Result<(), String> {
        let current = PageTree::new(&editor.document);
        if current
            .rows
            .iter()
            .map(|r| r.id)
            .eq(self.ids.iter().copied())
            && current.rows.iter().all(|r| r.parent == self.parents[&r.id])
        {
            return Ok(());
        }
        let mut document = editor.document.clone();
        let mut pages: HashMap<_, _> = document
            .pages
            .iter()
            .filter(|p| !p.trashed)
            .cloned()
            .map(|p| (p.id, p))
            .collect();
        let mut ids = self.ids.into_iter();
        // Retained pages keep their exact slots and stored ancestry.
        for slot in document.pages.iter_mut().filter(|p| !p.trashed) {
            let id = ids.next().unwrap();
            let mut page = pages.remove(&id).unwrap();
            page.parent = self.parents[&id];
            *slot = page;
        }
        document.encode()?;
        editor.edit(label, |e| e.document = document);
        Ok(())
    }
}

impl Editor {
    /// Move an explicit live selection before a gap in the complete live tree.
    /// The caller expands a collapsed-row scope and maps visible gaps first.
    pub fn reorder_pages(&mut self, pages: &[Id], target: usize) -> Result<(), String> {
        let (mut order, selected) = Order::new(&self.document, pages)?;
        if target > order.ids.len() {
            return Err("The page drop position is no longer available".into());
        }
        if selected.is_empty() {
            return Ok(());
        }
        order.reorder(&selected, target);
        order.apply(self, "Reorder wireframes")
    }

    /// Change the indentation of an explicit live selection, preserving order.
    pub fn change_page_indent(&mut self, pages: &[Id], indent: bool) -> Result<(), String> {
        let (mut order, selected) = Order::new(&self.document, pages)?;
        if selected.is_empty() {
            return Ok(());
        }
        order.change_level(&selected, indent);
        order.apply(
            self,
            if indent {
                "Indent wireframes"
            } else {
                "Outdent wireframes"
            },
        )
    }
}
