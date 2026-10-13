// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::collections::{HashMap, HashSet};
pub const MAX_DEPTH: usize = 24;
pub const MAX_NODES: usize = 50_000;

struct Check<'a> {
    project: &'a Project,
    pages: HashSet<Id>,
    assets: HashSet<Id>,
    definitions: HashSet<Id>,
    count: usize,
    maximum_id: Id,
}
impl Project {
    pub fn validate(&self) -> Result<(), String> {
        if self.format_version != FORMAT_VERSION {
            return Err(format!(
                "Unsupported document version {}",
                self.format_version
            ));
        }
        if self.pages.is_empty() || self.pages.len() > 1000 {
            return Err("A project must have 1–1000 pages".into());
        }
        text(&self.title, 10000)?;
        if !self.settings.font_size.is_finite() || !(6.0..=120.0).contains(&self.settings.font_size)
        {
            return Err("Invalid project font size".into());
        }
        let mut resources = HashSet::new();
        let mut check = Check {
            project: self,
            pages: self.pages.iter().map(|p| p.id).collect(),
            assets: self.assets.iter().map(|a| a.id).collect(),
            definitions: self.definitions.iter().map(|d| d.id).collect(),
            count: 0,
            maximum_id: 0,
        };
        for id in self
            .pages
            .iter()
            .map(|p| p.id)
            .chain(self.assets.iter().map(|a| a.id))
            .chain(self.definitions.iter().map(|d| d.id))
            .chain(
                self.pages
                    .iter()
                    .flat_map(|p| p.alternates.iter().map(|a| a.id)),
            )
        {
            if id == 0 || !resources.insert(id) {
                return Err("Duplicate or zero resource ID".into());
            }
            check.maximum_id = check.maximum_id.max(id);
        }
        let mut asset_bytes = 0usize;
        for a in &self.assets {
            text(&a.name, 10000)?;
            text(&a.media_type, 200)?;
            asset_bytes = asset_bytes
                .checked_add(a.bytes.len())
                .ok_or("Asset size overflow")?;
            if asset_bytes > 32 * 1024 * 1024 {
                return Err("Embedded assets exceed 32 MB".into());
            }
        }
        for page in &self.pages {
            text(&page.name, 10000)?;
            text(&page.notes, 1_000_000)?;
            let mut parent = page.parent;
            let mut seen = HashSet::from([page.id]);
            while let Some(id) = parent {
                if !seen.insert(id) {
                    return Err("Cyclic page hierarchy".into());
                }
                parent = self
                    .pages
                    .iter()
                    .find(|p| p.id == id)
                    .ok_or("Missing parent page")?
                    .parent;
            }
            check.content(&page.content)?;
            for alt in &page.alternates {
                text(&alt.name, 10000)?;
                check.content(&alt.content)?;
            }
        }
        for d in &self.definitions {
            text(&d.name, 10000)?;
            check.content(&d.content)?;
            if !d.content.paint_order.is_empty() {
                return Err("Component definitions cannot use legacy paint order".into());
            }
        }
        let mut heights = HashMap::new();
        for d in &self.definitions {
            check.definition_height(d.id, &mut HashSet::new(), &mut heights)?;
        }
        for page in &self.pages {
            check.nodes_height(&page.content.nodes, &mut HashSet::new(), &mut heights)?;
            for alt in &page.alternates {
                check.nodes_height(&alt.content.nodes, &mut HashSet::new(), &mut heights)?;
            }
        }
        if self.next_id <= check.maximum_id || self.next_id == u64::MAX {
            return Err("Invalid next ID".into());
        }
        Ok(())
    }
}
impl Check<'_> {
    fn content(&mut self, c: &Content) -> Result<(), String> {
        self.nodes(&c.nodes, 0)?;
        if !c.paint_order.is_empty() {
            c.paint_paths()?;
        }
        Ok(())
    }
    fn nodes(&mut self, nodes: &[Node], depth: usize) -> Result<(), String> {
        if depth > MAX_DEPTH {
            return Err("Control hierarchy is too deep".into());
        }
        let mut ids = HashSet::new();
        for n in nodes {
            self.count += 1;
            if self.count > MAX_NODES {
                return Err("Project exceeds 50,000 controls".into());
            }
            if n.id == 0 || !ids.insert(n.id) {
                return Err("Duplicate or zero sibling ID".into());
            }
            self.maximum_id = self.maximum_id.max(n.id);
            point(n.origin)?;
            self.properties(&n.properties)?;
            if let Some(c) = n.crop
                && (![c.x, c.y, c.width, c.height]
                    .iter()
                    .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
                    || c.width == 0.
                    || c.height == 0.
                    || c.x + c.width > 1.00001
                    || c.y + c.height > 1.00001)
            {
                return Err("Invalid crop".into());
            }
            match &n.content {
                NodeContent::Control { kind, size: s } => {
                    text(kind, 100)?;
                    if kind.is_empty() {
                        return Err("Missing control kind".into());
                    }
                    size(*s)?;
                    if let Some(spec) = crate::catalog::get(kind) {
                        for (key, value) in &n.properties {
                            if let Some(property) = spec.property(key) {
                                property.validate(value).map_err(|e| e.to_string())?;
                            }
                        }
                    }
                }
                NodeContent::Group { children } => self.nodes(children, depth + 1)?,
                NodeContent::Instance {
                    definition,
                    overrides,
                } => {
                    if !self.definitions.contains(definition) {
                        return Err("Missing component definition".into());
                    }
                    let mut paths = HashSet::new();
                    for o in overrides {
                        if o.path.is_empty()
                            || o.path.len() > MAX_DEPTH
                            || o.path.contains(&0)
                            || !paths.insert(&o.path)
                        {
                            return Err("Invalid or duplicate override path".into());
                        }
                        self.properties(&o.patch.properties)?;
                        if let Some(p) = o.patch.origin {
                            point(p)?;
                        }
                        if let Some(s) = o.patch.size {
                            size(s)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn properties(&self, p: &Properties) -> Result<(), String> {
        if p.len() > 256 {
            return Err("Too many control properties".into());
        }
        for (key, value) in p {
            text(key, 100)?;
            if key.is_empty() {
                return Err("Empty property name".into());
            }
            match value {
                Value::Number(n) if !n.is_finite() => {
                    return Err("Non-finite property value".into());
                }
                Value::Number(n) if key == "font_size" && !(6.0..=120.0).contains(n) => {
                    return Err("Invalid font size".into());
                }
                Value::Text(s) | Value::Choice(s) => text(s, 100_000)?,
                Value::Link(Link::Page(id)) if !self.pages.contains(id) => {
                    return Err("Link points to a missing page".into());
                }
                Value::Link(Link::Url(s)) => {
                    text(s, 10000)?;
                    if !(s.starts_with("https://") || s.starts_with("http://"))
                        || s.chars().any(char::is_control)
                    {
                        return Err("Unsupported link URL".into());
                    }
                }
                Value::Asset(id) if !self.assets.contains(id) => {
                    return Err("Missing image asset".into());
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn definition_height(
        &self,
        id: Id,
        active: &mut HashSet<Id>,
        heights: &mut HashMap<Id, usize>,
    ) -> Result<usize, String> {
        if !active.insert(id) {
            return Err("Cyclic component definition".into());
        }
        if active.len() > MAX_DEPTH + 1 {
            return Err("Component dependency chain is too deep".into());
        }
        let height = if let Some(height) = heights.get(&id) {
            *height
        } else {
            let definition = self
                .project
                .definitions
                .iter()
                .find(|d| d.id == id)
                .ok_or("Missing component definition")?;
            let height = self.nodes_height(&definition.content.nodes, active, heights)?;
            heights.insert(id, height);
            height
        };
        active.remove(&id);
        Ok(height)
    }
    fn nodes_height(
        &self,
        nodes: &[Node],
        active: &mut HashSet<Id>,
        heights: &mut HashMap<Id, usize>,
    ) -> Result<usize, String> {
        let mut maximum = 0;
        for node in nodes {
            let height = match &node.content {
                NodeContent::Control { .. } => 0,
                NodeContent::Group { children } => {
                    1 + self.nodes_height(children, active, heights)?
                }
                NodeContent::Instance { definition, .. } => {
                    1 + self.definition_height(*definition, active, heights)?
                }
            };
            maximum = maximum.max(height);
            if maximum > MAX_DEPTH {
                return Err("Expanded component hierarchy is too deep".into());
            }
        }
        Ok(maximum)
    }
}
fn text(s: &str, max: usize) -> Result<(), String> {
    if s.len() > max {
        Err("Project text exceeds its limit".into())
    } else {
        Ok(())
    }
}
fn point(p: Point) -> Result<(), String> {
    if [p.x, p.y]
        .iter()
        .all(|v| v.is_finite() && v.abs() <= 100_000.)
    {
        Ok(())
    } else {
        Err("Invalid control position".into())
    }
}
fn size(s: Size) -> Result<(), String> {
    for d in [s.width, s.height] {
        if let Dimension::Fixed(v) = d
            && (!v.is_finite() || !(1.0..=100_000.).contains(&v))
        {
            return Err("Invalid control size".into());
        }
    }
    Ok(())
}
