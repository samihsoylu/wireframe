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
    resources: HashSet<Id>,
    count: usize,
    maximum_id: Id,
}
impl Project {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_libraries(true)
    }
    pub(super) fn validate_libraries(&self, require_owners: bool) -> Result<(), String> {
        if self.format_version != FORMAT_VERSION {
            return Err(format!(
                "Unsupported document version {}",
                self.format_version
            ));
        }
        let live = self.live_pages().count();
        self.validate_reviews()?;
        if live == 0 || live > 1000 || self.pages.len() - live > 1000 {
            return Err(
                "A project must have 1–1000 live pages and at most 1000 trashed pages".into(),
            );
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
            resources: HashSet::new(),
            count: 0,
            maximum_id: 0,
        };
        for id in self
            .pages
            .iter()
            .map(|p| p.id)
            .chain(self.assets.iter().map(|a| a.id))
            .chain(self.versions.iter().map(|v| v.id))
            .chain(self.definitions.iter().map(|d| d.id))
            .chain(self.libraries.iter().map(|l| l.id))
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
        check.resources = resources;
        if self.removed_pages.len() > MAX_NODES {
            return Err("Too many removed page identities".into());
        }
        for id in &self.removed_pages {
            if *id == 0 || !check.resources.insert(*id) {
                return Err("Removed page identity is zero or reused".into());
            }
            check.maximum_id = check.maximum_id.max(*id);
        }
        if self.removed_assets.len() > MAX_NODES {
            return Err("Too many removed image identities".into());
        }
        for id in &self.removed_assets {
            if *id == 0 || !check.resources.insert(*id) {
                return Err("Removed image identity is zero or reused".into());
            }
            check.maximum_id = check.maximum_id.max(*id);
        }
        let mut asset_import_count = 0usize;
        for (&owner, records) in &self.asset_imports {
            if !check.assets.contains(&owner) {
                return Err("Asset import correspondence has no owning asset".into());
            }
            let mut origins = HashSet::new();
            for record in records {
                asset_import_count += 1;
                if asset_import_count > MAX_NODES {
                    return Err("Project exceeds asset import correspondence limits".into());
                }
                if record.project.is_empty()
                    || record.project.len() > 512
                    || record.project.chars().any(char::is_control)
                    || record.asset == 0
                    || !origins.insert((&record.project, record.asset))
                {
                    return Err("Invalid or duplicate asset import origin".into());
                }
            }
        }
        let mut version_names = HashSet::new();
        let mut page_import_count = 0usize;
        let mut alternate_binding_count = 0usize;
        let mut alternate_owners = HashMap::new();
        for (&owner, records) in &self.page_imports {
            let page = self
                .pages
                .iter()
                .find(|p| p.id == owner)
                .ok_or("Page import correspondence has no owning page")?;
            let mut origins = HashSet::new();
            for record in records {
                page_import_count += 1;
                alternate_binding_count += record.alternates.len();
                if page_import_count > MAX_NODES || alternate_binding_count > MAX_NODES {
                    return Err("Project exceeds page import correspondence limits".into());
                }
                if record.project.is_empty()
                    || record.project.len() > 512
                    || record.project.chars().any(char::is_control)
                    || record.page == 0
                    || !origins.insert((&record.project, record.page))
                {
                    return Err("Invalid or duplicate page import origin".into());
                }
                let mut targets = HashSet::new();
                for (&foreign, &local) in &record.alternates {
                    if foreign == 0
                        || local == 0
                        || !targets.insert(local)
                        || alternate_owners
                            .insert(local, owner)
                            .is_some_and(|old| old != owner)
                        || (check.resources.contains(&local)
                            && !page.alternates.iter().any(|a| a.id == local))
                    {
                        return Err("Invalid page import alternate correspondence".into());
                    }
                    check.maximum_id = check.maximum_id.max(local);
                }
            }
        }
        for version in &self.versions {
            text(&version.name, 10000)?;
            if version.name.trim().is_empty() || !version_names.insert(&version.name) {
                return Err("Empty or duplicate version name".into());
            }
        }
        let mut asset_bytes = 0usize;
        for a in &self.assets {
            text(&a.name, 10000)?;
            text(&a.media_type, 200)?;
            if let Some(credit) = &a.attribution {
                for value in [
                    &credit.title,
                    &credit.creator,
                    &credit.license,
                    &credit.license_url,
                    &credit.source_url,
                ] {
                    text(value, 4000)?;
                }
                for address in [&credit.license_url, &credit.source_url] {
                    if !address.is_empty()
                        && !url::Url::parse(address).is_ok_and(|u| {
                            u.scheme() == "https"
                                && u.username().is_empty()
                                && u.password().is_none()
                        })
                    {
                        return Err("Invalid image attribution link".into());
                    }
                }
            }
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
            let mut memberships = HashSet::new();
            for alt in &page.alternates {
                text(&alt.name, 10000)?;
                text(&alt.notes, 1_000_000)?;
                if let Some(id) = alt.version {
                    let version = self
                        .versions
                        .iter()
                        .find(|v| v.id == id)
                        .ok_or("Missing named version")?;
                    if version.name != alt.name || !memberships.insert(id) {
                        return Err("Inconsistent or duplicate page version".into());
                    }
                }
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
        if self.libraries.len() > 1000 {
            return Err("Project exceeds 1000 component libraries".into());
        }
        if let Some(id) = self.default_library
            && !self.libraries.iter().any(|l| l.id == id)
        {
            return Err("Default component library is missing".into());
        }
        let mut import_records = 0usize;
        let mut import_bindings = 0usize;
        let mut owners = HashSet::new();
        for library in &self.libraries {
            text(&library.name, 10000)?;
            text(&library.notes, 1_000_000)?;
            if library.name.trim().is_empty() {
                return Err("Give the component library a name".into());
            }
            let mut origins = HashSet::new();
            for import in &library.imports {
                import_records = import_records
                    .checked_add(1)
                    .ok_or("Import record overflow")?;
                import_bindings = import_bindings
                    .checked_add(import.definitions.len())
                    .ok_or("Import binding overflow")?;
                if import_records > 1000 || import_bindings > MAX_NODES {
                    return Err("Project exceeds library import correspondence limits".into());
                }
                if import.project.is_empty()
                    || import.project.len() > 512
                    || import.project.chars().any(char::is_control)
                    || import.library == 0
                    || !origins.insert((&import.project, import.library))
                {
                    return Err("Invalid or duplicate library import origin".into());
                }
                let mut targets = HashSet::new();
                for (&source, &target) in &import.definitions {
                    if source == 0
                        || target == 0
                        || !targets.insert(target)
                        || (check.resources.contains(&target)
                            && !self.definitions.iter().any(|d| d.id == target))
                    {
                        return Err("Invalid library import definition correspondence".into());
                    }
                    // Deleted targets remain reserved; an unrelated new source must not acquire them.
                    check.maximum_id = check.maximum_id.max(target);
                }
            }
            let mut source_nodes = HashSet::new();
            for source in &library.sources {
                text(&source.notes, 1_000_000)?;
                if !owners.insert(source.definition)
                    || !source_nodes.insert(source.node)
                    || !self.definitions.iter().any(|d| d.id == source.definition)
                {
                    return Err("Invalid or duplicate component source ownership".into());
                }
                let node = library
                    .content
                    .nodes
                    .iter()
                    .find(|n| n.id == source.node)
                    .ok_or("Source placement is missing")?;
                if !matches!(&node.content, NodeContent::Instance { definition, overrides } if *definition == source.definition && overrides.is_empty())
                    || !node.properties.is_empty()
                {
                    return Err("Source placement cannot override canonical content".into());
                }
            }
            let mut siblings = HashSet::new();
            for node in &library.content.nodes {
                if node.id == 0 || !siblings.insert(node.id) {
                    return Err("Duplicate or zero library node ID".into());
                }
                if source_nodes.contains(&node.id) {
                    point(node.origin)?;
                    if node.crop.is_some_and(|crop| !crop.valid()) {
                        return Err("Invalid source crop".into());
                    }
                    check.maximum_id = check.maximum_id.max(node.id);
                } else {
                    check.nodes(std::slice::from_ref(node), 0)?;
                }
            }
            if !library.content.paint_order.is_empty() {
                library.content.paint_paths()?;
            }
        }
        if require_owners && owners.len() != self.definitions.len() {
            return Err("Every component requires a library source placement".into());
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
        for library in &self.libraries {
            for node in &library.content.nodes {
                if !library.sources.iter().any(|s| s.node == node.id) {
                    check.nodes_height(
                        std::slice::from_ref(node),
                        &mut HashSet::new(),
                        &mut heights,
                    )?;
                }
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
            self.properties(
                &n.properties,
                matches!(n.content, NodeContent::Group { .. }),
            )?;
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
                    if *definition == 0
                        || (self.resources.contains(definition)
                            && !self.project.definitions.iter().any(|d| d.id == *definition))
                    {
                        return Err("Invalid component reference ID".into());
                    }
                    // Dangling references must never be reused by subsequent allocations.
                    self.maximum_id = self.maximum_id.max(*definition);
                    let mut paths = HashSet::new();
                    for o in overrides {
                        if o.path.len() > MAX_DEPTH || o.path.contains(&0) || !paths.insert(&o.path)
                        {
                            return Err("Invalid or duplicate override path".into());
                        }
                        if o.path.is_empty()
                            && (o.patch.locked.is_some()
                                || o.patch.origin.is_some()
                                || o.patch.size.is_some()
                                || !o.patch.properties.is_empty())
                        {
                            return Err("Root overrides support only crop and markup".into());
                        }
                        self.properties(&o.patch.properties, false)?;
                        if let Some(CropOverride::Region(c)) = o.patch.crop
                            && !c.valid()
                        {
                            return Err("Invalid crop override".into());
                        }
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
    fn properties(&self, p: &Properties, group: bool) -> Result<(), String> {
        if p.len() > 256 {
            return Err("Too many control properties".into());
        }
        for (key, value) in p {
            text(key, 100)?;
            if key.is_empty() {
                return Err("Empty property name".into());
            }
            match value {
                Value::Text(s) if group && key == GROUP_NAME => text(s, 10_000)?,
                Value::Text(s) if group && key == GROUP_NOTES => text(s, 1_000_000)?,
                _ if group && (key == GROUP_NAME || key == GROUP_NOTES) => {
                    return Err("Invalid group metadata".into());
                }
                Value::Number(n) if !n.is_finite() => {
                    return Err("Non-finite property value".into());
                }
                Value::Number(n) if key == "font_size" && !(6.0..=120.0).contains(n) => {
                    return Err("Invalid font size".into());
                }
                Value::Text(s) | Value::Choice(s) => text(s, 100_000)?,
                Value::Link(Link::Page(id))
                    if !self.pages.contains(id) && !self.project.removed_pages.contains(id) =>
                {
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
                Value::Icon(icon) if !icon.valid() => return Err("Invalid control icon".into()),
                Value::Icon(IconSource::Asset(id)) if !self.assets.contains(id) && !self.project.removed_assets.contains(id) => {
                    return Err("Missing control icon asset".into());
                }
                Value::Icons(icons)
                    if icons.len() > 10_001 || !icons.iter().flatten().all(IconSource::valid) =>
                {
                    return Err("Invalid row icons".into());
                }
                Value::Icons(icons) => {
                    if icons.iter().flatten().any(
                        |icon| matches!(icon, IconSource::Asset(id) if !self.assets.contains(id) && !self.project.removed_assets.contains(id)),
                    ) {
                        return Err("Missing row icon asset".into());
                    }
                }
                Value::Links(links) => {
                    if links.len() > 10_001 {
                        return Err("Too many row links".into());
                    }
                    for link in links.iter().flatten() {
                        if !link.valid() {
                            return Err("Unsupported row link URL".into());
                        }
                        if let Link::Page(id) = link
                            && !self.pages.contains(id)
                            && !self.project.removed_pages.contains(id)
                        {
                            return Err("Row link points to a missing page".into());
                        }
                    }
                }
                Value::Asset(id) if !self.assets.contains(id) && !self.project.removed_assets.contains(id) => {
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
            let height =
                if let Some(definition) = self.project.definitions.iter().find(|d| d.id == id) {
                    self.nodes_height(&definition.content.nodes, active, heights)?
                } else {
                    0
                };
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
