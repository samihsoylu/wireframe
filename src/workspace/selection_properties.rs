// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::catalog::{PropertySpec, PropertyType};

#[derive(Clone, Debug)]
pub struct SelectionProperty {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: PropertyType,
    pub choices: Vec<&'static str>,
    pub value: Value,
    pub mixed: bool,
    pub explicit: bool,
    pub targets: Vec<(Id, &'static str)>,
}
const TEXT: &[&str] = &["font_size", "bold", "italic", "underline", "align"];
const COLORS: &[&str] = &["fill", "border_color", "text_color"];
const FIELDS: &[&str] = &[
    "fill",
    "border_color",
    "text_color",
    "alternate_fill",
    "background_opacity",
    "border",
    "stroke",
    "icon",
    "icon_size",
    "icon_rotation",
    "menu_icon",
    "nav_icon",
    "filter_icon",
    "state",
    "bold",
    "italic",
    "underline",
    "align",
    "font_size",
];
const SPECIALIZED: &[&str] = &[
    "device_model",
    "keyboard_device",
    "orientation",
    "transparent_background",
    "status_bar",
    "background_pattern",
    "device_resizing",
    "slider_thumb",
    "value",
    "progress_style",
    "indeterminate",
    "text_orientation",
    "direction",
    "arrow_position",
    "header_height",
    "footer_height",
    "minimize",
    "maximize",
    "close",
    "resize_handle",
    "date_mode",
    "tab_position",
    "shape",
    "shape_rotation",
    "start_head",
    "end_head",
    "label_position",
    "search_icon",
    "microphone_icon",
    "header",
    "row_height",
    "horizontal_lines",
    "vertical_lines",
    "vertical_scrollbar",
    "button_style",
    "sketch_filter",
    "image_border",
];
impl SelectionProperty {
    pub fn specialized(&self) -> bool {
        SPECIALIZED.contains(&self.key)
    }
}
fn specialized_available(
    controls: &[(&'static catalog::ControlSpec, Properties)],
    key: &str,
) -> bool {
    let same = |kinds: &[&str]| {
        controls.first().is_some_and(|(first, _)| {
            kinds.contains(&first.id) && controls.iter().all(|(s, _)| s.id == first.id)
        })
    };
    let family = |kinds: &[&str]| controls.iter().all(|(s, _)| kinds.contains(&s.id));
    match key {
        "device_model" | "background_pattern" => same(&["phone"]),
        "orientation" => same(&["phone", "tablet", "smartphone", "mobile_keyboard"]),
        "keyboard_device" => same(&["mobile_keyboard"]),
        "transparent_background" | "status_bar" | "device_resizing" => {
            same(&["phone", "tablet", "smartphone"])
        }
        "slider_thumb" => family(&["horizontal_slider", "vertical_slider"]),
        "value" => {
            family(&["horizontal_slider", "vertical_slider"])
                || family(&["horizontal_scrollbar", "vertical_scrollbar"])
                || same(&["progress"])
                || controls
                    .iter()
                    .all(|(s, _)| field(s, "vertical_scrollbar").is_some())
        }
        "progress_style" | "indeterminate" => same(&["progress"]),
        "text_orientation" => same(&["label"]),
        "direction" => same(&[
            "pointed_button",
            "tooltip",
            "horizontal_brace",
            "vertical_brace",
            "site_map",
            "popover",
        ]),
        "arrow_position" => same(&["popover"]),
        "header_height" | "footer_height" | "minimize" | "maximize" | "close" | "resize_handle" => {
            same(&["window"])
        }
        "date_mode" => same(&["date_picker"]),
        "tab_position" => same(&["tabs", "vertical_tabs"]),
        "shape" => same(&["search", "shape", "arrow"]),
        "shape_rotation" => {
            same(&["shape"])
                && controls
                    .iter()
                    .all(|(_, p)| p.get("shape") == Some(&Value::Choice("triangle".into())))
        }
        "start_head" | "end_head" | "label_position" => same(&["arrow"]),
        "search_icon" | "microphone_icon" => same(&["search"]),
        "header" | "row_height" => same(&["list", "table"]),
        "horizontal_lines" | "vertical_lines" => same(&["table"]),
        "vertical_scrollbar" => true,
        "button_style" => same(&["alert"]),
        "sketch_filter" => {
            family(&["image", "cover_flow"])
                && controls
                    .iter()
                    .any(|(_, p)| matches!(p.get("image"), Some(Value::Asset(_))))
        }
        "image_border" => same(&["image"]),
        _ => false,
    }
}
fn field(spec: &'static catalog::ControlSpec, key: &str) -> Option<&'static PropertySpec> {
    spec.property(key).filter(|f| f.visible)
}
impl Editor {
    /// Click order governs mixed-property display. Bulk selections use document order.
    pub fn ordered_selection(&self) -> Vec<Id> {
        if self.selection.is_empty() {
            return vec![];
        }
        let nodes = self.nodes();
        let valid: HashSet<_> = nodes.iter().map(|n| n.id).collect();
        let mut ids: Vec<_> = self
            .selection_order
            .iter()
            .copied()
            .filter(|id| self.selection.contains(id) && valid.contains(id))
            .collect();
        let mut seen: HashSet<_> = ids.iter().copied().collect();
        for node in nodes.iter() {
            if self.selection.contains(&node.id) && seen.insert(node.id) {
                ids.push(node.id);
            }
        }
        ids
    }
    pub fn selection_properties(&self) -> Vec<SelectionProperty> {
        let ids = self.ordered_selection();
        let nodes = self.nodes();
        let by_id: HashMap<_, _> = nodes.iter().map(|node| (node.id, node)).collect();
        let context = self.property_context();
        let Some(controls) = ids
            .iter()
            .map(|id| {
                let node = by_id.get(id)?;
                let NodeContent::Control { kind, .. } = &node.content else {
                    return None;
                };
                let spec = catalog::get(kind)?;
                Some((spec, spec.effective(node, &context).ok()?))
            })
            .collect::<Option<Vec<_>>>()
        else {
            return vec![];
        };
        if controls.is_empty() {
            return vec![];
        }
        let explicit: HashMap<_, _> = ids
            .iter()
            .map(|id| (*id, self.explicit_control_properties(*id)))
            .collect();
        let text_available = controls
            .iter()
            .all(|(s, _)| TEXT.iter().any(|key| field(s, key).is_some()));
        let all_tabs = controls.iter().all(|(s, _)| s.id == "tabs");
        let mut result = vec![];
        for &key in FIELDS.iter().chain(SPECIALIZED) {
            if SPECIALIZED.contains(&key) && !specialized_available(&controls, key) {
                continue;
            }
            let text = TEXT.contains(&key);
            if text && !text_available {
                continue;
            }
            // Checkbox borders and the multi-option border palette are separate inspectors.
            if key == "border" {
                let simple: Vec<_> = controls.iter().map(|(s, _)| {
                    matches!(field(s, key).map(|p| p.kind), Some(PropertyType::Choice(options))
                        if options.len() == 2 && options.contains(&"solid") && options.contains(&"none"))
                }).collect();
                if simple.iter().any(|x| *x) && simple.iter().any(|x| !*x) {
                    continue;
                }
            }
            let fields: Vec<_> = controls
                .iter()
                .enumerate()
                .filter_map(|(i, (s, p))| {
                    if key == "align" && s.id == "tabs" && !all_tabs {
                        return None;
                    }
                    Some((i, field(s, key)?, p.get(key)?))
                })
                .collect();
            if fields.is_empty() || (!text && fields.len() != controls.len()) {
                continue;
            }
            let first = fields[0].1;
            if fields.iter().any(|(_, f, _)| {
                std::mem::discriminant(&f.kind) != std::mem::discriminant(&first.kind)
            }) {
                continue;
            }
            let mut choices = vec![];
            if let PropertyType::Choice(options) = first.kind {
                let union = matches!(key, "state" | "border" | "stroke");
                if !union
                    && fields
                        .iter()
                        .any(|(_, f, _)| !matches!(f.kind,PropertyType::Choice(o) if o==options))
                {
                    continue;
                }
                for (_, f, _) in &fields {
                    if let PropertyType::Choice(options) = f.kind {
                        for &option in options {
                            if !choices.contains(&option) {
                                choices.push(option);
                            }
                        }
                    }
                }
            }
            let targets: Vec<_> = fields.iter().map(|(i, _, _)| (ids[*i], key)).collect();
            result.push(SelectionProperty {
                key,
                label: first.label,
                kind: first.kind,
                choices,
                value: fields[0].2.clone(),
                mixed: fields.iter().any(|(_, _, v)| *v != fields[0].2),
                explicit: targets.iter().any(|(id, k)| explicit[id].contains_key(*k)),
                targets,
            });
        }
        if !result.iter().any(|p| COLORS.contains(&p.key)) {
            let targets: Option<Vec<_>> = controls
                .iter()
                .enumerate()
                .map(|(i, (s, _))| {
                    COLORS
                        .iter()
                        .find(|key| field(s, key).is_some())
                        .map(|key| (ids[i], *key))
                })
                .collect();
            if let Some(targets) = targets {
                let values: Vec<_> = targets
                    .iter()
                    .zip(&controls)
                    .map(|((_, key), (_, values))| values[*key].clone())
                    .collect();
                result.insert(
                    0,
                    SelectionProperty {
                        key: "main_color",
                        label: "Color",
                        kind: PropertyType::Color,
                        choices: vec![],
                        value: values[0].clone(),
                        mixed: values.iter().any(|v| v != &values[0]),
                        explicit: targets.iter().any(|(id, k)| explicit[id].contains_key(*k)),
                        targets,
                    },
                );
            }
        }
        result
    }
    /// Atomic within the caller's transaction, including component-local overrides.
    pub fn set_selection_property(&mut self, key: &str, value: Value) -> Result<(), String> {
        let property = self
            .selection_properties()
            .into_iter()
            .find(|p| p.key == key)
            .ok_or("This property is not shared by the selection")?;
        if let Value::Choice(v) = &value
            && !property.choices.contains(&v.as_str())
        {
            return Err("This choice is unavailable for the selection".into());
        }
        let before = self.snapshot();
        for (id, key) in property.targets {
            let (spec, values) = self
                .control_properties(id)
                .ok_or("Control no longer exists")?;
            if let Value::Choice(v) = &value
                && let Some(PropertyType::Choice(options)) = spec.property(key).map(|p| p.kind)
                && !options.contains(&v.as_str())
            {
                continue;
            }
            if matches!(key, "icon_size" | "icon_rotation")
                && !matches!(values.get("icon"), Some(Value::Icon(_)))
            {
                continue;
            }
            if values.get(key) == Some(&value) {
                continue;
            }
            if let Err(error) = self.set_control_property(id, key, value.clone()) {
                self.restore(before);
                return Err(error);
            }
        }
        Ok(())
    }
    pub fn rotate_selection_icons(&mut self) -> Result<(), String> {
        let property = self
            .selection_properties()
            .into_iter()
            .find(|p| p.key == "icon_rotation")
            .ok_or("The selection does not share icon rotation")?;
        let before = self.snapshot();
        for (id, key) in property.targets {
            let (_, values) = self
                .control_properties(id)
                .ok_or("Control no longer exists")?;
            let rotation = match values.get(key) {
                Some(Value::Choice(v)) => v.parse::<u16>().unwrap_or(0),
                _ => 0,
            };
            if let Err(error) = self.set_control_property(
                id,
                key,
                Value::Choice(((rotation + 90) % 360).to_string()),
            ) {
                self.restore(before);
                return Err(error);
            }
        }
        Ok(())
    }
    /// Relative rotation is available only when every selected shape is a triangle.
    pub fn rotate_selection_shapes(&mut self) -> Result<(), String> {
        let property = self
            .selection_properties()
            .into_iter()
            .find(|p| p.key == "shape_rotation")
            .ok_or("Select only triangles to rotate their shapes")?;
        let before = self.snapshot();
        for (id, key) in property.targets {
            let (_, values) = self
                .control_properties(id)
                .ok_or("Shape no longer exists")?;
            let Some(Value::Number(rotation)) = values.get(key) else {
                self.restore(before);
                return Err("Shape rotation is unavailable".into());
            };
            if let Err(error) =
                self.set_control_property(id, key, Value::Number((rotation + 90.).rem_euclid(360.)))
            {
                self.restore(before);
                return Err(error);
            }
        }
        Ok(())
    }
    pub fn reset_selection_property(&mut self, key: &str) -> Result<(), String> {
        let property = self
            .selection_properties()
            .into_iter()
            .find(|p| p.key == key)
            .ok_or("This property is not shared by the selection")?;
        let before = self.snapshot();
        for (id, key) in property.targets {
            if let Err(error) = self.reset_control_property(id, key) {
                self.restore(before);
                return Err(error);
            }
        }
        Ok(())
    }
}
