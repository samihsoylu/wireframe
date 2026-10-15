// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Original contours with fixed-size corners and bounded edge variation.
//! A monotone coordinate map compresses opposing corners for small frames.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum Profile {
    Square,
    Button,
    Rounded,
}

fn axis(v: f32, size: f32, target: f32, first: f32, last: f32) -> f32 {
    if target < first + last {
        if v < first {
            v * target / (first + last)
        } else if v > size - last {
            target - (size - v) * target / (first + last)
        } else {
            first * target / (first + last)
        }
    } else if v < first {
        v
    } else if v > size - last {
        target - (size - v)
    } else {
        first + (v - first) * (target - first - last) / (size - first - last)
    }
}

// The same vertex topology describes both sides of a border. Joining them as
// one surface avoids seams and leaves transparent interiors truly transparent.
fn contour(b: Bounds, rx: f32, top: f32, bottom: f32, bow: f32) -> Vec<[f32; 2]> {
    let rx = rx.min(b.w / 2.).max(0.);
    let top = top.min(b.h / 2.).max(0.);
    let bottom = bottom.min(b.h / 2.).max(0.);
    let corners = [
        (b.x + b.w - rx, b.y + top, top, -90.),
        (b.x + b.w - rx, b.y + b.h - bottom, bottom, 0.),
        (b.x + rx, b.y + b.h - bottom, bottom, 90.),
        (b.x + rx, b.y + top, top, 180.),
    ];
    let mut points = Vec::with_capacity(100);
    for (i, &(cx, cy, ry, start)) in corners.iter().enumerate() {
        for step in 0..=12 {
            let angle = (start + step as f32 * 7.5).to_radians();
            points.push([cx + rx * angle.cos(), cy + ry * angle.sin()]);
        }
        let end = *points.last().unwrap();
        let (nx, ny, nr, ns) = corners[(i + 1) % 4];
        let angle = ns.to_radians();
        let next = [nx + rx * angle.cos(), ny + nr * angle.sin()];
        for step in 1..12 {
            let t = step as f32 / 12.;
            let wave = (t * std::f32::consts::PI).sin()
                * (0.7 + 0.3 * (t * std::f32::consts::TAU).cos())
                * bow;
            let mut p = [
                end[0] + (next[0] - end[0]) * t,
                end[1] + (next[1] - end[1]) * t,
            ];
            // Small, intentionally unequal bows on the four sides.
            match i {
                0 => p[0] -= wave * 0.8,
                1 => p[1] -= wave,
                2 => p[0] += wave * 0.6,
                _ => p[1] += wave * 0.55,
            }
            points.push(p);
        }
    }
    points.rotate_left(84);
    points
}

fn ring(outer: Vec<[f32; 2]>, inner: Vec<[f32; 2]>, color: [u8; 3]) -> Primitive {
    let n = outer.len();
    debug_assert_eq!(n, inner.len());
    let mut points = outer;
    points.extend(inner);
    let mut indices = Vec::with_capacity(n * 6);
    for i in 0..n {
        let j = (i + 1) % n;
        indices.extend([
            i as u32,
            j as u32,
            (n + i) as u32,
            j as u32,
            (n + j) as u32,
            (n + i) as u32,
        ]);
    }
    Primitive::Mesh {
        points,
        indices,
        fill: color,
    }
}

pub(super) fn sketch(b: Bounds, profile: Profile, color: [u8; 3]) -> Primitive {
    let (base, margins, outer, inner) = match profile {
        Profile::Square => (
            [48., 34.],
            [5., 5., 5., 5.],
            contour(Bounds::new(0.25, 0.2, 47.5, 33.6), 1.4, 1.4, 1.1, 0.5),
            contour(Bounds::new(2.05, 2., 43.9, 30.), 0.8, 0.7, 0.6, 0.35),
        ),
        Profile::Button => (
            [48., 34.],
            [5., 6., 4., 6.],
            contour(Bounds::new(0.25, 0.2, 47.5, 33.6), 1.4, 1.5, 1., 0.45),
            contour(Bounds::new(2., 2., 42., 28.), 0.6, 0.6, 0.4, 0.45),
        ),
        Profile::Rounded => (
            [70., 42.],
            [20., 20., 15., 12.],
            contour(
                Bounds::new(0.25, 0.25, 69.5, 41.5),
                19.75,
                14.75,
                11.75,
                0.6,
            ),
            contour(Bounds::new(2.1, 2.05, 65.8, 37.9), 17.9, 12.95, 9.95, 0.4),
        ),
    };
    let map = |points: Vec<[f32; 2]>| {
        points
            .into_iter()
            .map(|p| {
                [
                    b.x + axis(p[0], base[0], b.w, margins[0], margins[1]),
                    b.y + axis(p[1], base[1], b.h, margins[2], margins[3]),
                ]
            })
            .collect()
    };
    ring(map(outer), map(inner), color)
}

