// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

#[derive(Clone, Copy)]
pub(super) enum Mark {
    Empty,
    Selected,
    Mixed,
}
impl Scene {
    pub(super) fn choice_control(&mut self, layer: &Layer) {
        let c = &layer.control;
        let radio = c.kind == Kind::Radio;
        let size = if radio { 13. } else { 16. };
        let state = match layer.properties.get("state") {
            Some(Value::Choice(s)) => s.as_str(),
            _ => "normal",
        };
        let mark = if state.contains("indeterminate") {
            Mark::Mixed
        } else if state.contains("selected") {
            Mark::Selected
        } else {
            Mark::Empty
        };
        let b = Bounds::new(
            c.bounds.x,
            c.bounds.y + center_offset(c.bounds.h, size),
            size,
            size,
        );
        self.choice_symbol(
            b,
            radio,
            mark,
            clip_with(layer.clip, c.bounds),
            paint_options(&layer.properties).ink,
        );
    }
    pub(super) fn choice_symbol(
        &mut self,
        b: Bounds,
        radio: bool,
        mark: Mark,
        clip: Option<Bounds>,
        color: Color32,
    ) {
        let id = match (radio, mark) {
            (false, Mark::Empty) => "square-regular",
            (false, Mark::Selected) => "square-check-regular",
            (false, Mark::Mixed) => "square-minus-regular",
            (true, Mark::Selected) => "circle-dot-regular",
            (true, _) => "circle-regular",
        };
        self.items.push(Item {
            drawing: Drawing::Icon {
                bounds: b,
                id: id.into(),
                color: color.to_array(),
                turns: 0,
            },
            clip,
            opacity: 1.,
        });
        if radio && matches!(mark, Mark::Mixed) {
            // Original rounded minus, aligned with the licensed circle's stroke weight.
            let half_segment = b.w * 9. / 64.;
            self.primitive(
                Primitive::Line {
                    points: vec![
                        [b.x + b.w / 2. - half_segment, b.y + b.h / 2.],
                        [b.x + b.w / 2. + half_segment, b.y + b.h / 2.],
                    ],
                    color: [color.r(), color.g(), color.b()],
                    width: b.w * 3. / 32.,
                },
                clip,
            );
        }
    }
}

/// Center on the pixel grid; half-unit ties move toward the positive axis.
/// A symbol can be taller than its row, so negative offsets matter too.
pub(super) fn center_offset(row: f32, symbol: f32) -> f32 {
    ((row - symbol) / 2. + 0.5).floor()
}
