// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

/// Session state; cloning a draft or dropping it never changes the discussion.
#[derive(Clone, Debug)]
pub struct Draft {
    pub comment: Option<Id>,
    pub parent: Option<Id>,
    pub body: Body,
    original: Option<Body>,
    revision: Option<u64>,
}
impl Draft {
    pub fn new(discussion: &Discussion, parent: Option<Id>) -> Result<Self, String> {
        discussion.validate()?;
        if parent.is_some_and(|id| discussion.comments.get(&id).is_none_or(|c| c.deleted)) {
            return Err("Reply parent is no longer available".into());
        }
        Ok(Self {
            comment: None,
            parent,
            body: Body::default(),
            original: None,
            revision: None,
        })
    }
    pub fn edit(discussion: &Discussion, id: Id, actor: &str) -> Result<Self, String> {
        discussion.validate()?;
        let comment = discussion
            .comments
            .get(&id)
            .ok_or("Comment no longer exists")?;
        if comment.deleted || comment.imported || comment.author.id != actor {
            return Err("This comment cannot be edited".into());
        }
        Ok(Self {
            comment: Some(id),
            parent: comment.parent,
            body: comment.body.clone(),
            original: Some(comment.body.clone()),
            revision: Some(comment.revision),
        })
    }
    /// Source-text byte caret. The UI must map its displayed caret to this
    /// boundary before calling; invalid UTF-8 boundaries leave the draft intact.
    pub fn add_callout(
        &mut self,
        discussion: &Discussion,
        position: Point,
        caret: usize,
    ) -> Result<Id, String> {
        discussion.validate()?;
        if !self.body.text.is_char_boundary(caret) {
            return Err("Invalid comment caret".into());
        }
        if ![position.x, position.y]
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 10_000_000.)
        {
            return Err("Invalid marker position".into());
        }
        if self.body.callouts.len() >= 500 {
            return Err("Too many markers in one comment".into());
        }
        let all: Vec<_> = discussion
            .comments
            .values()
            .filter(|c| Some(c.id) != self.comment)
            .flat_map(|c| &c.body.callouts)
            .chain(&self.body.callouts)
            .collect();
        let id = all
            .iter()
            .map(|c| c.id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("Marker identity space exhausted")?
            .max(discussion.next_marker_id);
        let label = all
            .iter()
            .map(|c| c.label)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("Marker label space exhausted")?;
        let callout = Callout {
            id,
            label,
            position,
            tip: Tip::Left,
            color: Color::Blue,
        };
        let mut insertion = callout.markup();
        if caret > 0 && !self.body.text[..caret].ends_with(' ') {
            insertion.insert(0, ' ');
        }
        if caret < self.body.text.len() && !self.body.text[caret..].starts_with(' ') {
            insertion.push(' ');
        }
        if self.body.text.len().saturating_add(insertion.len()) > MAX_BODY_BYTES {
            return Err("Comment text is too large".into());
        }
        self.body.text.insert_str(caret, &insertion);
        self.body.callouts.push(callout);
        Ok(id)
    }
    pub fn commit(
        &self,
        discussion: &mut Discussion,
        author: Author,
        now_ms: u64,
    ) -> Result<Id, String> {
        if self.comment.is_none()
            && self
                .parent
                .is_some_and(|id| discussion.comments.get(&id).is_none_or(|c| c.deleted))
        {
            return Err("Reply parent is no longer available".into());
        }
        if let Some(id) = self.comment {
            let comment = discussion
                .comments
                .get(&id)
                .ok_or("Comment no longer exists")?;
            if self.original.as_ref() != Some(&comment.body)
                || self.revision != Some(comment.revision)
            {
                return Err("Comment changed while being edited; reopen the editor".into());
            }
            discussion.update(id, &author.id, self.body.clone(), now_ms)?;
            Ok(id)
        } else {
            discussion.post(author, self.parent, self.body.clone(), now_ms)
        }
    }
}
