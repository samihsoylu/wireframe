// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::{
    catalog::PropertyType,
    document::{Link, Value},
    workspace::{LinkAction, LinkTarget},
};
#[cfg(test)]
mod keyboard_tests;
#[cfg(test)]
mod link_action_tests;

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
pub(super) fn supported(kind: &str, key: &str) -> bool {
    match key {
        // The catalog's visibility flag decides which controls offer a target;
        // all whole-control targets already share the scene link renderer.
        "link" => true,
        "stroke" => matches!(kind, "vertical_rule" | "horizontal_rule" | "arrow"),
        "shape" => matches!(kind, "shape" | "search" | "arrow"),
        "start_head" | "end_head" => kind == "arrow",
        "shape_rotation" => kind == "shape",
        "arrow_position" => kind == "popover",
        "progress_style" | "indeterminate" => kind == "progress",
        "slider_thumb" => matches!(kind, "horizontal_slider" | "vertical_slider"),
        "date_mode" => kind == "date_picker",
        "device_model" | "background_pattern" => kind == "phone",
        "keyboard_device" => kind == "mobile_keyboard",
        "text_orientation" => kind == "label",
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
                | "date_picker"
                | "time_picker"
                | "alert"
                | "app_bar"
                | "site_map"
        ),
        "row_icons" => kind == "list",
        "icon" | "icon_size" | "icon_rotation" => {
            matches!(
                kind,
                "button"
                    | "shape"
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
        "direction" => matches!(
            kind,
            "pointed_button"
                | "tooltip"
                | "horizontal_brace"
                | "vertical_brace"
                | "popover"
                | "site_map"
        ),
        "label_position" => matches!(kind, "icon_label" | "circle_button" | "arrow"),
        "text" | "font_size" | "bold" | "italic" | "underline" | "text_color" | "align" => true,
        "sketch_filter" => matches!(kind, "image" | "cover_flow"),
        "image_border" => kind == "image",
        "state" => matches!(
            kind,
            "checkbox"
                | "toggle"
                | "radio"
                | "label"
                | "link"
                | "paragraph"
                | "button"
                | "text_input"
                | "text_area"
                | "search"
                | "horizontal_slider"
                | "vertical_slider"
                | "numeric_stepper"
                | "date_chooser"
                | "dropdown"
                | "icon_label"
                | "circle_button"
                | "pointed_button"
                | "tree"
        ),
        "value"
            if matches!(
                kind,
                "progress"
                    | "horizontal_slider"
                    | "vertical_slider"
                    | "horizontal_scrollbar"
                    | "vertical_scrollbar"
            ) =>
        {
            true
        }
        "vertical_scrollbar" | "value" => matches!(
            kind,
            "dropdown"
                | "text_area"
                | "list"
                | "table"
                | "tabs"
                | "vertical_tabs"
                | "tree"
                | "accordion"
                | "browser"
                | "window"
                | "cover_flow"
        ),
        "border_color" => matches!(
            kind,
            "arrow"
                | "rectangle"
                | "shape"
                | "vertical_rule"
                | "horizontal_rule"
                | "dropdown"
                | "text_input"
                | "text_area"
                | "link_bar"
                | "numeric_stepper"
                | "date_chooser"
        ),
        "border" => matches!(
            kind,
            "rectangle"
                | "menu_bar"
                | "mobile_menu"
                | "browser"
                | "alert"
                | "app_bar"
                | "site_map"
                | "tree"
                | "text_input"
                | "list"
                | "table"
                | "icon"
                | "icon_label"
                | "circle_button"
                | "pointed_button"
                | "tabs"
                | "vertical_tabs"
        ),
        "background_opacity" => matches!(
            kind,
            "arrow"
                | "rectangle"
                | "shape"
                | "vertical_rule"
                | "horizontal_rule"
                | "text_input"
                | "text_area"
                | "multiline_button"
                | "tabs"
                | "vertical_tabs"
                | "tree"
                | "field_set"
                | "callout"
                | "scratch_out"
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
        "search_icon" | "microphone_icon" => kind == "search",
        "fill" => matches!(
            kind,
            "progress"
                | "shape"
                | "toggle"
                | "rectangle"
                | "color_picker"
                | "popover"
                | "callout"
                | "scratch_out"
                | "sticky_note"
                | "app_bar"
                | "site_map"
                | "horizontal_slider"
                | "vertical_slider"
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
        "top_left" => "Top left",
        "top_right" => "Top right",
        "bottom_left" => "Bottom left",
        "bottom_right" => "Bottom right",
        "vertical" => "Vertical",
        "horizontal" => "Horizontal",
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
        "on" => "On",
        "off" => "Off",
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
        "ellipse" => "Ellipse",
        "rounded_rectangle" => "Rounded rectangle",
        "dotted_rectangle" => "Dashed rectangle",
        "diamond" => "Diamond",
        "star" => "Star",
        "parallelogram" => "Parallelogram",
        "triangle" => "Triangle",
        "torn_top" => "Torn top",
        "breakline_top" => "Breakline top",
        "breakline_bottom" => "Breakline bottom",
        "solid" => "Show border",
        "torn_bottom" => "Torn bottom",
        "none" => "No border",
        "line" => "Underline",
        other => other,
    }
}
pub(super) fn field_choice_label<'a>(kind: &str, key: &str, value: &'a str) -> &'a str {
    if key == "text_orientation" {
        match value {
            "upright" => "Upright",
            "left" => "Sideways Left",
            "right" => "Sideways Right",
            "diagonal" => "Diagonal",
            other => other,
        }
    } else if key == "stroke" {
        match value {
            "solid" => "Solid",
            "dashed" => "Dashed",
            "dotted" => "Dotted",
            other => choice_label(other),
        }
    } else if kind == "rectangle" && key == "border" {
        match value {
            "solid" => "Square",
            "rounded" => "Rounded",
            "dashed" | "dotted" => "Dashed",
            "none" => "No border",
            other => other,
        }
    } else {
        choice_label(value)
    }
}
impl WireframeApp {
    fn link_choice(
        &self,
        ui: &mut egui::Ui,
        id: Id,
        saved: Option<Link>,
        mixed: bool,
    ) -> Option<LinkAction> {
        self.link_choice_caption(ui, id, saved, mixed, None)
    }
    fn link_choice_caption(
        &self,
        ui: &mut egui::Ui,
        id: Id,
        saved: Option<Link>,
        mixed: bool,
        caption: Option<&str>,
    ) -> Option<LinkAction> {
        #[cfg(test)]
        ui.ctx().data_mut(|d| {
            let key = egui::Id::new("whole-link-picker-count");
            let mut fields = d.get_temp::<HashSet<egui::Id>>(key).unwrap_or_default();
            fields.insert(ui.make_persistent_id("target"));
            d.insert_temp(key, fields);
        });
        let key = ui.make_persistent_id((
            "whole-link-draft",
            id,
            self.editor.surface_id(),
            &self.editor.context,
            self.editor.clipboard_session(),
        ));
        let cached = ui
            .ctx()
            .data_mut(|d| d.get_temp::<(Option<Link>, Option<Link>)>(key));
        let mut draft = cached
            .filter(|(baseline, _)| baseline == &saved)
            .map_or(saved.clone(), |(_, draft)| draft);
        let title = if mixed {
            "Mixed links".into()
        } else {
            match &draft {
                None => "No link".into(),
                Some(Link::Back) => "Go back".into(),
                Some(Link::Url(_)) => "Web address".into(),
                Some(Link::Page(id)) => self
                    .editor
                    .document
                    .live_page(*id)
                    .map_or("Missing page".into(), |p| p.name.clone()),
            }
        };
        let mut creation = None;
        let mut picked = false;
        ui_text::combo_box(ui, "target", caption.unwrap_or(&title), |ui| {
            if ui_text::selectable_value(ui, &mut draft, None, "No link").clicked() {
                picked = true;
                ui.close();
            }
            for page in self.editor.document.live_pages() {
                if ui_text::selectable_value(ui, &mut draft, Some(Link::Page(page.id)), &page.name)
                    .clicked()
                {
                    picked = true;
                    ui.close();
                }
            }
            for (label, action) in [
                ("Link to new wireframe", LinkAction::NewPage),
                (
                    "Link to duplicate of this wireframe",
                    LinkAction::DuplicatePage,
                ),
            ] {
                if ui.button(label).clicked() {
                    creation = Some(action);
                    ui.close();
                }
            }
            if ui_text::selectable_value(ui, &mut draft, Some(Link::Back), "Go back").clicked() {
                picked = true;
                ui.close();
            }
            if ui
                .selectable_label(matches!(draft, Some(Link::Url(_))), "Web address…")
                .clicked()
            {
                if !matches!(draft, Some(Link::Url(_))) {
                    draft = Some(Link::Url(String::new()));
                }
                ui.close();
            }
        });
        let apply = if let Some(Link::Url(url)) = &mut draft {
            ui.add(TextInput::singleline(url).hint_text("https://…"));
            let mut cancelled = false;
            let apply = ui
                .horizontal(|ui| {
                    let response = ui.add_enabled(
                        Link::Url(url.clone()).valid(),
                        egui::Button::new("Apply link"),
                    );
                    #[cfg(test)]
                    ui.ctx().data_mut(|d| {
                        d.insert_temp(egui::Id::new("whole-link-apply-test"), response.rect)
                    });
                    cancelled = ui.button("Cancel").clicked();
                    response.clicked()
                })
                .inner;
            if cancelled {
                draft = saved.clone();
            }
            apply
        } else {
            draft != saved || (mixed && picked)
        };
        let result = apply.then(|| draft.clone());
        ui.ctx()
            .data_mut(|d| d.insert_temp(key, (result.clone().unwrap_or(saved), draft)));
        creation.or_else(|| result.map(LinkAction::Set))
    }
    pub(super) fn whole_link_properties(&mut self, ui: &mut egui::Ui, id: Id) {
        if self.editor.library.is_some() && self.editor.context.is_empty() {
            return;
        }
        ui.push_id((id, "whole-link"), |ui| {
            ui.label("Link");
            let saved = self.editor.node_link(id);
            if let Some(action) = self.link_choice(ui, id, saved, false) {
                self.apply_link_action(ui, &[LinkTarget::whole(id)], action);
            }
        });
    }
    fn apply_link_action(&mut self, ui: &mut egui::Ui, targets: &[LinkTarget], action: LinkAction) {
        let result = self.editor.edit_links(targets, action);
        self.link_action_result(ui, result);
    }
    fn link_action_result(&mut self, ui: &mut egui::Ui, result: Result<Option<Id>, String>) {
        match result {
            Ok(Some(page)) => {
                let key = egui::Id::new(("linked-page-notice", self.editor.clipboard_session()));
                ui.ctx().data_mut(|d| d.insert_temp(key, page));
                ui.ctx().request_repaint();
            }
            Ok(None) => {}
            Err(error) => self.error = Some(error),
        }
    }
    pub(super) fn linked_destination_notice(&mut self, ui: &mut egui::Ui) -> bool {
        let key = egui::Id::new(("linked-page-notice", self.editor.clipboard_session()));
        let Some(page) = ui.ctx().data_mut(|d| d.get_temp::<Id>(key)) else {
            return false;
        };
        let Some(name) = self.editor.document.live_page(page).map(|p| p.name.clone()) else {
            ui.ctx().data_mut(|d| d.remove::<Id>(key));
            return false;
        };
        ui.add(egui::Label::new(format!("Created linked wireframe: {name}")).truncate());
        let mut open = false;
        ui.horizontal(|ui| {
            open = ui.button("Open wireframe").clicked();
            if ui.small_button("Dismiss").clicked() || open {
                ui.ctx().data_mut(|d| d.remove::<Id>(key));
            }
        });
        ui.separator();
        if open {
            self.editor.switch_page(page);
        }
        open
    }
    pub(super) fn whole_link_available(&self, c: &wireframe::model::Control) -> bool {
        if self.editor.library.is_some() && self.editor.context.is_empty() {
            return false;
        }
        self.editor
            .control_properties(c.id)
            .is_some_and(|(spec, _)| spec.property("link").is_some_and(|f| f.visible))
            || c.group.is_some()
            || (self.editor.linear_axis(c.id).is_none()
                && !matches!(
                    c.kind,
                    Kind::Shape
                        | Kind::Ellipse
                        | Kind::Toggle
                        | Kind::SiteMap
                        | Kind::Popover
                        | Kind::StreetMap
                        | Kind::ModalOverlay
                        | Kind::ColorPicker
                        | Kind::HelpButton
                        | Kind::ButtonBar
                        | Kind::Breadcrumbs
                        | Kind::LinkBar
                        | Kind::Tabs
                        | Kind::VerticalTabs
                        | Kind::CheckboxGroup
                        | Kind::RadioGroup
                        | Kind::Menu
                        | Kind::MenuBar
                        | Kind::Tree
                        | Kind::Accordion
                        | Kind::Alert
                        | Kind::AppBar
                        | Kind::Playback
                        | Kind::Webcam
                        | Kind::Video
                        | Kind::CoverFlow
                        | Kind::ColumnChart
                        | Kind::BarChart
                        | Kind::LineChart
                        | Kind::PieChart
                        | Kind::HorizontalBrace
                        | Kind::VerticalBrace
                        | Kind::RedCross
                        | Kind::ScratchOut
                        | Kind::TextBlock
                        | Kind::TextLine
                        | Kind::TagCloud
                        | Kind::Callout
                        | Kind::StickyNote
                        | Kind::Tooltip
                ))
    }
    pub(super) fn selection_links(&mut self, ui: &mut egui::Ui) {
        let selected = self.editor.selected_views();
        let ids = self.editor.ordered_selection();
        if ids.len() < 2 {
            return;
        }
        ui.push_id(("selection-links", &ids), |ui| {
            if selected.iter().all(|c| self.whole_link_available(c)) {
                ui.label("Link");
                let targets: Vec<_> = ids.iter().map(|&id| LinkTarget::whole(id)).collect();
                if let Some(action) = self.common_link_choice(ui, &targets) {
                    self.apply_link_action(ui, &targets, action);
                }
            }
            let labels: Option<Vec<_>> = ids
                .iter()
                .map(|&id| self.editor.link_row_labels(id))
                .collect();
            if let Some(mut labels) = labels {
                let count = labels.iter().map(Vec::len).max().unwrap_or(0);
                if self.editor.common_row_targets(&ids, 0).is_err() {
                    return;
                }
                if ids.iter().all(|&id| {
                    self.editor
                        .control_properties(id)
                        .is_some_and(|(spec, _)| spec.id == "app_bar")
                }) {
                    for labels in &mut labels {
                        let menu = labels.pop().unwrap();
                        labels.resize(count - 1, "-".into());
                        labels.push(menu);
                    }
                }
                egui::CollapsingHeader::new("Edit row links").show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(280.)
                        .show(ui, |ui| {
                            for row in 0..count {
                                ui.push_id(row, |ui| {
                                    ui_text::label(
                                        ui,
                                        &labels
                                            .iter()
                                            .map(|v| v.get(row).map_or("-", String::as_str))
                                            .collect::<Vec<_>>()
                                            .join(" / "),
                                        ui.available_width(),
                                    );
                                    let targets = self
                                        .editor
                                        .common_row_targets(&ids, row)
                                        .unwrap_or_default();
                                    if targets.is_empty() {
                                        return;
                                    }
                                    let caption = ids
                                        .iter()
                                        .map(|id| {
                                            let Some(target) =
                                                targets.iter().find(|t| t.node == *id)
                                            else {
                                                return "-".to_owned();
                                            };
                                            match self.editor.link_target(*target) {
                                                None => "No link".into(),
                                                Some(Link::Back) => "Go back".into(),
                                                Some(Link::Url(url)) => url,
                                                Some(Link::Page(id)) => self
                                                    .editor
                                                    .document
                                                    .live_page(id)
                                                    .map_or("Missing page".into(), |p| {
                                                        p.name.clone()
                                                    }),
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                        .join(" / ");
                                    let first = self.editor.link_target(targets[0]);
                                    let mixed = targets
                                        .iter()
                                        .any(|t| self.editor.link_target(*t) != first);
                                    if let Some(action) = self.link_choice_caption(
                                        ui,
                                        targets[0].node,
                                        first,
                                        mixed,
                                        Some(&caption),
                                    ) {
                                        let result = self.editor.edit_row_links(&ids, row, action);
                                        self.link_action_result(ui, result);
                                    }
                                });
                            }
                        });
                });
            }
        });
    }
    pub(super) fn inline_link_properties(&mut self, ui: &mut egui::Ui, ids: &[Id]) {
        if ids.is_empty() {
            return;
        }
        let links: Result<Option<Vec<_>>, _> =
            ids.iter().map(|&id| self.editor.text_links(id)).collect();
        let links = match links {
            Ok(Some(links)) => links,
            Ok(None) => return,
            Err(error) => {
                ui.label(error);
                return;
            }
        };
        let count = links.iter().map(Vec::len).max().unwrap_or(0);
        if count == 0 {
            return;
        }
        // Text changes invalidate an unfinished URL draft even when the old and
        // new link positions happen to share a label and destination.
        let texts: Vec<_> = ids
            .iter()
            .map(|&id| {
                self.editor
                    .control_properties(id)
                    .and_then(|(_, p)| match p.get("text") {
                        Some(Value::Text(s)) => Some(s.clone()),
                        _ => None,
                    })
            })
            .collect();
        ui.push_id(("inline-links", ids), |ui| {
            egui::CollapsingHeader::new("Text links")
                .default_open(true)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(280.)
                        .show(ui, |ui| {
                            for index in 0..count {
                                ui.push_id((index, &texts), |ui| {
                                    let label = links
                                        .iter()
                                        .map(|v| v.get(index).map_or("-", |l| l.label.as_str()))
                                        .collect::<Vec<_>>()
                                        .join(" / ");
                                    ui_text::label(ui, &label, ui.available_width());
                                    let mut entries =
                                        ids.iter().zip(&links).filter_map(|(&id, values)| {
                                            values.get(index).map(|link| (id, link))
                                        });
                                    let (first_id, first) = entries.next().unwrap();
                                    let mixed =
                                        entries.any(|(_, link)| link.target != first.target);
                                    if let Some(action) =
                                        self.link_choice(ui, first_id, first.target.clone(), mixed)
                                    {
                                        let result =
                                            self.editor.edit_text_links(ids, index, action);
                                        self.link_action_result(ui, result);
                                    }
                                });
                            }
                        });
                });
        });
    }
    fn common_link_choice(
        &mut self,
        ui: &mut egui::Ui,
        targets: &[LinkTarget],
    ) -> Option<LinkAction> {
        let first = self.editor.link_target(targets[0]);
        let mixed = targets
            .iter()
            .skip(1)
            .any(|&t| self.editor.link_target(t) != first);
        self.link_choice(ui, targets[0].node, first, mixed)
    }
    pub(super) fn typed_properties(&mut self, ui: &mut egui::Ui, id: Id) -> bool {
        let Some((spec, values)) = self.editor.control_properties(id) else {
            return false;
        };
        if spec.id == "arrow" {
            self.arrow_properties(ui, id, &values);
            return true;
        }
        if matches!(spec.id, "image" | "cover_flow") {
            self.image_properties(ui, id, &values, spec.id == "image");
        }
        if !matches!(
            spec.id,
            "arrow"
                | "image"
                | "shape"
                | "toggle"
                | "rectangle"
                | "popover"
                | "site_map"
                | "formatting_toolbar"
                | "map"
                | "modal_overlay"
                | "vertical_rule"
                | "horizontal_rule"
                | "color_picker"
                | "help_button"
                | "icon"
                | "label"
                | "heading"
                | "subtitle"
                | "text_block"
                | "text_line"
                | "tag_cloud"
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
                | "horizontal_slider"
                | "vertical_slider"
                | "horizontal_scrollbar"
                | "vertical_scrollbar"
                | "progress"
                | "horizontal_splitter"
                | "vertical_splitter"
                | "volume_slider"
                | "playback"
                | "webcam"
                | "video"
                | "cover_flow"
                | "column_chart"
                | "bar_chart"
                | "line_chart"
                | "pie_chart"
                | "horizontal_brace"
                | "vertical_brace"
                | "red_cross"
                | "scratch_out"
                | "callout"
                | "sticky_note"
                | "tooltip"
                | "numeric_stepper"
                | "date_chooser"
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
                | "date_picker"
                | "time_picker"
                | "calendar"
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

        if spec.id == "shape"
            && values.get("shape") == Some(&Value::Choice("triangle".into()))
            && ui.button("Rotate 90°").clicked()
        {
            let rotation = match values.get("shape_rotation") {
                Some(Value::Number(n)) => *n,
                _ => 0.,
            };
            self.editor.commit();
            self.editor.begin("Rotate triangle");
            if let Err(error) = self.editor.set_control_property(
                id,
                "shape_rotation",
                Value::Number(((rotation / 90.).round() * 90. + 90.) % 360.),
            ) {
                self.error = Some(error);
            }
            self.editor.commit();
        }
        let mut import_row = None;
        let mut import_icon = false;
        let explicit = self.editor.explicit_control_properties(id);
        let local = self.editor.editing_instance();
        if spec.id == "shape"
            && !local
            && explicit.contains_key("shape_rotation")
            && ui.button("Reset rotation").clicked()
        {
            self.editor.commit();
            self.editor.begin("Reset shape rotation");
            if let Err(error) = self.editor.reset_control_property(id, "shape_rotation") {
                self.error = Some(error);
            }
            self.editor.commit();
        }
        ui.add_space(10.);
        ui.scope(|ui| {
            if spec.id=="site_map" && ui.button("Populate from project").clicked() {
                self.editor.commit();self.editor.begin("Populate site map");
                if let Err(error)=self.editor.populate_site_map(id){self.error=Some(error);}
                self.editor.commit();
            }
            let mut section_headers = std::collections::HashSet::new();
            let mut field_keys = ["start_head", "end_head", "arrow_position", "stroke", "link", "slider_thumb", "progress_style", "indeterminate", "date_mode", "keyboard_device", "device_model", "orientation", "device_resizing", "background_pattern", "transparent_background", "status_bar", "image_border", "sketch_filter", "text", "header_height", "footer_height", "minimize", "maximize", "close", "resize_handle", "button_style", "tab_position", "direction", "label_position", "icon", "icon_size", "icon_rotation", "nav_icon", "filter_icon", "menu_icon", "icons", "row_links", "row_icons", "font_size", "bold", "italic", "underline", "align", "text_orientation", "state", "shape", "search_icon", "microphone_icon", "vertical_scrollbar", "value", "text_color", "border", "border_color", "fill", "background_opacity", "header", "row_height", "selected_index", "alternate_fill", "horizontal_lines", "vertical_lines"] .to_vec();
            if spec.id == "tabs" {
                field_keys.retain(|key| *key != "align");
                let position = field_keys.iter().position(|key| *key == "tab_position").unwrap();
                field_keys.insert(position + 1, "align");
            }
            for key in field_keys {
                if key == "link" && self.editor.library.is_some() && self.editor.context.is_empty() {continue;}
                if key == "sketch_filter" && spec.id == "cover_flow" && !matches!(values.get("image"), Some(Value::Asset(_))) { continue; }
                if matches!(key, "icon_size" | "icon_rotation") && !matches!(values.get("icon"),Some(Value::Icon(_))) {continue;}
                if key == "value" && !matches!(spec.id,"progress"|"horizontal_slider"|"vertical_slider"|"horizontal_scrollbar"|"vertical_scrollbar") && values.get("vertical_scrollbar") != Some(&Value::Bool(true)) { continue; }
                let Some(field) = spec.property(key).filter(|f| f.visible && supported(spec.id, key)) else { continue; };
                let Some(mut value) = values.get(key).cloned() else { continue; };
                if local && let Some(section) = wireframe::workspace::PropertySection::for_control(spec.id, key)
                    && section != wireframe::workspace::PropertySection::Image && section_headers.insert(section) && self.component_section_header(ui, &[id], section) { break; }
                let original = value.clone();
                let mut reset = false;
                let mut finish = false;
                let mut link_action = None;
                ui.push_id((id,key), |ui| {
                    ui.horizontal(|ui| {
                        if let Value::Bool(v) = &mut value {
                            finish = ui.checkbox(v, field.label).changed();
                        } else if local && explicit.contains_key(key) { ui.colored_label(Color32::from_rgb(30,120,70), field.label); } else {ui.label(field.label);}
                        if explicit.contains_key(key) && !(local && wireframe::workspace::PropertySection::for_control(spec.id, key).is_some()) {
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
                            let labels = self.editor.link_row_labels(id).unwrap_or_default();
                            links.resize(labels.len(),None);
                            // Keep partial URL input outside document history until Apply.
                            egui::CollapsingHeader::new(if spec.id=="app_bar" {"Edit icon links"} else if matches!(spec.id,"alert"|"date_picker"|"time_picker") {"Edit button links"} else if matches!(spec.id,"tabs"|"vertical_tabs") {"Edit tab links"} else if spec.id=="button_bar" {"Edit segment links"} else if matches!(spec.id,"breadcrumbs"|"link_bar"|"menu_bar") {"Edit item links"} else {"Edit row links"}).show(ui,|ui| {
                                egui::ScrollArea::vertical().max_height(280.).show(ui,|ui| {
                                    for (row,label) in labels.iter().enumerate() {
                                        ui.push_id(row,|ui| {
                                            let label=wireframe::rich_text::RichText::parse(label,false).display_text(false);
                                            ui_text::label(ui,&if label.is_empty(){format!("{} {} (empty)",if matches!(spec.id,"tabs"|"vertical_tabs") {"Tab"} else if spec.id=="button_bar" {"Segment"} else if matches!(spec.id,"breadcrumbs"|"link_bar"|"menu_bar") {"Item"} else {"Row"},row+1)}else{label},ui.available_width());
                                            if let Some(action)=self.link_choice(ui,id,links[row].clone(),false) {
                                                link_action=Some((LinkTarget::row(id,row),action));
                                            }
                                        });
                                    }
                                });
                            });
                            // Do not create a property solely to pad absent trailing slots.
                            if !finish && let Value::Links(old)=&original {*links=old.clone();}
                        }
                        (PropertyType::Text, Value::Text(v)) => {
                            if key == "text" {
                                match self.editor.control_text_for_edit(id, v) {
                                    Ok(mut displayed) => {
                                        let before = displayed.clone();
                                        let r = wireframe::text_entry::multiline(ui, &mut displayed, 4, false);
                                        finish = r.lost_focus();
                                        if displayed != before {
                                            match self.editor.control_text_from_edit(id, &displayed) {
                                                Ok(stored) => *v = stored,
                                                Err(error) => self.error = Some(error),
                                            }
                                        }
                                    }
                                    Err(error) => { self.error = Some(error); }
                                }
                            } else {
                                let r = wireframe::text_entry::multiline(ui, v, 4, false);
                                finish = r.lost_focus();
                            }
                        }
                        (PropertyType::Number {..}, Value::Number(v)) if key == "selected_index" && matches!(spec.id, "list" | "button_bar" | "link_bar" | "tabs" | "vertical_tabs" | "menu" | "menu_bar" | "tree" | "accordion") => {
                            let source=match values.get("text") {Some(Value::Text(s))=>s.as_str(),_=>""};
                            let raw:Vec<String>=if spec.id=="accordion" {wireframe::accordion::labels(source)} else if spec.id=="site_map" {wireframe::site_map::labels(source)} else if spec.id=="tree" {wireframe::tree::labels(source)} else if matches!(spec.id,"menu"|"menu_bar") {wireframe::menu::labels(source,spec.id=="menu_bar")} else if matches!(spec.id,"tabs"|"vertical_tabs") {wireframe::tabs::labels(source,spec.id=="vertical_tabs")} else if spec.id=="link_bar" {wireframe::navigation::labels(source,false)} else if spec.id=="button_bar" {wireframe::button_bar::labels(source).into_iter().map(str::to_owned).collect()} else {wireframe::list::labels(source,values.get("header")==Some(&Value::Bool(true))).into_iter().map(str::to_owned).collect()};
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
                        (PropertyType::Number {..}, Value::Number(v)) if key == "selected_index" && spec.id == "table" => {
                            let labels = wireframe::workspace::selectable_row_labels(spec.id, &values).unwrap_or_default().iter().map(|s| wireframe::rich_text::RichText::parse(s,false).display_text(false)).collect::<Vec<_>>();
                            let current = if *v < 0. { "None" } else { labels.get(*v as usize).map_or("No matching item", String::as_str) };
                            ui_text::combo_box(ui,"selection",current,|ui| {
                                if ui_text::selectable_value(ui,v,-1.,"None").clicked() { ui.close(); }
                                ui.separator();
                                for (index,label) in labels.iter().enumerate() {
                                    if ui_text::selectable_value(ui,v,index as f64,label).clicked() { ui.close(); }
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
                        (PropertyType::Number {..}, Value::Number(v)) if key == "arrow_position" => {
                            let response=ui.add(egui::Slider::new(v,0.0..=4.0).integer());
                            finish=response.lost_focus() || response.drag_stopped() || (response.changed() && !response.dragged() && !response.has_focus());
                        }
                        (PropertyType::Number {min,max,integer}, Value::Number(v)) => {
                            let mut edit = egui::DragValue::new(v).range(*min..=*max).speed(1.);
                            if *integer { edit = edit.max_decimals(0); }
                            let r = ui.add(edit);
                            finish = r.lost_focus() || r.drag_stopped();
                        }
                        (PropertyType::Choice(options), Value::Choice(v)) => {
                            egui::ComboBox::from_id_salt("value").selected_text(if key == "stroke" { match v.as_str() { "solid" => "Solid", "dashed" => "Dashed", "dotted" => "Dotted", other => field_choice_label(spec.id,key,other) } } else {field_choice_label(spec.id,key,v)}).show_ui(ui, |ui| {
                                for option in *options {
                                    if spec.id == "rectangle" && key == "border" && *option == "dotted" { continue; }
                                    if spec.id == "browser" && key == "border" && !matches!(*option, "solid" | "torn_bottom") {
                                        continue;
                                    }
                                    if ui.selectable_value(v, (*option).into(), if key == "stroke" { match *option { "solid" => "Solid", "dashed" => "Dashed", "dotted" => "Dotted", other => field_choice_label(spec.id,key,other) } } else {field_choice_label(spec.id,key,option)}).clicked() {
                                        ui.close();
                                    }
                                }
                            });
                            finish = *v != match &original { Value::Choice(s) => s, _ => unreachable!() }.as_str();
                        }
                        (PropertyType::Link, v) => {
                            let saved=match v {Value::Link(link)=>Some(link.clone()),_=>None};
                            if let Some(action)=self.link_choice(ui,id,saved,false) {
                                link_action=Some((LinkTarget::whole(id),action));
                            }
                        },
                        (PropertyType::Color, Value::Color(v)) => {
                            let mut rgb = [v[0],v[1],v[2]];
                            ui.color_edit_button_srgb(&mut rgb);
                            v[..3].copy_from_slice(&rgb);
                        }
                        _ => {}
                    }
                });
                if let Some((target,action)) = link_action {
                    self.apply_link_action(ui,&[target],action);
                } else if reset {
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
    fn link_frame(
        ctx: &egui::Context,
        app: &mut WireframeApp,
        id: Id,
        seed: Option<Option<Link>>,
        events: Vec<egui::Event>,
    ) -> Option<Rect> {
        ctx.data_mut(|d| d.remove::<Rect>(egui::Id::new("whole-link-apply-test")));
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(500., 400.))),
                events,
                ..Default::default()
            },
            |ui| {
                let saved = app.editor.node_link(id);
                if let Some(draft) = seed.clone() {
                    let key = ui.make_persistent_id((
                        "whole-link-draft",
                        id,
                        app.editor.surface_id(),
                        &app.editor.context,
                        app.editor.clipboard_session(),
                    ));
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(key, (saved.clone(), draft)));
                }
                if let Some(action) = app.link_choice(ui, id, saved, false) {
                    app.editor
                        .edit_links(&[LinkTarget::whole(id)], action)
                        .unwrap();
                }
            },
        );
        output.textures_delta.clear();
        ctx.data_mut(|d| d.get_temp(egui::Id::new("whole-link-apply-test")))
    }
    #[test]
    fn whole_link_picker_keeps_url_drafts_out_of_history_until_apply_and_isolates_projects() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Button, 100., 100.);
        let id = *app.editor.selection.iter().next().unwrap();
        let before = app.editor.document.clone();
        let invalid = Some(Link::Url("javascript:example".into()));
        let r = link_frame(&ctx, &mut app, id, Some(invalid), vec![]).unwrap();
        link_frame(&ctx, &mut app, id, None, vec![pointer(r.center(), true)]);
        link_frame(&ctx, &mut app, id, None, vec![pointer(r.center(), false)]);
        assert_eq!(app.editor.document, before);
        let url = Some(Link::Url("https://example.com/contact".into()));
        let r = link_frame(&ctx, &mut app, id, Some(url.clone()), vec![]).unwrap();
        assert_eq!(app.editor.document, before);
        link_frame(&ctx, &mut app, id, None, vec![pointer(r.center(), true)]);
        link_frame(&ctx, &mut app, id, None, vec![pointer(r.center(), false)]);
        assert_eq!(app.editor.node_link(id), url);
        let after = app.editor.document.clone();
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        app.editor.redo();
        assert_eq!(app.editor.document, after);
        link_frame(
            &ctx,
            &mut app,
            id,
            Some(Some(Link::Url("https://example.com/unfinished".into()))),
            vec![],
        );
        app.editor = Editor::default();
        app.editor.add(Kind::Button, 100., 100.);
        let new_id = *app.editor.selection.iter().next().unwrap();
        assert_eq!(new_id, id);
        assert!(link_frame(&ctx, &mut app, new_id, None, vec![]).is_none());
        assert_eq!(app.editor.node_link(new_id), None);
    }
    #[test]
    fn typed_and_legacy_controls_each_expose_one_full_link_picker() {
        for kind in [
            Kind::Button,
            Kind::Label,
            Kind::Link,
            Kind::TextInput,
            Kind::Image,
            Kind::Rectangle,
            Kind::CircleButton,
            Kind::PointedButton,
            Kind::MultilineButton,
            Kind::Shape,
            Kind::Toggle,
        ] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(kind, 0., 0.);
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 1400.))),
                    ..Default::default()
                },
                |ui| app.inspector(ui),
            );
            out.textures_delta.clear();
            assert_eq!(
                ctx.data_mut(
                    |d| d.get_temp::<HashSet<egui::Id>>(egui::Id::new("whole-link-picker-count"))
                )
                .map(|fields| fields.len()),
                Some(1),
                "{kind:?}"
            );
        }
    }

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
    fn progress_inspector_exposes_style_value_and_indeterminate_without_caption() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Progress, 20., 20.);
        let id = *app.editor.selection.iter().next().unwrap();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400., 900.))),
                ..Default::default()
            },
            |ui| {
                assert!(app.typed_properties(ui, id));
            },
        );
        output.textures_delta.clear();
        let labels: Vec<_> = output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .filter_map(|(_, n)| n.label().or_else(|| n.value()))
            .collect();
        for key in ["progress_style", "value", "indeterminate", "fill"] {
            let label = wireframe::catalog::get("progress")
                .unwrap()
                .property(key)
                .unwrap()
                .label;
            assert!(labels.contains(&label), "Missing {key}: {labels:?}");
        }
        assert!(!labels.contains(&"Text"));
    }
    #[test]
    fn horizontal_rule_inspector_exposes_stroke_color_opacity_and_hides_legacy_flag() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Line, 20., 20.);
        let id = *app.editor.selection.iter().next().unwrap();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400., 900.))),
                ..Default::default()
            },
            |ui| assert!(app.typed_properties(ui, id)),
        );
        output.textures_delta.clear();
        let labels: Vec<_> = output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .filter_map(|(_, n)| n.label().or_else(|| n.value()))
            .collect();
        for label in ["Stroke", "Border color", "Opacity", "Solid"] {
            assert!(labels.contains(&label), "Missing {label}: {labels:?}");
        }
        assert!(!labels.contains(&"Legacy line frame"));
        assert!(!labels.contains(&"Text"));
    }

    #[test]
    fn rectangle_uses_typed_inspector_and_keeps_legacy_caption_fields() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Rectangle, 20., 20.);
        let id = *app.editor.selection.iter().next().unwrap();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400., 900.),
                )),
                ..Default::default()
            },
            |ui| {
                ui.set_width(260.);
                assert!(app.typed_properties(ui, id))
            },
        );
        output.textures_delta.clear();
        for key in [
            "text",
            "font_size",
            "bold",
            "fill",
            "border",
            "border_color",
            "background_opacity",
        ] {
            assert!(
                wireframe::catalog::get("rectangle")
                    .unwrap()
                    .property(key)
                    .unwrap()
                    .visible
            );
            assert!(supported("rectangle", key));
        }
    }
    #[test]
    fn toggle_inspector_exposes_only_state_color_and_link() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Toggle, 20., 20.);
        let id = *app.editor.selection.iter().next().unwrap();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400., 900.))),
                ..Default::default()
            },
            |ui| {
                assert!(app.typed_properties(ui, id));
            },
        );
        output.textures_delta.clear();
        let labels: Vec<_> = output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .filter_map(|(_, n)| n.label().or_else(|| n.value()))
            .collect();
        for key in ["state", "fill", "link"] {
            let field = wireframe::catalog::get("toggle")
                .unwrap()
                .property(key)
                .unwrap();
            assert!(supported("toggle", key));
            assert!(labels.contains(&field.label), "Missing {key}: {labels:?}");
        }
        assert!(!labels.contains(&"Text"));
        assert!(!labels.contains(&"Selected state"));
    }
    #[test]
    fn shape_inspector_fields_and_triangle_rotation_follow_real_button_input() {
        fn frame(
            app: &mut WireframeApp,
            ctx: &egui::Context,
            id: Id,
            events: Vec<egui::Event>,
        ) -> egui::FullOutput {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    events,
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(420., 1600.))),
                    ..Default::default()
                },
                |ui| {
                    assert!(app.typed_properties(ui, id));
                },
            );
            out.textures_delta.clear();
            out
        }
        fn button(out: &egui::FullOutput, label: &str) -> Option<Pos2> {
            out.platform_output
                .accesskit_update
                .as_ref()?
                .nodes
                .iter()
                .find_map(|(_, n)| {
                    if n.label() != Some(label) || n.role() != egui::accesskit::Role::Button {
                        return None;
                    }
                    let b = n.bounds()?;
                    Some(egui::pos2(
                        (b.x0 + b.x1) as f32 / 2.,
                        (b.y0 + b.y1) as f32 / 2.,
                    ))
                })
        }
        fn click(app: &mut WireframeApp, ctx: &egui::Context, id: Id, pos: Pos2) {
            for pressed in [true, false] {
                frame(
                    app,
                    ctx,
                    id,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Modifiers::NONE,
                        },
                    ],
                );
            }
        }
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Shape, 0., 0.);
        let id = *app.editor.selection.iter().next().unwrap();
        let out = frame(&mut app, &ctx, id, vec![]);
        assert!(button(&out, "Rotate 90°").is_none());
        for key in [
            "shape",
            "border_color",
            "background_opacity",
            "icon",
            "icon_size",
            "icon_rotation",
            "fill",
            "link",
            "align",
            "text",
        ] {
            let field = wireframe::catalog::get("shape")
                .unwrap()
                .property(key)
                .unwrap();
            assert!(field.visible && supported("shape", key), "{key}");
        }
        app.editor.begin("Choose triangle");
        app.editor
            .set_control_property(id, "shape", Value::Choice("triangle".into()))
            .unwrap();
        app.editor.commit();
        frame(&mut app, &ctx, id, vec![]);
        let before = app.editor.document.clone();
        let out = frame(&mut app, &ctx, id, vec![]);
        let pos = button(&out, "Rotate 90°").unwrap();
        click(&mut app, &ctx, id, pos);
        assert_eq!(
            app.editor.control_properties(id).unwrap().1["shape_rotation"],
            Value::Number(90.)
        );
        let after = app.editor.document.clone();
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        app.editor.redo();
        assert_eq!(app.editor.document, after);
        let out = frame(&mut app, &ctx, id, vec![]);
        let pos = button(&out, "Reset rotation").unwrap();
        click(&mut app, &ctx, id, pos);
        assert_eq!(
            app.editor.control_properties(id).unwrap().1["shape_rotation"],
            Value::Number(0.)
        );
    }
    #[test]
    fn specialized_inspector_fields_have_renderer_support() {
        for (kind, fields) in [
            (
                "popover",
                vec!["direction", "arrow_position", "fill", "text"],
            ),
            ("site_map", vec!["direction", "row_links", "text"]),
            ("help_button", vec!["link"]),
            ("label", vec!["text_orientation"]),
            ("color_picker", vec!["fill"]),
            ("numeric_stepper", vec!["text", "state", "border_color"]),
            ("callout", vec!["fill", "background_opacity", "text"]),
            ("sticky_note", vec!["fill", "align", "text"]),
            ("tooltip", vec!["direction", "align", "text"]),
            ("horizontal_brace", vec!["direction", "text", "text_color"]),
            ("vertical_brace", vec!["direction", "text", "text_color"]),
            ("scratch_out", vec!["fill", "background_opacity"]),
            ("table", vec!["border", "vertical_scrollbar", "value"]),
            (
                "rectangle",
                vec![
                    "border",
                    "border_color",
                    "fill",
                    "background_opacity",
                    "text",
                    "font_size",
                    "bold",
                ],
            ),
        ] {
            for key in fields {
                assert!(supported(kind, key));
                assert!(
                    wireframe::catalog::get(kind)
                        .unwrap()
                        .property(key)
                        .unwrap()
                        .visible
                );
            }
        }
    }
    #[test]
    fn fractional_typing_is_one_undo_step_on_enter_or_blur() {
        for (kind, key) in [
            (Kind::FieldSet, "background_opacity"),
            (Kind::Callout, "background_opacity"),
            (Kind::Browser, "value"),
            (Kind::Table, "value"),
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
