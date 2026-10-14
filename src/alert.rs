// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Alert text, intrinsic dimensions and positioned button regions.
use crate::{
    document::{Node, Properties, Value},
    layout::{TextMeasurer, TextMetrics, TextStyle},
    model::Bounds,
    rich_text::RichText,
};
pub const DEFAULT: &str = "Confirm changes\nApply these changes to the project?\nCancel, Apply";
#[derive(Debug)]
pub struct Button {
    pub source: String,
    pub text: RichText,
}
#[derive(Debug)]
pub struct Alert {
    pub title: RichText,
    pub message: Option<RichText>,
    pub buttons: Vec<Button>,
}
pub struct Metrics {
    pub width: f32,
    pub height: f32,
    pub title: TextMetrics,
    pub buttons: Vec<TextMetrics>,
}
#[derive(Clone, Debug)]
pub struct ButtonPlacement {
    pub index: usize,
    pub text: Bounds,
    pub link: Option<Bounds>,
}
pub struct Layout {
    pub title: Option<Bounds>,
    pub message: Option<Bounds>,
    pub buttons: Vec<ButtonPlacement>,
    pub divider_y: Option<f32>,
    pub split: bool,
}
fn trim_label(s: &str) -> &str {
    s.trim_matches(|c: char| (c.is_whitespace() && c != '\u{85}') || c == '\u{feff}')
}
impl Alert {
    pub fn parse(source: &str) -> Result<Self, String> {
        if source.len() > 100_000 {
            return Err("Alert exceeds the text limit".into());
        }
        let mut lines = source.split('\n');
        let title = RichText::parse(lines.next().unwrap_or(""), false);
        let message = lines.next().map(|s| RichText::parse(s, false));
        let buttons = lines
            .next()
            .map(|s| {
                s.split(',')
                    .take(2)
                    .map(|label| {
                        let source = trim_label(label).to_owned();
                        let text = RichText::parse(&source, false);
                        Button { source, text }
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Self {
            title,
            message,
            buttons,
        })
    }
    pub fn measure(&self, fonts: &impl TextMeasurer, style: TextStyle) -> Metrics {
        let title = fonts.measure_rich(
            &self.title,
            TextStyle {
                size: style.size + 2.,
                ..style
            },
            None,
            true,
        );
        let buttons: Vec<_> = self
            .buttons
            .iter()
            .map(|b| fonts.measure_rich(&b.text, style, None, true))
            .collect();
        let width = (title.width + 20.)
            .max(buttons.iter().map(|m| m.width + 20.).sum())
            .max(200.)
            .max(style.size * 10.);
        let message_height = self.message.as_ref().map_or(0., |t| {
            13. + fonts.measure_rich(t, style, Some(width), true).height
        });
        let button_height = buttons.iter().map(|m| m.height + 20.).fold(0., f32::max);
        Metrics {
            width,
            height: 28. + title.height + message_height + button_height,
            title,
            buttons,
        }
    }
    pub fn layout(
        &self,
        metrics: &Metrics,
        fonts: &impl TextMeasurer,
        style: TextStyle,
        width: f32,
        height: f32,
        inline: bool,
    ) -> Layout {
        let mut remaining = height - metrics.title.height - 10.;
        let title = (remaining > 0.)
            .then(|| Bounds::new(10., 8., (width - 20.).max(0.), metrics.title.height));
        let y = title.map_or(8., |b| b.y + b.h + 3.);
        let mut message = None;
        if let Some(text) = &self.message
            && remaining > 0.
        {
            let available = width - 20.;
            let measured =
                fonts.measure_rich(text, style, (available > 0.).then_some(available), true);
            remaining -= measured.height + 20.;
            if remaining > 0. {
                message = Some(Bounds::new(
                    10.,
                    y + 10.,
                    available.max(0.),
                    measured.height,
                ));
            }
        }
        let mut out = Layout {
            title,
            message,
            buttons: vec![],
            divider_y: None,
            split: false,
        };
        let Some(left) = metrics.buttons.first() else {
            return out;
        };
        let right = metrics.buttons.get(1);
        let h = right.map_or(left.height, |r| left.height.max(r.height));
        remaining -= h + 10.;
        if remaining <= 0. {
            return out;
        }
        let y = (height - 10. - h + 3.).max(0.);
        let link_y = (height - h - 15.).max(0.);
        let link_height = h + 14.;
        if !inline {
            out.divider_y = Some(height - h - 15.);
            out.split = right.is_some();
        }
        if let Some(right) = right {
            if inline {
                let right_x = width - 20. - right.width;
                let left_x = right_x - 20. - left.width;
                let top = y + (h - right.height) / 2.;
                let link_x = (left_x - 10.).max(0.);
                out.buttons.push(ButtonPlacement {
                    index: 0,
                    text: Bounds::new(left_x, top, left.width, right.height),
                    link: (left_x >= 0.).then(|| {
                        Bounds::new(
                            link_x,
                            link_y,
                            left.width + if link_x < 10. { 10. + link_x } else { 20. },
                            link_height,
                        )
                    }),
                });
                let right_link_x = (right_x - 10.).max(left_x + left.width).max(0.);
                out.buttons.push(ButtonPlacement {
                    index: 1,
                    text: Bounds::new(right_x, top, right.width, right.height),
                    link: Some(Bounds::new(
                        right_link_x,
                        link_y,
                        (width - right_link_x - 2.).max(0.),
                        link_height,
                    )),
                });
            } else {
                let w = ((width - 30.) / 2.).max(0.);
                out.buttons.push(ButtonPlacement {
                    index: 0,
                    text: Bounds::new(8., y + (h - left.height) / 2., w, h),
                    link: Some(Bounds::new(0., link_y, w + 14., link_height)),
                });
                let x = width - 7. - w;
                out.buttons.push(ButtonPlacement {
                    index: 1,
                    text: Bounds::new(x, y + (h - right.height) / 2., w, right.height),
                    link: Some(Bounds::new(x - 7., link_y, w + 14., link_height)),
                });
            }
        } else {
            let (x, w) = if inline {
                let w = left.width + 20.;
                (width - w - 10., w)
            } else {
                let w = left.width.max(width / 2.).min((width - 20.).max(0.));
                ((width - w) / 2., w)
            };
            out.buttons.push(ButtonPlacement {
                index: 0,
                text: Bounds::new(x, y, w, left.height),
                link: Some(Bounds::new(0., link_y, width, link_height)),
            });
        }
        out
    }
}
pub fn labels(source: &str) -> Vec<String> {
    Alert::parse(source)
        .map(|a| a.buttons.into_iter().map(|b| b.source).collect())
        .unwrap_or_default()
}
pub fn apply_text(node: &mut Node, source: &str, baseline: &Properties) -> Result<(), String> {
    Alert::parse(source)?;
    let old = match baseline.get("text") {
        Some(Value::Text(s)) => s.as_str(),
        _ => DEFAULT,
    };
    if let Some(Value::Links(links)) = baseline.get("row_links") {
        let before = labels(old);
        let after = labels(source);
        let before: Vec<_> = before.iter().map(String::as_str).collect();
        let after: Vec<_> = after.iter().map(String::as_str).collect();
        node.properties.insert(
            "row_links".into(),
            Value::Links(crate::list::remap_labels(&before, &after, links)),
        );
    }
    node.properties
        .insert("text".into(), Value::Text(source.into()));
    Ok(())
}
