// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::{
    catalog::PropertyType,
    document::Value,
    workspace::{PropertySection, SelectionProperty},
};

// Inspector glyphs keep their identifying edges at small sizes. Canvas geometry
// deliberately suppresses tears below its minimum control dimensions.
fn shape_option_outline(choice: &str) -> Vec<[f32; 2]> {
    match choice {
        "dotted_rectangle" => vec![[0.5, 0.5], [16.5, 0.5], [16.5, 16.5], [0.5, 16.5]],
        "torn_bottom" => vec![
            [0.5, 0.5],
            [16.5, 0.5],
            [16.5, 12.5],
            [11., 16.5],
            [6., 12.5],
            [0.5, 16.5],
        ],
        "torn_top" => vec![
            [0.5, 4.5],
            [6., 0.5],
            [11., 4.5],
            [16.5, 0.5],
            [16.5, 16.5],
            [0.5, 16.5],
        ],
        _ => wireframe::shapes::geometry(choice, 0., 17., 17., false).outline,
    }
}

fn app_bar_glyph(
    ui: &mut egui::Ui,
    kind: &str,
    label: &str,
    selected: Option<bool>,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(if selected.is_some() { 36. } else { 28. }, 24.),
        egui::Sense::click(),
    );
    let visuals = ui
        .style()
        .interact_selectable(&response, selected == Some(true));
    ui.painter().rect_filled(rect, 4., visuals.bg_fill);
    if response.clicked() {
        response.request_focus();
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            4.,
            Stroke::new(
                1.5,
                if selected == Some(true) {
                    ui.visuals().selection.stroke.color
                } else {
                    ui.visuals().selection.bg_fill
                },
            ),
            egui::StrokeKind::Inside,
        );
    }
    let p = ui.painter();
    let center = response.rect.center();
    let color = if selected == Some(true) {
        ui.visuals().selection.stroke.color
    } else {
        ui.visuals().text_color()
    };
    let stroke = egui::Stroke::new(1., color);
    let line = |a: [f32; 2], b: [f32; 2]| {
        p.line_segment(
            [
                center + egui::vec2(a[0], a[1]),
                center + egui::vec2(b[0], b[1]),
            ],
            stroke,
        );
    };
    match kind {
        "nav_icon" => {
            for y in [-5., 0., 5.] {
                line([-6., y], [6., y]);
            }
        }
        "filter_icon" => {
            line([-5., -3.], [0., 3.]);
            line([0., 3.], [5., -3.]);
            line([5., -3.], [-5., -3.]);
        }
        "menu_icon" => {
            for y in [-5., 0., 5.] {
                p.circle_filled(center + egui::vec2(0., y), 1., color);
            }
        }
        "browse" => {
            for y in [-5., 0., 5.] {
                line([-7., y], [-3., y]);
            }
            p.circle_stroke(center + egui::vec2(2., -1.), 3.5, stroke);
            line([4.5, 1.5], [8., 5.]);
        }
        "remove" => {
            line([-5., -5.], [5., 5.]);
            line([-5., 5.], [5., -5.]);
        }
        _ => {}
    }
    response.widget_info(|| match selected {
        Some(value) => egui::WidgetInfo::selected(egui::WidgetType::Button, true, value, label),
        None => egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label),
    });
    response.on_hover_text(label)
}

