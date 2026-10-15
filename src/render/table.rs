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
        let row_height = number(p, "row_height", 30.).clamp(16., 80.);
        let style = TextStyle {
            size: c.font_size,
            bold: c.bold,
            italic: boolean(p, "italic"),
        };
        let metrics = grid.table_layout(fonts, style, Some(b.w), row_height, header, sketch)?;
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
        let disabled = matches!(p.get("state"), Some(Value::Choice(s)) if s.contains("disabled"));
        let mut y = b.y + metrics.top;
        let mut row_index = 0usize;
        while y < b.y + b.h {
            let height = metrics.rows.get(row_index).copied().unwrap_or(row_height);
            let background = if header && row_index == 0 {
                [204, 204, 204]
            } else if selected >= 0
                && row_index < grid.rows.len()
                && row_index as i32 == selected + i32::from(header)
            {
                [149, 188, 242]
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
                self.primitive(
                    Primitive::Line {
                        points: vec![[b.x, y], [b.x + b.w, y]],
                        color: rule_ink,
                        width: 1.,
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
                            CellContent::Choice(choice) => self.choice(
                                *choice,
                                x + align.offset(available, 12.),
                                y + (height - 12.) / 2.,
                                cell_clip,
                                options.ink,
                            ),
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
                        points: vec![[x - 1., b.y], [x - 1., b.y + b.h]],
                        color: rule_ink,
                        width: 1.,
                    },
                    clip,
                );
            }
        }
        for points in [
            vec![[b.x, b.y], [b.x + b.w, b.y]],
            vec![[b.x + b.w, b.y], [b.x + b.w, b.y + b.h]],
            vec![[b.x + b.w, b.y + b.h], [b.x, b.y + b.h]],
            vec![[b.x, b.y + b.h], [b.x, b.y]],
        ] {
            self.primitive(
                Primitive::Line {
                    points,
                    color: rule_ink,
                    width: 1.,
                },
                layer.clip,
            );
        }
        Ok(())
    }
    fn choice(&mut self, choice: Choice, x: f32, y: f32, clip: Option<Bounds>, color: Color32) {
        use super::choice::Mark;
        let radio = matches!(choice, Choice::RadioOff | Choice::RadioOn);
        let mark = match choice {
            Choice::Checked | Choice::RadioOn => Mark::Selected,
            Choice::Indeterminate => Mark::Mixed,
            _ => Mark::Empty,
        };
        self.choice_symbol(Bounds::new(x, y, 12., 12.), radio, mark, clip, color);
    }
    fn sort_icon(&mut self, x: f32, y: f32, sort: Sort, clip: Option<Bounds>, color: Color32) {
        let color = [color.r(), color.g(), color.b()];
        if matches!(sort, Sort::Ascending | Sort::Both) {
            self.primitive(
                Primitive::Line {
                    points: vec![[x + 3., y + 6.], [x + 7., y + 2.], [x + 11., y + 6.]],
                    color,
                    width: 1.,
                },
                clip,
            );
        }
        if matches!(sort, Sort::Descending | Sort::Both) {
            self.primitive(
                Primitive::Line {
                    points: vec![[x + 3., y + 9.], [x + 7., y + 13.], [x + 11., y + 9.]],
                    color,
                    width: 1.,
                },
                clip,
            );
        }
    }
}
