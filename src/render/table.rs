// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::rich_text::structured::{Align, CellContent, Choice, Grid, Sort};
impl Scene {
    pub(super) fn table(
        &mut self,
        layer: &Layer,
        sketch: bool,
        fonts: &EguiMeasurer<'_>,
    ) -> Result<(), String> {
        let c = &layer.control;
        let b = c.bounds;
        let p = &layer.properties;
        let grid = Grid::parse(&c.text);
        let header = boolean(p, "header");
        let bordered = !matches!(p.get("border"), Some(Value::Choice(v)) if v == "none");
        let scrollbar = boolean(p, "vertical_scrollbar");
        let reserved = if scrollbar { 14. } else { 0. };
        let row_height = number(p, "row_height", 30.).clamp(16., 80.);
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let metrics = grid.table_layout(
            fonts,
            style,
            Some((b.w - reserved).max(1.)),
            row_height,
            header,
            sketch,
        )?;
        let clip = clip_with(layer.clip, b);
        let fill = rgb(p, "fill", [255, 255, 255]);
        let alternate = rgb(p, "alternate_fill", [238, 238, 238]);
        self.primitive(
            Primitive::Rect {
                b,
                fill,
                stroke: false,
                radius: 0.,
            },
            clip,
        );
        let selected = number(p, "selected_index", -1.) as i32;
        let rule_ink = if sketch { [0; 3] } else { [51; 3] };
        let rule_width = if sketch { 2. } else { 1. };
        let disabled = matches!(p.get("state"), Some(Value::Choice(s)) if s.contains("disabled"));
        let top = if bordered { metrics.top } else { 0. };
        let mut y = b.y + top;
        let mut row_index = 0usize;
        while y < b.y + b.h {
            let height = metrics.rows.get(row_index).copied().unwrap_or(row_height);
            let background = if header && row_index == 0 {
                [204, 204, 204]
            } else if selected >= 0
                && row_index < grid.rows.len()
                && row_index as i32 == selected + i32::from(header)
            {
                selection_color(p)
            } else if row_index % 2 == 1 {
                alternate
            } else {
                fill
            };
            self.primitive(
                Primitive::Rect {
                    b: Bounds::new(b.x, y, b.w, height),
                    fill: background,
                    stroke: false,
                    radius: 0.,
                },
                clip,
            );
            if boolean(p, "horizontal_lines") && row_index > 0 {
                let rule_y = y + if sketch { -1. } else { 0.5 };
                self.primitive(
                    Primitive::Line {
                        points: vec![[b.x, rule_y], [b.x + b.w, rule_y]],
                        color: rule_ink,
                        width: rule_width,
                    },
                    clip,
                );
            }
            if let Some(row) = grid.rows.get(row_index) {
                let mut x = b.x + 7.;
                for (i, cell) in row.iter().enumerate() {
                    let width = metrics.columns.get(i).copied().unwrap_or(0.);
                    let pad = if i == 0 || i + 1 == metrics.columns.len() {
                        3.
                    } else {
                        6.
                    };
                    let mut available = (width - pad).max(0.);
                    let mut options = paint_options(p);
                    let dark = u32::from(background[0]) * 299
                        + u32::from(background[1]) * 587
                        + u32::from(background[2]) * 114
                        < 153_000;
                    if dark && !disabled {
                        if boolean(p, "__table_auto_text_color") {
                            options.ink = Color32::WHITE;
                        }
                        if boolean(p, "__table_auto_link_color") {
                            options.link = Color32::WHITE;
                        }
                    }
                    let align =
                        grid.columns
                            .get(i)
                            .map_or(alignment(p), |column| match column.align {
                                Align::Inherit => alignment(p),
                                Align::Left => Alignment::Left,
                                Align::Center => Alignment::Center,
                                Align::Right => Alignment::Right,
                            });
                    if let Some(sort) = cell.sort {
                        if header && row_index == 0 && i + 1 == metrics.columns.len() {
                            available += reserved;
                        }
                        self.sort_icon(
                            x + available - 17.,
                            y + (height - 15.) / 2.,
                            sort,
                            clip,
                            options.ink,
                        );
                        available = (available - 17.).max(0.);
                    }
                    let cell_bounds = Bounds::new(x, y, available, height);
                    let cell_clip = clip_with(clip, cell_bounds);
                    if available > 0. {
                        match &cell.content {
                            CellContent::Text(text) => self.text(
                                text,
                                style,
                                TextBox {
                                    bounds: cell_bounds,
                                    align,
                                    middle: true,
                                    wrap: false,
                                    soft_breaks: true,
                                    truncate: true,
                                    clip: cell_clip,
                                },
                                fonts,
                                options,
                            )?,
                            CellContent::Choice(choice) => {
                                let inset = if matches!(align, Alignment::Right) {
                                    2.
                                } else {
                                    0.
                                };
                                let local_x = x - b.x + align.offset(available, 12.) - inset;
                                let size = if matches!(choice, Choice::RadioOff | Choice::RadioOn) {
                                    13.
                                } else {
                                    16.
                                };
                                self.choice(
                                    *choice,
                                    Bounds::new(
                                        b.x + local_x.round(),
                                        y + super::choice::center_offset(height, size),
                                        size,
                                        size,
                                    ),
                                    cell_clip,
                                    options.ink,
                                );
                            }
                        }
                    }
                    x += width;
                }
            }
            y += height;
            row_index += 1;
            if row_index > 10_000 {
                return Err("Table contains too many visible rows".into());
            }
        }
        if boolean(p, "vertical_lines") {
            let mut x = b.x + 7.;
            for width in metrics
                .columns
                .iter()
                .take(metrics.columns.len().saturating_sub(1))
            {
                x += width;
                self.primitive(
                    Primitive::Line {
                        points: vec![
                            [x - 1. - rule_width / 2., b.y],
                            [x - 1. - rule_width / 2., b.y + b.h],
                        ],
                        color: rule_ink,
                        width: rule_width,
                    },
                    clip,
                );
            }
        }
        if scrollbar {
            let inset = if bordered && sketch { 2. } else { 0. };
            let start = top
                + if header {
                    metrics.rows.first().copied().unwrap_or(row_height)
                } else {
                    0.
                };
            self.vertical_scrollbar(
                Bounds::new(
                    b.x + b.w - 16. - inset,
                    b.y + start,
                    16.,
                    b.h - start - inset,
                ),
                number(p, "value", 0.),
                false,
                sketch,
                clip,
            );
        }
        if bordered {
            self.rectangle_frame(b, rule_ink, rule_width, layer.clip);
        }
        Ok(())
    }
    fn choice(&mut self, choice: Choice, bounds: Bounds, clip: Option<Bounds>, color: Color32) {
        use super::choice::Mark;
        let radio = matches!(choice, Choice::RadioOff | Choice::RadioOn);
        let mark = match choice {
            Choice::Checked | Choice::RadioOn => Mark::Selected,
            Choice::Indeterminate => Mark::Mixed,
            _ => Mark::Empty,
        };
        self.choice_symbol(bounds, radio, mark, clip, color);
    }
    fn sort_icon(&mut self, x: f32, y: f32, sort: Sort, clip: Option<Bounds>, color: Color32) {
        let color = [color.r(), color.g(), color.b()];
        if matches!(sort, Sort::Ascending | Sort::Both) {
            self.primitive(
                Primitive::Polygon {
                    points: vec![[x + 2.5, y + 6.5], [x + 7.5, y + 1.], [x + 12.5, y + 6.5]],
                    fill: color,
                },
                clip,
            );
        }
        if matches!(sort, Sort::Descending | Sort::Both) {
            self.primitive(
                Primitive::Polygon {
                    points: vec![[x + 2.5, y + 8.5], [x + 12.5, y + 8.5], [x + 7.5, y + 14.]],
                    fill: color,
                },
                clip,
            );
        }
    }
}
