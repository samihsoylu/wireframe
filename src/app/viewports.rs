// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
mod preferences;
#[cfg(test)]
mod scroll_tests;
#[cfg(test)]
mod tests;

use super::*;
use std::collections::BTreeMap;
use wireframe::viewport::{Gesture, View, next_zoom};

#[derive(Default)]
pub(super) struct Viewports {
    editor: BTreeMap<Id, View>,
    presentation: BTreeMap<Id, View>,
    active: Option<(Id, bool)>,
    gesture: Gesture,
    input_frame: Option<u64>,
    pointer: Option<Pos2>,
    input_start: Option<Pos2>,
    events: Option<Vec<egui::Event>>,
    preferences: preferences::Preferences,
    pub(super) canvas_layer: Option<egui::LayerId>,
    scroll: egui::InputState,
    scroll_events: Vec<egui::Event>,
    touch_canvas: Option<bool>,
}

impl WireframeApp {
    /// Route before egui combines events or smooths them. The canvas has its
    /// own ordinary-wheel state, so residuals cannot migrate between panels.
    pub(super) fn prepare_viewport_input(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.viewports.input_start = self.viewports.pointer;
        self.viewports.events = Some(
            raw.events
                .iter()
                .filter(|event| match event {
                    egui::Event::PointerMoved(_)
                    | egui::Event::PointerGone
                    | egui::Event::WindowFocused(false)
                    | egui::Event::Zoom(_)
                    | egui::Event::SmartZoom => true,
                    egui::Event::MouseWheel { modifiers, .. } => modifiers.command,
                    _ => false,
                })
                .cloned()
                .collect(),
        );
        self.viewports.scroll_events.clear();
        let blocked = self.canvas_input_blocked() || !raw.focused;
        if blocked {
            self.reset_canvas_scroll();
        }
        let mut pointer = self.viewports.pointer;
        let mut focused = raw.focused;
        raw.events.retain(|event| {
            match event {
                egui::Event::PointerMoved(pos) => pointer = Some(*pos),
                egui::Event::PointerGone => pointer = None,
                egui::Event::WindowFocused(value) => {
                    focused = *value;
                    if !focused {
                        self.viewports.gesture.reset();
                        self.reset_canvas_scroll();
                        pointer = None;
                    }
                }
                egui::Event::MouseWheel {
                    modifiers, phase, ..
                } => {
                    if modifiers.command {
                        // Ending a plain touch stream must release its owner
                        // even when Command was pressed during the gesture.
                        if matches!(phase, egui::TouchPhase::End | egui::TouchPhase::Cancel) {
                            match self.viewports.touch_canvas.take() {
                                Some(true) => self.viewports.scroll_events.push(event.clone()),
                                Some(false) => return true,
                                None => {}
                            }
                        }
                        return false;
                    }
                    let hit = !blocked
                        && focused
                        && pointer.is_some_and(|p| self.canvas_input_target(ctx, p));
                    let canvas = match phase {
                        egui::TouchPhase::Start => {
                            self.viewports.touch_canvas = Some(hit);
                            hit
                        }
                        egui::TouchPhase::Move => self.viewports.touch_canvas.unwrap_or(hit),
                        egui::TouchPhase::End | egui::TouchPhase::Cancel => {
                            self.viewports.touch_canvas.take().unwrap_or(hit)
                        }
                    };
                    if canvas && !blocked && focused {
                        self.viewports.scroll_events.push(event.clone());
                        return false;
                    }
                }
                _ => {}
            }
            true
        });
        self.viewports.pointer = pointer;
    }

    fn canvas_input_target(&self, ctx: &egui::Context, pointer: Pos2) -> bool {
        self.canvas_rect.contains(pointer)
            && self
                .viewports
                .canvas_layer
                .is_none_or(|layer| ctx.layer_id_at(pointer) == Some(layer))
    }

    pub(super) fn reset_canvas_scroll(&mut self) {
        self.viewports.scroll = egui::InputState::default();
        self.viewports.scroll_events.clear();
        self.viewports.touch_canvas = None;
    }

