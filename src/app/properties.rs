// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::{catalog::PropertyType, document::Value};

// Expose fields whose behavior the shared renderer currently implements. The
// catalog's full field inventory remains separate from completed rendering.
fn supported(kind: &str, key: &str) -> bool {
    match key {
        "text" | "font_size" | "bold" | "italic" | "underline" | "text_color" | "align" => true,
        "state" => matches!(
            kind,
            "checkbox"
                | "radio"
                | "label"
                | "link"
                | "paragraph"
                | "button"
                | "text_input"
                | "text_area"
        ),
        "fill" => matches!(
            kind,
            "button" | "text_input" | "text_area" | "dropdown" | "search" | "table"
        ),
        "alternate_fill" | "header" | "row_height" | "selected_index" | "horizontal_lines"
        | "vertical_lines" => kind == "table",
        _ => false,
    }
}
fn choice_label(value: &str) -> &str {
    match value {
        "normal" => "Normal",
        "selected" => "Selected",
        "focused" => "Focused",
        "disabled" => "Disabled",
        "disabled_selected" => "Selected and disabled",
        "indeterminate" => "Indeterminate",
        "disabled_indeterminate" => "Indeterminate and disabled",
        "left" => "Left",
        "center" => "Center",
        "right" => "Right",
        other => other,
    }
}
impl WireframeApp {
    pub(super) fn typed_properties(&mut self, ui: &mut egui::Ui, id: Id) -> bool {
        let Some((spec, values)) = self.editor.control_properties(id) else {
            return false;
        };
        if !matches!(
            spec.id,
            "label"
                | "heading"
                | "link"
                | "paragraph"
                | "text_area"
                | "button"
                | "text_input"
                | "dropdown"
                | "search"
                | "checkbox"
                | "radio"
                | "table"
        ) {
            return false;
        }
        let node = self.editor.nodes().iter().find(|n| n.id == id).unwrap();
        let locked = node.locked;
        let explicit = node.properties.clone();
        ui.add_space(10.);
        ui.add_enabled_ui(!locked, |ui| {
            if spec.autosize != wireframe::catalog::AutoSize::None && ui.button("Auto-size").clicked() {
                self.editor.commit();
                self.editor.begin("Auto-size control");
                if let Err(error) = self.editor.auto_size_control(id) { self.error = Some(error); }
                self.editor.commit();
            }
            for key in ["text", "font_size", "bold", "italic", "underline", "align", "state", "text_color", "fill", "header", "row_height", "selected_index", "alternate_fill", "horizontal_lines", "vertical_lines"] {
                let Some(field) = spec.property(key).filter(|f| f.visible && supported(spec.id, key)) else { continue; };
                let Some(mut value) = values.get(key).cloned() else { continue; };
                let original = value.clone();
                let mut reset = false;
                let mut finish = false;
                ui.push_id((id,key), |ui| {
                    ui.horizontal(|ui| {
                        if let Value::Bool(v) = &mut value {
                            finish = ui.checkbox(v, field.label).changed();
                        } else { ui.label(field.label); }
                        if explicit.contains_key(key) {
                            reset = ui.small_button("Reset").on_hover_text("Use the default value").clicked();
                        }
                    });
                    match (&field.kind, &mut value) {
                        (PropertyType::Text, Value::Text(v)) => {
                            let r = ui.add(egui::TextEdit::multiline(v).desired_rows(4).desired_width(f32::INFINITY));
                            finish = r.lost_focus();
                        }
                        (PropertyType::Number {min,max,integer}, Value::Number(v)) => {
                            let mut edit = egui::DragValue::new(v).range(*min..=*max).speed(1.);
                            if *integer { edit = edit.max_decimals(0); }
                            let r = ui.add(edit);
                            finish = r.lost_focus() || r.drag_stopped();
                        }
                        (PropertyType::Choice(options), Value::Choice(v)) => {
                            egui::ComboBox::from_id_salt("value").selected_text(choice_label(v)).show_ui(ui, |ui| {
                                for option in *options { if ui.selectable_value(v, (*option).into(), choice_label(option)).clicked() { ui.close(); } }
                            });
                            finish = *v != match &original { Value::Choice(s) => s, _ => unreachable!() }.as_str();
                        }
                        (PropertyType::Color, Value::Color(v)) => {
                            let mut rgb = [v[0],v[1],v[2]];
                            ui.color_edit_button_srgb(&mut rgb);
                            v[..3].copy_from_slice(&rgb);
                        }
                        _ => {}
                    }
                });
                if reset {
                    self.editor.commit();
                    self.editor.begin(&format!("Reset {}", field.label));
                    if let Err(error) = self.editor.reset_control_property(id, key) { self.error = Some(error); }
                    self.editor.commit();
                } else if value != original {
                    self.editor.begin(&format!("Edit {}", field.label));
                    if let Err(error) = self.editor.set_control_property(id, key, value) { self.error = Some(error); }
                }
                if finish { self.editor.commit(); }
                if key == "text" && spec.id == "table" {
                    ui.small("Separate columns with commas and rows with new lines. End with weights such as {60L,20R,20C}.");
                }
            }
        });
        true
    }
}