fn dash_mesh(path: &[[f32; 2]], width: f32, color: [u8; 3]) -> Primitive {
    let mut strokes: Vec<Vec<[f32; 2]>> = Vec::new();
    let mut stroke = Vec::new();
    let mut traveled = 0.;
    for i in 0..path.len() {
        let a = path[i];
        let b = path[(i + 1) % path.len()];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let length = dx.hypot(dy);
        if length < 0.0001 {
            continue;
        }
        let mut at = 0.;
        while at < length {
            let phase = (traveled + at) % 16.;
            let end = (at + if phase < 8. { 8. - phase } else { 16. - phase }).min(length);
            if phase < 8. {
                if stroke.is_empty() {
                    stroke.push([a[0] + dx * at / length, a[1] + dy * at / length]);
                }
                stroke.push([a[0] + dx * end / length, a[1] + dy * end / length]);
            } else if !stroke.is_empty() {
                strokes.push(std::mem::take(&mut stroke));
            }
            if end <= at {
                break;
            }
            at = end;
        }
        traveled += length;
    }
    if !stroke.is_empty() {
        strokes.push(stroke);
    }
    let mut points = Vec::new();
    let mut indices = Vec::new();
    for stroke in strokes {
        let start = points.len() as u32;
        let normals: Vec<_> = stroke
            .windows(2)
            .map(|p| {
                let dx = p[1][0] - p[0][0];
                let dy = p[1][1] - p[0][1];
                let len = dx.hypot(dy).max(0.0001);
                [-dy / len, dx / len]
            })
            .collect();
        for (i, p) in stroke.iter().enumerate() {
            let prev = normals[i.saturating_sub(1).min(normals.len() - 1)];
            let next = normals[i.min(normals.len() - 1)];
            let sum = [prev[0] + next[0], prev[1] + next[1]];
            let scale = width / 2. / (1. + prev[0] * next[0] + prev[1] * next[1]).max(0.5);
            let offset = [sum[0] * scale, sum[1] * scale];
            points.extend([
                [p[0] + offset[0], p[1] + offset[1]],
                [p[0] - offset[0], p[1] - offset[1]],
            ]);
            if i > 0 {
                let n = start + i as u32 * 2;
                indices.extend([n - 2, n - 1, n, n - 1, n + 1, n]);
            }
        }
    }
    Primitive::Mesh {
        points,
        indices,
        fill: color,
    }
}

impl Scene {
    pub(super) fn rectangle_control(&mut self, layer: &Layer, sketch: bool) {
        let b = layer.control.bounds;
        let clip = clip_with(layer.clip, b);
        let border = match layer.properties.get("border") {
            Some(Value::Choice(s)) => s.as_str(),
            _ => "solid",
        };
        let shape = match layer.properties.get("shape") {
            Some(Value::Choice(s)) => s.as_str(),
            _ => "rectangle",
        };
        let dashed = matches!(border, "dashed" | "dotted") || shape == "dotted_rectangle";
        let rounded = border == "rounded" || shape == "rounded_rectangle";
        let radius = if dashed {
            if sketch { 12. } else { 8. }
        } else if rounded {
            if sketch { 18. } else { 4. }
        } else {
            0.
        };
        let pad: f32 = if border == "none" {
            0.
        } else if rounded && sketch {
            2.
        } else {
            1.
        };
        let inset_x = pad.min(b.w / 2.);
        let inset_y = pad.min(b.h / 2.);
        let fill = Bounds::new(
            b.x + inset_x,
            b.y + inset_y,
            (b.w - 2. * inset_x).max(0.),
            (b.h - 2. * inset_y).max(0.),
        );
        if fill.w > 0. && fill.h > 0. {
            let drawing = if rounded && sketch {
                let Primitive::Mesh { mut points, .. } =
                    self::sketch(b, Profile::Rounded, layer.control.fill)
                else {
                    unreachable!()
                };
                points.truncate(points.len() / 2);
                Primitive::Polygon {
                    points,
                    fill: layer.control.fill,
                }
            } else {
                Primitive::Rect {
                    b: fill,
                    fill: layer.control.fill,
                    stroke: false,
                    radius,
                }
            };
            self.primitive(drawing, clip);
            self.items.last_mut().unwrap().opacity =
                number(&layer.properties, "background_opacity", 1.);
        }
        if border != "none" {
            let color = rgb(
                &layer.properties,
                "border_color",
                if sketch { [0; 3] } else { [51; 3] },
            );
            if dashed {
                let width = if sketch { 3. } else { 1. };
                let inset = (width / 2.0_f32).min(b.w / 2.).min(b.h / 2.);
                let p = contour(
                    Bounds::new(
                        b.x + inset,
                        b.y + inset,
                        (b.w - 2. * inset).max(0.),
                        (b.h - 2. * inset).max(0.),
                    ),
                    radius,
                    radius,
                    radius,
                    0.,
                );
                self.primitive(dash_mesh(&p, width, color), clip);
            } else if sketch {
                self.primitive(
                    self::sketch(
                        b,
                        if rounded {
                            Profile::Rounded
                        } else {
                            Profile::Square
                        },
                        color,
                    ),
                    clip,
                );
            } else if rounded {
                let r = 5.5_f32.min(b.w / 2.).min(b.h / 2.);
                let outer = contour(b, r, r, r, 0.);
                let ix = 1_f32.min(b.w / 2.);
                let iy = 1_f32.min(b.h / 2.);
                let inner = contour(
                    Bounds::new(b.x + ix, b.y + iy, b.w - 2. * ix, b.h - 2. * iy),
                    (r - 1.).max(0.),
                    (r - 1.).max(0.),
                    (r - 1.).max(0.),
                    0.,
                );
                self.primitive(ring(outer, inner, color), clip);
            } else {
                self.rectangle_frame(b, color, 1., clip);
            }
        }
        // Retain stored legacy rectangle captions while their text support is
        // migrated separately; outlining must not silently drop content.
        for p in crate::scene::control_scene(&layer.control, sketch) {
            if matches!(p, Primitive::Text { .. }) {
                self.primitive(p, layer.clip);
            }
        }
    }
}
