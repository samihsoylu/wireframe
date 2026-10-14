// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Tablet layout separates sizing, status content and frame artwork.
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    hour: u8,
    minute: u8,
}
impl Clock {
    pub fn new(hour: u8, minute: u8) -> Option<Self> {
        (hour < 24 && minute < 60).then_some(Self { hour, minute })
    }
    pub fn now() -> Self {
        use chrono::Timelike;
        let local = chrono::Local::now();
        Self {
            hour: local.hour() as u8,
            minute: local.minute() as u8,
        }
    }
    pub fn label(self) -> String {
        let hour = match self.hour % 12 {
            0 => 12,
            h => h,
        };
        format!(
            "{hour}:{:02} {}",
            self.minute,
            if self.hour < 12 { "AM" } else { "PM" }
        )
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Fill {
    pub bounds: Bounds,
    pub color: [u8; 3],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusKind {
    Label,
    Time,
    Battery,
    Wifi,
}
#[derive(Clone, Copy, Debug)]
pub struct StatusItem {
    pub kind: StatusKind,
    pub bounds: Bounds,
    pub size: f32,
    pub color: [u8; 3],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailKind {
    Home,
    Camera,
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
            (916., 616.)
        } else {
            (616., 916.)
        }
    }
    pub fn sections(sketch: bool) -> [f32; 4] {
        if sketch {
            [96., 37., 38., 97.]
        } else {
            [97., 38., 39., 96.]
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
        measure_label: impl FnOnce(f32) -> TextMetrics,
    ) -> Layout {
        let frame = self.frame(w, h);
        let zoom = if self.stretch {
            1.
        } else {
            frame.h / self.native().1
        };
        let mut screen = if self.stretch {
            if self.landscape {
                Bounds::new(94., 32., (w - 186.).max(1.), (h - 64.).max(1.))
            } else {
                Bounds::new(32., 94., (w - 64.).max(1.), (h - 186.).max(1.))
            }
        } else {
            let (sw, sh) = if self.landscape {
                (730., 550.)
            } else {
                (550., 730.)
            };
            Bounds::new(
                frame.x + (frame.w - sw * zoom) / 2.,
                frame.y + (frame.h - sh * zoom) / 2.,
                sw * zoom,
                sh * zoom,
            )
        };
        let mut details = Vec::new();
        let home = if sketch { 54. } else { 50. };
        let camera = if sketch { 12. } else { 16. };
        for (kind, size) in [(DetailKind::Home, home), (DetailKind::Camera, camera)] {
            // Fitted details are authored in the native portrait frame and rotate
            // together. Stretched details retain their absolute dimensions.
            if !self.stretch {
                let x = (616. - size) / 2.;
                let y = if kind == DetailKind::Home {
                    (916. + 730. + size) / 2. - 6.
                } else {
                    (94. - size) / 2. - 1.
                };
                let r = if self.landscape {
                    Bounds::new(
                        frame.x + (916. - y - size) * zoom,
                        frame.y + x * zoom,
                        size * zoom,
                        size * zoom,
                    )
                } else {
                    Bounds::new(
                        frame.x + x * zoom,
                        frame.y + y * zoom,
                        size * zoom,
                        size * zoom,
                    )
                };
                details.push(Detail { kind, bounds: r });
            } else if self.landscape {
                if w > 186. && h > size + 10. {
                    let x = if kind == DetailKind::Home {
                        (screen.x - size) / 2. + 2.
                    } else {
                        (w + screen.x + screen.w - size) / 2.
                    };
                    details.push(Detail {
                        kind,
                        bounds: Bounds::new(x, (h - size) / 2., size, size),
                    });
                }
            } else if h > 186. && w > size + 10. {
                let y = if kind == DetailKind::Home {
                    (h + screen.h + size) / 2. - 6.
                } else {
                    (screen.y - size) / 2. - 1.
                };
                details.push(Detail {
                    kind,
                    bounds: Bounds::new((w - size) / 2., y, size, size),
                });
            }
        }
        let mut hide_items = false;
        if self.stretch {
            if w < if self.landscape { 194. } else { 72. } {
                screen.x = 2.;
                screen.w = w - 4.;
                hide_items = true;
            }
            if screen.h < 16. {
                screen.y = 2.;
                screen.h = h - 4.;
                hide_items = true;
            }
        }
        let mut fills = Vec::new();
        if !self.transparent {
            let bounds = if self.stretch {
                screen
            } else {
                let pad = frame.w.min(frame.h) * 0.03;
                Bounds::new(
                    frame.x + pad,
                    frame.y + pad,
                    frame.w - 2. * pad,
                    frame.h - 2. * pad,
                )
            };
            fills.push(Fill {
                bounds,
                color: [255; 3],
            });
        }
        let mut status_items = Vec::new();
        if self.status && (!self.stretch || screen.h > 16.) {
            let bar = Bounds::new(
                screen.x,
                screen.y
                    + if self.stretch && self.landscape {
                        2.
                    } else {
                        0.
                    },
                screen.w,
                16. * zoom,
            );
            fills.push(Fill {
                bounds: bar,
                color: [214, 222, 225],
            });
            if !hide_items {
                let color = if sketch { [0; 3] } else { [51; 3] };
                let font = 10. * zoom;
                let text = Bounds::new(bar.x, bar.y + bar.h / 4., bar.w, bar.h / 2.);
                if !self.stretch || measure_label(font).width + 4. < screen.w {
                    status_items.push(StatusItem {
                        kind: StatusKind::Label,
                        bounds: text,
                        size: font,
                        color,
                    });
                }
                if !self.stretch || w > if self.landscape { 306. } else { 185. } {
                    status_items.push(StatusItem {
                        kind: StatusKind::Time,
                        bounds: text,
                        size: font,
                        color,
                    });
                }
                let size = 14. * zoom;
                if !self.stretch || screen.w > 66. {
                    status_items.push(StatusItem {
                        kind: StatusKind::Battery,
                        bounds: Bounds::new(bar.x + bar.w - 24. * zoom, bar.y + zoom, size, size),
                        size,
                        color,
                    });
                }
                if !self.stretch || screen.w > 42. {
                    status_items.push(StatusItem {
                        kind: StatusKind::Wifi,
                        bounds: Bounds::new(bar.x + 28. * zoom, bar.y + zoom, size, size),
                        size,
                        color,
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
        }
    }
}
