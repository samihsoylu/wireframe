// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Geometry and colors for illustrative one-axis sliders.
use crate::{
    document::{Properties, Value},
    model::Bounds,
};
use serde::Serialize;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Thumb {
    Pointy,
    Rectangle,
    Circle,
}
#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub vertical: bool,
    pub sketch: bool,
    pub length: f32,
    pub value: f32,
    pub thumb: Thumb,
    pub disabled: bool,
    pub color: [u8; 3],
}
impl Options {
    pub fn from_properties(
        properties: &Properties,
        vertical: bool,
        sketch: bool,
        length: f32,
    ) -> Self {
        Self {
            vertical,
            sketch,
            length,
            value: match properties.get("value") {
                Some(Value::Number(v)) => *v as f32,
                _ => 0.05,
            },
            thumb: match properties.get("slider_thumb") {
                Some(Value::Choice(s)) if s == "circle" => Thumb::Circle,
                Some(Value::Choice(s)) if s == "rectangle" => Thumb::Rectangle,
                _ => Thumb::Pointy,
            },
            disabled: matches!(properties.get("state"),Some(Value::Choice(s)) if s=="disabled"),
            color: match properties.get("fill") {
                Some(Value::Color(c)) => [c[0], c[1], c[2]],
                _ => {
                    if sketch {
                        [0; 3]
                    } else {
                        [102; 3]
                    }
                }
            },
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Layout {
    pub thumb: Bounds,
    pub tracks: [Bounds; 2],
    pub leading_color: [u8; 3],
    pub rest_color: [u8; 3],
}
pub fn layout(o: Options) -> Layout {
    let intrinsic = if !o.sketch && o.thumb == Thumb::Circle {
        12.
    } else {
        14.
    };
    let pad = if o.thumb == Thumb::Circle {
        if o.sketch && o.vertical { 0. } else { 0.5 }
    } else if o.sketch {
        0.
    } else {
        1.
    };
    let position = (o.length - intrinsic) * o.value.clamp(0., 1.);
    let split = position + intrinsic / 2.;
    let (thumb, tracks) = if o.vertical {
        let x = if o.sketch { 0.5 } else { 2. };
        (
            Bounds::new(pad, position, 12., 12.),
            [
                Bounds::new(x, 0., 12., split),
                Bounds::new(x, split, 12., o.length - split),
            ],
        )
    } else {
        let y = if o.sketch { 0. } else { 1. };
        // Trailing artwork extends through the clip edge, avoiding an extra end cap.
        (
            Bounds::new(position, pad, 12., 12.),
            [
                Bounds::new(0., y, split, 12.),
                Bounds::new(split, y, o.length - y, 12.),
            ],
        )
    };
    let rest_color = if o.disabled {
        if o.sketch { [137; 3] } else { [204; 3] }
    } else if o.sketch {
        [0; 3]
    } else {
        [102; 3]
    };
    Layout {
        thumb,
        tracks,
        leading_color: if o.disabled { rest_color } else { o.color },
        rest_color,
    }
}

/// Resize one end without moving or scaling the fixed-thickness axis.
pub fn resize(bounds: Bounds, vertical: bool, edge: i8, delta: f32) -> Bounds {
    let old = if vertical { bounds.h } else { bounds.w };
    let length = (old + delta * edge as f32).clamp(1., 10_000.);
    if vertical {
        Bounds::new(
            bounds.x,
            if edge < 0 {
                bounds.y + old - length
            } else {
                bounds.y
            },
            12.,
            length,
        )
    } else {
        Bounds::new(
            if edge < 0 {
                bounds.x + old - length
            } else {
                bounds.x
            },
            bounds.y,
            length,
            12.,
        )
    }
}
