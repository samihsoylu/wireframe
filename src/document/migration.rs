// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::model as legacy;
use std::collections::HashMap;

pub fn decode(bytes: &[u8]) -> Result<Project, String> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err("Project exceeds the 64 MB file limit".into());
    }
    #[derive(Deserialize)]
    struct PageState {
        #[serde(default)]
        trashed: bool,
        #[serde(default)]
        attribution: Option<serde_json::Value>,
    }
    #[derive(Deserialize)]
    struct Version {
        format_version: u32,
        #[serde(default)]
        reviews: BTreeMap<Id, serde_json::Value>,
        #[serde(default)]
        removed_pages: Vec<Id>,
        #[serde(default)]
        removed_assets: Vec<Id>,
        #[serde(default)]
        pages: Vec<PageState>,
        #[serde(default)]
        assets: Vec<PageState>,
        #[serde(default)]
        libraries: Vec<serde_json::Value>,
        #[serde(default)]
        default_library: Option<Id>,
    }
    let version: Version =
        serde_json::from_slice(bytes).map_err(|e| format!("Cannot read project: {e}"))?;
    if version.format_version < 16
        && version
            .assets
            .iter()
            .any(|asset| asset.attribution.is_some())
    {
        return Err("Image credits require document version 16".into());
    }
    if version.format_version < 15
        && (!version.removed_assets.is_empty() || version.assets.iter().any(|asset| asset.trashed))
    {
        return Err("Retained images require document version 15".into());
    }
    if version.format_version < 11 && !version.reviews.is_empty() {
        return Err("Review comments require document version 11".into());
    }
    if version.format_version < 10 && !version.removed_pages.is_empty() {
        return Err("Removed page identities require document version 10".into());
    }
    if version.format_version < 5 && version.pages.iter().any(|page| page.trashed) {
        return Err("Retained pages require document version 5".into());
    }
    if version.format_version < 6
        && (!version.libraries.is_empty() || version.default_library.is_some())
    {
        return Err("Component libraries require document version 6".into());
    }
    if version.format_version < 8
        && version.libraries.iter().any(|library| {
            library
                .get("imports")
                .is_some_and(|imports| imports.as_array().is_none_or(|v| !v.is_empty()))
        })
    {
        return Err("Library import correspondence requires document version 8".into());
    }
    if version.format_version < 9
        && version.libraries.iter().any(|library| {
            library
                .get("trashed")
                .is_some_and(|v| v != &serde_json::Value::Bool(false))
        })
    {
        return Err("Retained component libraries require document version 9".into());
    }
    let mut project = match version.format_version {
        1 => Project::try_from(
            serde_json::from_slice::<legacy::Document>(bytes).map_err(|e| e.to_string())?,
        )?,
        2 => {
            let mut project: Project =
                serde_json::from_slice(bytes).map_err(|e| format!("Cannot read project: {e}"))?;
            project.format_version = FORMAT_VERSION;
            project.versions.clear();
            for page in &mut project.pages {
                for alternate in &mut page.alternates {
                    alternate.version = None;
                    // Version 2 displayed/exported the owning page's notes for every alternate.
                    alternate.notes = page.notes.clone();
                }
            }
            project
        }
        3..=15 => {
            let mut project: Project =
                serde_json::from_slice(bytes).map_err(|e| format!("Cannot read project: {e}"))?;
            project.format_version = FORMAT_VERSION;
            project
        }
        FORMAT_VERSION => {
            serde_json::from_slice(bytes).map_err(|e| format!("Cannot read project: {e}"))?
        }
        version => return Err(format!("Unsupported document version {version}")),
    };
    if version.format_version < 7 {
        migrate_source_appearance(&mut project)?;
    }
    if version.format_version < 6 {
        project.validate_libraries(false)?;
        project.organize_components()?;
    }
    if version.format_version < 12 {
        preserve_switch_states(&mut project);
    }
    if version.format_version < 13 {
        preserve_arrow_geometry(&mut project);
    }
    if version.format_version < 14 {
        preserve_rule_frames(&mut project);
    }
    project.validate()?;
    Ok(project)
}

