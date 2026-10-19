// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::page_tree::PageTree;

#[derive(Default)]
pub(crate) struct Navigator {
    pub(crate) drag: Option<super::drag::PageDrag>,
    pub(crate) press: Option<super::drag::PagePress>,
    pub(crate) scroll_to: Option<Id>,
    pub(crate) focus_to: Option<Id>,
    pub(crate) focus_ids: HashSet<egui::Id>,
    pub(crate) selected: HashSet<Id>,
    pub(crate) collapsed: HashSet<Id>,
    current: Option<Id>,
    live: HashSet<Id>,
}

impl WireframeApp {
    pub(in crate::app) fn page_command_enabled(
        &self,
        ctx: &egui::Context,
        command: menus::Command,
    ) -> Option<bool> {
        if self.editor.in_source_workspace()
            || !self.show_navigator
            || self
                .command_navigation_focus(ctx)
                .is_none_or(|id| !self.navigator.focus_ids.contains(&id))
        {
            return None;
        }
        use menus::Command::*;
        match command {
            AlignLeft | AlignCenter | AlignRight | AlignTop | AlignMiddle | AlignBottom
            | DistributeHorizontal | DistributeVertical | SpaceHorizontal | SpaceVertical
            | AutoSize => Some(false),
            SelectAll | Duplicate | Delete | Cut | Copy | Paste => Some(true),
            PasteControls => Some(self.clipboard.pages.is_some()),
            Group | Ungroup | SendToBack | BringToFront | SendBackward | BringForward => {
                Some(false)
            }
            _ => None,
        }
    }

    pub(in crate::app) fn run_page_command(
        &mut self,
        ctx: &egui::Context,
        command: menus::Command,
    ) -> bool {
        let Some(enabled) = self.page_command_enabled(ctx, command) else {
            return false;
        };
        if !enabled {
            return true;
        }
        let tree = PageTree::new(&self.editor.document);
        self.navigator.sync(
            &tree,
            self.editor.page,
            self.editor.duplicated_page_selection(),
        );
        let targets: Vec<_> = tree
            .rows
            .iter()
            .filter(|r| self.navigator.selected.contains(&r.id))
            .map(|r| r.id)
            .collect();
        self.end_pointer_gestures();
        match command {
            menus::Command::Copy | menus::Command::Cut => {
                self.copy_page_resources(ctx, &targets, command == menus::Command::Cut);
            }
            menus::Command::Paste => self.request_page_paste(ctx, &targets),
            menus::Command::PasteControls => self.paste_clipboard(&self.clipboard.clone()),
            menus::Command::SelectAll => {
                self.navigator.selected = tree.rows.iter().map(|r| r.id).collect();
            }
            menus::Command::Duplicate => {
                if let Err(error) = self.editor.duplicate_pages(&targets) {
                    self.error = Some(error);
                }
            }
            menus::Command::Delete => {
                if let Err(error) = self.editor.trash_pages(&targets) {
                    self.error = Some(error);
                }
            }
            _ => unreachable!(),
        }
        true
    }
}

