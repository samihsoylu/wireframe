// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::review::{Callout, Color, Tip};
impl WireframeApp {
    fn review_markers(&self) -> Vec<Callout> {
        if !self.reviews_visible() {
            return vec![];
        }
        let Some(state) = self.review.surfaces.get(&self.editor.surface_id()) else {
            return vec![];
        };
        let draft = state.draft.as_ref().filter(|d| !d.stale).map(|d| &d.draft);
        state
            .observed
            .visible_callouts(draft, state.selected)
            .unwrap_or_default()
            .into_iter()
            .cloned()
            .collect()
    }
    pub(in crate::app) fn review_pointer(&mut self, ui: &egui::Ui) -> bool {
        let surface = self.editor.surface_id();
        let blocked = !self.reviews_visible()
            || self.canvas_input_blocked()
            || self.font_import.is_some()
            || self.present && self.presentation_view.settings_open
            || !ui.input(|i| i.focused);
        if blocked
            || self
                .review
                .drag
                .as_ref()
                .is_some_and(|d| d.surface != surface)
        {
            self.cancel_marker_drag();
            return false;
        }
        let mut captured = self.review.drag.is_some();
        let events = ui.input(|i| i.events.clone());
        for event in events {
            match event {
                egui::Event::Key {
                    key: Key::Escape,
                    pressed: true,
                    ..
                } => {
                    if self.review.drag.is_some() {
                        self.cancel_marker_drag();
                        captured = true;
                    }
                }
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    ..
                } if self.canvas_rect.contains(pos)
                    && ui.ctx().layer_id_at(pos) == Some(ui.layer_id()) =>
                {
                    let world = self.world(pos);
                    if let Some(marker) = self
                        .review_markers()
                        .into_iter()
                        .rev()
                        .find(|m| marker_hit(m, world))
                    {
                        self.review_surface().selected = Some(marker.id);
                        self.review.drag = Some(MarkerDrag {
                            surface,
                            marker: marker.id,
                            start: world,
                            original: marker.position,
                        });
                        captured = true;
                    }
                }
                egui::Event::PointerMoved(pos)
                | egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    ..
                } => {
                    if let Some(drag) = &self.review.drag {
                        let world = self.world(pos);
                        let position = wireframe::document::Point {
                            x: (drag.original.x + world.x - drag.start.x).clamp(-1e7, 1e7),
                            y: (drag.original.y + world.y - drag.start.y).clamp(-1e7, 1e7),
                        };
                        let id = drag.marker;
                        if let Some(composer) = &mut self.review_surface().draft
                            && !composer.stale
                            && let Some(marker) =
                                composer.draft.body.callouts.iter_mut().find(|m| m.id == id)
                        {
                            marker.position = position;
                        }
                        captured = true;
                        if matches!(event, egui::Event::PointerButton { pressed: false, .. }) {
                            self.review.drag = None;
                        }
                    }
                }
                egui::Event::PointerGone if self.review.drag.is_some() => {
                    self.cancel_marker_drag();
                    captured = true;
                }
                _ => {}
            }
        }
        captured
    }
    fn cancel_marker_drag(&mut self) {
        if let Some(drag) = self.review.drag.take()
            && let Some(state) = self.review.surfaces.get_mut(&drag.surface)
            && let Some(composer) = &mut state.draft
            && let Some(marker) = composer
                .draft
                .body
                .callouts
                .iter_mut()
                .find(|m| m.id == drag.marker)
        {
            marker.position = drag.original;
        }
    }
    pub(in crate::app) fn paint_review_markers(&self, painter: &egui::Painter) {
        let names = self
            .review
            .surfaces
            .get(&self.editor.surface_id())
            .map(Surface::marker_names)
            .unwrap_or_default();
        for marker in self.review_markers() {
            let center = self.screen([marker.position.x, marker.position.y]);
            let points = marker_points(&marker)
                .into_iter()
                .map(|p| center + p * self.zoom)
                .collect::<Vec<_>>();
            let fill = match marker.color {
                Color::Blue => Color32::from_rgb(43, 120, 228),
                Color::Green => Color32::from_rgb(0, 158, 15),
                Color::Yellow => Color32::from_rgb(241, 194, 50),
                Color::Red => Color32::from_rgb(207, 42, 39),
                Color::Gray => Color32::from_gray(221),
            };
            // Convex circle-plus-tip outline; native drawing resolution follows zoom/DPI.
            painter.add(egui::Shape::convex_polygon(
                points,
                fill,
                Stroke::new(1.5 * self.zoom, Color32::BLACK),
            ));
            let offset = match marker.tip {
                Tip::Left => Vec2::new(12.5, 0.),
                Tip::Right => Vec2::new(-12.5, 0.),
                Tip::Top => Vec2::new(0., 12.5),
                Tip::Bottom => Vec2::new(0., -12.5),
                Tip::None => Vec2::ZERO,
            };
            let ink = if matches!(marker.color, Color::Yellow | Color::Gray) {
                Color32::BLACK
            } else {
                Color32::WHITE
            };
            painter.text(
                center + offset * self.zoom,
                Align2::CENTER_CENTER,
                names
                    .get(&marker.id)
                    .cloned()
                    .unwrap_or_else(|| marker.label.to_string()),
                FontId::proportional(18. * self.zoom),
                ink,
            );
        }
    }
}
fn marker_points(marker: &Callout) -> Vec<Vec2> {
    if marker.tip == Tip::None {
        return (0..48)
            .map(|i| Vec2::angled(i as f32 * std::f32::consts::TAU / 48.) * 20.)
            .collect();
    }
    let mut points = vec![Vec2::new(-32.5, 0.)];
    let tangent = (-20.0_f32 / 45.).acos();
    points.extend((0..=40).map(|i| {
        Vec2::new(12.5, 0.) + Vec2::angled(-tangent + 2. * tangent * i as f32 / 40.) * 20.
    }));
    for point in &mut points {
        *point = match marker.tip {
            Tip::Top => Vec2::new(-point.y, point.x),
            Tip::Right => -*point,
            Tip::Bottom => Vec2::new(point.y, -point.x),
            _ => *point,
        };
    }
    points
}
fn marker_hit(marker: &Callout, world: Pos2) -> bool {
    let point = Vec2::new(world.x - marker.position.x, world.y - marker.position.y);
    let outline = marker_points(marker);
    let mut sign = 0.;
    for i in 0..outline.len() {
        let a = outline[i];
        let b = outline[(i + 1) % outline.len()];
        let cross = (b.x - a.x) * (point.y - a.y) - (b.y - a.y) * (point.x - a.x);
        if cross.abs() < 0.001 {
            continue;
        }
        if sign == 0. {
            sign = cross.signum();
        } else if sign != cross.signum() {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callout_outlines_are_convex_and_hit_test_matches_all_tip_directions() {
        for tip in [Tip::Left, Tip::Top, Tip::Right, Tip::Bottom, Tip::None] {
            let marker = Callout {
                id: 1,
                label: 1,
                position: wireframe::document::Point { x: 100., y: 100. },
                tip,
                color: Color::Blue,
            };
            let points = marker_points(&marker);
            let mut sign = 0.;
            for i in 0..points.len() {
                let a = points[i];
                let b = points[(i + 1) % points.len()];
                let c = points[(i + 2) % points.len()];
                let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
                if cross.abs() < 0.01 {
                    continue;
                }
                if sign == 0. {
                    sign = cross.signum();
                } else {
                    assert_eq!(sign, cross.signum(), "{tip:?}");
                }
            }
            assert!(marker_hit(&marker, egui::pos2(100., 100.)));
            assert!(!marker_hit(&marker, egui::pos2(130., 130.)));
            let bounds = Rect::from_points(
                &points
                    .iter()
                    .map(|p| Pos2::new(p.x, p.y))
                    .collect::<Vec<_>>(),
            );
            let size = marker.size();
            assert!((bounds.width() - size[0]).abs() < 0.05);
            assert!((bounds.height() - size[1]).abs() < 0.05);
        }
    }
}
