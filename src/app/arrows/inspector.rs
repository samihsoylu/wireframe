// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::document::{Properties, Value};

pub(in crate::app) fn icon_button(
    ui: &mut egui::Ui,
    name: &str,
    selected: bool,
    width: f32,
) -> egui::Response {
    let mut button = egui::Button::new("").selected(selected);
    if selected {
        button = button.fill(Color32::from_rgb(52, 152, 219));
    }
    let response = ui.add_sized([width, 24.], button);
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), selected, name)
    });
    let rect = Rect::from_center_size(response.rect.center(), Vec2::new(16., 14.));
    let stroke = Stroke::new(
        1.5,
        if selected {
            Color32::WHITE
        } else {
            ui.style().interact(&response).text_color()
        },
    );
    let painter = ui.painter();
    let a = rect.left_top();
    let b = rect.right_bottom();
    match name {
        "Curve" => {
            let points = (0..=16)
                .map(|i| {
                    let t = i as f32 / 16.;
                    Pos2::new(
                        a.x + rect.width() * (2. * t - t * t),
                        a.y + rect.height() * t * t,
                    )
                })
                .collect();
            painter.add(egui::Shape::line(points, stroke));
        }
        "Lines" => {
            painter.add(egui::Shape::line(
                vec![a, Pos2::new(b.x - 2., a.y + 2.), b],
                stroke,
            ));
        }
        "Start arrowhead" | "End arrowhead" => {
            let y = rect.center().y;
            painter.line_segment([Pos2::new(a.x, y), Pos2::new(b.x, y)], stroke);
            let (tip, back) = if name == "Start arrowhead" {
                (a.x, a.x + 5.)
            } else {
                (b.x, b.x - 5.)
            };
            painter.add(egui::Shape::line(
                vec![
                    Pos2::new(back, y - 5.),
                    Pos2::new(tip, y),
                    Pos2::new(back, y + 5.),
                ],
                stroke,
            ));
        }
        "Solid" | "Dashed" | "Dotted" => {
            let y = rect.center().y;
            let (dash, gap) = match name {
                "Dashed" => (3., 2.),
                "Dotted" => (1., 3.),
                _ => (rect.width(), 0.),
            };
            let mut x = a.x;
            while x < b.x {
                painter.line_segment([Pos2::new(x, y), Pos2::new((x + dash).min(b.x), y)], stroke);
                x += dash + gap;
            }
        }
        _ => {}
    }
    response.on_hover_text(name)
}
// Reserve the actual numeric widget width; a fractional-pixel overflow can
// otherwise make a resizable parent panel grow on each subsequent frame.
fn slider_track_width(ui: &egui::Ui, decimals: usize) -> f32 {
    let font = ui.style().drag_value_text_style.resolve(ui.style());
    // A pointer edit changes the value after slider layout. Reserve the full
    // numeric range so its first frame cannot expand the enclosing panel.
    let count = if decimals == 0 { 10 } else { 100 };
    let text_width = (0..=count)
        .map(|n| {
            let value = if decimals == 0 {
                n as f64
            } else {
                n as f64 / 100.
            };
            ui.painter()
                .layout_no_wrap(
                    format!("{value:.decimals$}"),
                    font.clone(),
                    ui.visuals().text_color(),
                )
                .size()
                .x
        })
        .fold(0., f32::max);
    let numeric = (text_width + 2. * ui.spacing().button_padding.x)
        .max(ui.spacing().interact_size.x)
        .ceil();
    (ui.available_width() - numeric - ui.spacing().item_spacing.x)
        .floor()
        .max(45.)
}

