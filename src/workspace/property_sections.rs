// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum PropertySection {
    Text,
    Color,
    Image,
    Border,
    AppBarIcons,
    Arrow,
    Icon,
    Menu,
    State,
    Selection,
    Shape,
    Search,
    Slider,
    Scrollbar,
    Progress,
    Phone,
    Tablet,
    Keyboard,
    Window,
    Popover,
    Direction,
    TextOrientation,
    Date,
    Tabs,
    Buttons,
    List,
    Table,
    LabelPosition,
}
impl PropertySection {
    pub fn label(self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Color => "Color",
            Self::Image => "Image",
            Self::Border => "Border",
            Self::AppBarIcons => "Icons",
            Self::Arrow => "Arrow",
            Self::Icon => "Icon",
            Self::Menu => "Menu",
            Self::State => "State",
            Self::Selection => "Selection",
            Self::Shape => "Shape",
            Self::Search => "Search",
            Self::Slider => "Slider",
            Self::Scrollbar => "Scrollbar",
            Self::Progress => "Progress Bar",
            Self::Phone => "Phone",
            Self::Tablet => "Tablet",
            Self::Keyboard => "Keyboard",
            Self::Window => "Window",
            Self::Popover => "Popover",
            Self::Direction => "Direction",
            Self::TextOrientation => "Text Orientation",
            Self::Date => "Date Picker",
            Self::Tabs => "Tabs Position",
            Self::Buttons => "Alert Buttons",
            Self::List => "List",
            Self::Table => "Table",
            Self::LabelPosition => "Label Position",
        }
    }
    pub fn keys(self) -> &'static [&'static str] {
        match self {
            Self::Text => &["font_size", "bold", "italic", "underline", "align", "text"],
            Self::Color => &["fill", "border_color", "text_color", "background_opacity"],
            Self::Image => &["image", "sketch_filter", "rotation"],
            Self::Border => &["border", "image_border"],
            Self::AppBarIcons => &["icons", "fill"],
            Self::Arrow => &[
                "start_head",
                "end_head",
                "label_position",
                "shape",
                "start_x",
                "start_y",
                "end_x",
                "end_y",
                "bend_along",
                "bend_across",
                "legacy_horizontal",
            ],
            Self::Icon => &["icon", "icon_size", "icon_rotation"],
            Self::Menu => &["menu_icon"],
            Self::State => &["state", "checked"],
            Self::Selection => &["selected_index"],
            Self::Shape => &["shape", "shape_rotation"],
            Self::Search => &["shape", "search_icon", "microphone_icon"],
            Self::Slider => &["value", "slider_thumb"],
            Self::Scrollbar => &["vertical_scrollbar", "value"],
            Self::Progress => &["value", "progress_style", "indeterminate"],
            Self::Phone => &[
                "device_model",
                "orientation",
                "transparent_background",
                "status_bar",
                "background_pattern",
                "device_resizing",
            ],
            Self::Tablet => &[
                "orientation",
                "transparent_background",
                "status_bar",
                "device_resizing",
            ],
            Self::Keyboard => &["keyboard_device", "orientation"],
            Self::Window => &[
                "footer_height",
                "header_height",
                "minimize",
                "maximize",
                "resize_handle",
                "close",
            ],
            Self::Popover => &["arrow_position", "direction"],
            Self::Direction => &["direction"],
            Self::TextOrientation => &["text_orientation"],
            Self::Date => &["date_mode"],
            Self::Tabs => &["tab_position", "align"],
            Self::Buttons => &["button_style"],
            Self::List => &[
                "header",
                "alternate_fill",
                "fill",
                "row_height",
                "row_icons",
            ],
            Self::Table => &[
                "header",
                "horizontal_lines",
                "vertical_lines",
                "alternate_fill",
                "fill",
                "row_height",
            ],
            Self::LabelPosition => &["label_position"],
        }
    }
    /// Classify a field by its control family; repeated key names are not interchangeable.
    pub fn for_control(kind: &str, key: &str) -> Option<Self> {
        if kind == "tabs" && key == "align" {
            return Some(Self::Tabs);
        }
        if let Some(section) = Self::for_key(key) {
            return Some(section);
        }
        let section = match key {
            "icon" | "icon_size" | "icon_rotation" => Self::Icon,
            "menu_icon" => Self::Menu,
            "border" | "image_border" => Self::Border,
            "icons" if kind == "app_bar" => Self::AppBarIcons,
            "state" | "checked" => Self::State,
            "selected_index" => Self::Selection,
            "label_position" if kind != "arrow" => Self::LabelPosition,
            "text_orientation" => Self::TextOrientation,
            "date_mode" => Self::Date,
            "tab_position" => Self::Tabs,
            "button_style" => Self::Buttons,
            _ => match kind {
                "image" | "cover_flow" if Self::Image.keys().contains(&key) => Self::Image,
                "arrow" => Self::Arrow,
                "shape" => Self::Shape,
                "search" => Self::Search,
                "horizontal_slider" | "vertical_slider" => Self::Slider,
                "progress" => Self::Progress,
                "phone" => Self::Phone,
                "tablet" => Self::Tablet,
                "mobile_keyboard" => Self::Keyboard,
                "window" if Self::Window.keys().contains(&key) => Self::Window,
                "popover" => Self::Popover,
                "pointed_button" | "tooltip" | "horizontal_brace" | "vertical_brace"
                | "site_map" => Self::Direction,
                "list" if Self::List.keys().contains(&key) => Self::List,
                "table" if Self::Table.keys().contains(&key) => Self::Table,
                "horizontal_scrollbar"
                | "vertical_scrollbar"
                | "dropdown"
                | "text_area"
                | "list"
                | "table"
                | "tabs"
                | "vertical_tabs"
                | "tree"
                | "accordion"
                | "browser"
                | "window"
                | "cover_flow" => Self::Scrollbar,
                _ => return None,
            },
        };
        section.keys().contains(&key).then_some(section)
    }
    fn allows_mixed_kinds(self) -> bool {
        matches!(
            self,
            Self::Text
                | Self::Image
                | Self::Border
                | Self::Color
                | Self::Icon
                | Self::Menu
                | Self::State
                | Self::Selection
                | Self::LabelPosition
                | Self::Slider
                | Self::Scrollbar
        )
    }
    pub fn for_key(key: &str) -> Option<Self> {
        [Self::Text, Self::Color]
            .into_iter()
            .find(|section| section.keys().contains(&key))
    }
}
impl Editor {
    pub fn selection_property_section(
        &self,
        targets: &[(Id, &'static str)],
    ) -> Option<PropertySection> {
        let &(first_id, first_key) = targets.first()?;
        let (first_spec, _) = self.control_properties(first_id)?;
        let section = PropertySection::for_control(first_spec.id, first_key)?;
        targets
            .iter()
            .all(|&(id, key)| {
                self.control_properties(id).is_some_and(|(spec, _)| {
                    PropertySection::for_control(spec.id, key) == Some(section)
                        && (section.allows_mixed_kinds() || spec.id == first_spec.id)
                })
            })
            .then_some(section)
    }

    pub fn has_section_overrides(&self, ids: &[Id], section: PropertySection) -> bool {
        self.editing_instance()
            && ids.iter().any(|&id| {
                let explicit = self.explicit_control_properties(id);
                self.control_properties(id).is_some_and(|(spec, _)| {
                    section.keys().iter().any(|key| {
                        (spec.property(key).is_some()
                            || section == PropertySection::State && *key == "checked")
                            && !(section == PropertySection::Text
                                && spec.id == "tabs"
                                && *key == "align")
                            && !(section == PropertySection::Tabs
                                && spec.id != "tabs"
                                && *key == "align")
                            && !(section == PropertySection::AppBarIcons && *key != "icons")
                            && explicit.contains_key(*key)
                    })
                })
            })
    }
    /// Restore one inherited section on captured sibling controls as a single edit.
    pub fn reset_component_section(
        &mut self,
        ids: &[Id],
        section: PropertySection,
    ) -> Result<(), String> {
        if !self.editing_instance() {
            return Err("Section reset requires component contents".into());
        }
        if ids.is_empty() {
            return Err("Select controls to reset".into());
        }
        let mut seen = HashSet::new();
        for &id in ids {
            if !seen.insert(id) {
                return Err("Reset targets must be unique".into());
            }
            let (spec, _) = self
                .control_properties(id)
                .ok_or("Reset target no longer exists")?;
            if !section.keys().iter().any(|key| {
                spec.property(key).is_some()
                    && PropertySection::for_control(spec.id, key) == Some(section)
            }) {
                return Err("Section is not available for this control".into());
            }
        }
        if !self.has_section_overrides(ids, section) {
            return Ok(());
        }
        self.component_change(&format!("Reset {} to source", section.label()), |e| {
            for &id in ids {
                if section == PropertySection::Image
                    && e.explicit_control_properties(id).contains_key("rotation")
                    && e.control_properties(id)
                        .is_some_and(|(spec, _)| spec.id == "image")
                {
                    // Source/filter inheritance is resolved before rotation's
                    // geometry side effects, including automatic image sizing.
                    e.reset_local_properties(id, &["image", "sketch_filter"])?;
                    e.rotate_image_value(id)?;
                }
                let kind = e.control_properties(id).unwrap().0.id;
                let keys: Vec<_> = section
                    .keys()
                    .iter()
                    .copied()
                    .filter(|key| {
                        !(section == PropertySection::Text && kind == "tabs" && *key == "align")
                            && !(section == PropertySection::Tabs
                                && kind != "tabs"
                                && *key == "align")
                    })
                    .collect();
                e.reset_local_properties(id, &keys)?;
            }
            e.layout_page(e.surface_id())?;
            Ok(())
        })
    }
}
