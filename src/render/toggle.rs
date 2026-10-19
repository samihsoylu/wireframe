// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Scene {
    pub(super) fn toggle(&mut self, layer: &Layer, sketch: bool) {
        let b = layer.control.bounds;
        let on = matches!(layer.properties.get("state"), Some(Value::Choice(s)) if s == "on");
        let color = layer.control.fill;
        let border = if sketch {
            [1; 3]
        } else if on {
            color
        } else {
            [153; 3]
        };
        let background = if on {
            color
        } else if sketch {
            [226; 3]
        } else {
            [255; 3]
        };
        let clip = clip_with(layer.clip, b);
        // Original parametric contours, shared by canvas and all exports.
        // Off reflects the whole drawing, keeping the same outer bounds.
        let mut contour = |left: f32, right: f32, radius: f32, fill| {
            let points = (0..=64)
                .map(|i| {
                    let angle = i as f32 * std::f32::consts::TAU / 64.;
                    let r = if sketch {
                        radius - 0.12 + 0.1 * (3. * angle).sin()
                    } else {
                        radius
                    };
                    let center = if angle.cos() >= 0. { right } else { left };
                    let x = center + r * angle.cos();
                    let y = 12. + r * angle.sin();
                    let (x, y) = if on { (x, y) } else { (42. - x, 24. - y) };
                    [b.x + x * b.w / 42., b.y + y * b.h / 24.]
                })
                .collect();
            self.primitive(Primitive::Polygon { points, fill }, clip);
        };
        let stroke = if sketch { 0.9 } else { 12. / 7. };
        contour(12., 30., 12., border);
        contour(12., 30., 12. - stroke, background);
        contour(30., 30., 12., border);
        contour(30., 30., 12. - stroke, [255; 3]);
    }
}
