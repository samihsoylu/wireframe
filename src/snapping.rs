// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Deterministic alignment geometry. Guides and drag samples are transient UI data.
use crate::model::Bounds;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Edge,
    Spacing,
    Center,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Guide {
    pub axis: Axis,
    pub kind: Kind,
    pub position: f32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Constraint {
    #[default]
    Free,
    Horizontal,
    Vertical,
}
#[derive(Clone, Debug)]
pub struct Result {
    pub bounds: Bounds,
    pub guides: Vec<Guide>,
}
#[derive(Clone, Debug)]
pub struct Targets {
    guides: Vec<Guide>,
    gravity: f32,
}
fn rounded(v: f32) -> f32 {
    (v + 0.5).floor()
}
fn origin(b: Bounds, axis: Axis) -> f32 {
    if axis == Axis::X { b.x } else { b.y }
}
fn extent(b: Bounds, axis: Axis) -> f32 {
    if axis == Axis::X { b.w } else { b.h }
}
fn middle(b: Bounds, axis: Axis) -> f32 {
    rounded(origin(b, axis) + extent(b, axis) / 2.)
}
impl Targets {
    pub fn new(gravity: f32) -> Self {
        Self {
            guides: Vec::new(),
            gravity: gravity.max(0.),
        }
    }
    pub fn add(&mut self, axis: Axis, kind: Kind, position: f32) {
        if position.is_finite() {
            self.guides.push(Guide {
                axis,
                kind,
                position: rounded(position),
            });
        }
    }
    /// The order of targets is significant when attraction bands overlap.
    pub fn frame(&mut self, b: Bounds, inside: bool) {
        self.edges(b);
        let d = if inside { 16. } else { -16. };
        self.add(Axis::X, Kind::Spacing, b.x + d);
        self.add(Axis::X, Kind::Spacing, b.x + b.w - d);
        self.add(Axis::Y, Kind::Spacing, b.y + d);
        self.add(Axis::Y, Kind::Spacing, b.y + b.h - d);
    }
    pub fn edges(&mut self, b: Bounds) {
        for axis in [Axis::X, Axis::Y] {
            self.add(axis, Kind::Center, origin(b, axis) + extent(b, axis) / 2.);
            self.add(axis, Kind::Edge, origin(b, axis));
            self.add(axis, Kind::Edge, origin(b, axis) + extent(b, axis));
        }
    }
    fn find(&self, axis: Axis, kind: Kind, coordinate: f32) -> Option<Guide> {
        self.guides.iter().rev().copied().find(|g| {
            g.axis == axis && g.kind == kind && (g.position - coordinate).abs() <= self.gravity
        })
    }
    pub fn matches(&self, b: Bounds) -> Vec<Guide> {
        let mut result = Vec::new();
        for axis in [Axis::X, Axis::Y] {
            for point in [origin(b, axis), origin(b, axis) + extent(b, axis)] {
                for kind in [Kind::Edge, Kind::Spacing] {
                    if let Some(g) = self.find(axis, kind, point)
                        && g.position == point
                    {
                        result.push(g);
                    }
                }
            }
            let point = middle(b, axis);
            if let Some(g) = self.find(axis, Kind::Center, point)
                && g.position == point
            {
                result.push(g);
            }
        }
        result
    }
    /// A creation origin follows an edge/spacing guide before a center guide.
    pub fn point(&self, x: f32, y: f32) -> Result {
        let mut bounds = Bounds::new(x, y, 0., 0.);
        let mut guides = Vec::new();
        for (axis, value) in [(Axis::X, x), (Axis::Y, y)] {
            if let Some(guide) = [Kind::Edge, Kind::Spacing, Kind::Center]
                .into_iter()
                .find_map(|kind| self.find(axis, kind, rounded(value)))
            {
                if axis == Axis::X {
                    bounds.x = guide.position;
                } else {
                    bounds.y = guide.position;
                }
                guides.push(guide);
            }
        }
        Result { bounds, guides }
    }
    pub fn moved(&self, input: Bounds, constraint: Constraint) -> Result {
        let mut b = Bounds::new(
            rounded(input.x),
            rounded(input.y),
            rounded(input.w),
            rounded(input.h),
        );
        for axis in [Axis::X, Axis::Y] {
            if (axis == Axis::X && constraint == Constraint::Vertical)
                || (axis == Axis::Y && constraint == Constraint::Horizontal)
            {
                continue;
            }
            let center = self.find(axis, Kind::Center, middle(input, axis));
            let shift = if let Some(g) = center {
                Some(g.position - middle(b, axis))
            } else {
                [false, true].into_iter().find_map(|trailing| {
                    let point =
                        origin(input, axis) + if trailing { extent(input, axis) } else { 0. };
                    [Kind::Edge, Kind::Spacing]
                        .into_iter()
                        .find_map(|kind| self.find(axis, kind, rounded(point)))
                        .map(|g| {
                            g.position
                                - origin(b, axis)
                                - if trailing { extent(b, axis) } else { 0. }
                        })
                })
            };
            if let Some(shift) = shift {
                if axis == Axis::X {
                    b.x += shift;
                } else {
                    b.y += shift;
                }
            }
        }
        Result {
            bounds: b,
            guides: self.matches(b),
        }
    }
    pub fn resized(&self, input: Bounds, handle: (i8, i8), ratio: Option<f32>) -> Result {
        let mut b = input;
        for (axis, side) in [(Axis::X, handle.0), (Axis::Y, handle.1)] {
            if side == 0 {
                continue;
            }
            let trailing = side > 0;
            let point = origin(input, axis) + if trailing { extent(input, axis) } else { 0. };
            let edge = [Kind::Edge, Kind::Spacing]
                .into_iter()
                .find_map(|k| self.find(axis, k, rounded(point)));
            let chosen = edge.map(|g| (g, false)).or_else(|| {
                self.find(axis, Kind::Center, middle(input, axis))
                    .filter(|g| {
                        (g.position - middle(input, axis)).abs() <= (self.gravity / 2.).floor()
                    })
                    .map(|g| (g, true))
            });
            let Some((guide, center)) = chosen else {
                continue;
            };
            let initial = b;
            let anchor = origin(b, axis) + if trailing { 0. } else { extent(b, axis) };
            let size = (guide.position - anchor)
                * if trailing { 1. } else { -1. }
                * if center { 2. } else { 1. };
            let start = if trailing { anchor } else { anchor - size };
            if axis == Axis::X {
                b.x = start;
                b.w = size;
            } else {
                b.y = start;
                b.h = size;
            }
            if let Some(r) = ratio.filter(|r| r.is_finite() && *r > 0.) {
                if axis == Axis::X && handle.1 != 0 {
                    b.h = b.w / r;
                    if handle.1 < 0 {
                        b.y -= b.h - initial.h;
                    }
                } else if axis == Axis::Y {
                    b.w = b.h * r;
                    if handle.0 < 0 {
                        b.x -= b.w - initial.w;
                    }
                }
            }
        }
        Result {
            bounds: b,
            guides: self.matches(b),
        }
    }
}

/// Event-distance suppression; deliberately independent of wall-clock speed.
#[derive(Clone, Debug, Default)]
pub struct Motion {
    points: std::collections::VecDeque<[f32; 2]>,
}
impl Motion {
    pub fn too_fast(&self) -> bool {
        if self.points.len() < 3 {
            return false;
        }
        self.points
            .iter()
            .zip(self.points.iter().skip(1))
            .map(|(a, b)| (a[0] - b[0]).abs().max((a[1] - b[1]).abs()))
            .sum::<f32>()
            / 3.
            > 2.
    }
    pub fn push(&mut self, point: [f32; 2]) {
        if self.points.len() == 3 {
            self.points.pop_front();
        }
        self.points.push_back(point);
    }
}
