// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

/// Scroll requests in viewport points, independent of document zoom.
pub fn edge_delta(point: [f32; 2], size: [f32; 2]) -> [f32; 2] {
    std::array::from_fn(|axis| {
        let distance = point[axis] - (size[axis] - 10.);
        if distance > 0. {
            if distance > 90. { 4. } else { 1. }
        } else if point[axis] < 10. {
            if point[axis] < -80. { -4. } else { -1. }
        } else {
            0.
        }
    })
}

/// Available scrolling after the document's trailing workspace padding.
pub fn scroll_limits(content: [f32; 2], size: [f32; 2], zoom: f32) -> [f32; 2] {
    std::array::from_fn(|axis| {
        let maximum = 10_000. * zoom;
        let extent = (content[axis].max(0.) * zoom + 0.5).floor();
        let padding = (400_f32.min(maximum - extent) + 0.5).floor();
        let canvas = (extent + padding).max(size[axis]).min(maximum);
        let margin = (canvas - maximum + 400.).max(0.);
        (canvas + margin - size[axis]).max(0.)
    })
}

/// One timer per gesture. Content may enlarge the scrollable area while held,
/// but shrinking it must not pull the viewport back under the pointer.
#[derive(Default)]
pub struct DragScroll {
    limits: [f32; 2],
    next_tick: Option<f64>,
}

impl DragScroll {
    pub fn advance(
        &mut self,
        time: f64,
        moved: bool,
        point: [f32; 2],
        size: [f32; 2],
        limits: [f32; 2],
        scroll: [f32; 2],
    ) -> ([f32; 2], Option<f64>) {
        for (retained, current) in self.limits.iter_mut().zip(limits) {
            *retained = retained.max(current);
        }
        let requested = edge_delta(point, size);
        if requested == [0., 0.] {
            self.next_tick = None;
            return ([0., 0.], None);
        }
        if !moved
            && let Some(next) = self.next_tick
            && time < next
        {
            return ([0., 0.], Some(next - time));
        }
        self.next_tick = Some(time + 0.1);
        let delta = std::array::from_fn(|axis| {
            // A freely panned viewport may already be outside this range.
            // Permit gradual movement toward it, without a sudden clamp.
            if requested[axis] > 0. {
                requested[axis].min((self.limits[axis] - scroll[axis]).max(0.))
            } else {
                requested[axis].max((-scroll[axis]).min(0.))
            }
        });
        (delta, Some(0.1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_bands_are_strict_and_diagonal_requests_are_independent() {
        for (x, expected) in [
            (-81., -4.),
            (-80., -1.),
            (9., -1.),
            (10., 0.),
            (790., 0.),
            (791., 1.),
            (880., 1.),
            (881., 4.),
        ] {
            assert_eq!(edge_delta([x, 300.], [800., 600.]), [expected, 0.]);
        }
        assert_eq!(edge_delta([-81., 681.], [800., 600.]), [-4., 4.]);
        assert_eq!(edge_delta([9., 9.], [10., 10.]), [1., 1.]);
    }

    #[test]
    fn workspace_padding_limits_and_zoom() {
        assert_eq!(scroll_limits([0., 0.], [800., 600.], 1.), [0., 0.]);
        assert_eq!(scroll_limits([900., 700.], [800., 600.], 1.), [500., 500.]);
        assert_eq!(scroll_limits([900., 700.], [800., 600.], 0.5), [50., 150.]);
        assert_eq!(
            scroll_limits([10_100., 10_000.], [800., 600.], 1.),
            [9600., 9800.]
        );
        assert_eq!(
            scroll_limits([10_000., 10_000.], [800., 600.], 2.),
            [19600., 19800.]
        );
    }

    #[test]
    fn timed_scroll_grows_without_shrinking_or_jumping_after_free_pan() {
        let mut drag = DragScroll::default();
        let tick = |drag: &mut DragScroll, time, moved, point, limit, scroll| {
            drag.advance(time, moved, point, [800., 600.], limit, scroll)
        };
        assert_eq!(
            tick(&mut drag, 0., true, [799., 599.], [2., 3.], [0., 0.]).0,
            [1., 1.]
        );
        assert_eq!(
            tick(&mut drag, 0.05, false, [799., 599.], [2., 3.], [1., 1.]).0,
            [0., 0.]
        );
        assert_eq!(
            tick(&mut drag, 0.1, false, [799., 599.], [2., 3.], [1., 1.]).0,
            [1., 1.]
        );
        assert_eq!(
            tick(&mut drag, 0.2, true, [900., 700.], [0., 0.], [2., 2.]).0,
            [0., 1.]
        );
        assert_eq!(
            tick(&mut drag, 0.3, true, [900., 700.], [7., 9.], [2., 3.]).0,
            [4., 4.]
        );
        assert_eq!(
            tick(&mut drag, 0.4, true, [400., 300.], [0., 0.], [6., 7.]),
            ([0., 0.], None)
        );
        assert_eq!(
            tick(&mut drag, 0.5, true, [799., 599.], [0., 0.], [-50., -20.]).0,
            [1., 1.]
        );
        assert_eq!(
            tick(&mut drag, 0.6, true, [0., 0.], [0., 0.], [-50., -20.]).0,
            [0., 0.]
        );
    }
}
