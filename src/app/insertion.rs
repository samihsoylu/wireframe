// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

use super::*;

impl WireframeApp {
    pub(super) fn insertion_for_size(&self, size: Vec2) -> Pos2 {
        let controls = self.editor.current().controls;
        let coordinate_key = |x: f32, y: f32| {
            let bits = |v: f32| if v == 0. { 0 } else { v.to_bits() };
            (bits(x), bits(y))
        };
        let occupied: HashSet<_> = controls
            .iter()
            .map(|c| coordinate_key(c.bounds.x, c.bounds.y))
            .collect();
        let height = if self.canvas_rect.is_finite() {
            self.canvas_rect.height()
        } else {
            600.
        };
        let mut visited = HashSet::new();
        let mut point = Pos2::ZERO;
        let mut offset = Vec2::splat(50.);
        // There can be no more occupied distinct origins than controls. The
        // visited set also terminates when clamping maps two slots to one point.
        for _ in 0..=controls.len() {
            if offset.y > height {
                offset.x += 50.;
                offset.y = 50.;
            }
            // Offset is already relative to the canvas. This also works before
            // its first layout, when the stored screen rectangle is empty.
            let world = (offset - self.pan) / self.zoom;
            point = Pos2::new(
                round_coordinate(world.x).clamp(0., (10_000. - size.x).max(0.)),
                round_coordinate(world.y).clamp(0., (10_000. - size.y).max(0.)),
            );
            let key = coordinate_key(point.x, point.y);
            if !visited.insert(key) || !occupied.contains(&key) {
                break;
            }
            offset.y += 50.;
        }
        point
    }
}
