// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Control-specific text parsing precedes ordinary inline formatting.
use super::RichText;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Align {
    Inherit,
    Left,
    Center,
    Right,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sort {
    Ascending,
    Descending,
    Both,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Choice {
    Unchecked,
    Checked,
    Indeterminate,
    RadioOff,
    RadioOn,
}
#[derive(Clone, Debug, PartialEq)]
pub enum CellContent {
    Text(RichText),
    Choice(Choice),
}
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub content: CellContent,
    pub sort: Option<Sort>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Column {
    pub weight: u32,
    pub align: Align,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub source: String,
    pub rows: Vec<Vec<Cell>>,
    pub columns: Vec<Column>,
}
impl Grid {
    /// Use the same cells as rendering, excluding choice markers and column
    /// specifications, while retaining source offsets through escaped commas.
    pub fn source_links(text: &str) -> Result<Vec<super::SourceLink>, String> {
        let mut result = vec![];
        let mut line_start = 0;
        let lines: Vec<_> = text.split('\n').collect();
        for (row, line) in lines.iter().enumerate() {
            if row + 1 == lines.len() && parse_columns(line).is_some() {
                break;
            }
            for range in cell_ranges(line) {
                let raw = &line[range.clone()];
                let trimmed = raw.trim();
                let start = line_start + range.start + raw.len() - raw.trim_start().len();
                let mut decoded = String::new();
                let mut offsets = vec![];
                let mut chars = trimmed.char_indices().peekable();
                while let Some((mut i, mut c)) = chars.next() {
                    if c == '\\' && chars.peek().is_some_and(|(_, c)| *c == ',') {
                        (i, c) = chars.next().unwrap();
                    }
                    decoded.push(c);
                    offsets.extend((0..c.len_utf8()).map(|byte| start + i + byte));
                }
                if let CellContent::Text(cell) = parse_cell(&decoded, row == 0).content {
                    for mut link in super::source_links(&cell.source)? {
                        link.range = offsets[link.range.start]..offsets[link.range.end - 1] + 1;
                        // The opening bracket is unchanged by comma decoding. Include
                        // a leading escape in the original label span.
                        link.label = link.range.start + 1..offsets[link.label.end - 1] + 1;
                        result.push(link);
                    }
                }
            }
            line_start += line.len() + 1;
        }
        Ok(result)
    }
    pub fn parse(text: &str) -> Self {
        let mut lines: Vec<_> = text.split('\n').collect();
        let widths = lines.last().and_then(|s| parse_columns(s));
        if widths.is_some() {
            lines.pop();
        }
        let mut rows = vec![];
        let mut count = 0;
        for (index, line) in lines.iter().enumerate() {
            let cells = split_cells(line)
                .into_iter()
                .map(|s| parse_cell(&s, index == 0))
                .collect::<Vec<_>>();
            count = count.max(cells.len());
            rows.push(cells);
        }
        let mut columns = widths.unwrap_or_default();
        columns.resize(
            count.max(columns.len()),
            Column {
                weight: 0,
                align: Align::Inherit,
            },
        );
        Self {
            source: text.into(),
            rows,
            columns,
        }
    }
    /// Inspector choices share the table grammar, omit headers and size rows,
    /// and retain choice markers while removing first-row sorting suffixes.
    pub fn selectable_labels(text: &str, header: bool) -> Vec<String> {
        let mut lines: Vec<_> = text.split('\n').collect();
        if lines.last().is_some_and(|s| parse_columns(s).is_some()) {
            lines.pop();
        }
        lines
            .iter()
            .enumerate()
            .skip(usize::from(header))
            .filter_map(|(row, line)| {
                let label = split_cells(line)
                    .into_iter()
                    .map(|source| match parse_cell(&source, row == 0).content {
                        CellContent::Text(text) => text.source,
                        CellContent::Choice(_) => source,
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                (!label.is_empty()).then_some(label)
            })
            .collect()
    }
    /// Unweighted columns scale proportionally. With positive weights, zero-weight
    /// columns keep intrinsic width and positive weights divide the remainder.
    pub fn column_widths(&self, intrinsic: &[f32], available: f32) -> Vec<f32> {
        let count = intrinsic.len();
        let weight: u64 = self
            .columns
            .iter()
            .take(count)
            .map(|c| c.weight as u64)
            .sum();
        if weight == 0 {
            let total: f32 = intrinsic.iter().sum();
            return intrinsic
                .iter()
                .map(|w| {
                    if total > 0. {
                        (available * w / total).ceil()
                    } else {
                        0.
                    }
                })
                .collect();
        }
        let reserved: f32 = intrinsic
            .iter()
            .enumerate()
            .filter(|(i, _)| self.columns.get(*i).is_none_or(|c| c.weight == 0))
            .map(|(_, w)| *w)
            .sum();
        let remaining = (available - reserved).max(0.);
        intrinsic
            .iter()
            .enumerate()
            .map(|(i, w)| match self.columns.get(i) {
                Some(c) if c.weight > 0 => (remaining * c.weight as f32 / weight as f32).round(),
                _ => *w,
            })
            .collect()
    }
}
fn split_cells(line: &str) -> Vec<String> {
    cell_ranges(line)
        .into_iter()
        .map(|range| line[range].replace("\\,", ",").trim().to_owned())
        .collect()
}
fn cell_ranges(line: &str) -> Vec<std::ops::Range<usize>> {
    let mut result = vec![];
    let mut start = 0;
    let mut chars = line.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '\\' && chars.peek().is_some_and(|(_, c)| *c == ',') {
            chars.next();
        } else if matches!(c, ',' | '\t') {
            result.push(start..i);
            start = i + c.len_utf8();
        }
    }
    result.push(start..line.len());
    result
}
fn parse_cell(text: &str, first: bool) -> Cell {
    let mut value = text;
    let mut sort = None;
    if first {
        for (suffix, kind) in [
            (" ^v", Sort::Both),
            (" v^", Sort::Both),
            (" ^", Sort::Ascending),
            (" v", Sort::Descending),
        ] {
            if value.to_ascii_lowercase().ends_with(suffix) {
                sort = Some(kind);
                value = &value[..value.len() - suffix.len()];
                break;
            }
        }
    }
    let choice = match text {
        "[ ]" | "[]" => Some(Choice::Unchecked),
        "[x]" | "[X]" | "[*]" => Some(Choice::Checked),
        "[-]" => Some(Choice::Indeterminate),
        "( )" | "()" => Some(Choice::RadioOff),
        "(o)" | "(O)" => Some(Choice::RadioOn),
        _ => None,
    };
    Cell {
        content: choice
            .map(CellContent::Choice)
            .unwrap_or_else(|| CellContent::Text(RichText::parse(value, false))),
        sort,
    }
}
fn parse_columns(text: &str) -> Option<Vec<Column>> {
    let inner = text.strip_prefix('{')?.strip_suffix('}')?;
    if inner.is_empty()
        || !inner
            .chars()
            .all(|c| c.is_ascii_digit() || ",\t LCRlcr".contains(c))
    {
        return None;
    }
    inner
        .split([',', '\t'])
        .map(|part| {
            let part = part.trim();
            let digits = part.trim_end_matches(|c: char| c.is_ascii_alphabetic());
            let weight = if digits.is_empty() {
                0
            } else {
                digits.parse().ok()?
            };
            let align = match &part[digits.len()..] {
                "" => Align::Inherit,
                "l" | "L" => Align::Left,
                "c" | "C" => Align::Center,
                "r" | "R" => Align::Right,
                _ => return None,
            };
            Some(Column { weight, align })
        })
        .collect()
}
#[derive(Clone, Debug, PartialEq)]
pub struct TreeRow {
    pub source: String,
    pub depth: usize,
    pub marker: Option<String>,
    pub text: RichText,
}
pub fn tree(text: &str) -> Vec<TreeRow> {
    if text.is_empty() {
        return vec![];
    }
    text.trim_end_matches(|c: char| matches!(c, '\u{0009}'..='\u{000d}' | ' ' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'))
        .split('\n')
        .map(|line| {
            let depth = line
                .chars()
                .take_while(|c| matches!(c, ' ' | '.' | '\t'))
                .count();
            let rest = &line[depth..];
            let markers = [
                ("[+]", "expand"),
                ("[-]", "collapse"),
                ("[=]", "collapse"),
                ("[x]", "checked"),
                ("[*]", "checked"),
                ("[ ]", "unchecked"),
                ("[]", "unchecked"),
                ("f", "folder"),
                ("F", "open_folder"),
                (">", "right"),
                ("<", "left"),
                ("v", "down"),
                ("-", "file"),
                ("_", "blank"),
            ];
            for (prefix, marker) in markers {
                if rest == prefix
                    || rest
                        .strip_prefix(prefix)
                        .is_some_and(|s| s.starts_with(' '))
                {
                    let content = rest.get(prefix.len() + 1..).unwrap_or("");
                    return TreeRow {
                        source: content.into(),
                        depth,
                        marker: Some(marker.into()),
                        text: RichText::parse(content, false),
                    };
                }
            }
            TreeRow {
                source: rest.into(),
                depth,
                marker: None,
                text: RichText::parse(rest, false),
            }
        })
        .collect()
}

/// Intrinsic cell sizes and allocated columns. Fixed-height distribution belongs to the renderer.
#[derive(Clone, Debug)]
pub struct GridMetrics {
    pub intrinsic_widths: Vec<f32>,
    pub column_widths: Vec<f32>,
    pub row_heights: Vec<f32>,
}
impl GridMetrics {
    pub fn width(&self) -> f32 {
        self.column_widths.iter().sum()
    }
    pub fn height(&self) -> f32 {
        self.row_heights.iter().sum()
    }
}
impl Grid {
    pub fn measure<M: crate::layout::TextMeasurer>(
        &self,
        measurer: &M,
        style: crate::layout::TextStyle,
        width: Option<f32>,
        padding: (f32, f32),
    ) -> Result<GridMetrics, String> {
        if ![style.size, padding.0, padding.1]
            .iter()
            .all(|v| v.is_finite() && *v >= 0.)
            || width.is_some_and(|w| !w.is_finite() || w < 0.)
        {
            return Err("Invalid grid measurement parameters".into());
        }
        let mut intrinsic_widths = vec![padding.0; self.columns.len()];
        let mut row_heights = vec![];
        for row in &self.rows {
            let mut height = style.size;
            for (i, cell) in row.iter().enumerate() {
                let (mut w, h) = match &cell.content {
                    CellContent::Text(text) => {
                        let m = measurer.measure_rich(text, style, None, true);
                        (m.width, m.height)
                    }
                    CellContent::Choice(_) => (style.size, style.size),
                };
                if !w.is_finite() || !h.is_finite() || w < 0. || h < 0. {
                    return Err("Text measurement returned invalid grid geometry".into());
                }
                if cell.sort.is_some() {
                    w += style.size;
                }
                if let Some(column) = intrinsic_widths.get_mut(i) {
                    *column = column.max(w + padding.0);
                }
                height = height.max(h);
            }
            row_heights.push(height + padding.1);
        }
        let column_widths = width.map_or_else(
            || intrinsic_widths.clone(),
            |w| self.column_widths(&intrinsic_widths, w),
        );
        Ok(GridMetrics {
            intrinsic_widths,
            column_widths,
            row_heights,
        })
    }
}

#[derive(Clone, Debug)]
pub struct TableMetrics {
    pub columns: Vec<f32>,
    pub rows: Vec<f32>,
    pub natural_width: f32,
    pub natural_height: f32,
    pub top: f32,
    pub bottom: f32,
}
impl Grid {
    /// Table frame padding is separate from intrinsic cell padding and row heights.
    pub fn table_layout<M: crate::layout::TextMeasurer>(
        &self,
        fonts: &M,
        style: crate::layout::TextStyle,
        width: Option<f32>,
        row_height: f32,
        header: bool,
        sketch: bool,
    ) -> Result<TableMetrics, String> {
        let count = self.columns.len();
        let mut intrinsic = vec![0f32; count];
        let mut rows = vec![];
        for (r, row) in self.rows.iter().enumerate() {
            let mut h = 0f32;
            for (column, cell) in row.iter().enumerate() {
                let (mut w, height) = match &cell.content {
                    CellContent::Choice(_) => (22., 22.),
                    CellContent::Text(text) => {
                        let m = fonts.measure_rich(text, style, None, true);
                        (m.width, m.height + 3.)
                    }
                };
                if !w.is_finite() || !height.is_finite() || w < 0. || height < 0. {
                    return Err("Invalid table text geometry".into());
                }
                if cell.sort.is_some() {
                    w += 17.;
                }
                intrinsic[column] = intrinsic[column].max(w);
                h = h.max(height);
            }
            rows.push(if header && r == 0 {
                h.max(16.)
            } else {
                h.max(row_height.clamp(16., 80.))
            });
        }
        for (i, w) in intrinsic.iter_mut().enumerate() {
            *w += if i == 0 || i + 1 == count { 3. } else { 6. };
        }
        let (top, bottom) = if sketch { (2., 3.) } else { (1., 1.) };
        let natural_width =
            intrinsic.iter().sum::<f32>() + 14. + count.saturating_sub(1) as f32 * 6.;
        let natural_height = rows.iter().sum::<f32>() + top + bottom;
        let available = (width.unwrap_or(natural_width) - 14.).max(1.);
        let columns = self.column_widths(&intrinsic, available);
        Ok(TableMetrics {
            columns,
            rows,
            natural_width,
            natural_height,
            top,
            bottom,
        })
    }
}
