// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
#[cfg(test)]
mod tests;

const FONT_SIZE: f32 = 11.;
const BUTTON_HEIGHT: f32 = 24.;
const GAP: f32 = 4.;

pub(super) fn show(
    ui: &mut egui::Ui,
    selected: &[CatalogCategory],
) -> Option<(CatalogCategory, bool)> {
    let categories = CatalogCategory::visible(selected);
    let widths: Vec<f32> = categories
        .iter()
        .map(|category| {
            let text = ui.painter().layout_no_wrap(
                category.name().to_owned(),
                FontId::proportional(FONT_SIZE),
                Color32::BLACK,
            );
            (text.size().x * 2.).ceil() / 2. + 16.
        })
        .collect();
    let group_width = widths.iter().sum::<f32>() + GAP * categories.len().saturating_sub(1) as f32;
    let mut chosen = None;
    egui::Frame::NONE
        .fill(chrome::HEADING)
        .inner_margin(egui::Margin::symmetric(6, 3))
        .show(ui, |ui| {
            let leading = ((ui.available_width() - group_width) / 2.).max(0.);
            egui::ScrollArea::horizontal()
                .id_salt("library-categories")
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    ui.horizontal(|ui| {
                        ui.set_height(BUTTON_HEIGHT);
                        if leading > 0. {
                            ui.add_space(leading);
                        }
                        for (category, width) in categories.into_iter().zip(widths) {
                            let response = ui
                                .push_id(category.name(), |ui| {
                                    button(ui, category, selected.contains(&category), width)
                                })
                                .inner;
                            if response.gained_focus() {
                                response.scroll_to_me(Some(egui::Align::Center));
                            }
                            if response.clicked() {
                                chosen = Some((category, activation::click_modifiers(ui).shift));
                            }
                        }
                    });
                });
        });
    chosen
}

fn button(
    ui: &mut egui::Ui,
    category: CatalogCategory,
    selected: bool,
    width: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, BUTTON_HEIGHT), Sense::click());
    let fill = if selected {
        Color32::from_rgb(55, 155, 228)
    } else {
        Color32::from_gray(188)
    };
    ui.painter().rect_filled(rect, 4., fill);
    if response.has_focus() || response.is_pointer_button_down_on() {
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            4.,
            Stroke::new(1., Color32::from_rgb(25, 90, 145)),
            egui::StrokeKind::Inside,
        );
    }
    ui_text::paint(
        ui,
        rect.shrink2(Vec2::new(8., 0.)),
        category.name(),
        FONT_SIZE,
        if selected {
            Color32::WHITE
        } else {
            Color32::from_gray(35)
        },
        true,
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Button,
            ui.is_enabled(),
            selected,
            category.name(),
        )
    });
    response.on_hover_text("Choose a category. Shift-click to combine categories.")
}
