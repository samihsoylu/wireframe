// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Seven visible rows of a fixed mobile picker; all source text remains in storage.
use crate::{
    layout::{TextMeasurer, TextStyle},
    model::Bounds,
    rich_text::RichText,
};
pub const DEFAULT: &str = "9,13\n10,14\n11,15, AM\n12,16, PM\n1,17\n2,18\n3,19";
pub const SIZE: (f32, f32) = (214., 160.);
pub struct Picker {
    pub rows: Vec<Vec<Option<RichText>>>,
}
#[derive(Debug, serde::Serialize)]
pub struct Layout {
    pub columns: Vec<f32>,
    pub cells: Vec<Vec<Bounds>>,
}
pub fn style(row: usize, base: f32) -> TextStyle {
    TextStyle {
        size: match row {
            0 | 6 => 10.,
            1 | 5 => 11.,
            2 | 4 => 12.,
            _ => base,
        },
        bold: false,
        italic: false,
    }
}
pub fn ink(row: usize, sketch: bool) -> [u8; 3] {
    match row {
        0 | 6 => [204; 3],
        1 | 5 => [153; 3],
        2 | 4 => [119; 3],
        _ => {
            if sketch {
                [0; 3]
            } else {
                [51; 3]
            }
        }
    }
}
impl Picker {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > 100_000 {
            return Err("Mobile picker exceeds the text limit".into());
        }
        Ok(Self {
            rows: text
                .split('\n')
                .take(7)
                .map(|line| {
                    crate::navigation::split(line)
                        .into_iter()
                        .map(|cell| (!line.is_empty()).then(|| RichText::parse(cell.trim(), false)))
                        .collect()
                })
                .collect(),
        })
    }
    pub fn layout(&self, fonts: &impl TextMeasurer, base: f32) -> Layout {
        let count = self.rows.iter().map(Vec::len).max().unwrap_or(0);
        let mut columns = vec![0_f32; count];
        for (row, cells) in self.rows.iter().enumerate() {
            for (col, cell) in cells.iter().enumerate() {
                let width = cell.as_ref().map_or(2., |t| {
                    fonts.measure_rich(t, style(row, base), None, true).width
                });
                columns[col] = columns[col].max(width);
            }
        }
        let measured = 14. + columns.iter().sum::<f32>() + count.saturating_sub(1) as f32 * 6.;
        let grid = measured.clamp(144., 214.);
        let factor = (grid / measured).max(1.);
        let cells = self
            .rows
            .iter()
            .enumerate()
            .map(|(row, cells)| {
                let mut x = (214. - grid) / 2. + 7. * factor;
                cells
                    .iter()
                    .enumerate()
                    .map(|(col, _)| {
                        let width = columns[col] * factor;
                        let bounds = Bounds::new(x, 8. + row as f32 * 22., width, 22.);
                        x += width + 6. * factor;
                        bounds
                    })
                    .collect()
            })
            .collect();
        Layout { columns, cells }
    }
}
