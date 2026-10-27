// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
mod category_row;
#[cfg(test)]
mod tests;

pub(super) fn default_categories() -> Vec<CatalogCategory> {
    vec![CatalogCategory::All]
}
pub(super) fn normalize_categories(categories: &mut Vec<CatalogCategory>) {
    if categories.is_empty() || categories.contains(&CatalogCategory::All) {
        *categories = default_categories();
    } else {
        let mut seen = vec![];
        categories.retain(|category| {
            if seen.contains(category) {
                false
            } else {
                seen.push(*category);
                true
            }
        });
    }
}
impl WireframeApp {
    fn select_catalog_category(&mut self, category: CatalogCategory, shift: bool) {
        self.end_pointer_gestures();
        if !shift
            || category == CatalogCategory::All
            || self.categories.contains(&CatalogCategory::All)
        {
            self.categories = vec![category];
        } else if self.categories.contains(&category) {
            self.categories.retain(|old| *old != category);
        } else {
            self.categories.push(category);
        }
        normalize_categories(&mut self.categories);
    }
    pub(super) fn catalog_item_enabled(&self, item: CatalogItem) -> bool {
        self.editor
            .catalog_entry(item)
            .is_some_and(|entry| entry.enabled)
    }
    pub(super) fn insert_catalog_at(&mut self, item: CatalogItem, point: Pos2) {
        let Some(entry) = self
            .editor
            .catalog_entry(item)
            .filter(|entry| entry.enabled)
        else {
            self.error = Some("This item is no longer available in the current workspace".into());
            return;
        };
        match item {
            CatalogItem::Control(kind) => self.editor.add(kind, point.x, point.y),
            CatalogItem::Preset(preset) => self.editor.add_preset(preset, point.x, point.y),
            CatalogItem::Icon(id) => {
                if let Err(error) = self.editor.insert_catalog_icon(id, point.x, point.y) {
                    self.error = Some(error);
                }
            }
            CatalogItem::Asset(id) => {
                if let Err(error) = self.editor.insert_catalog_asset(id, point.x, point.y) {
                    self.error = Some(error);
                }
            }
            CatalogItem::Component(id) => {
                if let Err(error) = self.editor.insert_catalog_component(id, point.x, point.y) {
                    self.error = Some(error);
                }
            }
        }
        if self.error.is_none() && self.editor.error.is_none() {
            self.status = format!("Added {}", entry.name);
        }
    }
    pub(super) fn add_catalog_item(&mut self, item: CatalogItem, point: Option<Pos2>) {
        if !self.catalog_item_enabled(item) {
            self.error = Some("This item is no longer available in the current workspace".into());
            return;
        }
        if !self.finish_page_notes() {
            return;
        }
        self.end_pointer_gestures();
        let point = match point {
            Some(point) => point,
            None => match self.previews.item_scene(item, &self.editor.document) {
                Ok(scene) => self.insertion_for_size(Vec2::new(scene.bounds.w, scene.bounds.h)),
                Err(error) => {
                    self.error = Some(error);
                    return;
                }
            },
        };
        self.insert_catalog_at(item, point);
    }
    pub(super) fn library(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("control-library")
            .frame(egui::Frame::NONE.fill(chrome::PANEL))
            .show(ui, |ui| {
                if self.image_crop.is_some() || self.canvas_input_blocked() {
                    ui.disable();
                }
                ui.spacing_mut().item_spacing.y = 0.;
                let window_center = ui.available_rect_before_wrap().center().x;
                ui.horizontal(|ui| {
                    let button_width = 108.;
                    let categories_width =
                        (ui.available_width() - button_width - ui.spacing().item_spacing.x).max(0.);
                    ui.allocate_ui_with_layout(
                        Vec2::new(categories_width, 30.),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            if let Some((category, shift)) =
                                category_row::show(ui, &self.categories, window_center)
                            {
                                self.select_catalog_category(category, shift);
                            }
                        },
                    );
                    if ui
                        .add_sized(
                            [button_width, 24.],
                            egui::Button::new(egui::RichText::new("More Controls…").size(11.)),
                        )
                        .clicked()
                    {
                        self.open_more_controls();
                    }
                });
                egui::Frame::NONE.inner_margin(6).show(ui, |ui| {
                    // Keep the canvas stationary when a category becomes empty
                    // or fits without scrolling. Overflow gets its own visible
                    // track instead of painting over the thumbnail captions.
                    ui.set_min_height(77.);
                    ui.spacing_mut().scroll = egui::style::ScrollStyle {
                        bar_width: 9.,
                        bar_inner_margin: 0.,
                        bar_outer_margin: 0.,
                        foreground_color: true,
                        ..egui::style::ScrollStyle::solid()
                    };
                    let entries = self.editor.catalog_categories(&self.categories);
                    egui::ScrollArea::horizontal()
                        .id_salt("library-previews")
                        .show_viewport(ui, |ui, viewport| {
                            let total = entries.len();
                            let first = (((viewport.min.x - 10.).max(0.) / 84.).floor() as usize)
                                .min(total);
                            let end = ((((viewport.max.x - 10.).max(0.) / 84.).ceil() as usize)
                                + 1)
                            .min(total);
                            ui.set_min_width((10. + total as f32 * 84. - 16.).max(0.));
                            ui.horizontal(|ui| {
                                ui.set_min_height(68.);
                                ui.spacing_mut().item_spacing.x = 16.;
                                ui.add_space(10. + first as f32 * 84.);
                                for entry in entries
                                    .into_iter()
                                    .skip(first)
                                    .take(end.saturating_sub(first))
                                {
                                    // Identity remains stable across sorting, renames and equal labels.
                                    let identity = match entry.item {
                                        CatalogItem::Control(kind) => {
                                            format!("control:{}", kind.catalog_id())
                                        }
                                        CatalogItem::Preset(preset) => {
                                            format!("preset:{}", preset.id())
                                        }
                                        CatalogItem::Component(id) => format!("component:{id}"),
                                        CatalogItem::Asset(id) => format!("asset:{id}"),
                                        CatalogItem::Icon(id) => format!("icon:{id}"),
                                    };
                                    ui.push_id(identity, |ui| {
                                        ui.add_enabled_ui(entry.enabled, |ui| {
                                            let (r, response) = ui.allocate_exact_size(
                                                Vec2::splat(68.),
                                                Sense::click_and_drag(),
                                            );
                                            if !ui.is_rect_visible(r) {
                                                return;
                                            }
                                            if response.hovered()
                                                || response.is_pointer_button_down_on()
                                                || self.library_drag == Some(entry.item)
                                            {
                                                ui.painter().rect_filled(
                                                    r,
                                                    3,
                                                    Color32::from_gray(222),
                                                );
                                            }
                                            let painter = ui.painter().with_clip_rect(r);
                                            match self
                                                .previews
                                                .item_scene(entry.item, &self.editor.document)
                                            {
                                                Ok(drawing) => {
                                                    let image = library_drag::TileImage::for_item(
                                                        entry.item,
                                                        r,
                                                        drawing.bounds,
                                                    );
                                                    Self::paint_scene(
                                                        &painter,
                                                        &drawing,
                                                        image.origin,
                                                        image.scale,
                                                    );
                                                }
                                                Err(_) => {
                                                    painter.text(
                                                        r.center_top() + Vec2::new(0., 18.),
                                                        Align2::CENTER_TOP,
                                                        "Preview unavailable",
                                                        FontId::proportional(10.),
                                                        Color32::DARK_GRAY,
                                                    );
                                                }
                                            }
                                            let mut label =
                                                egui::text::LayoutJob::simple_singleline(
                                                    entry.name.clone(),
                                                    FontId::proportional(11.),
                                                    Color32::from_gray(35),
                                                );
                                            label.wrap.max_width = r.width();
                                            label.wrap.max_rows = 1;
                                            let label = painter.layout_job(label);
                                            painter.galley(
                                                r.center_top()
                                                    + Vec2::new(-label.size().x / 2., 51.),
                                                label,
                                                Color32::from_gray(35),
                                            );
                                            response.widget_info(|| {
                                                egui::WidgetInfo::labeled(
                                                    egui::WidgetType::Button,
                                                    ui.is_enabled(),
                                                    &entry.name,
                                                )
                                            });
                                            if response.double_clicked()
                                                || (response.clicked()
                                                    && !ui.input(|i| i.pointer.any_released()))
                                            {
                                                // Keyboard/accessibility activation has no mouse release.
                                                self.add_catalog_item(entry.item, None);
                                            }
                                            if let CatalogItem::Asset(id) = entry.item {
                                                response.context_menu(|ui| {
                                                    if ui.button("Move Image to Trash").clicked() {
                                                        ui.close();
                                                        self.move_asset_to_trash(id);
                                                    }
                                                });
                                            }
                                            self.library_drag_source(ui, entry.item, r, &response);
                                            response.on_hover_text(if entry.enabled {
                                            format!(
                                                "{} — double-click to add, or drag onto the canvas",
                                                entry.name
                                            )
                                        } else {
                                            format!(
                                                "{} — unavailable in this workspace",
                                                entry.name
                                            )
                                        });
                                        });
                                    });
                                }
                            });
                        });
                });
            });
    }
}
