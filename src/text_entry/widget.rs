// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

use super::{
    Caret, Draft, Selection,
    layout::{Engine, Layout},
};
use eframe::egui::{
    self, Color32, Event, Id, ImeEvent, Key, Pos2, Rect, Response, Sense, TextureHandle, Ui, Vec2,
};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
mod accessibility;

#[derive(Clone)]
struct Composition {
    text: String,
    active: Option<Range<usize>>,
}

#[derive(Clone)]
struct State {
    draft: Draft,
    layout: Option<Layout>,
    layout_key: (f32, f32, f32, bool),
    texture: Option<TextureHandle>,
    texture_key: Option<(Vec2, f32, Color32)>,
    scroll: Vec2,
    preferred_x: Option<f32>,
    composition: Option<Composition>,
    ax_runs: Vec<accessibility::Run>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            draft: Draft::new(String::new()),
            layout: None,
            layout_key: (0., 0., 0., true),
            texture: None,
            texture_key: None,
            scroll: Vec2::ZERO,
            preferred_x: None,
            composition: None,
            ax_runs: vec![],
        }
    }
}

impl State {
    fn display(&self) -> (String, Caret, Option<Range<usize>>) {
        if let Some(composition) = &self.composition {
            let range = self.draft.selection().range();
            let mut text = self.draft.text().to_owned();
            text.replace_range(range.clone(), &composition.text);
            let active = composition
                .active
                .as_ref()
                .map_or(composition.text.len(), |r| {
                    composition
                        .text
                        .char_indices()
                        .nth(r.end)
                        .map_or(composition.text.len(), |v| v.0)
                });
            (
                text,
                Caret::new(range.start + active),
                Some(range.start..range.start + composition.text.len()),
            )
        } else {
            (self.draft.text().into(), self.draft.selection().focus, None)
        }
    }

    fn layout(
        &mut self,
        engine: &mut Engine,
        size: f32,
        width: f32,
        scale: f32,
        wrap: bool,
    ) -> Result<(), String> {
        let (source, _, _) = self.display();
        if self.layout_key != (size, width, scale, wrap)
            || self.layout.as_ref().is_none_or(|l| l.source() != source)
        {
            self.layout = None;
            self.texture = None;
            self.layout = Some(Layout::with_wrap(
                engine, &source, size, width, scale, wrap,
            )?);
            self.layout_key = (size, width, scale, wrap);
            self.texture_key = None;
        }
        Ok(())
    }
}

pub fn forget(ctx: &egui::Context, id: Id) {
    ctx.data_mut(|d| d.remove::<State>(id));
}

/// A source-aware input field. Single-line mode does not soft-wrap and leaves
/// Enter and vertical arrows available to the containing search or form.
pub struct TextInput<'a> {
    value: &'a mut String,
    multiline: bool,
    rows: usize,
    autofocus: bool,
    width: Option<f32>,
    id: Option<Id>,
    hint: &'a str,
}
impl<'a> TextInput<'a> {
    pub fn singleline(value: &'a mut String) -> Self {
        Self {
            value,
            multiline: false,
            rows: 1,
            autofocus: false,
            width: None,
            id: None,
            hint: "",
        }
    }
    pub fn multiline(value: &'a mut String) -> Self {
        Self {
            multiline: true,
            rows: 4,
            ..Self::singleline(value)
        }
    }
    pub fn desired_rows(mut self, rows: usize) -> Self {
        self.rows = rows;
        self
    }
    pub fn desired_width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
    pub fn autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = autofocus;
        self
    }
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id = Some(Id::new(salt));
        self
    }
    pub fn hint_text(mut self, hint: &'a str) -> Self {
        self.hint = hint;
        self
    }
}
impl egui::Widget for TextInput<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        input(ui, self)
    }
}

#[derive(Clone, Copy)]
struct InputFrame(u64);

/// Whether a source-aware field currently owns keyboard editing. Unlike a
/// general focus check, this excludes buttons and other focusable controls.
pub fn is_focused(ctx: &egui::Context) -> bool {
    let Some(id) = ctx.memory(|m| m.focused()) else {
        return false;
    };
    ctx.data_mut(|d| d.get_temp::<InputFrame>(id.with("source-input-frame")))
        .is_some_and(|r| ctx.cumulative_frame_nr().saturating_sub(r.0) <= 1)
}

