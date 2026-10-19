// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl WireframeApp {
    pub(in crate::app) fn restore_source_navigator_focus(&mut self, id: egui::Id) -> bool {
        if !self.show_navigator || !self.editor.in_source_workspace() {
            return false;
        }
        let Some(row) = self.source_navigation.focus.get(&id).copied() else {
            return false;
        };
        if !self.source_navigation.visible.contains(&row) {
            return false;
        }
        self.source_navigation.keyboard_focus = Some(row);
        true
    }

    pub(super) fn visible_resource_rows(&self, ctx: &egui::Context) -> Vec<ResourceRow> {
        self.editor
            .document
            .live_libraries()
            .flat_map(|library| {
                let open = self
                    .source_navigation
                    .collapse_ids
                    .get(&library.id)
                    .and_then(|id| egui::collapsing_header::CollapsingState::load(ctx, *id))
                    .is_none_or(|state| state.is_open());
                std::iter::once(ResourceRow::Library(library.id)).chain(
                    library
                        .sources
                        .iter()
                        .filter(move |_| open)
                        .map(|s| ResourceRow::Source(s.definition)),
                )
            })
            .collect()
    }

    pub(super) fn focus_resource_keyboard_row(
        &mut self,
        response: &egui::Response,
        row: ResourceRow,
    ) {
        if self.source_navigation.keyboard_focus == Some(row) {
            response.request_focus();
            response.scroll_to_me(Some(egui::Align::Center));
            self.source_navigation.keyboard_focus = None;
        }
    }

    pub(in crate::app) fn source_navigation_key(
        &mut self,
        ctx: &egui::Context,
        key: Key,
        modifiers: Modifiers,
    ) {
        if !modifiers.matches_logically(Modifiers::NONE)
            && !modifiers.matches_logically(Modifiers::SHIFT)
        {
            return;
        }
        let Some(row) = self.focused_resource_row(ctx) else {
            return;
        };
        // egui records directional focus before application shortcuts run.
        // Row navigation owns this key, including on the first focused frame.
        if matches!(
            key,
            Key::ArrowUp | Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight
        ) {
            ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
        }
        match key {
            Key::ArrowUp | Key::ArrowDown => {
                let rows = self.visible_resource_rows(ctx);
                let Some(index) = rows.iter().position(|r| *r == row) else {
                    return;
                };
                let next = if key == Key::ArrowUp {
                    index.saturating_sub(1)
                } else {
                    (index + 1).min(rows.len() - 1)
                };
                if next == index || !self.finish_page_notes() {
                    return;
                }
                self.end_pointer_gestures();
                self.source_navigation.visible = rows.clone();
                match self.select_resource_row(rows[next], modifiers) {
                    Ok(()) => {
                        self.source_navigation.focus_to = None;
                        self.source_navigation.keyboard_focus = Some(rows[next]);
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            Key::ArrowLeft | Key::ArrowRight if !modifiers.shift => {
                if let ResourceRow::Library(library) = row
                    && let Some(id) = self.source_navigation.collapse_ids.get(&library)
                    && let Some(mut state) =
                        egui::collapsing_header::CollapsingState::load(ctx, *id)
                {
                    state.set_open(key == Key::ArrowRight);
                    state.store(ctx);
                    self.source_navigation.visible = self.visible_resource_rows(ctx);
                }
            }
            // Horizontal keys on sources and Return retain the current resource.
            _ => {}
        }
    }
}
