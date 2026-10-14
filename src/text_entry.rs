// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

//! Source-based text editing. Layout may reorder glyphs, but must return UTF-8
//! source offsets to this model. Rendering never supplies the editable string.

use std::{collections::VecDeque, ops::Range};
use unicode_segmentation::UnicodeSegmentation;

/// The side of a source boundary to which a visual caret belongs. Both sides
/// may have different screen positions at a bidirectional or wrapped boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Affinity {
    #[default]
    Upstream,
    Downstream,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Caret {
    pub byte: usize,
    pub affinity: Affinity,
}

impl Caret {
    pub fn new(byte: usize) -> Self {
        Self {
            byte,
            affinity: Affinity::Upstream,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub anchor: Caret,
    pub focus: Caret,
}

impl Selection {
    pub fn range(self) -> Range<usize> {
        self.anchor.byte.min(self.focus.byte)..self.anchor.byte.max(self.focus.byte)
    }
}

#[derive(Clone, Debug)]
struct Change {
    start: usize,
    removed: String,
    inserted: String,
    before: Selection,
    after: Selection,
}

impl Change {
    fn bytes(&self) -> usize {
        self.removed.len() + self.inserted.len()
    }
}

/// A plain-text draft with grapheme-safe editing and reversible changes.
/// Each replacement is one undo transaction; the widget decides transaction
/// boundaries, including committing an entire IME composition at once.
#[derive(Clone, Debug)]
pub struct Draft {
    text: String,
    selection: Selection,
    undo: VecDeque<Change>,
    redo: Vec<Change>,
    history_bytes: usize,
}

impl Draft {
    pub fn new(text: String) -> Self {
        let end = Caret::new(text.len());
        Self {
            text,
            selection: Selection {
                anchor: end,
                focus: end,
            },
            undo: VecDeque::new(),
            redo: Vec::new(),
            history_bytes: 0,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn selection(&self) -> Selection {
        self.selection
    }
    pub fn selected_text(&self) -> &str {
        &self.text[self.selection.range()]
    }
    pub fn character_count(&self) -> usize {
        self.text.graphemes(true).count()
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Clamp externally supplied offsets to whole graphemes, including CRLF.
    /// A byte inside a grapheme resolves to its beginning; affinity is retained.
    pub fn select(&mut self, mut anchor: Caret, mut focus: Caret) {
        anchor.byte = floor_boundary(&self.text, anchor.byte);
        focus.byte = floor_boundary(&self.text, focus.byte);
        self.selection = Selection { anchor, focus };
    }

    pub fn move_to(&mut self, caret: Caret, extend: bool) {
        self.select(if extend { self.selection.anchor } else { caret }, caret);
    }

    pub fn select_all(&mut self) {
        self.select(Caret::new(0), Caret::new(self.text.len()));
    }

    /// Logical movement, for commands that explicitly move through source text.
    /// Visual left/right movement must instead use the layout's caret map.
    pub fn move_logical(&mut self, forward: bool, extend: bool) {
        let range = self.selection.range();
        let byte = if !extend && !range.is_empty() {
            if forward { range.end } else { range.start }
        } else if forward {
            next_boundary(&self.text, self.selection.focus.byte)
        } else {
            previous_boundary(&self.text, self.selection.focus.byte)
        };
        self.move_to(Caret::new(byte), extend);
    }

    pub fn insert(&mut self, text: &str) -> bool {
        self.replace(self.selection.range(), text)
    }

    pub fn delete_backward(&mut self) -> bool {
        let mut range = self.selection.range();
        if range.is_empty() {
            range.start = previous_boundary(&self.text, range.start);
        }
        self.replace(range, "")
    }

    pub fn delete_forward(&mut self) -> bool {
        let mut range = self.selection.range();
        if range.is_empty() {
            range.end = next_boundary(&self.text, range.end);
        }
        self.replace(range, "")
    }

    fn replace(&mut self, range: Range<usize>, replacement: &str) -> bool {
        let removed = self.text[range.clone()].to_owned();
        if removed == replacement {
            self.move_to(Caret::new(range.end), false);
            return false;
        }
        let before = self.selection;
        self.text.replace_range(range.clone(), replacement);
        let end = range.start + replacement.len();
        // Pasting can join surrounding combining marks or ZWJ sequences.
        let floor = floor_boundary(&self.text, end);
        let byte = if floor == end {
            end
        } else {
            next_boundary(&self.text, floor)
        };
        self.move_to(Caret::new(byte), false);
        let change = Change {
            start: range.start,
            removed,
            inserted: replacement.into(),
            before,
            after: self.selection,
        };
        for old in self.redo.drain(..) {
            self.history_bytes -= old.bytes();
        }
        self.history_bytes += change.bytes();
        self.undo.push_back(change);
        // Keep at least the latest edit undoable, even for a large paste.
        while self.undo.len() > 1
            && (self.undo.len() > 256 || self.history_bytes > 16 * 1024 * 1024)
        {
            self.history_bytes -= self.undo.pop_front().unwrap().bytes();
        }
        true
    }

    pub fn undo(&mut self) -> bool {
        let Some(change) = self.undo.pop_back() else {
            return false;
        };
        self.text.replace_range(
            change.start..change.start + change.inserted.len(),
            &change.removed,
        );
        self.selection = change.before;
        self.redo.push(change);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(change) = self.redo.pop() else {
            return false;
        };
        self.text.replace_range(
            change.start..change.start + change.removed.len(),
            &change.inserted,
        );
        self.selection = change.after;
        self.undo.push_back(change);
        true
    }
}

fn floor_boundary(text: &str, byte: usize) -> usize {
    if byte >= text.len() {
        return text.len();
    }
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .take_while(|&i| i <= byte)
        .last()
        .unwrap_or(0)
}

fn previous_boundary(text: &str, byte: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .take_while(|&i| i < byte)
        .last()
        .unwrap_or(0)
}

fn next_boundary(text: &str, byte: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .find(|&i| i > byte)
        .unwrap_or(text.len())
}
