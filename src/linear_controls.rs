// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Fixed-thickness controls and their shared interactive sizing constraints.
use crate::model::{Bounds, Kind};
use serde::Serialize;
#[derive(Clone, Copy, Debug)]
pub struct Axis {
    pub vertical: bool,
    pub thickness: f32,
}
impl Axis {
    pub fn for_kind(kind: Kind) -> Option<Self> {
        let (vertical, thickness) = match kind {
            Kind::HorizontalSlider | Kind::HorizontalSplitter => (false, 12.),
            Kind::VerticalSlider | Kind::VerticalSplitter => (true, 12.),
            Kind::HorizontalScrollbar | Kind::VolumeSlider => (false, 16.),
            Kind::VerticalScrollbar => (true, 16.),
            _ => return None,
        };
        Some(Self {
            vertical,
            thickness,
        })
    }
    /// UI editing minimum. Loading and rendering retain smaller saved lengths.
    pub const MIN_LENGTH: f32 = 10.;
    pub fn resize(self, bounds: Bounds, edge: i8, delta: f32) -> Bounds {
        let old = if self.vertical { bounds.h } else { bounds.w };
        let length = (old + delta * edge as f32).clamp(Self::MIN_LENGTH, 10_000.);
        if self.vertical {
            Bounds::new(
                bounds.x,
                if edge < 0 {
                    bounds.y + old - length
                } else {
                    bounds.y
                },
                self.thickness,
                length,
            )
        } else {
            Bounds::new(
                if edge < 0 {
                    bounds.x + old - length
                } else {
                    bounds.x
                },
                bounds.y,
                length,
                self.thickness,
            )
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Layout {
    pub frame: Bounds,
    pub background: Option<Bounds>,
    pub thumb: Option<Bounds>,
    pub ink: [u8; 3],
}
/// Local geometry; callers translate the complete result to the control origin.
pub fn scrollbar(vertical: bool, length: f32, value: f32, sketch: bool) -> Layout {
    let frame = if vertical {
        Bounds::new(0., 0., 16., length)
    } else {
        Bounds::new(0., 0., length, 16.)
    };
    let along = 13. + (length - 48.) * value.clamp(0., 1.);
    Layout {
        frame,
        background: Some(Bounds::new(
            1.,
            1.,
            (frame.w - 2.).max(1.),
            (frame.h - 2.).max(1.),
        )),
        thumb: (length > 45.).then(|| {
            if vertical {
                Bounds::new(4., along, 8., 21.)
            } else {
                Bounds::new(along, 4., 21., 8.)
            }
        }),
        ink: if sketch { [0; 3] } else { [51; 3] },
    }
}
pub fn splitter(vertical: bool, length: f32, sketch: bool) -> Layout {
    let frame = if vertical {
        Bounds::new(0., 0., 12., length)
    } else {
        Bounds::new(0., 0., length, 12.)
    };
    let (w, h) = if vertical {
        (4., 16.)
    } else if sketch {
        (17., 4.)
    } else {
        (16., 4.)
    };
    Layout {
        frame,
        background: None,
        thumb: Some(Bounds::new((frame.w - w) / 2., (frame.h - h) / 2., w, h)),
        ink: if sketch { [0; 3] } else { [102; 3] },
    }
}
