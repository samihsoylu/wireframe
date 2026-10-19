// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

pub(super) fn identity(value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
        return Err("Invalid reviewer identity".into());
    }
    Ok(())
}
impl Discussion {
    pub fn validate(&self) -> Result<(), String> {
        if self.next_id == 0 || self.next_marker_id == 0 || self.comments.len() > MAX_COMMENTS {
            return Err("Invalid comment collection size or allocator".into());
        }
        let mut markers = BTreeSet::new();
        for (&key, comment) in &self.comments {
            if key == 0 || key != comment.id || key >= self.next_id || comment.revision == 0 {
                return Err("Invalid comment identity".into());
            }
            identity(&comment.author.id)?;
            if comment.author.name.trim().is_empty()
                || comment.author.name.len() > 256
                || comment.author.name.chars().any(char::is_control)
            {
                return Err("Invalid reviewer name".into());
            }
            if comment
                .edited_ms
                .is_some_and(|time| time < comment.created_ms)
            {
                return Err("Invalid comment edit time".into());
            }
            for identities in [&comment.read_by, &comment.liked_by] {
                if identities.len() > MAX_COMMENTS {
                    return Err("Too many reviewer identities".into());
                }
                for id in identities {
                    identity(id)?;
                }
            }
            if comment.deleted {
                if comment.body != Body::default() {
                    return Err("Deleted comment retains body data".into());
                }
            } else if comment.body.text.trim().is_empty() {
                return Err("Enter a comment before posting".into());
            }
            if comment.body.text.contains('\0') || comment.body.callouts.len() > 500 {
                return Err("Invalid comment body".into());
            }
            let refs = comment.body.marker_references()?;
            for callout in &comment.body.callouts {
                if callout.id == 0
                    || callout.id >= self.next_marker_id
                    || callout.label == 0
                    || !markers.insert(callout.id)
                    || !refs.contains(&callout.id)
                    || ![callout.position.x, callout.position.y]
                        .iter()
                        .all(|v| v.is_finite() && v.abs() <= 10_000_000.)
                {
                    return Err("Invalid, duplicate or unreferenced review marker".into());
                }
            }
            let mut path = BTreeSet::new();
            let mut parent = Some(comment.id);
            while let Some(id) = parent {
                if !path.insert(id) || path.len() > MAX_THREAD_DEPTH {
                    return Err("Comment thread is cyclic or too deep".into());
                }
                parent = self
                    .comments
                    .get(&id)
                    .ok_or("Comment parent is missing")?
                    .parent;
            }
        }
        Ok(())
    }
}
