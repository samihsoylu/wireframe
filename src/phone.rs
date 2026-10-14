// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Device layout is independent of frame artwork, typography and export backends.
use crate::{
    document::{Properties, Value},
    layout::TextMetrics,
    model::Bounds,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Classic,
    Tall,
    Wide,
    Notched,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pattern {
    White,
    Header,
    HeaderFooter,
    HeaderDarkFooter,
    Dark,
}
#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub model: Model,
    pub landscape: bool,
    pub stretch: bool,
    pub pattern: Pattern,
    pub transparent: bool,
    pub status: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            model: Model::Classic,
            landscape: false,
            stretch: false,
            pattern: Pattern::White,
            transparent: false,
            status: true,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Fill {
    pub bounds: Bounds,
    pub color: [u8; 3],
    pub radius: f32,
    pub round_top: bool,
    pub round_bottom: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusKind {
    Carrier,
    Time,
    Signal,
    Wifi,
    Battery,
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
    Speaker,
}
#[derive(Clone, Copy, Debug)]
pub struct Detail {
    pub kind: DetailKind,
    pub bounds: Bounds,
    pub rotated: bool,
}
impl Detail {
    pub fn visible_bounds(self) -> Bounds {
        if self.rotated {
            Bounds::new(
                self.bounds.x,
                self.bounds.y - self.bounds.w,
                self.bounds.h,
                self.bounds.w,
            )
        } else {
            self.bounds
        }
    }
}
#[derive(Debug)]
pub struct Layout {
    pub frame: Bounds,
    pub zoom: f32,
    pub screen: Bounds,
    pub fills: Vec<Fill>,
    pub status_items: Vec<StatusItem>,
    pub details: Vec<Detail>,
    /// Local coordinates of left, top, right and bottom proximity guides.
    pub guides: [f32; 4],
    pub status_visible: bool,
}
impl Model {
    pub fn native(self) -> (f32, f32) {
        match self {
            Self::Classic => (282., 510.),
            Self::Tall => (282., 573.),
            Self::Wide => (380., 754.),
            Self::Notched => (436., 867.),
        }
    }
    /// Top, left, right and bottom fixed frame sections for each appearance.
    pub fn sections(self, sketch: bool) -> [f32; 4] {
        match (self, sketch) {
            (Self::Classic, true) => [87., 161., 94., 90.],
            (Self::Classic, false) => [164., 42., 98., 90.],
            (Self::Tall, true) => [204., 62., 97., 88.],
            (Self::Tall, false) => [167., 37., 97., 93.],
            (Self::Wide, true) => [263., 61., 73., 99.],
            (Self::Wide, false) => [265., 57., 69., 101.],
            (Self::Notched, true) => [331., 140., 137., 71.],
            (Self::Notched, false) => [325., 142., 136., 62.],
        }
    }
    pub fn detail_sizes(self, sketch: bool) -> (Option<(f32, f32)>, (f32, f32)) {
        let home = match self {
            Self::Notched => None,
            Self::Wide => Some(if sketch { (58., 58.) } else { (56., 56.) }),
            _ => Some(if sketch { (52., 52.) } else { (50., 50.) }),
        };
        let speaker = match (self, sketch) {
            (Self::Classic, true) => (84., 24.),
            (Self::Classic, false) => (80., 16.),
            (Self::Tall, true) => (60., 38.),
            (Self::Tall, false) => (52., 32.),
            (Self::Wide, true) => (84., 38.),
            (Self::Wide, false) => (84., 34.),
            (Self::Notched, true) => (87., 17.),
            (Self::Notched, false) => (81., 16.),
        };
        (home, speaker)
    }
}
impl Options {
    pub fn from_properties(p: &Properties) -> Self {
        let choice = |key| match p.get(key) {
            Some(Value::Choice(s)) => s.as_str(),
            _ => "",
        };
        Self {
            model: match choice("device_model") {
                "tall" => Model::Tall,
                "wide" => Model::Wide,
                "notched" => Model::Notched,
                _ => Model::Classic,
            },
            landscape: choice("orientation") == "landscape",
            stretch: choice("device_resizing") == "stretch",
            pattern: match choice("background_pattern") {
                "header" => Pattern::Header,
                "header_footer" => Pattern::HeaderFooter,
                "header_dark_footer" => Pattern::HeaderDarkFooter,
                "dark" => Pattern::Dark,
                _ => Pattern::White,
            },
            transparent: p.get("transparent_background") == Some(&Value::Bool(true)),
            status: p.get("status_bar") != Some(&Value::Bool(false)),
        }
    }
    pub fn native(self) -> (f32, f32) {
        let (w, h) = self.model.native();
        if self.landscape { (h, w) } else { (w, h) }
    }
    pub fn frame(self, w: f32, h: f32) -> Bounds {
        if self.stretch {
            return Bounds::new(0., 0., w, h);
        }
        let (nw, nh) = self.native();
        let scale = (w / nw).min(h / nh);
        let fw = nw * scale;
        let fh = nh * scale;
        Bounds::new((w - fw) / 2., (h - fh) / 2., fw, fh)
    }
    /// The caller measures the two fixed status labels at the supplied font size.
    pub fn layout(
        self,
        w: f32,
        h: f32,
        sketch: bool,
        measure: impl FnOnce(f32) -> (TextMetrics, TextMetrics),
    ) -> Layout {
        let frame = self.frame(w, h);
        let (nw, _) = self.native();
        let zoom = if self.stretch { 1. } else { frame.w / nw };
        let notched = self.model == Model::Notched;
        let wide = self.model == Model::Wide;
        let sections = self.model.sections(sketch);
        let (left, top, right, bottom) = if !self.stretch {
            let [pl, pr, pt, pb] = match self.model {
                Model::Wide => [29., 29., 90., 95.],
                Model::Notched => [25., 24., 22., 22.],
                _ => [24., 24., 83., 80.],
            };
            let edges = if self.landscape {
                [pt, pr, pb, pl]
            } else {
                [pl, pt, pr, pb]
            };
            (
                frame.x + edges[0] * zoom,
                frame.y + edges[1] * zoom,
                w - frame.x - frame.w + edges[2] * zoom,
                h - frame.y - frame.h + edges[3] * zoom,
            )
        } else {
            let (side, cap, mut tail) = if notched {
                (24., 22., 0.)
            } else if wide {
                (24., 87., 7.)
            } else {
                (23., 81., 3.)
            };
            if self.landscape {
                let min_w = if wide { 360. } else { 254. };
                let min_h = if wide { 126. } else { 132. };
                let x = cap - (min_w - w).max(0.) / if wide { 4. } else { 3. };
                let y = if notched && h < 230. {
                    1.
                } else {
                    (side - (min_h - h).max(0.) / if wide { 4. } else { 2. }).max(0.)
                };
                (x, y, x, y)
            } else {
                let min_h = sections[0] + sections[3];
                let min_w = if wide {
                    107.
                } else if notched {
                    237.
                } else {
                    135.
                };
                let y = (cap - (min_h - h).max(0.) / if wide { 4. } else { 2. }).max(0.);
                if h < min_h {
                    tail /= 4.;
                }
                let x = side - (min_w - w).max(0.) / 2.;
                (x, y, x, y + tail)
            }
        };
        let screen = Bounds::new(left, top, w - left - right, h - top - bottom);
        let status_h = zoom * if notched { 30. } else { 16. };
        let detail_width = w > if self.landscape { 215. } else { 260. };
        let gate = if self.landscape {
            sections[1] + sections[2]
        } else {
            sections[0] + sections[3]
        };
        let visible = (!self.stretch || h > gate) && (!notched || detail_width);
        let mut content_top = top
            + if self.status {
                status_h - if notched { 1. } else { 0. }
            } else {
                0.
            };
        let mut guides = [left, content_top, w - right, h - bottom];
        let mut fills = Vec::new();
        let dark = self.pattern == Pattern::Dark;
        let base_color = if dark { [63; 3] } else { [255; 3] };
        if !self.transparent {
            let b = if self.stretch {
                Bounds::new(left.max(1.), top, screen.w.min(w - 2.), screen.h)
            } else {
                let offset = if wide { 1. } else { 0. };
                Bounds::new(
                    left,
                    (top - offset).max(0.),
                    screen.w,
                    (screen.h + offset).min(h),
                )
            };
            fills.push(Fill {
                bounds: b,
                color: base_color,
                radius: if notched && self.stretch { 35. } else { 1. },
                round_top: true,
                round_bottom: true,
            });
        }
        let mut header_h = 40. * zoom;
        let footer_h = zoom * if self.landscape { 30. } else { 40. };
        let dark_footer_h = zoom * if self.landscape { 39. } else { 40. };
        if notched && !visible && self.status {
            content_top = top;
            header_h += status_h;
        }
        let free = h - content_top - bottom;
        let header = self.pattern == Pattern::Header && visible && free > header_h;
        let two_bars = matches!(
            self.pattern,
            Pattern::HeaderFooter | Pattern::HeaderDarkFooter
        ) && free > header_h + footer_h;
        let mut pattern_color = base_color;
        if header || two_bars {
            pattern_color = [130, 136, 150];
            guides[1] += header_h;
            let radius = if notched && (!detail_width || !visible) {
                35.
            } else {
                1.
            };
            fills.push(Fill {
                bounds: Bounds::new(left, content_top, screen.w, header_h),
                color: pattern_color,
                radius,
                round_top: notched && self.stretch,
                round_bottom: false,
            });
            if two_bars {
                let black = self.pattern == Pattern::HeaderDarkFooter;
                let height = if black { dark_footer_h } else { footer_h };
                guides[3] -= height;
                fills.push(Fill {
                    bounds: Bounds::new(left, h - bottom - height, screen.w, height),
                    color: if black { [63; 3] } else { pattern_color },
                    radius,
                    round_top: false,
                    round_bottom: notched && self.stretch,
                });
            }
        }
        let font_size = zoom * if notched { 12. } else { 10. };
        let (carrier, time) = measure(font_size);
        let mut status_items = Vec::new();
        if self.status && visible {
            let ink = if (notched && self.pattern != Pattern::White) || (!notched && dark) {
                [255; 3]
            } else if sketch {
                [0; 3]
            } else {
                [51; 3]
            };
            if !notched
                || !self.transparent
                || matches!(
                    self.pattern,
                    Pattern::Header | Pattern::HeaderFooter | Pattern::HeaderDarkFooter
                )
            {
                fills.push(Fill {
                    bounds: Bounds::new(left, top, screen.w, status_h),
                    color: if notched {
                        pattern_color
                    } else if dark {
                        [46; 3]
                    } else {
                        [214, 222, 225]
                    },
                    radius: 0.,
                    round_top: false,
                    round_bottom: false,
                });
            }
            let start = left + zoom * 10.;
            if ((!notched && (!self.stretch || start + status_h < screen.w))
                || (notched && detail_width)
                || !self.stretch)
                && status_h > 2.
            {
                let icon_size = status_h - if notched { 15. } else { 2. };
                let show_notch = detail_width && icon_size > 0.;
                let mut cursor = start;
                let signal_x = if notched {
                    left + screen.w - 25. * zoom - 3. * (icon_size + 5.)
                } else {
                    start + 1.
                };
                let signal_y = if notched {
                    top + status_h - icon_size - 1.
                } else {
                    top + 1.
                };
                if icon_size > 0. {
                    status_items.push(StatusItem {
                        kind: StatusKind::Signal,
                        bounds: Bounds::new(signal_x, signal_y, icon_size, icon_size),
                        size: icon_size,
                        color: ink,
                    });
                    cursor = if notched {
                        signal_x + icon_size + 5.
                    } else {
                        start + icon_size + 2. + zoom * 2.
                    };
                }
                if notched || !self.stretch || cursor + carrier.width < w - left {
                    let offset = if wide { 1. } else { 0. };
                    if !notched {
                        status_items.push(StatusItem {
                            kind: StatusKind::Carrier,
                            bounds: Bounds::new(cursor, top + offset, carrier.width, status_h),
                            size: font_size,
                            color: ink,
                        });
                    }
                    if (font_size > 0. && !self.stretch)
                        || screen.w - (left + zoom + 2. * status_h + carrier.width) > time.width
                        || (notched && show_notch)
                    {
                        let bounds = if notched {
                            Bounds::new(
                                left + 28. * zoom,
                                top + status_h - time.height,
                                time.width,
                                time.height,
                            )
                        } else {
                            Bounds::new(left, top + offset, screen.w, status_h)
                        };
                        status_items.push(StatusItem {
                            kind: StatusKind::Time,
                            bounds,
                            size: font_size,
                            color: ink,
                        });
                    }
                    if notched && show_notch {
                        status_items.push(StatusItem {
                            kind: StatusKind::Wifi,
                            bounds: Bounds::new(cursor, signal_y, icon_size, icon_size),
                            size: icon_size,
                            color: ink,
                        });
                        cursor += icon_size + 5.;
                    }
                    if (!notched
                        && (!self.stretch
                            || w - right - status_h - zoom * 12. > cursor + carrier.width))
                        || (notched && show_notch)
                    {
                        let x = if notched {
                            cursor
                        } else {
                            w - right - icon_size - zoom * 12.
                        };
                        let y = if notched { signal_y } else { top };
                        status_items.push(StatusItem {
                            kind: StatusKind::Battery,
                            bounds: Bounds::new(x, y, icon_size, icon_size),
                            size: icon_size,
                            color: ink,
                        });
                    }
                }
            }
        }
        let mut details = Vec::new();
        if self.stretch {
            let (home, (sw, sh)) = self.model.detail_sizes(sketch);
            let (hx, hy, sx, sy) = match self.model {
                Model::Classic => (
                    -4.,
                    -3.,
                    if self.landscape { 5. } else { -13. },
                    if self.landscape { 29. } else { 5. },
                ),
                Model::Tall => (
                    -3.,
                    0.,
                    if self.landscape { -10. } else { 0. },
                    if self.landscape { 1. } else { -7. },
                ),
                Model::Wide => (
                    0.,
                    2.,
                    if self.landscape { -5. } else { -16. },
                    if self.landscape { 30. } else { -6. },
                ),
                Model::Notched => (
                    0.,
                    0.,
                    if self.landscape { 22. } else { 14. },
                    if self.landscape { -13. } else { 21. },
                ),
            };
            if let Some((bw, bh)) = home {
                let show = if self.landscape {
                    screen.h > bw && left > bh
                } else {
                    screen.w > bw && bottom > bh
                };
                if show {
                    let b = if self.landscape {
                        Bounds::new(w - (right + bh) / 2. + hx, (h + bw) / 2., bw, bh)
                    } else {
                        Bounds::new((w - bw) / 2., h - (bottom + bh) / 2. + hy, bw, bh)
                    };
                    details.push(Detail {
                        kind: DetailKind::Home,
                        bounds: b,
                        rotated: self.landscape,
                    });
                }
            }
            let show = if self.landscape {
                if notched {
                    w > 389. && h > 350.
                } else {
                    screen.h > sw && right > sh
                }
            } else {
                (if notched { w > 356. } else { screen.w > sw }) && top > sh
            };
            if show {
                let b = if self.landscape {
                    Bounds::new((left - sh) / 2. + sx, (h + sw) / 2. + sy, sw, sh)
                } else {
                    Bounds::new((w - sw) / 2. + sx, (top - sh) / 2. + sy, sw, sh)
                };
                details.push(Detail {
                    kind: DetailKind::Speaker,
                    bounds: b,
                    rotated: self.landscape,
                });
            }
        }
        Layout {
            frame,
            zoom,
            screen,
            fills,
            status_items,
            details,
            guides,
            status_visible: visible,
        }
    }
}