    fn canvas_scroll_delta(&mut self, ctx: &egui::Context) -> Vec2 {
        // Use the same timing, viewport units and options as the main input
        // state, but feed only events owned by the canvas. No native curve is
        // reimplemented here, and no shared ScrollArea can consume this queue.
        let mut raw = ctx.input(|i| egui::RawInput {
            time: Some(i.time),
            predicted_dt: i.stable_dt,
            screen_rect: Some(i.viewport_rect()),
            focused: i.focused,
            ..Default::default()
        });
        raw.events = std::mem::take(&mut self.viewports.scroll_events);
        self.viewports.scroll = std::mem::take(&mut self.viewports.scroll).begin_pass(
            raw,
            false,
            ctx.pixels_per_point(),
            ctx.options(|o| o.input_options),
        );
        let delta = self.viewports.scroll.smooth_scroll_delta;
        if delta != Vec2::ZERO {
            ctx.request_repaint();
        }
        delta
    }

    fn viewport(&self) -> View {
        View {
            zoom: self.zoom,
            scroll: [-self.pan.x, -self.pan.y],
        }
    }

    fn set_viewport(&mut self, view: View) {
        self.zoom = view.zoom;
        self.pan = -Vec2::from(view.scroll);
    }

    /// Keep the existing rendering transform synchronized at UI action boundaries.
    /// Surface IDs remain stable across renames, moves, content replacement and Undo.
    pub(super) fn sync_viewport(&mut self) {
        let current = self.viewport();
        if let Some((id, presenting)) = self.viewports.active {
            let views = if presenting {
                &mut self.viewports.presentation
            } else {
                &mut self.viewports.editor
            };
            views.insert(id, current);
        }
        let owner = (self.editor.surface_id(), self.present);
        if self.viewports.active != Some(owner) {
            self.viewports.gesture.reset();
            self.reset_canvas_scroll();
            let view = if self.present {
                *self
                    .viewports
                    .presentation
                    .entry(owner.0)
                    .or_insert_with(|| {
                        self.viewports
                            .editor
                            .get(&owner.0)
                            .copied()
                            .unwrap_or_default()
                    })
            } else {
                if self
                    .viewports
                    .active
                    .is_some_and(|(_, presenting)| presenting)
                {
                    self.viewports.presentation.clear();
                }
                self.viewports
                    .editor
                    .get(&owner.0)
                    .copied()
                    .unwrap_or_else(|| {
                        if self.viewports.active.is_none() {
                            current
                        } else {
                            View::default()
                        }
                    })
            };
            self.viewports.active = Some(owner);
            self.set_viewport(view);
        }
    }

    pub(super) fn restore_viewports(&mut self) {
        let markup_visible = self
            .version_preferences_key()
            .is_none_or(|key| self.viewports.preferences.markup_visible(&key));
        self.editor.set_markup_visible(markup_visible);
        self.viewports.editor = self
            .version_preferences_key()
            .map(|key| self.viewports.preferences.get(&key))
            .unwrap_or_default();
        self.viewports.presentation.clear();
        self.viewports.active = None;
        self.present = false;
        self.set_viewport(View::default());
        self.sync_viewport();
    }

    pub(super) fn load_viewport_preferences(&mut self, root: &std::path::Path) {
        if let Err(error) = self.viewports.preferences.load(root) {
            self.error = Some(format!(
                "Saved canvas views couldn't be read. The settings file is unchanged.\n{error}"
            ));
        }
    }

    pub(super) fn remember_viewports(&mut self, force: bool) {
        self.sync_viewport();
        let now = Instant::now();
        if let Some(key) = self.version_preferences_key() {
            self.viewports.preferences.remember(
                key,
                &self.viewports.editor,
                self.editor.markup_visible(),
                now,
            );
        }
        if let Err(error) = self.viewports.preferences.flush(now, force) {
            self.status = format!("Canvas views are retained for this session: {error}");
        }
    }

    pub(super) fn zoom_step(&mut self, increase: bool) {
        self.sync_viewport();
        if let Some(zoom) = next_zoom(self.zoom, increase) {
            let selection = if self.present {
                None
            } else {
                self.editor.movable_bounds()
            };
            let view = self
                .viewport()
                .centered(zoom, self.canvas_rect.size().into(), selection);
            self.viewports.gesture.reset();
            self.set_viewport(view);
        }
    }

    pub(super) fn actual_size(&mut self) {
        self.sync_viewport();
        let view = self
            .viewport()
            .centered(1., self.canvas_rect.size().into(), None);
        self.viewports.gesture.reset();
        self.set_viewport(view);
    }

    fn smart_zoom(&mut self) {
        self.sync_viewport();
        if self.zoom == 1. {
            self.fit();
        } else {
            self.actual_size();
        }
    }