fn finish(response: &egui::Response) -> bool {
    response.lost_focus()
        || response.drag_stopped()
        || (response.changed() && !response.dragged() && !response.has_focus())
}
fn number(values: &Properties, key: &str, default: f64) -> f64 {
    match values.get(key) {
        Some(Value::Number(v)) => *v,
        _ => default,
    }
}
pub(in crate::app) fn label_position_widget(ui: &mut egui::Ui, mut value: f64) -> (f64, bool) {
    let mut finish_edit = false;
    ui.horizontal(|ui| {
        let label = ui.label("Label position");

        let mut index = (value - 1.) / 2.;
        ui.spacing_mut().slider_width = slider_track_width(ui, 0);
        let slider = ui
            .add(
                egui::Slider::new(&mut index, 0.0..=4.0)
                    .integer()
                    .show_value(false)
                    .handle_shape(egui::style::HandleShape::Circle)
                    .trailing_fill(true)
                    .clamping(egui::SliderClamping::Never),
            )
            .labelled_by(label.id);
        if slider.clicked() || slider.dragged() {
            slider.request_focus();
        }
        let tick_margin = slider.rect.height() / 2.;
        for tick in 0..5 {
            let x = egui::lerp(
                (slider.rect.left() + tick_margin)..=(slider.rect.right() - tick_margin),
                tick as f32 / 4.,
            );
            ui.painter().circle_filled(
                Pos2::new(x, slider.rect.bottom() + 1.),
                1.,
                ui.visuals().weak_text_color(),
            );
        }
        if slider.changed() {
            value = 1. + 2. * index;
        }
        let numeric = ui
            .add(
                egui::DragValue::new(&mut value)
                    .range(0.0..=10.0)
                    .max_decimals(0)
                    .speed(1.),
            )
            .labelled_by(label.id);
        finish_edit = finish(&slider) || finish(&numeric);
    });
    (value, finish_edit)
}

