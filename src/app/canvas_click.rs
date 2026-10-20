// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

#[derive(Default)]
pub(super) struct CanvasClicks {
    scope: Option<(String, Id, Vec<Id>)>,
    previous: Option<(f64, Pos2)>,
    current: Option<(f64, Pos2)>,
    held: bool,
}

impl CanvasClicks {
    fn event(&mut self, time: f64, pos: Pos2, pressed: bool) -> bool {
        // Native canvas coordinates are whole logical pixels, independent of zoom.
        let pos = Pos2::new(pos.x.trunc(), pos.y.trunc());
        if pressed {
            self.previous = self.current.replace((time, pos));
            self.held = true;
            false
        } else if std::mem::take(&mut self.held) {
            self.previous.is_some_and(|(start, point)| {
                time >= start
                    && time - start < 0.350
                    && (pos.x - point.x).abs() < 4.
                    && (pos.y - point.y).abs() < 4.
            })
        } else {
            false
        }
    }
}

impl WireframeApp {
    pub(super) fn canvas_double_click(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
    ) -> Option<(Pos2, Modifiers)> {
        if !ui.is_enabled()
            || self.canvas_input_blocked()
            || self.text_edit.is_some()
            || self.library_drag.is_some()
            || !ui.input(|i| i.focused)
        {
            self.canvas_clicks = CanvasClicks::default();
            return None;
        }
        let scope = (
            self.editor.clipboard_session().to_owned(),
            self.editor.surface_id(),
            self.editor.context.clone(),
        );
        if self.canvas_clicks.scope.as_ref() != Some(&scope) {
            self.canvas_clicks = CanvasClicks {
                scope: Some(scope),
                ..Default::default()
            };
        }
        let mut result = None;
        let (time, events) = ui.input(|i| (i.time, i.events.clone()));
        for event in &events {
            if let egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers,
            } = event
            {
                if *button != egui::PointerButton::Primary {
                    self.canvas_clicks.previous = None;
                    self.canvas_clicks.current = None;
                    self.canvas_clicks.held = false;
                    continue;
                }
                if !response.rect.contains(*pos)
                    || ui.ctx().layer_id_at(*pos) != Some(response.layer_id)
                {
                    self.canvas_clicks.previous = None;
                    self.canvas_clicks.current = None;
                    self.canvas_clicks.held = false;
                    continue;
                }
                if self
                    .canvas_clicks
                    .event(time, *pos - response.rect.min.to_vec2(), *pressed)
                {
                    result = Some((*pos, *modifiers));
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(
        ctx: &egui::Context,
        app: &mut WireframeApp,
        time: f64,
        event: Option<(Pos2, bool, Modifiers)>,
    ) {
        let events = event.map_or_else(Vec::new, |(pos, pressed, modifiers)| {
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    pressed,
                    modifiers,
                    button: egui::PointerButton::Primary,
                },
            ]
        });
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 800.))),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ui| app.canvas(ui),
        )
        .drop_without_applying_deltas();
    }

    #[test]
    fn canvas_dispatch_uses_its_own_timing_and_distance_policy() {
        for (times, offset, expected) in [
            ([0., 0.030, 0.100, 0.130], 0., true),
            ([0., 0.010, 0.300, 0.330], 0., true),
            ([0., 0.300, 0.350, 0.400], 0., false),
            ([0., 0.030, 0.100, 0.130], 4., false),
        ] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(Kind::Image, 150., 150.);
            let id = *app.editor.selection.iter().next().unwrap();
            frame(&ctx, &mut app, 0., None);
            let pos = app.screen([180., 180.]);
            for (index, time) in times.into_iter().enumerate() {
                let point = pos
                    + if index > 1 {
                        Vec2::new(offset, 0.)
                    } else {
                        Vec2::ZERO
                    };
                frame(
                    &ctx,
                    &mut app,
                    1. + time,
                    Some((point, index % 2 == 0, Modifiers::NONE)),
                );
            }
            assert_eq!(
                app.image_source_request,
                expected.then_some(id),
                "{times:?}, {offset}"
            );
        }
    }

    #[test]
    fn navigation_and_outside_clicks_break_the_canvas_sequence() {
        for navigate in [false, true] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            let first = app.editor.page;
            app.editor.add(Kind::Image, 150., 150.);
            app.editor.add_page();
            let second = app.editor.page;
            app.editor.add(Kind::Image, 150., 150.);
            app.editor.switch_page(first);
            frame(&ctx, &mut app, 0., None);
            let pos = app.screen([180., 180.]);
            frame(&ctx, &mut app, 1., Some((pos, true, Modifiers::NONE)));
            frame(&ctx, &mut app, 1.01, Some((pos, false, Modifiers::NONE)));
            if navigate {
                app.editor.switch_page(second);
            } else {
                frame(
                    &ctx,
                    &mut app,
                    1.02,
                    Some((Pos2::new(-10., -10.), true, Modifiers::NONE)),
                );
                frame(
                    &ctx,
                    &mut app,
                    1.03,
                    Some((Pos2::new(-10., -10.), false, Modifiers::NONE)),
                );
            }
            frame(&ctx, &mut app, 1.1, Some((pos, true, Modifiers::NONE)));
            frame(&ctx, &mut app, 1.11, Some((pos, false, Modifiers::NONE)));
            assert!(app.image_source_request.is_none());
        }
    }

    #[test]
    fn ordered_clicks_measure_from_first_press_to_second_release() {
        for (times, offset, expected) in [
            ([0., 0.030, 0.100, 0.130], 0., true),
            ([0., 0.010, 0.300, 0.330], 0., true),
            ([0., 0.300, 0.350, 0.400], 0., false),
            ([0., 0.030, 0.100, 0.130], 4., false),
            ([0., 0.010, 0.300, 0.350], 0., false),
        ] {
            let mut clicks = CanvasClicks::default();
            let origin = Pos2::new(100., 100.);
            assert!(!clicks.event(times[0], origin, true));
            assert!(!clicks.event(times[1], origin, false));
            assert!(!clicks.event(times[2], origin, true));
            assert_eq!(
                clicks.event(times[3], origin + Vec2::splat(offset), false),
                expected
            );
        }
    }

    #[test]
    fn whole_coordinate_axis_bounds_and_third_click_use_consecutive_presses() {
        let mut clicks = CanvasClicks::default();
        clicks.event(0., Pos2::new(100.9, 100.9), true);
        clicks.event(0.01, Pos2::new(100.9, 100.9), false);
        clicks.event(0.10, Pos2::new(104.1, 100.9), true);
        assert!(!clicks.event(0.11, Pos2::new(104.1, 100.9), false));
        clicks.event(0.15, Pos2::new(107.1, 103.9), true);
        assert!(clicks.event(0.16, Pos2::new(107.1, 103.9), false));
        assert!(!clicks.event(0.17, Pos2::new(107.1, 103.9), false));
    }
}
