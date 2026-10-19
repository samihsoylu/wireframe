// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Text-family geometry and parsing shared by the canvas and exporters.
use crate::{
    layout::{TextMeasurer, TextMetrics, TextStyle},
    model::Bounds,
    rich_text::RichText,
};

pub const CLOUD_DEFAULT: &str = "ideas design sketch prototype {color:#FF0000}feedback{color} simple clear useful explore create web\\ design build share refine";
pub fn fixed_size(kind: &str) -> Option<(f32, f32)> {
    match kind {
        "text_block" => Some((275., 80.)),
        "text_line" => Some((200., 27.)),
        "tag_cloud" => Some((350., 110.)),
        _ => None,
    }
}
pub fn title_text(source: &str, subtitle: bool, page_title: &str) -> RichText {
    let source = if source.is_empty() {
        if subtitle { "Subtitle" } else { "Title" }
    } else {
        source
    };
    let literal = page_title
        .replace('\\', "\\\\")
        .replace('_', "\\_")
        .replace('*', "\\*")
        .replace('[', "\\[");
    RichText::parse(&source.replace("{mockup-name}", &literal), false)
}
pub fn title_box(width: f32, height: f32, subtitle: bool) -> Bounds {
    let top = if subtitle { 1. } else { 4. };
    Bounds::new(0., top, width.max(1.), (height - top - 4.).max(1.))
}
pub fn line_width(width: f32, height: f32) -> f32 {
    (4056. / 72. * height).max(width).min(4056.)
}
/// Rows are omitted unless their full font-height fits strictly inside the frame.
pub fn block_rows(width: f32, height: f32, font: f32) -> Vec<Bounds> {
    const SHORTEN: [u8; 22] = [
        0, 2, 0, 1, 10, 0, 5, 0, 1, 3, 2, 30, 0, 1, 4, 9, 1, 4, 7, 8, 4, 6,
    ];
    let mut rows = Vec::new();
    let font = font.max(1.);
    let mut y = 3_f64;
    let step = f64::from(font) * (1. + 1. / 4.64);
    let mut i = 0;
    while y + f64::from(font) < f64::from(height) {
        let w =
            width - (f64::from(width) * f64::from(SHORTEN[i % SHORTEN.len()]) / 30.).round() as f32;
        if w > 0. {
            rows.push(Bounds::new(0., y as f32, w, font));
        }
        y += step;
        i += 1;
    }
    rows
}
/// Original word rhythm and silhouettes. Paragraph spacing is independent of frame width.
pub fn block_words(width: f32, height: f32, font: f32) -> Vec<Bounds> {
    const WORDS: &str = "Every fresh idea starts with a small sketch Give each thought room to grow and leave space for the next question Make a simple plan then bring the useful details together";
    const PARAGRAPHS: [usize; 6] = [105, 63, 48, 115, 49, 18];
    let words: Vec<_> = WORDS.split(' ').map(str::len).collect();
    let font = font.max(1.);
    let scale = font / 13.;
    let mut out = Vec::new();
    let (mut x, mut y, mut row, mut word, mut paragraph, mut count) = (0., 3., 1, 0, 0, 0);
    while y + font < height && row < 90 {
        let w = (words[word % words.len()] as f32 * 4.7 + 4.) * scale;
        if x > 0. && x + w > width {
            x = 0.;
            y += font + font / 4.64;
            row += 1;
            continue;
        }
        out.push(Bounds::new(x, y, w, 10. * scale));
        x += w + 4. * scale;
        word += 1;
        count += 1;
        if count == PARAGRAPHS[paragraph % PARAGRAPHS.len()] {
            x = 0.;
            y += 2. * (font + font / 4.64);
            row += 2;
            paragraph += 1;
            count = 0;
        }
    }
    out
}
/// Stable size cadence, expressed as numeric behavior, independent of text content.
pub fn cloud_font(id: u64, index: usize) -> f32 {
    const SIZES: [u8; 76] = [
        12, 10, 18, 17, 13, 16, 13, 17, 19, 19, 11, 8, 6, 10, 16, 12, 18, 17, 17, 16, 12, 17, 9, 6,
        15, 15, 19, 10, 19, 6, 17, 21, 9, 10, 11, 12, 11, 10, 19, 14, 19, 9, 20, 18, 6, 9, 13, 21,
        15, 11, 13, 20, 15, 15, 21, 12, 11, 13, 13, 9, 13, 11, 12, 14, 9, 12, 8, 13, 17, 19, 17, 6,
        14, 19, 6, 7,
    ];
    f32::from(SIZES[((id % 76) as usize + index % 76) % 76])
}
pub fn cloud_labels(source: &str) -> Vec<String> {
    let mut labels = vec![String::new()];
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&' ') {
            chars.next();
            labels.last_mut().unwrap().push(' ');
        } else if c == ' ' {
            labels.push(String::new());
        } else {
            labels.last_mut().unwrap().push(c);
        }
    }
    if labels.len() == 1 {
        labels.push("...".into());
    }
    labels
}
pub fn cloud_rebuild(labels: &[String]) -> String {
    labels
        .iter()
        .map(|s| s.replace(' ', "\\ "))
        .collect::<Vec<_>>()
        .join(" ")
}
#[derive(Clone, Debug)]
pub struct CloudLabel {
    pub text: RichText,
    pub font: f32,
    pub bounds: Bounds,
    pub visible: bool,
}
pub fn cloud_layout(
    source: &str,
    id: u64,
    width: f32,
    height: f32,
    measurer: &impl TextMeasurer,
    style: TextStyle,
) -> Vec<CloudLabel> {
    cloud_layout_with(source, id, width, height, |text, font| {
        measurer.measure_rich(
            text,
            TextStyle {
                size: font,
                ..style
            },
            None,
            true,
        )
    })
}
pub fn cloud_layout_with(
    source: &str,
    id: u64,
    width: f32,
    height: f32,
    mut measure: impl FnMut(&RichText, f32) -> TextMetrics,
) -> Vec<CloudLabel> {
    let (mut x, mut baseline) = (9., 25.);
    cloud_labels(source)
        .into_iter()
        .enumerate()
        .map(|(index, label)| {
            let font = cloud_font(id, index);
            let text = RichText::parse(&label, false);
            let m = measure(&text, font);
            if x + m.width + 5. >= width {
                x = 9.;
                baseline += 24.;
            }
            let bounds = Bounds::new(x, baseline - m.height, m.width, m.height);
            x += m.width + 5.;
            CloudLabel {
                text,
                font,
                bounds,
                visible: baseline < height,
            }
        })
        .collect()
}

