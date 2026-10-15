// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

pub(super) fn button_text_colors(layer: &Layer, sketch: bool, options: &mut PaintOptions) {
    let state = match layer.properties.get("state") {
        Some(Value::Choice(s)) => s.as_str(),
        _ => "normal",
    };
    if state == "disabled" {
        return;
    }
    let fill = if state == "selected" {
        selection_color(&layer.properties)
    } else {
        layer.control.fill
    };
    *options = background_text_options(
        &layer.properties,
        fill,
        1.,
        if sketch { [0; 3] } else { [51; 3] },
        false,
    );
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
        if button && !sketch {
            self.form_border(layer, sketch);
            return;
        }
        let selection = selection_color(&layer.properties);
        let fill = if button && state == "selected" {
            selection
        } else {
            control.fill
        };
        let clip = clip_with(layer.clip, bounds);
        let frame = if button && sketch {
            Bounds::new(
                bounds.x + 2.,
                bounds.y + 2.,
                (bounds.w - 6.).max(0.),
                (bounds.h - 5.).max(0.),
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
        if !button {
            self.items.last_mut().expect("Form background").opacity =
                number(&layer.properties, "background_opacity", 1.);
        }
        if control.kind != Kind::TextArea {
            self.form_border(layer, sketch);
        }
    }
    pub(super) fn form_border(&mut self, layer: &Layer, sketch: bool) {
        let control = &layer.control;
        let bounds = control.bounds;
        let state = match layer.properties.get("state") {
            Some(Value::Choice(s)) => s.as_str(),
            _ => "normal",
        };
        let button = control.kind == Kind::Button;
        let selection = selection_color(&layer.properties);
        let border = match state {
            "disabled" if button && !sketch => [204, 204, 204],
            "disabled" => [128, 128, 139],
            "focused" => selection,
            _ if !button => rgb(
                &layer.properties,
                "border_color",
                if sketch { [0; 3] } else { [51; 3] },
            ),
            _ if sketch => [0, 0, 0],
            _ => [51, 51, 51],
        };
        let clip = clip_with(layer.clip, bounds);
        if button && !sketch {
            let radius = 3.5_f32.min(bounds.w / 2.).min(bounds.h / 2.);
            self.primitive(
                Primitive::Rect {
                    b: bounds,
                    fill: border,
                    stroke: false,
                    radius,
                },
                clip,
            );
            if bounds.w > 2. && bounds.h > 2. {
                self.primitive(
                    Primitive::Rect {
                        b: Bounds::new(bounds.x + 1., bounds.y + 1., bounds.w - 2., bounds.h - 2.),
                        fill: if state == "selected" {
                            selection
                        } else {
                            control.fill
                        },
                        stroke: false,
                        radius: (radius - 1.).max(0.),
                    },
                    clip,
                );
            }
            return;
        }
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
        if control.kind == Kind::TextInput
            && matches!(layer.properties.get("border"), Some(Value::Choice(s)) if s == "line")
        {
            let y = bounds.y + bounds.h - if sketch { 1. } else { 0.5 };
            self.primitive(
                Primitive::Line {
                    points: vec![[bounds.x, y], [bounds.x + bounds.w, y]],
                    color: border,
                    width: if sketch { 2. } else { 1. },
                },
                clip,
            );
            return;
        }
        if sketch {
            self.primitive(
                outlines::sketch(
                    bounds,
                    if button {
                        outlines::Profile::Button
                    } else {
                        outlines::Profile::Square
                    },
                    border,
                ),
                clip,
            );
        } else {
            self.rectangle_frame(frame, border, 1., clip);
        }
    }
}
