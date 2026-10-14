// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use eframe::egui::{self, Color32};
use wireframe::{document::IconSource, icons};

pub(super) fn choice(ui: &mut egui::Ui, value: &mut Option<IconSource>) {
    let title = match value {
        Some(IconSource::Catalog(id)) => icons::get(id).map_or_else(
            || format!("Unavailable: {id}"),
            |icon| format!("{} ({})", icon.label, icon.style),
        ),
        None => "Choose icon…".into(),
    };
    ui.horizontal(|ui| {
        if let Some(IconSource::Catalog(id)) = value
            && let Some(icon) = icons::get(id)
        {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(22., 22.), egui::Sense::hover());
            icon.paint(ui.painter(), rect, ui.visuals().text_color());
        }
        egui::containers::menu::MenuButton::new(title)
            .config(
                egui::containers::menu::MenuConfig::new()
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
            )
            .ui(ui, |ui| {
                ui.set_min_width(330.);
                if ui.button("No icon").clicked() {
                    *value = None;
                    ui.close();
                }
                let query_key = ui.make_persistent_id("icon-query");
                let style_key = ui.make_persistent_id("icon-style");
                let mut query = ui
                    .ctx()
                    .data_mut(|d| d.get_temp::<String>(query_key))
                    .unwrap_or_default();
                let mut style = ui
                    .ctx()
                    .data_mut(|d| d.get_temp::<String>(style_key))
                    .unwrap_or_default();
                ui.add(
                    egui::TextEdit::singleline(&mut query)
                        .hint_text("Search icons…")
                        .desired_width(330.),
                );
                ui.horizontal(|ui| {
                    for (name, key) in [
                        ("All", ""),
                        ("Solid", "solid"),
                        ("Regular", "regular"),
                        ("Brands", "brands"),
                    ] {
                        ui.selectable_value(&mut style, key.to_owned(), name);
                    }
                });
                let found = icons::search(&query, &style);
                ui.label(format!("{} icons", found.len()));
                egui::ScrollArea::vertical().max_height(300.).show_rows(
                    ui,
                    56.,
                    found.len().div_ceil(5),
                    |ui, rows| {
                        for row in rows {
                            ui.horizontal(|ui| {
                            for icon in found.iter().skip(row * 5).take(5) {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(58., 52.),
                                    egui::Sense::click(),
                                );
                                let selected =
                                    matches!(value,Some(IconSource::Catalog(id)) if id==&icon.id);
                                if selected || response.hovered() {
                                    ui.painter().rect_filled(
                                        rect,
                                        4.,
                                        ui.visuals().selection.bg_fill,
                                    );
                                }
                                icon.paint(
                                    ui.painter(),
                                    rect.shrink(12.),
                                    if selected || response.hovered() {
                                        ui.visuals().selection.stroke.color
                                    } else {
                                        ui.visuals().text_color()
                                    },
                                );
                                let label = format!("{} ({})", icon.label, icon.style);
                                response.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::Button,
                                        true,
                                        &label,
                                    )
                                });
                                if response.on_hover_text(label).clicked() {
                                    *value = Some(IconSource::Catalog(icon.id.clone()));
                                    ui.close();
                                }
                            }
                        });
                        }
                    },
                );
                if found.is_empty() {
                    ui.colored_label(Color32::GRAY, "No matching icons");
                }
                ui.small("Font Awesome Free · CC BY 4.0");
                ui.ctx().data_mut(|d| {
                    d.insert_temp(query_key, query);
                    d.insert_temp(style_key, style);
                });
            });
        if value.is_some() && ui.small_button("×").on_hover_text("Remove icon").clicked() {
            *value = None;
        }
    });
}
