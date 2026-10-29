// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

// A preferred normal size, not a work-area guarantee or a later resize limit.
// Leave proportional room for decorations and desktop panels on a sole monitor.
const MONITOR_FRACTION: f64 = 0.9;
const PREFERRED: [f64; 2] = [1360., 900.];
const MINIMUM: [f64; 2] = [920., 620.];

#[cfg(target_os = "linux")]
pub(super) fn configure(cc: &eframe::CreationContext<'_>) {
    let Some(window) = cc.winit_window() else {
        return;
    };
    if window.is_maximized() || window.fullscreen().is_some() {
        return;
    }
    let monitors = window.available_monitors().map(|monitor| {
        let size = monitor.size();
        ([size.width, size.height], monitor.scale_factor())
    });
    if let Some([width, height]) = preferred_size(monitors, cc.egui_ctx.zoom_factor()) {
        let mut size = window.inner_size();
        size.width = width;
        size.height = height;
        // AppCreator runs once, before Glow shows the window. X11 applies this
        // request asynchronously; no retries or later user-size enforcement.
        let _ = window.request_inner_size(size);
    }
}

fn preferred_size(
    mut monitors: impl Iterator<Item = ([u32; 2], f64)>,
    egui_zoom: f32,
) -> Option<[u32; 2]> {
    let (physical_size, monitor_scale) = monitors.next()?;
    // There is no unambiguous opening monitor with multiple/unknown displays.
    if monitors.next().is_some()
        || !monitor_scale.is_finite()
        || monitor_scale <= 0.
        || !egui_zoom.is_finite()
        || egui_zoom <= 0.
    {
        return None;
    }
    let pixels_per_point = monitor_scale * f64::from(egui_zoom);
    let room = physical_size.map(|pixels| f64::from(pixels) / pixels_per_point * MONITOR_FRACTION);
    if room
        .iter()
        .zip(MINIMUM)
        .any(|(&available, minimum)| !available.is_finite() || available < minimum)
    {
        // Preserve the app minimum. Very small displays need a separate policy.
        return None;
    }
    let points = [PREFERRED[0].min(room[0]), PREFERRED[1].min(room[1])];
    if points == PREFERRED {
        return None;
    }
    Some(points.map(|points| (points * pixels_per_point).round() as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(physical: [u32; 2], scale: f64, zoom: f32) -> Option<[u32; 2]> {
        preferred_size([(physical, scale)].into_iter(), zoom)
    }

    #[test]
    fn short_display_leaves_room_for_measured_normal_frame() {
        let requested = size([1440, 800], 1., 1.).unwrap();
        assert_eq!(requested, [1296, 720]);
        // Reviewed Openbox baseline: outer origin0,0, decorations2×27.
        // This is a policy check, not proof of a future WM request/placement.
        assert!(requested[0] + 2 <= 1440);
        assert!(requested[1] + 27 <= 800);
        assert_eq!(size([1280, 720], 1., 1.), Some([1152, 648]));
    }

    #[test]
    fn larger_displays_keep_preferred_size_and_only_constrained_axes_shrink() {
        assert_eq!(size([1920, 1080], 1., 1.), None);
        assert_eq!(size([3840, 2160], 2., 1.), None);
        assert_eq!(size([1920, 800], 1., 1.), Some([1360, 720]));
        assert_eq!(size([1440, 1200], 1., 1.), Some([1296, 900]));
    }

    #[test]
    fn actual_monitor_scale_and_egui_zoom_determine_logical_room() {
        assert_eq!(size([2880, 1600], 2., 1.), Some([2592, 1440]));
        assert_eq!(size([2160, 1200], 1.5, 1.), Some([1944, 1080]));
        assert_eq!(size([2880, 1600], 1., 2.), Some([2592, 1440]));
        assert_eq!(size([2880, 1600], 2., 0.5), None);
    }

    #[test]
    fn missing_multiple_invalid_and_too_small_monitors_keep_existing_behavior() {
        assert_eq!(preferred_size(std::iter::empty(), 1.), None);
        assert_eq!(
            preferred_size([([1440, 800], 1.), ([1920, 1080], 1.)].into_iter(), 1.),
            None
        );
        for dimensions in [[0, 800], [1440, 0], [920, 620], [1024, 680]] {
            assert_eq!(size(dimensions, 1., 1.), None);
        }
        for scale in [0., -1., f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            assert_eq!(size([1440, 800], scale, 1.), None);
        }
        for zoom in [0., -1., f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
            assert_eq!(size([1440, 800], 1., zoom), None);
        }
        assert_eq!(size([1440, 800], f64::MAX, f32::MAX), None);
        assert_eq!(
            size([1440, 800], f64::MIN_POSITIVE, f32::MIN_POSITIVE),
            None
        );
        assert_eq!(size([u32::MAX, u32::MAX], 1., 1.), None);
        // Just above the minimum-room boundary still preserves920×620.
        assert_eq!(size([1023, 689], 1., 1.), Some([921, 620]));
    }
}
