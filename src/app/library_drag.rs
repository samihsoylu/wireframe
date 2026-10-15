// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

#[derive(Clone, Copy)]
pub(super) struct TileImage {
    pub origin: Pos2,
    pub scale: f32,
}
impl TileImage {
    pub fn new(tile: Rect, bounds: Bounds) -> Self {
        let size = Vec2::new(bounds.w.ceil().max(1.), bounds.h.ceil().max(1.));
        let scale = (68. / size.x).min(46. / size.y).min(1.);
        let fitted = size * scale;
        let left = ((68. - fitted.x) / 2.).round();
        // The source image region has an upward Y axis. Preserve its rounded
        // bottom inset when mapping to the canvas's downward Y axis.
        let top = 46. - ((46. - fitted.y) / 2.).round() - fitted.y;
        Self {
            origin: tile.min + Vec2::new(left, top) - Vec2::new(bounds.x, bounds.y) * scale,
            scale,
        }
    }
    pub fn anchor(&self, point: Pos2) -> Vec2 {
        // Padding is a valid grab point; do not clamp it to the image.
        (point - self.origin) / self.scale
    }
}

#[derive(Clone, Copy)]
pub(super) struct LibraryPress {
    kind: Kind,
    point: Pos2,
    image_area: Rect,
    image: TileImage,
}
impl LibraryPress {
    fn activation(&self, point: Pos2) -> Option<Vec2> {
        if point.distance(self.point) <= 6. {
            return None;
        }
        if self.image_area.contains(point) {
            Some(self.image.anchor(point))
        } else if self.image_area.contains(self.point) {
            // Fast/coalesced input can omit every intermediate source sample.
            // Preserve that usable gesture using its known image-area press.
            Some(self.image.anchor(self.point))
        } else {
            None
        }
    }
}

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
                && let Ok(scene) = self.previews.scene(
                    kind,
                    &self.editor.document.settings,
                    &self.editor.document.font,
                )
            {
                let image = TileImage::new(rect, scene.bounds);
                self.library_press = Some(LibraryPress {
                    kind,
                    point: pos,
                    image,
                    image_area: Rect::from_min_size(rect.min, Vec2::new(68., 46.))
                        .intersect(ui.clip_rect()),
                });
                self.library_anchor = image.anchor(pos);
            }
        }
    }

    // Resolve after viewport layout and before painting, so release has no
    // empty frame between the transient preview and the committed control.
    pub(super) fn library_drag_update(&mut self, ctx: &egui::Context) {
        let (focused, escape, held, released, pointer, activation) = ctx.input(|i| {
            // Consider only motion belonging to this press, ending at its release.
            // The GUI framework also starts drags after a stationary long hold.
            let first = i
                .events
                .iter()
                .rposition(|e| {
                    matches!(
                        e,
                        egui::Event::PointerButton {
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            ..
                        }
                    )
                })
                .map_or(0, |n| n + 1);
            let release = i.events[first..]
                .iter()
                .position(|e| {
                    matches!(
                        e,
                        egui::Event::PointerButton {
                            button: egui::PointerButton::Primary,
                            pressed: false,
                            ..
                        }
                    )
                })
                .map(|n| first + n);
            let pointer = release
                .and_then(|n| match i.events[n] {
                    egui::Event::PointerButton { pos, .. } => Some(pos),
                    _ => None,
                })
                .or_else(|| i.pointer.latest_pos());
            let activation = self.library_press.and_then(|press| {
                i.events[first..release.unwrap_or(i.events.len())]
                    .iter()
                    .filter_map(|e| match e {
                        egui::Event::PointerMoved(pos) => Some(*pos),
                        _ => None,
                    })
                    .find_map(|point| press.activation(point))
                    .or_else(|| pointer.and_then(|point| press.activation(point)))
            });
            (
                i.focused,
                i.key_pressed(Key::Escape),
                i.pointer.primary_down(),
                i.pointer.button_released(egui::PointerButton::Primary),
                pointer,
                activation,
            )
        });
        if !focused
            || escape
            || self.present
            || !self.show_library
            || self.image_crop.is_some()
            || self.save_busy()
            || self.recovery_chooser_open()
            || self.pending.is_some()
            || self.text_edit.is_some()
            || self.error.is_some()
            || self.recovery_offer.is_some()
        {
            self.library_drag = None;
            self.library_press = None;
            return;
        }
        if (held || released)
            && self.library_drag.is_none()
            && let (Some(press), Some(anchor)) = (self.library_press, activation)
        {
            self.library_anchor = anchor;
            self.library_drag = Some(press.kind);
        }
        if released || !held {
            self.library_press = None;
            let Some(kind) = self.library_drag.take() else {
                return;
            };
            if released && let Some(pos) = pointer.filter(|p| self.canvas_rect.contains(*p)) {
                let point = self.world((pos - self.library_anchor).max(self.canvas_rect.min));
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
                Self::paint_scene(&painter, scene, pos - self.library_anchor, 1.);
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
        frame_at(ctx, app, events, focused, None)
    }
    fn frame_at(
        ctx: &egui::Context,
        app: &mut WireframeApp,
        events: Vec<egui::Event>,
        focused: bool,
        time: Option<f64>,
    ) -> (Rect, Rect) {
        let mut source = Rect::NOTHING;
        let mut preview = Rect::NOTHING;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
                focused,
                events,
                time,
                ..Default::default()
            },
            |ui| {
                app.shortcuts(ui.ctx());
                egui::Panel::top("test-library").show(ui, |ui| {
                    let (r, response) =
                        ui.allocate_exact_size(Vec2::splat(68.), Sense::click_and_drag());
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
        let start = source.min + Vec2::new(34., 23.);
        frame(
            ctx,
            app,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
            true,
        );
        start
    }
    #[test]
    fn holding_library_tile_still_does_not_start_drag_but_later_motion_does() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        let before = app.editor.document.clone();
        let start = begin(&ctx, &mut app);
        let (_, bounds) = frame_at(&ctx, &mut app, vec![], true, Some(2.));
        assert!(
            app.library_drag.is_none(),
            "elapsed time alone must not start a drag"
        );
        assert!(!bounds.is_positive());
        let (_, bounds) = frame_at(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(start + Vec2::new(6., 0.))],
            true,
            Some(2.1),
        );
        assert!(
            app.library_drag.is_none(),
            "movement at the threshold stays pending"
        );
        assert!(!bounds.is_positive());
        let (_, bounds) = frame_at(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(start + Vec2::new(7., 0.))],
            true,
            Some(2.2),
        );
        assert_eq!(app.library_drag, Some(Kind::CircleButton));
        assert!(bounds.is_positive());
        assert_eq!(app.editor.document, before);
        let target = app.canvas_rect.center();
        frame_at(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(target), pointer(target, false)],
            true,
            Some(2.3),
        );
        assert_eq!(app.editor.current().controls.len(), 1);
        app.editor.undo();
        assert_eq!(app.editor.document, before);
    }
    #[test]
    fn library_motion_uses_press_to_release_path_and_ignores_later_hover() {
        for excursion in [false, true] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            frame(&ctx, &mut app, vec![], true);
            let (source, _) = frame(&ctx, &mut app, vec![], true);
            let start = source.min + Vec2::new(34., 23.);
            let mut events = vec![
                egui::Event::PointerMoved(Pos2::new(900., 600.)),
                egui::Event::PointerMoved(start),
                pointer(start, true),
            ];
            if excursion {
                events.push(egui::Event::PointerMoved(start + Vec2::new(8., 0.)));
                events.push(egui::Event::PointerMoved(start));
            }
            frame(&ctx, &mut app, events, true);
            assert_eq!(app.library_drag.is_some(), excursion);
            let target = app.canvas_rect.center();
            let anchor = if excursion {
                Vec2::new(31. * 48. / 46., 24.)
            } else {
                Vec2::splat(24.)
            };
            let expected = app.world(target - anchor);
            frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::PointerMoved(target),
                    pointer(target, false),
                    egui::Event::PointerMoved(Pos2::new(-100., -100.)),
                ],
                true,
            );
            assert_eq!(app.editor.current().controls.len(), 1);
            let b = app.editor.current().controls[0].bounds;
            assert_eq!((b.x, b.y), (expected.x, expected.y));
            app.editor.undo();
            assert!(app.editor.current().controls.is_empty());
        }
    }
    #[test]
    fn held_preview_follows_pointer_without_editing_then_drops_once_at_zoomed_position() {
        let mut preview_size: Option<Vec2> = None;
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
            if let Some(size) = preview_size {
                assert!((preview_bounds.size() - size).length() < 0.01);
            }
            preview_size = Some(preview_bounds.size());
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
            assert_eq!((b.x, b.y), (160. + 6. / zoom, 200. - 9. / zoom));
            frame(&ctx, &mut app, vec![], true);
            assert_eq!(app.editor.current().controls.len(), 2);
            app.editor.undo();
            assert_eq!(app.editor.document, before);
            app.editor.redo();
            assert_eq!(app.editor.current().controls.len(), 2);
        }
    }
    #[test]
    fn fitted_thumbnails_center_without_upscaling_and_allow_padding_grabs() {
        let tile = Rect::from_min_size(Pos2::new(10., 20.), Vec2::splat(68.));
        for (size, expected, scale) in [
            ([48., 48.], [11., 0.], 46. / 48.),
            ([60., 27.], [4., 9.], 1.),
            ([300., 200.], [0., 2. / 3.], 68. / 300.),
            ([20., 20.], [24., 13.], 1.),
        ] {
            let image = TileImage::new(tile, Bounds::new(0., 0., size[0], size[1]));
            assert!((image.origin - tile.min - Vec2::from(expected)).length() < 0.0001);
            assert!((image.scale - scale).abs() < 0.0001);
        }
        let circle = TileImage::new(tile, Bounds::new(0., 0., 48., 48.));
        assert!(
            (circle.anchor(tile.min + Vec2::new(34., 23.)) - Vec2::splat(24.)).length() < 0.0001
        );
        let left = circle.anchor(tile.min + Vec2::new(0., 23.));
        assert!((left.x + 11.478261).abs() < 0.0001);
        assert_eq!(left.y, 24.);
    }

    #[test]
    fn padding_grab_survives_drop_and_near_edge_origin_is_clamped() {
        for padding in [false, true] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            frame(&ctx, &mut app, vec![], true);
            let (source, _) = frame(&ctx, &mut app, vec![], true);
            let start = source.min + Vec2::new(if padding { 0. } else { 34. }, 23.);
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(start), pointer(start, true)],
                true,
            );
            let target = if padding {
                app.screen([160., 200.])
            } else {
                app.canvas_rect.min + Vec2::splat(2.)
            };
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(target)],
                true,
            );
            frame(&ctx, &mut app, vec![pointer(target, false)], true);
            assert_eq!(app.editor.current().controls.len(), 1);
            let b = app.editor.current().controls[0].bounds;
            if padding {
                assert!((b.x - 171.47826).abs() < 0.001);
                assert!((b.y - 176.).abs() < 0.001);
            } else {
                assert_eq!((b.x, b.y), (0., 0.));
            }
            app.editor.undo();
            assert!(app.editor.current().controls.is_empty());
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
        let start = source.min + Vec2::new(34., 23.);
        let target = app.canvas_rect.center();
        let expected = app.world(target - Vec2::splat(24.));
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
            assert_eq!((b.x, b.y), (160. - 24. / zoom, 200. - 24. / zoom));
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
    #[test]
    fn staged_drag_samples_the_activation_point_and_keeps_it_until_release() {
        for zoom in [0.5, 1., 2.] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.zoom = zoom;
            let start = begin(&ctx, &mut app);
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(start + Vec2::new(8., 4.))],
                true,
            );
            // 48-unit artwork is fitted to 46 points, with 11 points of left
            // padding. Activation is at image-field coordinate (42, 27).
            let anchor = Vec2::new(31. * 48. / 46., 27. * 48. / 46.);
            assert!((app.library_anchor - anchor).length() < 0.001);
            let target = app.screen([160., 200.]);
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(target)],
                true,
            );
            assert!((app.library_anchor - anchor).length() < 0.001);
            assert!(app.editor.current().controls.is_empty());
            frame(&ctx, &mut app, vec![pointer(target, false)], true);
            let b = app.editor.current().controls[0].bounds;
            assert!((b.x - (160. - anchor.x / zoom)).abs() < 0.001);
            assert!((b.y - (200. - anchor.y / zoom)).abs() < 0.001);
            app.editor.undo();
            assert!(app.editor.current().controls.is_empty());
        }
    }
    #[test]
    fn label_press_needs_to_enter_the_image_area_before_starting_a_drag() {
        for enters_image in [false, true] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            frame(&ctx, &mut app, vec![], true);
            let (source, _) = frame(&ctx, &mut app, vec![], true);
            let start = source.min + Vec2::new(34., 60.);
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(start), pointer(start, true)],
                true,
            );
            if enters_image {
                frame(
                    &ctx,
                    &mut app,
                    vec![egui::Event::PointerMoved(source.min + Vec2::new(34., 23.))],
                    true,
                );
                assert!(app.library_drag.is_some());
            }
            let target = app.canvas_rect.center();
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(target), pointer(target, false)],
                true,
            );
            assert_eq!(
                app.editor.current().controls.len(),
                usize::from(enters_image)
            );
        }
    }
    #[test]
    fn save_boundary_cancels_a_held_library_preview_before_it_can_insert() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.enable_persistence(&ctx, dir.path().to_owned());
        begin(&ctx, &mut app);
        let target = app.canvas_rect.center();
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(target)],
            true,
        );
        assert!(app.library_drag.is_some());
        let before = app.editor.document.clone();
        assert!(app.queue_save(dir.path().join("new.wireframe"), true));
        frame(&ctx, &mut app, vec![pointer(target, false)], true);
        assert_eq!(app.editor.document, before);
        assert!(app.library_drag.is_none() && app.library_press.is_none());
        assert!(app.editor.undo_name().is_none());
    }
}
