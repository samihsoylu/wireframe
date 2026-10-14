// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::{
    catalog::PropertyType,
    document::{Link, Value},
};

// Expose fields whose behavior the shared renderer currently implements. The
// catalog's full field inventory remains separate from completed rendering.
fn supported(kind: &str, key: &str) -> bool {
    match key {
        "row_links" | "row_icons" => kind == "list",
        "icon" | "icon_size" | "icon_rotation" => {
            matches!(
                kind,
                "button" | "dropdown" | "icon" | "icon_label" | "circle_button"
            )
        }
        "menu_icon" => kind == "button",
        "label_position" => matches!(kind, "icon_label" | "circle_button"),
        "text" | "font_size" | "bold" | "italic" | "underline" | "text_color" | "align" => true,
        "sketch_filter" | "image_border" => kind == "image",
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
                | "search"
                | "dropdown"
                | "icon_label"
                | "circle_button"
        ),
        "vertical_scrollbar" | "value" => matches!(kind, "dropdown" | "text_area" | "list"),
        "border_color" => matches!(kind, "dropdown" | "text_input" | "text_area"),
        "border" => matches!(
            kind,
            "text_input" | "list" | "icon" | "icon_label" | "circle_button"
        ),
        "background_opacity" => matches!(kind, "text_input" | "text_area"),
        "selected_index" => matches!(kind, "table" | "dropdown" | "list"),
        "shape" | "search_icon" | "microphone_icon" => kind == "search",
        "fill" => matches!(
            kind,
            "button"
                | "text_input"
                | "text_area"
                | "dropdown"
                | "search"
                | "table"
                | "list"
                | "circle_button"
        ),
        "alternate_fill" | "header" | "row_height" => matches!(kind, "table" | "list"),
        "horizontal_lines" | "vertical_lines" => kind == "table",
        _ => false,
    }
}
fn choice_label(value: &str) -> &str {
    match value {
        "normal" => "Normal",
        "selected" => "Selected",
        "focused" => "Focused",
        "closed" => "Closed",
        "disabled_closed" => "Disabled and closed",
        "focused_closed" => "Focused and closed",
        "disabled" => "Disabled",
        "disabled_selected" => "Selected and disabled",
        "indeterminate" => "Indeterminate",
        "disabled_indeterminate" => "Indeterminate and disabled",
        "bottom" => "Bottom",
        "left" => "Left",
        "center" => "Center",
        "right" => "Right",
        "rounded" => "Rounded corners",
        "rectangle" => "Rectangle",
        "solid" => "Show border",
        "none" => "No border",
        "line" => "Underline",
        other => other,
    }
}
impl WireframeApp {
    pub(super) fn typed_properties(&mut self, ui: &mut egui::Ui, id: Id) -> bool {
        let Some((spec, values)) = self.editor.control_properties(id) else {
            return false;
        };
        if spec.id == "image" {
            self.image_properties(ui, id, &values);
        }
        if !matches!(
            spec.id,
            "image"
                | "icon"
                | "label"
                | "heading"
                | "link"
                | "paragraph"
                | "text_area"
                | "button"
                | "text_input"
                | "dropdown"
                | "icon_label"
                | "circle_button"
                | "search"
                | "checkbox"
                | "radio"
                | "table"
                | "list"
        ) {
            return false;
        }
        let locked = self
            .editor
            .current()
            .controls
            .iter()
            .find(|n| n.id == id)
            .is_none_or(|n| n.locked);
        let mut import_row = None;
        let mut import_icon = false;
        let explicit = self.editor.explicit_control_properties(id);
        let local = self.editor.editing_instance();
        ui.add_space(10.);
        ui.add_enabled_ui(!locked, |ui| {
            if !matches!(spec.id,"image"|"icon") && spec.autosize != wireframe::catalog::AutoSize::None && ui.button("Auto-size").clicked() {
                self.editor.commit();
                self.editor.begin("Auto-size control");
                if let Err(error) = self.editor.auto_size_control(id) { self.error = Some(error); }
                self.editor.commit();
            }
            for key in ["image_border", "sketch_filter", "text", "label_position", "icon", "icon_size", "icon_rotation", "menu_icon", "row_links", "row_icons", "font_size", "bold", "italic", "underline", "align", "state", "shape", "search_icon", "microphone_icon", "vertical_scrollbar", "value", "text_color", "border", "border_color", "fill", "background_opacity", "header", "row_height", "selected_index", "alternate_fill", "horizontal_lines", "vertical_lines"] {
                if matches!(key, "icon_size" | "icon_rotation") && !matches!(values.get("icon"),Some(Value::Icon(_))) {continue;}
                if key == "value" && values.get("vertical_scrollbar") != Some(&Value::Bool(true)) { continue; }
                let Some(field) = spec.property(key).filter(|f| f.visible && supported(spec.id, key)) else { continue; };
                let Some(mut value) = values.get(key).cloned() else { continue; };
                let original = value.clone();
                let mut reset = false;
                let mut finish = false;
                ui.push_id((id,key), |ui| {
                    ui.horizontal(|ui| {
                        if let Value::Bool(v) = &mut value {
                            finish = ui.checkbox(v, field.label).changed();
                        } else if local && explicit.contains_key(key) { ui.colored_label(Color32::from_rgb(30,120,70), field.label); } else {ui.label(field.label);}
                        if explicit.contains_key(key) {
                            reset = ui.small_button("Reset").on_hover_text(if local {"Use the component source value"} else {"Use the default value"}).clicked();
                        }
                    });
                    match (&field.kind, &mut value) {
                        (PropertyType::Icon, v) => {
                            let mut chosen = match v {Value::Icon(source)=>Some(source.clone()),_=>None};
                            import_icon |= super::icon_picker::choice(ui,&mut chosen,&self.editor.document.assets);
                            *v = chosen.map(Value::Icon).unwrap_or(Value::Null);
                            finish = *v != original;
                        },
                        (PropertyType::Icons, Value::Icons(icons)) => {
                            let source=match values.get("text") {Some(Value::Text(s))=>s.as_str(),_=>""};
                            let labels:Vec<_>=source.split('\n').collect();
                            icons.resize(labels.len(),None);
                            egui::CollapsingHeader::new("Edit row icons").show(ui,|ui| {
                                egui::ScrollArea::vertical().id_salt("row-icons").max_height(280.).show(ui,|ui| {
                                    for (row,label) in labels.iter().enumerate() {
                                        ui.push_id(row,|ui| {
                                            let label=wireframe::rich_text::RichText::parse(label,false).display_text(false);
                                            ui.label(if label.is_empty(){format!("Row {} (empty)",row+1)}else{label});
                                            let before=icons[row].clone();
                                            if super::icon_picker::choice(ui,&mut icons[row],&self.editor.document.assets) {import_row = Some(row);}
                                            finish |= before != icons[row];
                                        });
                                    }
                                });
                            });
                            if !finish && let Value::Icons(old)=&original {*icons=old.clone();}
                        }
                        (PropertyType::Links, Value::Links(links)) => {
                            let source=match values.get("text") {Some(Value::Text(s))=>s.as_str(),_=>""};
                            let labels:Vec<_>=source.split('\n').collect();
                            links.resize(labels.len(),None);
                            // Keep partial URL input outside document history until Apply.
                            egui::CollapsingHeader::new("Edit row links").show(ui,|ui| {
                                egui::ScrollArea::vertical().max_height(280.).show(ui,|ui| {
                                    for (row,label) in labels.iter().enumerate() {
                                        ui.push_id(row,|ui| {
                                            let label=wireframe::rich_text::RichText::parse(label,false).display_text(false);
                                            ui.label(if label.is_empty(){format!("Row {} (empty)",row+1)}else{label});
                                            let key=ui.make_persistent_id(("link-draft",id,self.editor.surface_id(),&self.editor.context));
                                            let saved=links[row].clone();
                                            let cached=ui.ctx().data_mut(|d|d.get_temp::<(Option<Link>,Option<Link>)>(key));
                                            let mut draft=cached.filter(|(baseline,_)|baseline==&saved).map_or(saved.clone(),|(_,draft)|draft);
                                            let title=match &draft {None=>"No link".into(),Some(Link::Back)=>"Go back".into(),Some(Link::Url(_))=>"Web address".into(),Some(Link::Page(id))=>self.editor.document.pages.iter().find(|p|p.id==*id).map_or("Missing page".into(),|p|p.name.clone())};
                                            egui::ComboBox::from_id_salt("target").selected_text(title).show_ui(ui,|ui| {
                                                if ui.selectable_value(&mut draft,None,"No link").clicked(){ui.close();}
                                                for page in &self.editor.document.pages {
                                                    if ui.selectable_value(&mut draft,Some(Link::Page(page.id)),&page.name).clicked(){ui.close();}
                                                }
                                                if ui.selectable_value(&mut draft,Some(Link::Back),"Go back").clicked(){ui.close();}
                                                if ui.selectable_label(matches!(draft,Some(Link::Url(_))),"Web address…").clicked(){if !matches!(draft,Some(Link::Url(_))){draft=Some(Link::Url(String::new()));}ui.close();}
                                            });
                                            let apply=if let Some(Link::Url(url))=&mut draft {
                                                ui.add(egui::TextEdit::singleline(url).hint_text("https://…"));
                                                ui.add_enabled(Link::Url(url.clone()).valid(),egui::Button::new("Apply link")).clicked()
                                            }else{draft!=saved};
                                            if apply {links[row]=draft.clone();finish=true;}
                                            ui.ctx().data_mut(|d|d.insert_temp(key,(links[row].clone(),draft)));
                                        });
                                    }
                                });
                            });
                            // Do not create a property solely to pad absent trailing slots.
                            if !finish && let Value::Links(old)=&original {*links=old.clone();}
                        }
                        (PropertyType::Text, Value::Text(v)) => {
                            let r = ui.add(egui::TextEdit::multiline(v).desired_rows(4).desired_width(f32::INFINITY));
                            finish = r.lost_focus();
                        }
                        (PropertyType::Number {..}, Value::Number(v)) if key == "selected_index" && spec.id == "list" => {
                            let source=match values.get("text") {Some(Value::Text(s))=>s.as_str(),_=>""};
                            let labels=wireframe::list::labels(source,values.get("header")==Some(&Value::Bool(true))).iter().map(|s|wireframe::rich_text::RichText::parse(s,false).display_text(false)).collect::<Vec<_>>();
                            let current=if *v<0. {"None"} else {labels.get(*v as usize).map_or("No matching row",String::as_str)};
                            egui::ComboBox::from_id_salt("selection").selected_text(if current.is_empty(){"(Empty row)"}else{current}).show_ui(ui,|ui| {
                                if ui.selectable_value(v,-1.,"None").clicked(){ui.close();}
                                for (index,label) in labels.iter().enumerate() {
                                    if ui.selectable_value(v,index as f64,if label.is_empty(){"(Empty row)"}else{label}).clicked(){ui.close();}
                                }
                            });
                            finish=value!=original;
                        }
                        (PropertyType::Number {..}, Value::Number(v)) if key == "selected_index" && spec.id == "dropdown" => {
                            let source = match values.get("text") {Some(Value::Text(s)) => s.as_str(),_=>""};
                            let labels = wireframe::dropdown::selectable_labels(source);
                            let current = labels.get(*v as usize).copied().unwrap_or("No matching item");
                            egui::ComboBox::from_id_salt("selection").selected_text(if current.is_empty() {"(Empty title)"} else {current}).show_ui(ui, |ui| {
                                for (index,label) in labels.iter().enumerate() {
                                    if ui.selectable_value(v,index as f64,if label.is_empty() {"(Empty title)"} else {label}).clicked() {ui.close();}
                                }
                            });
                            finish = value != original;
                        }
                        (PropertyType::Number {..}, Value::Number(v)) if matches!(key, "value" | "background_opacity") => {
                            let r = ui.add(egui::Slider::new(v, 0.0..=1.0).fixed_decimals(2));
                            finish = r.lost_focus() || r.drag_stopped() || (r.changed() && !r.dragged());
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
                if key == "text" && spec.id == "dropdown" {
                    ui.small("First line is the title. Add menu items on new lines; a blank line is a separator. Prefix x or o plus a space for check or dot marks; end with > for a submenu arrow.");
                }
                if key == "text" && spec.id == "table" {
                    ui.small("Separate columns with commas and rows with new lines. End with weights such as {60L,20R,20C}.");
                }
            }
        });
        if import_icon {
            self.start_image_import(wireframe::workspace::ImageTarget::Icon(id));
        }
        if let Some(row) = import_row {
            self.start_image_import(wireframe::workspace::ImageTarget::ListRow {
                control: id,
                row,
            });
        }
        true
    }
}
