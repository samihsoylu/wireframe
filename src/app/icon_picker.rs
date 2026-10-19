// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use eframe::egui::{self, Color32};
use wireframe::{
    document::{Asset, IconSource},
    icons,
};
use wireframe::{text_entry::TextInput, ui_text};

pub(super) fn choice(ui: &mut egui::Ui, value: &mut Option<IconSource>, assets: &[Asset]) -> bool {
    let mut import = false;
    let title = match value {
        Some(IconSource::Catalog(id)) => icons::get(id).map_or_else(
            || format!("Unavailable: {id}"),
            |icon| format!("{} ({})", icon.label, icon.style),
        ),
        Some(IconSource::Asset(id)) => assets
            .iter()
            .find(|a| a.id == *id && !a.trashed)
            .map_or_else(
                || "Unavailable image".into(),
                |a| format!("Image: {}", a.name),
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
        if let Some(IconSource::Asset(id)) = value
            && let Some(asset) = assets.iter().find(|a| a.id == *id && !a.trashed)
        {
            preview(ui, asset, 22.);
        }
        egui::containers::menu::MenuButton::new(title)
            .config(
                egui::containers::menu::MenuConfig::new()
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
            )
            .ui(ui, |ui| {
                import = menu_contents(ui, value, assets);
            });
        if value.is_some() && ui.small_button("×").on_hover_text("Remove icon").clicked() {
            *value = None;
        }
    });
    import
}

/// A bounded, single-row search trigger used by collection inspectors.
pub(super) fn compact(
    ui: &mut egui::Ui,
    value: &mut Option<IconSource>,
    assets: &[Asset],
    caption: &str,
    width: f32,
) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width.max(40.), 24.), egui::Sense::click());
    ui.painter()
        .rect_filled(rect, 12., Color32::from_rgb(250, 250, 250));
    ui.painter().rect_stroke(
        rect,
        12.,
        egui::Stroke::new(1., Color32::from_gray(210)),
        egui::StrokeKind::Inside,
    );
    if response.clicked() {
        response.request_focus();
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect,
            12.,
            egui::Stroke::new(1.5, ui.visuals().selection.bg_fill),
            egui::StrokeKind::Inside,
        );
    }
    let mut text_rect = response.rect.shrink2(egui::vec2(5., 0.));
    text_rect.min.x += 16.;
    if !ui.is_sizing_pass() {
        ui_text::paint(
            ui,
            text_rect,
            caption,
            egui::TextStyle::Body.resolve(ui.style()).size,
            ui.visuals().text_color(),
            false,
        );
    }
    let center = response.rect.left_center() + egui::vec2(10., -1.);
    let stroke = egui::Stroke::new(1., ui.visuals().weak_text_color());
    ui.painter().circle_stroke(center, 3.5, stroke);
    ui.painter().line_segment(
        [center + egui::vec2(2.5, 2.5), center + egui::vec2(6., 6.)],
        stroke,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            format!("Choose icon: {caption}"),
        )
    });
    let mut import = false;
    egui::Popup::menu(&response)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            import = menu_contents(ui, value, assets);
        });
    response.on_hover_text(caption);
    import
}

fn menu_contents(ui: &mut egui::Ui, value: &mut Option<IconSource>, assets: &[Asset]) -> bool {
    let mut import = false;
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
        TextInput::singleline(&mut query)
            .hint_text("Search icons…")
            .desired_width(330.),
    );
    ui.horizontal(|ui| {
        for (name, key) in [
            ("All", ""),
            ("Solid", "solid"),
            ("Regular", "regular"),
            ("Brands", "brands"),
            ("Project images", "assets"),
        ] {
            ui.selectable_value(&mut style, key.to_owned(), name);
        }
    });
    if style == "assets" {
        if ui.button("Import image…").clicked() {
            import = true;
            ui.close();
        }
        let query = query.to_lowercase();
        let found: Vec<_> = assets
            .iter()
            .filter(|a| {
                !a.trashed
                    && a.name.to_lowercase().contains(&query)
                    && wireframe::images::dimensions(&a.bytes).is_ok()
            })
            .collect();
        ui.label(format!("{} project images", found.len()));
        egui::ScrollArea::vertical().max_height(300.).show_rows(
            ui,
            44.,
            found.len(),
            |ui, rows| {
                for row in rows {
                    let asset = found[row];
                    ui.horizontal(|ui| {
                        preview(ui, asset, 32.);
                        if ui_text::selectable(
                            ui,
                            matches!(value,Some(IconSource::Asset(id)) if *id==asset.id),
                            &asset.name,
                            ui.available_width(),
                        )
                        .clicked()
                        {
                            *value = Some(IconSource::Asset(asset.id));
                            ui.close();
                        }
                    });
                }
            },
        );
        if found.is_empty() {
            ui.small("Import an image to use it as an icon.");
        }
    } else {
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
                            let (rect, response) =
                                ui.allocate_exact_size(egui::vec2(58., 52.), egui::Sense::click());
                            let selected =
                                matches!(value,Some(IconSource::Catalog(id)) if id==&icon.id);
                            if selected || response.hovered() {
                                ui.painter()
                                    .rect_filled(rect, 4., ui.visuals().selection.bg_fill);
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
                                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label)
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
    }
    ui.ctx().data_mut(|d| {
        d.insert_temp(query_key, query);
        d.insert_temp(style_key, style);
    });
    import
}

#[derive(Clone, Default)]
struct Previews(std::collections::VecDeque<(u64, egui::TextureHandle)>);
pub(super) fn preview(ui: &mut egui::Ui, asset: &Asset, side: f32) {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    asset.bytes.hash(&mut hasher);
    let hash = hasher.finish();
    let key = egui::Id::new("project-icon-previews");
    let texture = ui
        .ctx()
        .data_mut(|d| {
            let cache = d.get_temp_mut_or_default::<Previews>(key);
            let index = cache.0.iter().position(|(h, _)| *h == hash)?;
            let entry = cache.0.remove(index)?;
            let texture = entry.1.clone();
            cache.0.push_back(entry);
            Some(texture)
        })
        .or_else(|| {
            let image = wireframe::images::thumbnail(&asset.bytes).ok()?;
            let texture = ui.ctx().load_texture(
                "Project icon preview",
                image.pixels,
                egui::TextureOptions::LINEAR,
            );
            ui.ctx().data_mut(|d| {
                let cache = d.get_temp_mut_or_default::<Previews>(key);
                cache.0.push_back((hash, texture.clone()));
                while cache.0.len() > 32 {
                    cache.0.pop_front();
                }
            });
            Some(texture)
        });
    if let Some(texture) = texture {
        ui.add(
            egui::Image::new((texture.id(), egui::vec2(side, side)))
                .fit_to_exact_size(egui::vec2(side, side)),
        );
    }
}
