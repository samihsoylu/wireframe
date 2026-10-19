// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::collections::{BTreeMap, BTreeSet};
use wireframe::review::{Author, DeletedBody, Discussion, Draft};
mod identity;
mod interface;
mod overlay;
#[cfg(test)]
mod tests;
mod text;

#[derive(Default)]
pub(super) struct Review {
    pub open: bool,
    identity: identity::Identity,
    surfaces: BTreeMap<Id, Surface>,
    drag: Option<MarkerDrag>,
    generation: u64,
}
#[derive(Default)]
struct Surface {
    observed: Discussion,
    draft: Option<Composer>,
    deleted: BTreeMap<Id, DeletedBody>,
    selected: Option<Id>,
    error: Option<String>,
}
impl Surface {
    fn marker_names(&self) -> BTreeMap<Id, String> {
        let draft = self.draft.as_ref().filter(|d| !d.stale).map(|d| &d.draft);
        let markers: BTreeMap<_, _> = self
            .observed
            .comments
            .values()
            .filter(|c| !c.deleted && draft.is_none_or(|d| d.comment != Some(c.id)))
            .flat_map(|c| &c.body.callouts)
            .chain(draft.into_iter().flat_map(|d| &d.body.callouts))
            .map(|m| (m.id, m.label))
            .collect();
        let mut counts = BTreeMap::<u64, usize>::new();
        for label in markers.values() {
            *counts.entry(*label).or_default() += 1;
        }
        let mut ordinals = BTreeMap::<u64, usize>::new();
        markers
            .into_iter()
            .map(|(id, label)| {
                let index = ordinals.entry(label).or_default();
                *index += 1;
                (
                    id,
                    if counts[&label] > 1 {
                        format!("{label}.{index}")
                    } else {
                        label.to_string()
                    },
                )
            })
            .collect()
    }
}
struct Composer {
    #[cfg(test)]
    caret_visible: bool,
    input: egui::Id,
    draft: Draft,
    text: text::Text,
    caret: usize,
    focus: bool,
    stale: bool,
    callout_settings: bool,
}
struct MarkerDrag {
    surface: Id,
    marker: Id,
    start: Pos2,
    original: wireframe::document::Point,
}
impl Review {
    pub fn reset(&mut self) {
        self.surfaces.clear();
        self.drag = None;
    }
}
impl WireframeApp {
    pub(super) fn toggle_review_inspector(&mut self) {
        if self.review.open {
            self.review.open = false;
            self.show_inspector = true;
        } else {
            self.show_inspector = !self.show_inspector;
        }
    }
    pub(super) fn load_reviewer(&mut self, root: &std::path::Path) {
        self.review.identity.load(root);
    }
    pub(super) fn sync_reviews(&mut self) {
        for (&id, state) in &mut self.review.surfaces {
            let discussion = self
                .editor
                .document
                .reviews
                .get(&id)
                .cloned()
                .unwrap_or_default();
            if state.observed != discussion || !self.editor.document.live_review_surface(id) {
                state.observed = discussion;
                state.deleted.clear();
                state.selected = None;
                if let Some(composer) = &mut state.draft {
                    composer.stale = true;
                }
                if self.review.drag.as_ref().is_some_and(|d| d.surface == id) {
                    self.review.drag = None;
                }
            }
        }
    }
    fn review_surface(&mut self) -> &mut Surface {
        let id = self.editor.surface_id();
        self.review.surfaces.entry(id).or_insert_with(|| Surface {
            observed: self
                .editor
                .document
                .reviews
                .get(&id)
                .cloned()
                .unwrap_or_default(),
            ..Default::default()
        })
    }
    fn change_review<T>(
        &mut self,
        change: impl FnOnce(&mut Discussion) -> Result<T, String>,
    ) -> Option<T> {
        let surface = self.editor.surface_id();
        match self.editor.change_review(surface, change) {
            Ok(value) => {
                let discussion = self
                    .editor
                    .document
                    .reviews
                    .get(&surface)
                    .cloned()
                    .unwrap_or_default();
                self.review_surface().observed = discussion;
                self.review_surface().error = None;
                Some(value)
            }
            Err(error) => {
                self.review_surface().error = Some(error);
                None
            }
        }
    }
    fn begin_review(&mut self, parent: Option<Id>, edit: Option<Id>) {
        if !self.finish_page_notes() {
            return;
        }
        self.end_pointer_gestures();
        if self.review_surface().draft.is_some() {
            self.review_surface().error =
                Some("Post or cancel the current draft before starting another comment.".into());
            return;
        }
        let discussion = &self.review_surface().observed.clone();
        let draft = if let Some(id) = edit {
            Draft::edit(discussion, id, &self.review.identity.author.id)
        } else {
            Draft::new(discussion, parent)
        };
        let names = self.review_surface().marker_names();
        self.review.generation += 1;
        let input = egui::Id::new(("review-composer", self.review.generation));
        let result = draft.and_then(|draft| {
            let mut text = text::Text::default();
            for (id, label) in names {
                text.alias(&label, &format!("callout:{id}"));
            }
            text.show_source(&draft.body.text)?;
            Ok(Composer {
                #[cfg(test)]
                caret_visible: false,
                input,
                caret: text.value.len(),
                draft,
                text,
                focus: true,
                stale: false,
                callout_settings: false,
            })
        });
        match result {
            Ok(draft) => {
                let state = self.review_surface();
                state.draft = Some(draft);
                state.selected = None;
                state.error = None;
            }
            Err(error) => self.review_surface().error = Some(error),
        }
    }
    fn post_review(&mut self) {
        let Some(composer) = &mut self.review_surface().draft else {
            return;
        };
        if composer.stale {
            return;
        }
        match composer.text.encode(composer.caret) {
            Ok((text, _)) => composer.draft.body.text = text,
            Err(error) => {
                self.review_surface().error = Some(error);
                return;
            }
        }
        let draft = composer.draft.clone();
        let author = match self.review.identity.save() {
            Ok(author) => author,
            Err(error) => {
                self.review_surface().error = Some(error);
                return;
            }
        };
        let now = chrono::Utc::now().timestamp_millis().max(0) as u64;
        if self
            .change_review(|discussion| draft.commit(discussion, author, now))
            .is_some()
        {
            self.cancel_review();
        }
    }
    fn cancel_review(&mut self) {
        let state = self.review_surface();
        state.draft = None;
        state.selected = None;
        state.error = None;
        self.review.drag = None;
    }
    pub(super) fn review_focused(&self, ctx: &egui::Context) -> bool {
        self.reviews_visible() && ctx.memory(|m| m.focused()) == Some(self.review_input_id())
    }
    fn review_input_id(&self) -> egui::Id {
        self.review
            .surfaces
            .get(&self.editor.surface_id())
            .and_then(|s| s.draft.as_ref())
            .map_or(egui::Id::NULL, |d| d.input)
    }
    fn reviews_visible(&self) -> bool {
        !self.editor.in_source_workspace()
            && if self.present {
                self.presentation_view.session.settings().comments
            } else {
                self.review.open
            }
    }
    pub(super) fn review_shortcut(&mut self, ctx: &egui::Context, event: &egui::Event) -> bool {
        if !self.review_focused(ctx) || self.canvas_input_blocked() {
            return false;
        }
        if let egui::Event::Key {
            key: Key::Enter,
            modifiers,
            pressed: true,
            ..
        } = event
            && modifiers.matches_logically(Modifiers::COMMAND)
        {
            self.post_review();
            return true;
        }
        // Escape leaves the field, retaining its text. Cancel is explicit.
        if matches!(event, egui::Event::Key { key: Key::Escape, pressed: true, modifiers, .. } if modifiers.is_none())
        {
            ctx.memory_mut(|m| m.surrender_focus(self.review_input_id()));
            return true;
        }
        false
    }
    fn review_link(&mut self, target: &str) {
        if let Some(id) = target
            .strip_prefix("callout:")
            .and_then(|s| s.parse::<Id>().ok())
        {
            let state = self.review_surface();
            let marker = state
                .draft
                .as_ref()
                .and_then(|d| d.draft.body.callouts.iter().find(|m| m.id == id))
                .or_else(|| state.observed.callout(id))
                .cloned();
            if let Some(marker) = marker {
                state.selected = Some(id);
                let pos = self.screen([marker.position.x, marker.position.y]);
                if !self.canvas_rect.shrink(40.).contains(pos) {
                    self.pan += self.canvas_rect.center() - pos;
                }
            } else {
                state.error = Some("This callout is no longer available.".into());
            }
        } else if let Some(id) = target
            .strip_prefix("surface:")
            .and_then(|s| s.parse::<Id>().ok())
        {
            if self.editor.document.live_review_surface(id) {
                self.navigate_review_surface(id);
            } else {
                self.review_surface().error =
                    Some("This page or alternate is no longer available.".into());
            }
        }
    }
}