impl WireframeApp {
    fn change_selection_property(&mut self, key: &str, value: Value, finish: bool) {
        self.editor.begin("Edit selection properties");
        if let Err(error) = self.editor.set_selection_property(key, value) {
            self.editor.cancel();
            self.error = Some(error);
        } else if finish {
            self.editor.commit();
        }
    }
    fn common_property_row(&mut self, ui: &mut egui::Ui, property: &SelectionProperty) {
        let mut value = property.value.clone();
        let mut changed = false;
        let mut finish = false;
        let mut reset = false;
        let label = if property.key == "fill" {
            "Color"
        } else {
            property.label
        };
        let escape = ui.input(|i| i.key_pressed(Key::Escape));
        ui.push_id(property.key, |ui| {
            ui.horizontal(|ui| {
                if let Value::Bool(v) = &mut value {
                    changed = ui
                        .add(egui::Checkbox::new(v, label).indeterminate(
                            matches!(property.key, "header" | "sketch_filter") && property.mixed,
                        ))
                        .changed();
                    finish = changed;
                } else {
                    ui.label(label);
                }
                if property.explicit
                    && !(self.editor.editing_instance()
                        && self
                            .editor
                            .selection_property_section(&property.targets)
                            .is_some())
                {
                    reset = ui.small_button("Reset").clicked();
                }
            });
            match (&property.kind, &mut value) {
                (PropertyType::Choice(_), Value::Choice(v)) => {
                    let kind = property
                        .targets
                        .first()
                        .and_then(|(id, _)| self.editor.control_properties(*id))
                        .map_or("", |(s, _)| s.id);
                    egui::ComboBox::from_id_salt("value")
                        .width(ui.available_width())
                        .selected_text(properties::field_choice_label(kind, property.key, v))
                        .show_ui(ui, |ui| {
                            for option in &property.choices {
                                if ui
                                    .selectable_label(
                                        v == option,
                                        properties::field_choice_label(kind, property.key, option),
                                    )
                                    .clicked()
                                {
                                    *v = (*option).into();
                                    changed = true;
                                    finish = true;
                                    ui.close();
                                }
                            }
                        });
                }
                (PropertyType::Color, Value::Color(c)) => {
                    let mut rgb = [c[0], c[1], c[2]];
                    changed = ui.color_edit_button_srgb(&mut rgb).changed();
                    c[..3].copy_from_slice(&rgb);
                }
                (PropertyType::Number { min, max, integer }, Value::Number(v)) => {
                    let response = if matches!(property.key, "background_opacity" | "value") {
                        ui.scope(|ui| {
                            ui.spacing_mut().slider_width = (ui.available_width() - 64.).max(30.);
                            ui.add(egui::Slider::new(v, *min..=*max).fixed_decimals(2))
                        })
                        .inner
                    } else {
                        let mut edit = egui::DragValue::new(v)
                            .range(*min..=*max)
                            .update_while_editing(false);
                        if *integer {
                            edit = edit.max_decimals(0);
                        }
                        ui.add(edit)
                    };
                    let slider = matches!(property.key, "background_opacity" | "value");
                    if slider && (response.clicked() || response.dragged()) {
                        response.request_focus();
                    }
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            if slider {
                                egui::WidgetType::Slider
                            } else {
                                egui::WidgetType::DragValue
                            },
                            true,
                            label,
                        )
                    });
                    changed =
                        response.changed() || (property.mixed && response.lost_focus() && !escape);
                    finish = response.lost_focus() || response.drag_stopped();
                }
                _ => {}
            }
        });
        if reset {
            self.editor.commit();
            self.editor.begin("Reset selection property");
            if let Err(error) = self.editor.reset_selection_property(property.key) {
                self.editor.cancel();
                self.error = Some(error);
            } else {
                self.editor.commit();
            }
        } else if changed {
            self.change_selection_property(property.key, value, finish);
        } else if finish {
            self.editor.commit();
        }
    }
    fn shared_shape_arrow(&mut self, ui: &mut egui::Ui, fields: &[SelectionProperty]) {
        let Some(shape) = fields.iter().find(|p| p.key == "shape") else {
            return;
        };
        let kind = shape
            .targets
            .first()
            .and_then(|(id, _)| self.editor.control_properties(*id))
            .map_or("", |(s, _)| s.id);
        if kind == "shape" {
            if self.component_section_header(
                ui,
                &self.editor.ordered_selection(),
                PropertySection::Shape,
            ) {
                return;
            }
            for row in shape.choices.chunks(5) {
                ui.horizontal(|ui| {
                    let width =
                        ((ui.available_width() - 4. * ui.spacing().item_spacing.x) / 5.).max(24.);
                    for &choice in row {
                        let selected = shape.value == Value::Choice(choice.into());
                        let label = properties::field_choice_label("shape", "shape", choice);
                        let response =
                            ui.add_sized([width, 26.], egui::Button::new("").selected(selected));
                        response.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::Checkbox,
                                ui.is_enabled(),
                                selected,
                                label,
                            )
                        });
                        let rect = egui::Rect::from_center_size(
                            response.rect.center(),
                            egui::vec2(17., 17.),
                        );
                        let mut points: Vec<_> = shape_option_outline(choice)
                            .iter()
                            .map(|p| rect.min + egui::vec2(p[0], p[1]))
                            .collect();
                        if let Some(first) = points.first().copied() {
                            points.push(first);
                        }
                        let stroke = Stroke::new(1., ui.style().interact(&response).text_color());
                        if choice == "dotted_rectangle" {
                            for pair in points.windows(2) {
                                let delta = pair[1] - pair[0];
                                let length = delta.length();
                                let mut distance = 0.;
                                while distance < length {
                                    ui.painter().line_segment(
                                        [
                                            pair[0] + delta.normalized() * distance,
                                            pair[0]
                                                + delta.normalized() * (distance + 3.).min(length),
                                        ],
                                        stroke,
                                    );
                                    distance += 5.;
                                }
                            }
                        } else {
                            ui.painter().add(egui::Shape::line(points, stroke));
                        }
                        if response.on_hover_text(label).clicked() {
                            self.editor.commit();
                            self.change_selection_property(
                                "shape",
                                Value::Choice(choice.into()),
                                true,
                            );
                        }
                    }
                });
            }
            if fields.iter().any(|p| p.key == "shape_rotation")
                && ui.button("Rotate shape 90°").clicked()
            {
                self.editor.commit();
                self.editor.begin("Rotate shapes");
                if let Err(error) = self.editor.rotate_selection_shapes() {
                    self.editor.cancel();
                    self.error = Some(error);
                } else {
                    self.editor.commit();
                }
            }
        } else if kind == "arrow" {
            if self.component_section_header(
                ui,
                &self.editor.ordered_selection(),
                PropertySection::Arrow,
            ) {
                return;
            }
            ui.horizontal(|ui| {
                for (name, key, value) in [
                    ("Curve", "shape", Value::Choice("curve".into())),
                    ("Lines", "shape", Value::Choice("lines".into())),
                    ("Start arrowhead", "start_head", Value::Bool(true)),
                    ("End arrowhead", "end_head", Value::Bool(true)),
                ] {
                    let Some(field) = fields.iter().find(|p| p.key == key) else {
                        continue;
                    };
                    let selected = field.value == value;
                    if arrows::inspector::icon_button(ui, name, selected, 32.).clicked() {
                        self.editor.commit();
                        self.change_selection_property(
                            key,
                            if key == "shape" {
                                value
                            } else {
                                Value::Bool(!selected)
                            },
                            true,
                        );
                    }
                }
            });
            if let Some(field) = fields.iter().find(|p| p.key == "label_position")
                && let Value::Number(original) = field.value
            {
                let (value, finished) = arrows::inspector::label_position_widget(ui, original);
                if value != original {
                    self.change_selection_property(
                        "label_position",
                        Value::Number(value),
                        finished,
                    );
                } else if finished {
                    self.editor.commit();
                }
            }
        }
        if matches!(kind, "shape" | "arrow")
            && !self.editor.editing_instance()
            && fields.iter().any(|p| {
                matches!(
                    p.key,
                    "shape" | "shape_rotation" | "start_head" | "end_head" | "label_position"
                ) && p.explicit
            })
        {
            ui.menu_button("Reset shape options", |ui| {
                for field in fields.iter().filter(|p| {
                    matches!(
                        p.key,
                        "shape" | "shape_rotation" | "start_head" | "end_head" | "label_position"
                    ) && p.explicit
                }) {
                    if ui.button(format!("Reset {}", field.label)).clicked() {
                        self.editor.commit();
                        self.editor.begin("Reset selection property");
                        if let Err(error) = self.editor.reset_selection_property(field.key) {
                            self.editor.cancel();
                            self.error = Some(error);
                        } else {
                            self.editor.commit();
                        }
                        ui.close();
                    }
                }
            });
        }
    }
    fn shared_app_bar_icons(&mut self, ui: &mut egui::Ui, fields: &[SelectionProperty]) -> bool {
        let ids = self.editor.ordered_selection();
        let Some(slots) = self.editor.app_bar_icon_slots(&ids) else {
            return false;
        };
        if self.component_section_header(ui, &ids, PropertySection::AppBarIcons) {
            return true;
        }
        let toggle = |app: &mut Self, ui: &mut egui::Ui, key: &str| {
            if let Some(field) = fields.iter().find(|field| field.key == key)
                && let Value::Bool(value) = field.value
            {
                let response = app_bar_glyph(ui, key, field.label, Some(value));
                if response.clicked() {
                    app.change_selection_property(key, Value::Bool(!value), true);
                }
                if field.explicit {
                    response
                        .on_hover_text("Right-click for reset options")
                        .context_menu(|ui| {
                            if ui.button(format!("Reset {}", field.label)).clicked() {
                                app.editor.commit();
                                app.editor.begin("Reset selection property");
                                if let Err(error) = app.editor.reset_selection_property(key) {
                                    app.editor.cancel();
                                    app.error = Some(error);
                                } else {
                                    app.editor.commit();
                                }
                                ui.close();
                            }
                        });
                }
            }
        };
        toggle(self, ui, "nav_icon");
        toggle(self, ui, "filter_icon");
        let count = slots.len();
        for slot in slots {
            let mut chosen = slot.icon.clone();
            let mut remove = false;
            ui.push_id(("app-bar-icon", slot.index), |ui| {
                ui.horizontal(|ui| {
                    let width =
                        (ui.available_width() - 56. - ui.spacing().item_spacing.x * 2.).max(40.);
                    if super::icon_picker::compact(
                        ui,
                        &mut chosen,
                        &self.editor.document.assets,
                        &slot.label,
                        width,
                    ) {
                        self.start_image_import(
                            wireframe::workspace::ImageImportTarget::AppBarIcons {
                                controls: ids.clone(),
                                index: slot.index,
                            },
                        );
                    }
                    if app_bar_glyph(
                        ui,
                        "browse",
                        &format!("Browse icon {}…", slot.index + 1),
                        None,
                    )
                    .clicked()
                    {
                        self.begin_app_bar_icon_dialog(ids.clone(), slot.index);
                    }
                    remove = app_bar_glyph(
                        ui,
                        "remove",
                        &format!("Remove icon {}", slot.index + 1),
                        None,
                    )
                    .clicked();
                });
            });
            if remove || chosen != slot.icon {
                self.change_app_bar_icon(&ids, slot.index, if remove { None } else { chosen });
                return true;
            }
        }
        ui.push_id("app-bar-add", |ui| {
            ui.horizontal(|ui| {
                ui.label("Add:");
                let mut chosen = None;
                let width =
                    (ui.available_width() - 56. - ui.spacing().item_spacing.x * 2.).max(40.);
                if super::icon_picker::compact(
                    ui,
                    &mut chosen,
                    &self.editor.document.assets,
                    "Search",
                    width,
                ) {
                    self.start_image_import(wireframe::workspace::ImageImportTarget::AppBarIcons {
                        controls: ids.clone(),
                        index: count,
                    });
                }
                if let Some(icon) = chosen {
                    self.change_app_bar_icon(&ids, count, Some(icon));
                }
                if app_bar_glyph(ui, "browse", "Browse new icon…", None).clicked() {
                    self.begin_app_bar_icon_dialog(ids.clone(), count);
                }
                ui.add_space(28.);
            });
        });
        toggle(self, ui, "menu_icon");
        false
    }

    fn change_app_bar_icon(
        &mut self,
        ids: &[Id],
        index: usize,
        icon: Option<wireframe::document::IconSource>,
    ) {
        self.editor.commit();
        self.editor.begin("Change app bar icon");
        if let Err(error) = self.editor.set_app_bar_icon(ids, index, icon) {
            self.editor.cancel();
            self.error = Some(error);
        } else {
            self.editor.commit();
        }
    }

    fn shared_row_icons(&mut self, ui: &mut egui::Ui) {
        let ids = self.editor.ordered_selection();
        let Some(rows) = self.editor.list_row_icons(&ids) else {
            return;
        };
        egui::CollapsingHeader::new("Edit row icons").show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("shared-row-icons")
                .max_height(280.)
                .show(ui, |ui| {
                    for row in rows {
                        ui.push_id(row.row, |ui| {
                            ui_text::label(ui, &row.label, ui.available_width());
                            ui.horizontal(|ui| {
                                let label = match &row.icon {
                                    Some(wireframe::document::IconSource::Catalog(id)) => {
                                        wireframe::icons::get(id)
                                            .map_or("Unavailable icon".into(), |icon| {
                                                icon.label.clone()
                                            })
                                    }
                                    Some(wireframe::document::IconSource::Asset(id)) => self
                                        .editor
                                        .document
                                        .live_asset(*id)
                                        .map_or("Unavailable image".into(), |asset| {
                                            asset.name.clone()
                                        }),
                                    None => "Choose icon…".into(),
                                };
                                if ui.button(label).clicked() {
                                    self.begin_row_icon_dialog(row.targets.clone(), row.row);
                                }
                                if row.icon.is_some() && ui.small_button("Remove").clicked() {
                                    self.editor.commit();
                                    self.editor.begin("Remove row icon");
                                    if let Err(error) =
                                        self.editor.set_list_row_icon(&ids, row.row, None)
                                    {
                                        self.editor.cancel();
                                        self.error = Some(error);
                                    } else {
                                        self.editor.commit();
                                    }
                                }
                            });
                        });
                    }
                });
        });
    }
    fn shared_row_selection(&mut self, ui: &mut egui::Ui) {
        let Some(rows) = self.editor.row_selection() else {
            return;
        };
        let captions = rows.captions();
        let mut reset = false;
        if self.editor.editing_instance()
            && self.component_section_header(
                ui,
                &self.editor.ordered_selection(),
                PropertySection::Selection,
            )
        {
            return;
        }
        ui.horizontal(|ui| {
            if !self.editor.editing_instance() {
                ui.label("Selection");
            }
            if rows.explicit && !self.editor.editing_instance() {
                reset = ui.small_button("Reset").clicked();
            }
        });
        let caption = if rows.value < 0 {
            "None"
        } else {
            captions
                .get(rows.value as usize)
                .map_or("No matching item", String::as_str)
        };
        let mut chosen = None;
        ui_text::combo_box(
            ui,
            "shared-row-selection",
            if caption.is_empty() {
                "(Empty item)"
            } else {
                caption
            },
            |ui| {
                if rows.allow_none {
                    if ui_text::selectable(ui, rows.value == -1, "None", ui.available_width())
                        .clicked()
                    {
                        chosen = Some(-1);
                        ui.close();
                    }
                    if !rows.labels.is_empty() {
                        ui.separator();
                    }
                }
                for (index, label) in captions.iter().enumerate() {
                    if ui_text::selectable(
                        ui,
                        rows.value == index as i32,
                        if label.is_empty() {
                            "(Empty item)"
                        } else {
                            label
                        },
                        ui.available_width(),
                    )
                    .clicked()
                    {
                        chosen = Some(index as i32);
                        ui.close();
                    }
                }
            },
        );
        if reset || chosen.is_some() {
            self.editor.commit();
            self.editor.begin(if reset {
                "Reset selection row"
            } else {
                "Change selection row"
            });
            let result = if reset {
                self.editor.reset_selection_row()
            } else {
                self.editor.set_selection_row(chosen.unwrap())
            };
            if let Err(error) = result {
                self.editor.cancel();
                self.error = Some(error);
            } else {
                self.editor.commit();
            }
        }
    }
    pub(super) fn selection_properties(&mut self, ui: &mut egui::Ui) {
        let ids = self.editor.ordered_selection();
        let controls: std::collections::HashMap<_, _> = ids
            .iter()
            .filter_map(|id| {
                self.editor
                    .control_properties(*id)
                    .map(|control| (*id, control))
            })
            .collect();
        let fields: Vec<_> = self
            .editor
            .selection_properties()
            .into_iter()
            .filter(|p| {
                p.targets.iter().all(|(id, key)| {
                    controls
                        .get(id)
                        .is_some_and(|(s, _)| properties::supported(s.id, key))
                })
            })
            .collect();
        let tabs = !controls.is_empty() && controls.values().all(|(s, _)| s.id == "tabs");
        let app_bars = !controls.is_empty() && controls.values().all(|(s, _)| s.id == "app_bar");
        let mut section_headers = std::collections::HashSet::new();
        ui.push_id(("selection-properties", ids), |ui| {
            self.shared_image_properties(ui);
            self.shared_row_selection(ui);
            self.shared_shape_arrow(ui, &fields);
            for field in fields.iter().filter(|p| {
                p.specialized()
                    && !matches!(
                        p.key,
                        "shape_rotation" | "start_head" | "end_head" | "label_position"
                    )
                    && !(p.key == "shape"
                        && p.targets.first().is_some_and(|(id, _)| {
                            controls
                                .get(id)
                                .is_some_and(|(s, _)| matches!(s.id, "shape" | "arrow"))
                        }))
            }) {
                if self.common_section_header(ui, field, &mut section_headers) {
                    return;
                }
                self.common_property_row(ui, field);
            }
            if tabs && let Some(field) = fields.iter().find(|p| p.key == "align") {
                if self.common_section_header(ui, field, &mut section_headers) {
                    return;
                }
                self.common_property_row(ui, field);
            }
            if self.editor.editing_instance()
                && fields
                    .iter()
                    .any(|p| PropertySection::for_key(p.key) == Some(PropertySection::Color))
                && self.component_section_header(
                    ui,
                    &self.editor.ordered_selection(),
                    PropertySection::Color,
                )
            {
                return;
            }
            for field in fields.iter().filter(|p| {
                matches!(
                    p.key,
                    "fill"
                        | "border_color"
                        | "text_color"
                        | "main_color"
                        | "alternate_fill"
                        | "background_opacity"
                        | "border"
                        | "stroke"
                )
            }) {
                if field.key == "border"
                    && self.common_section_header(ui, field, &mut section_headers)
                {
                    return;
                }
                self.common_property_row(ui, field);
            }
            if app_bars && self.shared_app_bar_icons(ui, &fields) {
                return;
            }
            if fields.iter().any(|p| p.key == "icon") {
                if self.component_section_header(
                    ui,
                    &self.editor.ordered_selection(),
                    PropertySection::Icon,
                ) {
                    return;
                }
                let mut labels = vec![];
                let mut has_icon = false;
                for id in self.editor.ordered_selection() {
                    if let Some((_, v)) = controls.get(&id) {
                        match v.get("icon") {
                            Some(Value::Icon(wireframe::document::IconSource::Catalog(id))) => {
                                has_icon = true;
                                labels.push(
                                    wireframe::icons::get(id)
                                        .map_or("Unavailable icon".to_owned(), |i| i.label.clone()),
                                );
                            }
                            Some(Value::Icon(wireframe::document::IconSource::Asset(id))) => {
                                has_icon = true;
                                labels.push(
                                    self.editor
                                        .document
                                        .live_asset(*id)
                                        .map_or("Unavailable image".to_owned(), |a| a.name.clone()),
                                );
                            }
                            _ => labels.push("—".into()),
                        }
                    }
                }
                if has_icon {
                    ui_text::label(ui, &labels.join(" / "), ui.available_width());
                }
                if ui.button("Choose icon…").clicked() {
                    self.begin_selection_icon_dialog();
                }
                if has_icon {
                    if ui.button("Rotate 90°").clicked() {
                        self.editor.commit();
                        self.editor.begin("Rotate icons");
                        if let Err(error) = self.editor.rotate_selection_icons() {
                            self.editor.cancel();
                            self.error = Some(error);
                        } else {
                            self.editor.commit();
                        }
                    }
                    if ui.button("Remove icons").clicked() {
                        self.change_selection_property("icon", Value::Null, true);
                    }
                    for field in fields.iter().filter(|p| p.key == "icon_size") {
                        self.common_property_row(ui, field);
                    }
                }
            }
        });
        self.selection_links(ui);
        self.shared_row_icons(ui);
        for field in fields
            .iter()
            .filter(|p| p.key == "state" || (p.key == "menu_icon" && !app_bars))
        {
            if self.common_section_header(ui, field, &mut section_headers) {
                return;
            }
            self.common_property_row(ui, field);
        }
        if fields.iter().any(|p| {
            matches!(
                p.key,
                "bold" | "italic" | "underline" | "align" | "font_size"
            )
        }) && self.component_section_header(
            ui,
            &self.editor.ordered_selection(),
            PropertySection::Text,
        ) {
            return;
        }
        ui.horizontal_wrapped(|ui| {
            for (key, label, text) in [
                ("bold", "Bold", "B"),
                ("italic", "Italic", "I"),
                ("underline", "Underline", "U"),
            ] {
                if let Some(p) = fields.iter().find(|p| p.key == key) {
                    let on = p.value == Value::Bool(true);
                    let response = ui
                        .add_sized(
                            [24., 24.],
                            egui::Button::selectable(
                                on,
                                match key {
                                    "bold" => egui::RichText::new(text).strong(),
                                    "italic" => egui::RichText::new(text).italics(),
                                    _ => egui::RichText::new(text).underline(),
                                },
                            ),
                        )
                        .on_hover_text(label);
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label)
                    });
                    if response.clicked() {
                        self.change_selection_property(key, Value::Bool(!on), true);
                    }
                }
            }
            if !tabs && let Some(p) = fields.iter().find(|p| p.key == "align") {
                for (value, label) in [
                    ("left", "Text align left"),
                    ("center", "Text align center"),
                    ("right", "Text align right"),
                ] {
                    if !p.choices.contains(&value) {
                        continue;
                    }
                    let response = ui
                        .add_sized(
                            [24., 24.],
                            egui::Button::selectable(p.value == Value::Choice(value.into()), ""),
                        )
                        .on_hover_text(label);
                    let center = response.rect.center();
                    let stroke = Stroke::new(1., ui.style().interact(&response).text_color());
                    for (y, width) in [(-5., 12.), (0., 8.), (5., 12.)] {
                        let x = match value {
                            "left" => -6.,
                            "right" => 6. - width,
                            _ => -width / 2.,
                        };
                        ui.painter().line_segment(
                            [center + Vec2::new(x, y), center + Vec2::new(x + width, y)],
                            stroke,
                        );
                    }
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label)
                    });
                    if response.clicked() {
                        self.change_selection_property("align", Value::Choice(value.into()), true);
                    }
                }
            }
        });
        if let Some(p) = fields.iter().find(|p| p.key == "font_size") {
            self.common_property_row(ui, p);
        }
        if !wireframe::text_entry::is_focused(ui.ctx())
            && !ui.ctx().text_edit_focused()
            && !egui::Popup::is_any_open(ui.ctx())
            && !ui.input(|i| i.pointer.any_down())
        {
            self.editor.commit();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compact_app_bar_rows_fit_and_named_glyphs_change_only_their_targets() {
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
            for x in [50., 600.] {
                app.editor.add(Kind::AppBar, x, 50.);
            }
            app.editor.select_all();
            let ids = app.editor.ordered_selection();
            let icon = |s: &str| Some(wireframe::document::IconSource::Catalog(s.into()));
            app.editor
                .set_control_property(
                    ids[0],
                    "icons",
                    Value::Icons(vec![
                        icon("asterisk-solid"),
                        icon("heart-solid"),
                        icon("star-solid"),
                    ]),
                )
                .unwrap();
            app.editor
                .set_control_property(ids[1], "icons", Value::Icons(vec![icon("circle-solid")]))
                .unwrap();
            app.editor.commit();
            let before = app.editor.document.clone();
            let frame = |app: &mut WireframeApp, events| {
                let mut raw = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 900.))),
                    events,
                    ..Default::default()
                };
                raw.viewports
                    .get_mut(&egui::ViewportId::ROOT)
                    .unwrap()
                    .native_pixels_per_point = Some(density);
                let mut out = ctx.run_ui(raw, |ui| {
                    let fields = app.editor.selection_properties();
                    app.shared_app_bar_icons(ui, &fields);
                    assert!(
                        ui.min_rect().width() <= width + 0.1,
                        "width {width}: {:?}",
                        ui.min_rect()
                    );
                    assert!(
                        ui.min_rect().height() < 260.,
                        "compact rows too tall: {:?}",
                        ui.min_rect()
                    );
                });
                out.textures_delta.clear();
                out
            };
            frame(&mut app, vec![]);
            let out = frame(&mut app, vec![]);
            assert_eq!(app.editor.document, before);
            let nodes = &out.platform_output.accesskit_update.as_ref().unwrap().nodes;
            let bounds = |label: &str| {
                nodes
                    .iter()
                    .find(|(_, n)| n.label() == Some(label))
                    .unwrap()
                    .1
                    .bounds()
                    .unwrap()
            };
            let choose = bounds("Choose icon: Star / No Icon");
            let browse = bounds("Browse icon 3…");
            let remove = bounds("Remove icon 3");
            assert!((choose.y0 - browse.y0).abs() < 1. && (browse.y0 - remove.y0).abs() < 1.);
            assert!(choose.x1 <= browse.x0 && browse.x1 <= remove.x0);
            let nav = bounds("Navigation icon");
            let at = Pos2::new((nav.x0 + nav.x1) as f32 / 2., (nav.y0 + nav.y1) as f32 / 2.);
            for pressed in [true, false] {
                frame(
                    &mut app,
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
            for id in ids {
                assert_eq!(
                    app.editor.control_properties(id).unwrap().1["nav_icon"],
                    Value::Bool(false)
                );
            }
            app.editor.undo();
            assert_eq!(app.editor.document, before);
        }
    }

    #[test]
    fn shared_fields_fit_inspector_widths_without_mutating_mixed_values() {
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
            app.editor.add(Kind::Button, 50., 50.);
            let first = *app.editor.selection.iter().next().unwrap();
            app.editor.add(Kind::Button, 200., 50.);
            let second = *app.editor.selection.iter().next().unwrap();
            app.editor
                .set_control_property(first, "font_size", Value::Number(17.))
                .unwrap();
            app.editor
                .set_control_property(second, "font_size", Value::Number(25.))
                .unwrap();
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
                        "panel grew at {width}/{density}"
                    );
                });
                let nodes = &out.platform_output.accesskit_update.as_ref().unwrap().nodes;
                for label in [
                    "Choose icon…",
                    "Bold",
                    "Italic",
                    "Underline",
                    "Text align left",
                    "Text align center",
                    "Text align right",
                    "Font size",
                ] {
                    assert!(
                        nodes.iter().any(|(_, n)| n.label() == Some(label)),
                        "missing {label}"
                    );
                }
                out.textures_delta.clear();
            }
            assert_eq!(app.editor.document, before);
        }
    }
    #[test]
    fn shared_slider_keyboard_changes_values_without_moving_controls() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.current_mut().content.nodes = ["horizontal_slider", "vertical_slider"]
            .into_iter()
            .enumerate()
            .map(|(i, kind)| {
                wireframe::document::Node::control(
                    i as u64 + 2,
                    kind,
                    wireframe::document::Point::new(50. + i as f32 * 200., 50.),
                    wireframe::document::Size::fixed(160., 100.),
                )
            })
            .collect();
        app.editor.document.next_id = 4;
        app.editor.select_all();
        let frame = |app: &mut WireframeApp, events| {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(300., 900.))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    app.shortcuts(ui.ctx());
                    app.selection_properties(ui);
                },
            );
            out.textures_delta.clear();
            out
        };
        frame(&mut app, vec![]);
        let out = frame(&mut app, vec![]);
        let nodes = &out.platform_output.accesskit_update.as_ref().unwrap().nodes;
        let r = nodes
            .iter()
            .find(|(_, n)| n.role() == egui::accesskit::Role::Slider && n.label() == Some("Value"))
            .unwrap()
            .1
            .bounds()
            .unwrap();
        let at = Pos2::new((r.x0 + r.x1) as f32 / 2., (r.y0 + r.y1) as f32 / 2.);
        for pressed in [true, false] {
            frame(
                &mut app,
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
        frame(&mut app, vec![]);
        let before = app.editor.document.clone();
        let value = app.editor.control_properties(2).unwrap().1["value"].clone();
        frame(
            &mut app,
            vec![egui::Event::Key {
                key: Key::ArrowLeft,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
        let next = app.editor.control_properties(2).unwrap().1["value"].clone();
        assert!(matches!((&value,&next),(Value::Number(a),Value::Number(b)) if b<a));
        assert_eq!(app.editor.control_properties(3).unwrap().1["value"], next);
        for (a, b) in before.pages[0]
            .content
            .nodes
            .iter()
            .zip(app.editor.nodes().iter())
        {
            assert_eq!(a.origin, b.origin);
        }
    }
    #[test]
    fn shared_shape_and_arrow_actions_follow_real_pointer_and_keyboard_input() {
        fn frame(
            app: &mut WireframeApp,
            ctx: &egui::Context,
            events: Vec<egui::Event>,
        ) -> egui::FullOutput {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(300., 1600.))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    app.shortcuts(ui.ctx());
                    app.selection_properties(ui);
                },
            );
            out.textures_delta.clear();
            out
        }
        fn locate(out: &egui::FullOutput, name: &str, role: egui::accesskit::Role) -> Pos2 {
            let node = &out
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .find(|(_, n)| {
                    n.role() == role
                        && (n.label() == Some(name)
                            || n.labelled_by().iter().any(|id| {
                                out.platform_output
                                    .accesskit_update
                                    .as_ref()
                                    .unwrap()
                                    .nodes
                                    .iter()
                                    .any(|(other, node)| {
                                        other == id
                                            && node.label().or_else(|| node.value()) == Some(name)
                                    })
                            }))
                })
                .unwrap_or_else(|| panic!("missing {name}"))
                .1;
            let r = node.bounds().unwrap();
            Pos2::new((r.x0 + r.x1) as f32 / 2., (r.y0 + r.y1) as f32 / 2.)
        }
        fn click(app: &mut WireframeApp, ctx: &egui::Context, at: Pos2) {
            for pressed in [true, false] {
                frame(
                    app,
                    ctx,
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
        }
        use egui::accesskit::Role;
        for kind in ["shape", "arrow"] {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.current_mut().content.nodes = (2..4)
                .map(|id| {
                    wireframe::document::Node::control(
                        id,
                        kind,
                        wireframe::document::Point::new(50. + (id - 2) as f32 * 250., 50.),
                        wireframe::document::Size::fixed(180., 140.),
                    )
                })
                .collect();
            app.editor.document.next_id = 4;
            app.editor.select_all();
            frame(&mut app, &ctx, vec![]);
            let out = frame(&mut app, &ctx, vec![]);
            let baseline = app.editor.document.clone();
            if kind == "shape" {
                let at = locate(&out, "Triangle", Role::CheckBox);
                click(&mut app, &ctx, at);
                let triangle = app.editor.document.clone();
                for id in [2, 3] {
                    assert_eq!(
                        app.editor.control_properties(id).unwrap().1["shape"],
                        Value::Choice("triangle".into())
                    );
                }
                let out = frame(&mut app, &ctx, vec![]);
                let at = locate(&out, "Rotate shape 90°", Role::Button);
                click(&mut app, &ctx, at);
                for id in [2, 3] {
                    assert_eq!(
                        app.editor.control_properties(id).unwrap().1["shape_rotation"],
                        Value::Number(90.)
                    );
                }
                app.editor.undo();
                assert_eq!(app.editor.document, triangle);
                app.editor.undo();
                assert_eq!(app.editor.document, baseline);
            } else {
                let at = locate(&out, "Lines", Role::CheckBox);
                click(&mut app, &ctx, at);
                for id in [2, 3] {
                    assert_eq!(
                        app.editor.control_properties(id).unwrap().1["shape"],
                        Value::Choice("lines".into())
                    );
                }
                let out = frame(&mut app, &ctx, vec![]);
                let at = locate(&out, "Start arrowhead", Role::CheckBox);
                click(&mut app, &ctx, at);
                for id in [2, 3] {
                    assert_eq!(
                        app.editor.control_properties(id).unwrap().1["start_head"],
                        Value::Bool(false)
                    );
                }
                let out = frame(&mut app, &ctx, vec![]);
                let at = locate(&out, "Label position", Role::Slider);
                click(&mut app, &ctx, at);
                let before = app.editor.document.clone();
                frame(
                    &mut app,
                    &ctx,
                    vec![egui::Event::Key {
                        key: Key::ArrowRight,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::NONE,
                    }],
                );
                for id in [2, 3] {
                    assert_eq!(
                        app.editor.control_properties(id).unwrap().1["label_position"],
                        Value::Number(7.)
                    );
                }
                for (a, b) in before.pages[0]
                    .content
                    .nodes
                    .iter()
                    .zip(app.editor.nodes().iter())
                {
                    assert_eq!(a.origin, b.origin);
                }
                app.editor.commit();
                app.editor.undo();
                assert_eq!(app.editor.document, before);
            }
        }
    }
}
