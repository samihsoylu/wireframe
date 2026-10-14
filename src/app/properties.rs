// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::{
    catalog::PropertyType,
    document::{Link, Value},
};

// Slider preserves its numeric editor's focus in its combined response.
// Intermediate digits preview live but belong to one undoable edit.
fn fraction_slider(ui: &mut egui::Ui, value: &mut f64) -> (egui::Response, bool) {
    let response = ui.add(egui::Slider::new(value, 0.0..=1.0).fixed_decimals(2));
    let finish = response.lost_focus()
        || response.drag_stopped()
        || (response.changed() && !response.dragged() && !response.has_focus());
    (response, finish)
}

// Expose fields whose behavior the shared renderer currently implements. The
// catalog's full field inventory remains separate from completed rendering.
fn supported(kind: &str, key: &str) -> bool {
    match key {
        "device_model" | "background_pattern" => kind == "phone",
        "keyboard_device" => kind == "mobile_keyboard",
        "orientation" => matches!(kind, "phone" | "tablet" | "smartphone" | "mobile_keyboard"),
        "device_resizing" | "transparent_background" | "status_bar" => {
            matches!(kind, "phone" | "tablet" | "smartphone")
        }
        "row_links" => matches!(
            kind,
            "list"
                | "button_bar"
                | "breadcrumbs"
                | "link_bar"
                | "tabs"
                | "vertical_tabs"
                | "checkbox_group"
                | "radio_group"
                | "menu"
                | "menu_bar"
                | "mobile_menu"
                | "tree"
                | "accordion"
                | "alert"
                | "app_bar"
        ),
        "row_icons" => kind == "list",
        "icon" | "icon_size" | "icon_rotation" => {
            matches!(
                kind,
                "button"
                    | "dropdown"
                    | "icon"
                    | "icon_label"
                    | "circle_button"
                    | "pointed_button"
                    | "multiline_button"
                    | "menu_bar"
            )
        }
        "menu_icon" => matches!(kind, "button" | "pointed_button" | "app_bar"),
        "nav_icon" | "filter_icon" | "icons" => kind == "app_bar",
        "button_style" => kind == "alert",
        "header_height" | "footer_height" | "minimize" | "maximize" | "close" | "resize_handle" => {
            kind == "window"
        }
        "tab_position" => matches!(kind, "tabs" | "vertical_tabs"),
        "direction" => kind == "pointed_button",
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
                | "pointed_button"
                | "tree"
        ),
        "vertical_scrollbar" | "value" => matches!(
            kind,
            "dropdown"
                | "text_area"
                | "list"
                | "tabs"
                | "vertical_tabs"
                | "tree"
                | "accordion"
                | "browser"
                | "window"
        ),
        "border_color" => matches!(kind, "dropdown" | "text_input" | "text_area" | "link_bar"),
        "border" => matches!(
            kind,
            "menu_bar"
                | "mobile_menu"
                | "browser"
                | "alert"
                | "app_bar"
                | "tree"
                | "text_input"
                | "list"
                | "icon"
                | "icon_label"
                | "circle_button"
                | "pointed_button"
                | "tabs"
                | "vertical_tabs"
        ),
        "background_opacity" => matches!(
            kind,
            "text_input"
                | "text_area"
                | "multiline_button"
                | "tabs"
                | "vertical_tabs"
                | "tree"
                | "field_set"
        ),
        "selected_index" => matches!(
            kind,
            "table"
                | "dropdown"
                | "list"
                | "button_bar"
                | "link_bar"
                | "tabs"
                | "vertical_tabs"
                | "menu"
                | "menu_bar"
                | "tree"
                | "accordion"
        ),
        "shape" | "search_icon" | "microphone_icon" => kind == "search",
        "fill" => matches!(
            kind,
            "app_bar"
                | "field_set"
                | "browser"
                | "button"
                | "text_input"
                | "text_area"
                | "dropdown"
                | "search"
                | "table"
                | "list"
                | "circle_button"
                | "pointed_button"
                | "multiline_button"
                | "tabs"
                | "vertical_tabs"
                | "tree"
        ),
        "alternate_fill" | "header" | "row_height" => matches!(kind, "table" | "list"),
        "horizontal_lines" | "vertical_lines" => kind == "table",
        _ => false,
    }
}
fn choice_label(value: &str) -> &str {
    match value {
        "classic" => "Classic",
        "tall" => "Tall",
        "wide" => "Wide",
        "notched" => "Notched",
        "phone" => "Phone",
        "tablet" => "Tablet",
        "portrait" => "Portrait",
        "landscape" => "Landscape",
        "stretch" => "Stretch",
        "fit" => "Maintain proportions",
        "white" => "White",
        "header" => "White with header",
        "header_footer" => "White with header and footer",
        "header_dark_footer" => "White with header and dark footer",
        "dark" => "Dark",
        "normal" => "Normal",
        "divided" => "Divided",
        "inline" => "Inline",
        "selected" => "Selected",
        "focused" => "Focused",
        "closed" => "Closed",
        "disabled_closed" => "Disabled and closed",
        "focused_closed" => "Focused and closed",
        "disabled" => "Disabled",
        "disabled_selected" => "Selected and disabled",
        "indeterminate" => "Indeterminate",
        "disabled_indeterminate" => "Indeterminate and disabled",
        "top" => "Top",
        "bottom" => "Bottom",
        "left" => "Left",
        "center" => "Center",
        "right" => "Right",
        "rounded" => "Rounded corners",
        "rectangle" => "Rectangle",
        "solid" => "Show border",
        "torn_bottom" => "Torn bottom",
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
                | "pointed_button"
                | "multiline_button"
                | "search"
                | "checkbox"
                | "radio"
                | "checkbox_group"
                | "radio_group"
                | "menu"
                | "menu_bar"
                | "mobile_menu"
                | "tree"
                | "accordion"
                | "alert"
                | "app_bar"
                | "field_set"
                | "phone"
                | "tablet"
                | "smartphone"
                | "mobile_keyboard"
                | "mobile_picker"
                | "browser"
                | "window"
                | "table"
                | "list"
                | "button_bar"
                | "breadcrumbs"
                | "link_bar"
                | "tabs"
                | "vertical_tabs"
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
            if !matches!(spec.id,"image"|"icon"|"breadcrumbs"|"link_bar") && spec.autosize != wireframe::catalog::AutoSize::None && ui.button("Auto-size").clicked() {
                self.editor.commit();
                self.editor.begin("Auto-size control");
                if let Err(error) = self.editor.auto_size_control(id) { self.error = Some(error); }
                self.editor.commit();
            }
            for key in ["keyboard_device", "device_model", "orientation", "device_resizing", "background_pattern", "transparent_background", "status_bar", "image_border", "sketch_filter", "text", "header_height", "footer_height", "minimize", "maximize", "close", "resize_handle", "button_style", "tab_position", "direction", "label_position", "icon", "icon_size", "icon_rotation", "nav_icon", "filter_icon", "menu_icon", "icons", "row_links", "row_icons", "font_size", "bold", "italic", "underline", "align", "state", "shape", "search_icon", "microphone_icon", "vertical_scrollbar", "value", "text_color", "border", "border_color", "fill", "background_opacity", "header", "row_height", "selected_index", "alternate_fill", "horizontal_lines", "vertical_lines"] {
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
                        (PropertyType::Icons, Value::Icons(icons)) if spec.id == "app_bar" => {
                            egui::CollapsingHeader::new("Edit custom icons").show(ui, |ui| {
                                let mut remove = None;
                                let mut swap = None;
                                for row in 0..icons.len() {
                                    ui.push_id(row, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(format!("Icon {}", row + 1));
                                            if ui.add_enabled(row > 0, egui::Button::new("Move left")).clicked() { swap = Some((row, row - 1)); }
                                            if ui.add_enabled(row + 1 < icons.len(), egui::Button::new("Move right")).clicked() { swap = Some((row, row + 1)); }
                                        });
                                        let before = icons[row].clone();
                                        if super::icon_picker::choice(ui, &mut icons[row], &self.editor.document.assets) { import_row = Some(row); }
                                        if icons[row].is_none() && before.is_some() { remove = Some(row); }
                                        finish |= before != icons[row];
                                    });
                                }
                                if let Some(row) = remove { icons.remove(row); }
                                if let Some((a,b)) = swap { icons.swap(a,b); finish = true; }
                                ui.push_id("add", |ui| {
                                    ui.label("Add icon");
                                    let mut chosen = None;
                                    if super::icon_picker::choice(ui, &mut chosen, &self.editor.document.assets) { import_row = Some(icons.len()); }
                                    if chosen.is_some() { icons.push(chosen); finish = true; }
                                });
                            });
                        }
                        (PropertyType::Icons, Value::Icons(icons)) => {
                            let source=match values.get("text") {Some(Value::Text(s))=>s.as_str(),_=>""};
                            let labels:Vec<_>=source.split('\n').collect();
                            icons.resize(labels.len(),None);
                            egui::CollapsingHeader::new("Edit row icons").show(ui,|ui| {
                                egui::ScrollArea::vertical().id_salt("row-icons").max_height(280.).show(ui,|ui| {
                                    for (row,label) in labels.iter().enumerate() {
                                        ui.push_id(row,|ui| {
                                            let label=wireframe::rich_text::RichText::parse(label,false).display_text(false);
                                            ui_text::label(ui,&if label.is_empty(){format!("Row {} (empty)",row+1)}else{label},ui.available_width());
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
                            let labels:Vec<String>=if spec.id=="app_bar" {wireframe::app_bar::labels(&values)} else if spec.id=="alert" {wireframe::alert::labels(source)} else if spec.id=="accordion" {wireframe::accordion::labels(source)} else if spec.id=="tree" {wireframe::tree::labels(source)} else if matches!(spec.id,"menu"|"menu_bar") {wireframe::menu::labels(source,spec.id=="menu_bar")} else if matches!(spec.id,"tabs"|"vertical_tabs") {wireframe::tabs::labels(source,spec.id=="vertical_tabs")} else if matches!(spec.id,"breadcrumbs"|"link_bar") {wireframe::navigation::labels(source,spec.id=="breadcrumbs")} else if spec.id=="button_bar" {wireframe::button_bar::labels(source).into_iter().map(str::to_owned).collect()} else {source.split('\n').map(str::to_owned).collect()};
                            links.resize(labels.len(),None);
                            // Keep partial URL input outside document history until Apply.
                            egui::CollapsingHeader::new(if spec.id=="app_bar" {"Edit icon links"} else if spec.id=="alert" {"Edit button links"} else if matches!(spec.id,"tabs"|"vertical_tabs") {"Edit tab links"} else if spec.id=="button_bar" {"Edit segment links"} else if matches!(spec.id,"breadcrumbs"|"link_bar"|"menu_bar") {"Edit item links"} else {"Edit row links"}).show(ui,|ui| {
                                egui::ScrollArea::vertical().max_height(280.).show(ui,|ui| {
                                    for (row,label) in labels.iter().enumerate() {
                                        ui.push_id(row,|ui| {
                                            let label=wireframe::rich_text::RichText::parse(label,false).display_text(false);
                                            ui_text::label(ui,&if label.is_empty(){format!("{} {} (empty)",if matches!(spec.id,"tabs"|"vertical_tabs") {"Tab"} else if spec.id=="button_bar" {"Segment"} else if matches!(spec.id,"breadcrumbs"|"link_bar"|"menu_bar") {"Item"} else {"Row"},row+1)}else{label},ui.available_width());
                                            let key=ui.make_persistent_id(("link-draft",id,self.editor.surface_id(),&self.editor.context));
                                            let saved=links[row].clone();
                                            let cached=ui.ctx().data_mut(|d|d.get_temp::<(Option<Link>,Option<Link>)>(key));
                                            let mut draft=cached.filter(|(baseline,_)|baseline==&saved).map_or(saved.clone(),|(_,draft)|draft);
                                            let title=match &draft {None=>"No link".into(),Some(Link::Back)=>"Go back".into(),Some(Link::Url(_))=>"Web address".into(),Some(Link::Page(id))=>self.editor.document.pages.iter().find(|p|p.id==*id).map_or("Missing page".into(),|p|p.name.clone())};
                                            ui_text::combo_box(ui,"target",&title,|ui| {
                                                if ui_text::selectable_value(ui,&mut draft,None,"No link").clicked(){ui.close();}
                                                for page in &self.editor.document.pages {
                                                    if ui_text::selectable_value(ui,&mut draft,Some(Link::Page(page.id)),&page.name).clicked(){ui.close();}
                                                }
                                                if ui_text::selectable_value(ui,&mut draft,Some(Link::Back),"Go back").clicked(){ui.close();}
                                                if ui.selectable_label(matches!(draft,Some(Link::Url(_))),"Web address…").clicked(){if !matches!(draft,Some(Link::Url(_))){draft=Some(Link::Url(String::new()));}ui.close();}
                                            });
                                            let apply=if let Some(Link::Url(url))=&mut draft {
                                                ui.add(TextInput::singleline(url).hint_text("https://…"));
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
                            let r = wireframe::text_entry::multiline(ui, v, 4, false);
                            finish = r.lost_focus();
                        }
                        (PropertyType::Number {..}, Value::Number(v)) if key == "selected_index" && matches!(spec.id, "list" | "button_bar" | "link_bar" | "tabs" | "vertical_tabs" | "menu" | "menu_bar" | "tree" | "accordion") => {
                            let source=match values.get("text") {Some(Value::Text(s))=>s.as_str(),_=>""};
                            let raw:Vec<String>=if spec.id=="accordion" {wireframe::accordion::labels(source)} else if spec.id=="tree" {wireframe::tree::labels(source)} else if matches!(spec.id,"menu"|"menu_bar") {wireframe::menu::labels(source,spec.id=="menu_bar")} else if matches!(spec.id,"tabs"|"vertical_tabs") {wireframe::tabs::labels(source,spec.id=="vertical_tabs")} else if spec.id=="link_bar" {wireframe::navigation::labels(source,false)} else if spec.id=="button_bar" {wireframe::button_bar::labels(source).into_iter().map(str::to_owned).collect()} else {wireframe::list::labels(source,values.get("header")==Some(&Value::Bool(true))).into_iter().map(str::to_owned).collect()};
                            let labels=raw.iter().map(|s|wireframe::rich_text::RichText::parse(s,false).display_text(false)).collect::<Vec<_>>();
                            let current=if *v<0. {"None"} else {labels.get(*v as usize).map_or("No matching item",String::as_str)};
                            ui_text::combo_box(ui,"selection",if current.is_empty(){"(Empty item)"}else{current},|ui| {
                                if ui_text::selectable_value(ui,v,-1.,"None").clicked(){ui.close();}
                                for (index,label) in labels.iter().enumerate() {
                                    if ui_text::selectable_value(ui,v,index as f64,if label.is_empty(){"(Empty item)"}else{label}).clicked(){ui.close();}
                                }
                            });
                            finish=value!=original;
                        }
                        (PropertyType::Number {..}, Value::Number(v)) if key == "selected_index" && spec.id == "dropdown" => {
                            let source = match values.get("text") {Some(Value::Text(s)) => s.as_str(),_=>""};
                            let labels = wireframe::dropdown::selectable_labels(source);
                            let current = labels.get(*v as usize).copied().unwrap_or("No matching item");
                            ui_text::combo_box(ui,"selection",if current.is_empty() {"(Empty title)"} else {current}, |ui| {
                                for (index,label) in labels.iter().enumerate() {
                                    if ui_text::selectable_value(ui,v,index as f64,if label.is_empty() {"(Empty title)"} else {label}).clicked() {ui.close();}
                                }
                            });
                            finish = value != original;
                        }
                        (PropertyType::Number {..}, Value::Number(v)) if matches!(key, "value" | "background_opacity") => {
                            finish = fraction_slider(ui, v).1;
                        }
                        (PropertyType::Number {min,max,integer}, Value::Number(v)) => {
                            let mut edit = egui::DragValue::new(v).range(*min..=*max).speed(1.);
                            if *integer { edit = edit.max_decimals(0); }
                            let r = ui.add(edit);
                            finish = r.lost_focus() || r.drag_stopped();
                        }
                        (PropertyType::Choice(options), Value::Choice(v)) => {
                            egui::ComboBox::from_id_salt("value").selected_text(choice_label(v)).show_ui(ui, |ui| {
                                for option in *options {
                                    if spec.id == "browser" && key == "border" && !matches!(*option, "solid" | "torn_bottom") {
                                        continue;
                                    }
                                    if ui.selectable_value(v, (*option).into(), choice_label(option)).clicked() {
                                        ui.close();
                                    }
                                }
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
            self.start_image_import(if spec.id == "app_bar" {
                wireframe::workspace::ImageTarget::AppBarIcon {
                    control: id,
                    index: row,
                }
            } else {
                wireframe::workspace::ImageTarget::ListRow { control: id, row }
            });
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SliderFixture {
        ctx: egui::Context,
        editor: Editor,
        id: Id,
        time: f64,
    }
    impl SliderFixture {
        fn new(kind: Kind) -> Self {
            let mut editor = Editor::default();
            editor.add(kind, 0., 0.);
            let id = *editor.selection.iter().next().unwrap();
            Self {
                ctx: egui::Context::default(),
                editor,
                id,
                time: 0.,
            }
        }
        fn value(&self, key: &str) -> f64 {
            match self.editor.control_properties(self.id).unwrap().1[key] {
                Value::Number(v) => v,
                _ => unreachable!(),
            }
        }
        fn frame(
            &mut self,
            key: &str,
            events: Vec<egui::Event>,
            blur: bool,
        ) -> (egui::Response, bool) {
            self.time += 0.1;
            let mut result = None;
            let ctx = self.ctx.clone();
            let mut output = ctx.run_ui(
                egui::RawInput {
                    time: Some(self.time),
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400., 200.))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    if blur && let Some(id) = ui.memory(|m| m.focused()) {
                        ui.memory_mut(|m| m.surrender_focus(id));
                    }
                    let original = self.value(key);
                    let mut value = original;
                    let (response, finish) = fraction_slider(ui, &mut value);
                    if value != original {
                        self.editor.begin("Edit fraction");
                        self.editor
                            .set_control_property(self.id, key, Value::Number(value))
                            .unwrap();
                    }
                    if finish {
                        self.editor.commit();
                    }
                    result = Some((response, finish));
                },
            );
            output.textures_delta.clear();
            result.unwrap()
        }
    }
    fn pointer(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }
    #[test]
    fn fractional_typing_is_one_undo_step_on_enter_or_blur() {
        for (kind, key) in [
            (Kind::FieldSet, "background_opacity"),
            (Kind::Browser, "value"),
        ] {
            for blur in [false, true] {
                let mut f = SliderFixture::new(kind);
                let baseline = f.value(key);
                let (response, _) = f.frame(key, vec![], false);
                let pos = Pos2::new(response.rect.right() - 10., response.rect.center().y);
                f.frame(
                    key,
                    vec![egui::Event::PointerMoved(pos), pointer(pos, true)],
                    false,
                );
                f.frame(key, vec![pointer(pos, false)], false);
                f.frame(key, vec![], false);
                // Numeric editor auto-selects its contents on focus.
                let (r, finish) = f.frame(key, vec![egui::Event::Text("0".into())], false);
                assert!(r.has_focus());
                assert!(!finish);
                assert_eq!(f.value(key), 0.);
                f.frame(key, vec![egui::Event::Text(".".into())], false);
                let (_, finish) = f.frame(key, vec![egui::Event::Text("5".into())], false);
                assert!(!finish);
                assert_eq!(f.value(key), 0.5);
                let events = if blur {
                    vec![]
                } else {
                    vec![egui::Event::Key {
                        key: Key::Enter,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::NONE,
                    }]
                };
                let (_, finish) = f.frame(key, events, blur);
                assert!(finish);
                f.editor.undo();
                assert_eq!(f.value(key), baseline);
                f.editor.redo();
                assert_eq!(f.value(key), 0.5);
            }
        }
    }
    #[test]
    fn fractional_drag_previews_live_and_commits_once_on_release() {
        let mut f = SliderFixture::new(Kind::FieldSet);
        let key = "background_opacity";
        let (response, _) = f.frame(key, vec![], false);
        let start = Pos2::new(response.rect.left() + 20., response.rect.center().y);
        let end = start + Vec2::new(40., 0.);
        let (_, finish) = f.frame(
            key,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
            false,
        );
        assert!(!finish);
        let (_, finish) = f.frame(key, vec![egui::Event::PointerMoved(end)], false);
        assert!(!finish);
        let value = f.value(key);
        assert!(value > 0. && value < 1.);
        let (_, finish) = f.frame(key, vec![pointer(end, false)], false);
        assert!(finish);
        f.editor.undo();
        assert_eq!(f.value(key), 1.);
        f.editor.redo();
        assert_eq!(f.value(key), value);
    }
}
