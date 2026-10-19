// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Transient draw gestures. No IDs, document changes or history until insertion.
use crate::{
    arrows::{Arrow, Mode as ArrowMode, Placement, relative_to},
    catalog,
    document::{Node, NodeContent, Point, Size, Value},
    model::Bounds,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Image,
    Rectangle,
    TextBlock,
    TextLine,
    Arrow,
}
impl Mode {
    pub fn kind(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Rectangle => "rectangle",
            Self::TextBlock => "text_block",
            Self::TextLine => "text_line",
            Self::Arrow => "arrow",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Gesture {
    pub start: Point,
    pub end: Point,
    samples: Vec<Point>,
}
impl Gesture {
    pub fn new(start: Point) -> Self {
        Self {
            start,
            end: start,
            samples: Vec::new(),
        }
    }
    /// Only motion events contribute to curvature; release is an endpoint update.
    pub fn motion(&mut self, point: Point) {
        self.end = point;
        self.samples.push(point);
    }
    pub fn bounds(&self) -> Bounds {
        let x = self.end.x.max(0.);
        let y = self.end.y.max(0.);
        Bounds::new(
            self.start.x.min(x),
            self.start.y.min(y),
            (self.start.x - x).abs(),
            (self.start.y - y).abs(),
        )
    }
    pub fn middle(&self) -> Option<Point> {
        let n = self.samples.len() as f64;
        if n < 5. {
            return None;
        }
        let (sx, sy, sxx, sxy) =
            self.samples
                .iter()
                .fold((0., 0., 0., 0.), |(sx, sy, sxx, sxy), p| {
                    let (x, y) = (f64::from(p.x), f64::from(p.y));
                    (sx + x, sy + y, sxx + x * x, sxy + x * y)
                });
        let slope = (sxy - sx * sy / n) / (sxx - sx * sx / n);
        let intercept = sy / n - slope * sx / n;
        let residual = self
            .samples
            .iter()
            .map(|p| {
                let d = f64::from(p.y) - intercept - slope * f64::from(p.x);
                d * d
            })
            .sum::<f64>()
            / (n - 2.);
        let denominator = n * sxx - sx * sx;
        if (n / denominator * residual).sqrt() < 0.01
            || (residual / denominator * sxx).sqrt() > 150.
        {
            return None;
        }
        let distances: Vec<f64> = self
            .samples
            .windows(2)
            .map(|pair| {
                let dx = f64::from(pair[1].x) - f64::from(pair[0].x);
                let dy = f64::from(pair[1].y) - f64::from(pair[0].y);
                (dx * dx + dy * dy).sqrt()
            })
            .collect();
        let half = distances.iter().sum::<f64>() / 2.;
        let mut total = 0.;
        distances
            .iter()
            .position(|d| {
                total += d;
                total >= half
            })
            .map(|i| self.samples[i])
    }
    /// Create a detached, world-space node. A caller may supply snapped bounds.
    pub fn node(&self, mode: Mode, mut bounds: Bounds) -> Node {
        let mut node = catalog::get(mode.kind())
            .unwrap()
            .create(0, Point::new(bounds.x, bounds.y));
        if mode == Mode::Arrow {
            let middle = self.middle();
            if let Some(p) = middle {
                bounds = bounds.union(Bounds::new(p.x, p.y, 0., 0.));
            }
            let forward = self.end.x > self.start.x
                || self.end.x == self.start.x && self.end.y > self.start.y;
            let local = |p: Point| Point::new(p.x - bounds.x, p.y - bounds.y);
            let (start, end) = if forward {
                (local(self.start), local(self.end))
            } else {
                (local(self.end), local(self.start))
            };
            let arrow = Arrow {
                start,
                end,
                mode: ArrowMode::Curve,
                middle_relative: middle
                    .and_then(|p| relative_to(start, end, local(p)))
                    .unwrap_or(Point::new(0.5, 0.)),
            };
            Placement {
                origin: Point::new(bounds.x, bounds.y),
                width: bounds.w.max(1.),
                height: bounds.h.max(1.),
                arrow,
            }
            .apply(&mut node);
            node.properties
                .insert("start_head".into(), Value::Bool(!forward));
            node.properties
                .insert("end_head".into(), Value::Bool(forward));
        } else if let NodeContent::Control { size, .. } = &mut node.content {
            *size = Size::fixed(bounds.w.max(1.), bounds.h.max(1.));
        }
        node
    }
}
