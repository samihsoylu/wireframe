// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Shared editor surface colors; document artwork keeps its own palette.
use super::*;
mod panel_resize;
pub(super) use panel_resize::{
    Edge as PanelEdge, owns_pointer as panel_owns_pointer, prepare as prepare_panel_resize,
};
#[cfg(target_os = "macos")]
mod macos;

pub(super) const TOOLBAR: Color32 = Color32::from_rgb(57, 64, 69);
pub(super) const PANEL: Color32 = Color32::from_gray(238);
pub(super) const PRESENTATION_HEADING: Color32 = Color32::from_gray(224);
pub(super) const HEADING: Color32 = Color32::from_rgb(201, 203, 204);
const INACTIVE_HEADING: Color32 = Color32::from_rgb(224, 225, 225);

pub(super) struct ToolbarPalette {
    pub selected: Color32,
    pub pressed: Color32,
}

impl Default for ToolbarPalette {
    fn default() -> Self {
        Self {
            selected: Color32::from_rgb(61, 155, 232),
            pressed: Color32::from_rgb(177, 215, 245),
        }
    }
}

pub(super) fn toolbar_palette() -> ToolbarPalette {
    #[cfg(target_os = "macos")]
    if let Some(palette) = macos::toolbar_palette() {
        return palette;
    }
    ToolbarPalette::default()
}

pub(super) fn panel_heading(ui: &egui::Ui, menus: &menus::Menus) -> Color32 {
    // A native sheet may take key focus while the document remains the main
    // window. Use that window's state when available, rather than key focus.
    if menus
        .window_is_main()
        .unwrap_or_else(|| ui.input(|i| i.focused))
    {
        HEADING
    } else {
        INACTIVE_HEADING
    }
}

pub(super) fn close_button(ui: &mut egui::Ui, name: &str, help: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(24.), Sense::click());
    let color = ui.style().interact(&response).fg_stroke.color;
    if response.hovered() || response.has_focus() {
        ui.painter()
            .rect_filled(rect, 3, ui.style().interact(&response).bg_fill);
    }
    let center = rect.center();
    for sign in [-1., 1.] {
        ui.painter().line_segment(
            [
                center + Vec2::new(-6., -6. * sign),
                center + Vec2::new(6., 6. * sign),
            ],
            Stroke::new(1., color),
        );
    }
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), name));
    response.on_hover_text(help)
}

pub(super) fn panel_header(
    ui: &mut egui::Ui,
    title: &str,
    fill: Color32,
    actions_width: f32,
    actions: impl FnOnce(&mut egui::Ui),
) {
    let (row, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.), Sense::hover());
    ui.painter().rect_filled(row, 0, fill);
    let label = Rect::from_min_max(
        row.min + Vec2::new(16., 0.),
        row.max - Vec2::new(actions_width + 16., 0.),
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(label)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui_text::label_styled(
                ui,
                title,
                label.width(),
                ui_text::CaptionStyle::panel_heading(),
            );
        },
    );
    let controls = Rect::from_min_max(
        egui::pos2(row.right() - actions_width - 8., row.top()),
        row.max - Vec2::new(8., 0.),
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(controls)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
        actions,
    );
}

#[cfg(test)]
mod panel_resize_tests;
