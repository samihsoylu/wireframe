// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

use crate::model::Bounds;
use serde::{Deserialize, Serialize};

pub const MIN_ZOOM: f32 = 0.1;
pub const MAX_ZOOM: f32 = 3.;
pub const ZOOM_STEPS: [f32; 11] = [0.1, 0.25, 0.33, 0.5, 0.66, 0.75, 1., 1.5, 2., 2.5, 3.];

/// The next strict stop, including when Fit or a gesture left a fractional scale.
pub fn next_zoom(zoom: f32, increase: bool) -> Option<f32> {
    if !zoom.is_finite() {
        return None;
    }
    if increase {
        ZOOM_STEPS.into_iter().find(|step| *step > zoom)
    } else {
        ZOOM_STEPS.into_iter().rev().find(|step| *step < zoom)
    }
}

/// Local view preferences, independent of document content and history.
/// Scroll is in logical screen points; a renderer's translation is its negative.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct View {
    pub zoom: f32,
    pub scroll: [f32; 2],
}

impl Default for View {
    fn default() -> Self {
        Self {
            zoom: 1.,
            scroll: [0.; 2],
        }
    }
}

fn rounded_scroll(value: f32) -> f32 {
    (value + 0.5).floor().max(0.)
}

impl View {
    pub fn is_valid(self) -> bool {
        self.zoom.is_finite()
            && (MIN_ZOOM..=MAX_ZOOM).contains(&self.zoom)
            && self.scroll.into_iter().all(|v| v.is_finite())
    }

    pub fn point_at(self, screen: [f32; 2]) -> [f32; 2] {
        std::array::from_fn(|axis| (screen[axis] + self.scroll[axis]) / self.zoom)
    }

    /// Center the selection, or preserve the old visible center when unselected.
    pub fn centered(self, zoom: f32, size: [f32; 2], selection: Option<Bounds>) -> Self {
        let center = selection.map_or_else(
            || self.point_at(size.map(|v| v / 2.)),
            |b| [b.x + b.w / 2., b.y + b.h / 2.],
        );
        let zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        Self {
            zoom,
            scroll: std::array::from_fn(|axis| {
                rounded_scroll(center[axis] * zoom - size[axis] / 2.)
            }),
        }
    }

    pub fn fit(bounds: Bounds, size: [f32; 2]) -> Self {
        if ![bounds.x, bounds.y, bounds.w, bounds.h, size[0], size[1]]
            .into_iter()
            .all(f32::is_finite)
            || bounds.w <= 0.
            || bounds.h <= 0.
            || size[0] <= 0.
            || size[1] <= 0.
        {
            return Self::default();
        }
        let zoom = (size[0] / (bounds.w + 20.))
            .min(size[1] / (bounds.h + 20.))
            .clamp(MIN_ZOOM, MAX_ZOOM);
        Self {
            zoom,
            scroll: [bounds.x, bounds.y].map(|v| rounded_scroll((v - 10.) * zoom)),
        }
    }

    /// Preserve the model point beneath the pointer, subject to origin rounding.
    pub fn anchored(self, zoom: f32, pointer: [f32; 2]) -> Self {
        let point = self.point_at(pointer);
        Self {
            zoom,
            scroll: std::array::from_fn(|axis| {
                rounded_scroll(self.scroll[axis] + point[axis] * (zoom - self.zoom))
            }),
        }
    }
}

/// Retain the unsnapped scale within a continuous gesture so it can leave 100%.
#[derive(Default)]
pub struct Gesture {
    accumulator: f64,
    last_event: Option<f64>,
}

