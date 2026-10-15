// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
impl WireframeApp {
    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        // A held library control is transient; no document command may run underneath it.
        if self.library_drag.is_some() || self.library_press.is_some() {
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                self.library_drag = None;
                self.library_press = None;
            }
            return;
        }
        // Inspect each event once, in arrival order. A Save observes all earlier
        // document commands and none of the later commands in the same frame.
        let events = ctx.input_mut(|i| std::mem::take(&mut i.events));
        let mut remaining = Vec::with_capacity(events.len());
        for event in events {
            if !self.shortcut_event(ctx, &event) {
                remaining.push(event);
            }
        }
        ctx.input_mut(|i| {
            remaining.append(&mut i.events);
            i.events = remaining;
        });
    }
    fn shortcut_event(&mut self, ctx: &egui::Context, event: &egui::Event) -> bool {
        // Re-evaluate after each command: New/Open/Enter can establish a modal
        // boundary that later events must not bypass.
        if self.font_import.is_some()
            || self.pdf_options.is_some()
            || self.save_busy()
            || self.recovery_chooser_open()
            || self.pending.is_some()
            || self.text_edit.is_some()
            || self.recovery_offer.is_some()
            || self.image_crop.is_some()
        {
            return false;
        }
        let is = |m, k| matches!(event, egui::Event::Key {key,modifiers,pressed:true,..} if *key==k && modifiers.matches_logically(m));
        if self.present {
            if is(Modifiers::NONE, Key::Escape) {
                self.present = false;
                return true;
            }
            return false;
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
            self.show_inspector = !self.show_inspector;
        } else if is(Modifiers::COMMAND, Key::Num0) {
            self.fit();
        } else if is(Modifiers::COMMAND, Key::Num1) {
            self.zoom = 1.;
        } else if is(Modifiers::COMMAND | Modifiers::SHIFT, Key::F) {
            self.editor.commit();
            self.presentation_history.clear();
            self.present = true;
        } else {
            // Text editors and popups retain their own input, including local Undo.
            if wireframe::text_entry::is_focused(ctx)
                || ctx.text_edit_focused()
                || ctx.any_popup_open()
                || self.quick_focus
            {
                return false;
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
                self.editor.select_all();
            } else if is(Modifiers::COMMAND, Key::D) {
                self.editor.duplicate();
            } else if is(Modifiers::COMMAND | Modifiers::SHIFT, Key::G) {
                self.editor.ungroup();
            } else if is(Modifiers::COMMAND, Key::G) {
                self.editor.group();
            } else if is(Modifiers::NONE, Key::Delete) || is(Modifiers::NONE, Key::Backspace) {
                self.editor.delete();
            } else if is(Modifiers::NONE, Key::Escape) {
                self.present = false;
                self.editor.cancel();
                self.drag = None;
                self.alignment_guides.clear();
                if !self.editor.leave() {
                    self.editor.selection.clear();
                }
            } else if is(Modifiers::NONE, Key::Enter) {
                if let Some(c) = self.editor.selected_views().first()
                    && !self.editor.enter(c.id)
                    && c.group.is_none()
                    && c.kind.text_editable()
                {
                    self.begin_text_edit(c.id);
                }
            } else {
                for (k, dx, dy) in [
                    (Key::ArrowLeft, -1., 0.),
                    (Key::ArrowRight, 1., 0.),
                    (Key::ArrowUp, 0., -1.),
                    (Key::ArrowDown, 0., 1.),
                ] {
                    if is(Modifiers::SHIFT, k) {
                        self.editor.move_by(dx * 10., dy * 10.);
                        return true;
                    }
                    if is(Modifiers::NONE, k) {
                        self.editor.move_by(dx, dy);
                        return true;
                    }
                }
                match event {
                    egui::Event::Copy => self.copy(ctx),
                    egui::Event::Cut => {
                        self.copy(ctx);
                        self.editor.delete();
                    }
                    egui::Event::Paste(s) => self.pasted(s),
                    _ => return false,
                }
            }
        }
        true
    }
}
