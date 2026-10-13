// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

//! Versioned project data. Layout caches and editor selection are not persisted here.
mod migration;
mod resolve;
mod validation;

pub use migration::decode;
pub use resolve::{Resolution, ResolvedNode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub use validation::{MAX_DEPTH, MAX_NODES};

pub type Id = u64;
pub type NodePath = Vec<Id>;
pub type Properties = BTreeMap<String, Value>;
pub const FORMAT_VERSION: u32 = 2;
pub const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn translated(self, by: Self) -> Self {
        Self::new(self.x + by.x, self.y + by.y)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dimension {
    Auto,
    Fixed(f32),
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub width: Dimension,
    pub height: Dimension,
}
impl Size {
    pub fn fixed(width: f32, height: f32) -> Self {
        Self {
            width: Dimension::Fixed(width),
            height: Dimension::Fixed(height),
        }
    }
}
impl Default for Size {
    fn default() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Auto,
        }
    }
}

/// Normalized nondestructive crop within the uncropped content, before rotation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Crop {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Link {
    Page(Id),
    Url(String),
    Back,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Value {
    Bool(bool),
    Number(f64),
    Text(String),
    Color([u8; 4]),
    Choice(String),
    Link(Link),
    Asset(Id),
    Null,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: Id,
    pub origin: Point,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub markup: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<Crop>,
    #[serde(default)]
    pub properties: Properties,
    pub content: NodeContent,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeContent {
    Control {
        kind: String,
        size: Size,
    },
    Group {
        children: Vec<Node>,
    },
    Instance {
        definition: Id,
        overrides: Vec<Override>,
    },
}
impl Node {
    pub fn control(id: Id, kind: impl Into<String>, origin: Point, size: Size) -> Self {
        Self {
            id,
            origin,
            locked: false,
            markup: false,
            crop: None,
            properties: Properties::new(),
            content: NodeContent::Control {
                kind: kind.into(),
                size,
            },
        }
    }
    pub fn group(id: Id, origin: Point, children: Vec<Node>) -> Self {
        Self {
            id,
            origin,
            locked: false,
            markup: false,
            crop: None,
            properties: Properties::new(),
            content: NodeContent::Group { children },
        }
    }
    /// Translate immediate children into the parent's coordinate system.
    /// Cropped groups need an explicit layout-aware operation instead.
    pub fn ungroup(self) -> Result<Vec<Node>, String> {
        if self.crop.is_some() || self.locked {
            return Err("Unlock and remove the crop before ungrouping".into());
        }
        match self.content {
            NodeContent::Group { mut children } => {
                for child in &mut children {
                    child.origin = child.origin.translated(self.origin);
                    child.markup |= self.markup;
                }
                Ok(children)
            }
            _ => Err("Only groups can be ungrouped".into()),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Patch {
    #[serde(default)]
    pub properties: Properties,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<Point>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<Size>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Override {
    pub path: NodePath,
    pub patch: Patch,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Content {
    pub nodes: Vec<Node>,
    /// Optional leaf order for migrated documents whose logical groups interleave.
    /// Empty means ordinary depth-first sibling order. Paths retain parent transforms.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paint_order: Vec<NodePath>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Alternate {
    pub id: Id,
    pub name: String,
    pub content: Content,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub id: Id,
    pub name: String,
    pub notes: String,
    #[serde(default)]
    pub parent: Option<Id>,
    pub content: Content,
    #[serde(default)]
    pub alternates: Vec<Alternate>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Definition {
    pub id: Id,
    pub name: String,
    pub content: Content,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub id: Id,
    pub name: String,
    pub media_type: String,
    pub bytes: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub sketch: bool,
    pub font_size: f32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub format_version: u32,
    pub title: String,
    pub next_id: Id,
    pub settings: Settings,
    pub pages: Vec<Page>,
    #[serde(default)]
    pub definitions: Vec<Definition>,
    #[serde(default)]
    pub assets: Vec<Asset>,
}
impl Default for Project {
    fn default() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            title: "Untitled project".into(),
            next_id: 2,
            settings: Settings {
                sketch: true,
                font_size: 13.,
            },
            pages: vec![Page {
                id: 1,
                name: "Wireframe 1".into(),
                notes: String::new(),
                parent: None,
                content: Content::default(),
                alternates: vec![],
            }],
            definitions: vec![],
            assets: vec![],
        }
    }
}
impl Project {
    pub fn allocate_id(&mut self) -> Result<Id, String> {
        let next = self
            .next_id
            .checked_add(1)
            .filter(|id| *id < u64::MAX)
            .ok_or("Project ID space exhausted")?;
        let id = self.next_id;
        self.next_id = next;
        Ok(id)
    }
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        if bytes.len() > MAX_FILE_BYTES {
            return Err("Project exceeds the 64 MB file limit".into());
        }
        Ok(bytes)
    }
    /// Explicit values override context/catalog defaults; Null is an explicit value.
    pub fn effective_properties(&self, node: &Node, defaults: &Properties) -> Properties {
        let mut values = defaults.clone();
        values
            .entry("font_size".into())
            .or_insert(Value::Number(self.settings.font_size as f64));
        values.extend(node.properties.clone());
        values
    }
}
