// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::arrows::{Arrow, Mode};

fn strokes(path: &[[f32; 2]], dash: f32, gap: f32) -> Vec<Vec<[f32; 2]>> {
    if dash == 0. {
        return vec![path.to_vec()];
    }
    let mut out = Vec::new();
    let mut current = Vec::new();
    let mut remaining = dash;
    let mut painting = true;
    for pair in path.windows(2) {
        let [a, b] = [pair[0], pair[1]];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = dx.hypot(dy);
        if len < 0.00001 {
            continue;
        }
        let mut at = 0_f64;
        let len = f64::from(len);
        while at < len {
            let end = (at + f64::from(remaining)).min(len);
            let point = |v: f64| {
                let ratio = (v / len) as f32;
                [a[0] + dx * ratio, a[1] + dy * ratio]
            };
            if painting {
                if current.is_empty() {
                    current.push(point(at));
                }
                current.push(point(end));
            }
            remaining -= (end - at) as f32;
            at = end;
            if remaining < 0.00001 {
                if painting && !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
                painting = !painting;
                remaining = if painting { dash } else { gap };
            }
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}
impl Scene {
    pub(super) fn arrow_control(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let p = &layer.properties;
        let b = c.bounds;
        let arrow = Arrow::from_properties(p, b.w, b.h);
        let offset = arrow.display_offset(p);
        let map =
            |point: crate::document::Point| [b.x + point.x - offset.x, b.y + point.y - offset.y];
        let color = rgb(p, "border_color", if sketch { [0; 3] } else { [51; 3] });
        let alpha = number(p, "background_opacity", 1.);
        let pattern = match p.get("stroke") {
            Some(Value::Choice(s)) => s.as_str(),
            _ => "solid",
        };
        let thickness = if sketch { 4. } else { 2. };
        let (dash, gap) = match pattern {
            "dotted" => (1., 8.),
            "dashed" => (thickness * 3., thickness * 6.),
            _ => (0., 0.),
        };
        let clip = layer.clip;
        self.items.push(Item {
            opacity: 1.,
            clip,
            drawing: Drawing::BeginOpacity(alpha),
        });
        let path = arrow.path().into_iter().map(map).collect::<Vec<_>>();
        let mut ink_bounds = Bounds::new(b.x, b.y, b.w, b.h);
        for points in strokes(&path, dash, gap) {
            for point in &points {
                ink_bounds = ink_bounds.union(Bounds::new(
                    point[0] - thickness / 2.,
                    point[1] - thickness / 2.,
                    thickness,
                    thickness,
                ));
            }
            for pair in points.windows(2) {
                self.primitive(
                    super::rules::round_segment(pair[0], pair[1], thickness, color),
                    clip,
                );
            }
        }
        self.items.push(Item {
            opacity: 1.,
            clip,
            drawing: Drawing::EndOpacity,
        });
        let toward = if arrow.mode == Mode::Curve {
            arrow.control()
        } else {
            arrow.middle()
        };
        for (key, tip, other) in [
            ("start_head", arrow.start, arrow.end),
            ("end_head", arrow.end, arrow.start),
        ] {
            if !boolean(p, key) {
                continue;
            }
            let mut dx = toward.x - tip.x;
            let mut dy = toward.y - tip.y;
            if dx.hypot(dy) < 0.00001 {
                dx = other.x - tip.x;
                dy = other.y - tip.y;
            }
            let len = dx.hypot(dy);
            if len < 0.00001 {
                dx = 1.;
                dy = 0.;
            } else {
                dx /= len;
                dy /= len;
            }
            // Original parametric chevrons. Sketch heads are compact and
            // rounded; the clean style retains its narrower stroke.
            let (along, across, head_width) = if sketch {
                (9., 5.5, 5.)
            } else {
                (9.55, 9.55, 2.5)
            };
            let tip = map(tip);
            let points = [
                [
                    tip[0] + along * dx - across * dy,
                    tip[1] + along * dy + across * dx,
                ],
                tip,
                [
                    tip[0] + along * dx + across * dy,
                    tip[1] + along * dy - across * dx,
                ],
            ];
            for point in &points {
                ink_bounds = ink_bounds.union(Bounds::new(
                    point[0] - head_width / 2.,
                    point[1] - head_width / 2.,
                    head_width,
                    head_width,
                ));
            }
            // Each head fades independently of the path, while its two
            // legs compose together so the head's own joint stays uniform.
            self.items.push(Item {
                opacity: 1.,
                clip,
                drawing: Drawing::BeginOpacity(alpha),
            });
            if sketch {
                for pair in points.windows(2) {
                    self.primitive(
                        super::rules::round_segment(pair[0], pair[1], head_width, color),
                        clip,
                    );
                }
            } else {
                self.primitive(
                    Primitive::Line {
                        points: points.to_vec(),
                        color,
                        width: head_width,
                    },
                    clip,
                );
            }
            self.items.push(Item {
                opacity: 1.,
                clip,
                drawing: Drawing::EndOpacity,
            });
        }
        if !c.text.is_empty() {
            let parsed = RichText::parse(&c.text, false);
            let style = TextStyle {
                size: c.font_size,
                bold: c.bold,
                italic: boolean(p, "italic"),
            };
            let options = PaintOptions {
                ink: color32(color),
                ..paint_options(p)
            };
            let measured = fonts.rich_galley_with(&parsed, style, None, false, options);
            if let Some(error) = measured.error {
                return Err(error);
            }
            let center = map(arrow.at(number(p, "label_position", 5.) / 10.));
            let bounds = Bounds::new(
                center[0] - measured.size.x / 2.,
                center[1] - measured.size.y / 2.,
                measured.size.x,
                measured.size.y,
            );
            let frame = Bounds::new(bounds.x - 5., bounds.y - 4., bounds.w + 10., bounds.h + 8.);
            ink_bounds = ink_bounds.union(frame);
            self.items.push(Item {
                opacity: 1.,
                clip,
                drawing: Drawing::BeginOpacity(0.85 * alpha),
            });
            self.primitive(
                Primitive::Rect {
                    b: frame,
                    fill: [204; 3],
                    stroke: false,
                    radius: 4.,
                },
                clip,
            );
            self.primitive(
                Primitive::Rect {
                    b: Bounds::new(frame.x + 1., frame.y + 1., frame.w - 2., frame.h - 2.),
                    fill: [255; 3],
                    stroke: false,
                    radius: 3.,
                },
                clip,
            );
            self.items.push(Item {
                opacity: 1.,
                clip,
                drawing: Drawing::EndOpacity,
            });
            // Caption text remains readable at every arrow opacity. Its fading
            // background has its own scope so native compositing keeps it.
            self.text(
                &parsed,
                style,
                TextBox {
                    bounds,
                    align: Alignment::Center,
                    middle: true,
                    wrap: false,
                    soft_breaks: false,
                    truncate: false,
                    clip,
                },
                fonts,
                options,
            )?;
        }
        let visible = clip.map_or(ink_bounds, |clip| intersect(ink_bounds, clip));
        if visible.w > 0. && visible.h > 0. {
            self.bounds = self.bounds.union(visible);
        }
        Ok(())
    }
}
