// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Deterministic calendar and date-field geometry, independent of the wall clock.
use crate::{
    layout::{TextMeasurer, TextStyle},
    model::Bounds,
    rich_text::RichText,
};
use chrono::{Datelike, NaiveDate};

pub const CHOOSER_DEFAULT: &str = "  /  /    ";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Date(NaiveDate);
impl Date {
    pub fn new(year: i32, month: u32, day: u32) -> Option<Self> {
        NaiveDate::from_ymd_opt(year, month, day).map(Self)
    }
    pub fn today() -> Self {
        Self(chrono::Local::now().date_naive())
    }
    pub fn year(self) -> i32 {
        self.0.year()
    }
    pub fn month(self) -> u32 {
        self.0.month()
    }
    pub fn day(self) -> u32 {
        self.0.day()
    }
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct Label {
    pub text: String,
    pub bounds: Bounds,
    pub muted: bool,
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct CalendarLayout {
    pub font_size: f32,
    pub heading: Option<Label>,
    pub weekdays: Vec<Label>,
    pub days: Vec<Label>,
    pub selection: Option<Bounds>,
    pub arrows: Vec<Bounds>,
    pub rules: Vec<Bounds>,
    pub border: f32,
}
pub fn calendar<M: TextMeasurer>(
    date: Date,
    width: f32,
    height: f32,
    project_font: f32,
    sketch: bool,
    fonts: &M,
) -> CalendarLayout {
    let border = if sketch { 3. } else { 1. };
    let mut font_size = 8.;
    for size in 9..=19 {
        let threshold = 97. + 10. * (size as f32 - 8.);
        if font_size >= project_font
            || threshold >= width - border / 2.
            || threshold >= height - border
        {
            break;
        }
        font_size = size as f32;
    }
    let style = TextStyle {
        size: font_size,
        bold: false,
        italic: false,
    };
    let months = [
        "JANUARY",
        "FEBRUARY",
        "MARCH",
        "APRIL",
        "MAY",
        "JUNE",
        "JULY",
        "AUGUST",
        "SEPTEMBER",
        "OCTOBER",
        "NOVEMBER",
        "DECEMBER",
    ];
    let month = months[(date.month() - 1) as usize];
    let title = format!("{month} {}", date.year());
    let title_size = fonts.measure(&title, style, None);
    let day_size = fonts.measure("95", style, None);
    let heading_height = (title_size.height + 11.).max(14.);
    let row_top = heading_height + 1.5 * day_size.height + 8.;
    let pitch = (height - row_top) / 6.;
    let show_days = day_size.width * 7. + 8. + 2. * border <= width
        && row_top + day_size.height * 6. + 2. * border < height;
    let mut result = CalendarLayout {
        font_size,
        heading: None,
        weekdays: vec![],
        days: vec![],
        selection: None,
        arrows: vec![],
        rules: vec![Bounds::new(0., 10. + title_size.height, width, border)],
        border,
    };
    if show_days {
        for (column, name) in ["S", "M", "T", "W", "T", "F", "S"].iter().enumerate() {
            result.weekdays.push(Label {
                text: (*name).into(),
                bounds: Bounds::new(
                    (width - 2. * border) * column as f32 / 7. + 3. + border,
                    heading_height + day_size.height / 2.,
                    day_size.width,
                    day_size.height,
                ),
                muted: false,
            });
        }
        let first = date.0.with_day(1).expect("valid first day");
        let leading = first.weekday().num_days_from_sunday() as usize;
        let start = first.checked_sub_signed(chrono::Duration::days(leading as i64));
        if let Some(start) = start {
            for cell in 0..42 {
                let Some(value) = start.checked_add_signed(chrono::Duration::days(cell as i64))
                else {
                    break;
                };
                let row = (cell / 7) as f32;
                let column = (cell % 7) as f32;
                let x = border - 1.5 + column * (width / 7. - 1.5);
                result.days.push(Label {
                    text: value.day().to_string(),
                    bounds: Bounds::new(
                        x + 6.,
                        row_top + pitch * row + if row == 0. { 0. } else { 1. },
                        (width - 2. * border) / 7.,
                        day_size.height,
                    ),
                    muted: value.month() != date.month() || value.year() != date.year(),
                });
                if value == date.0 && width > 93. && height > 93. {
                    result.selection = Some(Bounds::new(
                        x + if date.day() > 9 { 5. } else { 1. },
                        row_top + pitch * row - 2. - if font_size < 9. { 0. } else { 1. },
                        font_size * 1.5,
                        font_size * 1.5,
                    ));
                }
            }
        }
    }
    let full = format!("{} {title}", date.day());
    let mut heading = if !show_days && fonts.measure(&full, style, None).width < width {
        full
    } else {
        title.clone()
    };
    let mut arrows = true;
    if fonts.measure(&heading, style, None).width + 12. > width {
        arrows = false;
        heading = title.clone();
        if fonts.measure(&heading, style, None).width + 12. > width {
            heading = format!("{} '{:02}", &month[..3], date.year().rem_euclid(100));
            if fonts.measure(&heading, style, None).width + 12. > width {
                heading = title;
            }
        }
    }
    let heading_width = fonts.measure(&heading, style, None).width;
    if width >= heading_width + 2. * border && height >= 10. + title_size.height + border {
        result.heading = Some(Label {
            text: heading,
            bounds: Bounds::new(
                border,
                border,
                width - 2. * border,
                heading_height - 2. * border,
            ),
            muted: false,
        });
    }
    if arrows && height >= 12. + 2. * border && width >= heading_width + 48. {
        result.arrows = vec![
            Bounds::new(border, (heading_height - 12.) / 2., 12., 12.),
            Bounds::new(width - border - 12., (heading_height - 12.) / 2., 12., 12.),
        ];
    }
    if width >= 200. && height >= 200. {
        let weight = if sketch { 2. } else { 1. };
        for row in 0..6 {
            result.rules.push(Bounds::new(
                border / 2.,
                row_top + row as f32 * pitch - 5.,
                width - border,
                weight,
            ));
        }
        for column in 1..7 {
            result.rules.push(Bounds::new(
                border + column as f32 * (width / 7. - 1.5),
                row_top - 5.,
                weight,
                height - row_top + day_size.height / 2. - border + 5.,
            ));
        }
    }
    result
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ChooserLayout {
    pub width: f32,
    pub height: f32,
    pub field: Option<Bounds>,
    pub text: Option<Bounds>,
    pub icon: Bounds,
}
pub fn chooser<M: TextMeasurer>(
    text: &RichText,
    style: TextStyle,
    width: Option<f32>,
    sketch: bool,
    fonts: &M,
) -> ChooserLayout {
    let measured = fonts.measure_rich(text, style, None, true);
    let width = width.unwrap_or((measured.width + 47.).max(90.));
    let height = (measured.height + if sketch { 12. } else { 11. }).max(24.);
    let field_width = width - if sketch { 35. } else { 30. };
    let field = (field_width > 0.).then(|| Bounds::new(0., 0., field_width, height));
    let y = (height - measured.height) / 2. + (style.size - 8.) / 12.;
    ChooserLayout {
        width,
        height,
        field,
        text: field.map(|b| Bounds::new(6., y, (b.w - 6.).max(1.), (height - y).max(1.))),
        icon: Bounds::new(width - 24., (height - 24.) / 2., 24., 24.),
    }
}
