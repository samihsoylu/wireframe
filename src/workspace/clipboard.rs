// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Clipboard {
    pub nodes: Vec<Node>,
    pub definitions: Vec<Definition>,
    pub assets: Vec<Asset>,
}
impl Clipboard {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}
impl Editor {
    pub fn copy(&self) -> Clipboard {
        let mut copy = Clipboard {
            nodes: self
                .nodes()
                .iter()
                .filter(|n| self.selection.contains(&n.id))
                .cloned()
                .collect(),
            ..Clipboard::default()
        };
        let parent = self.context_origin();
        for n in &mut copy.nodes {
            n.clear_empty_component_markers();
            n.origin = n.origin.translated(parent);
        }
        let mut defs = HashSet::new();
        let mut assets = HashSet::new();
        references(&copy.nodes, &mut defs, &mut assets);
        loop {
            let add: Vec<_> = self
                .document
                .definitions
                .iter()
                .filter(|d| {
                    defs.contains(&d.id) && !copy.definitions.iter().any(|old| old.id == d.id)
                })
                .cloned()
                .collect();
            if add.is_empty() {
                break;
            }
            for d in add {
                references(&d.content.nodes, &mut defs, &mut assets);
                copy.definitions.push(d);
            }
        }
        copy.assets = self
            .document
            .assets
            .iter()
            .filter(|a| assets.contains(&a.id))
            .cloned()
            .collect();
        copy
    }
    pub fn paste(&mut self, copy: &Clipboard, offset: f32) {
        if !self.structure_edit_allowed() {
            return;
        }
        if copy.nodes.is_empty() {
            return;
        }
        self.edit("Paste controls", |e| {
            let mut copy = copy.clone();
            let mut refs = HashSet::new();
            let mut assets = HashSet::new();
            references(&copy.nodes, &mut refs, &mut assets);
            for d in &copy.definitions {
                references(&d.content.nodes, &mut refs, &mut assets);
            }
            let mut missing: Vec<_> = refs
                .into_iter()
                .filter(|id| !copy.definitions.iter().any(|d| d.id == *id))
                .collect();
            missing.sort_unstable();
            let reuse = missing.is_empty()
                && copy
                    .definitions
                    .iter()
                    .all(|d| e.document.definitions.contains(d))
                && copy.assets.iter().all(|a| e.document.assets.contains(a));
            if reuse {
                copy.definitions.clear();
                copy.assets.clear();
            }
            let mut map = HashMap::new();
            for id in copy
                .definitions
                .iter()
                .map(|d| d.id)
                .chain(copy.assets.iter().map(|a| a.id))
                .chain(missing)
            {
                let Ok(next) = e.document.allocate_id() else {
                    return;
                };
                map.insert(id, next);
            }
            for d in &mut copy.definitions {
                d.id = map[&d.id];
                remap(&mut d.content.nodes, &map);
            }
            for a in &mut copy.assets {
                a.id = map[&a.id];
            }
            remap(&mut copy.nodes, &map);
            let parent = e.context_origin();
            let mut selected = HashSet::new();
            for n in &mut copy.nodes {
                let Ok(id) = e.document.allocate_id() else {
                    return;
                };
                n.id = id;
                n.locked = false;
                n.origin.x += offset - parent.x;
                n.origin.y += offset - parent.y;
                selected.insert(id);
            }
            // Scoped descendant IDs remain stable; advance the allocator above all imported IDs.
            fn maximum(nodes: &[Node]) -> Id {
                nodes
                    .iter()
                    .map(|n| match &n.content {
                        NodeContent::Group { children } => n.id.max(maximum(children)),
                        _ => n.id,
                    })
                    .max()
                    .unwrap_or(0)
            }
            let max = maximum(&copy.nodes).max(
                copy.definitions
                    .iter()
                    .map(|d| maximum(&d.content.nodes))
                    .max()
                    .unwrap_or(0),
            );
            if max >= e.document.next_id {
                let Some(next) = max.checked_add(1) else {
                    return;
                };
                e.document.next_id = next;
            }
            let pages: HashSet<_> = e.document.pages.iter().map(|p| p.id).collect();
            strip_missing_links(&mut copy.nodes, &pages);
            for d in &mut copy.definitions {
                strip_missing_links(&mut d.content.nodes, &pages);
            }
            let had_order = !e.active_content().paint_order.is_empty();
            e.nodes_mut().extend(copy.nodes);
            e.document.definitions.extend(copy.definitions);
            e.document.assets.extend(copy.assets);
            e.selection = selected;
            if had_order {
                // New content must extend the stored leaf order, including group paths.
                let context = e.context.clone();
                let order = Content {
                    nodes: e.nodes().to_vec(),
                    paint_order: vec![],
                }
                .paint_paths()
                .map(|leaves| {
                    leaves
                        .into_iter()
                        .filter(|l| e.selection.contains(&l[0]))
                        .map(|l| {
                            let mut p = context.clone();
                            p.extend(l);
                            p
                        })
                        .collect::<Vec<_>>()
                });
                match order {
                    Ok(paths) => e.active_content_mut().paint_order.extend(paths),
                    Err(err) => e.error = Some(err),
                }
            }
        });
    }
    pub fn paste_legacy(&mut self, controls: &[Control], offset: f32) {
        let max = controls
            .iter()
            .flat_map(|c| [c.id, c.group.unwrap_or(0)])
            .max()
            .unwrap_or(1);
        let mut doc = model::Document {
            next_id: max.saturating_add(1),
            ..model::Document::default()
        };
        doc.pages[0].controls = controls.to_vec();
        for c in &mut doc.pages[0].controls {
            c.link = None;
        }
        match Project::try_from(doc) {
            Ok(p) => self.paste(
                &Clipboard {
                    nodes: p.pages[0].content.nodes.clone(),
                    ..Clipboard::default()
                },
                offset,
            ),
            Err(err) => self.error = Some(err),
        }
    }
    pub fn duplicate(&mut self) {
        self.paste(&self.copy(), 20.);
    }
}
fn references(nodes: &[Node], defs: &mut HashSet<Id>, assets: &mut HashSet<Id>) {
    fn props(p: &Properties, assets: &mut HashSet<Id>) {
        for v in p.values() {
            match v {
                Value::Asset(id) | Value::Icon(IconSource::Asset(id)) => {
                    assets.insert(*id);
                }
                Value::Icons(icons) => {
                    for icon in icons.iter().flatten() {
                        if let IconSource::Asset(id) = icon {
                            assets.insert(*id);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    for n in nodes {
        props(&n.properties, assets);
        match &n.content {
            NodeContent::Group { children } => references(children, defs, assets),
            NodeContent::Instance {
                definition,
                overrides,
            } => {
                defs.insert(*definition);
                for o in overrides {
                    props(&o.patch.properties, assets);
                }
            }
            _ => {}
        }
    }
}
fn remap(nodes: &mut [Node], map: &HashMap<Id, Id>) {
    fn props(p: &mut Properties, map: &HashMap<Id, Id>) {
        for v in p.values_mut() {
            match v {
                Value::Asset(id) | Value::Icon(IconSource::Asset(id)) => {
                    if let Some(new) = map.get(id) {
                        *id = *new;
                    }
                }
                Value::Icons(icons) => {
                    for icon in icons.iter_mut().flatten() {
                        if let IconSource::Asset(id) = icon
                            && let Some(new) = map.get(id)
                        {
                            *id = *new;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    for n in nodes {
        props(&mut n.properties, map);
        match &mut n.content {
            NodeContent::Group { children } => remap(children, map),
            NodeContent::Instance {
                definition,
                overrides,
            } => {
                if let Some(new) = map.get(definition) {
                    *definition = *new;
                }
                for o in overrides {
                    props(&mut o.patch.properties, map);
                }
            }
            _ => {}
        }
    }
}
fn strip_missing_links(nodes: &mut [Node], pages: &HashSet<Id>) {
    fn props(p: &mut Properties, pages: &HashSet<Id>) {
        for v in p.values_mut() {
            if let Value::Links(links) = v {
                for target in links {
                    if let Some(Link::Page(id)) = target
                        && !pages.contains(id)
                    {
                        *target = None;
                    }
                }
            }
            if let Value::Link(Link::Page(id)) = v
                && !pages.contains(id)
            {
                *v = Value::Null;
            }
        }
    }
    for n in nodes {
        props(&mut n.properties, pages);
        match &mut n.content {
            NodeContent::Group { children } => strip_missing_links(children, pages),
            NodeContent::Instance { overrides, .. } => {
                for o in overrides {
                    props(&mut o.patch.properties, pages);
                }
            }
            _ => {}
        }
    }
}