impl Gesture {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Signed native magnification, not an already transformed zoom factor.
    pub fn apply(&mut self, view: View, delta: f64, time: f64, pointer: [f32; 2]) -> View {
        if !view.is_valid()
            || !delta.is_finite()
            || delta == 0.
            || !time.is_finite()
            || !pointer.into_iter().all(f32::is_finite)
        {
            return view;
        }
        if self
            .last_event
            .is_none_or(|last| time < last || time - last > 0.5)
        {
            self.accumulator = f64::from(view.zoom);
        }
        self.last_event = Some(time);
        let factor = if delta > 0. {
            1. + delta
        } else {
            1. / (1. - delta)
        };
        self.accumulator =
            (self.accumulator * factor).clamp(f64::from(MIN_ZOOM), f64::from(MAX_ZOOM));
        let zoom = if (0.85..=1.15).contains(&self.accumulator) {
            1.
        } else {
            self.accumulator as f32
        };
        view.anchored(zoom, pointer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_steps_cover_fractional_scales_and_endpoints() {
        for pair in ZOOM_STEPS.windows(2) {
            assert_eq!(next_zoom(pair[0], true), Some(pair[1]));
            assert_eq!(next_zoom(pair[1], false), Some(pair[0]));
            let middle = (pair[0] + pair[1]) / 2.;
            assert_eq!(next_zoom(middle, true), Some(pair[1]));
            assert_eq!(next_zoom(middle, false), Some(pair[0]));
        }
        assert_eq!(next_zoom(MIN_ZOOM, false), None);
        assert_eq!(next_zoom(MAX_ZOOM, true), None);
        assert_eq!(next_zoom(f32::NAN, true), None);
    }

    #[test]
    fn selection_and_visible_center_have_distinct_anchors() {
        let v = View::default();
        let size = [1062., 890.];
        let button = Bounds {
            x: 65.,
            y: 375.,
            w: 210.,
            h: 44.,
        };
        assert_eq!(v.centered(1.5, size, Some(button)).scroll, [0., 151.]);
        let unselected = v.centered(1.5, size, None);
        assert_eq!(unselected.scroll, [266., 223.]);
        assert_eq!(unselected.centered(1., size, None), v);
    }

    #[test]
    fn fit_keeps_document_padding_and_allows_large_and_small_scales() {
        let b = Bounds {
            x: 20.,
            y: 20.,
            w: 960.,
            h: 700.,
        };
        let v = View::fit(b, [1062., 890.]);
        assert!((v.zoom - 1.0836735).abs() < 0.000001);
        assert_eq!(v.scroll, [11., 11.]);
        assert_eq!(
            View::fit(
                Bounds {
                    w: 10.,
                    h: 10.,
                    ..b
                },
                [800., 600.]
            )
            .zoom,
            3.
        );
        assert_eq!(
            View::fit(
                Bounds {
                    w: 100_000.,
                    h: 100_000.,
                    ..b
                },
                [800., 600.]
            )
            .zoom,
            0.1
        );
        for width in [0., -1., f32::NAN, f32::INFINITY] {
            assert_eq!(
                View::fit(Bounds { w: width, ..b }, [800., 600.]),
                View::default()
            );
        }
    }

    #[test]
    fn gestures_cross_snap_range_and_keep_pointer_with_rounding_tolerance() {
        let mut gesture = Gesture::default();
        let pointer = [400., 300.];
        let mut view = View {
            zoom: 1.,
            scroll: [200., 100.],
        };
        let point = view.point_at(pointer);
        for time in [0., 0.1] {
            view = gesture.apply(view, 0.06, time, pointer);
            assert_eq!(view.zoom, 1.);
        }
        view = gesture.apply(view, 0.06, 0.2, pointer);
        assert!((view.zoom - 1.191016).abs() < 0.000001);
        for (actual, expected) in view.point_at(pointer).into_iter().zip(point) {
            assert!((actual - expected).abs() <= 0.5 / view.zoom);
        }
        let mut inverse = Gesture::default();
        let restored = inverse.apply(view, -0.191016, 1., pointer);
        assert_eq!(restored.zoom, 1.);
        assert_eq!(restored.scroll, [200., 100.]);
    }

    #[test]
    fn idle_gap_resets_accumulator_and_invalid_events_do_not_extend_it() {
        let mut gesture = Gesture::default();
        let v = View::default();
        assert_eq!(gesture.apply(v, 0.1, 0., [0.; 2]), v);
        assert_eq!(gesture.apply(v, 0., 0.4, [0.; 2]), v);
        assert_eq!(gesture.apply(v, f64::NAN, 0.4, [0.; 2]), v);
        assert_eq!(gesture.apply(v, 0.1, 0.501, [0.; 2]), v);
        // Exactly half a second remains part of the same gesture.
        assert!((gesture.apply(v, 0.1, 1.001, [0.; 2]).zoom - 1.21).abs() < 0.000001);
        gesture.reset();
        assert_eq!(gesture.apply(v, 100., 2., [0.; 2]).zoom, MAX_ZOOM);
        assert_eq!(gesture.apply(v, -10000., 3., [0.; 2]).zoom, MIN_ZOOM);
    }
}
