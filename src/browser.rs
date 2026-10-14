// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Fixed-size browser-frame content and geometry, independent of painting.
use crate::{
    layout::{TextMeasurer, TextStyle},
    model::Bounds,
    rich_text::RichText,
};
pub const DEFAULT: &str = "Page title\nhttps://example.com";
pub struct Browser {
    pub title: RichText,
    pub address: RichText,
}
#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub address_top: f32,
    pub address_left: f32,
    pub address_right: f32,
    pub content_top: f32,
    pub bottom: f32,
    pub scrollbar_right: f32,
    pub slices: [f32; 4],
    pub tear_stroke: f32,
}
impl Style {
    pub fn new(sketch: bool) -> Self {
        if sketch {
            Self {
                address_top: 35.,
                address_left: 160.,
                address_right: 94.,
                content_top: 74.,
                bottom: 27.,
                scrollbar_right: 4.,
                slices: [77., 162., 93., 34.],
                tear_stroke: 3.,
            }
        } else {
            Self {
                address_top: 48.,
                address_left: 126.,
                address_right: 46.,
                content_top: 72.,
                bottom: 0.,
                scrollbar_right: 0.,
                slices: [73., 123., 38., 4.],
                tear_stroke: 1.,
            }
        }
    }
}
#[derive(Debug)]
pub struct Layout {
    pub title: Bounds,
    pub address: Option<Bounds>,
    pub white: Bounds,
    pub content: Option<Bounds>,
    pub skin: Bounds,
    pub scrollbar: Option<Bounds>,
    pub torn: bool,
    pub mask: bool,
}
impl Browser {
    pub fn parse(source: &str, page_title: &str) -> Result<Self, String> {
        if source.len() > 100_000 {
            return Err("Browser text exceeds the text limit".into());
        }
        let (title, address) = source.split_once('\n').unwrap_or((source, ""));
        // A page name is literal content, even if it contains formatting syntax.
        let literal = page_title
            .replace('_', "\\_")
            .replace('*', "\\*")
            .replace('[', "\\[");
        Ok(Self {
            title: RichText::parse(&title.replace("{mockup-name}", &literal), false),
            address: RichText::parse(&address.replace("{mockup-name}", &literal), false),
        })
    }
    #[allow(clippy::too_many_arguments)]
    pub fn layout(
        &self,
        fonts: &impl TextMeasurer,
        font_size: f32,
        width: f32,
        height: f32,
        sketch: bool,
        torn: bool,
        scrollbar: bool,
    ) -> Layout {
        let text = TextStyle {
            size: font_size,
            bold: false,
            italic: false,
        };
        self.layout_with_heights(
            width,
            height,
            sketch,
            torn,
            scrollbar,
            fonts.measure_rich(&self.title, text, None, true).height,
            fonts.measure_rich(&self.address, text, None, true).height,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn layout_with_heights(
        &self,
        width: f32,
        height: f32,
        sketch: bool,
        torn: bool,
        scrollbar: bool,
        title_height: f32,
        address_height: f32,
    ) -> Layout {
        let s = Style::new(sketch);
        let torn = torn && height > 38.;
        let address_width = width - s.address_left - s.address_right;
        let content_height = height - s.content_top - s.bottom;
        let scroll_height = height - s.content_top - if torn { 0. } else { s.bottom };
        Layout {
            title: Bounds::new(0., 8., width, title_height),
            address: (address_width > 0.)
                .then(|| Bounds::new(s.address_left, s.address_top, address_width, address_height)),
            white: Bounds::new(
                2.,
                2.,
                width - 4.,
                if torn {
                    height - 2.
                } else {
                    s.content_top - 4.
                },
            ),
            content: (content_height > 0.).then(|| {
                Bounds::new(
                    1.,
                    s.content_top,
                    width - 2.,
                    content_height + if torn { s.bottom } else { 0. },
                )
            }),
            skin: Bounds::new(0., 0., width, height + if torn { s.bottom } else { 0. }),
            scrollbar: (scrollbar && scroll_height > 0.).then(|| {
                Bounds::new(
                    width - s.scrollbar_right - 16.,
                    s.content_top,
                    16.,
                    scroll_height,
                )
            }),
            torn,
            mask: torn && width > 66.,
        }
    }
}
/// A monotone lower edge. Narrow frames have one centered notch; larger ones
/// place notches near both ends, adding intermediate notches as space grows.
pub fn torn_edge(width: f32, baseline: f32, thickness: f32) -> Vec<[f32; 2]> {
    let gap = width * 0.8 - 116.;
    let starts = if gap < 50. {
        vec![(width / 2. - 29.).floor()]
    } else {
        let inner = (gap / 308.).floor() as usize;
        let mut starts = vec![width * 0.1];
        starts.extend(
            (1..=inner).map(|i| width * 0.1 + 58. + gap * i as f32 / (inner + 1) as f32 - 29.),
        );
        starts.push(width * 0.9 - 58.);
        starts
    };
    let amplitude = 19. - thickness / 2.;
    let mut points = vec![[0., baseline]];
    for x in starts {
        points.extend([
            [x, baseline],
            [x + 18., baseline - amplitude],
            [x + 40., baseline + amplitude],
            [x + 58., baseline],
        ]);
    }
    points.push([width, baseline]);
    points
}
/// Transform fixed chrome regions independently from its stretchable center.
#[derive(Clone, Copy)]
pub struct Chrome {
    pub width: f32,
    pub height: f32,
    pub style: Style,
}
impl Chrome {
    pub fn x(&self, x: f32, from_right: bool) -> f32 {
        let factor = (self.width / (self.style.slices[1] + self.style.slices[2])).min(1.);
        if from_right {
            self.width - x * factor
        } else {
            x * factor
        }
    }
    pub fn y(&self, y: f32, from_bottom: bool) -> f32 {
        let factor = (self.height / (self.style.slices[0] + self.style.slices[3])).min(1.);
        if from_bottom {
            self.height - y * factor
        } else {
            y * factor
        }
    }
}
