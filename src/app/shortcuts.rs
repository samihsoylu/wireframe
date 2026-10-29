// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
impl WireframeApp {
    /// A field applies input during UI construction, after document shortcuts.
    /// Split at document/view commands and pointer actions so earlier text reaches
    /// its model before a command captures it, replaces it or removes its field.
    /// Input after a pointer action waits for the clicked widget to acquire focus.
    /// Later events wait for that command, preserving text-local Undo order.
    /// This must run before egui derives pointer/focus state from the events.
    pub(super) fn order_text_input(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        let text_focus = wireframe::text_entry::is_focused(ctx) || ctx.text_edit_focused();
        if self.save_busy()
            && (text_focus
                || self.deferred_text_focus.is_some()
                || !self.deferred_text_input.is_empty())
        {
            // Disabled fields surrender focus. Keep their owner and input,
            // including new typing arriving while the worker is still busy.
            if self.deferred_text_focus.is_none() && text_focus {
                self.deferred_text_focus = ctx.memory(|m| m.focused());
            }
            self.deferred_text_input.append(&mut raw.events);
            if !self.deferred_text_input.is_empty() {
                ctx.request_repaint_after(Duration::from_millis(16));
            }
            return;
        }
        if let Some(id) = self.deferred_text_focus.take() {
            wireframe::text_entry::restore_focus(ctx, id);
        }
        if !self.deferred_text_input.is_empty() {
            self.deferred_text_input.append(&mut raw.events);
            raw.events = std::mem::take(&mut self.deferred_text_input);
        }
        // Shortcuts run before widgets are drawn. Deliver a preceding click
        // first so a following key, text edit or command uses its new owner.
        let mut pointer_action = false;
        let pointer_boundary = raw.events.iter().position(|event| match event {
            egui::Event::PointerButton { .. } => {
                pointer_action = true;
                false
            }
            // Escape must still cancel a pointer gesture before its release
            // can apply a resize, drag or drawing action in this batch.
            egui::Event::Key {
                key: Key::Escape, ..
            } => false,
            egui::Event::Key { pressed: true, .. }
            | egui::Event::Text(_)
            | egui::Event::Paste(_)
            | egui::Event::Copy
            | egui::Event::Cut
            | egui::Event::Ime(_)
            | egui::Event::AppCommand(_) => pointer_action,
            _ => false,
        });
        if let Some(boundary) = pointer_boundary {
            self.deferred_text_input = raw.events.split_off(boundary);
            ctx.request_repaint();
        }
        if !wireframe::text_entry::is_focused(ctx) && !ctx.text_edit_focused() {
            return;
        }
        let quick_focused =
            self.quick_input_id.is_some() && self.quick_input_id == ctx.memory(|m| m.focused());
        let boundary = raw
            .events
            .iter()
            .enumerate()
            .find_map(|(index, event)| match event {
                // Canvas text closes during UI construction. Let that finish
                // before a following Undo/Redo chooses its history owner.
                egui::Event::Key {
                    key,
                    modifiers,
                    pressed: true,
                    ..
                } if self.text_session_exit_key(ctx, *key, *modifiers) => {
                    Some(if index == 0 { 1 } else { index })
                }
                // Query edits and their history must finish before deciding
                // whether a following command belongs to the document.
                egui::Event::Key {
                    key,
                    modifiers,
                    pressed: true,
                    ..
                } if quick_focused
                    && (matches!(key, Key::Escape | Key::Enter)
                        || modifiers.command && matches!(key, Key::Z | Key::Y)) =>
                {
                    Some(if index == 0 { 1 } else { index })
                }
                egui::Event::Key {
                    key,
                    modifiers,
                    pressed: true,
                    ..
                } if (self.notes_editing() || self.review_focused(ctx))
                    && index > 0
                    && (*key == Key::Escape || modifiers.command && *key == Key::Enter) =>
                {
                    Some(index)
                }
                egui::Event::AppCommand(_) => Some(if index == 0 { 1 } else { index }),
                egui::Event::Key {
                    key,
                    modifiers,
                    pressed: true,
                    ..
                } if (modifiers.matches_logically(Modifiers::COMMAND)
                    && matches!(
                        key,
                        Key::S | Key::N | Key::O | Key::J | Key::L | Key::Semicolon
                    ))
                    || (modifiers.matches_logically(Modifiers::COMMAND | Modifiers::SHIFT)
                        && matches!(key, Key::S | Key::N | Key::F)) =>
                {
                    Some(if index == 0 { 1 } else { index })
                }
                // Toolbar/sidebar actions can remove a field or capture its model
                // before that field is drawn. Deliver preceding edits first.
                egui::Event::PointerButton { pressed: true, .. }
                    if index > 0
                        && raw.events[..index].iter().any(|event| {
                            matches!(
                                event,
                                egui::Event::Text(_)
                                    | egui::Event::Paste(_)
                                    | egui::Event::Cut
                                    | egui::Event::Ime(_)
                                    | egui::Event::Key { pressed: true, .. }
                            )
                        }) =>
                {
                    Some(index)
                }
                // Tab may wrap focus at the end of the UI pass. Deliver following
                // input only after the destination field has acquired that focus.
                egui::Event::Key {
                    key: Key::Tab,
                    pressed: true,
                    ..
                } => Some(index + 1),
                _ => None,
            });
        if let Some(boundary) = boundary {
            let mut later = raw.events.split_off(boundary);
            later.append(&mut self.deferred_text_input);
            self.deferred_text_input = later;
            if !self.deferred_text_input.is_empty() {
                ctx.request_repaint();
            }
        }
    }

    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.startup_open.is_some() {
            return;
        }
        self.sync_paste_target();
        let mut events = ctx.input_mut(|i| std::mem::take(&mut i.events));
        if let Some(pending) = self.pending_cut_request() {
            if let Some(index) = events.iter().position(|event| {
                matches!(event,
                egui::Event::ClipboardWriteResult {request,..} if *request == pending)
            }) {
                let event = events.remove(index);
                self.shortcut_event(ctx, &event);
            } else {
                self.deferred_cut_input.extend(events);
                return;
            }
        }
        if !self.deferred_cut_input.is_empty() {
            self.deferred_cut_input.append(&mut events);
            events = std::mem::take(&mut self.deferred_cut_input);
        }
        // A source transfer keeps commands from replacing its captured graph.
        if self.source_navigation.drag.is_some() {
            if !self.show_navigator
                || self.present
                || !self.editor.in_source_workspace()
                || ctx.input(|i| !i.focused)
                || events.iter().any(|e| {
                    matches!(
                        e,
                        egui::Event::Key {
                            key: Key::Escape,
                            pressed: true,
                            ..
                        }
                    )
                })
            {
                self.source_navigation.drag = None;
            }
            events.retain(|event| {
                !matches!(
                    event,
                    egui::Event::Key { .. }
                        | egui::Event::AppCommand(_)
                        | egui::Event::Copy
                        | egui::Event::Cut
                        | egui::Event::Paste(_)
                )
            });
            ctx.input_mut(|i| i.events = events);
            return;
        }
        // A page drag owns input until drop/cancel; commands cannot change its source.
        if self.navigator.drag.is_some() || self.navigator.press.is_some() {
            if !self.show_navigator
                || self.present
                || self.editor.in_source_workspace()
                || ctx.input(|i| !i.focused)
                || events.iter().any(|e| {
                    matches!(
                        e,
                        egui::Event::Key {
                            key: Key::Escape,
                            pressed: true,
                            ..
                        }
                    )
                })
            {
                self.navigator.drag = None;
                self.navigator.press = None;
            }
            events.retain(|event| {
                !matches!(
                    event,
                    egui::Event::Key { .. }
                        | egui::Event::AppCommand(_)
                        | egui::Event::Copy
                        | egui::Event::Cut
                        | egui::Event::Paste(_)
                )
            });
            ctx.input_mut(|i| i.events = events);
            return;
        }
        // A held library control is transient; no document command may run underneath it.
        if self.library_drag.is_some() || self.library_press.is_some() {
            events.retain(|event| {
                let escape = matches!(event, egui::Event::Key {
                    key: Key::Escape, pressed: true, modifiers, ..
                } if *modifiers == Modifiers::NONE);
                if escape {
                    self.library_drag = None;
                    self.library_press = None;
                }
                !escape
            });
            ctx.input_mut(|i| i.events = events);
            return;
        }
        // A Cut is a frame boundary: later commands wait for native clipboard
        // completion so Cut/Undo and Cut/Save retain their original order.
        let mut remaining = Vec::with_capacity(events.len());
        let mut events = events.into_iter();
        while let Some(event) = events.next() {
            if !self.shortcut_event(ctx, &event) {
                remaining.push(event);
            }
            if self.pending_cut.is_some() {
                self.deferred_cut_input.extend(events);
                break;
            }
        }
        ctx.input_mut(|i| {
            remaining.append(&mut i.events);
            i.events = remaining;
        });
    }
    fn shortcut_event(&mut self, ctx: &egui::Context, event: &egui::Event) -> bool {
        if let egui::Event::ClipboardWriteResult { request, result } = event {
            self.finish_cut(ctx, *request, result);
            return true;
        }
        if self.review_shortcut(ctx, event) || self.notes_shortcut(event) {
            return true;
        }
        if let egui::Event::AppCommand(id) = event {
            if let Some(command) = menus::Command::from_id(*id) {
                self.run_command(ctx, command);
            }
            return true;
        }
        if matches!(event, egui::Event::Key {
            key: Key::S, modifiers, pressed: true, ..
        } if modifiers.matches_logically(Modifiers::COMMAND | Modifiers::SHIFT))
        {
            self.run_command(ctx, menus::Command::SaveAs);
            return true;
        }
        // Save committed document changes without accepting the active canvas
        // draft. Keep the same modal/gesture checks as the native menu command.
        if self.text_edit.is_some()
            && matches!(event, egui::Event::Key {
                key: Key::S, modifiers, pressed: true, ..
            } if modifiers.matches_logically(Modifiers::COMMAND))
        {
            self.run_command(ctx, menus::Command::Save);
            return true;
        }
        // Re-evaluate after each command: New/Open/Enter can establish a modal
        // boundary that later events must not bypass.
        if self.startup_open.is_some()
            || self.error.is_some()
            || self.font_import.is_some()
            || self.library_import.is_some()
            || self.more_controls.is_some()
            || self.page_paste.is_some()
            || self.pdf_options.is_some()
            || self.image_export.blocks_input()
            || self.versions.rename.is_some()
            || self.page_rename.is_some()
            || self.trash.is_some()
            || self.save_busy()
            || self.recovery_chooser_open()
            || self.pending.is_some()
            || self.text_edit.is_some()
            || self.icon_dialog.is_some()
            || self.image_import.is_some()
            || self.web_image.is_some()
            || self.screenshot_text.is_some()
            || self.recovery_offer.is_some()
            || self.image_crop.is_some()
        {
            return false;
        }
        let is = |m, k| matches!(event, egui::Event::Key {key,modifiers,pressed:true,..} if *key==k && modifiers.matches_logically(m));
        // Shift can turn a digit's logical key into punctuation (for example
        // Command-Shift-1 arrives as '!'). Retain the numeric shortcut identity.
        let is_number = |m, k| {
            matches!(event, egui::Event::Key {key,physical_key,modifiers,pressed:true,..}
            if (*key == k || *physical_key == Some(k)) && modifiers.matches_logically(m))
        };
        if self.present {
            return match event {
                egui::Event::Key {
                    key,
                    modifiers,
                    pressed: true,
                    ..
                } => self.presentation_shortcut(ctx, *key, *modifiers),
                _ => false,
            };
        }
        self.sync_viewport();
        if (self.notes_editing() || self.review_focused(ctx))
            && (is(Modifiers::COMMAND | Modifiers::SHIFT, Key::N)
                || is(Modifiers::COMMAND, Key::S)
                || is(Modifiers::COMMAND, Key::O)
                || is(Modifiers::COMMAND, Key::N)
                || is(Modifiers::COMMAND, Key::Semicolon)
                || is(Modifiers::COMMAND | Modifiers::SHIFT, Key::F))
            && !self.finish_page_notes()
        {
            return true;
        }
        // More specific modifiers must precede their less specific counterparts.
        if is(Modifiers::COMMAND | Modifiers::SHIFT, Key::N) {
            self.editor.add_page();
        } else if is(Modifiers::COMMAND, Key::S) {
            self.save(false);
        } else if is(Modifiers::COMMAND, Key::O) {
            self.open(ctx);
        } else if is(Modifiers::COMMAND, Key::N) {
            self.request(Pending::New, ctx);
        } else if is(Modifiers::COMMAND, Key::J) {
            self.show_navigator = !self.show_navigator;
        } else if is(Modifiers::COMMAND, Key::L) {
            self.show_library = !self.show_library;
        } else if is(Modifiers::COMMAND, Key::Semicolon) {
            self.toggle_review_inspector();
        } else if is_number(Modifiers::COMMAND | Modifiers::SHIFT, Key::Num0) {
            self.zoom_all(true);
        } else if is_number(Modifiers::COMMAND | Modifiers::SHIFT, Key::Num1) {
            self.zoom_all(false);
        } else if is(Modifiers::COMMAND, Key::Plus) || is(Modifiers::COMMAND, Key::Equals) {
            self.zoom_step(true);
        } else if is(Modifiers::COMMAND, Key::Minus) {
            self.zoom_step(false);
        } else if is(Modifiers::COMMAND, Key::Num0) {
            self.fit();
        } else if is(Modifiers::COMMAND, Key::Num1) {
            self.actual_size();
        } else if is(Modifiers::COMMAND | Modifiers::SHIFT, Key::F) {
            self.start_presentation();
        } else {
            if self.quick_add_shortcut(ctx, event) {
                return true;
            }
            // Text editors and popups retain their own input, including local Undo.
            if wireframe::text_entry::is_focused(ctx)
                || ctx.text_edit_focused()
                || ctx.any_popup_open()
                || egui::Popup::is_any_open(ctx)
                || self.quick_focus
            {
                return false;
            }
            if let Some(command) = [
                (Key::Num0, menus::Command::AutoSize),
                (Key::Num1, menus::Command::AlignLeft),
                (Key::Num2, menus::Command::AlignCenter),
                (Key::Num3, menus::Command::AlignRight),
                (Key::Num4, menus::Command::AlignTop),
                (Key::Num5, menus::Command::AlignMiddle),
                (Key::Num6, menus::Command::AlignBottom),
            ]
            .into_iter()
            .find_map(|(key, command)| {
                is_number(Modifiers::COMMAND | Modifiers::ALT, key).then_some(command)
            }) {
                self.run_command(ctx, command);
                return true;
            }
            if is(Modifiers::NONE, Key::Slash) || is(Modifiers::NONE, Key::Plus) {
                self.quick_focus = true;
            } else if is(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)
                || is(Modifiers::COMMAND, Key::Y)
            {
                self.editor.redo();
            } else if is(Modifiers::COMMAND, Key::Z) {
                self.editor.undo();
            } else if is(Modifiers::COMMAND, Key::A) {
                self.run_command(ctx, menus::Command::SelectAll);
            } else if is(Modifiers::COMMAND | Modifiers::SHIFT, Key::K) {
                self.run_command(ctx, menus::Command::TreatAsMarkup);
            } else if is(Modifiers::COMMAND, Key::K) {
                self.run_command(ctx, menus::Command::ShowMarkup);
            } else if is(Modifiers::COMMAND, Key::D) {
                self.run_command(ctx, menus::Command::Duplicate);
            } else if is(Modifiers::COMMAND | Modifiers::SHIFT, Key::G) {
                self.run_command(ctx, menus::Command::Ungroup);
            } else if is(Modifiers::COMMAND, Key::G) {
                self.run_command(ctx, menus::Command::Group);
            } else if is(Modifiers::COMMAND | Modifiers::SHIFT, Key::ArrowUp) {
                self.run_command(ctx, menus::Command::BringToFront);
            } else if is(Modifiers::COMMAND | Modifiers::SHIFT, Key::ArrowDown) {
                self.run_command(ctx, menus::Command::SendToBack);
            } else if is(Modifiers::COMMAND, Key::ArrowUp) {
                self.run_command(ctx, menus::Command::BringForward);
            } else if is(Modifiers::COMMAND, Key::ArrowDown) {
                self.run_command(ctx, menus::Command::SendBackward);
            } else if is(Modifiers::NONE, Key::Delete) || is(Modifiers::NONE, Key::Backspace) {
                self.run_command(ctx, menus::Command::Delete);
            } else if is(Modifiers::NONE, Key::Escape) {
                if self.cancel_quick_draw() {
                    return true;
                }
                self.present = false;
                self.editor.cancel();
                self.drag = None;
                self.alignment_guides.clear();
                if !self.editor.leave() {
                    self.editor.selection.clear();
                }
            } else if self.source_navigation_focused(ctx) {
                if let egui::Event::Key {
                    key,
                    modifiers,
                    pressed: true,
                    ..
                } = event
                {
                    self.source_navigation_key(ctx, *key, *modifiers);
                }
                match event {
                    egui::Event::Copy => self.run_command(ctx, menus::Command::Copy),
                    egui::Event::Cut => self.run_command(ctx, menus::Command::Cut),
                    egui::Event::Paste(text) if self.finish_page_notes() => self.pasted(text),
                    _ => {}
                }
                return matches!(
                    event,
                    egui::Event::Copy
                        | egui::Event::Cut
                        | egui::Event::Paste(_)
                        | egui::Event::Key {
                            pressed: true,
                            key: Key::ArrowLeft
                                | Key::ArrowRight
                                | Key::ArrowUp
                                | Key::ArrowDown
                                | Key::Enter,
                            ..
                        }
                );
            } else if !self.editor.in_source_workspace()
                && self.show_navigator
                && self.navigator.focused(ctx)
            {
                if (is(Modifiers::NONE, Key::F2) || is(Modifiers::COMMAND | Modifiers::ALT, Key::R))
                    && self.editor.selection.is_empty()
                {
                    self.begin_page_rename(ctx, self.editor.page);
                    return true;
                }
                if let egui::Event::Key {
                    key,
                    modifiers,
                    pressed: true,
                    ..
                } = event
                {
                    if *key == Key::Tab {
                        self.end_pointer_gestures();
                        let tree = wireframe::page_tree::PageTree::new(&self.editor.document);
                        self.navigator.sync(
                            &tree,
                            self.editor.page,
                            self.editor.duplicated_page_selection(),
                        );
                        let pages: Vec<_> = tree
                            .rows
                            .iter()
                            .filter(|r| self.navigator.selected.contains(&r.id))
                            .map(|r| r.id)
                            .collect();
                        if let Err(error) = self.editor.change_page_indent(&pages, !modifiers.shift)
                        {
                            self.error = Some(error);
                        } else {
                            let tree = wireframe::page_tree::PageTree::new(&self.editor.document);
                            for page in pages {
                                self.navigator.reveal(&tree, page);
                            }
                            self.navigator.scroll_to = Some(self.editor.page);
                        }
                        ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
                        return true;
                    }
                    let vertical = matches!(key, Key::ArrowUp | Key::ArrowDown);
                    let ordinary = matches!(
                        key,
                        Key::ArrowLeft | Key::ArrowRight | Key::PageUp | Key::PageDown
                    );
                    if (vertical
                        && (*modifiers == Modifiers::NONE || *modifiers == Modifiers::SHIFT))
                        || (ordinary && *modifiers == Modifiers::NONE)
                    {
                        let tree = wireframe::page_tree::PageTree::new(&self.editor.document);
                        self.navigator.sync(
                            &tree,
                            self.editor.page,
                            self.editor.duplicated_page_selection(),
                        );
                        let page =
                            self.navigator
                                .navigate(&tree, self.editor.page, *key, modifiers.shift);
                        // Page traversal owns this key; do not move focus into
                        // a spatially adjacent toolbar widget at the end of the pass.
                        ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
                        self.editor.switch_page(page);
                        return true;
                    }
                }
                match event {
                    egui::Event::Copy => self.run_command(ctx, menus::Command::Copy),
                    egui::Event::Cut => self.run_command(ctx, menus::Command::Cut),
                    egui::Event::Paste(text) if self.finish_page_notes() => {
                        self.pasted_page_text(text)
                    }
                    _ => {}
                }
                // Page navigation owns these events; they never reach controls.
                return matches!(
                    event,
                    egui::Event::Copy
                        | egui::Event::Cut
                        | egui::Event::Paste(_)
                        | egui::Event::Key {
                            pressed: true,
                            key: Key::ArrowLeft
                                | Key::ArrowRight
                                | Key::ArrowUp
                                | Key::ArrowDown
                                | Key::Enter,
                            ..
                        }
                );
            } else if is(Modifiers::NONE, Key::Enter) {
                // Clickable interface controls own Return while focused. Page
                // and source navigators have their explicit routes above.
                if ctx
                    .memory(|m| m.focused())
                    .is_some_and(|id| Some(id) != self.canvas_focus_id)
                {
                    return false;
                }
                if let Some(c) = self.editor.selected_views().first()
                    && !self.editor.enter(c.id)
                    && c.group.is_none()
                    && c.kind.text_editable()
                {
                    self.begin_text_edit(c.id);
                }
            } else {
                // Focused inspector controls own directional keys. Only the
                // canvas (or no focused widget) may nudge the selection.
                if matches!(
                    event,
                    egui::Event::Key {
                        key: Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown,
                        ..
                    }
                ) && ctx
                    .memory(|m| m.focused())
                    .is_some_and(|id| Some(id) != self.canvas_focus_id)
                {
                    return false;
                }
                for (k, dx, dy) in [
                    (Key::ArrowLeft, -1., 0.),
                    (Key::ArrowRight, 1., 0.),
                    (Key::ArrowUp, 0., -1.),
                    (Key::ArrowDown, 0., 1.),
                ] {
                    if is(Modifiers::SHIFT, k) {
                        ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
                        self.editor.move_by(dx * 10., dy * 10.);
                        return true;
                    }
                    if is(Modifiers::NONE, k) {
                        // egui chooses directional focus before application
                        // shortcuts run. Consuming this nudge must also cancel
                        // that pending navigation into another interface field.
                        ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
                        self.editor.move_by(dx, dy);
                        return true;
                    }
                }
                match event {
                    egui::Event::Copy => {
                        self.copy(ctx);
                    }
                    egui::Event::Cut => {
                        self.run_command(ctx, menus::Command::Cut);
                    }
                    egui::Event::Paste(s) => self.pasted(s),
                    _ => return false,
                }
            }
        }
        true
    }

    /// Search history takes precedence, then exhausted history returns to the
    /// document. Other text fields retain their existing editing ownership.
    pub(super) fn quick_add_shortcut(&mut self, ctx: &egui::Context, event: &egui::Event) -> bool {
        let Some(id) = self
            .quick_input_id
            .filter(|id| Some(*id) == ctx.memory(|m| m.focused()))
        else {
            return false;
        };
        let egui::Event::Key {
            key,
            modifiers,
            pressed: true,
            ..
        } = event
        else {
            return false;
        };
        let history = wireframe::text_entry::history_state(ctx, id, &self.quick);
        if history.composing {
            return false;
        }
        if *key == Key::Enter && *modifiers == Modifiers::NONE && self.quick.is_empty() {
            let selected = self.editor.selected_views();
            if let [control] = selected.as_slice() {
                self.begin_text_edit(control.id);
                self.text_edit_return_to_quick = self.text_edit.is_some();
            }
            // Groups, non-text controls and multiple selections keep their
            // current context; this command never enters the canvas selection.
            return true;
        }
        if *key == Key::Escape && *modifiers == Modifiers::NONE {
            if self.quick.is_empty() {
                ctx.memory_mut(|m| m.surrender_focus(id));
            } else {
                self.quick.clear();
                self.quick_index = 0;
                wireframe::text_entry::forget(ctx, id);
            }
            return true;
        }
        let redo = (*key == Key::Z
            && modifiers.matches_logically(Modifiers::COMMAND | Modifiers::SHIFT))
            || (*key == Key::Y && modifiers.matches_logically(Modifiers::COMMAND));
        let undo = *key == Key::Z && modifiers.matches_logically(Modifiers::COMMAND) && !redo;
        if (undo && !history.undo) || (redo && !history.redo) {
            if (undo && self.editor.undo_name().is_some())
                || (redo && self.editor.redo_name().is_some())
            {
                // Crossing back to document history ends the query's redo branch.
                wireframe::text_entry::forget(ctx, id);
                if redo {
                    self.editor.redo();
                } else {
                    self.editor.undo();
                }
            }
            return true;
        }
        false
    }
}

#[cfg(test)]
mod modal_guard_tests {
    use super::*;
    fn setup() -> (egui::Context, WireframeApp) {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Button, 20., 30.);
        (ctx, app)
    }
    fn delete_key() -> egui::Event {
        egui::Event::Key {
            key: Key::Delete,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }
    #[test]
    fn raw_shortcuts_cannot_edit_under_an_open_failure_dialog() {
        let (ctx, mut app) = setup();
        let before = app.editor.document.clone();
        app.error = Some("Could not open the requested project".into());
        assert!(!app.shortcut_event(&ctx, &delete_key()));
        assert_eq!(app.editor.document, before);
        assert!(app.error.is_some());
    }
    #[test]
    fn later_input_in_the_same_batch_cannot_edit_after_open_has_started() {
        let (ctx, mut app) = setup();
        let before = app.editor.document.clone();
        let dir = tempfile::tempdir().unwrap();
        app.start_menu_open(dir.path().join("Missing.wireframe"), false, &ctx);
        // The outer shortcuts entry point already ran before Open began. Every
        // subsequent event must recheck the newly established modal boundary.
        assert!(!app.shortcut_event(&ctx, &delete_key()));
        assert_eq!(app.editor.document, before);
        assert!(app.startup_open.is_some());
    }
}
