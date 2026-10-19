// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
pub(super) mod inspector;
pub(super) struct ArrowHandle {
    pub id: Id,
    pub kind: wireframe::arrows::Handle,
    pub position: Pos2,
    pub origin: Pos2,
    pub offset: wireframe::document::Point,
}

impl WireframeApp {
    pub(super) fn paint_arrow_selection(&self, painter: &egui::Painter) -> bool {
        let selected = self.editor.selected_views();
        let [c] = selected.as_slice() else {
            return false;
        };
        if c.kind != Kind::Arrow || c.group.is_some() {
            return false;
        }
        let Some((_, properties)) = self.editor.control_properties(c.id) else {
            return false;
        };
        let arrow = wireframe::arrows::Arrow::from_properties(&properties, c.bounds.w, c.bounds.h);
        let offset = arrow.display_offset(&properties);
        let points: Vec<_> = arrow
            .path()
            .into_iter()
            .map(|p| self.screen([c.bounds.x + p.x - offset.x, c.bounds.y + p.y - offset.y]))
            .collect();
        let start = painter.ctx().graphics(|g| {
            g.get(painter.layer_id())
                .map_or(0, |list| list.next_idx().0)
        });
        let color = Color32::from_gray(if c.locked { 112 } else { 0 });
        let radius = 10. * self.zoom;
        // Compose the opaque stroke and round joins before fading the whole
        // highlight, so overlapping segments do not darken bends or endpoints.
        for pair in points.windows(2) {
            painter.line_segment([pair[0], pair[1]], Stroke::new(2. * radius, color));
        }
        for point in points {
            painter.circle_filled(point, radius, color);
        }
        wireframe::render::compositor::Compositor::finish(painter, start, 0.2);
        wireframe::render::compositor::Compositor::resolve(painter, start);
        true
    }
    pub(super) fn arrow_handles(&self) -> Vec<ArrowHandle> {
        if self.present {
            return vec![];
        }
        let selected = self.editor.selected_views();
        let [c] = selected.as_slice() else {
            return vec![];
        };
        if c.kind != Kind::Arrow || c.locked || c.group.is_some() {
            return vec![];
        }
        let Some((_, properties)) = self.editor.control_properties(c.id) else {
            return vec![];
        };
        let arrow = wireframe::arrows::Arrow::from_properties(&properties, c.bounds.w, c.bounds.h);
        let offset = arrow.display_offset(&properties);
        use wireframe::arrows::Handle;
        [
            (Handle::Start, arrow.start),
            (Handle::Middle, arrow.middle()),
            (Handle::End, arrow.end),
        ]
        .into_iter()
        .map(|(kind, p)| ArrowHandle {
            id: c.id,
            kind,
            position: self.screen([c.bounds.x + p.x - offset.x, c.bounds.y + p.y - offset.y]),
            origin: Pos2::new(c.bounds.x, c.bounds.y),
            offset,
        })
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wireframe::arrows::{Handle, node_geometry};
    fn frame(
        ctx: &egui::Context,
        app: &mut WireframeApp,
        events: Vec<egui::Event>,
        modifiers: Modifiers,
    ) {
        let mut events = events;
        events.insert(0, egui::Event::ModifiersChanged(modifiers));
        let mut output = ctx.run_ui(
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
        output.textures_delta.clear();
    }
    fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers,
        }
    }
    #[test]
    fn arrow_handles_follow_held_and_coalesced_drags_with_single_undo() {
        for zoom in [0.75, 1., 2.] {
            for coalesced in [false, true] {
                let ctx = egui::Context::default();
                let mut app = WireframeApp::workspace(&ctx, None);
                app.zoom = zoom;
                app.snap = false;
                app.editor.add(Kind::Arrow, 200., 180.);
                frame(&ctx, &mut app, vec![], Modifiers::NONE);
                let before = app.editor.document.clone();
                let handle = app
                    .arrow_handles()
                    .into_iter()
                    .find(|h| h.kind == Handle::Middle)
                    .unwrap();
                assert!(app.resize_handles(app.editor.bounds().unwrap()).is_empty());
                let end = handle.position + Vec2::new(40., -100.) * zoom;
                if coalesced {
                    frame(
                        &ctx,
                        &mut app,
                        vec![
                            egui::Event::PointerMoved(handle.position),
                            button(handle.position, true, Modifiers::NONE),
                            egui::Event::PointerMoved(end),
                            button(end, false, Modifiers::NONE),
                        ],
                        Modifiers::NONE,
                    );
                } else {
                    frame(
                        &ctx,
                        &mut app,
                        vec![
                            egui::Event::PointerMoved(handle.position),
                            button(handle.position, true, Modifiers::NONE),
                        ],
                        Modifiers::NONE,
                    );
                    frame(
                        &ctx,
                        &mut app,
                        vec![egui::Event::PointerMoved(end)],
                        Modifiers::NONE,
                    );
                    assert!(matches!(app.drag, Some(Drag::Arrow { .. })));
                    assert_ne!(
                        app.editor.document, before,
                        "held preview must update before release"
                    );
                    let current = app
                        .arrow_handles()
                        .into_iter()
                        .find(|h| h.kind == Handle::Middle)
                        .unwrap();
                    assert!(current.position.distance(end) <= 1.5 * zoom);
                    frame(
                        &ctx,
                        &mut app,
                        vec![button(end, false, Modifiers::NONE)],
                        Modifiers::NONE,
                    );
                }
                assert!(app.error.is_none(), "{:?}", app.error);
                assert!(app.drag.is_none());
                let after = app.editor.document.clone();
                assert_ne!(after, before);
                app.editor.undo();
                assert_eq!(app.editor.document, before);
                app.editor.redo();
                assert_eq!(app.editor.document, after);
            }
        }
    }
    #[test]
    fn shifted_endpoint_drag_and_escape_keep_geometry_and_history_consistent() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.snap = false;
        app.editor.add(Kind::Arrow, 200., 180.);
        frame(&ctx, &mut app, vec![], Modifiers::NONE);
        let before = app.editor.document.clone();
        let h = app
            .arrow_handles()
            .into_iter()
            .find(|h| h.kind == Handle::End)
            .unwrap();
        let end = h.position + Vec2::new(60., 20.);
        frame(
            &ctx,
            &mut app,
            vec![button(h.position, true, Modifiers::SHIFT)],
            Modifiers::SHIFT,
        );
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(end)],
            Modifiers::SHIFT,
        );
        let a = node_geometry(&app.editor.nodes()[0]).unwrap().0;
        assert_eq!((a.end.x - a.start.x).abs(), (a.end.y - a.start.y).abs());
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
            Modifiers::NONE,
        );
        assert_eq!(app.editor.document, before);
        assert!(app.drag.is_none());
    }
    #[test]
    fn arrow_inspector_exposes_working_fields_and_hides_geometry_storage() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Arrow, 0., 0.);
        let id = *app.editor.selection.iter().next().unwrap();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(500., 900.))),
                ..Default::default()
            },
            |ui| assert!(app.typed_properties(ui, id)),
        );
        output.textures_delta.clear();
        let spec = wireframe::catalog::get("arrow").unwrap();
        for key in [
            "shape",
            "stroke",
            "border_color",
            "background_opacity",
            "start_head",
            "end_head",
            "label_position",
            "text",
        ] {
            assert!(spec.property(key).unwrap().visible);
        }
        for key in ["start_x", "end_y", "bend_along", "legacy_horizontal"] {
            assert!(!spec.property(key).unwrap().visible);
        }
        app.editor.lock(true);
        assert!(app.arrow_handles().is_empty());
    }
}
