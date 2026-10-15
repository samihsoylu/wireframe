// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Geometry for static media placeholders. They do not control system devices.
use crate::model::{Bounds, Kind};
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
pub struct Layout {
    pub frame: Bounds,
    pub artwork_scale: [f32; 2],
    pub volume_left: f32,
    pub volume_middle: f32,
    pub volume_right: f32,
    pub ink: [u8; 3],
    pub background: [u8; 3],
}
pub fn layout(kind: Kind, width: f32, height: f32, sketch: bool) -> Layout {
    let (w, h) = if kind == Kind::Playback {
        (110., 36.)
    } else if kind == Kind::VolumeSlider {
        (width, 16.)
    } else {
        (width, height)
    };
    let compression = (w / 54.).min(1.);
    Layout {
        frame: Bounds::new(0., 0., w, h),
        artwork_scale: if kind == Kind::Webcam {
            [w / 177., h / 146.]
        } else if kind == Kind::VolumeSlider {
            [compression, 1.]
        } else {
            [1., 1.]
        },
        volume_left: if kind == Kind::VolumeSlider {
            52. * compression
        } else {
            0.
        },
        volume_middle: if kind == Kind::VolumeSlider {
            (w - 54.).max(0.)
        } else {
            0.
        },
        volume_right: if kind == Kind::VolumeSlider {
            2. * compression
        } else {
            0.
        },
        ink: if sketch { [0; 3] } else { [102; 3] },
        background: if sketch {
            [255; 3]
        } else if kind == Kind::Webcam {
            [103; 3]
        } else {
            [221; 3]
        },
    }
}
/// Preserve the opposite edge (or center for a side handle), with a ten-unit UI minimum.
/// The stored dimensions are not clamped on load or when merely selecting a control.
pub fn resize_placeholder(b: Bounds, handle: (i8, i8), delta: (f32, f32), aspect: bool) -> Bounds {
    let proposed = b.resized(handle, delta, aspect);
    let (mut w, mut h) = (proposed.w, proposed.h);
    if aspect {
        let scale = (w / b.w).max(10. / b.w).max(10. / b.h);
        w = b.w * scale;
        h = b.h * scale;
    } else {
        if handle.0 != 0 {
            w = w.max(10.);
        }
        if handle.1 != 0 {
            h = h.max(10.);
        }
    }
    let anchor = |origin: f32, old: f32, new: f32, end: i8| {
        if end < 0 {
            origin + old - new
        } else if end == 0 {
            origin + (old - new) / 2.
        } else {
            origin
        }
    };
    Bounds::new(
        anchor(b.x, b.w, w, handle.0),
        anchor(b.y, b.h, h, handle.1),
        w,
        h,
    )
}
