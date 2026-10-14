// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Window frame geometry shared by the editor and exports.
use crate::{model::Bounds, rich_text::RichText};
pub const DEFAULT: &str = "Window title";
#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub header: f32,
    pub footer: f32,
    pub minimize: bool,
    pub maximize: bool,
    pub close: bool,
    pub resize_handle: bool,
    pub scrollbar: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            header: 26.,
            footer: 25.,
            minimize: false,
            maximize: false,
            close: false,
            resize_handle: false,
            scrollbar: false,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Button {
    Minimize,
    Maximize,
    Close,
}
#[derive(Debug)]
pub struct Layout {
    pub header: Bounds,
    pub footer: Option<Bounds>,
    pub title: Bounds,
    pub buttons: Vec<(Button, Bounds)>,
    pub resize_handle: Option<Bounds>,
    pub scrollbar: Option<Bounds>,
}
pub fn parse(source: &str) -> Result<RichText, String> {
    if source.len() > 100_000 {
        return Err("Window title exceeds the text limit".into());
    }
    Ok(RichText::parse(source, false))
}
impl Options {
    pub fn layout(self, width: f32, height: f32, title_height: f32, sketch: bool) -> Layout {
        let header = self.header.max(title_height);
        let (left, top, right, scroll_top, scroll_bottom) = if sketch {
            (6., 7., 2., header, self.footer - 2.)
        } else {
            (5., 8., 0., header - 1., self.footer - 1.)
        };
        let buttons = [
            (Button::Minimize, self.minimize, 38.),
            (Button::Maximize, self.maximize, 22.),
            (Button::Close, self.close, if sketch { 7. } else { 8. }),
        ]
        .into_iter()
        .filter_map(|(kind, enabled, inset)| {
            let x = width - inset - 13.;
            (enabled && height >= 19. && x > 0.).then(|| (kind, Bounds::new(x, 6., 13., 13.)))
        })
        .collect();
        let grip_height = if sketch { 11. } else { 11.001 };
        let scroll_height = height - scroll_top - scroll_bottom;
        Layout {
            header: Bounds::new(0., 0., width, header),
            footer: (height - self.footer > header)
                .then(|| Bounds::new(0., height - self.footer, width, self.footer.min(height))),
            title: Bounds::new(left, top, (width - left - 53.).max(0.), title_height),
            buttons,
            resize_handle: (self.resize_handle && width > 17. && height >= 17. + grip_height)
                .then(|| Bounds::new(width - 17., height - 17., 13., grip_height)),
            scrollbar: (self.scrollbar && width >= right + 16. && scroll_height > 0.)
                .then(|| Bounds::new(width - right - 16., scroll_top, 16., scroll_height)),
        }
    }
}
