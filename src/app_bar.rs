// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! App-bar measurement, icon slots and destination-preserving collection edits.
use crate::{
    document::{IconSource, Link, Properties, Value},
    layout::{TextMeasurer, TextMetrics, TextStyle},
    model::Bounds,
    rich_text::RichText,
};

pub const DEFAULT: &str = "Heading";
pub struct AppBar {
    pub title: RichText,
    pub nonempty: bool,
    pub navigation: bool,
    pub filter: bool,
    pub menu: bool,
    pub icons: Vec<Option<IconSource>>,
}
pub struct Metrics {
    pub width: f32,
    pub height: f32,
    pub title: TextMetrics,
}
#[derive(Debug)]
pub struct Placement {
    pub index: usize,
    pub bounds: Bounds,
    pub link: Bounds,
}
pub struct Layout {
    pub title: Option<Bounds>,
    pub icons: Vec<Placement>,
}
pub fn text_style(size: f32) -> TextStyle {
    TextStyle {
        size,
        bold: false,
        italic: false,
    }
}
impl AppBar {
    pub fn parse(source: &str, properties: &Properties) -> Result<Self, String> {
        if source.len() > 100_000 {
            return Err("App bar exceeds the text limit".into());
        }
        let enabled = |key| !matches!(properties.get(key), Some(Value::Bool(false)));
        Ok(Self {
            title: RichText::parse(source, false),
            nonempty: !source.is_empty(),
            navigation: enabled("nav_icon"),
            filter: enabled("filter_icon"),
            menu: enabled("menu_icon"),
            icons: match properties.get("icons") {
                Some(Value::Icons(icons)) => icons.clone(),
                _ => vec![],
            },
        })
    }
    pub fn measure(&self, fonts: &impl TextMeasurer, size: f32) -> Metrics {
        let title = fonts.measure_rich(&self.title, text_style(size), None, true);
        let width = 13.
            + if self.navigation { 42. } else { 0. }
            + if self.filter { 29. } else { 0. }
            + if self.menu { 29. } else { 0. }
            + if self.nonempty { title.width + 6.5 } else { 0. }
            + self.icons.len() as f32 * 29.;
        Metrics {
            width: width.trunc().max(320.),
            height: (title.height + 15.).max(44.),
            title,
        }
    }
    pub fn layout(&self, metrics: &Metrics, width: f32, height: f32) -> Layout {
        let mut icons = vec![];
        let mut place = |index, x: f32, y: f32| {
            icons.push(Placement {
                index,
                bounds: Bounds::new(x, y, 16., 16.),
                link: Bounds::new((x - 2.).max(0.), (y - 2.).max(0.), 20., 20.),
            });
        };
        let mut cursor = 13.;
        if self.navigation && cursor + 16. < width && height > 30. {
            place(0, cursor, 14.);
            cursor += 42.;
        }
        let title_y = height - 14. - metrics.title.height;
        let title = (title_y > 0.)
            .then(|| Bounds::new(cursor, title_y, metrics.title.width, metrics.title.height));
        let filter_x = cursor + metrics.title.width + 6.5;
        if title.is_some() {
            cursor += metrics.title.width + 13.;
        }
        let filter_y = if self.nonempty {
            title_y + (metrics.title.height - 16.) / 2.
        } else {
            height - 30.
        };
        let filter = self.filter && cursor + 16. < width && filter_y > 0.;
        if filter {
            place(1, filter_x, filter_y);
            cursor += 16.;
        }
        let menu = self.menu && cursor + 16. < width && height > 30.;
        let boundary = cursor - if filter { 0. } else { 13. };
        let mut x = width - (self.icons.len() + usize::from(menu)) as f32 * 29.;
        for index in 0..self.icons.len() {
            if boundary < x && height > 30. {
                place(index + 2, x, 14.);
            }
            x += 29.;
        }
        if menu {
            place(self.icons.len() + 2, width - 29., 14.);
        }
        Layout { title, icons }
    }
    pub fn icon(&self, index: usize) -> Option<IconSource> {
        match index {
            0 => Some(IconSource::Catalog("bars-solid".into())),
            1 => Some(IconSource::Catalog("caret-down-solid".into())),
            i if i == self.icons.len() + 2 => {
                Some(IconSource::Catalog("ellipsis-vertical-solid".into()))
            }
            i => self.icons.get(i.checked_sub(2)?).cloned().flatten(),
        }
    }
}
pub fn foreground(fill: [u8; 4]) -> [u8; 4] {
    let brightness = u32::from(fill[0]) * 299 + u32::from(fill[1]) * 587 + u32::from(fill[2]) * 114;
    if brightness < 153_000 {
        [255; 4]
    } else {
        [0, 0, 0, 255]
    }
}
/// Match duplicate icons in occurrence order. Built-in destinations retain their slots.
pub fn remap_links(
    before: &[Option<IconSource>],
    after: &[Option<IconSource>],
    links: &[Option<Link>],
) -> Vec<Option<Link>> {
    let mut destinations: std::collections::HashMap<_, std::collections::VecDeque<_>> =
        std::collections::HashMap::new();
    for (index, icon) in before.iter().enumerate() {
        destinations
            .entry(icon)
            .or_default()
            .push_back(links.get(index + 2).cloned().flatten());
    }
    let mut out = vec![
        links.first().cloned().flatten(),
        links.get(1).cloned().flatten(),
    ];
    for icon in after {
        out.push(
            destinations
                .get_mut(icon)
                .and_then(|queue| queue.pop_front())
                .flatten(),
        );
    }
    out.push(links.last().cloned().flatten());
    out
}
pub fn labels(properties: &Properties) -> Vec<String> {
    let mut labels = vec!["Navigation".into(), "Filter".into()];
    if let Some(Value::Icons(icons)) = properties.get("icons") {
        labels.extend(icons.iter().enumerate().map(|(i, icon)| {
            match icon {
                Some(IconSource::Catalog(id)) => crate::icons::get(id)
                    .map_or_else(|| format!("Icon {}", i + 1), |icon| icon.label.clone()),
                Some(IconSource::Asset(_)) => format!("Image {}", i + 1),
                None => format!("Icon {}", i + 1),
            }
        }));
    }
    labels.push("Menu".into());
    labels
}
