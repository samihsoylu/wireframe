// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::workspace::PropertySection;

impl WireframeApp {
    pub(super) fn common_section_header(
        &mut self,
        ui: &mut egui::Ui,
        field: &wireframe::workspace::SelectionProperty,
        seen: &mut std::collections::HashSet<PropertySection>,
    ) -> bool {
        if self.editor.editing_instance()
            && let Some(section) = self.editor.selection_property_section(&field.targets)
            && section != PropertySection::Image
            && seen.insert(section)
        {
            return self.component_section_header(
                ui,
                &field.targets.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
                section,
            );
        }
        false
    }
    pub(super) fn component_section_header(
        &mut self,
        ui: &mut egui::Ui,
        ids: &[Id],
        section: PropertySection,
    ) -> bool {
        let overridden = self.editor.has_section_overrides(ids, section);
        let mut reset = false;
        ui.push_id(("component-section", section), |ui| {
            ui.horizontal(|ui| {
                let title = egui::RichText::new(section.label()).strong();
                if overridden {
                    ui.label(title.color(Color32::from_rgb(30, 120, 70)));
                } else {
                    ui.label(title);
                }
                if overridden {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let name = format!("Reset {} to source", section.label());
                        let response = ui.small_button("⊗").on_hover_text(&name);
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name)
                        });
                        reset = response.clicked();
                    });
                }
            });
        });
        if reset && let Err(error) = self.editor.reset_component_section(ids, section) {
            self.error = Some(error);
        }
        reset
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wireframe::document::{Node, Point, Size, Value};
    fn frame(
        app: &mut WireframeApp,
        ctx: &egui::Context,
        shared: bool,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(420., 1800.))),
                events,
                ..Default::default()
            },
            |ui| {
                if shared {
                    app.selection_properties(ui);
                } else {
                    app.typed_properties(ui, 2);
                }
            },
        );
        out.textures_delta.clear();
        out
    }
    #[test]
    fn single_shared_and_arrow_text_section_resets_restore_content_without_touching_color() {
        for (kind, shared) in [("button", false), ("button", true), ("arrow", false)] {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.current_mut().content.nodes = (0..2)
                .map(|i| {
                    Node::control(
                        2 + i,
                        kind,
                        Point::new(50. + i as f32 * 200., 50.),
                        Size::fixed(150., 60.),
                    )
                })
                .collect();
            app.editor.document.next_id = 5;
            for id in [2, 3] {
                app.editor
                    .set_control_property(id, "text", Value::Text("Source".into()))
                    .unwrap();
            }
            app.editor.select_all();
            app.editor.group();
            app.editor.create_component("Pair".into()).unwrap();
            let root = *app.editor.selection.iter().next().unwrap();
            app.editor.enter(root);
            if shared {
                app.editor.select_all();
            } else {
                app.editor.select(2, false);
            }
            for id in [2, 3] {
                app.editor
                    .set_control_property(id, "text", Value::Text("Local".into()))
                    .unwrap();
                app.editor
                    .set_control_property(id, "bold", Value::Bool(true))
                    .unwrap();
            }
            let color = if kind == "arrow" {
                "border_color"
            } else {
                "fill"
            };
            app.editor
                .set_control_property(2, color, Value::Color([255, 0, 0, 255]))
                .unwrap();
            let before = app.editor.document.clone();
            frame(&mut app, &ctx, shared, vec![]);
            let out = frame(&mut app, &ctx, shared, vec![]);
            let node = &out
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .find(|(_, n)| {
                    n.label() == Some("Reset Text to source")
                        && n.role() == egui::accesskit::Role::Button
                })
                .unwrap_or_else(|| panic!("missing {kind} shared={shared}"))
                .1;
            let r = node.bounds().unwrap();
            let at = Pos2::new((r.x0 + r.x1) as f32 / 2., (r.y0 + r.y1) as f32 / 2.);
            for pressed in [true, false] {
                frame(
                    &mut app,
                    &ctx,
                    shared,
                    vec![
                        egui::Event::PointerMoved(at),
                        egui::Event::PointerButton {
                            pos: at,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Modifiers::NONE,
                        },
                    ],
                );
            }
            assert_eq!(
                app.editor.control_properties(2).unwrap().1["text"],
                Value::Text("Source".into())
            );
            assert_eq!(
                app.editor.control_properties(3).unwrap().1["text"],
                Value::Text(if shared { "Source" } else { "Local" }.into())
            );
            assert_eq!(
                app.editor.control_properties(2).unwrap().1[color],
                Value::Color([255, 0, 0, 255])
            );
            app.editor.undo();
            assert_eq!(app.editor.document, before);
        }
    }
    #[test]
    fn scalar_section_buttons_reset_single_and_shared_controls_in_one_step() {
        use wireframe::document::Value;
        for (kind, key, value, section) in [
            (
                "rectangle",
                "border",
                Value::Choice("rounded".into()),
                PropertySection::Border,
            ),
            (
                "tabs",
                "align",
                Value::Choice("right".into()),
                PropertySection::Tabs,
            ),
            (
                "app_bar",
                "icons",
                Value::Icons(vec![Some(wireframe::document::IconSource::Catalog(
                    "star-solid".into(),
                ))]),
                PropertySection::AppBarIcons,
            ),
            (
                "arrow",
                "end_head",
                Value::Bool(false),
                PropertySection::Arrow,
            ),
            (
                "image",
                "sketch_filter",
                Value::Bool(true),
                PropertySection::Image,
            ),
            (
                "cover_flow",
                "sketch_filter",
                Value::Bool(true),
                PropertySection::Image,
            ),
            (
                "horizontal_slider",
                "value",
                Value::Number(0.8),
                PropertySection::Slider,
            ),
            (
                "phone",
                "orientation",
                Value::Choice("landscape".into()),
                PropertySection::Phone,
            ),
            (
                "shape",
                "shape_rotation",
                Value::Number(90.),
                PropertySection::Shape,
            ),
            (
                "search",
                "search_icon",
                Value::Bool(false),
                PropertySection::Search,
            ),
            (
                "icon",
                "icon_rotation",
                Value::Choice("90".into()),
                PropertySection::Icon,
            ),
            (
                "list",
                "row_height",
                Value::Number(35.),
                PropertySection::List,
            ),
            (
                "checkbox",
                "state",
                Value::Choice("selected".into()),
                PropertySection::State,
            ),
        ] {
            for shared in [false, true] {
                let ctx = egui::Context::default();
                ctx.enable_accesskit();
                let mut app = WireframeApp::workspace(&ctx, None);
                app.editor.current_mut().content.nodes = (0..2)
                    .map(|i| {
                        Node::control(
                            2 + i,
                            kind,
                            Point::new(50. + i as f32 * 200., 50.),
                            Size::fixed(150., 60.),
                        )
                    })
                    .collect();
                app.editor.document.next_id = 5;
                let inherited = app.editor.control_properties(2).unwrap().1[key].clone();
                app.editor.select_all();
                app.editor.group();
                app.editor.create_component("Pair".into()).unwrap();
                let root = *app.editor.selection.iter().next().unwrap();
                app.editor.enter(root);
                if shared {
                    app.editor.select_all();
                } else {
                    app.editor.select(2, false);
                }
                for id in [2, 3] {
                    app.editor
                        .set_control_property(id, key, value.clone())
                        .unwrap();
                }
                let before = app.editor.document.clone();
                frame(&mut app, &ctx, shared, vec![]);
                let out = frame(&mut app, &ctx, shared, vec![]);
                let label = format!("Reset {} to source", section.label());
                let node = &out
                    .platform_output
                    .accesskit_update
                    .as_ref()
                    .unwrap()
                    .nodes
                    .iter()
                    .find(|(_, n)| {
                        n.label() == Some(label.as_str())
                            && n.role() == egui::accesskit::Role::Button
                    })
                    .unwrap_or_else(|| panic!("missing {kind}, shared={shared}: {label}"))
                    .1;
                let r = node.bounds().unwrap();
                let at = Pos2::new((r.x0 + r.x1) as f32 / 2., (r.y0 + r.y1) as f32 / 2.);
                for pressed in [true, false] {
                    frame(
                        &mut app,
                        &ctx,
                        shared,
                        vec![
                            egui::Event::PointerMoved(at),
                            egui::Event::PointerButton {
                                pos: at,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: Modifiers::NONE,
                            },
                        ],
                    );
                }
                assert_eq!(
                    app.editor.control_properties(2).unwrap().1[key],
                    inherited,
                    "{kind}, shared={shared}"
                );
                assert_eq!(
                    app.editor.control_properties(3).unwrap().1[key],
                    if shared {
                        inherited.clone()
                    } else {
                        value.clone()
                    },
                    "{kind}, shared={shared}"
                );
                app.editor.undo();
                assert_eq!(app.editor.document, before, "{kind}, shared={shared}");
            }
        }
    }
}
