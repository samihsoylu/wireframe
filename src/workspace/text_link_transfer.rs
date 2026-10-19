// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Text destinations are rewritten before component path/resource correspondence.
use super::*;
use crate::rich_text::{self, SourceLink};

fn kind_at<'a>(nodes: &'a [Node], path: &[Id], definitions: &'a [Definition]) -> Option<&'a str> {
    let (&first, rest) = path.split_first()?;
    let node = nodes.iter().find(|n| n.id == first)?;
    if rest.is_empty() {
        return match &node.content {
            NodeContent::Control { kind, .. } => Some(kind),
            _ => Some(""),
        };
    }
    match &node.content {
        NodeContent::Group { children } => kind_at(children, rest, definitions),
        NodeContent::Instance { definition, .. } => kind_at(
            &definitions
                .iter()
                .find(|d| d.id == *definition)?
                .content
                .nodes,
            rest,
            definitions,
        ),
        _ => None,
    }
}
fn override_kind<'a>(
    definition: Id,
    path: &[Id],
    definitions: &'a [Definition],
) -> Option<&'a str> {
    if path.is_empty() {
        return Some("");
    }
    kind_at(
        &definitions
            .iter()
            .find(|d| d.id == definition)?
            .content
            .nodes,
        path,
        definitions,
    )
}
fn links(text: &str, kind: Option<&str>) -> Result<Vec<(SourceLink, Id)>, String> {
    if kind == Some("") || !text.contains("page:") {
        return Ok(vec![]);
    }
    let mut sources = if kind == Some("table") {
        rich_text::structured::Grid::source_links(text)?
    } else {
        rich_text::source_links(text)?
    };
    // An orphan override has no recoverable control kind. Reserve/remap all
    // explicit destinations that could belong to either ordinary text or cells.
    if kind.is_none() {
        sources.extend(rich_text::structured::Grid::source_links(text)?);
    }
    sources.sort_by_key(|s| (s.label.end, s.range.end));
    sources.dedup_by_key(|s| (s.label.end, s.range.end));
    Ok(sources
        .into_iter()
        .filter_map(|s| {
            if s.range.end == s.label.end + 1 {
                return None;
            } // Inherited target, no suffix of its own.
            let id = s
                .target
                .as_deref()?
                .strip_prefix("page:")?
                .parse::<Id>()
                .ok()?;
            Some((s, id))
        })
        .collect())
}
fn collect_properties(
    p: &Properties,
    kind: Option<&str>,
    ids: &mut HashSet<Id>,
) -> Result<(), String> {
    if let Some(Value::Text(text)) = p.get("text") {
        ids.extend(links(text, kind)?.into_iter().map(|(_, id)| id));
    }
    Ok(())
}
pub(super) fn collect(
    nodes: &[Node],
    definitions: &[Definition],
    ids: &mut HashSet<Id>,
) -> Result<(), String> {
    for node in nodes {
        match &node.content {
            NodeContent::Control { kind, .. } => {
                collect_properties(&node.properties, Some(kind), ids)?
            }
            NodeContent::Group { children } => collect(children, definitions, ids)?,
            NodeContent::Instance {
                definition,
                overrides,
            } => {
                collect_properties(&node.properties, None, ids)?;
                for item in overrides {
                    collect_properties(
                        &item.patch.properties,
                        override_kind(*definition, &item.path, definitions),
                        ids,
                    )?;
                }
            }
        }
    }
    Ok(())
}
fn rewrite_properties(
    p: &mut Properties,
    kind: Option<&str>,
    replace: &mut impl FnMut(Id) -> Option<Id>,
) -> Result<(), String> {
    let Some(Value::Text(text)) = p.get_mut("text") else {
        return Ok(());
    };
    let links = links(text, kind)?;
    // Suffixes are disjoint even for nested links. Reverse suffix order keeps
    // every original offset valid while labels remain byte-identical.
    for (link, id) in links.into_iter().rev() {
        let new = replace(id);
        if new == Some(id) {
            continue;
        }
        let suffix = new.map_or(String::new(), |id| format!("(page:{id})"));
        text.replace_range(link.label.end + 1..link.range.end, &suffix);
    }
    Ok(())
}
pub(super) fn rewrite(
    nodes: &mut [Node],
    definitions: &[Definition],
    replace: &mut impl FnMut(Id) -> Option<Id>,
) -> Result<(), String> {
    for node in nodes {
        match &mut node.content {
            NodeContent::Control { kind, .. } => {
                rewrite_properties(&mut node.properties, Some(kind), replace)?
            }
            NodeContent::Group { children } => rewrite(children, definitions, replace)?,
            NodeContent::Instance {
                definition,
                overrides,
            } => {
                rewrite_properties(&mut node.properties, None, replace)?;
                for item in overrides {
                    rewrite_properties(
                        &mut item.patch.properties,
                        override_kind(*definition, &item.path, definitions),
                        replace,
                    )?;
                }
            }
        }
    }
    Ok(())
}
pub(super) fn remap(
    nodes: &mut [Node],
    definitions: &[Definition],
    map: &HashMap<Id, Id>,
) -> Result<(), String> {
    rewrite(nodes, definitions, &mut |id| {
        Some(map.get(&id).copied().unwrap_or(id))
    })
}
pub(super) fn retain(
    nodes: &mut [Node],
    definitions: &[Definition],
    pages: &HashSet<Id>,
) -> Result<(), String> {
    rewrite(nodes, definitions, &mut |id| {
        pages.contains(&id).then_some(id)
    })
}
