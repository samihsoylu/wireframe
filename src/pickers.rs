// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Measured layouts for illustrative date and time dialogs.
use crate::{
    date_controls::Date,
    layout::{TextMeasurer, TextMetrics, TextStyle},
    model::Bounds,
    tablet::Clock,
};
use chrono::{Datelike, NaiveDate};
use serde::Serialize;

pub const BUTTONS: [&str; 2] = ["CANCEL", "OK"];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Month,
    Year,
    Time,
}
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub enum Ink {
    Normal,
    Selected,
    Active,
    Inactive,
    Link,
}
#[derive(Clone, Debug, Serialize)]
pub struct Label {
    pub text: String,
    pub bounds: Bounds,
    pub size: f32,
    pub bold: bool,
    pub ink: Ink,
    pub button: Option<usize>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Layout {
    pub labels: Vec<Label>,
    pub heading: Option<Bounds>,
    pub face: Option<Bounds>,
    pub selection: Option<Bounds>,
    pub hand: Option<[[f32; 2]; 2]>,
    pub arrows: Vec<Bounds>,
    pub border: f32,
}
fn style(size: f32, bold: bool) -> TextStyle {
    TextStyle {
        size,
        bold,
        italic: false,
    }
}
fn metrics(fonts: &impl TextMeasurer, s: &str, size: f32) -> TextMetrics {
    fonts.measure(s, style(size, false), None)
}
impl Layout {
    fn label(&mut self, text: impl Into<String>, bounds: Bounds, size: f32, ink: Ink, bold: bool) {
        self.labels.push(Label {
            text: text.into(),
            bounds,
            size,
            ink,
            bold,
            button: None,
        });
    }
    fn years(
        &mut self,
        b: Bounds,
        scale: f32,
        year: i32,
        neighbors: i32,
        fonts: &impl TextMeasurer,
    ) {
        let small = metrics(fonts, "TEST", 18. * scale).height;
        let gap = (b.h - (neighbors * 2 + 1) as f32 * small) / (2 + 2 * neighbors) as f32;
        if gap <= 0. {
            return;
        }
        let mut y = b.y + gap;
        for offset in -neighbors..=neighbors {
            let size = if offset == 0 { 32. } else { 18. } * scale;
            let text = (year + offset).to_string();
            let m = metrics(fonts, &text, size);
            self.label(
                text,
                Bounds::new(b.x + (b.w - m.width) / 2., y, m.width, m.height),
                size,
                Ink::Normal,
                false,
            );
            y += m.height + gap;
        }
    }
    fn month(&mut self, b: Bounds, scale: f32, date: Date, fonts: &impl TextMeasurer) {
        let font = 13. * scale;
        let size = font.round();
        if b.w <= 0. || b.h <= 0. || size <= 0. {
            return;
        }
        let month = format!(
            "{} {}",
            MONTHS[(date.month() - 1) as usize].to_uppercase(),
            date.year()
        );
        let m = metrics(fonts, &month, font);
        let icon = 24. * scale;
        let padding = 5. * scale;
        let header = (icon + 2.).max(m.height + 2. * padding);
        self.label(
            month,
            Bounds::new(
                b.x + (b.w - m.width) / 2.,
                b.y + (header - m.height) / 2.,
                m.width,
                m.height,
            ),
            font,
            Ink::Normal,
            false,
        );
        self.arrows = vec![
            Bounds::new(b.x, b.y + (header - icon) / 2., icon, icon),
            Bounds::new(b.x + b.w - icon, b.y + (header - icon) / 2., icon, icon),
        ];
        let day_height = metrics(fonts, "S", size).height;
        let weekday_height = day_height * 1.5;
        let gap = (b.h - header - weekday_height - 6. * (day_height - 3.)) / 6. - font * 0.4;
        let column = (b.w - 2.) / if b.h > b.w { 6.525 } else { 6.4 };
        let first = NaiveDate::from_ymd_opt(date.year(), date.month(), 1).expect("valid date");
        let leading = first.weekday().num_days_from_sunday() as usize;
        let days = (28..=31)
            .rev()
            .find(|d| NaiveDate::from_ymd_opt(date.year(), date.month(), *d).is_some())
            .unwrap();
        let labels = ["S", "M", "T", "W", "T", "F", "S"]
            .into_iter()
            .map(str::to_owned)
            .chain((0..leading).map(|_| String::new()))
            .chain((1..=days).map(|d| d.to_string()));
        for (i, text) in labels.enumerate() {
            let selected = i == 6 + leading + date.day() as usize;
            let m = fonts.measure(&text, style(size, selected), None);
            let y = if i < 7 {
                b.y + 10. * scale + header + weekday_height * 0.3
            } else {
                b.y + 25. * scale
                    + header
                    + weekday_height
                    + (gap + m.height) * ((i - 7) / 7) as f32
                    + font * 0.4
            };
            let bounds = Bounds::new(
                b.x + column * (i % 7) as f32 + 1.5 * padding,
                y,
                m.width,
                m.height,
            );
            if selected {
                let diameter = font * 2.4;
                self.selection = Some(Bounds::new(
                    bounds.x + (m.width - diameter) / 2.,
                    bounds.y + (m.height - diameter) / 2.,
                    diameter,
                    diameter,
                ));
            }
            self.label(
                text,
                bounds,
                size,
                if selected { Ink::Selected } else { Ink::Normal },
                selected,
            );
        }
    }
}
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
/// Positions content independently of drawing and destination storage.
pub struct Options {
    pub mode: Mode,
    pub width: f32,
    pub height: f32,
    pub sketch: bool,
    pub project_font: f32,
}
pub fn layout(options: Options, date: Date, clock: Clock, fonts: &impl TextMeasurer) -> Layout {
    let Options {
        mode,
        width,
        height,
        sketch,
        project_font,
    } = options;
    let mut out = Layout {
        border: if sketch { 3. } else { 1. },
        ..Default::default()
    };
    let button_sizes = BUTTONS.map(|s| metrics(fonts, s, 14.));
    let bh = button_sizes[0].height.max(button_sizes[1].height);
    let right = width - button_sizes[1].width - 20.;
    for (index, x) in [(0, right - 20. - button_sizes[0].width), (1, right)] {
        if x > 0. {
            out.labels.push(Label {
                text: BUTTONS[index].into(),
                bounds: Bounds::new(x, height - bh - 10., button_sizes[index].width, bh),
                size: 14.,
                bold: false,
                ink: Ink::Link,
                button: Some(index),
            });
        }
    }
    let portrait = width < height;
    let scale = if portrait {
        ((height - 95. - bh - 10.) / (473. - 95. - bh - 10.)).min(width / 320.)
    } else {
        ((height - bh - 10.) / (303. - bh - 10.)).min((width - 188.) / (570. - 188.))
    };
    let heading = if portrait { 95. } else { 188. } * scale;
    let b = out.border;
    if mode == Mode::Time {
        let panel = if portrait {
            Bounds::new(b, b, width - 2. * b, heading - 2. * b)
        } else {
            Bounds::new(b, b, heading - 2. * b, height - 2. * b)
        };
        if panel.w >= 0. && panel.h > 0. && (portrait || panel.w > 0.) {
            out.heading = Some(panel);
            digital(&mut out, panel, scale, portrait, clock, fonts);
        }
        let body_scale = if portrait {
            ((height - heading - bh - 10.) / (473. - 95. - bh - 10.)).min(width / 320.)
        } else if height < 25. {
            0.
        } else {
            ((height - bh - 10.) / (303. - bh - 10.))
                .min((width - heading) / (570. - 188.))
                .abs()
        };
        analog(
            &mut out,
            [width, height],
            heading,
            body_scale,
            portrait,
            clock,
            fonts,
        );
        return out;
    }
    let panel = if portrait {
        Bounds::new(b, b, width - 2. * b, heading - b)
    } else {
        Bounds::new(b, b, heading - b, height - 2. * b)
    };
    if panel.w <= 0. || panel.h <= 0. {
        return out;
    }
    out.heading = Some(panel);
    let body_scale = if portrait {
        ((height - heading - bh - 10.) / (473. - heading - bh - 10.)).min(width / 320.)
    } else {
        ((height - bh - 10.) / (303. - bh - 10.)).min((width - heading) / (570. - heading))
    };
    let year = date.year().to_string();
    let day = NaiveDate::from_ymd_opt(date.year(), date.month(), date.day()).expect("valid date");
    let weekday = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
        [day.weekday().num_days_from_sunday() as usize];
    let text = format!(
        "{weekday}{}{} {:02}",
        if portrait { ", " } else { "\n" },
        &MONTHS[(date.month() - 1) as usize][..3],
        date.day()
    );
    let ym = metrics(fonts, &year, project_font);
    let dm = metrics(fonts, &text, project_font);
    let (x, y) = if portrait {
        let spare = width - b - 10. - (b + 10. + 290. * scale);
        (b + 10. + spare / 2., 20. * scale)
    } else {
        let spare = (height - bh - 10. - (20. + b + 260. * body_scale)).max(0.);
        (20. * scale + b, b + 10. * scale + spare / 2.)
    };
    if mode == Mode::Year {
        let area = if portrait {
            Bounds::new(
                b,
                heading + b,
                (width - 2. * b).max(1.),
                (height - heading - 2. * b - bh).max(1.),
            )
        } else {
            Bounds::new(
                heading + b,
                b,
                (width - heading - b).max(1.) - b,
                (height - 2. * b - bh).max(1.),
            )
        };
        out.years(
            area,
            scale,
            date.year(),
            if portrait { 3 } else { 2 },
            fonts,
        );
    } else {
        let pad = 10. * body_scale;
        let area = if portrait {
            Bounds::new(
                pad,
                heading + b + pad,
                width - 2. * (pad + b),
                height - 3. * pad - 2. * b - bh - heading,
            )
        } else {
            let h = height - 2. * b - 3. * pad - bh;
            Bounds::new(
                heading + b + pad,
                (height - h - bh) / 2.,
                (width - heading - 2. * (pad + b)).max(0.),
                h,
            )
        };
        out.month(area, body_scale, date, fonts);
    }
    out.label(
        text,
        Bounds::new(x, y + ym.height, dm.width, dm.height),
        project_font,
        Ink::Normal,
        false,
    );
    out.label(
        year,
        Bounds::new(x, y, ym.width, ym.height),
        project_font,
        Ink::Normal,
        false,
    );
    out
}
fn digital(
    out: &mut Layout,
    panel: Bounds,
    scale: f32,
    portrait: bool,
    clock: Clock,
    fonts: &impl TextMeasurer,
) {
    let label = clock.label();
    let (time, period) = label.split_once(' ').expect("clock label");
    let size = if portrait { 60. } else { 54. } * scale;
    let ps = if portrait { 14. } else { 18. } * scale;
    let tm = metrics(fonts, time, size);
    let am = metrics(fonts, "AM", ps);
    let pm = metrics(fonts, "PM", ps);
    let (x, y) = if portrait {
        (
            (panel.w - tm.width - am.width) / 2.,
            (panel.h - tm.height) / 2.,
        )
    } else {
        (
            (panel.w - tm.width) / 2.,
            (panel.h - tm.height - am.height) / 2.,
        )
    };
    out.label(
        time,
        Bounds::new(x, y, tm.width, tm.height),
        size,
        Ink::Normal,
        false,
    );
    let (px, py) = if portrait {
        (x + tm.width + 20., y + tm.height / 2. - am.height)
    } else {
        (x + (tm.width - am.width) / 2., y + tm.height)
    };
    for (p, m, y) in [("AM", am, py), ("PM", pm, py + am.height)] {
        out.label(
            p,
            Bounds::new(px, y, m.width, m.height),
            ps,
            if p == period {
                Ink::Active
            } else {
                Ink::Inactive
            },
            false,
        );
    }
}
fn analog(
    out: &mut Layout,
    size: [f32; 2],
    heading: f32,
    scale: f32,
    portrait: bool,
    clock: Clock,
    fonts: &impl TextMeasurer,
) {
    let [w, h] = size;
    let diameter = if portrait { 285. } else { 250. } * scale;
    if diameter <= 0. {
        return;
    }
    let cx = (w + if portrait { 0. } else { heading }) / 2.;
    let cy = (h + if portrait { heading } else { 0. }) / 2.;
    out.face = Some(Bounds::new(
        cx - diameter / 2.,
        cy - diameter / 2.,
        diameter,
        diameter,
    ));
    let hour = clock.hour_12();
    for number in 1..=12 {
        let text = number.to_string();
        let size = 14. * scale;
        let m = metrics(fonts, &text, size);
        let angle = (number as f32 * 30.).to_radians();
        let x = cx + 0.45 * diameter * angle.sin();
        let y = cy - 0.45 * diameter * angle.cos();
        let selected = number == hour;
        if selected {
            out.hand = Some([[cx, cy], [x, y]]);
            out.selection = Some(Bounds::new(
                x - diameter * 0.0425,
                y - diameter * 0.0425,
                diameter * 0.085,
                diameter * 0.085,
            ));
        }
        out.label(
            text,
            Bounds::new(x - m.width / 2., y - m.height / 2., m.width, m.height),
            size,
            if selected { Ink::Selected } else { Ink::Normal },
            false,
        );
    }
}