fn preserve_rule_frames(project: &mut Project) {
    fn visit(nodes: &mut [Node]) {
        for node in nodes {
            match &mut node.content {
                NodeContent::Group { children } => visit(children),
                NodeContent::Control { kind, .. }
                    if matches!(kind.as_str(), "line" | "horizontal_rule") =>
                {
                    node.properties
                        .entry("legacy_rule_frame".into())
                        .or_insert(Value::Bool(true));
                    node.properties
                        .entry("border_color".into())
                        .or_insert(Value::Color([44, 48, 55, 255]));
                }
                _ => {}
            }
        }
    }
    for page in &mut project.pages {
        visit(&mut page.content.nodes);
        for alternate in &mut page.alternates {
            visit(&mut alternate.content.nodes);
        }
    }
    for definition in &mut project.definitions {
        visit(&mut definition.content.nodes);
    }
    for library in &mut project.libraries {
        visit(&mut library.content.nodes);
    }
}

// Older arrows used the horizontal midline of their frame and one right head.
// Keep the size-relative fallback so existing component size overrides still work.
fn preserve_arrow_geometry(project: &mut Project) {
    fn visit(nodes: &mut [Node]) {
        for node in nodes {
            match &mut node.content {
                NodeContent::Group { children } => visit(children),
                NodeContent::Control { kind, .. } if kind == "arrow" => {
                    for (key, value) in [
                        ("legacy_horizontal", Value::Bool(true)),
                        ("start_head", Value::Bool(false)),
                        ("end_head", Value::Bool(true)),
                        ("border_color", Value::Color([44, 48, 55, 255])),
                    ] {
                        node.properties.entry(key.into()).or_insert(value);
                    }
                }
                _ => {}
            }
        }
    }
    for page in &mut project.pages {
        visit(&mut page.content.nodes);
        for alternate in &mut page.alternates {
            visit(&mut alternate.content.nodes);
        }
    }
    for definition in &mut project.definitions {
        visit(&mut definition.content.nodes);
    }
    for library in &mut project.libraries {
        visit(&mut library.content.nodes);
    }
}

// The initial switch default was off. Keep sparse older documents off while
// allowing newly created switches to use the corrected on default. Retain the
// legacy checked representation so an instance's checked override still wins.
fn preserve_switch_states(project: &mut Project) {
    fn visit(nodes: &mut [Node]) {
        for node in nodes {
            match &mut node.content {
                NodeContent::Group { children } => visit(children),
                NodeContent::Control { kind, .. }
                    if kind == "toggle" && !node.properties.contains_key("state") =>
                {
                    node.properties
                        .entry("checked".into())
                        .or_insert(Value::Bool(false));
                }
                _ => {}
            }
        }
    }
    for page in &mut project.pages {
        visit(&mut page.content.nodes);
        for alternate in &mut page.alternates {
            visit(&mut alternate.content.nodes);
        }
    }
    for definition in &mut project.definitions {
        visit(&mut definition.content.nodes);
    }
    for library in &mut project.libraries {
        visit(&mut library.content.nodes);
    }
}

