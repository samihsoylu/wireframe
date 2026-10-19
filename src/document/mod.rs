// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

//! Versioned project data. Layout caches and editor selection are not persisted here.
mod libraries;
mod migration;
pub use libraries::{ComponentLibrary, ComponentSource, LibraryImport};
mod palette;
pub use palette::Palette;
mod resolve;
mod reviews;
mod validation;

pub use migration::decode;
pub use resolve::{Resolution, ResolvedNode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub use validation::{MAX_DEPTH, MAX_NODES};

pub type Id = u64;
pub type NodePath = Vec<Id>;
pub type Properties = BTreeMap<String, Value>;
pub const FORMAT_VERSION: u32 = 15;
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
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    Icon(IconSource),
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
/// Ordinary groups retain source labels/notes when library structure changes.
pub const GROUP_NAME: &str = "group_name";
pub const GROUP_NOTES: &str = "group_notes";
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
    /// Discard this group's crop, revealing children in their existing positions.
    /// Origins are stored before cropping; descendant crops remain unchanged.
    pub fn ungroup(self) -> Result<Vec<Node>, String> {
        if self.locked {
            return Err("Unlock the group before ungrouping".into());
        }
        match self.content {
            NodeContent::Group { mut children } => {
                for child in &mut children {
                    child.origin = child.origin.translated(self.origin);
                    child.markup |= self.markup;
                    if self.markup
                        && let NodeContent::Instance { overrides, .. } = &mut child.content
                        && let Some(root) = overrides.iter_mut().find(|o| o.path.is_empty())
                    {
                        // The removed ancestor's markup still covers this child,
                        // including an instance that locally cleared its source.
                        root.patch.markup = Some(true);
                    }
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
    pub markup: Option<bool>,
    /// Editing state owned by this instance, never inherited from a definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
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
pub struct NamedVersion {
    pub id: Id,
    pub name: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Alternate {
    pub id: Id,
    pub name: String,
    /// None preserves an isolated legacy label without inventing shared identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<Id>,
    #[serde(default)]
    pub notes: String,
    pub content: Content,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub id: Id,
    /// Retained pages keep their complete content and canonical ordering.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub trashed: bool,
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
    /// Retain the bytes and identity while excluding this asset from live lookup.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub trashed: bool,
    pub name: String,
    pub media_type: String,
    pub bytes: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Palette::is_default")]
    pub palette: Palette,
    pub sketch: bool,
    pub font_size: f32,
}
/// Foreign page identity and alternate bindings. Retired alternate IDs remain
/// reserved so later imports cannot attach to an unrelated resource.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageImport {
    pub project: String,
    pub page: Id,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub alternates: BTreeMap<Id, Id>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetImport {
    pub project: String,
    pub asset: Id,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    /// Discussions are scoped to a page or alternate, including retained pages.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub reviews: BTreeMap<Id, crate::review::Discussion>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub asset_imports: BTreeMap<Id, Vec<AssetImport>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub page_imports: BTreeMap<Id, Vec<PageImport>>,
    /// Known removed destinations remain identifiable without retaining their content.
    #[serde(default, skip_serializing_if = "std::collections::BTreeSet::is_empty")]
    pub removed_pages: std::collections::BTreeSet<Id>,
    /// Reserved image identities without names, metadata or encoded bytes.
    #[serde(default, skip_serializing_if = "std::collections::BTreeSet::is_empty")]
    pub removed_assets: std::collections::BTreeSet<Id>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub libraries: Vec<ComponentLibrary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_library: Option<Id>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub versions: Vec<NamedVersion>,
    pub format_version: u32,
    pub title: String,
    pub next_id: Id,
    pub settings: Settings,
    pub pages: Vec<Page>,
    #[serde(default)]
    pub definitions: Vec<Definition>,
    #[serde(default)]
    pub assets: Vec<Asset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<std::sync::Arc<crate::fonts::ProjectFont>>,
}
impl Default for Project {
    fn default() -> Self {
        Self {
            reviews: BTreeMap::new(),
            asset_imports: BTreeMap::new(),
            page_imports: BTreeMap::new(),
            removed_pages: Default::default(),
            removed_assets: Default::default(),
            libraries: vec![],
            default_library: None,
            format_version: FORMAT_VERSION,
            versions: vec![],
            title: "Untitled project".into(),
            next_id: 2,
            settings: Settings {
                palette: Palette::default(),
                sketch: true,
                font_size: 13.,
            },
            pages: vec![Page {
                id: 1,
                trashed: false,
                name: "Wireframe 1".into(),
                notes: String::new(),
                parent: None,
                content: Content::default(),
                alternates: vec![],
            }],
            definitions: vec![],
            assets: vec![],
            font: Some(crate::fonts::default_project_font()),
        }
    }
}
impl Project {
    pub fn live_assets(&self) -> impl DoubleEndedIterator<Item = &Asset> {
        self.assets.iter().filter(|asset| !asset.trashed)
    }
    pub fn live_asset(&self, id: Id) -> Option<&Asset> {
        self.live_assets().find(|asset| asset.id == id)
    }
    /// Live navigation is separate from the complete retained resource graph.
    pub fn live_pages(&self) -> impl DoubleEndedIterator<Item = &Page> {
        self.pages.iter().filter(|page| !page.trashed)
    }
    pub fn live_page(&self, id: Id) -> Option<&Page> {
        self.live_pages().find(|page| page.id == id)
    }
    pub fn first_live_page(&self) -> &Page {
        self.live_pages()
            .next()
            .expect("Validated project has a live page")
    }
    /// A retained parent is preserved in storage but absent from visible hierarchy.
    pub fn live_parent(&self, page: &Page) -> Option<Id> {
        page.parent.filter(|id| self.live_page(*id).is_some())
    }
    /// Official pages, alternate pages and component definitions share live surface lookup.
    pub fn surface(&self, id: Id) -> Option<(&str, &Content)> {
        for page in self.live_pages() {
            if page.id == id {
                return Some((&page.name, &page.content));
            }
            if let Some(alt) = page.alternates.iter().find(|a| a.id == id) {
                return Some((&page.name, &alt.content));
            }
        }
        self.live_definitions()
            .find(|d| d.id == id)
            .map(|d| (d.name.as_str(), &d.content))
            .or_else(|| {
                self.live_libraries()
                    .find(|l| l.id == id)
                    .map(|l| (l.name.as_str(), &l.content))
            })
    }
    pub fn surface_notes(&self, id: Id) -> &str {
        for page in self.live_pages() {
            if page.id == id {
                return &page.notes;
            }
            if let Some(alt) = page.alternates.iter().find(|a| a.id == id) {
                return &alt.notes;
            }
        }
        if let Some(library) = self.live_libraries().find(|l| l.id == id) {
            return &library.notes;
        }
        self.live_component_source(id)
            .map_or("", |(_, source)| source.notes.as_str())
    }
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
        for library in &self.libraries {
            reserved(&library.content.nodes, &mut ids);
            for import in &library.imports {
                ids.extend(import.definitions.values().copied());
            }
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
