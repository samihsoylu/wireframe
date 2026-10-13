// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

use crate::model::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
struct Change {
    name: String,
    before: Document,
    after: Document,
}

pub struct Editor {
    pub document: Document,
    pub page: Id,
    pub selection: HashSet<Id>,
    history: Vec<Change>,
    cursor: usize,
    saved: Document,
    pending: Option<(String, Document)>,
}
impl Default for Editor {
    fn default() -> Self {
        Self::new(Document::default())
    }
}
impl Editor {
    pub fn new(document: Document) -> Self {
        let page = document.pages[0].id;
        Self {
            saved: document.clone(),
            document,
            page,
            selection: HashSet::new(),
            history: Vec::new(),
            cursor: 0,
            pending: None,
        }
    }
    pub fn current(&self) -> &Page {
        self.document
            .pages
            .iter()
            .find(|p| p.id == self.page)
            .unwrap_or(&self.document.pages[0])
    }
    pub fn current_mut(&mut self) -> &mut Page {
        let idx = self
            .document
            .pages
            .iter()
            .position(|p| p.id == self.page)
            .unwrap_or(0);
        &mut self.document.pages[idx]
    }
    pub fn dirty(&self) -> bool {
        self.document != self.saved
    }
    pub fn mark_saved(&mut self) {
        self.saved = self.document.clone();
    }
    pub fn undo_name(&self) -> Option<&str> {
        self.cursor
            .checked_sub(1)
            .map(|i| self.history[i].name.as_str())
    }
    pub fn redo_name(&self) -> Option<&str> {
        self.history.get(self.cursor).map(|c| c.name.as_str())
    }
    pub fn begin(&mut self, name: &str) {
        if self.pending.is_none() {
            self.pending = Some((name.into(), self.document.clone()));
        }
    }
    pub fn commit(&mut self) {
        if let Some((name, before)) = self.pending.take() {
            self.record(name, before);
        }
    }
    pub fn cancel(&mut self) {
        if let Some((_, before)) = self.pending.take() {
            self.document = before;
            self.reconcile();
        }
    }
    fn record(&mut self, name: String, before: Document) {
        if before == self.document {
            return;
        }
        self.history.truncate(self.cursor);
        self.history.push(Change {
            name,
            before,
            after: self.document.clone(),
        });
        // A bounded history avoids holding an unbounded number of document snapshots.
        let size = |c: &Change| {
            c.before
                .pages
                .iter()
                .chain(c.after.pages.iter())
                .map(|p| {
                    p.controls.len() * 256
                        + p.controls.iter().map(|c| c.text.len()).sum::<usize>()
                        + p.notes.len()
                        + p.name.len()
                })
                .sum::<usize>()
                + c.before.title.len()
                + c.after.title.len()
        };
        let mut bytes: usize = self.history.iter().map(size).sum();
        // Keep the newest operation undoable even when one very large snapshot
        // exceeds the soft byte budget.
        while self.history.len() > 1 && (self.history.len() > 100 || bytes > 64 * 1024 * 1024) {
            bytes = bytes.saturating_sub(size(&self.history.remove(0)));
        }
        self.cursor = self.history.len();
    }
    pub fn edit(&mut self, name: &str, f: impl FnOnce(&mut Self)) {
        self.commit();
        let before = self.document.clone();
        f(self);
        self.record(name.into(), before);
        self.reconcile();
    }
    fn reconcile(&mut self) {
        if !self.document.pages.iter().any(|p| p.id == self.page) {
            self.page = self.document.pages[0].id;
        }
        let ids: HashSet<_> = self.current().controls.iter().map(|c| c.id).collect();
        self.selection.retain(|id| ids.contains(id));
    }
    pub fn undo(&mut self) {
        self.commit();
        if self.cursor > 0 {
            self.cursor -= 1;
            self.document = self.history[self.cursor].before.clone();
            self.reconcile();
        }
    }
    pub fn redo(&mut self) {
        self.commit();
        if self.cursor < self.history.len() {
            self.document = self.history[self.cursor].after.clone();
            self.cursor += 1;
            self.reconcile();
        }
    }
    pub fn switch_page(&mut self, id: Id) {
        self.commit();
        if self.document.pages.iter().any(|p| p.id == id) {
            self.page = id;
            self.selection.clear();
        }
    }
    pub fn add(&mut self, kind: Kind, x: f32, y: f32) {
        self.edit("Add control", |e| {
            let id = e.document.allocate_id();
            e.current_mut().controls.push(Control::new(id, kind, x, y));
            e.selection = HashSet::from([id]);
        });
    }
    pub fn add_page(&mut self) {
        self.edit("New page", |e| {
            let id = e.document.allocate_id();
            let name = format!("Wireframe {}", e.document.pages.len() + 1);
            e.document.pages.push(Page {
                id,
                name,
                notes: String::new(),
                controls: Vec::new(),
            });
            e.page = id;
            e.selection.clear();
        });
    }
    pub fn delete_page(&mut self) {
        if self.document.pages.len() > 1 {
            self.edit("Delete page", |e| {
                let id = e.page;
                e.document.pages.retain(|p| p.id != id);
                for p in &mut e.document.pages {
                    for c in &mut p.controls {
                        if c.link == Some(id) {
                            c.link = None;
                        }
                    }
                }
                e.page = e.document.pages[0].id;
                e.selection.clear();
            });
        }
    }
    pub fn select(&mut self, id: Id, toggle: bool) {
        let group = self
            .current()
            .controls
            .iter()
            .find(|c| c.id == id)
            .and_then(|c| c.group);
        let related: Vec<_> = self
            .current()
            .controls
            .iter()
            .filter(|c| c.id == id || (group.is_some() && c.group == group))
            .map(|c| c.id)
            .collect();
        let removing = toggle && self.selection.contains(&id);
        if !toggle {
            self.selection.clear();
        }
        for id in related {
            if removing {
                self.selection.remove(&id);
            } else {
                self.selection.insert(id);
            }
        }
    }
    pub fn select_area(&mut self, bounds: Bounds, previous: &HashSet<Id>) {
        let groups: HashSet<_> = self
            .current()
            .controls
            .iter()
            .filter(|c| !c.locked && bounds.intersects(c.bounds))
            .filter_map(|c| c.group)
            .collect();
        self.selection = previous.clone();
        let ids: Vec<_> = self
            .current()
            .controls
            .iter()
            .filter(|c| {
                !c.locked
                    && (bounds.intersects(c.bounds) || c.group.is_some_and(|g| groups.contains(&g)))
            })
            .map(|c| c.id)
            .collect();
        self.selection.extend(ids);
    }
    pub fn select_all(&mut self) {
        self.selection = self
            .current()
            .controls
            .iter()
            .filter(|c| !c.locked)
            .map(|c| c.id)
            .collect();
    }
    pub fn bounds(&self) -> Option<Bounds> {
        self.current()
            .controls
            .iter()
            .filter(|c| self.selection.contains(&c.id))
            .map(|c| c.bounds)
            .reduce(Bounds::union)
    }
    pub fn selected_mut(&mut self, mut f: impl FnMut(&mut Control)) {
        let ids = self.selection.clone();
        for c in &mut self.current_mut().controls {
            if ids.contains(&c.id) && !c.locked {
                f(c);
            }
        }
    }
    pub fn move_by(&mut self, dx: f32, dy: f32) {
        self.edit("Move", |e| {
            e.selected_mut(|c| {
                c.bounds.x = (c.bounds.x + dx).clamp(-50_000., 50_000.);
                c.bounds.y = (c.bounds.y + dy).clamp(-50_000., 50_000.);
            })
        });
    }
    pub fn delete(&mut self) {
        self.edit("Delete controls", |e| {
            let ids = e.selection.clone();
            e.current_mut()
                .controls
                .retain(|c| !ids.contains(&c.id) || c.locked);
            e.selection.clear();
        });
    }
    pub fn copy(&self) -> Vec<Control> {
        self.current()
            .controls
            .iter()
            .filter(|c| self.selection.contains(&c.id))
            .cloned()
            .collect()
    }
    pub fn paste(&mut self, controls: &[Control], offset: f32) {
        self.edit("Paste controls", |e| {
            let mut groups = HashMap::new();
            let mut pasted = Vec::new();
            for c in controls {
                let mut c = c.clone();
                c.id = e.document.allocate_id();
                c.locked = false;
                c.bounds.x += offset;
                c.bounds.y += offset;
                c.group = c
                    .group
                    .map(|g| *groups.entry(g).or_insert_with(|| e.document.allocate_id()));
                if c.link
                    .is_some_and(|id| !e.document.pages.iter().any(|p| p.id == id))
                {
                    c.link = None;
                }
                pasted.push(c);
            }
            e.selection = pasted.iter().map(|c| c.id).collect();
            e.current_mut().controls.extend(pasted);
        });
    }
    pub fn duplicate(&mut self) {
        self.paste(&self.copy(), 20.);
    }
    pub fn group(&mut self) {
        if self.selection.len() > 1 {
            self.edit("Group", |e| {
                let id = e.document.allocate_id();
                e.selected_mut(|c| c.group = Some(id));
            });
        }
    }
    pub fn ungroup(&mut self) {
        self.edit("Ungroup", |e| e.selected_mut(|c| c.group = None));
    }
    pub fn lock(&mut self, locked: bool) {
        self.edit(if locked { "Lock" } else { "Unlock" }, |e| {
            let ids = e.selection.clone();
            for c in &mut e.current_mut().controls {
                if ids.contains(&c.id) {
                    c.locked = locked;
                }
            }
        });
    }
    pub fn reorder(&mut self, front: bool) {
        self.edit("Change layering", |e| {
            let ids = e.selection.clone();
            e.current_mut().controls.sort_by_key(|c| {
                if front {
                    ids.contains(&c.id)
                } else {
                    !ids.contains(&c.id)
                }
            });
        });
    }
    pub fn align(&mut self, axis: usize, edge: usize) {
        if let Some(b) = self.bounds() {
            self.edit("Align", |e| {
                e.selected_mut(|c| {
                    let (start, size, own) = if axis == 0 {
                        (b.x, b.w, c.bounds.w)
                    } else {
                        (b.y, b.h, c.bounds.h)
                    };
                    let value = start
                        + match edge {
                            0 => 0.,
                            1 => (size - own) / 2.,
                            _ => size - own,
                        };
                    if axis == 0 {
                        c.bounds.x = value;
                    } else {
                        c.bounds.y = value;
                    }
                })
            });
        }
    }
    pub fn distribute(&mut self, axis: usize) {
        let mut items: Vec<_> = self
            .current()
            .controls
            .iter()
            .filter(|c| self.selection.contains(&c.id) && !c.locked)
            .map(|c| {
                (
                    c.id,
                    if axis == 0 {
                        c.bounds.x + c.bounds.w / 2.
                    } else {
                        c.bounds.y + c.bounds.h / 2.
                    },
                )
            })
            .collect();
        if items.len() < 3 {
            return;
        }
        items.sort_by(|a, b| a.1.total_cmp(&b.1));
        let start = items[0].1;
        let step = (items.last().unwrap().1 - start) / (items.len() - 1) as f32;
        self.edit("Distribute", |e| {
            e.selected_mut(|c| {
                if let Some(i) = items.iter().position(|v| v.0 == c.id) {
                    if axis == 0 {
                        c.bounds.x = start + i as f32 * step - c.bounds.w / 2.;
                    } else {
                        c.bounds.y = start + i as f32 * step - c.bounds.h / 2.;
                    }
                }
            })
        });
    }
}
