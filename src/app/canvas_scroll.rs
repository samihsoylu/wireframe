// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::viewport::{DragScroll, scroll_limits};

impl WireframeApp {
    pub(super) fn scroll_during_drag(&mut self, ctx: &egui::Context) -> bool {
        let active = matches!(
            self.drag,
            Some(Drag::Move { .. } | Drag::Resize { .. } | Drag::Marquee { .. })
        ) && ctx
            .input(|i| i.focused && i.pointer.primary_down() && !i.key_pressed(Key::Escape))
            && !self.present
            && !self.canvas_input_blocked();
        if !active {
            self.drag_scroll = None;
            return false;
        }
        let Some(pointer) = ctx.input(|i| i.pointer.latest_pos()) else {
            self.drag_scroll = None;
            return false;
        };
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
        let size = [self.canvas_rect.width(), self.canvas_rect.height()];
        let point = pointer - self.canvas_rect.min;
        let (time, moved) = ctx.input(|i| {
            (
                i.time,
                i.events
                    .iter()
                    .any(|event| matches!(event, egui::Event::PointerMoved(_))),
            )
        });
        let (delta, wait) = self
            .drag_scroll
            .get_or_insert_with(DragScroll::default)
            .advance(
                time,
                moved,
                [point.x, point.y],
                size,
                scroll_limits(content, size, self.zoom),
                [-self.pan.x, -self.pan.y],
            );
        self.pan -= Vec2::from(delta);
        if let Some(wait) = wait {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait.max(0.)));
        }
        delta != [0., 0.]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn button(pos: Pos2, pressed: bool) -> egui::Event {
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
        time: f64,
        events: Vec<egui::Event>,
        focused: bool,
    ) {
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
                time: Some(time),
                events,
                focused,
                ..Default::default()
            },
            |ui| {
                app.shortcuts(ui.ctx());
                app.canvas(ui);
            },
        );
        output.drop_without_applying_deltas();
    }

    #[test]
    fn held_move_scrolls_without_mouse_motion_and_remains_one_undo() {
        for zoom in [0.5, 1., 2.] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.zoom = zoom;
            app.editor.add(Kind::CircleButton, 850. / zoom, 300. / zoom);
            let before = app.editor.document.clone();
            frame(&ctx, &mut app, 0., vec![], true);
            let start = app.screen([850. / zoom + 24., 300. / zoom + 24.]);
            frame(
                &ctx,
                &mut app,
                0.01,
                vec![egui::Event::PointerMoved(start), button(start, true)],
                true,
            );
            let end = Pos2::new(995., start.y);
            frame(
                &ctx,
                &mut app,
                0.02,
                vec![egui::Event::PointerMoved(end)],
                true,
            );
            assert_eq!(app.pan.x, -1.);
            let first = app.editor.current().controls[0].bounds;
            frame(&ctx, &mut app, 0.08, vec![], true);
            assert_eq!(app.pan.x, -1.);
            frame(&ctx, &mut app, 0.13, vec![], true);
            assert_eq!(app.pan.x, -2.);
            let second = app.editor.current().controls[0].bounds;
            let expected =
                |scroll: f32| (850. / zoom + (end.x + scroll - start.x) / zoom + 0.5).floor();
            assert_eq!(first.x, expected(1.));
            assert_eq!(second.x, expected(2.));
            frame(&ctx, &mut app, 0.3, vec![button(end, false)], true);
            assert_eq!(app.pan.x, -2.);
            assert!(app.drag_scroll.is_none());
            let after = app.editor.document.clone();
            frame(&ctx, &mut app, 1., vec![], true);
            assert_eq!(app.editor.document, after);
            app.editor.undo();
            assert_eq!(app.editor.document, before);
            app.editor.redo();
            assert_eq!(app.editor.document, after);
        }
    }

    #[test]
    fn stationary_resize_and_marquee_follow_the_scrolled_viewport() {
        for resize in [false, true] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor
                .add(Kind::Rectangle, if resize { 650. } else { 1000. }, 50.);
            let before = app.editor.document.clone();
            frame(&ctx, &mut app, 0., vec![], true);
            let initial = app.editor.current().controls[0].bounds;
            let start = if resize {
                app.screen_rect(initial).right_bottom() + Vec2::splat(4.)
            } else {
                Pos2::new(20., 20.)
            };
            frame(
                &ctx,
                &mut app,
                0.01,
                vec![egui::Event::PointerMoved(start), button(start, true)],
                true,
            );
            let end = Pos2::new(995., 650.);
            frame(
                &ctx,
                &mut app,
                0.02,
                vec![egui::Event::PointerMoved(end)],
                true,
            );
            assert!(if resize {
                matches!(app.drag, Some(Drag::Resize { .. }))
            } else {
                matches!(app.drag, Some(Drag::Marquee { .. }))
            });
            let first = app.editor.current().controls[0].bounds;
            for tick in 1..=250 {
                frame(&ctx, &mut app, tick as f64 * 0.11, vec![], true);
            }
            assert!(app.pan.x < -200.);
            if resize {
                assert!(app.editor.current().controls[0].bounds.w > first.w + 200.);
            } else {
                assert_eq!(app.editor.document, before);
                assert_eq!(app.editor.selection.len(), 1);
            }
            frame(&ctx, &mut app, 30., vec![button(end, false)], true);
            if resize {
                app.editor.undo();
                assert_eq!(app.editor.document, before);
            }
        }
    }

    #[test]
    fn focus_loss_and_escape_end_edge_scrolling_and_cancel_edit() {
        for escape in [false, true] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(Kind::CircleButton, 850., 300.);
            let before = app.editor.document.clone();
            frame(&ctx, &mut app, 0., vec![], true);
            let start = app.screen([874., 324.]);
            frame(
                &ctx,
                &mut app,
                0.01,
                vec![egui::Event::PointerMoved(start), button(start, true)],
                true,
            );
            frame(
                &ctx,
                &mut app,
                0.02,
                vec![egui::Event::PointerMoved(Pos2::new(995., 324.))],
                true,
            );
            assert_eq!(app.pan.x, -1.);
            let events = if escape {
                vec![egui::Event::Key {
                    key: Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }]
            } else {
                vec![]
            };
            frame(&ctx, &mut app, 0.2, events, escape);
            assert!(app.drag_scroll.is_none());
            assert_eq!(app.editor.document, before);
            frame(&ctx, &mut app, 1., vec![], escape);
            assert_eq!(app.pan.x, -1.);
        }
    }
    #[test]
    fn saving_finishes_the_visible_canvas_move_and_stops_edge_scrolling() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.enable_persistence(&ctx, dir.path().to_owned());
        app.editor.add(Kind::CircleButton, 850., 300.);
        let before = app.editor.document.clone();
        frame(&ctx, &mut app, 0., vec![], true);
        let start = app.screen([874., 324.]);
        frame(
            &ctx,
            &mut app,
            0.01,
            vec![egui::Event::PointerMoved(start), button(start, true)],
            true,
        );
        let end = Pos2::new(995., start.y);
        frame(
            &ctx,
            &mut app,
            0.02,
            vec![egui::Event::PointerMoved(end)],
            true,
        );
        assert!(app.drag.is_some());
        assert_eq!(app.pan.x, -1.);
        let captured = app.editor.document.clone();
        assert!(app.queue_save(dir.path().join("gesture.wireframe"), true));
        assert!(app.drag.is_none() && app.drag_scroll.is_none());
        frame(
            &ctx,
            &mut app,
            0.2,
            vec![egui::Event::PointerMoved(end + Vec2::new(30., 20.))],
            true,
        );
        assert_eq!(app.editor.document, captured);
        assert_eq!(app.pan.x, -1.);
        frame(&ctx, &mut app, 0.3, vec![button(end, false)], true);
        assert_eq!(app.editor.document, captured);
        app.editor.undo();
        assert_eq!(app.editor.document, before);
    }
}
