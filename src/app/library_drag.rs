// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl WireframeApp {
    pub(super) fn library_drag_source(
        &mut self,
        ui: &egui::Ui,
        kind: Kind,
        rect: Rect,
        response: &egui::Response,
    ) {
        if !response.enabled() || ui.input(|i| i.key_pressed(Key::Escape)) {
            return;
        }
        let visible = rect.intersect(ui.clip_rect());
        for event in ui.input(|i| i.events.clone()) {
            if let egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                ..
            } = event
                && visible.contains(pos)
                && ui.ctx().layer_id_at(pos) == Some(ui.layer_id())
            {
                self.library_press = Some((kind, pos));
            }
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
            self.library_drag = Some(kind);
        }
    }

    // Resolve after viewport layout and before painting, so release has no
    // empty frame between the transient preview and the committed control.
    pub(super) fn library_drag_update(&mut self, ctx: &egui::Context) {
        let (focused, escape, held, released, pointer) = ctx.input(|i| {
            (
                i.focused,
                i.key_pressed(Key::Escape),
                i.pointer.primary_down(),
                i.pointer.button_released(egui::PointerButton::Primary),
                i.pointer.latest_pos(),
            )
        });
        if !focused
            || escape
            || self.present
            || !self.show_library
            || self.image_crop.is_some()
            || self.pending.is_some()
            || self.text_edit.is_some()
            || self.error.is_some()
            || self.recovery_offer.is_some()
        {
            self.library_drag = None;
            self.library_press = None;
            return;
        }
        if released || !held {
            if released
                && self.library_drag.is_none()
                && let Some((kind, start)) = self.library_press
                && pointer.is_some_and(|pos| pos.distance(start) > 6.)
            {
                self.library_drag = Some(kind);
            }
            self.library_press = None;
            let Some(kind) = self.library_drag.take() else {
                return;
            };
            if released && let Some(pos) = pointer.filter(|p| self.canvas_rect.contains(*p)) {
                let point = self.world(pos);
                self.editor.add(kind, point.x, point.y);
                ctx.request_repaint();
            }
        }
    }

    pub(super) fn library_drag_overlay(&mut self, ctx: &egui::Context) {
        let Some(kind) = self.library_drag else {
            return;
        };
        if let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) {
            let valid = self.canvas_rect.contains(pos);
            ctx.set_cursor_icon(if valid {
                egui::CursorIcon::Grabbing
            } else {
                egui::CursorIcon::NotAllowed
            });
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("library-drag-preview"),
            ));
            if let Ok(scene) = self.previews.scene(
                kind,
                &self.editor.document.settings,
                &self.editor.document.font,
            ) {
                Self::paint_scene(&painter, scene, pos, self.zoom);
            }
        }
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pointer(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }
    fn frame(
        ctx: &egui::Context,
        app: &mut WireframeApp,
        events: Vec<egui::Event>,
        focused: bool,
    ) -> (Rect, Rect) {
        let mut source = Rect::NOTHING;
        let mut preview = Rect::NOTHING;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
                focused,
                events,
                ..Default::default()
            },
            |ui| {
                app.shortcuts(ui.ctx());
                egui::Panel::top("test-library").show(ui, |ui| {
                    let (r, response) =
                        ui.allocate_exact_size(Vec2::new(94., 68.), Sense::click_and_drag());
                    source = r;
                    app.library_drag_source(ui, Kind::CircleButton, r, &response);
                });
                app.canvas(ui);
                app.library_drag_overlay(ui.ctx());
                preview = ctx.graphics(|g| {
                    g.get(egui::LayerId::new(
                        egui::Order::Tooltip,
                        egui::Id::new("library-drag-preview"),
                    ))
                    .map(|list| {
                        list.all_entries().fold(Rect::NOTHING, |bounds, item| {
                            bounds.union(item.shape.visual_bounding_rect())
                        })
                    })
                    .unwrap_or(Rect::NOTHING)
                });
            },
        );
        output.textures_delta.clear();
        (source, preview)
    }
    fn begin(ctx: &egui::Context, app: &mut WireframeApp) -> Pos2 {
        frame(ctx, app, vec![], true);
        let (source, _) = frame(ctx, app, vec![], true);
        let start = source.center();
        frame(
            ctx,
            app,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
            true,
        );
        start
    }
    #[test]
    fn held_preview_follows_pointer_without_editing_then_drops_once_at_zoomed_position() {
        for zoom in [0.5, 1., 2.] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(Kind::Label, 10., 20.);
            let before = app.editor.document.clone();
            let selection = app.editor.selection.clone();
            app.zoom = zoom;
            app.pan = Vec2::new(23., -17.);
            let start = begin(&ctx, &mut app);
            assert!(app.library_drag.is_none());
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(start + Vec2::splat(1.))],
                true,
            );
            assert!(app.library_drag.is_none());
            let target = app.screen([160., 200.]);
            let (_, preview_bounds) = frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(target)],
                true,
            );
            assert_eq!(app.library_drag, Some(Kind::CircleButton));
            assert_eq!(app.editor.document, before);
            assert_eq!(app.editor.selection, selection);
            assert!(preview_bounds.is_positive());
            let target = target + Vec2::new(30., 15.);
            let (_, moved_bounds) = frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(target)],
                true,
            );
            assert_eq!(app.editor.document, before);
            assert!((moved_bounds.min - preview_bounds.min - Vec2::new(30., 15.)).length() < 0.01);
            assert!((moved_bounds.size() - preview_bounds.size()).length() < 0.01);
            frame(&ctx, &mut app, vec![pointer(target, false)], true);
            assert!(app.library_drag.is_none());
            assert_eq!(app.editor.current().controls.len(), 2);
            let b = app.editor.current().controls[1].bounds;
            assert_eq!((b.x, b.y), (160. + 30. / zoom, 200. + 15. / zoom));
            frame(&ctx, &mut app, vec![], true);
            assert_eq!(app.editor.current().controls.len(), 2);
            app.editor.undo();
            assert_eq!(app.editor.document, before);
            app.editor.redo();
            assert_eq!(app.editor.current().controls.len(), 2);
        }
    }
    #[test]
    fn cancelled_and_outside_drags_preserve_document_selection_and_history() {
        for mode in ["escape", "focus", "outside", "gone"] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            let empty = app.editor.document.clone();
            app.editor.add(Kind::Button, 20., 30.);
            let before = app.editor.document.clone();
            let selection = app.editor.selection.clone();
            let start = begin(&ctx, &mut app);
            let target = app.canvas_rect.center();
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(target)],
                true,
            );
            assert!(app.library_drag.is_some());
            match mode {
                "escape" => {
                    frame(
                        &ctx,
                        &mut app,
                        vec![egui::Event::Key {
                            key: Key::Escape,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: Modifiers::NONE,
                        }],
                        true,
                    );
                }
                "focus" => {
                    frame(&ctx, &mut app, vec![], false);
                }
                "gone" => {
                    frame(&ctx, &mut app, vec![egui::Event::PointerGone], false);
                }
                _ => {
                    frame(
                        &ctx,
                        &mut app,
                        vec![egui::Event::PointerMoved(start), pointer(start, false)],
                        true,
                    );
                }
            }
            frame(&ctx, &mut app, vec![pointer(target, false)], true);
            assert!(app.library_drag.is_none(), "{mode}");
            assert_eq!(app.editor.document, before, "{mode}");
            assert_eq!(app.editor.selection, selection, "{mode}");
            app.editor.undo();
            assert_eq!(app.editor.document, empty, "{mode}");
        }
    }
    #[test]
    fn library_drag_coalesced_into_one_frame_inserts_exactly_once() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        frame(&ctx, &mut app, vec![], true);
        let (source, _) = frame(&ctx, &mut app, vec![], true);
        let start = source.center();
        let target = app.canvas_rect.center();
        let expected = app.world(target);
        frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(start),
                pointer(start, true),
                egui::Event::PointerMoved(target),
                pointer(target, false),
            ],
            true,
        );
        assert_eq!(app.editor.current().controls.len(), 1);
        let b = app.editor.current().controls[0].bounds;
        assert_eq!((b.x, b.y), (expected.x, expected.y));
        app.editor.undo();
        assert!(app.editor.current().controls.is_empty());
    }
    #[test]
    fn library_drag_with_press_in_previous_frame_inserts_once() {
        for zoom in [0.5, 1., 2.] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.zoom = zoom;
            let before = app.editor.document.clone();
            begin(&ctx, &mut app);
            let target = app.screen([160., 200.]);
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(target), pointer(target, false)],
                true,
            );
            assert_eq!(app.editor.current().controls.len(), 1);
            let b = app.editor.current().controls[0].bounds;
            assert_eq!((b.x, b.y), (160., 200.));
            frame(&ctx, &mut app, vec![], true);
            assert_eq!(app.editor.current().controls.len(), 1);
            app.editor.undo();
            assert_eq!(app.editor.document, before);
        }
    }
    #[test]
    fn pending_library_press_cancels_without_late_insertion() {
        for mode in ["escape", "focus", "hidden", "click", "outside"] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(Kind::Label, 10., 20.);
            let before = app.editor.document.clone();
            let selection = app.editor.selection.clone();
            let start = begin(&ctx, &mut app);
            let target = app.canvas_rect.center();
            match mode {
                "escape" => {
                    frame(
                        &ctx,
                        &mut app,
                        vec![egui::Event::Key {
                            key: Key::Escape,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: Modifiers::NONE,
                        }],
                        true,
                    );
                }
                "focus" => {
                    frame(&ctx, &mut app, vec![], false);
                }
                "hidden" => {
                    app.show_library = false;
                    frame(&ctx, &mut app, vec![], true);
                    app.show_library = true;
                }
                "click" => {
                    frame(&ctx, &mut app, vec![pointer(start, false)], true);
                }
                _ => {
                    frame(
                        &ctx,
                        &mut app,
                        vec![
                            egui::Event::PointerMoved(Pos2::new(-20., -20.)),
                            pointer(Pos2::new(-20., -20.), false),
                        ],
                        true,
                    );
                }
            }
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(target), pointer(target, false)],
                true,
            );
            assert_eq!(app.editor.document, before, "{mode}");
            assert_eq!(app.editor.selection, selection, "{mode}");
            assert!(
                app.library_drag.is_none() && app.library_press.is_none(),
                "{mode}"
            );
        }
    }
}