/// Restore an input owner after a temporarily disabled UI. Refresh its source
/// editor registration before document shortcuts are dispatched again.
pub fn restore_focus(ctx: &egui::Context, id: Id) {
    ctx.memory_mut(|m| m.request_focus(id));
    let frame = ctx.cumulative_frame_nr();
    ctx.data_mut(|d| {
        if d.get_temp::<InputFrame>(id.with("source-input-frame"))
            .is_some()
        {
            d.insert_temp(id.with("source-input-frame"), InputFrame(frame));
        }
    });
}

pub fn multiline(ui: &mut Ui, value: &mut String, rows: usize, autofocus: bool) -> Response {
    ui.add(
        TextInput::multiline(value)
            .desired_rows(rows)
            .desired_width(f32::INFINITY)
            .autofocus(autofocus),
    )
}

// Interaction updates focus for the whole frame before fields read keyboard
// events. Route the text on either side of a pointer/Tab boundary separately,
// so the old field keeps its final characters and the new field gets only the
// input following the focus change. This also works in either widget order.
fn editing_events(ui: &Ui, response: &Response) -> Vec<(usize, Event)> {
    let had_focus = ui.memory(|m| m.had_focus_last_frame(response.id));
    let has_focus = response.has_focus();
    if !ui.is_enabled() || !ui.input(|i| i.focused) || (!had_focus && !has_focus) {
        return vec![];
    }
    let events = ui.input(|i| i.events.clone());
    let boundary = events.iter().any(|event| {
        matches!(
            event,
            Event::PointerButton {
                pressed: true,
                button: egui::PointerButton::Primary,
                ..
            } | Event::Key {
                key: Key::Tab,
                pressed: true,
                ..
            }
        )
    });
    let mut active = had_focus || (has_focus && !boundary);
    events
        .into_iter()
        .enumerate()
        .filter(|(_, event)| match event {
            Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                ..
            } => {
                active = has_focus && response.rect.contains(*pos);
                false
            }
            Event::Key {
                key: Key::Tab,
                pressed: true,
                ..
            } => {
                active = has_focus && !had_focus;
                false
            }
            Event::WindowFocused(false) => {
                active = false;
                false
            }
            _ => active,
        })
        .collect()
}

