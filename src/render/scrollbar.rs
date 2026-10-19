// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

enum ScrollbarAppearance {
    Inline,
    Panel,
    Browser,
}

impl Scene {
    /// Shared Clean artwork for standalone and embedded scrollbars on either axis.
    /// Placement and covered-link handling belong to the calling control.
    pub(super) fn clean_scrollbar(
        &mut self,
        b: Bounds,
        vertical: bool,
        value: f32,
        parent_clip: Option<Bounds>,
    ) {
        if b.w <= 0. || b.h <= 0. {
            return;
        }
        let clip = clip_with(parent_clip, b);
        let (length, thickness) = if vertical { (b.h, b.w) } else { (b.w, b.h) };
        let point = |along: f32, across: f32| {
            if vertical {
                [b.x + across, b.y + along]
            } else {
                [b.x + along, b.y + across]
            }
        };
        let ink = [81; 3];
        self.primitive(
            Primitive::Rect {
                b: Bounds::new(b.x + 2., b.y + 2., (b.w - 4.).max(0.), (b.h - 4.).max(0.)),
                fill: [255; 3],
                stroke: false,
                radius: 0.,
            },
            clip,
        );
        // Filled strips give square, opaque corners at whole-pixel export scale.
        let edge_x = b.w.min(1.);
        let edge_y = b.h.min(1.);
        for edge in [
            Bounds::new(b.x, b.y, b.w, edge_y),
            Bounds::new(b.x, b.y + b.h - edge_y, b.w, edge_y),
            Bounds::new(b.x, b.y, edge_x, b.h),
            Bounds::new(b.x + b.w - edge_x, b.y, edge_x, b.h),
        ] {
            self.primitive(
                Primitive::Rect {
                    b: edge,
                    fill: ink,
                    stroke: false,
                    radius: 0.,
                },
                clip,
            );
        }
        let end = 12_f32.min(length / 2.);
        let middle = thickness / 2.;
        let cross_ratio = if vertical { 1. / 3. } else { 3. / 8. };
        let half_width = ((thickness - 4.).max(0.) * cross_ratio).min(middle);
        let depth = if vertical { 2. } else { 2.5 } * end / 12.;
        let arrow_center = if vertical { 6. } else { 5.5 } * end / 12.;
        for (along, direction) in [(arrow_center, -1.), (length - arrow_center, 1.)] {
            self.primitive(
                Primitive::Polygon {
                    points: vec![
                        point(along - direction * depth, middle - half_width),
                        point(along - direction * depth, middle + half_width),
                        point(along + direction * depth, middle),
                    ],
                    fill: ink,
                },
                clip,
            );
        }
        if length > 45. {
            let along = 13. + (length - 48.) * value.clamp(0., 1.);
            let thumb = if vertical {
                Bounds::new(b.x + middle - 4., b.y + along, 8., 21.)
            } else {
                Bounds::new(b.x + along, b.y + middle - 4., 21., 8.)
            };
            self.primitive(
                Primitive::Rect {
                    b: thumb,
                    fill: ink,
                    stroke: false,
                    radius: 4.,
                },
                clip,
            );
        }
    }

    pub(super) fn vertical_scrollbar(
        &mut self,
        b: Bounds,
        value: f32,
        disabled: bool,
        sketch: bool,
        parent_clip: Option<Bounds>,
    ) {
        self.draw_vertical_scrollbar(
            b,
            value,
            disabled,
            (sketch, ScrollbarAppearance::Inline),
            parent_clip,
        );
    }
    pub(super) fn panel_scrollbar(
        &mut self,
        b: Bounds,
        value: f32,
        disabled: bool,
        sketch: bool,
        parent_clip: Option<Bounds>,
    ) {
        self.draw_vertical_scrollbar(
            b,
            value,
            disabled,
            (sketch, ScrollbarAppearance::Panel),
            parent_clip,
        );
    }
    pub(super) fn browser_scrollbar(
        &mut self,
        b: Bounds,
        value: f32,
        sketch: bool,
        parent_clip: Option<Bounds>,
    ) {
        self.draw_vertical_scrollbar(
            b,
            value,
            false,
            (sketch, ScrollbarAppearance::Browser),
            parent_clip,
        );
    }
    fn draw_vertical_scrollbar(
        &mut self,
        b: Bounds,
        value: f32,
        disabled: bool,
        appearance: (bool, ScrollbarAppearance),
        parent_clip: Option<Bounds>,
    ) {
        let (sketch, appearance) = appearance;
        let panel = !matches!(appearance, ScrollbarAppearance::Inline);
        let browser = matches!(appearance, ScrollbarAppearance::Browser);
        if b.h <= 0. || b.w <= 0. {
            return;
        }
        let clip = clip_with(parent_clip, b);
        // Covered inline links must not remain interactive through the track.
        let cover = clip.map_or(b, |c| intersect(b, c));
        if !panel {
            self.links = self
                .links
                .drain(..)
                .flat_map(|link| {
                    subtract(link.bounds, cover)
                        .into_iter()
                        .map(move |bounds| LinkRegion {
                            quad: None,
                            hint: link.hint,
                            target: link.target.clone(),
                            bounds,
                        })
                })
                .collect();
        }
        if !sketch {
            self.clean_scrollbar(b, true, value, parent_clip);
            return;
        }
        // This is a diagram of a scrollbar. Its value moves only the thumb.
        let color = if disabled { [128, 128, 139] } else { [0; 3] };
        let background = Bounds::new(b.x + 2., b.y + 2., (b.w - 4.).max(0.), (b.h - 4.).max(0.));
        self.primitive(
            Primitive::Rect {
                b: background,
                fill: [255; 3],
                stroke: false,
                radius: 3.,
            },
            clip,
        );
        let width = 1.5;
        let inset = width / 2.;
        self.primitive(
            Primitive::Line {
                points: vec![
                    [b.x + inset, b.y + inset],
                    [b.x + b.w - inset, b.y + inset],
                    [b.x + b.w - inset, b.y + b.h - inset],
                    [b.x + inset, b.y + b.h - inset],
                    [b.x + inset, b.y + inset],
                ],
                color,
                width,
            },
            clip,
        );
        let separators = if browser {
            vec![b.y + 11., b.y + b.h - 11.]
        } else if panel {
            vec![b.y + 14., b.y + b.h - 15.]
        } else {
            vec![b.y + 12., b.y + b.h - 12.]
        };
        for y in separators {
            self.primitive(
                Primitive::Line {
                    points: vec![[b.x, y], [b.x + b.w, y]],
                    color,
                    width,
                },
                clip,
            );
        }
        let x = b.x + b.w / 2.;
        for (y, direction) in [(b.y + 6., -1.), (b.y + b.h - 6., 1.)] {
            let half_height = if browser { 3.5 } else { 2.5 };
            self.primitive(
                Primitive::Polygon {
                    points: vec![
                        [x - 4., y - direction * half_height],
                        [x + 4., y - direction * half_height],
                        [x, y + direction * half_height],
                    ],
                    fill: color,
                },
                clip,
            );
        }
        if b.h > 45. {
            self.primitive(
                Primitive::Rect {
                    b: Bounds::new(
                        x - 4.,
                        b.y + 13. + (b.h - 48.) * value.clamp(0., 1.),
                        8.,
                        21.,
                    ),
                    fill: color,
                    stroke: false,
                    radius: 2.,
                },
                clip,
            );
        }
    }
}
