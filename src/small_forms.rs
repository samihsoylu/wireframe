// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Geometry and colors for compact form controls, shared by all renderers.
use crate::{layout::TextMetrics, model::Bounds};

pub fn fixed_size(kind: &str) -> Option<(f32, f32)> {
    match kind {
        "color_picker" => Some((26., 28.)),
        "help_button" => Some((18., 18.)),
        _ => None,
    }
}
pub fn stepper_size(text: TextMetrics, width: Option<f32>) -> (f32, f32) {
    (
        width.unwrap_or(text.width + 33.).max(1.),
        (text.height + 11.).max(1.),
    )
}
pub fn stepper_text(width: f32, height: f32) -> Bounds {
    Bounds::new(9., 6., (width - 33.).max(1.), (height - 11.).max(1.))
}
pub fn stepper_button(width: f32, height: f32) -> Bounds {
    Bounds::new(width - 18., 0., 18., height)
}
pub fn picker_swatch(sketch: bool) -> Bounds {
    Bounds::new(2., 2., 22., if sketch { 20. } else { 22. })
}
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct StepperColors {
    pub border: [u8; 3],
    pub text: [u8; 3],
    pub link: [u8; 3],
    pub button: [u8; 3],
}
pub fn stepper_colors(sketch: bool, disabled: bool, ink: [u8; 3]) -> StepperColors {
    let muted = [128, 128, 139];
    StepperColors {
        border: if disabled {
            if sketch { muted } else { [205, 204, 204] }
        } else {
            ink
        },
        text: if disabled { muted } else { ink },
        link: if disabled { muted } else { [8, 83, 148] },
        button: if sketch { [255; 3] } else { [221; 3] },
    }
}
/// Map original artwork through a frame with stable corner dimensions. When the
/// frame is too small, its opposing corners shrink proportionally.
fn axis(value: f32, source: f32, target: f32, first: f32, last: f32) -> f32 {
    let (value, source, target, first, last) = (
        f64::from(value),
        f64::from(source),
        f64::from(target),
        f64::from(first),
        f64::from(last),
    );
    let ratio = (target / (first + last)).min(1.);
    let a = first * ratio;
    let center = (target - (first + last) * ratio).max(0.);
    let mapped = if value <= first {
        value * ratio
    } else if value >= source - last {
        a + center + (value - (source - last)) * ratio
    } else {
        a + (value - first) / (source - first - last) * center
    };
    mapped.clamp(0., target) as f32
}
pub fn stepper_point(point: [f32; 2], width: f32, height: f32, sketch: bool) -> [f32; 2] {
    let (top, right, bottom, source_height) = if sketch {
        (4., 19., 5., 26.)
    } else {
        (3., 20., 3., 24.)
    };
    [
        axis(point[0], 44., width, 3., right),
        axis(point[1], source_height, height, top, bottom),
    ]
}