/// Label text direction is independent of device or icon orientation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LabelOrientation {
    #[default]
    Upright,
    Left,
    Right,
    Diagonal,
}
impl LabelOrientation {
    pub fn from_properties(properties: &crate::document::Properties) -> Self {
        match properties.get("text_orientation") {
            Some(crate::document::Value::Choice(value)) => match value.as_str() {
                "left" => Self::Left,
                "right" => Self::Right,
                "diagonal" => Self::Diagonal,
                _ => Self::Upright,
            },
            _ => Self::Upright,
        }
    }
    pub fn radians(self) -> f32 {
        match self {
            Self::Upright => 0.,
            Self::Left => -std::f32::consts::FRAC_PI_2,
            Self::Right => std::f32::consts::FRAC_PI_2,
            Self::Diagonal => -std::f32::consts::PI / 12.,
        }
    }
    pub fn size(self, text_width: f32, text_height: f32, vertical_padding: f32) -> (f32, f32) {
        let (w, h) = match self {
            Self::Upright => (text_width, text_height + vertical_padding),
            Self::Left | Self::Right => (text_height + vertical_padding, text_width),
            Self::Diagonal => {
                let (s, c) = self.radians().abs().sin_cos();
                (
                    (text_width * c + text_height * s).round(),
                    (text_width * s + text_height * c).round(),
                )
            }
        };
        (w.max(1.), h.max(1.))
    }
}
