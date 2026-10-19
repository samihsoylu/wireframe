// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

fn icon(ui: &mut egui::Ui, command: menus::Command, width: f32) -> egui::Response {
    use wireframe::workspace::AlignmentAction::*;
    let action = command.alignment_action().unwrap();
    let response = ui.add_sized([width, 24.], egui::Button::new(""));
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), command.label())
    });
    let center = response.rect.center();
    let stroke = Stroke::new(1., ui.style().interact(&response).text_color());
    let point = |x, y| {
        center
            + if action.vertical() {
                Vec2::new(y, x)
            } else {
                Vec2::new(x, y)
            }
    };
    let line = |a: (f32, f32), b: (f32, f32)| {
        ui.painter()
            .line_segment([point(a.0, a.1), point(b.0, b.1)], stroke);
    };
    if matches!(action, Left | Center | Right | Top | Middle | Bottom) {
        let edge = match action {
            Left | Top => -7.,
            Right | Bottom => 7.,
            _ => 0.,
        };
        line((edge, -8.), (edge, 8.));
        for (y, w) in [(-5., 6.), (1., 10.)] {
            let x = match action {
                Left | Top => edge + 2.,
                Right | Bottom => edge - w - 2.,
                _ => -w / 2.,
            };
            for (a, b) in [
                ((x, y), (x + w, y)),
                ((x + w, y), (x + w, y + 3.)),
                ((x + w, y + 3.), (x, y + 3.)),
                ((x, y + 3.), (x, y)),
            ] {
                line(a, b);
            }
        }
    } else {
        let spacing = matches!(action, SpaceHorizontal | SpaceVertical);
        for (i, (x, h)) in [(-6., 10.), (0., 15.), (6., 7.)].into_iter().enumerate() {
            let h = if spacing { 12. } else { h };
            for (a, b) in [
                ((x - 1., -h / 2.), (x + 1., -h / 2.)),
                ((x + 1., -h / 2.), (x + 1., h / 2.)),
                ((x + 1., h / 2.), (x - 1., h / 2.)),
                ((x - 1., h / 2.), (x - 1., -h / 2.)),
            ] {
                line(a, b);
            }
            if spacing && i < 2 {
                line((x + 2., -2.), (x + 4., -2.));
                line((x + 2., 2.), (x + 4., 2.));
            }
            if !spacing {
                line((x, -8.), (x, 8.));
            }
        }
    }
    response.on_hover_text(command.label())
}
impl WireframeApp {
    pub(super) fn arrangement_properties(&mut self, ui: &mut egui::Ui) {
        use menus::Command::*;
        if !self.editor.can_align_selection() {
            return;
        }
        ui.add_space(10.);
        let gap = 12.;
        let width = ((ui.available_width() - gap) / 6.).floor();
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 0.;
            for (i, (command, label)) in [
                (AlignLeft, "Left"),
                (AlignCenter, "Center"),
                (AlignRight, "Right"),
                (AlignTop, "Top"),
                (AlignMiddle, "Middle"),
                (AlignBottom, "Bottom"),
            ]
            .into_iter()
            .enumerate()
            {
                if i == 3 {
                    ui.add_space(gap);
                }
                ui.allocate_ui_with_layout(
                    Vec2::new(width, 42.),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        ui.set_min_width(width);
                        ui.spacing_mut().item_spacing.y = 2.;
                        if ui
                            .add_enabled_ui(self.command_enabled(ui.ctx(), command), |ui| {
                                icon(ui, command, width)
                            })
                            .inner
                            .clicked()
                        {
                            self.run_command(ui.ctx(), command);
                        }
                        ui.small(label);
                    },
                );
            }
        });
        ui.add_space(6.);
        let group_width = ((ui.available_width() - gap) / 2.).floor();
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (commands, label) in [
                (
                    [DistributeHorizontal, DistributeVertical],
                    "Distribute Centers",
                ),
                ([SpaceHorizontal, SpaceVertical], "Space Out"),
            ] {
                ui.allocate_ui_with_layout(
                    Vec2::new(group_width, 42.),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        ui.set_min_width(group_width);
                        ui.spacing_mut().item_spacing.y = 2.;
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.;
                            for command in commands {
                                if ui
                                    .add_enabled_ui(self.command_enabled(ui.ctx(), command), |ui| {
                                        icon(ui, command, group_width / 2.)
                                    })
                                    .inner
                                    .clicked()
                                {
                                    self.run_command(ui.ctx(), command);
                                }
                            }
                        });
                        ui.small(label);
                    },
                );
            }
        });
    }
    pub(super) fn auto_size_properties(&mut self, ui: &mut egui::Ui) {
        if !self.editor.can_auto_size_selection(true) {
            return;
        }
        ui.add_space(6.);
        let left = (ui.available_width() * 0.4).floor() + 8.;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.;
            ui.add_space(left);
            let response = ui
                .add_enabled_ui(
                    self.command_enabled(ui.ctx(), menus::Command::AutoSize),
                    |ui| {
                        ui.add_sized(
                            [ui.available_width(), 24.],
                            egui::Button::new("↔ Auto-size"),
                        )
                    },
                )
                .inner;
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "Auto-size")
            });
            if response.clicked() {
                self.run_command(ui.ctx(), menus::Command::AutoSize);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arrangement_controls_fit_inspector_widths_and_expose_every_action() {
        for (width, density) in [
            (220., 1.),
            (300., 1.),
            (400., 1.),
            (220., 2.),
            (300., 2.),
            (400., 2.),
        ] {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            let mut app = WireframeApp::workspace(&ctx, None);
            for x in [0., 33., 200.] {
                app.editor.add(Kind::Button, x, 50.);
            }
            app.editor.select_all();
            let before = app.editor.document.clone();
            ctx.data_mut(|d| {
                d.insert_persisted(
                    egui::Id::new("inspector"),
                    egui::containers::panel::PanelState {
                        outer_rect: Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 900.)),
                    },
                )
            });
            for _ in 0..4 {
                let mut raw = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1360., 900.))),
                    ..Default::default()
                };
                raw.viewports
                    .get_mut(&egui::ViewportId::ROOT)
                    .unwrap()
                    .native_pixels_per_point = Some(density);
                let mut out = ctx.run_ui(raw, |ui| {
                    let right = ui.available_rect_before_wrap().right();
                    app.inspector(ui);
                    assert!(
                        (right - ui.available_rect_before_wrap().right() - width).abs() < 0.1,
                        "inspector width grew"
                    );
                });
                let nodes = &out.platform_output.accesskit_update.as_ref().unwrap().nodes;
                for &command in menus::Command::ALIGNMENT
                    .iter()
                    .chain(std::iter::once(&menus::Command::AutoSize))
                {
                    assert!(
                        nodes
                            .iter()
                            .any(|(_, n)| n.role() == egui::accesskit::Role::Button
                                && n.label() == Some(command.label())),
                        "missing {}",
                        command.label()
                    );
                }
                out.textures_delta.clear();
            }
            assert!(app.editor.document == before);
        }
    }
    #[test]
    fn arrangement_shortcuts_have_one_undo_and_respect_text_and_navigator_focus() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        for x in [0., 33., 200.] {
            app.editor.add(Kind::Button, x, 50.);
        }
        app.editor.select_all();
        let before = app.editor.document.clone();
        let frame = |app: &mut WireframeApp| {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key: Key::Num3,
                        physical_key: Some(Key::Num3),
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::COMMAND | Modifiers::ALT,
                    }],
                    ..Default::default()
                },
                |ui| app.shortcuts(ui.ctx()),
            );
            out.textures_delta.clear();
        };
        frame(&mut app);
        assert!(app.editor.document != before);
        app.editor.undo();
        assert!(app.editor.document == before);
        let field = egui::Id::new("arrangement-page-focus");
        app.navigator.focus_ids.insert(field);
        ctx.memory_mut(|m| m.request_focus(field));
        frame(&mut app);
        assert!(app.editor.document == before);
        ctx.memory_mut(|m| m.surrender_focus(field));
        let mut draft = "Draft".to_owned();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.add(TextInput::singleline(&mut draft).autofocus(true));
        });
        out.textures_delta.clear();
        app.run_command(&ctx, menus::Command::AlignRight);
        app.run_command(&ctx, menus::Command::AutoSize);
        assert!(app.editor.document == before);
    }
}
