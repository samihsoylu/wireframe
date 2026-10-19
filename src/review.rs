// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Review state for one page surface. Callers own persistence, draft lifetime
//! and reconciliation with drawing history; this module never edits artwork.
mod draft;
mod validation;
use crate::{
    document::Point,
    rich_text::{Inline, RichText},
};
pub use draft::Draft;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Review identities are local to a surface, separate from artwork resources.
pub type Id = u64;
pub const MAX_COMMENTS: usize = 10_000;
pub const MAX_THREAD_DEPTH: usize = 64;
pub const MAX_BODY_BYTES: usize = 1_048_576;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Author {
    pub id: String,
    pub name: String,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tip {
    #[default]
    Left,
    Top,
    Right,
    Bottom,
    None,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Color {
    #[default]
    Blue,
    Green,
    Yellow,
    Red,
    Gray,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Callout {
    pub id: Id,
    pub label: u64,
    pub position: Point,
    pub tip: Tip,
    pub color: Color,
}
impl Callout {
    pub fn markup(&self) -> String {
        format!("[({})](callout:{})", self.label, self.id)
    }
    pub fn size(&self) -> [f32; 2] {
        match self.tip {
            Tip::Left | Tip::Right => [65., 40.],
            Tip::Top | Tip::Bottom => [40., 65.],
            Tip::None => [40., 40.],
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Body {
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub callouts: Vec<Callout>,
}
impl Body {
    pub fn surface_references(&self) -> Result<BTreeSet<Id>, String> {
        if self.text.len() > MAX_BODY_BYTES {
            return Err("Comment text is too large".into());
        }
        let parsed = RichText::parse(&self.text, false);
        if parsed.complexity_limited {
            return Err("Comment formatting is too complex".into());
        }
        Ok(parsed
            .items
            .iter()
            .filter_map(|item| match item {
                Inline::Span { style, .. } if style.link => style
                    .target
                    .as_deref()
                    .and_then(|target| target.strip_prefix("surface:"))
                    .and_then(|id| id.parse::<Id>().ok())
                    .filter(|id| *id != 0),
                _ => None,
            })
            .collect())
    }
    /// Parse actual link spans, so escaped/literal markup cannot retain markers.
    /// Dangling references may remain after another comment deletes its marker.
    pub fn marker_references(&self) -> Result<BTreeSet<Id>, String> {
        if self.text.len() > MAX_BODY_BYTES {
            return Err("Comment text is too large".into());
        }
        let parsed = RichText::parse(&self.text, false);
        if parsed.complexity_limited {
            return Err("Comment formatting is too complex".into());
        }
        Ok(parsed
            .items
            .iter()
            .filter_map(|item| match item {
                Inline::Span { style, .. } if style.link => style
                    .target
                    .as_deref()
                    .and_then(|target| target.strip_prefix("callout:"))
                    .and_then(|id| id.parse::<Id>().ok())
                    .filter(|id| *id != 0),
                _ => None,
            })
            .collect())
    }
    pub fn remove_unreferenced_callouts(&mut self) -> Result<(), String> {
        let ids = self.marker_references()?;
        self.callouts.retain(|c| ids.contains(&c.id));
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    pub id: Id,
    pub revision: u64,
    pub parent: Option<Id>,
    pub author: Author,
    pub created_ms: u64,
    pub edited_ms: Option<u64>,
    pub imported: bool,
    pub deleted: bool,
    pub body: Body,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub read_by: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub liked_by: BTreeSet<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Discussion {
    pub next_id: Id,
    /// Never reuse a deleted marker identity: replies can still reference it.
    pub next_marker_id: Id,
    pub comments: BTreeMap<Id, Comment>,
}
impl Default for Discussion {
    fn default() -> Self {
        Self {
            next_id: 1,
            next_marker_id: 1,
            comments: BTreeMap::new(),
        }
    }
}
/// Session-only restoration data. Intentionally not serializable.
#[derive(Clone, Debug)]
pub struct DeletedBody {
    id: Id,
    revision: u64,
    body: Body,
    tombstone: Comment,
}
pub struct Row<'a> {
    pub comment: &'a Comment,
    pub depth: usize,
}

impl Discussion {
    pub fn post(
        &mut self,
        author: Author,
        parent: Option<Id>,
        mut body: Body,
        now_ms: u64,
    ) -> Result<Id, String> {
        self.validate()?;
        if parent.is_some_and(|id| self.comments.get(&id).is_none_or(|c| c.deleted)) {
            return Err("Reply parent is no longer available".into());
        }
        body.remove_unreferenced_callouts()?;
        self.check_new_markers(&body, None)?;
        let id = self.next_id;
        let mut next = self.clone();
        next.next_id = id
            .checked_add(1)
            .ok_or("Comment identity space exhausted")?;
        next.reserve_markers(&body)?;
        if let Some(parent) = parent {
            next.comments
                .get_mut(&parent)
                .unwrap()
                .read_by
                .insert(author.id.clone());
        }
        let read_by = BTreeSet::from([author.id.clone()]);
        next.comments.insert(
            id,
            Comment {
                id,
                revision: 1,
                parent,
                author,
                created_ms: now_ms,
                edited_ms: None,
                imported: false,
                deleted: false,
                body,
                read_by,
                liked_by: BTreeSet::new(),
            },
        );
        next.validate()?;
        *self = next;
        Ok(id)
    }
    pub fn update(
        &mut self,
        id: Id,
        actor: &str,
        mut body: Body,
        now_ms: u64,
    ) -> Result<(), String> {
        self.validate()?;
        body.remove_unreferenced_callouts()?;
        let mut next = self.clone();
        next.reserve_markers(&body)?;
        let comment = next
            .comments
            .get_mut(&id)
            .ok_or("Comment no longer exists")?;
        if comment.deleted || comment.imported || comment.author.id != actor {
            return Err("Only your own non-imported comments can be edited".into());
        }
        self.check_new_markers(&body, Some(id))?;
        if comment.body == body {
            return Ok(());
        }
        comment.body = body;
        comment.revision = comment
            .revision
            .checked_add(1)
            .ok_or("Comment revision space exhausted")?;
        comment.edited_ms = Some(
            now_ms
                .max(comment.created_ms)
                .max(comment.edited_ms.unwrap_or(0)),
        );
        next.validate()?;
        *self = next;
        Ok(())
    }
    pub fn delete(&mut self, id: Id) -> Result<DeletedBody, String> {
        self.validate()?;
        let comment = self
            .comments
            .get_mut(&id)
            .ok_or("Comment no longer exists")?;
        if comment.deleted {
            return Err("Comment is already deleted".into());
        }
        let revision = comment
            .revision
            .checked_add(1)
            .ok_or("Comment revision space exhausted")?;
        comment.deleted = true;
        comment.revision = revision;
        let body = std::mem::take(&mut comment.body);
        Ok(DeletedBody {
            id,
            revision,
            body,
            tombstone: comment.clone(),
        })
    }
    pub fn restore_deleted(&mut self, cached: &DeletedBody) -> Result<(), String> {
        self.validate()?;
        let mut next = self.clone();
        let comment = next
            .comments
            .get_mut(&cached.id)
            .ok_or("Comment no longer exists")?;
        if !comment.deleted || comment.revision != cached.revision || *comment != cached.tombstone {
            return Err("Deletion restoration is no longer available".into());
        }
        comment.body = cached.body.clone();
        comment.deleted = false;
        comment.revision = comment
            .revision
            .checked_add(1)
            .ok_or("Comment revision space exhausted")?;
        next.validate()?;
        *self = next;
        Ok(())
    }
    pub fn mark_read(&mut self, id: Id, actor: &str, read: bool) -> Result<(), String> {
        self.set_flag(id, actor, read, false)
    }
    pub fn mark_liked(&mut self, id: Id, actor: &str, liked: bool) -> Result<(), String> {
        self.set_flag(id, actor, liked, true)
    }
    fn set_flag(&mut self, id: Id, actor: &str, value: bool, like: bool) -> Result<(), String> {
        self.validate()?;
        validation::identity(actor)?;
        let comment = self
            .comments
            .get_mut(&id)
            .ok_or("Comment no longer exists")?;
        if comment.deleted {
            return Err("Comment is deleted".into());
        }
        let values = if like {
            &mut comment.liked_by
        } else {
            &mut comment.read_by
        };
        if value {
            if values.len() >= MAX_COMMENTS && !values.contains(actor) {
                return Err("Too many reviewer identities".into());
            }
            values.insert(actor.into());
        } else {
            values.remove(actor);
        }
        Ok(())
    }
    /// Deleted ancestors remain when they connect visible replies. Restorable
    /// deleted leaves are supplied by the caller's current session cache.
    pub fn rows(&self, restorable: &BTreeSet<Id>) -> Result<Vec<Row<'_>>, String> {
        self.validate()?;
        let mut visible = BTreeSet::new();
        for comment in self
            .comments
            .values()
            .filter(|c| !c.deleted || restorable.contains(&c.id))
        {
            let mut id = Some(comment.id);
            while let Some(current) = id {
                if !visible.insert(current) {
                    break;
                }
                id = self.comments[&current].parent;
            }
        }
        let mut children: BTreeMap<Option<Id>, Vec<&Comment>> = BTreeMap::new();
        for comment in self.comments.values().filter(|c| visible.contains(&c.id)) {
            children.entry(comment.parent).or_default().push(comment);
        }
        for (parent, children) in &mut children {
            children.sort_by_key(|c| (c.created_ms, c.id));
            if parent.is_none() {
                children.reverse();
            }
        }
        let mut stack: Vec<_> = children
            .remove(&None)
            .unwrap_or_default()
            .into_iter()
            .rev()
            .map(|c| (c, 0))
            .collect();
        let mut rows = vec![];
        while let Some((comment, depth)) = stack.pop() {
            if let Some(replies) = children.remove(&Some(comment.id)) {
                stack.extend(replies.into_iter().rev().map(|c| (c, depth + 1)));
            }
            rows.push(Row { comment, depth });
        }
        Ok(rows)
    }
    pub fn callout(&self, id: Id) -> Option<&Callout> {
        self.comments
            .values()
            .filter(|c| !c.deleted)
            .flat_map(|c| &c.body.callouts)
            .find(|c| c.id == id)
    }
    /// Explicit selection takes precedence; otherwise only an edited thread
    /// reveals markers. A new root draft reveals only its own markers.
    pub fn visible_callouts<'a>(
        &'a self,
        draft: Option<&'a Draft>,
        selected: Option<Id>,
    ) -> Result<Vec<&'a Callout>, String> {
        self.validate()?;
        if let Some(draft) = draft {
            self.check_new_markers(&draft.body, draft.comment)?;
        }
        let all: Vec<_> = self
            .comments
            .values()
            .filter(|c| !c.deleted && draft.is_none_or(|d| d.comment != Some(c.id)))
            .flat_map(|c| &c.body.callouts)
            .chain(draft.into_iter().flat_map(|d| &d.body.callouts))
            .collect();
        if let Some(id) = selected {
            return Ok(all.into_iter().filter(|c| c.id == id).take(1).collect());
        }
        let Some(draft) = draft else {
            return Ok(vec![]);
        };
        let anchor = draft.comment.or(draft.parent);
        let Some(anchor) = anchor else {
            return Ok(draft.body.callouts.iter().collect());
        };
        if self.comments.get(&anchor).is_none_or(|c| c.deleted) {
            return Err("Edited comment or reply parent is no longer available".into());
        }
        let root = self.thread_root(anchor)?;
        let mut markers = vec![];
        for comment in self
            .comments
            .values()
            .filter(|c| !c.deleted && Some(c.id) != draft.comment)
        {
            if self.thread_root(comment.id)? == root {
                markers.extend(&comment.body.callouts);
            }
        }
        markers.extend(&draft.body.callouts);
        Ok(markers)
    }
    fn thread_root(&self, mut id: Id) -> Result<Id, String> {
        for _ in 0..MAX_THREAD_DEPTH {
            match self
                .comments
                .get(&id)
                .ok_or("Comment parent is missing")?
                .parent
            {
                Some(parent) => id = parent,
                None => return Ok(id),
            }
        }
        Err("Comment thread is cyclic or too deep".into())
    }
    fn reserve_markers(&mut self, body: &Body) -> Result<(), String> {
        if let Some(maximum) = body.callouts.iter().map(|c| c.id).max() {
            self.next_marker_id = self.next_marker_id.max(
                maximum
                    .checked_add(1)
                    .ok_or("Marker identity space exhausted")?,
            );
        }
        Ok(())
    }
    fn check_new_markers(&self, body: &Body, editing: Option<Id>) -> Result<(), String> {
        for marker in &body.callouts {
            if marker.id < self.next_marker_id
                && !editing
                    .and_then(|id| self.comments.get(&id))
                    .is_some_and(|c| c.body.callouts.iter().any(|owned| owned.id == marker.id))
            {
                return Err("Marker identity changed while editing; refresh the draft".into());
            }
        }
        Ok(())
    }
}