impl Navigator {
    pub(crate) fn restore(&mut self, tree: &PageTree, current: Id, collapsed: HashSet<Id>) {
        *self = Self {
            collapsed,
            ..Self::default()
        };
        self.sync(tree, current, vec![]);
        // A restored closed current parent owns its complete operation scope.
        self.selected = self.scope(tree, current).into_iter().collect();
    }
    pub(crate) fn navigate(&mut self, tree: &PageTree, current: Id, key: Key, shift: bool) -> Id {
        let rows = tree.visible(&self.collapsed);
        let Some(anchor) = rows.iter().position(|r| r.id == current) else {
            return current;
        };
        if matches!(key, Key::ArrowLeft | Key::ArrowRight) {
            let row = rows[anchor];
            let closed = self.collapsed.contains(&current);
            if row.has_children && closed == (key == Key::ArrowRight) {
                self.toggle(tree, current, current);
            }
            return current;
        }
        let mut edge = anchor;
        if shift {
            let selected: Vec<_> = rows
                .iter()
                .enumerate()
                .filter(|(_, r)| self.selected.contains(&r.id))
                .map(|(i, _)| i)
                .collect();
            if selected.windows(2).all(|w| w[1] == w[0] + 1) {
                if selected.first() == Some(&anchor) {
                    edge = *selected.last().unwrap();
                } else if selected.last() == Some(&anchor) {
                    edge = *selected.first().unwrap();
                }
            }
        }
        let next = if matches!(key, Key::ArrowUp | Key::PageUp) {
            edge.saturating_sub(1)
        } else {
            (edge + 1).min(rows.len() - 1)
        };
        if next == edge {
            return current;
        }
        let target = rows[next].id;
        self.focus_to = Some(target);
        self.select(
            tree,
            target,
            current,
            if shift {
                Modifiers::SHIFT
            } else {
                Modifiers::NONE
            },
        )
    }
    pub(crate) fn focused(&self, ctx: &egui::Context) -> bool {
        ctx.memory(|m| m.focused())
            .is_some_and(|id| self.focus_ids.contains(&id))
    }
    pub(crate) fn sync(&mut self, tree: &PageTree, current: Id, duplicates: Vec<Id>) -> bool {
        let current_changed = self.current != Some(current);
        self.collapsed
            .retain(|id| tree.row(*id).is_some_and(|r| r.has_children));
        self.selected.retain(|id| tree.row(*id).is_some());
        if current_changed || self.selected.is_empty() {
            self.scroll_to = Some(current);
            self.selected = if !self.live.contains(&current) && duplicates.contains(&current) {
                duplicates.into_iter().collect()
            } else {
                HashSet::from([current])
            };
            for id in self.selected.clone() {
                self.reveal(tree, id);
            }
        }
        self.current = Some(current);
        self.live = tree.rows.iter().map(|r| r.id).collect();
        current_changed
    }

    pub(in crate::app) fn reveal(&mut self, tree: &PageTree, id: Id) {
        let mut parent = tree.row(id).and_then(|r| r.parent);
        while let Some(id) = parent {
            self.collapsed.remove(&id);
            parent = tree.row(id).and_then(|r| r.parent);
        }
    }

    pub(crate) fn scope(&self, tree: &PageTree, id: Id) -> Vec<Id> {
        if self.collapsed.contains(&id) {
            tree.subtree(id).iter().map(|r| r.id).collect()
        } else if tree.row(id).is_some() {
            vec![id]
        } else {
            vec![]
        }
    }

    pub(crate) fn context(&self, tree: &PageTree, id: Id) -> Vec<Id> {
        if self.collapsed.contains(&id) && !self.selected.contains(&id) {
            self.scope(tree, id)
        } else if self.selected.contains(&id) {
            tree.rows
                .iter()
                .filter(|r| self.selected.contains(&r.id))
                .map(|r| r.id)
                .collect()
        } else {
            self.scope(tree, id)
        }
    }

    pub(crate) fn select(
        &mut self,
        tree: &PageTree,
        id: Id,
        current: Id,
        modifiers: Modifiers,
    ) -> Id {
        self.scroll_to = Some(id);
        let mut active = current;
        if modifiers.shift {
            let rows = tree.visible(&self.collapsed);
            let from = rows.iter().position(|r| r.id == current).unwrap_or(0);
            let to = rows.iter().position(|r| r.id == id).unwrap_or(from);
            self.selected = rows[from.min(to)..=from.max(to)]
                .iter()
                .flat_map(|r| self.scope(tree, r.id))
                .collect();
        } else if modifiers.command {
            let scope = self.scope(tree, id);
            if scope.iter().all(|id| self.selected.contains(id)) {
                for id in scope {
                    self.selected.remove(&id);
                }
            } else {
                self.selected.extend(scope);
            }
            if !self.selected.contains(&current) {
                active = tree
                    .rows
                    .iter()
                    .find(|r| self.selected.contains(&r.id))
                    .map_or(current, |r| r.id);
            }
        } else {
            self.selected = self.scope(tree, id).into_iter().collect();
            active = id;
        }
        self.current = Some(active);
        if active != current {
            self.scroll_to = Some(active);
        }
        active
    }

    pub(crate) fn toggle(&mut self, tree: &PageTree, id: Id, current: Id) -> Id {
        if !self.collapsed.remove(&id) {
            self.collapsed.insert(id);
        }
        let active = if tree.subtree(id).iter().any(|r| r.id == current) {
            id
        } else {
            current
        };
        self.selected = if active == id {
            self.scope(tree, id).into_iter().collect()
        } else {
            HashSet::from([active])
        };
        self.current = Some(active);
        active
    }
}
