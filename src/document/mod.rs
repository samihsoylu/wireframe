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
/// Internal marker on disposable resolved placeholders; canonical instances retain their data.
pub const MISSING_COMPONENT: &str = "_missing_component";
pub const EMPTY_COMPONENT: &str = "_empty_component";
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

/// Normalized nondestructive crop within the oriented uncropped content.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Crop {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
impl Crop {
    pub const ALL: Self = Self {
        x: 0.,
        y: 0.,
        width: 1.,
        height: 1.,
    };
    pub fn valid(self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
            && self.width > 0.
            && self.height > 0.
            && self.x + self.width <= 1.00001
            && self.y + self.height <= 1.00001
    }
}
/// Missing patch inherits; Clear explicitly removes an inherited crop.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum CropOverride {
    Clear,
    Region(Crop),
}
impl CropOverride {
    pub fn value(self) -> Option<Crop> {
        match self {
            Self::Clear => None,
            Self::Region(c) => Some(c),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Link {
    Page(Id),
    Url(String),
    Back,
}
impl Link {
    pub fn target(&self) -> String {
        match self {
            Self::Page(id) => format!("page:{id}"),
            Self::Url(url) => url.clone(),
            Self::Back => "back:".into(),
        }
    }
    pub fn valid(&self) -> bool {
        match self {
            Self::Url(url) => {
                url.len() <= 10_000
                    && (url.starts_with("https://") || url.starts_with("http://"))
                    && !url.chars().any(char::is_control)
            }
            _ => true,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source", content = "id", rename_all = "snake_case")]
pub enum IconSource {
    Catalog(String),
    Asset(Id),
}
impl IconSource {
    pub fn valid(&self) -> bool {
        match self {
            Self::Asset(id) => *id != 0,
            Self::Catalog(id) => {
                !id.is_empty()
                    && id.len() <= 128
                    && id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            }
        }
    }
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
    Links(Vec<Option<Link>>),
    Icons(Vec<Option<IconSource>>),
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
    /// Remove derived identity when materializing a resolved group as ordinary content.
    pub fn clear_empty_component_markers(&mut self) {
        self.properties.remove(EMPTY_COMPONENT);
        if let NodeContent::Group { children } = &mut self.content {
            for child in children {
                child.clear_empty_component_markers();
            }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<CropOverride>,
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
        fn reserved(nodes: &[Node], ids: &mut std::collections::HashSet<Id>) {
            for node in nodes {
                match &node.content {
                    NodeContent::Group { children } => reserved(children, ids),
                    NodeContent::Instance { overrides, .. } => {
                        for patch in overrides {
                            ids.extend(patch.path.iter().copied());
                        }
                    }
                    _ => {}
                }
            }
        }
        // Removed child identities still belong to retained overrides, including imported ones.
        let mut ids = std::collections::HashSet::new();
        for page in &self.pages {
            reserved(&page.content.nodes, &mut ids);
            for alternate in &page.alternates {
                reserved(&alternate.content.nodes, &mut ids);
            }
        }
        for definition in &self.definitions {
            reserved(&definition.content.nodes, &mut ids);
        }
        let mut id = self.next_id;
        while ids.contains(&id) {
            id = id.checked_add(1).ok_or("Project ID space exhausted")?;
        }
        let next = id
            .checked_add(1)
            .filter(|n| *n < u64::MAX)
            .ok_or("Project ID space exhausted")?;
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
}

impl Content {
    /// Empty groups are visible atomic units. Nonempty groups retain leaf ordering.
    fn structural_paths(&self) -> Result<(Vec<NodePath>, Vec<NodePath>), String> {
        fn collect(
            nodes: &[Node],
            path: &mut NodePath,
            all: &mut Vec<NodePath>,
            empty: &mut Vec<NodePath>,
        ) -> Result<(), String> {
            if path.len() > MAX_DEPTH {
                return Err("Control hierarchy is too deep".into());
            }
            for node in nodes {
                path.push(node.id);
                match &node.content {
                    NodeContent::Group { children } if !children.is_empty() => {
                        collect(children, path, all, empty)?
                    }
                    NodeContent::Group { .. } => {
                        all.push(path.clone());
                        empty.push(path.clone());
                    }
                    _ => all.push(path.clone()),
                }
                path.pop();
            }
            Ok(())
        }
        let (mut all, mut empty) = (vec![], vec![]);
        collect(&self.nodes, &mut vec![], &mut all, &mut empty)?;
        Ok((all, empty))
    }
    pub fn paint_paths(&self) -> Result<Vec<NodePath>, String> {
        let (paths, empty) = self.structural_paths()?;
        if self.paint_order.is_empty() {
            return Ok(paths);
        }
        let expected: std::collections::BTreeSet<_> = paths.iter().collect();
        let actual: std::collections::BTreeSet<_> = self.paint_order.iter().collect();
        // Older files omitted invisible empty groups. Preserve existing relative order;
        // their newly visible placeholders are appended without rewriting the file.
        if actual.len() != self.paint_order.len()
            || !actual.is_subset(&expected)
            || expected.difference(&actual).any(|p| !empty.contains(p))
        {
            return Err("Invalid or incomplete paint order".into());
        }
        let mut order = self.paint_order.clone();
        order.extend(paths.into_iter().filter(|p| !actual.contains(p)));
        Ok(order)
    }
    /// Keep the layer position when a group loses its last child or gains its first.
    /// Other stale or malformed paths remain validation errors.
    pub fn repair_empty_order(&mut self, before: &Content) -> Result<(), String> {
        if self.paint_order.is_empty() {
            return Ok(());
        }
        let (now, new_empty) = self.structural_paths()?;
        let (_, old_empty) = before.structural_paths()?;
        for path in old_empty {
            if !now.contains(&path)
                && let Some(index) = self.paint_order.iter().position(|p| *p == path)
            {
                let children: Vec<_> = now
                    .iter()
                    .filter(|p| p.starts_with(&path))
                    .cloned()
                    .collect();
                self.paint_order.remove(index);
                self.paint_order.retain(|p| !children.contains(p));
                let index = index.min(self.paint_order.len());
                self.paint_order.splice(index..index, children);
            }
        }
        let old_order = before.paint_paths()?;
        for path in new_empty {
            if self.paint_order.contains(&path) {
                continue;
            }
            let index = old_order
                .iter()
                .position(|p| p.starts_with(&path))
                .and_then(|start| {
                    old_order[start..]
                        .iter()
                        .find_map(|p| self.paint_order.iter().position(|q| q == p))
                })
                .unwrap_or(self.paint_order.len());
            self.paint_order.insert(index, path);
        }
        Ok(())
    }
}
