// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Arrow geometry shared by path rendering, labels and interactive handles.
use crate::{
    document::{Dimension, Node, NodeContent, Point, Properties, Size, Value},
    model::{Bounds, round_coordinate},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Curve,
    Lines,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Handle {
    Start,
    Middle,
    End,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arrow {
    pub start: Point,
    pub end: Point,
    /// Coordinates along the endpoint chord and its clockwise normal.
    pub middle_relative: Point,
    pub mode: Mode,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub origin: Point,
    pub width: f32,
    pub height: f32,
    pub arrow: Arrow,
}
impl Placement {
    pub fn apply(self, node: &mut Node) {
        node.origin = self.origin;
        if let NodeContent::Control { size, .. } = &mut node.content {
            *size = Size::fixed(self.width, self.height);
        }
        self.arrow.write(&mut node.properties);
    }
}

pub fn node_geometry(node: &Node) -> Option<(Arrow, Point)> {
    let NodeContent::Control { kind, size } = &node.content else {
        return None;
    };
    if kind != "arrow" {
        return None;
    }
    let width = match size.width {
        Dimension::Fixed(v) => v,
        Dimension::Auto => 150.,
    };
    let height = match size.height {
        Dimension::Fixed(v) => v,
        Dimension::Auto => 100.,
    };
    Some((
        Arrow::from_properties(&node.properties, width, height),
        Point::new(width, height),
    ))
}
fn add(a: Point, b: Point) -> Point {
    Point::new(a.x + b.x, a.y + b.y)
}
fn sub(a: Point, b: Point) -> Point {
    Point::new(a.x - b.x, a.y - b.y)
}
fn scale(a: Point, s: f32) -> Point {
    Point::new(a.x * s, a.y * s)
}
fn length(a: Point) -> f32 {
    a.x.hypot(a.y)
}
fn interpolate(a: Point, b: Point, t: f32) -> Point {
    add(a, scale(sub(b, a), t))
}
fn quadratic(a: Point, c: Point, b: Point, t: f32) -> Point {
    add(
        add(scale(a, (1. - t) * (1. - t)), scale(c, 2. * t * (1. - t))),
        scale(b, t * t),
    )
}
pub fn relative_to(start: Point, end: Point, point: Point) -> Option<Point> {
    let chord = sub(end, start);
    let d = sub(point, start);
    let squared = chord.x * chord.x + chord.y * chord.y;
    if squared == 0. {
        return None;
    }
    Some(Point::new(
        (chord.x * d.x + chord.y * d.y) / squared,
        (chord.y * d.x - chord.x * d.y) / squared,
    ))
}
pub fn from_relative(start: Point, end: Point, relative: Point) -> Point {
    let chord = sub(end, start);
    add(
        interpolate(start, end, relative.x),
        scale(Point::new(chord.y, -chord.x), relative.y),
    )
}
impl Arrow {
    pub fn display_offset(self, properties: &Properties) -> Point {
        if properties.get("legacy_horizontal") == Some(&Value::Bool(true)) {
            Point::new(0., 0.)
        } else {
            let b = self.bounds();
            Point::new(b.x, b.y)
        }
    }
    /// Subdivide by geometric flatness rather than document size, keeping small
    /// curves smooth and large almost-straight curves inexpensive.
    pub fn path(self) -> Vec<Point> {
        if self.mode == Mode::Lines {
            return vec![self.start, self.middle(), self.end];
        }
        fn split(a: Point, c: Point, b: Point, depth: u8, out: &mut Vec<Point>) {
            let mid = interpolate(a, b, 0.5);
            if depth >= 12 || length(sub(c, mid)) <= 0.2 {
                out.push(b);
                return;
            }
            let ac = interpolate(a, c, 0.5);
            let cb = interpolate(c, b, 0.5);
            let center = interpolate(ac, cb, 0.5);
            split(a, ac, center, depth + 1, out);
            split(center, cb, b, depth + 1, out);
        }
        let mut points = vec![self.start];
        split(self.start, self.control(), self.end, 0, &mut points);
        points
    }
    pub fn distance_to(self, point: Point) -> f32 {
        self.path()
            .windows(2)
            .map(|pair| {
                let delta = sub(pair[1], pair[0]);
                let squared = delta.x * delta.x + delta.y * delta.y;
                let t = if squared > 0. {
                    let relative = sub(point, pair[0]);
                    ((relative.x * delta.x + relative.y * delta.y) / squared).clamp(0., 1.)
                } else {
                    0.
                };
                length(sub(point, interpolate(pair[0], pair[1], t)))
            })
            .fold(f32::INFINITY, f32::min)
    }
    pub fn from_properties(properties: &Properties, width: f32, height: f32) -> Self {
        let legacy = properties.get("legacy_horizontal") == Some(&Value::Bool(true));
        let read = |key: &str, default: f32| match properties.get(key) {
            Some(Value::Number(n)) if n.is_finite() => *n as f32,
            _ => default,
        };
        Self {
            start: Point::new(
                read("start_x", 0.),
                read("start_y", if legacy { height / 2. } else { 0. }),
            ),
            end: Point::new(
                read("end_x", width),
                read("end_y", if legacy { height / 2. } else { height }),
            ),
            middle_relative: Point::new(read("bend_along", 0.5), read("bend_across", 0.)),
            mode: if properties.get("shape") == Some(&Value::Choice("lines".into())) {
                Mode::Lines
            } else {
                Mode::Curve
            },
        }
    }
    pub fn write(self, properties: &mut Properties) {
        for (key, value) in [
            ("start_x", self.start.x),
            ("start_y", self.start.y),
            ("end_x", self.end.x),
            ("end_y", self.end.y),
            ("bend_along", self.middle_relative.x),
            ("bend_across", self.middle_relative.y),
        ] {
            properties.insert(key.into(), Value::Number(f64::from(value)));
        }
        properties.insert(
            "shape".into(),
            Value::Choice(
                if self.mode == Mode::Curve {
                    "curve"
                } else {
                    "lines"
                }
                .into(),
            ),
        );
    }
    pub fn straight(width: f32, height: f32) -> Self {
        Self {
            start: Point::new(0., 0.),
            end: Point::new(width, height),
            middle_relative: Point::new(0.5, 0.),
            mode: Mode::Curve,
        }
    }
    pub fn middle(self) -> Point {
        from_relative(self.start, self.end, self.middle_relative)
    }
    pub fn control(self) -> Point {
        self.control_through(self.middle())
    }
    fn control_through(self, middle: Point) -> Point {
        if middle == self.start || middle == self.end {
            interpolate(self.start, self.end, 0.5)
        } else {
            sub(scale(middle, 2.), scale(add(self.start, self.end), 0.5))
        }
    }
    pub fn at(self, t: f32) -> Point {
        let t = t.clamp(0., 1.);
        if self.mode == Mode::Curve {
            return quadratic(self.start, self.control(), self.end, t);
        }
        let middle = self.middle();
        let first = length(sub(middle, self.start));
        let second = length(sub(self.end, middle));
        let total = first + second;
        if total == 0. {
            return self.start;
        }
        let distance = t * total;
        if first > 0. && distance <= first {
            interpolate(self.start, middle, distance / first)
        } else if second > 0. {
            interpolate(middle, self.end, (distance - first) / second)
        } else {
            self.end
        }
    }
    pub fn bounds(self) -> Bounds {
        self.bounds_through(self.middle())
    }
    fn bounds_through(self, middle: Point) -> Bounds {
        let mut min = Point::new(self.start.x.min(self.end.x), self.start.y.min(self.end.y));
        let mut max = Point::new(self.start.x.max(self.end.x), self.start.y.max(self.end.y));
        let mut include = |p: Point| {
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            max.x = max.x.max(p.x);
            max.y = max.y.max(p.y);
        };
        if self.mode == Mode::Lines {
            include(middle);
        } else {
            let c = self.control_through(middle);
            for (a, c_axis, b) in [
                (self.start.x, c.x, self.end.x),
                (self.start.y, c.y, self.end.y),
            ] {
                let divisor = a - 2. * c_axis + b;
                if divisor != 0. {
                    let t = (a - c_axis) / divisor;
                    if t > 0. && t < 1. {
                        include(quadratic(self.start, c, self.end, t));
                    }
                }
            }
        }
        Bounds::new(min.x, min.y, max.x - min.x, max.y - min.y)
    }
    /// Endpoint movement retains the middle's relation to the chord; middle
    /// movement changes it. Rounded rebasing keeps the stationary endpoint fixed.
    pub fn move_handle(
        mut self,
        origin: Point,
        handle: Handle,
        target: Point,
        constrain: bool,
    ) -> Placement {
        let target = if constrain && handle != Handle::Middle {
            snap_endpoint(
                if handle == Handle::Start {
                    self.end
                } else {
                    self.start
                },
                target,
            )
        } else {
            target
        };
        let mut through = None;
        match handle {
            Handle::Start => self.start = target,
            Handle::End => self.end = target,
            Handle::Middle => {
                if let Some(relative) = relative_to(self.start, self.end, target) {
                    self.middle_relative = relative;
                    through = Some(target);
                }
            }
        }
        // Keep the pointer's exact position for rounding. Converting it to the
        // chord basis and back can cross a half-unit boundary in floating point.
        let bounds = self.bounds_through(through.unwrap_or_else(|| self.middle()));
        self.rebase_from_bounds(origin, bounds, true)
    }
    fn rebase(self, origin: Point, rounded: bool) -> Placement {
        let bounds = self.bounds();
        self.rebase_from_bounds(origin, bounds, rounded)
    }
    fn rebase_from_bounds(mut self, origin: Point, bounds: Bounds, rounded: bool) -> Placement {
        let delta = if rounded {
            Point::new(round_coordinate(bounds.x), round_coordinate(bounds.y))
        } else {
            Point::new(bounds.x, bounds.y)
        };
        self.start = sub(self.start, delta);
        self.end = sub(self.end, delta);
        let bounds = self.bounds();
        Placement {
            origin: add(origin, delta),
            width: bounds.w.max(1.),
            height: bounds.h.max(1.),
            arrow: self,
        }
    }
    pub fn change_mode(mut self, origin: Point, mode: Mode) -> Placement {
        self.mode = mode;
        self.rebase(origin, false)
    }
    pub fn resize(self, old_size: Point, new_bounds: Bounds) -> Placement {
        let width = new_bounds.w.max(1.);
        let height = new_bounds.h.max(1.);
        let x = width / old_size.x.max(1.);
        let y = height / old_size.y.max(1.);
        let stretched = |p: Point| Point::new(p.x * x, p.y * y);
        let rounded = |p: Point| {
            let p = stretched(p);
            Point::new(round_coordinate(p.x), round_coordinate(p.y))
        };
        let start = rounded(self.start);
        let end = rounded(self.end);
        let middle = if self.mode == Mode::Curve {
            quadratic(start, stretched(self.control()), end, 0.5)
        } else {
            stretched(self.middle())
        };
        Placement {
            origin: Point::new(new_bounds.x, new_bounds.y),
            width,
            height,
            arrow: Self {
                start,
                end,
                middle_relative: relative_to(start, end, middle).unwrap_or(self.middle_relative),
                mode: self.mode,
            },
        }
    }
    pub fn snap_middle(self, origin: Point) -> Placement {
        let center = interpolate(self.start, self.end, 0.5);
        let target = if self.mode == Mode::Curve {
            center
        } else {
            let middle = self.middle();
            [
                Point::new(self.start.x, self.end.y),
                Point::new(self.end.x, self.start.y),
                center,
            ]
            .into_iter()
            .min_by(|a, b| length(sub(*a, middle)).total_cmp(&length(sub(*b, middle))))
            .unwrap()
        };
        self.move_handle(origin, Handle::Middle, target, false)
    }
}
fn snap_endpoint(center: Point, target: Point) -> Point {
    let d = sub(target, center);
    let x = d.x.abs();
    let y = d.y.abs();
    let threshold = (std::f32::consts::PI / 8.).tan();
    if y < x * threshold {
        Point::new(target.x, center.y)
    } else if x < y * threshold {
        Point::new(center.x, target.y)
    } else {
        let size = x.min(y);
        add(center, Point::new(d.x.signum() * size, d.y.signum() * size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn close(a: Point, b: Point) {
        assert!(
            (a.x - b.x).abs() < 0.001 && (a.y - b.y).abs() < 0.001,
            "{a:?} / {b:?}"
        );
    }
    #[test]
    fn middle_handle_half_unit_does_not_drift_during_rebasing() {
        let a = Arrow {
            start: Point::new(-20.5, 3.5),
            end: Point::new(80.5, 92.5),
            middle_relative: Point::new(-0.2, 0.45),
            mode: Mode::Lines,
        };
        let placed = a.move_handle(
            Point::new(320., 240.),
            Handle::Middle,
            Point::new(0.5, -0.5),
            false,
        );
        assert_eq!(placed.origin, Point::new(300., 240.));
        close(placed.arrow.middle(), Point::new(20.5, -0.5));
    }
    #[test]
    fn relative_middle_round_trips_across_chord_directions() {
        for end in [
            Point::new(100., 0.),
            Point::new(0., -80.),
            Point::new(-40., 75.),
        ] {
            for r in [
                Point::new(0.5, 0.),
                Point::new(-0.2, 0.4),
                Point::new(1.3, -0.5),
            ] {
                let start = Point::new(5., -3.);
                close(
                    relative_to(start, end, from_relative(start, end, r)).unwrap(),
                    r,
                );
            }
        }
    }
    #[test]
    fn curve_passes_through_middle_and_bounds_contain_entire_path() {
        let mut a = Arrow::straight(120., 80.);
        a.middle_relative = Point::new(0.3, 0.7);
        close(a.at(0.), a.start);
        close(a.at(1.), a.end);
        close(a.at(0.5), a.middle());
        let b = a.bounds();
        for i in 0..=1000 {
            let p = a.at(i as f32 / 1000.);
            assert!(
                p.x >= b.x - 0.001
                    && p.x <= b.x + b.w + 0.001
                    && p.y >= b.y - 0.001
                    && p.y <= b.y + b.h + 0.001
            );
        }
    }
    #[test]
    fn labels_on_lines_follow_travel_distance_not_equal_leg_time() {
        let a = Arrow {
            start: Point::new(0., 0.),
            end: Point::new(30., 40.),
            middle_relative: relative_to(
                Point::new(0., 0.),
                Point::new(30., 40.),
                Point::new(30., 0.),
            )
            .unwrap(),
            mode: Mode::Lines,
        };
        close(a.at(3. / 7.), Point::new(30., 0.));
        close(a.at(0.5), Point::new(30., 5.));
    }
    #[test]
    fn moving_and_switching_modes_preserve_stationary_world_points() {
        let mut a = Arrow::straight(120., 80.);
        a.middle_relative = Point::new(0.4, 0.6);
        let origin = Point::new(250., 100.);
        let moved = a.move_handle(origin, Handle::Middle, Point::new(-20., -40.), false);
        close(add(moved.origin, moved.arrow.start), add(origin, a.start));
        close(add(moved.origin, moved.arrow.end), add(origin, a.end));
        close(
            add(moved.origin, moved.arrow.middle()),
            add(origin, Point::new(-20., -40.)),
        );
        let line = a.change_mode(origin, Mode::Lines);
        close(add(line.origin, line.arrow.start), add(origin, a.start));
        close(
            add(line.origin, line.arrow.middle()),
            add(origin, a.middle()),
        );
        let curve = line.arrow.change_mode(line.origin, Mode::Curve);
        close(
            add(curve.origin, curve.arrow.middle()),
            add(origin, a.middle()),
        );
    }
    #[test]
    fn collapse_and_resize_keep_finite_state_for_later_expansion() {
        let a = Arrow::straight(100., 100.);
        let collapsed = a.move_handle(Point::new(0., 0.), Handle::End, a.start, false);
        assert_eq!((collapsed.width, collapsed.height), (1., 1.));
        assert_eq!(collapsed.arrow.middle_relative, a.middle_relative);
        close(collapsed.arrow.at(0.5), collapsed.arrow.start);
        let expanded =
            collapsed
                .arrow
                .move_handle(collapsed.origin, Handle::End, Point::new(30., 60.), false);
        assert_eq!(expanded.arrow.middle_relative, a.middle_relative);
        let resized = a.resize(Point::new(100., 100.), Bounds::new(15., 25., 1., 200.));
        close(resized.arrow.end, Point::new(1., 200.));
    }
    #[test]
    fn shifted_endpoints_and_middle_snap_preserve_expected_directions() {
        let a = Arrow::straight(120., 80.);
        let o = Point::new(0., 0.);
        close(
            a.move_handle(o, Handle::End, Point::new(70., 49.), true)
                .arrow
                .end,
            Point::new(49., 49.),
        );
        close(
            a.move_handle(o, Handle::End, Point::new(80., 10.), true)
                .arrow
                .end,
            Point::new(80., 0.),
        );
        let mut bent = a;
        bent.middle_relative = Point::new(0.4, 0.6);
        let snapped = bent.snap_middle(o);
        close(snapped.arrow.middle_relative, Point::new(0.5, 0.));
    }
}
