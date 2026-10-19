// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

fn layer_icon(ui: &mut egui::Ui, command: menus::Command, width: f32) -> egui::Response {
    use menus::Command::*;
    let response = ui.add_sized([width, 24.], egui::Button::new(""));
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), command.label())
    });
    let center = response.rect.center();
    let color = ui.style().interact(&response).text_color();
    let stroke = Stroke::new(1., color);
    let lower = Rect::from_min_size(center + Vec2::new(-7., -3.), Vec2::splat(9.));
    let upper = lower.translate(Vec2::new(3., -3.));
    let background = ui.style().interact(&response).weak_bg_fill;
    let painter = ui.painter();
    painter.rect(
        lower,
        0.,
        if command == SendToBack {
            color
        } else {
            background
        },
        stroke,
        StrokeKind::Inside,
    );
    painter.rect(
        upper,
        0.,
        if command == BringToFront {
            color
        } else {
            background
        },
        stroke,
        StrokeKind::Inside,
    );
    if matches!(command, SendBackward | BringForward) {
        let sign = if command == BringForward { -1. } else { 1. };
        let x = center.x + 1.;
        let y = center.y - 1.;
        painter.line_segment(
            [Pos2::new(x, y - sign * 3.), Pos2::new(x, y + sign * 3.)],
            stroke,
        );
        painter.add(egui::Shape::line(
            vec![
                Pos2::new(x - 2., y + sign),
                Pos2::new(x, y + sign * 3.),
                Pos2::new(x + 2., y + sign),
            ],
            stroke,
        ));
    }
    response.on_hover_text(command.label())
}

impl WireframeApp {
    pub(super) fn layering_properties(&mut self, ui: &mut egui::Ui) {
        use menus::Command::*;
        if self.editor.editing_instance() || self.editor.selection.is_empty() {
            return;
        }
        ui.add_space(12.);
        ui.strong("Layering");
        let gap = 8.;
        let width = ((ui.available_width() - gap) / 4.).floor();
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 0.;
            for (i, (command, caption)) in [
                (SendToBack, "Back"),
                (BringToFront, "Front"),
                (SendBackward, "Backward"),
                (BringForward, "Forward"),
            ]
            .into_iter()
            .enumerate()
            {
                if i == 2 {
                    ui.add_space(gap);
                }
                ui.allocate_ui_with_layout(
                    Vec2::new(width, 42.),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        let enabled = self.command_enabled(ui.ctx(), command);
                        let response = ui
                            .add_enabled_ui(enabled, |ui| layer_icon(ui, command, width))
                            .inner;
                        if response.clicked() {
                            self.run_command(ui.ctx(), command);
                        }
                        ui.small(caption);
                    },
                );
            }
        });
    }
}
