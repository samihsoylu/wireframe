// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Horizontal rule dimensions and original breakline geometry.
use crate::document::{Properties, Value};

pub fn stroke(properties: &Properties) -> &str {
    match properties.get("stroke") {
        Some(Value::Choice(s)) => s,
        _ => "solid",
    }
}
pub fn legacy_frame(properties: &Properties) -> bool {
    properties.get("legacy_rule_frame") == Some(&Value::Bool(true))
        && !matches!(stroke(properties), "breakline_top" | "breakline_bottom")
}
pub fn height(properties: &Properties) -> f32 {
    if matches!(stroke(properties), "breakline_top" | "breakline_bottom") {
        38.
    } else {
        10.
    }
}
pub fn edge(width: f32, baseline: f32, thickness: f32) -> Vec<[f32; 2]> {
    if width < 65. {
        vec![[0., baseline], [width, baseline]]
    } else {
        crate::browser::torn_edge(width, baseline, thickness)
    }
}
pub fn white_region(width: f32, height: f32, above: bool) -> Vec<[f32; 2]> {
    let mut points = edge(width, height / 2., 0.);
    let y = if above { 0. } else { height };
    points.extend([[width, y], [0., y]]);
    points
}
