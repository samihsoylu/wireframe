// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::{
    document::Point,
    quick_draw::{Gesture, Mode},
};

const KEYS: [(Key, Mode); 5] = [
    (Key::I, Mode::Image),
    (Key::R, Mode::Rectangle),
    (Key::T, Mode::TextBlock),
    (Key::Y, Mode::TextLine),
    (Key::A, Mode::Arrow),
];

#[derive(Default)]
pub(super) struct State {
    held: [bool; 5],
    space: bool,
    active: Option<Active>,
    cancelled: bool,
}
struct Active {
    surface: Id,
    context: Vec<Id>,
    session: String,
    gesture: Gesture,
    previous: HashSet<Id>,
    targets: Targets,
    motion: Motion,
    bounds: Bounds,
}
impl State {
    pub(super) fn is_active(&self) -> bool {
        self.active.is_some()
    }
    fn mode(&self) -> Option<Mode> {
        KEYS.iter()
            .zip(self.held)
            .find_map(|((_, mode), held)| held.then_some(*mode))
    }
}
impl WireframeApp {
    fn quick_draw_targets(&mut self) -> Targets {
        let selected = std::mem::take(&mut self.editor.selection);
        let targets = self
            .editor
            .alignment_targets(5.)
            .unwrap_or_else(|_| Targets::new(5.));
        self.editor.selection = selected;
        targets
    }
    fn begin_quick_draw(&mut self, start: Point, previous: HashSet<Id>) {
        let targets = self.quick_draw_targets();
        let snapped = targets.point(start.x, start.y);
        let gesture = Gesture::new(Point::new(snapped.bounds.x, snapped.bounds.y));
        let bounds = gesture.bounds();
        self.quick_draw.active = Some(Active {
            surface: self.editor.surface_id(),
            context: self.editor.context.clone(),
            session: self.editor.clipboard_session().into(),
            gesture,
            previous,
            targets,
            motion: Motion::default(),
            bounds,
        });
        self.canvas_press = None;
    }
    pub(super) fn cancel_quick_draw(&mut self) -> bool {
        let active = self.quick_draw.active.take();
        let cancelled = active.is_some();
        if let Some(active) = active.filter(|a| {
            a.surface == self.editor.surface_id()
                && a.context == self.editor.context
                && a.session == self.editor.clipboard_session()
        }) {
            self.editor.selection = active.previous;
        }
        self.quick_draw = State {
            cancelled,
            ..Default::default()
        };
        if cancelled {
            self.alignment_guides.clear();
            self.canvas_press = None;
        }
        cancelled
    }
    /// Process the ordered stream, not the frame's final held-key state.
    pub(super) fn quick_draw_input(&mut self, ui: &egui::Ui, response: &egui::Response) -> bool {
        let ctx = ui.ctx();
        if self.quick_draw.active.as_ref().is_some_and(|a| {
            a.surface != self.editor.surface_id()
                || a.context != self.editor.context
                || a.session != self.editor.clipboard_session()
        }) {
            self.cancel_quick_draw();
        }
        let cancelled = std::mem::take(&mut self.quick_draw.cancelled);
        if self.canvas_input_blocked()
            || self.present
            || !ui.input(|i| i.focused)
            || wireframe::text_entry::is_focused(ctx)
            || ctx.text_edit_focused()
            || ctx.any_popup_open()
            || egui::Popup::is_any_open(ctx)
            || self.quick_focus
            || self.library_drag.is_some()
            || self.library_press.is_some()
        {
            return self.cancel_quick_draw() || cancelled;
        }
        if cancelled {
            return true;
        }
        let mut used = self.quick_draw.active.is_some();
        for event in ui.input(|i| i.events.clone()) {
            match event {
                egui::Event::WindowFocused(false) => {
                    used |= self.cancel_quick_draw();
                }
                egui::Event::Key {
                    key: Key::Escape,
                    pressed: true,
                    ..
                } => {
                    used |= self.cancel_quick_draw();
                }
                egui::Event::Key {
                    key,
                    pressed,
                    modifiers,
                    ..
                } => {
                    if key == Key::Space {
                        self.quick_draw.space = pressed;
                    }
                    if let Some(i) = KEYS.iter().position(|(k, _)| *k == key) {
                        self.quick_draw.held[i] =
                            pressed && !modifiers.command && !modifiers.ctrl && !modifiers.alt;
                    }
                    if self.quick_draw.mode().is_some()
                        && self.editor.can_quick_draw()
                        && matches!(self.drag, Some(Drag::Marquee { .. }))
                        && let Some(Drag::Marquee {
                            start,
                            now,
                            previous,
                        }) = self.drag.take()
                    {
                        self.begin_quick_draw(Point::new(start.x, start.y), previous);
                        self.quick_draw.active.as_mut().unwrap().gesture.end =
                            Point::new(now.x, now.y);
                        used = true;
                    }
                }
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers,
                } => {
                    if self.drag.is_none()
                        && self.quick_draw.mode().is_some()
                        && !self.quick_draw.space
                        && !ui.input(|i| i.pointer.button_down(egui::PointerButton::Middle))
                        && !modifiers.command
                        && !modifiers.ctrl
                        && response.rect.contains(pos)
                        && ctx.layer_id_at(pos) == Some(ui.layer_id())
                        && self.editor.can_quick_draw()
                    {
                        let start = self.world(pos);
                        let previous = self.editor.selection.clone();
                        self.begin_quick_draw(Point::new(start.x, start.y), previous);
                        used = true;
                    }
                }
                egui::Event::PointerMoved(pos) => {
                    let point = self.world(pos);
                    if let Some(active) = self.quick_draw.active.as_mut() {
                        active.gesture.motion(Point::new(point.x, point.y));
                        active.motion.push([point.x, point.y]);
                    } else if self.quick_draw.mode().is_some()
                        && response.rect.contains(pos)
                        && self.editor.can_quick_draw()
                    {
                        self.alignment_guides =
                            self.quick_draw_targets().point(point.x, point.y).guides;
                    }
                }
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers,
                } => {
                    let point = self.world(pos);
                    if let Some(active) = self.quick_draw.active.as_mut() {
                        active.gesture.end = Point::new(point.x, point.y);
                        active.motion.push([point.x, point.y]);
                        self.update_quick_draw_preview(modifiers);
                        let mode = self.quick_draw.mode();
                        let active = self.quick_draw.active.take().unwrap();
                        if let Some(mode) = mode {
                            let node = active.gesture.node(mode, active.bounds);
                            if let Err(error) = self.editor.insert_quick_draw(node) {
                                self.error = Some(error);
                            }
                        }
                        self.alignment_guides.clear();
                        used = true;
                    }
                }
                _ => {}
            }
            self.update_quick_draw_preview(ui.input(|i| i.modifiers));
        }
        if self.quick_draw.active.is_some()
            && self.scroll_during_drag(ctx)
            && let Some(pos) = ui.input(|i| i.pointer.latest_pos())
        {
            let point = self.world(pos);
            self.quick_draw.active.as_mut().unwrap().gesture.end = Point::new(point.x, point.y);
            self.update_quick_draw_preview(ui.input(|i| i.modifiers));
        }
        if self.quick_draw.mode().is_none()
            && self.quick_draw.active.is_none()
            && self.drag.is_none()
        {
            self.alignment_guides.clear();
        }
        if self.quick_draw.mode().is_some() && response.hovered() && self.editor.can_quick_draw() {
            ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        used
    }
    fn update_quick_draw_preview(&mut self, modifiers: Modifiers) {
        let mode = self.quick_draw.mode();
        let Some(active) = self.quick_draw.active.as_mut() else {
            return;
        };
        let mut bounds = active.gesture.bounds();
        self.alignment_guides.clear();
        if mode.is_some() && !active.motion.too_fast() {
            let handle = (
                if active.gesture.end.x >= active.gesture.start.x {
                    1
                } else {
                    -1
                },
                if active.gesture.end.y >= active.gesture.start.y {
                    1
                } else {
                    -1
                },
            );
            let result = active.targets.resized(bounds, handle, None);
            bounds = result.bounds;
            bounds.w = bounds.w.max(10.);
            bounds.h = bounds.h.max(10.);
            self.alignment_guides = result.guides;
        }
        active.bounds = bounds;
        if mode.is_none() {
            let previous = if modifiers.shift || modifiers.command {
                active.previous.clone()
            } else {
                HashSet::new()
            };
            self.editor.select_area(bounds, &previous);
        } else {
            self.editor.selection = active.previous.clone();
        }
    }
    pub(super) fn paint_quick_draw(&self, ui: &egui::Ui, painter: &egui::Painter) {
        let Some(active) = self.quick_draw.active.as_ref() else {
            return;
        };
        if let Some(mode) = self.quick_draw.mode() {
            let fonts = wireframe::layout::EguiMeasurer {
                context: ui.ctx(),
                regular: wireframe::fonts::family(false, false),
                bold: wireframe::fonts::family(true, false),
            };
            let node = active.gesture.node(mode, active.bounds);
            if let Ok(scene) = self.editor.quick_draw_scene(node, &fonts) {
                Self::paint_scene(painter, &scene, self.canvas_rect.min + self.pan, self.zoom);
            }
        } else {
            painter.rect_stroke(
                self.screen_rect(active.bounds),
                0,
                Stroke::new(1., ACCENT),
                StrokeKind::Inside,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(key: Key, pressed: bool) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }
    fn pointer(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }
    fn frame(ctx: &egui::Context, app: &mut WireframeApp, events: Vec<egui::Event>) {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 800.))),
                events,
                ..Default::default()
            },
            |ui| {
                app.shortcuts(ui.ctx());
                app.canvas(ui);
            },
        );
        out.textures_delta.clear();
    }
    #[test]
    fn all_five_tools_preview_without_mutation_and_commit_one_undo_at_each_zoom() {
        for (key, mode) in KEYS {
            for zoom in [0.75, 1., 2.] {
                let ctx = egui::Context::default();
                let mut app = WireframeApp::workspace(&ctx, None);
                app.zoom = zoom;
                frame(&ctx, &mut app, vec![]);
                let before = app.editor.document.clone();
                let start = app.screen([100., 150.]);
                let end = app.screen([300., 260.]);
                frame(
                    &ctx,
                    &mut app,
                    vec![
                        key_event(key, true),
                        egui::Event::PointerMoved(start),
                        pointer(start, true),
                    ],
                );
                frame(&ctx, &mut app, vec![egui::Event::PointerMoved(end)]);
                assert!(app.quick_draw.active.is_some(), "{mode:?}");
                assert_eq!(
                    app.editor.document, before,
                    "preview allocated document data"
                );
                let active = app.quick_draw.active.as_ref().unwrap();
                let node = active.gesture.node(mode, active.bounds);
                let mut scene_ok = false;
                let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                    let fonts = wireframe::layout::EguiMeasurer {
                        context: ui.ctx(),
                        regular: wireframe::fonts::family(false, false),
                        bold: wireframe::fonts::family(true, false),
                    };
                    scene_ok = app.editor.quick_draw_scene(node.clone(), &fonts).is_ok();
                });
                out.textures_delta.clear();
                assert!(scene_ok, "{mode:?} preview failed");
                frame(
                    &ctx,
                    &mut app,
                    vec![pointer(end, false), key_event(key, false)],
                );
                assert!(app.error.is_none(), "{:?}", app.error);
                assert_eq!(app.editor.nodes().len(), 1);
                assert!(
                    matches!(&app.editor.nodes()[0].content,wireframe::document::NodeContent::Control {kind,..} if kind==mode.kind())
                );
                let after = app.editor.document.clone();
                assert_eq!(
                    wireframe::document::decode(&after.encode().unwrap()).unwrap(),
                    after
                );
                app.editor.undo();
                assert_eq!(app.editor.document, before);
                app.editor.redo();
                assert_eq!(app.editor.document, after);
            }
        }
    }
    fn key_event(k: Key, pressed: bool) -> egui::Event {
        key(k, pressed)
    }
    #[test]
    fn curve_and_release_key_order_survive_one_coalesced_frame() {
        for key_first in [false, true] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            frame(&ctx, &mut app, vec![]);
            let start = app.screen([100., 200.]);
            let end = app.screen([400., 200.]);
            let mut events = vec![
                key(Key::A, true),
                egui::Event::PointerMoved(start),
                pointer(start, true),
            ];
            for i in 1..20 {
                let t = i as f32 / 20.;
                events.push(egui::Event::PointerMoved(app.screen([
                    100. + 300. * t,
                    200. - 80. * (std::f32::consts::PI * t).sin(),
                ])));
            }
            if key_first {
                events.push(key(Key::A, false));
                events.push(pointer(end, false));
            } else {
                events.push(pointer(end, false));
                events.push(key(Key::A, false));
            }
            frame(&ctx, &mut app, events);
            assert!(app.quick_draw.active.is_none());
            if key_first {
                assert!(app.editor.nodes().is_empty());
            } else {
                assert_eq!(app.editor.nodes().len(), 1);
                let (arrow, _) = wireframe::arrows::node_geometry(&app.editor.nodes()[0]).unwrap();
                assert!(arrow.middle_relative.y.abs() > 0.1);
                let (_, p) = app
                    .editor
                    .control_properties(app.editor.nodes()[0].id)
                    .unwrap();
                assert_eq!(p["start_head"], wireframe::document::Value::Bool(false));
                assert_eq!(p["end_head"], wireframe::document::Value::Bool(true));
            }
        }
    }
    #[test]
    fn escape_and_focus_loss_cancel_without_changing_document_or_selection() {
        for cancel in [key(Key::Escape, true), egui::Event::WindowFocused(false)] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(Kind::Button, 100., 100.);
            frame(&ctx, &mut app, vec![]);
            let before = app.editor.document.clone();
            let selection = app.editor.selection.clone();
            let start = app.screen([100., 100.]);
            let end = app.screen([400., 300.]);
            frame(
                &ctx,
                &mut app,
                vec![key(Key::A, true), pointer(start, true)],
            );
            frame(&ctx, &mut app, vec![egui::Event::PointerMoved(end)]);
            assert!(app.quick_draw.active.is_some());
            frame(&ctx, &mut app, vec![cancel]);
            assert!(app.quick_draw.active.is_none());
            assert_eq!(app.editor.document, before);
            assert_eq!(app.editor.selection, selection);
        }
    }
    #[test]
    fn origin_snaps_to_selected_sibling_and_held_modes_can_change() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Button, 200., 200.);
        frame(&ctx, &mut app, vec![]);
        let start = app.screen([198., 198.]);
        let end = app.screen([450., 350.]);
        frame(
            &ctx,
            &mut app,
            vec![
                key(Key::A, true),
                key(Key::I, true),
                egui::Event::PointerMoved(start),
            ],
        );
        assert!(!app.alignment_guides.is_empty());
        frame(
            &ctx,
            &mut app,
            vec![pointer(start, true), egui::Event::PointerMoved(end)],
        );
        assert_eq!(app.quick_draw.mode(), Some(Mode::Image));
        assert_eq!(
            app.quick_draw.active.as_ref().unwrap().gesture.start,
            Point::new(200., 200.)
        );
        frame(&ctx, &mut app, vec![key(Key::I, false)]);
        assert_eq!(app.quick_draw.mode(), Some(Mode::Arrow));
        frame(
            &ctx,
            &mut app,
            vec![pointer(end, false), key(Key::A, false)],
        );
        assert_eq!(app.editor.nodes().len(), 2);
        assert!(wireframe::arrows::node_geometry(app.editor.nodes().last().unwrap()).is_some());
    }
    #[test]
    fn ordinary_marquee_can_become_a_drawing_before_release() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        frame(&ctx, &mut app, vec![]);
        let start = app.screen([100., 100.]);
        let end = app.screen([300., 200.]);
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
        );
        frame(&ctx, &mut app, vec![egui::Event::PointerMoved(end)]);
        assert!(matches!(app.drag, Some(Drag::Marquee { .. })));
        frame(&ctx, &mut app, vec![key(Key::R, true)]);
        assert!(app.drag.is_none());
        assert!(app.quick_draw.active.is_some());
        frame(
            &ctx,
            &mut app,
            vec![pointer(end, false), key(Key::R, false)],
        );
        assert_eq!(app.editor.nodes().len(), 1);
        assert_eq!(app.editor.selected_views()[0].kind, Kind::Rectangle);
    }
    #[test]
    fn text_focus_panning_and_changed_page_cannot_insert_a_stale_gesture() {
        for excluded in ["text", "space", "page"] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            frame(&ctx, &mut app, vec![]);
            let start = app.screen([100., 100.]);
            let end = app.screen([400., 300.]);
            if excluded == "text" {
                app.quick_focus = true;
            }
            let mut events = vec![key(Key::A, true), pointer(start, true)];
            if excluded == "space" {
                events.insert(0, key(Key::Space, true));
            }
            frame(&ctx, &mut app, events);
            if excluded == "page" {
                app.editor.add_page();
            }
            frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::PointerMoved(end),
                    pointer(end, false),
                    key(Key::A, false),
                ],
            );
            assert!(
                app.editor
                    .document
                    .pages
                    .iter()
                    .all(|p| p.content.nodes.is_empty()),
                "{excluded}"
            );
        }
    }
}
