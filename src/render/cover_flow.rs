// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Original perspective-panel artwork with a shared image and scrollbar pipeline.
use super::{media_controls::Art, *};
impl Scene {
    pub(super) fn cover_flow(
        &mut self,
        layer: &Layer,
        sketch: bool,
        rasters: &mut rasters::Rasters<'_>,
    ) {
        let b = layer.control.bounds;
        let l =
            crate::cover_flow::layout(b.w, b.h, boolean(&layer.properties, "vertical_scrollbar"));
        let clip = clip_with(layer.clip, b);
        let translate = |r: Bounds| Bounds::new(b.x + r.x, b.y + r.y, r.w, r.h);
        if let Some(Value::Asset(id)) = layer.properties.get("image")
            && let Some(data) = rasters.get(
                *id,
                rasters::Style::Image {
                    turns: crate::images::quarter_turns(&layer.properties),
                    sketch: boolean(&layer.properties, "sketch_filter"),
                },
            )
        {
            self.items.push(Item {
                opacity: 1.,
                drawing: Drawing::Image {
                    bounds: translate(l.image),
                    data,
                },
                clip,
            });
        }
        let mut art = Art {
            scene: self,
            origin: [b.x, b.y],
            scale: [l.frame.w / 493., l.frame.h / 198.],
            clip,
        };
        let ink = if sketch { [0; 3] } else { [103; 3] };
        let stroke = if sketch { 5.5 } else { 0.8 };
        let mut outline = |points: Vec<[f32; 2]>| {
            art.line(points.clone(), ink, stroke);
            if sketch {
                // Fill the butt-stroke corner gaps with original rounded joins.
                // The drawing is opaque before control/group opacity is applied.
                for point in points.iter().take(points.len().saturating_sub(1)) {
                    art.ellipse(
                        Bounds::new(
                            point[0] - stroke / 2.,
                            point[1] - stroke / 2.,
                            stroke,
                            stroke,
                        ),
                        ink,
                        None,
                    );
                }
            }
        };
        // Independent perspective panels: the inward edges meet the center window.
        for (outer, inner, top, bottom) in [
            (5., 45., 8., 182.),
            (45., 87., 10., 183.),
            (87., 170., 8., 183.),
        ] {
            for mirrored in [false, true] {
                let points = vec![
                    [outer, top],
                    [inner, top + (inner - outer) * 0.36],
                    [inner, bottom - (inner - outer) * 0.30],
                    [outer, bottom],
                    [outer, top],
                ];
                let points = points
                    .into_iter()
                    .enumerate()
                    .map(|(i, [x, y])| {
                        let x = if mirrored { 493. - x } else { x };
                        if sketch && i > 0 && i < 4 {
                            [
                                x + if i % 2 == 0 { 0.7 } else { -0.6 },
                                y + if i == 1 { 1.2 } else { -0.8 },
                            ]
                        } else {
                            [x, y]
                        }
                    })
                    .collect();
                outline(points);
            }
        }
        outline(if sketch {
            vec![
                [170., 22.],
                [246., 23.],
                [323., 22.],
                [322.5, 100.],
                [324., 176.],
                [246., 177.],
                [169.5, 176.],
                [170.5, 99.],
                [170., 22.],
            ]
        } else {
            vec![
                [169.5, 21.5],
                [323.5, 21.5],
                [323.5, 176.5],
                [169.5, 176.5],
                [169.5, 21.5],
            ]
        });
        if let Some(bar) = l.scrollbar {
            self.horizontal_scrollbar(
                translate(bar),
                number(&layer.properties, "value", 0.),
                sketch,
                clip,
            );
        }
    }
}