fn migrate_source_appearance(project: &mut Project) -> Result<(), String> {
    fn visit(nodes: &mut [Node], marked: &std::collections::HashSet<Id>) -> Result<(), String> {
        for node in nodes {
            match &mut node.content {
                NodeContent::Group { children } => visit(children, marked)?,
                NodeContent::Instance {
                    definition,
                    overrides,
                } => {
                    if overrides
                        .iter()
                        .any(|o| o.path.is_empty() || o.patch.markup.is_some())
                    {
                        return Err("Root appearance overrides require document version 7".into());
                    }
                    // Before v7 a source placement's markup applied only to its library view.
                    if marked.contains(definition) && !node.markup {
                        overrides.push(Override {
                            path: vec![],
                            patch: Patch {
                                markup: Some(false),
                                ..Patch::default()
                            },
                        });
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    let marked = project
        .definitions
        .iter()
        .filter(|d| project.source_appearance(d.id).1)
        .map(|d| d.id)
        .collect();
    for page in &mut project.pages {
        visit(&mut page.content.nodes, &marked)?;
        for alternate in &mut page.alternates {
            visit(&mut alternate.content.nodes, &marked)?;
        }
    }
    for definition in &mut project.definitions {
        visit(&mut definition.content.nodes, &marked)?;
    }
    for library in &mut project.libraries {
        for node in &mut library.content.nodes {
            if library.sources.iter().any(|s| s.node == node.id) {
                if node.crop.is_some() {
                    return Err("Shared source crop requires document version 7".into());
                }
            } else {
                visit(std::slice::from_mut(node), &marked)?;
            }
        }
    }
    Ok(())
}
impl TryFrom<legacy::Document> for Project {
    type Error = String;
    fn try_from(old: legacy::Document) -> Result<Self, String> {
        old.validate()?;
        let mut pages = vec![];
        for page in old.pages {
            let mut bounds: HashMap<Id, legacy::Bounds> = HashMap::new();
            for c in &page.controls {
                if let Some(g) = c.group {
                    bounds
                        .entry(g)
                        .and_modify(|b| *b = b.union(c.bounds))
                        .or_insert(c.bounds);
                }
            }
            let mut nodes = vec![];
            let mut group_indices = HashMap::new();
            let mut paint_order = vec![];
            for c in page.controls {
                let group = c.group;
                let mut node = migrate_control(c);
                if let Some(g) = group {
                    let b = bounds[&g];
                    let i = *group_indices.entry(g).or_insert_with(|| {
                        let i = nodes.len();
                        nodes.push(Node::group(g, Point::new(b.x, b.y), vec![]));
                        i
                    });
                    node.origin.x -= b.x;
                    node.origin.y -= b.y;
                    paint_order.push(vec![g, node.id]);
                    if let NodeContent::Group { children } = &mut nodes[i].content {
                        children.push(node);
                    }
                } else {
                    paint_order.push(vec![node.id]);
                    nodes.push(node);
                }
            }
            // Store an override only when recursive ordering cannot preserve old layering.
            let mut natural = vec![];
            for node in &nodes {
                match &node.content {
                    NodeContent::Group { children } => {
                        natural.extend(children.iter().map(|c| vec![node.id, c.id]))
                    }
                    _ => natural.push(vec![node.id]),
                }
            }
            if natural == paint_order {
                paint_order.clear();
            }
            pages.push(Page {
                id: page.id,
                trashed: false,
                name: page.name,
                notes: page.notes,
                parent: None,
                content: Content { nodes, paint_order },
                alternates: vec![],
            });
        }
        let project = Self {
            reviews: BTreeMap::new(),
            asset_imports: Default::default(),
            page_imports: Default::default(),
            removed_pages: Default::default(),
            removed_assets: Default::default(),
            libraries: vec![],
            default_library: None,
            format_version: FORMAT_VERSION,
            versions: vec![],
            title: old.title,
            next_id: old.next_id,
            settings: Settings {
                palette: Palette::default(),
                sketch: old.sketch,
                font_size: 15.,
            },
            pages,
            definitions: vec![],
            assets: vec![],
            font: None,
        };
        project.validate()?;
        Ok(project)
    }
}
fn migrate_control(c: legacy::Control) -> Node {
    // Stable product identifiers; display names remain a catalog concern.
    let kind = c.kind.catalog_id();
    let mut node = Node::control(
        c.id,
        kind,
        Point::new(c.bounds.x, c.bounds.y),
        Size::fixed(c.bounds.w, c.bounds.h),
    );
    node.locked = c.locked;
    node.properties = BTreeMap::from([
        ("text".into(), Value::Text(c.text)),
        ("font_size".into(), Value::Number(c.font_size as f64)),
        ("bold".into(), Value::Bool(c.bold)),
        (
            "fill".into(),
            Value::Color([c.fill[0], c.fill[1], c.fill[2], 255]),
        ),
        ("checked".into(), Value::Bool(c.checked)),
    ]);
    if let Some(page) = c.link {
        node.properties
            .insert("link".into(), Value::Link(Link::Page(page)));
    }
    node
}
