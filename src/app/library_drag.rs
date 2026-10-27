// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

#[derive(Clone, Copy)]
pub(super) struct TileImage {
    pub origin: Pos2,
    pub scale: f32,
}
impl TileImage {
    pub fn for_item(item: CatalogItem, tile: Rect, bounds: Bounds) -> Self {
        if matches!(item, CatalogItem::Icon(_)) {
            let scale = icon_preview_scale(bounds);
            let size = Vec2::new(bounds.w, bounds.h) * scale;
            Self {
                origin: tile.min + (Vec2::new(68., 46.) - size) / 2.
                    - Vec2::new(bounds.x, bounds.y) * scale,
                scale,
            }
        } else {
            Self::new(tile, bounds)
        }
    }
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

fn icon_preview_scale(bounds: Bounds) -> f32 {
    46. / bounds.w.max(bounds.h).max(1.)
}

fn control_preview_scale(item: CatalogItem) -> f32 {
    // Built-in controls drag their full-resolution thumbnail, independently
    // of canvas zoom. Icon and user-resource previews retain their own sizes.
    if matches!(item, CatalogItem::Control(_) | CatalogItem::Preset(_)) {
        2.
    } else {
        1.
    }
}

#[derive(Clone)]
pub(super) struct LibraryPress {
    item: CatalogItem,
    surface: Id,
    context: Vec<Id>,
    session: String,
    preview: std::sync::Arc<wireframe::render::Scene>,
    point: Pos2,
    image_area: Rect,
    provider_origin: Pos2,
    image: TileImage,
}
impl LibraryPress {
    fn anchor(&self, point: Pos2) -> Vec2 {
        if matches!(self.item, CatalogItem::Icon(_)) {
            // Individual icons use the complete provider area, including padding.
            // The preview offset retains the provider's inverted vertical axis.
            let local = point - self.provider_origin;
            Vec2::new(local.x * 46. / 68., 46. - local.y)
        } else {
            self.image.anchor(point) * control_preview_scale(self.item)
        }
    }
    fn activation(&self, point: Pos2) -> Option<Vec2> {
        if point == self.point {
            return None;
        }
        if self.image_area.contains(point) {
            Some(self.anchor(point))
        } else if self.image_area.contains(self.point) {
            // Fast/coalesced input can omit every intermediate source sample.
            // Preserve that usable gesture using its known image-area press.
            Some(self.anchor(self.point))
        } else {
            None
        }
    }
}

impl WireframeApp {
    pub(super) fn library_drag_source(
        &mut self,
        ui: &egui::Ui,
        item: CatalogItem,
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
                && let Ok(scene) = self.previews.item_scene(item, &self.editor.document)
            {
                let image = TileImage::for_item(item, rect, scene.bounds);
                let press = LibraryPress {
                    item,
                    surface: self.editor.surface_id(),
                    context: self.editor.context.clone(),
                    session: self.editor.clipboard_session().to_owned(),
                    preview: scene,
                    point: pos,
                    image,
                    provider_origin: rect.min,
                    image_area: Rect::from_min_size(rect.min, Vec2::new(68., 46.))
                        .intersect(ui.clip_rect()),
                };
                self.library_anchor = press.anchor(pos);
                self.library_press = Some(press);
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
            let activation = self.library_press.as_ref().and_then(|press| {
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
        let stale = self.library_press.as_ref().is_some_and(|press| {
            press.surface != self.editor.surface_id()
                || press.context != self.editor.context
                || press.session != self.editor.clipboard_session()
                || !self.catalog_item_enabled(press.item)
        });
        let changed_preview = if let Some(press) = &self.library_press {
            self.previews
                .item_scene(press.item, &self.editor.document)
                .map_or(true, |scene| {
                    !std::sync::Arc::ptr_eq(&scene, &press.preview)
                })
        } else {
            false
        };
        if !focused
            || stale
            || changed_preview
            || self.canvas_input_blocked()
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
            && let (Some(press), Some(anchor)) = (self.library_press.as_ref(), activation)
        {
            self.library_anchor = anchor;
            self.library_drag = Some(press.item);
        }
        if released || !held {
            self.library_press = None;
            let Some(item) = self.library_drag.take() else {
                return;
            };
            if released && let Some(pos) = pointer.filter(|p| self.canvas_rect.contains(*p)) {
                let point = self.world((pos - self.library_anchor).max(self.canvas_rect.min));
                // Placement uses whole document points, with half points
                // rounded toward positive infinity on either side of zero.
                let point = Pos2::new((point.x + 0.5).floor(), (point.y + 0.5).floor());
                self.insert_catalog_at(item, point);
                ctx.request_repaint();
            }
        }
    }

    pub(super) fn library_drag_overlay(&mut self, ctx: &egui::Context) {
        let Some(_item) = self.library_drag else {
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
            if let Some(press) = &self.library_press {
                let (scale, offset) = if matches!(press.item, CatalogItem::Icon(_)) {
                    let bounds = press.preview.bounds;
                    let scale = icon_preview_scale(bounds);
                    (scale, Vec2::new(bounds.x, bounds.y) * scale)
                } else {
                    (control_preview_scale(press.item), Vec2::ZERO)
                };
                Self::paint_scene(
                    &painter,
                    &press.preview,
                    pos - self.library_anchor - offset,
                    scale,
                );
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
        frame_item(
            ctx,
            app,
            events,
            focused,
            time,
            CatalogItem::Control(Kind::CircleButton),
        )
    }
    fn frame_item(
        ctx: &egui::Context,
        app: &mut WireframeApp,
        events: Vec<egui::Event>,
        focused: bool,
        time: Option<f64>,
        item: CatalogItem,
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
                    app.library_drag_source(ui, item, r, &response);
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
    fn preset_held_preview_and_release_keep_the_selected_recipe() {
        for preset in wireframe::workspace::CatalogPreset::ALL {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            let item = CatalogItem::Preset(preset);
            let before = app.editor.document.clone();
            frame_item(&ctx, &mut app, vec![], true, None, item);
            let (source, _) = frame_item(&ctx, &mut app, vec![], true, None, item);
            let start = source.center();
            frame_item(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(start), pointer(start, true)],
                true,
                None,
                item,
            );
            let target = app.canvas_rect.center();
            let (_, preview) = frame_item(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(target)],
                true,
                None,
                item,
            );
            assert_eq!(app.library_drag, Some(item), "{preset:?}");
            assert!(preview.is_positive());
            assert_eq!(app.editor.document, before);
            frame_item(
                &ctx,
                &mut app,
                vec![pointer(target, false)],
                true,
                None,
                item,
            );
            assert_eq!(app.editor.nodes().len(), 1);
            let origin = app.editor.nodes()[0].origin;
            let mut expected = Editor::new(before.clone());
            expected.add_preset(preset, origin.x, origin.y);
            assert_eq!(app.editor.document, expected.document);
            app.editor.undo();
            assert_eq!(app.editor.document, before);
        }
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
            vec![egui::Event::PointerMoved(start)],
            true,
            Some(2.1),
        );
        assert!(
            app.library_drag.is_none(),
            "an unchanged pointer stays pending"
        );
        assert!(!bounds.is_positive());
        let (_, bounds) = frame_at(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(start + Vec2::new(0.5, 0.))],
            true,
            Some(2.2),
        );
        assert_eq!(
            app.library_drag,
            Some(CatalogItem::Control(Kind::CircleButton))
        );
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
                Vec2::new(31. * 96. / 46., 48.)
            } else {
                Vec2::splat(48.)
            };
            let expected = app.world(target - anchor).round();
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
        for (zoom, expected) in [(0.5, (120., 130.)), (1., (140., 165.)), (2., (150., 182.))] {
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
            assert!(app.library_drag.is_some());
            let target = app.screen([160., 200.]);
            let (_, preview_bounds) = frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(target)],
                true,
            );
            assert_eq!(
                app.library_drag,
                Some(CatalogItem::Control(Kind::CircleButton))
            );
            assert_eq!(app.editor.document, before);
            assert_eq!(app.editor.selection, selection);
            assert!(preview_bounds.is_positive());
            assert!(preview_bounds.width() > 90. && preview_bounds.width() < 110.);
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
            assert_eq!((b.x, b.y), expected);
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
                assert_eq!((b.x, b.y), (183., 152.));
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
        let expected = app.world(target - Vec2::splat(48.)).round();
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
            assert_eq!((b.x, b.y), (160. - 48. / zoom, 200. - 48. / zoom));
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
            // The full-resolution 96-point thumbnail is fitted to 46 points,
            // with 11 points of left padding. Activation is at (42, 27).
            let anchor = Vec2::new(31. * 96. / 46., 27. * 96. / 46.);
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
            let expected = Vec2::new(160. - anchor.x / zoom, 200. - anchor.y / zoom).round();
            assert_eq!((b.x, b.y), (expected.x, expected.y));
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
    #[test]
    fn asset_drag_commits_one_shared_image_and_cancels_changed_or_removed_bytes() {
        for mode in ["insert", "replace", "remove", "corrupt", "trash"] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            let mut bytes = std::io::Cursor::new(vec![]);
            image::RgbaImage::from_pixel(80, 120, image::Rgba([220, 80, 30, 255]))
                .write_to(&mut bytes, image::ImageFormat::Png)
                .unwrap();
            let id = app.editor.document.allocate_id().unwrap();
            app.editor.document.assets.push(wireframe::document::Asset {
                trashed: false,
                id,
                name: "Shared image".into(),
                media_type: "image/png".into(),
                bytes: bytes.into_inner(),
            });
            let item = CatalogItem::Asset(id);
            let start = component_press(&ctx, &mut app, item);
            let (_, preview) = frame_item(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(start + Vec2::new(7., 0.))],
                true,
                None,
                item,
            );
            assert!(preview.is_positive());
            assert!(app.editor.nodes().is_empty());
            match mode {
                "replace" => {
                    let mut bytes = std::io::Cursor::new(vec![]);
                    image::RgbaImage::from_pixel(40, 50, image::Rgba([20, 180, 60, 255]))
                        .write_to(&mut bytes, image::ImageFormat::Png)
                        .unwrap();
                    app.editor.document.assets[0].bytes = bytes.into_inner();
                }
                "remove" => app.editor.document.assets.clear(),
                "trash" => app.editor.trash_assets(&[id]).unwrap(),
                "corrupt" => app.editor.document.assets[0].bytes = vec![0],
                _ => {}
            }
            let before = app.editor.document.clone();
            let target = app.canvas_rect.center();
            frame_item(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(target), pointer(target, false)],
                true,
                None,
                item,
            );
            if mode == "insert" {
                assert_eq!(app.editor.nodes().len(), 1);
                assert_eq!(
                    app.editor.nodes()[0].properties.get("image"),
                    Some(&wireframe::document::Value::Asset(id))
                );
                assert_eq!(app.editor.document.assets, before.assets);
                app.editor.undo();
            }
            assert_eq!(app.editor.document, before, "{mode}");
            assert!(app.library_drag.is_none());
        }
    }
    #[test]
    fn icon_tiles_enlarge_only_individual_icons_and_center_the_view_box() {
        let tile = Rect::from_min_size(Pos2::new(10., 20.), Vec2::splat(68.));
        let bounds = Bounds::new(0., 0., 24., 24.);
        let icon = TileImage::for_item(CatalogItem::Icon("star-regular"), tile, bounds);
        assert_eq!(icon.origin, tile.min + Vec2::new(11., 0.));
        assert_eq!(icon.scale, 46. / 24.);
        for item in [
            CatalogItem::Control(Kind::Icon),
            CatalogItem::Control(Kind::CircleButton),
        ] {
            let ordinary = TileImage::for_item(item, tile, bounds);
            assert_eq!(ordinary.scale, 1.);
            assert_eq!(ordinary.origin, tile.min + Vec2::new(22., 11.));
        }
    }

    #[test]
    fn individual_icon_drag_has_large_zoom_independent_preview_and_small_reversible_insertion() {
        let mut prior_size: Option<Vec2> = None;
        for zoom in [0.5, 1., 2.] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.zoom = zoom;
            app.pan = Vec2::new(13., -17.);
            let item = CatalogItem::Icon("star-regular");
            let before = app.editor.document.clone();
            let start = component_press(&ctx, &mut app, item);
            let (_, preview) = frame_item(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(start + Vec2::new(7., 0.))],
                true,
                None,
                item,
            );
            assert!(preview.is_positive());
            // Artwork may have internal padding and a one-pixel raster guard.
            assert!(
                preview.width() > 40. && preview.width() < 50.,
                "{preview:?}"
            );
            if let Some(size) = prior_size {
                assert!((preview.size() - size).length() < 0.01);
            }
            prior_size = Some(preview.size());
            assert_eq!(app.editor.document, before);
            let target = app.canvas_rect.center();
            let (_, moved) = frame_item(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(target)],
                true,
                None,
                item,
            );
            assert!(
                (moved.min - preview.min - (target - start - Vec2::new(7., 0.))).length() < 1.5
            );
            assert_eq!(app.editor.document, before);
            let expected = app
                .world((target - app.library_anchor).max(app.canvas_rect.min))
                .round();
            frame_item(
                &ctx,
                &mut app,
                vec![pointer(target, false)],
                true,
                None,
                item,
            );
            assert_eq!(app.editor.nodes().len(), 1);
            let node = &app.editor.current().controls[0];
            assert_eq!((node.bounds.x, node.bounds.y), (expected.x, expected.y));
            assert_eq!((node.bounds.w, node.bounds.h), (24., 24.));
            let node = &app.editor.nodes()[0];
            assert_eq!(
                node.properties.get("icon"),
                Some(&wireframe::document::Value::Icon(
                    wireframe::document::IconSource::Catalog("star-regular".into())
                ))
            );
            assert_eq!(app.editor.undo_name(), Some("Add icon"));
            let placed = app.editor.document.clone();
            frame_item(&ctx, &mut app, vec![], true, None, item);
            assert_eq!(app.editor.document, placed);
            app.editor.undo();
            assert_eq!(app.editor.document, before);
            app.editor.redo();
            assert_eq!(app.editor.document, placed);
        }
    }

    #[test]
    fn measured_native_library_drop_positions_cover_zoom_scroll_and_group_space() {
        // Inputs and integer results from eight accepted native drops. Anchor
        // values were read from the corresponding drag payloads.
        for (icon, zoom, scroll, nested, pointer_local, expected) in [
            (false, 1., [0., 0.], false, [506., 286.], [478., 256.]),
            (false, 1.5, [128., 35.], false, [706., 336.], [537., 227.]),
            (true, 1.5, [128., 35.], false, [456., 386.], [380., 260.]),
            (true, 1., [0., 0.], false, [556., 436.], [542., 404.]),
            (true, 1., [0., 0.], true, [806., 486.], [412., 227.]),
            (true, 1.5, [563., 314.], true, [706., 386.], [457., 219.]),
            (false, 1.5, [563., 314.], true, [606., 436.], [380., 253.]),
            (false, 1., [199., 81.], true, [756., 536.], [547., 360.]),
        ] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            if nested {
                app.editor.add(Kind::Button, 380., 227.);
                app.editor.add(Kind::Button, 537., 380.);
                app.editor.select_all();
                app.editor.group();
                let group = *app.editor.selection.iter().next().unwrap();
                assert!(app.editor.enter(group));
            }
            app.zoom = zoom;
            app.pan = -Vec2::from(scroll);
            let item = if icon {
                CatalogItem::Icon("0-solid")
            } else {
                CatalogItem::Control(Kind::CircleButton)
            };
            component_press(&ctx, &mut app, item);
            let before = app.editor.document.clone();
            app.library_drag = Some(item);
            app.library_anchor = if icon {
                Vec2::new(13.890201, 31.7)
            } else {
                Vec2::new(28.382559, 29.843485)
            };
            let target = app.canvas_rect.min + Vec2::from(pointer_local);
            frame_item(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(target), pointer(target, false)],
                true,
                None,
                item,
            );
            let origin = app.editor.nodes().last().unwrap().origin;
            assert_eq!([origin.x, origin.y], expected, "{icon}, {zoom}, {nested}");
            let placed = app.editor.document.clone();
            assert_eq!(
                wireframe::document::decode(&placed.encode().unwrap()).unwrap(),
                placed
            );
            app.editor.undo();
            assert_eq!(app.editor.document, before);
            app.editor.redo();
            assert_eq!(app.editor.document, placed);
        }
    }

    #[test]
    fn measured_native_normal_activation_uses_the_full_resolution_thumbnail() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        component_press(&ctx, &mut app, CatalogItem::Control(Kind::CircleButton));
        let mut press = app.library_press.clone().unwrap();
        press.point = press.provider_origin + Vec2::new(24., 14.);
        let anchor = press.activation(press.point + Vec2::new(0.6, 0.3)).unwrap();
        assert!((anchor.x - 28.382559).abs() < 0.002);
        assert!((anchor.y - 29.843485).abs() < 0.002);
    }

    #[test]
    fn measured_native_icon_activation_preserves_its_inverted_vertical_offset() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        component_press(&ctx, &mut app, CatalogItem::Icon("0-solid"));
        let mut press = app.library_press.clone().unwrap();
        press.point = press.provider_origin + Vec2::new(20., 14.);
        // Measured first dragged event and its serialized 46-point preview offset.
        let activation = press.point + Vec2::new(0.5333333, 0.3);
        let anchor = press
            .activation(activation)
            .expect("first motion activates");
        assert!((anchor.x - 13.890201).abs() < 0.002);
        assert!((anchor.y - 31.7).abs() < 0.002);
    }

    #[test]
    fn icon_anchor_uses_full_provider_and_inverts_vertical_offset() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        component_press(&ctx, &mut app, CatalogItem::Icon("star-regular"));
        let press = app.library_press.as_ref().unwrap();
        for local in [Vec2::new(0., 0.), Vec2::new(12., 9.), Vec2::new(68., 46.)] {
            assert_eq!(
                press.anchor(press.provider_origin + local),
                Vec2::new(local.x * 46. / 68., 46. - local.y)
            );
        }
        assert_eq!(icon_preview_scale(press.preview.bounds), 46. / 24.);
        assert_eq!((press.preview.bounds.w, press.preview.bounds.h), (24., 24.));
    }

    #[test]
    fn off_center_library_drops_keep_viewport_and_nested_group_coordinates() {
        // Expected anchors are in preview points, independently of canvas zoom:
        // a 96-point circle thumbnail fitted into 46 points, versus a 46-point icon
        // normalized across the complete 68-by-46 provider.
        for (item, anchor) in [
            (
                CatalogItem::Control(Kind::CircleButton),
                Vec2::new(13. * 96. / 46., 16. * 96. / 46.),
            ),
            (
                CatalogItem::Icon("star-regular"),
                Vec2::new(24. * 46. / 68., 30.),
            ),
        ] {
            for (zoom, edge_expected) in [
                (0.5, Vec2::new(-26., 34.)),
                (1., Vec2::new(-13., 17.)),
                (2., Vec2::new(-6., 9.)),
            ] {
                for nested in [false, true] {
                    for edge in [false, true] {
                        let ctx = egui::Context::default();
                        let mut app = WireframeApp::workspace(&ctx, None);
                        if nested {
                            app.editor.add(Kind::Button, 100., 120.);
                            app.editor.add(Kind::Button, 220., 180.);
                            app.editor.select_all();
                            app.editor.group();
                            let inner = *app.editor.selection.iter().next().unwrap();
                            app.editor.add(Kind::Button, 40., 60.);
                            app.editor.select_all();
                            app.editor.group();
                            let outer = *app.editor.selection.iter().next().unwrap();
                            assert!(app.editor.enter(outer));
                            assert!(app.editor.enter(inner));
                        }
                        app.zoom = zoom;
                        app.pan = Vec2::new(13., -17.);
                        let before = app.editor.document.clone();
                        let count = app.editor.nodes().len();
                        let (tile, _) = frame_item(&ctx, &mut app, vec![], true, None, item);
                        let press = tile.min + Vec2::new(17., 11.);
                        frame_item(
                            &ctx,
                            &mut app,
                            vec![egui::Event::PointerMoved(press), pointer(press, true)],
                            true,
                            None,
                            item,
                        );
                        frame_item(
                            &ctx,
                            &mut app,
                            vec![egui::Event::PointerMoved(tile.min + Vec2::new(24., 16.))],
                            true,
                            None,
                            item,
                        );
                        assert_eq!(app.library_drag, Some(item));
                        assert_eq!(app.editor.document, before);
                        let target = if edge {
                            app.canvas_rect.min + Vec2::splat(3.)
                        } else {
                            app.canvas_rect.min
                                + Vec2::new(13., -17.)
                                + Vec2::new(220., 210.) * zoom
                        };
                        frame_item(
                            &ctx,
                            &mut app,
                            vec![egui::Event::PointerMoved(target), pointer(target, false)],
                            true,
                            None,
                            item,
                        );
                        let root = if edge {
                            edge_expected
                        } else {
                            Vec2::new(220., 210.) - anchor / zoom
                        };
                        let expected = root.round()
                            - if nested {
                                Vec2::new(100., 120.)
                            } else {
                                Vec2::ZERO
                            };
                        assert_eq!(app.editor.nodes().len(), count + 1);
                        let inserted = app.editor.nodes().last().unwrap().origin;
                        assert!(
                            (Vec2::new(inserted.x, inserted.y) - expected).length() < 0.001,
                            "{item:?}, zoom {zoom}, nested {nested}, edge {edge}: {inserted:?}, expected {expected:?}"
                        );
                        let placed = app.editor.document.clone();
                        let reopened =
                            wireframe::document::decode(&placed.encode().unwrap()).unwrap();
                        assert_eq!(reopened, placed);
                        app.editor.undo();
                        assert_eq!(app.editor.document, before);
                        app.editor.redo();
                        assert_eq!(app.editor.document, placed);
                    }
                }
            }
        }
    }
    fn component_app(ctx: &egui::Context) -> (WireframeApp, CatalogItem) {
        let mut app = WireframeApp::workspace(ctx, None);
        app.editor.add(Kind::CircleButton, 0., 0.);
        let id = app
            .editor
            .create_component("Circular action".into())
            .unwrap();
        app.editor.select_all();
        app.editor.delete();
        (app, CatalogItem::Component(id))
    }
    fn component_press(ctx: &egui::Context, app: &mut WireframeApp, item: CatalogItem) -> Pos2 {
        frame_item(ctx, app, vec![], true, None, item);
        let (tile, _) = frame_item(ctx, app, vec![], true, None, item);
        let start = tile.min + Vec2::new(34., 23.);
        frame_item(
            ctx,
            app,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
            true,
            None,
            item,
        );
        start
    }
    #[test]
    fn component_held_preview_commits_at_release_and_one_undo_restores_document() {
        for zoom in [0.5, 1., 2.] {
            let ctx = egui::Context::default();
            let (mut app, item) = component_app(&ctx);
            app.zoom = zoom;
            let before = app.editor.document.clone();
            let start = component_press(&ctx, &mut app, item);
            let (_, preview) = frame_item(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(start + Vec2::new(7., 0.))],
                true,
                None,
                item,
            );
            assert!(preview.is_positive());
            assert_eq!(app.library_drag, Some(item));
            assert_eq!(app.editor.document, before);
            let target = app.canvas_rect.center();
            let anchor = app.library_anchor;
            let expected = app
                .world((target - anchor).max(app.canvas_rect.min))
                .round();
            frame_item(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(target), pointer(target, false)],
                true,
                None,
                item,
            );
            assert_eq!(app.editor.nodes().len(), 1);
            let node = &app.editor.nodes()[0];
            assert_eq!((node.origin.x, node.origin.y), (expected.x, expected.y));
            let CatalogItem::Component(id) = item else {
                panic!()
            };
            assert_eq!(app.editor.instance_definition(node.id), Some(id));
            let after = app.editor.document.clone();
            app.editor.undo();
            assert_eq!(app.editor.document, before);
            app.editor.redo();
            assert_eq!(app.editor.document, after);
        }
    }
    #[test]
    fn stale_component_gestures_cancel_on_surface_session_content_removal_and_modal_changes() {
        for mode in [
            "surface",
            "session",
            "content",
            "remove",
            "modal",
            "save-boundary",
            "focus",
        ] {
            let ctx = egui::Context::default();
            let (mut app, item) = component_app(&ctx);
            let start = component_press(&ctx, &mut app, item);
            frame_item(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(start + Vec2::new(7., 0.))],
                true,
                None,
                item,
            );
            let CatalogItem::Component(id) = item else {
                panic!()
            };
            match mode {
                "surface" => app.editor.edit_library_source(id).unwrap(),
                "session" => app.editor = Editor::new(app.editor.document.clone()),
                "content" => {
                    app.editor.document.definitions[0].content.nodes[0]
                        .properties
                        .insert(
                            "text".into(),
                            wireframe::document::Value::Text("Changed".into()),
                        );
                }
                "remove" => app.editor.delete_definitions(&[id]).unwrap(),
                "modal" => app.pdf_options = Some(Default::default()),
                "save-boundary" => app.end_pointer_gestures(),
                "focus" => {
                    frame_item(&ctx, &mut app, vec![], false, None, item);
                }
                _ => unreachable!(),
            }
            let before = app.editor.document.clone();
            let undo = app.editor.undo_name().map(str::to_owned);
            let target = app.canvas_rect.center();
            frame_item(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(target), pointer(target, false)],
                true,
                None,
                item,
            );
            assert!(app.library_drag.is_none(), "{mode}");
            assert!(app.library_press.is_none(), "{mode}");
            assert_eq!(app.editor.document, before, "{mode}");
            assert_eq!(app.editor.undo_name(), undo.as_deref(), "{mode}");
        }
    }
    #[test]
    fn coalesced_component_press_and_release_inserts_once_without_a_second_event_loop() {
        let ctx = egui::Context::default();
        let (mut app, item) = component_app(&ctx);
        frame_item(&ctx, &mut app, vec![], true, None, item);
        let (tile, _) = frame_item(&ctx, &mut app, vec![], true, None, item);
        let start = tile.min + Vec2::new(34., 23.);
        let target = app.canvas_rect.center();
        let before = app.editor.document.clone();
        frame_item(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(start),
                pointer(start, true),
                egui::Event::PointerMoved(target),
                pointer(target, false),
            ],
            true,
            None,
            item,
        );
        assert_eq!(app.editor.nodes().len(), 1);
        app.editor.undo();
        assert_eq!(app.editor.document, before);
    }
}
