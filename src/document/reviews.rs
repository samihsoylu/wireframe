// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Project {
    pub fn review_surfaces(&self) -> impl Iterator<Item = Id> + '_ {
        self.pages
            .iter()
            .flat_map(|p| std::iter::once(p.id).chain(p.alternates.iter().map(|a| a.id)))
    }
    pub fn live_review_surface(&self, id: Id) -> bool {
        self.live_pages()
            .any(|p| p.id == id || p.alternates.iter().any(|a| a.id == id))
    }
    pub fn has_review_comments(&self, surface: Id) -> bool {
        self.reviews
            .get(&surface)
            .is_some_and(|d| !d.comments.is_empty())
    }
    pub(super) fn validate_reviews(&self) -> Result<(), String> {
        let surfaces: std::collections::HashSet<_> = self.review_surfaces().collect();
        let other_resources: std::collections::HashSet<_> = self
            .assets
            .iter()
            .map(|a| a.id)
            .chain(self.removed_assets.iter().copied())
            .chain(self.libraries.iter().map(|l| l.id))
            .chain(self.definitions.iter().map(|d| d.id))
            .chain(self.versions.iter().map(|v| v.id))
            .collect();
        let mut count = 0usize;
        for (surface, discussion) in &self.reviews {
            if !surfaces.contains(surface) {
                return Err("Review discussion has no owning page or alternate".into());
            }
            count = count.saturating_add(discussion.comments.len());
            if count > crate::review::MAX_COMMENTS {
                return Err("Project has too many review comments".into());
            }
            discussion.validate()?;
            for comment in discussion.comments.values() {
                for target in comment.body.surface_references()? {
                    if target >= self.next_id || other_resources.contains(&target) {
                        return Err("Review link has an invalid page or alternate identity".into());
                    }
                }
            }
        }
        Ok(())
    }
    /// Use only after an explicit resource removal, never to repair input data.
    pub(crate) fn remove_unowned_reviews(&mut self) {
        let surfaces: std::collections::HashSet<_> = self.review_surfaces().collect();
        self.reviews.retain(|surface, _| surfaces.contains(surface));
    }
    pub fn review_links(&self) -> Result<std::collections::HashSet<Id>, String> {
        let mut links = std::collections::HashSet::new();
        for comment in self.reviews.values().flat_map(|d| d.comments.values()) {
            links.extend(comment.body.surface_references()?);
        }
        Ok(links)
    }
}