    fn fitted_view(&self, id: Id) -> Result<View, String> {
        let bounds = self
            .editor
            .visible_editor_layers(id)?
            .iter()
            .map(|layer| {
                layer.clip.map_or(layer.control.bounds, |clip| {
                    wireframe::render::intersect(layer.control.bounds, clip)
                })
            })
            .reduce(Bounds::union);
        Ok(bounds.map_or_else(View::default, |bounds| {
            View::fit(bounds, self.canvas_rect.size().into())
        }))
    }

    pub(super) fn fit(&mut self) {
        self.sync_viewport();
        match self.fitted_view(self.editor.surface_id()) {
            Ok(view) => {
                self.viewports.gesture.reset();
                self.set_viewport(view);
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub(super) fn zoom_all(&mut self, fit: bool) {
        self.sync_viewport();
        self.end_pointer_gestures();
        let ids: Vec<_> = if self.editor.definition.is_some() {
            self.editor
                .document
                .live_definitions()
                .map(|d| d.id)
                .collect()
        } else {
            self.editor
                .document
                .live_pages()
                .flat_map(|p| std::iter::once(p.id).chain(p.alternates.iter().map(|a| a.id)))
                .collect()
        };
        // Resolve the entire operation before changing any view.
        let result: Result<BTreeMap<_, _>, String> = ids
            .into_iter()
            .map(|id| {
                let view = if fit {
                    self.fitted_view(id)?
                } else {
                    View::default()
                };
                Ok((id, view))
            })
            .collect();
        match result {
            Ok(views) => {
                if self.editor.definition.is_none() {
                    while !self.editor.context.is_empty() {
                        self.editor.leave();
                    }
                }
                if !fit {
                    self.editor.selection.clear();
                }
                self.viewports.gesture.reset();
                let active = views
                    .get(&self.editor.surface_id())
                    .copied()
                    .unwrap_or_default();
                if self.present {
                    self.viewports.presentation.extend(views);
                } else {
                    self.viewports.editor.extend(views);
                }
                self.set_viewport(active);
                self.remember_viewports(true);
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub(super) fn viewport_input(&mut self, ctx: &egui::Context) {
        let frame = ctx.cumulative_frame_nr();
        if self.viewports.input_frame == Some(frame) {
            return;
        }
        self.viewports.input_frame = Some(frame);
        let (events, time, pointer) =
            ctx.input(|i| (i.events.clone(), i.time, i.pointer.hover_pos()));
        let prepared = self.viewports.events.take();
        let mut pointer = if prepared.is_some() {
            self.viewports.input_start
        } else {
            pointer
        };
        let events = prepared.unwrap_or(events);
        for event in events {
            let delta = match event {
                egui::Event::SmartZoom => {
                    if pointer.is_some_and(|p| self.canvas_input_target(ctx, p)) {
                        self.smart_zoom();
                    }
                    continue;
                }
                egui::Event::PointerMoved(pos) => {
                    pointer = Some(pos);
                    continue;
                }
                egui::Event::PointerGone | egui::Event::WindowFocused(false) => {
                    pointer = None;
                    continue;
                }
                egui::Event::MouseWheel {
                    delta, modifiers, ..
                } if modifiers.command => {
                    if delta.y > 0. {
                        0.06
                    } else if delta.y < 0. {
                        -0.06
                    } else {
                        0.
                    }
                }
                egui::Event::Zoom(factor) if factor.is_finite() && factor > 0. => {
                    f64::from(factor).ln()
                }
                _ => continue,
            };
            let Some(pointer) = pointer.filter(|p| self.canvas_input_target(ctx, *p)) else {
                continue;
            };
            let point = pointer - self.canvas_rect.min;
            let mut view = self
                .viewports
                .gesture
                .apply(self.viewport(), delta, time, point.into());
            if view != self.viewport() {
                let content = self
                    .editor
                    .layout_page(self.editor.surface_id())
                    .map(|layout| {
                        layout.nodes.iter().fold([0_f32; 2], |extent, node| {
                            [
                                extent[0].max(node.bounds.x + node.bounds.width),
                                extent[1].max(node.bounds.y + node.bounds.height),
                            ]
                        })
                    })
                    .unwrap_or([0.; 2]);
                let limits = wireframe::viewport::scroll_limits(
                    content,
                    self.canvas_rect.size().into(),
                    view.zoom,
                );
                for (scroll, limit) in view.scroll.iter_mut().zip(limits) {
                    *scroll = scroll.min(limit);
                }
            }
            self.set_viewport(view);
        }
        // Zoom uses ordered native events only. Ordinary scrolling uses its own
        // dependency-managed queue, independent of the final hover position.
        let scroll = self.canvas_scroll_delta(ctx);
        self.pan += scroll;
    }
}
