// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Smartphone geometry and content visibility, independent of frame artwork.
use crate::{
    document::{Properties, Value},
    layout::TextMetrics,
    model::Bounds,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub landscape: bool,
    pub stretch: bool,
    pub transparent: bool,
    pub status: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            landscape: false,
            stretch: false,
            transparent: false,
            status: true,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Fill {
    pub bounds: Bounds,
    pub color: [u8; 3],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusKind {
    Time,
    Battery,
    Signal,
    Wifi,
}
#[derive(Clone, Copy, Debug)]
pub struct StatusItem {
    pub kind: StatusKind,
    pub bounds: Bounds,
    pub size: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailKind {
    Bottom,
    Speaker,
}
#[derive(Clone, Copy, Debug)]
pub struct Detail {
    pub kind: DetailKind,
    pub bounds: Bounds,
}
#[derive(Debug)]
pub struct Layout {
    pub frame: Bounds,
    pub zoom: f32,
    pub screen: Bounds,
    pub fills: Vec<Fill>,
    pub status_items: Vec<StatusItem>,
    pub details: Vec<Detail>,
    /// Interior left, top, right, bottom coordinates. Stretch has outer guides only.
    pub guides: Option<[f32; 4]>,
}
impl Options {
    pub fn from_properties(p: &Properties) -> Self {
        Self {
            landscape: p.get("orientation") == Some(&Value::Choice("landscape".into())),
            stretch: p.get("device_resizing") == Some(&Value::Choice("stretch".into())),
            transparent: p.get("transparent_background") == Some(&Value::Bool(true)),
            status: p.get("status_bar") != Some(&Value::Bool(false)),
        }
    }
    pub fn native(self) -> (f32, f32) {
        if self.landscape {
            (725., 356.)
        } else {
            (356., 725.)
        }
    }
    pub fn sections(sketch: bool) -> [f32; 4] {
        if sketch {
            [86., 43., 38., 85.]
        } else {
            [83., 37., 38., 85.]
        }
    }
    pub fn frame(self, w: f32, h: f32) -> Bounds {
        if self.stretch {
            return Bounds::new(0., 0., w, h);
        }
        let (nw, nh) = self.native();
        let scale = (w / nw).min(h / nh);
        Bounds::new(
            (w - nw * scale) / 2.,
            (h - nh * scale) / 2.,
            nw * scale,
            nh * scale,
        )
    }
    pub fn layout(
        self,
        w: f32,
        h: f32,
        sketch: bool,
        measure_time: impl FnOnce(f32) -> TextMetrics,
    ) -> Layout {
        let frame = self.frame(w, h);
        let zoom = if self.stretch {
            1.
        } else {
            frame.w / self.native().0
        };
        let padding = if sketch { 4. } else { 0. };
        let bar_h = (16. + padding) * zoom;
        let (left, top, right, bottom, small) = if self.stretch {
            let (side, end) = if self.landscape { (h, w) } else { (w, h) };
            let narrow = side <= 70.;
            let short = end <= 167.;
            let s = if narrow { 1. } else { 19. };
            if self.landscape {
                (
                    if short { 25. } else { 79. },
                    s,
                    if short { 25. } else { 80. },
                    s,
                    narrow || short,
                )
            } else {
                (
                    s,
                    if short { 25. } else { 79. + padding },
                    s,
                    if short { 25. } else { 80. },
                    narrow || short,
                )
            }
        } else {
            let (l, t, r, b) = if self.landscape {
                (79., 19., 80., 19.)
            } else {
                (19., 79. + padding, 19., 80.)
            };
            (
                frame.x + l * zoom,
                frame.y + t * zoom,
                w - frame.x - frame.w + r * zoom,
                h - frame.y - frame.h + b * zoom,
                false,
            )
        };
        let content_top = top + if self.status { bar_h } else { 0. };
        let screen = Bounds::new(left, top, w - left - right, h - top - bottom);
        let mut fills = Vec::new();
        if !self.transparent {
            let fill = if self.stretch {
                Bounds::new(left, top, screen.w.max(1.), screen.h.max(1.))
            } else {
                Bounds::new(left, content_top, screen.w, h - content_top - bottom)
            };
            fills.push(Fill {
                bounds: fill,
                color: [255; 3],
            });
        }
        let mut status_items = Vec::new();
        if self.status && !small {
            fills.push(Fill {
                bounds: Bounds::new(left, top, screen.w, bar_h),
                color: [204; 3],
            });
            let font = 12_f32.min(bar_h);
            let metrics = measure_time(font);
            let mut x = w - right - 5. - metrics.width;
            if !self.stretch || x > left {
                status_items.push(StatusItem {
                    kind: StatusKind::Time,
                    bounds: Bounds::new(
                        x,
                        top + (bar_h - metrics.height) / 2. + 1.,
                        metrics.width,
                        metrics.height,
                    ),
                    size: font,
                });
                let size = bar_h - 4.;
                if size > 0. {
                    for (kind, gap, extra) in [
                        (StatusKind::Battery, 2., 0.),
                        (StatusKind::Signal, 3., 0.),
                        (StatusKind::Wifi, 4., 2.),
                    ] {
                        x -= size + gap;
                        if self.stretch && x <= left {
                            break;
                        }
                        status_items.push(StatusItem {
                            kind,
                            bounds: Bounds::new(
                                x,
                                top + (bar_h - size - extra) / 2.,
                                size + extra,
                                size + extra,
                            ),
                            size: size + extra,
                        });
                    }
                }
            }
        }
        let mut details = Vec::new();
        let button_h = if sketch { 16. } else { 13. };
        if self.stretch && !small {
            if self.landscape {
                if screen.h > 131. && left > button_h {
                    details.push(Detail {
                        kind: DetailKind::Bottom,
                        bounds: Bounds::new(
                            w - (right + button_h) / 2. + 1.,
                            (h - 131.) / 2.,
                            button_h,
                            131.,
                        ),
                    });
                }
                if screen.h - 22. > 177. && right > 14. {
                    details.push(Detail {
                        kind: DetailKind::Speaker,
                        bounds: Bounds::new(
                            (left - 14.) / 2. - 3.,
                            (h - 177.) / 2. + 22.,
                            14.,
                            177.,
                        ),
                    });
                }
            } else {
                if screen.w > 131. && bottom > button_h {
                    details.push(Detail {
                        kind: DetailKind::Bottom,
                        bounds: Bounds::new(
                            (w - 131.) / 2. + 1.,
                            h - (bottom + button_h) / 2. + 1.,
                            131.,
                            button_h,
                        ),
                    });
                }
                if screen.w - 21. > 177. && top > 14. {
                    details.push(Detail {
                        kind: DetailKind::Speaker,
                        bounds: Bounds::new(
                            (w - 177.) / 2. - 21.,
                            (top - 14.) / 2. - 3.,
                            177.,
                            14.,
                        ),
                    });
                }
            }
        }
        Layout {
            frame,
            zoom,
            screen,
            fills,
            status_items,
            details,
            guides: (!self.stretch).then_some([left, content_top, w - right, h - bottom]),
        }
    }
}
