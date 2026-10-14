// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Control identity, property contracts and context-dependent defaults.
mod entries;
mod fields;
use crate::document::{Dimension, Node, NodeContent, Point, Properties, Size, Value};
use std::sync::LazyLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeAxes {
    Both,
    Horizontal,
    Vertical,
    Computed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoSize {
    None,
    Width,
    Height,
    Independent,
    Linked,
    Intrinsic,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Text,
    Button,
    Choice,
    Input,
    Rows,
    Container,
    Shape,
    Image,
    Chart,
    Decoration,
    Device,
    Media,
}
#[derive(Clone, Copy, Debug)]
pub enum PropertyType {
    Bool,
    Text,
    Color,
    Number { min: f64, max: f64, integer: bool },
    Choice(&'static [&'static str]),
    Link,
    Links,
    Icons,
    Icon,
    Asset,
}
#[derive(Clone, Debug)]
pub enum DefaultValue {
    Literal(Value),
    FontSize,
    Ink,
    Fill,
    ButtonFill,
    SearchInk,
    FormInk,
    LinkColor,
}
#[derive(Clone, Debug)]
pub struct PropertySpec {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: PropertyType,
    pub default: DefaultValue,
    pub nullable: bool,
    pub visible: bool,
}
#[derive(Clone, Debug)]
pub struct ControlSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub aliases: &'static [&'static str],
    pub family: Family,
    pub resize: ResizeAxes,
    pub autosize: AutoSize,
    pub default_size: Size,
    pub properties: Vec<PropertySpec>,
}
#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub sketch: bool,
    pub font_size: f32,
    pub ink: [u8; 4],
    pub fill: [u8; 4],
    pub link_color: [u8; 4],
}
impl Default for Context {
    fn default() -> Self {
        Self {
            sketch: true,
            font_size: 13.,
            ink: [0, 0, 0, 255],
            fill: [255; 4],
            link_color: [0, 102, 204, 255],
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyError {
    pub key: String,
    pub message: String,
}
impl std::fmt::Display for PropertyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.key, self.message)
    }
}
impl std::error::Error for PropertyError {}