fn input(ui: &mut Ui, options: TextInput<'_>) -> Response {
    let TextInput {
        value,
        multiline,
        rows,
        autofocus,
        width,
        id,
        hint,
    } = options;
    let size = egui::TextStyle::Body.resolve(ui.style()).size;
    let padding = Vec2::new(4., if multiline { 4. } else { 2. });
    let height = size * 1.4 * if multiline { rows.max(1) as f32 } else { 1. } + 2. * padding.y;
    let width = width.unwrap_or(if multiline {
        ui.available_width()
    } else {
        ui.spacing().text_edit_width
    });
    let (allocated_id, rect) =
        ui.allocate_space(Vec2::new(width.min(ui.available_width()).max(40.), height));
    let id = id.map_or(allocated_id, |id| ui.make_persistent_id(id));
    let mut response = ui.interact(rect, id, Sense::click_and_drag());
    let saved = ui.ctx().data_mut(|d| d.remove_temp::<State>(id));
    let fresh = saved.is_none();
    let mut state = saved.unwrap_or_default();
    if state.draft.text() != value {
        state = State {
            draft: Draft::new(value.clone()),
            ..Default::default()
        };
    }
    if fresh && autofocus && ui.is_enabled() {
        response.request_focus();
    }
    let engine = super::layout::shared_engine(ui.ctx());
    let mut engine = engine.lock().unwrap_or_else(|e| e.into_inner());
    let inner = rect.shrink2(padding);
    let scale = ui.ctx().pixels_per_point();
    let mut error = state
        .layout(&mut engine, size, inner.width(), scale, multiline)
        .err();
    let old_value = value.clone();
    let mut reveal = fresh;
    let mut interrupt_ime = false;
    if response.clicked() || response.drag_started() {
        response.request_focus();
        interrupt_ime = state.composition.take().is_some();
        error = state
            .layout(&mut engine, size, inner.width(), scale, multiline)
            .err();
        let pos = if response.drag_started() {
            ui.input(|i| i.pointer.press_origin())
        } else {
            response.interact_pointer_pos()
        };
        if let (Some(pos), Some(layout)) = (pos, state.layout.as_ref()) {
            let caret = layout.hit(Pos2::new(
                pos.x - inner.left() + state.scroll.x,
                pos.y - inner.top() + state.scroll.y,
            ));
            if response.triple_clicked() {
                select_paragraph(&mut state.draft, caret.byte);
            } else if response.double_clicked() {
                select_word(&mut state.draft, caret.byte);
            } else {
                state.draft.move_to(caret, ui.input(|i| i.modifiers.shift));
            }
            state.preferred_x = None;
            reveal = true;
        }
    }
    if response.dragged()
        && let (Some(pos), Some(layout)) = (response.interact_pointer_pos(), state.layout.as_ref())
    {
        if pos.y < inner.top() {
            state.scroll.y = (state.scroll.y - size).max(0.);
        }
        if pos.y > inner.bottom() {
            state.scroll.y += size;
        }
        if !multiline && pos.x < inner.left() {
            state.scroll.x = (state.scroll.x - size).max(0.);
        }
        if !multiline && pos.x > inner.right() {
            state.scroll.x += size;
        }
        state.draft.move_to(
            layout.hit(Pos2::new(
                pos.x - inner.left() + state.scroll.x,
                pos.y - inner.top() + state.scroll.y,
            )),
            true,
        );
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
        let delta = ui.input_mut(|i| {
            let delta = i.smooth_scroll_delta;
            i.smooth_scroll_delta = Vec2::ZERO;
            delta
        });
        if multiline {
            state.scroll.y -= delta.y;
        } else {
            state.scroll.x -= delta.x + delta.y;
        }
    }
    let events = editing_events(ui, &response);
    if response.has_focus() && ui.is_enabled() {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    tab: false,
                    escape: true,
                },
            )
        });
    }
    let mut consumed_keys = Vec::new();
    if !events.is_empty() {
        for (event_index, event) in events {
            let vertical = matches!(
                &event,
                Event::Key {
                    key: Key::ArrowUp | Key::ArrowDown,
                    ..
                }
            );
            let handled = match event {
                Event::Copy => {
                    if !state.draft.selected_text().is_empty() {
                        ui.ctx().copy_text(state.draft.selected_text().into());
                    }
                    false
                }
                Event::Cut => {
                    if !state.draft.selected_text().is_empty() {
                        ui.ctx().copy_text(state.draft.selected_text().into());
                        state.draft.insert("");
                    }
                    true
                }
                Event::Paste(text) => {
                    interrupt_ime |= state.composition.take().is_some();
                    state.draft.insert(&text);
                    true
                }
                Event::Text(text) if state.composition.is_none() => {
                    state.draft.insert(&text);
                    true
                }
                Event::Ime(event) => {
                    ime(&mut state, &event);
                    true
                }
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if key == Key::Escape && state.composition.is_none() {
                        response.surrender_focus();
                    }
                    if !multiline && key == Key::Enter && state.composition.is_none() {
                        response.surrender_focus();
                    }
                    let composing_form_key = state.composition.is_some()
                        && matches!(key, Key::Enter | Key::ArrowUp | Key::ArrowDown);
                    let handled = composing_form_key
                        || ((multiline
                            || !matches!(key, Key::Enter | Key::ArrowUp | Key::ArrowDown))
                            && key_event(&mut state, key, modifiers));
                    if handled {
                        consumed_keys.push(event_index);
                    }
                    handled
                }
                Event::AccessKitActionRequest(request) => {
                    accessibility::action(&mut state, id, &request)
                }
                _ => false,
            };
            if handled {
                if !vertical {
                    state.preferred_x = None;
                }
                reveal = true;
                error = state
                    .layout(&mut engine, size, inner.width(), scale, multiline)
                    .err();
            }
        }
    }
    ui.input_mut(|i| {
        let mut index = 0;
        i.events.retain(|_| {
            let keep = !consumed_keys.contains(&index);
            index += 1;
            keep
        });
    });
    if !response.has_focus() && state.composition.take().is_some() {
        interrupt_ime = true;
        error = state
            .layout(&mut engine, size, inner.width(), scale, multiline)
            .err();
    }
    *value = state.draft.text().into();
    if *value != old_value {
        response.mark_changed();
    }
    response
        .widget_info(|| egui::WidgetInfo::text_edit(ui.is_enabled(), &old_value, &*value, hint));
    if ui.is_enabled() {
        let frame = ui.ctx().cumulative_frame_nr();
        ui.ctx()
            .data_mut(|d| d.insert_temp(id.with("source-input-frame"), InputFrame(frame)));
    }
    let visuals = ui.style().interact(&response);
    ui.painter().rect(
        rect,
        visuals.corner_radius,
        ui.visuals().extreme_bg_color,
        if response.has_focus() {
            ui.visuals().selection.stroke
        } else {
            visuals.bg_stroke
        },
        egui::StrokeKind::Inside,
    );
    let painter = ui.painter().with_clip_rect(inner.intersect(ui.clip_rect()));
    let (_, caret, composition) = state.display();
    if let Some(layout) = &mut state.layout {
        if reveal && let Some(c) = layout.caret_rect(caret) {
            if c.top() < state.scroll.y {
                state.scroll.y = c.top();
            }
            if c.bottom() > state.scroll.y + inner.height() {
                state.scroll.y = c.bottom() - inner.height();
            }
            if c.left() < state.scroll.x {
                state.scroll.x = c.left();
            }
            if c.right() > state.scroll.x + inner.width() {
                state.scroll.x = c.right() - inner.width();
            }
        }
        state.scroll.y = state
            .scroll
            .y
            .clamp(0., (layout.size.y - inner.height()).max(0.));
        state.scroll.x = state
            .scroll
            .x
            .clamp(0., (layout.size.x + 1. - inner.width()).max(0.));
        let offset = inner.min.to_vec2() - state.scroll;
        let color = ui.visuals().text_color();
        let key = (state.scroll, inner.height(), color);
        if state.texture_key != Some(key) {
            match layout.raster(
                &mut engine,
                Rect::from_min_size(state.scroll.to_pos2(), inner.size()),
                color,
            ) {
                Ok(image) => {
                    if let Some(texture) = &mut state.texture {
                        texture.set(image, egui::TextureOptions::LINEAR);
                    } else {
                        state.texture = Some(ui.ctx().load_texture(
                            "text-input",
                            image,
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                    state.texture_key = Some(key);
                }
                Err(message) => error = Some(message),
            }
        }
        let selection = if let Some(range) = &composition {
            Selection {
                anchor: Caret::new(range.start),
                focus: Caret::new(range.end),
            }
        } else {
            state.draft.selection()
        };
        for r in layout.selection_rects(selection) {
            if composition.is_some() {
                painter.line_segment(
                    [r.left_bottom() + offset, r.right_bottom() + offset],
                    (1., color),
                );
            } else {
                painter.rect_filled(r.translate(offset), 0., ui.visuals().selection.bg_fill);
            }
        }
        if let Some(texture) = &state.texture {
            painter.image(
                texture.id(),
                inner,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
                Color32::WHITE,
            );
        }
        if response.has_focus()
            && let Some(c) = layout.caret_rect(caret)
        {
            let screen = c.translate(offset);
            painter.line_segment([screen.left_top(), screen.left_bottom()], (1., color));
            ui.ctx().output_mut(|o| {
                o.ime = Some(egui::output::IMEOutput {
                    purpose: egui::IMEPurpose::Normal,
                    rect: inner,
                    cursor_rect: screen,
                    should_interrupt_composition: interrupt_ime,
                })
            });
        }
    }
    if value.is_empty() && state.composition.is_none() && !hint.is_empty() {
        painter.text(
            inner.min,
            egui::Align2::LEFT_TOP,
            hint,
            egui::FontId::proportional(size),
            ui.visuals().weak_text_color(),
        );
    }
    if let Some(message) = error {
        painter.text(
            inner.min,
            egui::Align2::LEFT_TOP,
            message,
            egui::FontId::proportional(size),
            ui.visuals().error_fg_color,
        );
    }
    accessibility::update(ui, &response, &mut state, inner, multiline, hint);
    ui.ctx().data_mut(|d| d.insert_temp(id, state));
    response
}

fn key_event(state: &mut State, key: Key, m: egui::Modifiers) -> bool {
    if state.composition.is_some() {
        if key == Key::Escape {
            state.composition = None;
            return true;
        }
        return false;
    }
    if m.command && key == Key::A {
        state.draft.select_all();
        return true;
    }
    if m.command && key == Key::Z {
        if m.shift {
            state.draft.redo();
        } else {
            state.draft.undo();
        }
        return true;
    }
    if m.command && key == Key::Y {
        state.draft.redo();
        return true;
    }
    let old = state.draft.selection();
    match key {
        Key::Enter => {
            state.draft.insert("\n");
        }
        Key::Backspace => {
            if m.mac_cmd
                && let Some(layout) = &state.layout
            {
                state.draft.delete_to(line_edge(layout, old.focus, false));
            } else if m.alt || m.ctrl {
                state.draft.delete_to(word_caret(&state.draft, false));
            } else {
                state.draft.delete_backward();
            }
        }
        Key::Delete => {
            if m.alt || m.ctrl {
                state.draft.delete_to(word_caret(&state.draft, true));
            } else {
                state.draft.delete_forward();
            }
        }
        Key::ArrowLeft | Key::ArrowRight => {
            let right = key == Key::ArrowRight;
            if !m.shift && !old.range().is_empty() {
                if let Some(layout) = &state.layout {
                    state
                        .draft
                        .move_to(layout.selection_edge(old, right), false);
                } else {
                    state.draft.move_logical(right, false);
                }
            } else if let Some(layout) = &state.layout {
                let caret = if m.mac_cmd {
                    line_edge(layout, old.focus, right)
                } else if m.alt || m.ctrl {
                    word_caret(&state.draft, right)
                } else {
                    layout.horizontal(old.focus, right)
                };
                state.draft.move_to(caret, m.shift);
            }
            state.preferred_x = None;
        }
        Key::ArrowUp | Key::ArrowDown => {
            let down = key == Key::ArrowDown;
            if m.command {
                state.draft.move_to(
                    Caret::new(if down { state.draft.text().len() } else { 0 }),
                    m.shift,
                );
            } else if let Some(layout) = &state.layout {
                let x = *state
                    .preferred_x
                    .get_or_insert_with(|| layout.caret_rect(old.focus).map_or(0., |r| r.left()));
                state
                    .draft
                    .move_to(layout.vertical(old.focus, down, x), m.shift);
            }
        }
        Key::Home | Key::End => {
            let end = key == Key::End;
            if m.command {
                state.draft.move_to(
                    Caret::new(if end { state.draft.text().len() } else { 0 }),
                    m.shift,
                );
            } else if let Some(layout) = &state.layout {
                state
                    .draft
                    .move_to(line_edge(layout, old.focus, end), m.shift);
            }
            state.preferred_x = None;
        }
        _ => return false,
    }
    true
}

fn line_edge(layout: &Layout, caret: Caret, end: bool) -> Caret {
    let Some(rect) = layout.caret_rect(caret) else {
        return caret;
    };
    layout
        .rows
        .iter()
        .find(|r| r.rect.top() == rect.top())
        .map_or(caret, |row| Caret {
            byte: if end {
                row.source.end
            } else {
                row.source.start
            },
            affinity: if end {
                super::Affinity::Upstream
            } else {
                super::Affinity::Downstream
            },
        })
}

fn word_caret(draft: &Draft, forward: bool) -> Caret {
    let at = draft.selection().focus.byte;
    let words = draft.text().unicode_word_indices();
    Caret::new(if forward {
        words
            .map(|(i, w)| i + w.len())
            .find(|&i| i > at)
            .unwrap_or(draft.text().len())
    } else {
        words
            .map(|(i, _)| i)
            .take_while(|&i| i < at)
            .last()
            .unwrap_or(0)
    })
}

fn select_word(draft: &mut Draft, byte: usize) {
    if let Some((i, w)) = draft
        .text()
        .unicode_word_indices()
        .find(|(i, w)| *i <= byte && byte < i + w.len())
    {
        let end = i + w.len();
        draft.select(Caret::new(i), Caret::new(end));
    } else {
        draft.move_to(Caret::new(byte), false);
        draft.move_logical(true, true);
    }
}

fn select_paragraph(draft: &mut Draft, byte: usize) {
    let start = draft.text()[..byte]
        .rfind(['\n', '\r'])
        .map_or(0, |i| i + 1);
    let end = draft.text()[byte..]
        .find(['\n', '\r'])
        .map_or(draft.text().len(), |i| byte + i + 1);
    draft.select(Caret::new(start), Caret::new(end));
}

fn ime(state: &mut State, event: &ImeEvent) {
    match event {
        ImeEvent::Preedit {
            text,
            active_range_chars,
        } => {
            state.composition = if text.is_empty() {
                None
            } else {
                Some(Composition {
                    text: text.clone(),
                    active: active_range_chars.clone(),
                })
            }
        }
        ImeEvent::Commit(text) => {
            if !text.is_empty() {
                state.draft.insert(text);
            }
            state.composition = None;
        }
        ImeEvent::DeleteSurrounding {
            before_chars,
            after_chars,
        } => {
            let byte = state.draft.selection().focus.byte;
            let before = state.draft.text()[..byte]
                .char_indices()
                .rev()
                .nth(before_chars.saturating_sub(1))
                .map_or(0, |v| v.0);
            let after = state.draft.text()[byte..]
                .char_indices()
                .nth(*after_chars)
                .map_or(state.draft.text().len(), |v| byte + v.0);
            let floor = super::floor_boundary(state.draft.text(), after);
            let after = if floor == after {
                after
            } else {
                super::next_boundary(state.draft.text(), floor)
            };
            state.draft.select(
                Caret::new(if *before_chars == 0 { byte } else { before }),
                Caret::new(after),
            );
            state.draft.insert("");
        }
        _ => {}
    }
}
