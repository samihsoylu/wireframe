// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

pub(super) struct UnlockPress {
    id: Id,
    start: Pos2,
    rect: Rect,
    surface: Id,
    context: Vec<Id>,
    session: String,
    pub(super) cancelled: bool,
}

impl WireframeApp {
    fn lock_badge_at(&self, pos: Pos2) -> Option<(Id, Rect)> {
        self.editor.current().controls.iter().rev().find_map(|c| {
            let rect = Rect::from_min_size(
                self.screen_rect(c.bounds).left_top() - Vec2::new(18., 0.),
                Vec2::splat(18.),
            );
            (c.locked && self.editor.selection.contains(&c.id) && rect.contains(pos))
                .then_some((c.id, rect))
        })
    }

    pub(super) fn unlock_badge_input(&mut self, ui: &egui::Ui, response: &egui::Response) -> bool {
        let ctx = ui.ctx();
        let blocked = !ui.is_enabled()
            || self.canvas_input_blocked()
            || self.text_edit.is_some()
            || self.library_drag.is_some()
            || self.library_press.is_some()
            || self.drag.is_some()
            || self.drag_scroll.is_some()
            || !ui.input(|i| i.focused)
            || ui.input(|i| i.key_pressed(Key::Escape))
            || ctx.any_popup_open()
            || egui::Popup::is_any_open(ctx);
        if let Some(press) = &mut self.unlock_press {
            press.cancelled |= blocked
                || press.surface != self.editor.surface_id()
                || press.context != self.editor.context
                || press.session != self.editor.clipboard_session()
                || !self.editor.selection.contains(&press.id);
        }
        let mut used = self.unlock_press.is_some();
        for event in ui.input(|i| i.events.clone()) {
            match event {
                egui::Event::WindowFocused(false)
                | egui::Event::PointerGone
                | egui::Event::Key {
                    key: Key::Escape,
                    pressed: true,
                    ..
                } => {
                    if let Some(press) = &mut self.unlock_press {
                        press.cancelled = true;
                    }
                }
                egui::Event::PointerMoved(pos) => {
                    if let Some(press) = &mut self.unlock_press {
                        press.cancelled |= press.start.distance(pos) > 6.;
                    }
                }
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    ..
                } if !blocked
                    && !ui.input(|i| {
                        i.key_down(Key::Space) || i.pointer.button_down(egui::PointerButton::Middle)
                    })
                    && response.rect.contains(pos)
                    && ctx.layer_id_at(pos) == Some(response.layer_id) =>
                {
                    if let Some((id, rect)) = self.lock_badge_at(pos) {
                        self.unlock_press = Some(UnlockPress {
                            id,
                            start: pos,
                            rect,
                            surface: self.editor.surface_id(),
                            context: self.editor.context.clone(),
                            session: self.editor.clipboard_session().into(),
                            cancelled: false,
                        });
                        self.canvas_press = None;
                        used = true;
                    }
                }
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers,
                } => {
                    if let Some(press) = self.unlock_press.take() {
                        used = true;
                        if !press.cancelled
                            && press.start.distance(pos) <= 6.
                            && press.rect.contains(pos)
                            && response.rect.contains(pos)
                            && !modifiers.shift
                            && !modifiers.command
                            && !modifiers.ctrl
                        {
                            if let Some(id) = ctx.memory(|m| m.focused()) {
                                ctx.memory_mut(|m| m.surrender_focus(id));
                            }
                            self.editor.unlock_selected_control(press.id);
                        }
                    }
                }
                _ => {}
            }
        }
        if used {
            self.canvas_press = None;
            if !ui.input(|i| i.pointer.primary_down()) {
                self.unlock_press = None;
            }
        } else if !blocked
            && let Some(pos) = response.hover_pos()
            && let Some((id, _)) = self.lock_badge_at(pos)
        {
            if self.editor.own_locked(id) {
                response
                    .clone()
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("Unlock control");
            } else {
                response
                    .clone()
                    .on_hover_text("Locked by a containing group");
            }
        }
        used
    }

    pub(super) fn paint_locked_selection(&self, painter: &egui::Painter, bounds: Bounds) {
        let r = self.screen_rect(bounds);
        let path = [
            r.left_top() - Vec2::new(18., 0.),
            r.right_top(),
            r.right_bottom(),
            r.left_bottom(),
            r.left_top() + Vec2::new(0., 18.),
            r.left_top() + Vec2::new(-18., 18.),
            r.left_top() - Vec2::new(18., 0.),
        ];
        painter.extend(egui::Shape::dashed_line(
            &path,
            Stroke::new(1., Color32::from_gray(153)),
            4.,
            4.,
        ));
        // The badge stays legible at every canvas zoom. Its body and shackle
        // are original geometry, separate from document artwork.
        let origin = r.left_top() + Vec2::new(-16., 2.);
        painter.rect(
            Rect::from_min_size(origin + Vec2::new(3., 0.), Vec2::new(8., 10.)),
            4,
            Color32::WHITE,
            Stroke::new(1., Color32::BLACK),
            StrokeKind::Inside,
        );
        painter.rect(
            Rect::from_min_size(origin + Vec2::new(1., 6.), Vec2::new(12., 8.)),
            1,
            Color32::WHITE,
            Stroke::new(1., Color32::BLACK),
            StrokeKind::Inside,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ctx: &egui::Context, app: &mut WireframeApp, events: Vec<egui::Event>) {
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
                events,
                ..Default::default()
            },
            |ui| app.canvas(ui),
        )
        .drop_without_applying_deltas();
    }
    fn pointer(pos: Pos2, pressed: bool, modifiers: Modifiers) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers,
        }
    }
    fn fixture(ctx: &egui::Context) -> (WireframeApp, Id, Pos2) {
        let mut app = WireframeApp::workspace(ctx, None);
        app.editor.add(Kind::Button, 100., 100.);
        let id = *app.editor.selection.iter().next().unwrap();
        app.editor.add(Kind::Button, 300., 100.);
        app.editor.select_all();
        app.editor.lock(true);
        frame(ctx, &mut app, vec![]);
        let badge = app.screen([100., 100.]) + Vec2::new(-9., 9.);
        (app, id, badge)
    }

    #[test]
    fn badge_click_unlocks_one_control_with_exact_history_in_batched_and_split_frames() {
        for split in [false, true] {
            let ctx = egui::Context::default();
            let (mut app, id, pos) = fixture(&ctx);
            let before = app.editor.document.clone();
            let press = vec![
                egui::Event::PointerMoved(pos),
                pointer(pos, true, Modifiers::NONE),
            ];
            if split {
                frame(&ctx, &mut app, press);
                assert_eq!(app.editor.document, before);
                assert_eq!(app.editor.selection.len(), 2);
                frame(&ctx, &mut app, vec![pointer(pos, false, Modifiers::NONE)]);
            } else {
                frame(
                    &ctx,
                    &mut app,
                    [press, vec![pointer(pos, false, Modifiers::NONE)]].concat(),
                );
            }
            let mut expected = before.clone();
            expected.pages[0].content.nodes[0].locked = false;
            assert_eq!(app.editor.document, expected);
            assert_eq!(app.editor.selection, HashSet::from([id]));
            assert!(app.drag.is_none() && app.unlock_press.is_none());
            assert!(app.text_edit.is_none());
            app.editor.undo();
            assert_eq!(app.editor.document, before);
            assert_eq!(app.editor.selection, HashSet::from([id]));
            app.editor.redo();
            assert_eq!(app.editor.document, expected);
        }
    }

    #[test]
    fn badge_release_modifiers_and_cancellation_preserve_selection_data_and_redo() {
        for cancel in [
            "drag", "return", "shift", "command", "escape", "focus", "modal", "deselect",
        ] {
            let ctx = egui::Context::default();
            let (mut app, _, pos) = fixture(&ctx);
            app.editor.add(Kind::Label, 500., 300.);
            let redo = app.editor.document.clone();
            app.editor.undo();
            let before = app.editor.document.clone();
            let selection = app.editor.selection.clone();
            frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::PointerMoved(pos),
                    pointer(pos, true, Modifiers::NONE),
                ],
            );
            let end = pos + Vec2::new(70., 60.);
            let mut release_pos = pos;
            let mut modifiers = Modifiers::NONE;
            match cancel {
                "drag" | "return" => {
                    frame(&ctx, &mut app, vec![egui::Event::PointerMoved(end)]);
                    if cancel == "drag" {
                        release_pos = end;
                    }
                }
                "shift" => modifiers = Modifiers::SHIFT,
                "command" => modifiers = Modifiers::COMMAND,
                "escape" => frame(
                    &ctx,
                    &mut app,
                    vec![egui::Event::Key {
                        key: Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::NONE,
                    }],
                ),
                "focus" => frame(&ctx, &mut app, vec![egui::Event::WindowFocused(false)]),
                "modal" => {
                    app.error = Some("Test error".into());
                    frame(&ctx, &mut app, vec![]);
                    app.error = None;
                }
                "deselect" => {
                    app.editor.selection.clear();
                    frame(&ctx, &mut app, vec![]);
                    app.editor.selection = selection.clone();
                }
                _ => unreachable!(),
            }
            // The release snapshot remains authoritative even after modifier-up.
            frame(
                &ctx,
                &mut app,
                vec![
                    pointer(release_pos, false, modifiers),
                    egui::Event::ModifiersChanged(Modifiers::NONE),
                ],
            );
            assert_eq!(app.editor.document, before, "{cancel}");
            assert_eq!(app.editor.selection, selection, "{cancel}");
            assert!(app.drag.is_none() && app.unlock_press.is_none(), "{cancel}");
            app.editor.redo();
            assert_eq!(app.editor.document, redo, "{cancel}");
        }
    }

    #[test]
    fn badge_follows_drawn_full_group_bounds_at_all_zooms_with_hidden_markup() {
        for zoom in [0.5, 1., 2.] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(Kind::Button, 100., 100.);
            let marked = *app.editor.selection.iter().next().unwrap();
            app.editor.set_markup(marked, true).unwrap();
            app.editor.add(Kind::Button, 399., 100.);
            app.editor.select_all();
            app.editor.group();
            let id = *app.editor.selection.iter().next().unwrap();
            app.editor.lock(true);
            app.editor.set_markup_visible(false);
            frame(&ctx, &mut app, vec![]);
            app.zoom = zoom;
            let pos = app.screen([100., 100.]) + Vec2::new(-9., 9.);
            assert_eq!(app.lock_badge_at(pos).map(|x| x.0), Some(id));
            frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::PointerMoved(pos),
                    pointer(pos, true, Modifiers::NONE),
                    pointer(pos, false, Modifiers::NONE),
                ],
            );
            assert!(!app.editor.own_locked(id), "zoom {zoom}");
            assert!(!app.editor.markup_visible());
        }
    }
    #[test]
    fn badge_release_cannot_target_a_replacement_document_with_reused_ids() {
        let ctx = egui::Context::default();
        let (mut app, id, pos) = fixture(&ctx);
        frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(pos),
                pointer(pos, true, Modifiers::NONE),
            ],
        );
        let document = app.editor.document.clone();
        app.editor = Editor::new(document.clone());
        app.editor.select(id, false);
        frame(&ctx, &mut app, vec![pointer(pos, false, Modifiers::NONE)]);
        assert_eq!(app.editor.document, document);
        assert!(app.editor.own_locked(id));
        assert!(app.editor.undo_name().is_none());
    }
}
