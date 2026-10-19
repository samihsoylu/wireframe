// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
mod hints;
mod interface;
mod navigator;
pub(super) use interface::State;
use wireframe::presentation::{Aid, Preset, Zoom};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Destination {
    page: Id,
    alternate: Option<Id>,
}

#[derive(Default)]
pub(super) struct History {
    entries: Vec<Destination>,
    cursor: usize,
}
impl History {
    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.cursor = 0;
    }
    fn record(&mut self, from: Destination, to: Destination) {
        if self.entries.is_empty() {
            self.entries.push(from);
            self.cursor = 0;
        }
        self.entries.truncate(self.cursor + 1);
        self.entries.push(to);
        self.cursor += 1;
    }
}

impl WireframeApp {
    fn presentation_destination(&self) -> Destination {
        Destination {
            page: self.editor.page,
            alternate: self.editor.active_alternate(self.editor.page),
        }
    }
    pub(super) fn start_presentation(&mut self) {
        if self.editor.in_source_workspace() {
            return;
        }
        if !self.finish_page_notes() {
            return;
        }
        self.initialize_presentation();
        self.sync_viewport();
        self.end_pointer_gestures();
        self.presentation_history.clear();
        self.presentation_history
            .entries
            .push(self.presentation_destination());
        self.present = true;
        self.sync_viewport();
    }
    fn visit_destination(&mut self, destination: Destination) {
        if !self.finish_page_notes() {
            return;
        }
        let from = self.presentation_destination();
        if destination == from {
            return;
        }
        match self
            .editor
            .switch_version(destination.page, destination.alternate)
        {
            Ok(()) => self.presentation_history.record(from, destination),
            Err(error) => self.status = error,
        }
    }
    pub(super) fn navigate_review_surface(&mut self, id: Id) {
        let destination = self.editor.document.live_pages().find_map(|page| {
            if page.id == id {
                Some(Destination {
                    page: id,
                    alternate: None,
                })
            } else if page.alternates.iter().any(|a| a.id == id) {
                Some(Destination {
                    page: page.id,
                    alternate: Some(id),
                })
            } else {
                None
            }
        });
        if let Some(destination) = destination {
            self.visit_destination(destination);
        } else {
            self.status = "This page or alternate is no longer available.".into();
        }
    }
    pub(super) fn navigate_page(&mut self, id: Id) {
        self.visit_destination(Destination {
            page: id,
            alternate: self.editor.active_alternate(id),
        });
    }
    pub(super) fn history_destination(&self, forward: bool) -> Option<(usize, Destination)> {
        let mut index = self.presentation_history.cursor;
        loop {
            index = if forward {
                index.checked_add(1)?
            } else {
                index.checked_sub(1)?
            };
            let mut destination = *self.presentation_history.entries.get(index)?;
            if let Some(page) = self
                .editor
                .document
                .live_pages()
                .find(|p| p.id == destination.page)
            {
                destination.alternate = destination
                    .alternate
                    .filter(|id| page.alternates.iter().any(|a| a.id == *id));
                if destination != self.presentation_destination() {
                    return Some((index, destination));
                }
            }
        }
    }
    fn navigate_history(&mut self, forward: bool) {
        if !self.finish_page_notes() {
            return;
        }
        if let Some((index, destination)) = self.history_destination(forward) {
            match self
                .editor
                .switch_version(destination.page, destination.alternate)
            {
                Ok(()) => {
                    self.presentation_history.cursor = index;
                    self.presentation_history.entries[index] = destination;
                }
                Err(error) => self.status = error,
            }
        }
    }
    fn adjacent_destination(&self, forward: bool, versions: bool) -> Option<Destination> {
        let pages: Vec<_> = self.editor.document.live_pages().collect();
        let index = pages.iter().position(|p| p.id == self.editor.page)?;
        let page = &pages[index];
        if versions {
            let version = self
                .editor
                .active_alternate(page.id)
                .and_then(|id| page.alternates.iter().position(|a| a.id == id))
                .map_or(0, |i| i + 1);
            if forward && version < page.alternates.len() {
                return Some(Destination {
                    page: page.id,
                    alternate: Some(page.alternates[version].id),
                });
            }
            if !forward && version > 0 {
                return Some(Destination {
                    page: page.id,
                    alternate: version.checked_sub(2).map(|i| page.alternates[i].id),
                });
            }
        }
        let next = if forward {
            index.checked_add(1)?
        } else {
            index.checked_sub(1)?
        };
        let page = pages.get(next)?;
        Some(Destination {
            page: page.id,
            alternate: if versions {
                if forward {
                    None
                } else {
                    page.alternates.last().map(|a| a.id)
                }
            } else {
                self.editor.active_alternate(page.id)
            },
        })
    }
    fn navigate_adjacent(&mut self, forward: bool, versions: bool) {
        if let Some(destination) = self.adjacent_destination(forward, versions) {
            self.visit_destination(destination);
        }
    }
    pub(super) fn follow_target(&mut self, target: &str, ctx: &egui::Context) {
        if let Some(id) = target
            .strip_prefix("page:")
            .and_then(|s| s.parse::<Id>().ok())
        {
            self.navigate_page(id);
        } else if target == "back:" {
            self.navigate_history(false);
        } else if target.starts_with("https://") || target.starts_with("http://") {
            ctx.open_url(egui::OpenUrl::new_tab(target));
        } else {
            self.status = "This link destination is not available".into();
        }
    }
    pub(super) fn presentation_shortcut(
        &mut self,
        ctx: &egui::Context,
        key: Key,
        modifiers: Modifiers,
    ) -> bool {
        if self.presentation_view.settings_open {
            if modifiers.is_none() && matches!(key, Key::Escape | Key::Enter | Key::S) {
                if self.presentation_view.exclusions.take().is_none() {
                    self.presentation_view.settings_open = false;
                }
                return true;
            }
            return false;
        }
        // Shortcuts run before menus are drawn. Persistent popup memory covers
        // menus that have not registered themselves in the current UI pass yet.
        if wireframe::text_entry::is_focused(ctx)
            || ctx.text_edit_focused()
            || egui::Popup::is_any_open(ctx)
            || ctx.any_popup_open()
        {
            return false;
        }
        if modifiers.matches_logically(Modifiers::NONE) && key == Key::Escape {
            if self.finish_page_notes() {
                self.present = false;
            }
            return true;
        }
        if modifiers.is_none() {
            let aid = match key {
                Key::J => Some(Aid::Navigator),
                Key::N => Some(Aid::Notes),
                Key::C => Some(Aid::Comments),
                Key::L => Some(Aid::LinkHints),
                Key::B => Some(Aid::BigCursor),
                Key::M => Some(Aid::Markup),
                _ => None,
            };
            if let Some(aid) = aid {
                self.presentation_view.session.toggle(aid);
                return true;
            }
            match key {
                Key::R => {
                    self.presentation_view.session.select(Preset::Review);
                    return true;
                }
                Key::U => {
                    self.presentation_view.session.select(Preset::UserTest);
                    return true;
                }
                Key::Z => {
                    self.presentation_view.session.toggle_zoom();
                    return true;
                }
                Key::S => {
                    self.presentation_view.settings_open = true;
                    return true;
                }
                _ => {}
            }
        }
        if (modifiers.matches_logically(Modifiers::ALT)
            || modifiers.matches_logically(Modifiers::NONE))
            && matches!(key, Key::ArrowLeft | Key::ArrowRight)
        {
            self.navigate_history(key == Key::ArrowRight);
            return true;
        }
        let versions = modifiers.matches_logically(Modifiers::ALT | Modifiers::SHIFT);
        if (versions || modifiers.matches_logically(Modifiers::ALT))
            && matches!(key, Key::ArrowUp | Key::ArrowDown)
        {
            self.navigate_adjacent(key == Key::ArrowDown, versions);
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests;
