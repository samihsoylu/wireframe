// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

impl Scene {
    pub(super) fn vertical_scrollbar(
        &mut self,
        b: Bounds,
        value: f32,
        disabled: bool,
        sketch: bool,
        parent_clip: Option<Bounds>,
    ) {
        if b.h <= 0. || b.w <= 0. {
            return;
        }
        let clip = clip_with(parent_clip, b);
        // This is a diagram of a scrollbar. Its value moves only the thumb.
        let color = if disabled {
            [128, 128, 139]
        } else if sketch {
            [0; 3]
        } else {
            [51; 3]
        };
        let background = Bounds::new(b.x + 2., b.y + 2., (b.w - 4.).max(0.), (b.h - 4.).max(0.));
        self.primitive(
            Primitive::Rect {
                b: background,
                fill: [255; 3],
                stroke: false,
                radius: if sketch { 3. } else { 0. },
            },
            clip,
        );
        // Covered inline links must not remain interactive through the track.
        let cover = clip.map_or(b, |c| intersect(b, c));
        self.links = self
            .links
            .drain(..)
            .flat_map(|link| {
                subtract(link.bounds, cover)
                    .into_iter()
                    .map(move |bounds| LinkRegion {
                        target: link.target.clone(),
                        bounds,
                    })
            })
            .collect();
        let width = if sketch { 1.5 } else { 1. };
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
        for y in [b.y + 12., b.y + b.h - 12.] {
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
            self.primitive(
                Primitive::Polygon {
                    points: vec![
                        [x - 4., y - direction * 2.5],
                        [x + 4., y - direction * 2.5],
                        [x, y + direction * 2.5],
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
                    radius: if sketch { 2. } else { 0. },
                },
                clip,
            );
        }
    }
}