pub static CONTROLS: LazyLock<Vec<ControlSpec>> = LazyLock::new(entries::build);
pub fn get(id: &str) -> Option<&'static ControlSpec> {
    let canonical = match id {
        "ellipse" => "shape",
        "line" => "horizontal_rule",
        "panel" => "window",
        other => other,
    };
    CONTROLS.iter().find(|c| c.id == canonical)
}
/// Stable ranking: exact display/ID, exact alias, prefix, then all-token substring.
pub fn search(query: &str) -> Vec<&'static ControlSpec> {
    let query = query.trim().to_lowercase();
    let words: Vec<_> = query.split_whitespace().collect();
    let mut found: Vec<_> = CONTROLS
        .iter()
        .filter_map(|c| {
            let name = c.name.to_lowercase();
            let id = c.id.replace('_', " ");
            let aliases: Vec<_> = c.aliases.iter().map(|s| s.to_lowercase()).collect();
            let rank = if query.is_empty() || name == query || id == query {
                0
            } else if aliases.iter().any(|a| a == &query) {
                1
            } else if name.starts_with(&query) || aliases.iter().any(|a| a.starts_with(&query)) {
                2
            } else {
                let haystack = format!("{name} {id} {}", aliases.join(" "));
                if words.iter().all(|w| haystack.contains(w)) {
                    3
                } else {
                    return None;
                }
            };
            Some((rank, c))
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.name.cmp(b.1.name)));
    found.into_iter().map(|(_, c)| c).collect()
}
impl PropertySpec {
    pub fn default_value(&self, context: &Context) -> Value {
        match &self.default {
            DefaultValue::Literal(v) => v.clone(),
            DefaultValue::FontSize => Value::Number(context.font_size as f64),
            DefaultValue::Ink => Value::Color(context.ink),
            DefaultValue::Fill => Value::Color(context.fill),
            DefaultValue::ButtonFill => Value::Color(if context.sketch {
                context.fill
            } else {
                [221, 221, 221, 255]
            }),
            DefaultValue::SearchInk => Value::Color(if context.sketch {
                [128, 128, 139, 255]
            } else {
                [51, 51, 51, 255]
            }),
            DefaultValue::FormInk => Value::Color(if context.sketch {
                context.ink
            } else {
                [51, 51, 51, 255]
            }),
            DefaultValue::LinkColor => Value::Color(context.link_color),
        }
    }
    pub fn validate(&self, value: &Value) -> Result<(), PropertyError> {
        let valid = if *value == Value::Null {
            self.nullable
        } else {
            match (self.kind, value) {
                (PropertyType::Bool, Value::Bool(_))
                | (PropertyType::Color, Value::Color(_))
                | (PropertyType::Link, Value::Link(_))
                | (PropertyType::Asset, Value::Asset(_)) => true,
                (PropertyType::Icon, Value::Icon(icon)) => icon.valid(),
                (PropertyType::Icons, Value::Icons(icons)) => {
                    icons.len() <= 10_001
                        && icons
                            .iter()
                            .flatten()
                            .all(crate::document::IconSource::valid)
                }
                (PropertyType::Links, Value::Links(links)) => {
                    links.len() <= 10_001
                        && links.iter().flatten().all(crate::document::Link::valid)
                }
                (PropertyType::Text, Value::Text(s)) => s.len() <= 100_000,
                (PropertyType::Number { min, max, integer }, Value::Number(n)) => {
                    n.is_finite() && *n >= min && *n <= max && (!integer || n.fract() == 0.)
                }
                (PropertyType::Choice(options), Value::Choice(s)) => options.contains(&s.as_str()),
                _ => false,
            }
        };
        if valid {
            Ok(())
        } else {
            Err(PropertyError {
                key: self.key.into(),
                message: format!("Invalid value for {}", self.label),
            })
        }
    }
}
impl ControlSpec {
    pub fn property(&self, key: &str) -> Option<&PropertySpec> {
        self.properties.iter().find(|p| p.key == key)
    }
    pub fn create(&self, id: u64, origin: Point) -> Node {
        let mut node = Node::control(id, self.id, origin, self.default_size);
        if self.id == "phone" {
            node.properties
                .insert("device_model".into(), Value::Choice("notched".into()));
            node.properties
                .insert("device_resizing".into(), Value::Choice("stretch".into()));
        }
        if self.id == "circle_button" {
            node.properties.insert(
                "icon".into(),
                Value::Icon(crate::document::IconSource::Catalog("plus-solid".into())),
            );
        }
        if self.id == "pointed_button" {
            node.properties
                .insert("direction".into(), Value::Choice("left".into()));
            node.properties
                .insert("border".into(), Value::Choice("none".into()));
        }
        node
    }
    pub fn defaults(&self, context: &Context) -> Properties {
        self.properties
            .iter()
            .map(|p| (p.key.into(), p.default_value(context)))
            .collect()
    }
    /// Unknown values survive loading so an older build cannot silently strip data.
    /// Inspector edits use set_property, which accepts only known properties.
    pub fn effective(&self, node: &Node, context: &Context) -> Result<Properties, PropertyError> {
        let mut values = self.defaults(context);
        for (key, value) in &node.properties {
            if let Some(spec) = self.property(key) {
                spec.validate(value)?;
            }
            values.insert(key.clone(), value.clone());
        }
        if self.id == "alert"
            && !node.properties.contains_key("text_color")
            && values.get("button_style") == Some(&Value::Choice("inline".into()))
        {
            values.insert("text_color".into(), Value::Color([8, 83, 148, 255]));
        }
        if matches!(self.id, "text_input" | "text_area") {
            if !node.properties.contains_key("link_color") {
                values.insert("link_color".into(), Value::Color([8, 83, 148, 255]));
            }
            let fill = match values.get("fill") {
                Some(Value::Color(c)) => *c,
                _ => context.fill,
            };
            let opacity = match values.get("background_opacity") {
                Some(Value::Number(n)) => *n,
                _ => 1.,
            };
            let dark = opacity >= 0.75
                && u32::from(fill[0]) * 299 + u32::from(fill[1]) * 587 + u32::from(fill[2]) * 114
                    < 153_000;
            if dark {
                for key in ["text_color", "link_color"] {
                    if !node.properties.contains_key(key) {
                        values.insert(key.into(), Value::Color([255; 4]));
                    }
                }
            }
        }
        // Version-1 checkbox/radio/switch values retain their meaning after migration.
        if matches!(self.id, "checkbox" | "radio" | "toggle")
            && !node.properties.contains_key("state")
            && let Some(Value::Bool(checked)) = node.properties.get("checked")
        {
            let state = if self.id == "toggle" {
                if *checked { "on" } else { "off" }
            } else {
                if *checked { "selected" } else { "normal" }
            };
            if let Some(spec) = self.property("state") {
                let value = Value::Choice(state.into());
                if spec.validate(&value).is_ok() {
                    values.insert("state".into(), value);
                }
            }
        }
        if matches!(&node.content,NodeContent::Control{kind,..} if kind=="ellipse")
            && !node.properties.contains_key("shape")
        {
            values.insert("shape".into(), Value::Choice("ellipse".into()));
        }
        Ok(values)
    }
    /// Store only a difference from the current catalog/context default.
    pub fn set_property(
        &self,
        node: &mut Node,
        key: &str,
        value: Value,
        context: &Context,
    ) -> Result<(), PropertyError> {
        // Preserve explicit colors across dependent fill or button-style changes.
        if key == "text_color" && matches!(self.id, "text_input" | "text_area" | "alert") {
            self.property(key)
                .expect("Control text color")
                .validate(&value)?;
            node.properties.insert(key.into(), value);
            return Ok(());
        }
        self.set_override(&mut node.properties, key, value, &self.defaults(context))
    }
    /// Component patches compare against the inherited value, not the global default.
    pub fn set_override(
        &self,
        properties: &mut Properties,
        key: &str,
        value: Value,
        inherited: &Properties,
    ) -> Result<(), PropertyError> {
        let spec = self.property(key).ok_or_else(|| PropertyError {
            key: key.into(),
            message: "Unsupported property".into(),
        })?;
        spec.validate(&value)?;
        if inherited.get(key) == Some(&value) {
            properties.remove(key);
        } else {
            properties.insert(key.into(), value);
        }
        if key == "state" {
            properties.remove("checked");
        }
        Ok(())
    }
    pub fn reset_property(&self, node: &mut Node, key: &str) -> Result<(), PropertyError> {
        self.property(key).ok_or_else(|| PropertyError {
            key: key.into(),
            message: "Unsupported property".into(),
        })?;
        node.properties.remove(key);
        if key == "state" {
            node.properties.remove("checked");
        }
        Ok(())
    }
    pub fn resized(&self, original: (f32, f32), proposed: (f32, f32)) -> (f32, f32) {
        let valid = |v: f32, fallback: f32| {
            if v.is_finite() {
                v.clamp(1., 100_000.)
            } else {
                fallback
            }
        };
        match self.resize {
            ResizeAxes::Both => (valid(proposed.0, original.0), valid(proposed.1, original.1)),
            ResizeAxes::Horizontal => (valid(proposed.0, original.0), original.1),
            ResizeAxes::Vertical => (original.0, valid(proposed.1, original.1)),
            ResizeAxes::Computed => original,
        }
    }
}
fn auto() -> Size {
    Size::default()
}
fn fixed(w: f32, h: f32) -> Size {
    Size::fixed(w, h)
}
fn number(
    key: &'static str,
    label: &'static str,
    value: f64,
    min: f64,
    max: f64,
    integer: bool,
) -> PropertySpec {
    prop(
        key,
        label,
        PropertyType::Number { min, max, integer },
        Value::Number(value),
    )
}
fn prop(key: &'static str, label: &'static str, kind: PropertyType, value: Value) -> PropertySpec {
    PropertySpec {
        key,
        label,
        kind,
        default: DefaultValue::Literal(value),
        nullable: false,
        visible: true,
    }
}
fn boolean(key: &'static str, label: &'static str, value: bool) -> PropertySpec {
    prop(key, label, PropertyType::Bool, Value::Bool(value))
}
fn choice(
    key: &'static str,
    label: &'static str,
    options: &'static [&'static str],
    value: &str,
) -> PropertySpec {
    prop(
        key,
        label,
        PropertyType::Choice(options),
        Value::Choice(value.into()),
    )
}
