// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Compare immutable, fully mapped import previews before omitting a choice.
use super::*;

impl PageImportPlan {
    /// Find undecided pages whose replacement would preserve local content.
    /// Explicit choices remain authoritative. Recompute after choices change:
    /// Keep Both can change an otherwise identical page's link/component targets.
    pub fn equivalent_pages(
        &self,
        pages: &BTreeMap<Id, PageImportChoice>,
        assets: &BTreeMap<Id, AssetImportChoice>,
        libraries: &BTreeMap<Id, LibraryImportChoice>,
    ) -> Result<Vec<Id>, String> {
        if !self
            .entries
            .iter()
            .any(|e| e.existing.is_some() && !pages.contains_key(&e.incoming))
        {
            return Ok(vec![]);
        }
        let mut proposed = pages.clone();
        for entry in self.entries.iter().filter(|e| e.existing.is_some()) {
            proposed
                .entry(entry.incoming)
                .or_insert(PageImportChoice::UseNew);
        }
        let plan = self.review_libraries(&proposed, assets)?;
        let decisions = plan
            .entries()
            .iter()
            .filter(|e| e.action == LibraryImportAction::Conflict)
            .map(|e| {
                (
                    e.incoming,
                    libraries
                        .get(&e.incoming)
                        .copied()
                        .unwrap_or(LibraryImportChoice::UseExisting),
                )
            })
            .collect();
        let preview = plan.prepare(&decisions)?;
        Ok(self
            .entries
            .iter()
            .filter_map(|entry| {
                let id = entry.existing?;
                if pages.contains_key(&entry.incoming) {
                    return None;
                }
                let before = self.base.live_page(id)?;
                let after = preview.after.live_page(id)?;
                same_page(before, after, &self.base, &preview.after).then_some(entry.incoming)
            })
            .collect())
    }
}

fn same_page(before: &Page, after: &Page, a: &Project, b: &Project) -> bool {
    // Names/tree positions are retained by replacement. Notes and discussions
    // deliberately still require review when changed; never silently discard them.
    before.notes == after.notes
        && same_content(&before.content, &after.content)
        && a.reviews.get(&before.id) == b.reviews.get(&after.id)
        && before.alternates.len() == after.alternates.len()
        && before
            .alternates
            .iter()
            .zip(&after.alternates)
            .all(|(old, new)| {
                old.id == new.id
                    && old.name == new.name
                    && old.version == new.version
                    && old.notes == new.notes
                    && same_content(&old.content, &new.content)
                    && a.reviews.get(&old.id) == b.reviews.get(&new.id)
            })
}

fn same_content(a: &Content, b: &Content) -> bool {
    fn structural(nodes: &[Node], path: &[Id]) -> Option<NodePath> {
        let (first, rest) = path.split_first()?;
        let index = nodes.iter().position(|n| n.id == *first)?;
        let mut result = vec![index as Id + 1];
        if !rest.is_empty() {
            let NodeContent::Group { children } = &nodes[index].content else {
                return None;
            };
            result.extend(structural(children, rest)?);
        }
        Some(result)
    }
    fn normalize_nodes(nodes: &mut [Node]) {
        for (index, node) in nodes.iter_mut().enumerate() {
            node.id = index as Id + 1;
            if let NodeContent::Group { children } = &mut node.content {
                normalize_nodes(children);
            }
        }
    }
    fn normalized(content: &Content) -> Option<Content> {
        let mut result = content.clone();
        result.paint_order = content
            .paint_paths()
            .ok()?
            .iter()
            .map(|path| structural(&content.nodes, path))
            .collect::<Option<_>>()?;
        normalize_nodes(&mut result.nodes);
        Some(result)
    }
    match (normalized(a), normalized(b)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}
