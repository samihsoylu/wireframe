// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::{
    layout::{EguiMeasurer, TextMeasurer, TextStyle},
    snapping::{Axis, Kind as GuideKind, Targets},
};

impl Editor {
    /// Snapshot sibling targets at gesture start. Coordinates are global even
    /// inside a group; selection editing converts them back to parent space.
    pub fn alignment_targets(&self, gravity: f32) -> Result<Targets, String> {
        let layout = self.layout_page(self.surface_id())?;
        let nodes = if self.context.is_empty() {
            &layout.nodes
        } else {
            &view::find_layout(&layout.nodes, &self.context)
                .ok_or("Editing group no longer exists")?
                .children
        };
        let mut result = Ok(Targets::new(gravity));
        let output = self.fonts.run_ui(eframe::egui::RawInput::default(), |ui| {
            let fonts = EguiMeasurer {
                context: ui.ctx(),
                regular: crate::fonts::family(false, false),
                bold: crate::fonts::family(true, false),
            };
            result = (|| {
                let mut targets = Targets::new(gravity);
                for node in nodes {
                    let c = view::control(node);
                    if self.selection.contains(&c.id) {
                        continue;
                    }
                    let b = c.bounds;
                    let p = &node.properties;
                    let sketch = self.document.settings.sketch;
                    let number = |key, default| match p.get(key) {
                        Some(Value::Number(v)) => *v as f32,
                        _ => default,
                    };
                    let style = TextStyle {
                        size: c.font_size,
                        bold: c.bold,
                        italic: matches!(p.get("italic"), Some(Value::Bool(true))),
                    };
                    if c.group.is_some() {
                        targets.frame(b, false);
                        continue;
                    }
                    match c.kind {
                        Kind::FieldSet => {
                            let title = crate::field_set::parse(&c.text)?;
                            let metrics = fonts.measure_rich(&title, style, None, true);
                            targets.edges(b);
                            targets.add(Axis::X, GuideKind::Spacing, b.x + 16.);
                            targets.add(Axis::X, GuideKind::Spacing, b.x + b.w - 16.);
                            targets.add(
                                Axis::Y,
                                GuideKind::Spacing,
                                b.y + 16. + metrics.height / 2.,
                            );
                            targets.add(Axis::Y, GuideKind::Spacing, b.y + b.h - 16.);
                        }
                        Kind::Panel => {
                            let title = crate::window::parse(&c.text)?;
                            let layout = crate::window::Options {
                                header: number("header_height", 26.),
                                footer: number("footer_height", 25.),
                                ..Default::default()
                            }
                            .layout(
                                b.w,
                                b.h,
                                fonts.measure_rich(&title, style, None, true).height,
                                sketch,
                            );
                            targets.edges(b);
                            targets.add(Axis::X, GuideKind::Spacing, b.x + 16.);
                            targets.add(Axis::X, GuideKind::Spacing, b.x + b.w - 16.);
                            targets.add(Axis::Y, GuideKind::Spacing, b.y + layout.header.h + 16.);
                            if let Some(footer) = layout.footer {
                                targets.add(
                                    Axis::Y,
                                    GuideKind::Spacing,
                                    b.y + b.h - footer.h - 16.,
                                );
                            }
                        }
                        Kind::Browser => {
                            let s = crate::browser::Style::new(sketch);
                            targets.edges(b);
                            targets.add(Axis::X, GuideKind::Spacing, b.x + 16.);
                            targets.add(Axis::X, GuideKind::Spacing, b.x + b.w - 16.);
                            targets.add(Axis::Y, GuideKind::Spacing, b.y + 16. + s.content_top);
                            targets.add(Axis::Y, GuideKind::Spacing, b.y + b.h - 16. - s.bottom);
                        }
                        Kind::Tabs | Kind::VerticalTabs => {
                            let vertical = c.kind == Kind::VerticalTabs;
                            let tabs = crate::tabs::Tabs::parse(&c.text, vertical)?;
                            let align = match p.get("align") {
                                Some(Value::Choice(s)) => s.as_str(),
                                _ => "left",
                            };
                            let body = tabs
                                .measure(&fonts, style, sketch)
                                .place(
                                    b,
                                    crate::tabs::Position::from_properties(p, vertical),
                                    align,
                                )
                                .content;
                            targets.edges(b);
                            targets.add(Axis::X, GuideKind::Spacing, body.x + 16.);
                            targets.add(Axis::X, GuideKind::Spacing, body.x + body.w - 16.);
                            targets.add(Axis::Y, GuideKind::Spacing, body.y + 16.);
                            targets.add(Axis::Y, GuideKind::Spacing, body.y + body.h - 16.);
                        }
                        _ => targets.frame(b, c.kind == Kind::Rectangle),
                    }
                }
                Ok(targets)
            })();
        });
        output.drop_without_applying_deltas();
        result
    }
}
