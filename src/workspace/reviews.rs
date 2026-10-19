// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::review::Discussion;

impl Editor {
    /// Review changes persist with the project but do not enter drawing Undo.
    /// The caller keeps drafts and deletion-restoration tokens keyed by surface.
    pub fn change_review<T>(
        &mut self,
        surface: Id,
        change: impl FnOnce(&mut Discussion) -> Result<T, String>,
    ) -> Result<T, String> {
        if !self.document.live_review_surface(surface) {
            return Err("Select an available page or alternate to review".into());
        }
        let mut discussion = self
            .document
            .reviews
            .get(&surface)
            .cloned()
            .unwrap_or_default();
        let before = discussion.clone();
        let result = change(&mut discussion)?;
        if discussion == before {
            return Ok(result);
        }
        let mut document = self.document.clone();
        document.reviews.insert(surface, discussion);
        document.encode()?;
        self.commit();
        self.document.reviews = document.reviews;
        Ok(result)
    }
}

/// Carry current feedback through artwork-only history. If this operation did
/// change a discussion, later feedback must not be overwritten by its replay.
pub(super) fn reconcile_history(
    current: &Project,
    before: &Project,
    after: &Project,
    target: &mut Project,
    undo: bool,
) -> Result<(), String> {
    let expected = if undo { after } else { before };
    let surfaces: HashSet<_> = target.review_surfaces().collect();
    let keys: HashSet<_> = current
        .reviews
        .keys()
        .chain(before.reviews.keys())
        .chain(after.reviews.keys())
        .copied()
        .collect();
    for id in keys {
        let current_review = current.reviews.get(&id);
        if before.reviews.get(&id) == after.reviews.get(&id) {
            if let Some(review) = current_review {
                if !surfaces.contains(&id) {
                    return Err("Cannot replay this drawing change: the page or alternate has review comments".into());
                }
                target.reviews.insert(id, review.clone());
                // Incoming review links can outlive an otherwise removable
                // drawing resource. Reserve its identity so it cannot later
                // point at an unrelated page after allocator rewind.
                for comment in review.comments.values() {
                    for destination in comment.body.surface_references()? {
                        if destination >= target.next_id {
                            target.next_id = destination
                                .checked_add(1)
                                .ok_or("Project ID space exhausted")?;
                            target.removed_pages.insert(destination);
                        }
                    }
                }
            } else {
                target.reviews.remove(&id);
            }
        } else if current_review != expected.reviews.get(&id) {
            return Err(
                "Cannot replay this drawing change: its review comments have changed".into(),
            );
        }
    }
    Ok(())
}
