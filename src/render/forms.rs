// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

fn selection_color(sketch: bool) -> [u8; 3] {
    if sketch {
        [149, 188, 242]
    } else {
        [151, 189, 240]
    }
}
pub(super) fn button_text_colors(layer: &Layer, sketch: bool, options: &mut PaintOptions) {
    let state = match layer.properties.get("state") {
        Some(Value::Choice(s)) => s.as_str(),
        _ => "normal",
    };
    if state == "disabled" {
        return;
    }
    let fill = if state == "selected" {
        selection_color(sketch)
    } else {
        layer.control.fill
    };
    let brightness = u32::from(fill[0]) * 299 + u32::from(fill[1]) * 587 + u32::from(fill[2]) * 114;
    options.ink = if brightness < 153_000 {
        Color32::WHITE
    } else if sketch {
        Color32::BLACK
    } else {
        Color32::from_gray(51)
    };
    options.link = if brightness < 153_000 {
        Color32::WHITE
    } else {
        Color32::from_rgb(8, 83, 148)
    };
}

impl Scene {
    pub(super) fn form_control(&mut self, layer: &Layer, sketch: bool) {
        let control = &layer.control;
        let bounds = control.bounds;
        let state = match layer.properties.get("state") {
            Some(Value::Choice(s)) => s.as_str(),
            _ => "normal",
        };
        let button = control.kind == Kind::Button;
        let selection = selection_color(sketch);
        let border = match state {
            "disabled" if button && !sketch => [204, 204, 204],
            "disabled" => [128, 128, 139],
            "focused" => selection,
            _ if sketch => [0, 0, 0],
            _ => [51, 51, 51],
        };
        let fill = if button && state == "selected" {
            selection
        } else {
            control.fill
        };
        let clip = clip_with(layer.clip, bounds);
        let frame = if button && sketch {
            Bounds::new(
                bounds.x,
                bounds.y,
                (bounds.w - 3.).max(0.),
                (bounds.h - 3.).max(0.),
            )
        } else {
            bounds
        };
        self.primitive(
            Primitive::Rect {
                b: frame,
                fill,
                stroke: false,
                radius: 0.,
            },
            clip,
        );
        let (x, y, w, h) = (frame.x, frame.y, frame.w, frame.h);
        let inset = (if sketch { 1.0_f32 } else { 0.5_f32 })
            .min(w / 2.)
            .min(h / 2.);
        self.primitive(
            Primitive::Line {
                points: vec![
                    [x + inset, y + inset],
                    [x + w - inset, y + inset],
                    [x + w - inset, y + h - inset],
                    [x + inset, y + h - inset],
                    [x + inset, y + inset],
                ],
                color: border,
                width: if sketch { 2. } else { 1. },
            },
            clip,
        );
        if button && sketch {
            self.primitive(
                Primitive::Line {
                    points: vec![
                        [x + w + 1.5, y + 2.],
                        [x + w + 1.5, y + h + 1.5],
                        [x + 2., y + h + 1.5],
                    ],
                    color: border,
                    width: 3.,
                },
                clip,
            );
        }
    }
}