impl WireframeApp {
    fn arrow_property(&mut self, id: Id, key: &str, original: &Value, value: Value, finish: bool) {
        if value != *original {
            let label = wireframe::catalog::get("arrow")
                .and_then(|s| s.property(key))
                .map_or(key, |f| f.label);
            self.editor.begin(&format!("Edit {label}"));
            if let Err(error) = self.editor.set_control_property(id, key, value) {
                self.error = Some(error);
            }
        }
        if finish {
            self.editor.commit();
        }
    }
    fn arrow_reset_menu(&mut self, ui: &mut egui::Ui, id: Id, keys: &[&str]) {
        let explicit = self.editor.explicit_control_properties(id);
        if !keys.iter().any(|key| explicit.contains_key(*key)) {
            return;
        }
        ui.menu_button("Reset", |ui| {
            for &key in keys {
                if !explicit.contains_key(key) {
                    continue;
                }
                let label = wireframe::catalog::get("arrow")
                    .and_then(|s| s.property(key))
                    .map_or(key, |f| f.label);
                if ui.button(format!("Reset {label}")).clicked() {
                    self.editor.commit();
                    self.editor.begin(&format!("Reset {label}"));
                    if let Err(error) = self.editor.reset_control_property(id, key) {
                        self.error = Some(error);
                    }
                    self.editor.commit();
                    ui.close();
                }
            }
        })
        .response
        .on_hover_text(if self.editor.editing_instance() {
            "Use component source values"
        } else {
            "Use default values"
        });
    }
    fn arrow_section(&mut self, ui: &mut egui::Ui, id: Id, name: &str, keys: &[&str]) {
        if self.editor.editing_instance() && matches!(name, "Text" | "Arrow") {
            ui.add_space(10.);
            let section = if name == "Text" {
                wireframe::workspace::PropertySection::Text
            } else {
                wireframe::workspace::PropertySection::Arrow
            };
            self.component_section_header(ui, &[id], section);
            return;
        }
        let explicit = self.editor.explicit_control_properties(id);
        let any = keys.iter().any(|key| explicit.contains_key(*key));
        ui.add_space(10.);
        ui.horizontal(|ui| {
            if any && self.editor.editing_instance() {
                ui.colored_label(
                    Color32::from_rgb(30, 120, 70),
                    egui::RichText::new(name).strong(),
                );
            } else {
                ui.strong(name);
            }
            self.arrow_reset_menu(ui, id, keys);
        });
    }
    pub(in crate::app) fn arrow_properties(
        &mut self,
        ui: &mut egui::Ui,
        id: Id,
        values: &Properties,
    ) {
        ui.push_id(("arrow-inspector", id), |ui| {
            ui.visuals_mut().selection.bg_fill = Color32::from_rgb(52, 152, 219);
            self.arrow_section(
                ui,
                id,
                "Arrow",
                &["shape", "start_head", "end_head", "label_position"],
            );
            ui.horizontal(|ui| {
                ui.label("Options");
                ui.spacing_mut().item_spacing.x = 3.;
                for (name, key, value) in [
                    ("Curve", "shape", Value::Choice("curve".into())),
                    ("Lines", "shape", Value::Choice("lines".into())),
                    ("Start arrowhead", "start_head", Value::Bool(true)),
                    ("End arrowhead", "end_head", Value::Bool(true)),
                ] {
                    let original = &values[key];
                    let selected = original == &value;
                    if icon_button(ui, name, selected, 32.).clicked() {
                        self.editor.commit();
                        let next = if key == "shape" {
                            value
                        } else {
                            Value::Bool(!selected)
                        };
                        self.arrow_property(id, key, original, next, true);
                    }
                }
            });
            let (value, finished) = label_position_widget(ui, number(values, "label_position", 5.));
            self.arrow_property(
                id,
                "label_position",
                &values["label_position"],
                Value::Number(value),
                finished,
            );
            if self.editor.editing_instance() {
                self.component_section_header(
                    ui,
                    &[id],
                    wireframe::workspace::PropertySection::Color,
                );
            }
            ui.add_space(10.);
            ui.horizontal(|ui| {
                let explicit = self.editor.explicit_control_properties(id);
                let label =
                    if self.editor.editing_instance() && explicit.contains_key("border_color") {
                        ui.colored_label(
                            Color32::from_rgb(30, 120, 70),
                            egui::RichText::new("Color").strong(),
                        )
                    } else {
                        ui.strong("Color")
                    };
                if let Value::Color(original) = values["border_color"] {
                    let mut rgb = [original[0], original[1], original[2]];
                    let r = ui
                        .scope(|ui| {
                            ui.spacing_mut().interact_size.x = 64.;
                            ui.color_edit_button_srgb(&mut rgb)
                        })
                        .inner
                        .labelled_by(label.id);
                    self.arrow_property(
                        id,
                        "border_color",
                        &values["border_color"],
                        Value::Color([rgb[0], rgb[1], rgb[2], original[3]]),
                        finish(&r),
                    );
                }
                if !self.editor.editing_instance() {
                    self.arrow_reset_menu(ui, id, &["border_color", "background_opacity"]);
                }
            });
            ui.horizontal(|ui| {
                let label = ui.label("Opacity");
                let mut value = number(values, "background_opacity", 1.);
                ui.spacing_mut().slider_width = slider_track_width(ui, 2);
                let r = ui
                    .add(
                        egui::Slider::new(&mut value, 0.0..=1.0)
                            .fixed_decimals(2)
                            .handle_shape(egui::style::HandleShape::Circle)
                            .trailing_fill(true),
                    )
                    .labelled_by(label.id);
                if r.clicked() || r.dragged() {
                    r.request_focus();
                }
                self.arrow_property(
                    id,
                    "background_opacity",
                    &values["background_opacity"],
                    Value::Number(value),
                    finish(&r),
                );
            });
            self.arrow_section(ui, id, "Stroke", &["stroke"]);
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 2. * ui.spacing().item_spacing.x) / 3.;
                for (name, value) in [
                    ("Solid", "solid"),
                    ("Dashed", "dashed"),
                    ("Dotted", "dotted"),
                ] {
                    let value = Value::Choice(value.into());
                    if icon_button(ui, name, values["stroke"] == value, width).clicked() {
                        self.editor.commit();
                        self.arrow_property(id, "stroke", &values["stroke"], value, true);
                    }
                }
            });
            self.arrow_section(
                ui,
                id,
                "Text",
                &["bold", "italic", "underline", "font_size", "text"],
            );
            ui.horizontal(|ui| {
                for (key, label, text) in [
                    ("bold", "Bold", egui::RichText::new("B").strong()),
                    ("italic", "Italic", egui::RichText::new("I").italics()),
                    (
                        "underline",
                        "Underline",
                        egui::RichText::new("U").underline(),
                    ),
                ] {
                    let selected = values[key] == Value::Bool(true);
                    let mut button = egui::Button::new(if selected {
                        text.color(Color32::WHITE)
                    } else {
                        text
                    })
                    .selected(selected);
                    if selected {
                        button = button.fill(Color32::from_rgb(52, 152, 219));
                    }
                    let r = ui.add_sized([30., 24.], button);
                    r.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::Checkbox,
                            ui.is_enabled(),
                            selected,
                            label,
                        )
                    });
                    if r.on_hover_text(label).clicked() {
                        self.editor.commit();
                        self.arrow_property(id, key, &values[key], Value::Bool(!selected), true);
                    }
                }
                let label = ui.label("Size");
                let mut size = number(values, "font_size", 13.);
                let r = ui
                    .add(egui::DragValue::new(&mut size).range(6.0..=120.0).speed(1.))
                    .labelled_by(label.id);
                self.arrow_property(
                    id,
                    "font_size",
                    &values["font_size"],
                    Value::Number(size),
                    finish(&r),
                );
            });
            egui::CollapsingHeader::new("Edit caption")
                .id_salt((id, "caption"))
                .show(ui, |ui| {
                    if let Value::Text(stored) = &values["text"] {
                        match self.editor.control_text_for_edit(id, stored) {
                            Ok(mut displayed) => {
                                let before = displayed.clone();
                                let r =
                                    wireframe::text_entry::multiline(ui, &mut displayed, 4, false);
                                if displayed != before {
                                    match self.editor.control_text_from_edit(id, &displayed) {
                                        Ok(next) => self.arrow_property(
                                            id,
                                            "text",
                                            &values["text"],
                                            Value::Text(next),
                                            r.lost_focus(),
                                        ),
                                        Err(error) => self.error = Some(error),
                                    }
                                } else if r.lost_focus() {
                                    self.editor.commit();
                                }
                            }
                            Err(error) => self.error = Some(error),
                        }
                    }
                });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(
        app: &mut WireframeApp,
        ctx: &egui::Context,
        id: Id,
        width: f32,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 900.))),
                events,
                ..Default::default()
            },
            |ui| {
                app.shortcuts(ui.ctx());
                assert!(app.typed_properties(ui, id));
                assert!(
                    ui.min_rect().right() <= width + 1.,
                    "inspector overflow: {:?}",
                    ui.min_rect()
                );
            },
        );
        out.textures_delta.clear();
        out
    }
    fn center(out: &egui::FullOutput, label: &str) -> Pos2 {
        let n = out
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some(label))
            .unwrap_or_else(|| panic!("missing {label}"));
        let r = n.1.bounds().unwrap();
        Pos2::new((r.x0 + r.x1) as f32 / 2., (r.y0 + r.y1) as f32 / 2.)
    }
    fn click(app: &mut WireframeApp, ctx: &egui::Context, id: Id, at: Pos2) -> egui::FullOutput {
        frame(
            app,
            ctx,
            id,
            300.,
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
        frame(
            app,
            ctx,
            id,
            300.,
            vec![egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            }],
        )
    }
    #[test]
    fn compact_arrow_inspector_fits_widths_and_exposes_named_controls() {
        for width in [220., 300., 400.] {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(Kind::Arrow, 0., 0.);
            let id = *app.editor.selection.iter().next().unwrap();
            let before = app.editor.document.clone();
            let out = frame(&mut app, &ctx, id, width, vec![]);
            for label in [
                "Curve",
                "Lines",
                "Start arrowhead",
                "End arrowhead",
                "Solid",
                "Dashed",
                "Dotted",
                "Bold",
                "Italic",
                "Underline",
            ] {
                center(&out, label);
            }
            assert_eq!(
                app.editor.document, before,
                "drawing the inspector must not normalize defaults"
            );
        }
    }
    #[test]
    fn compact_arrow_head_toggle_and_shape_change_have_independent_undo() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Arrow, 20., 30.);
        let id = *app.editor.selection.iter().next().unwrap();
        let before = app.editor.document.clone();
        let out = frame(&mut app, &ctx, id, 300., vec![]);
        click(&mut app, &ctx, id, center(&out, "End arrowhead"));
        let changed = app.editor.document.clone();
        let values = app.editor.control_properties(id).unwrap().1;
        assert_eq!(values["end_head"], Value::Bool(false));
        assert_eq!(values["label_position"], Value::Number(8.));
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        app.editor.redo();
        assert_eq!(app.editor.document, changed);
        let out = frame(&mut app, &ctx, id, 300., vec![]);
        click(&mut app, &ctx, id, center(&out, "Lines"));
        assert_eq!(
            app.editor.control_properties(id).unwrap().1["shape"],
            Value::Choice("lines".into())
        );
        app.editor.undo();
        assert_eq!(app.editor.document, changed);
    }
    #[test]
    fn compact_caption_slider_keeps_a_held_edit_in_one_history_action() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Arrow, 20., 30.);
        let id = *app.editor.selection.iter().next().unwrap();
        let before = app.editor.document.clone();
        let out = frame(&mut app, &ctx, id, 300., vec![]);
        let nodes = &out.platform_output.accesskit_update.as_ref().unwrap().nodes;
        let slider = nodes
            .iter()
            .find(|(_, n)| {
                n.role() == egui::accesskit::Role::Slider
                    && n.labelled_by().iter().any(|id| {
                        nodes.iter().any(|(other, node)| {
                            other == id
                                && node.label().or_else(|| node.value()) == Some("Label position")
                        })
                    })
            })
            .unwrap();
        let r = slider.1.bounds().unwrap();
        let start = Pos2::new((r.x0 + r.x1) as f32 / 2., (r.y0 + r.y1) as f32 / 2.);
        let end = Pos2::new(r.x1 as f32 - 2., start.y);
        frame(
            &mut app,
            &ctx,
            id,
            300.,
            vec![
                egui::Event::PointerMoved(start),
                egui::Event::PointerButton {
                    pos: start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
        frame(
            &mut app,
            &ctx,
            id,
            300.,
            vec![egui::Event::PointerMoved(start.lerp(end, 0.6))],
        );
        frame(
            &mut app,
            &ctx,
            id,
            300.,
            vec![egui::Event::PointerMoved(end)],
        );
        frame(
            &mut app,
            &ctx,
            id,
            300.,
            vec![egui::Event::PointerButton {
                pos: end,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            }],
        );
        let changed = app.editor.document.clone();
        assert_eq!(
            app.editor.control_properties(id).unwrap().1["label_position"],
            Value::Number(9.)
        );
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        app.editor.redo();
        assert_eq!(app.editor.document, changed);
    }
    #[test]
    fn clicked_opacity_slider_owns_arrow_keys_without_moving_selection() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Arrow, 20., 30.);
        let id = *app.editor.selection.iter().next().unwrap();
        let out = frame(&mut app, &ctx, id, 300., vec![]);
        let nodes = &out.platform_output.accesskit_update.as_ref().unwrap().nodes;
        let slider = nodes
            .iter()
            .find(|(_, n)| {
                n.role() == egui::accesskit::Role::Slider
                    && n.labelled_by().iter().any(|id| {
                        nodes.iter().any(|(other, node)| {
                            other == id && node.label().or_else(|| node.value()) == Some("Opacity")
                        })
                    })
            })
            .unwrap();
        let r = slider.1.bounds().unwrap();
        let at = Pos2::new((r.x0 + r.x1) as f32 / 2., (r.y0 + r.y1) as f32 / 2.);
        click(&mut app, &ctx, id, at);
        frame(&mut app, &ctx, id, 300., vec![]);
        let before = app.editor.document.clone();
        let opacity = app.editor.control_properties(id).unwrap().1["background_opacity"].clone();
        frame(
            &mut app,
            &ctx,
            id,
            300.,
            vec![egui::Event::Key {
                key: Key::ArrowLeft,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
        let after = app.editor.document.clone();
        assert_eq!(
            before.pages[0].content.nodes[0].origin,
            after.pages[0].content.nodes[0].origin
        );
        let next = app.editor.control_properties(id).unwrap().1["background_opacity"].clone();
        assert!(matches!((opacity, next), (Value::Number(a), Value::Number(b)) if b < a));
    }
}
