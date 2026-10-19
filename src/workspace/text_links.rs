// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::rich_text::{self, SourceLink};

#[derive(Clone, Debug)]
pub struct TextLink {
    pub label: String,
    pub target: Option<Link>,
    pub source: SourceLink,
}
fn supports(kind: &str) -> bool {
    matches!(
        kind,
        "arrow"
            | "multiline_button"
            | "callout"
            | "checkbox"
            | "radio"
            | "horizontal_brace"
            | "vertical_brace"
            | "table"
            | "icon_label"
            | "label"
            | "link"
            | "paragraph"
            | "sticky_note"
            | "tag_cloud"
            | "text_area"
            | "tooltip"
    )
}
fn sources(kind: &str, text: &str) -> Result<Vec<SourceLink>, String> {
    let mut links = if kind == "table" {
        rich_text::structured::Grid::source_links(text)?
    } else {
        rich_text::source_links(text)?
    };
    let mut end = 0;
    links.retain(|link| {
        if link.range.start < end {
            return false;
        }
        end = link.range.end;
        true
    });
    Ok(links)
}
impl Editor {
    /// Display live page names while keeping stable destinations in the document.
    /// Missing destinations stay explicit so editing cannot silently repair them.
    pub fn control_text_for_edit(&self, id: Id, text: &str) -> Result<String, String> {
        let Some((spec, _)) = self.control_properties(id).filter(|(s, _)| supports(s.id)) else {
            return Ok(text.to_owned());
        };
        let mut result = text.to_owned();
        for link in sources(spec.id, text)?.into_iter().rev() {
            if let Some(page) = link
                .target
                .as_deref()
                .and_then(|s| s.strip_prefix("page:"))
                .and_then(|s| s.parse::<Id>().ok())
                .and_then(|id| self.document.live_page(id))
            {
                result = rich_text::replace_source_link_target(&result, &link, Some(&page.name))?;
            }
        }
        Ok(result)
    }

    /// Resolve editable names without retargeting unchanged links. In particular,
    /// two pages with the same name retain their distinct identities when labels
    /// are moved or surrounding prose changes. New names use document page order.
    pub fn control_text_from_edit(&self, id: Id, text: &str) -> Result<String, String> {
        let Some((spec, properties)) = self.control_properties(id).filter(|(s, _)| supports(s.id))
        else {
            return Ok(text.to_owned());
        };
        let Some(Value::Text(saved)) = properties.get("text") else {
            return Ok(text.to_owned());
        };
        let displayed = self.control_text_for_edit(id, saved)?;
        if text == displayed {
            return Ok(saved.clone());
        }
        let original = sources(spec.id, saved)?;
        let shown = sources(spec.id, &displayed)?;
        let edited = sources(spec.id, text)?;
        // Reserve unchanged positions before matching moved labels. Otherwise a
        // renamed label could steal another unchanged link's duplicate-name target.
        let mut used = vec![false; shown.len()];
        let mut matches = vec![None; edited.len()];
        for (i, link) in edited.iter().enumerate() {
            if let Some(old) = shown.get(i)
                && old.target == link.target
                && displayed[old.label.clone()] == text[link.label.clone()]
            {
                matches[i] = Some(i);
                used[i] = true;
            }
        }
        let mut by_label =
            std::collections::BTreeMap::<_, std::collections::VecDeque<usize>>::new();
        for (i, link) in shown.iter().enumerate().filter(|(i, _)| !used[*i]) {
            by_label
                .entry((link.target.as_deref(), &displayed[link.label.clone()]))
                .or_default()
                .push_back(i);
        }
        for (i, link) in edited.iter().enumerate() {
            if matches[i].is_some() {
                continue;
            }
            if let Some(old) = by_label
                .get_mut(&(link.target.as_deref(), &text[link.label.clone()]))
                .and_then(|v| v.pop_front())
            {
                matches[i] = Some(old);
                used[old] = true;
            }
        }
        for (i, link) in edited.iter().enumerate() {
            if matches[i].is_none()
                && let Some(old) = shown.get(i)
                && !used[i]
                && old.target == link.target
            {
                matches[i] = Some(i);
                used[i] = true;
            }
        }
        let mut targets = vec![];
        for (index, link) in edited.iter().enumerate() {
            let Some(target) = link.target.as_deref() else {
                targets.push(None);
                continue;
            };
            let matching = matches[index];
            let resolved = if let Some(i) = matching {
                original.get(i).and_then(|old| old.target.clone())
            } else if target == "back:"
                || target
                    .strip_prefix("page:")
                    .is_some_and(|s| s.parse::<Id>().is_ok())
            {
                Some(target.to_owned())
            } else {
                Some(
                    self.document
                        .live_pages()
                        .find(|page| page.name == target)
                        .map_or_else(|| target.to_owned(), |page| format!("page:{}", page.id)),
                )
            };
            targets.push(resolved);
        }
        let mut result = text.to_owned();
        for (link, target) in edited.iter().zip(targets).rev() {
            if target != link.target {
                result = rich_text::replace_source_link_target(&result, link, target.as_deref())?;
            }
        }
        Ok(result)
    }

    pub fn text_links(&self, id: Id) -> Result<Option<Vec<TextLink>>, String> {
        let Some((spec, p)) = self
            .control_properties(id)
            .filter(|(spec, _)| supports(spec.id))
        else {
            return Ok(None);
        };
        let Some(Value::Text(text)) = p.get("text") else {
            return Ok(None);
        };
        let mut links = vec![];
        for source in sources(spec.id, text)? {
            let label =
                rich_text::RichText::parse(&text[source.label.clone()], false).display_text(false);
            let target = source
                .target
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(|target| {
                    if target == "back:" {
                        Link::Back
                    } else if let Some(id) = target
                        .strip_prefix("page:")
                        .and_then(|s| s.parse::<Id>().ok())
                    {
                        Link::Page(id)
                    } else {
                        Link::Url(target.to_owned())
                    }
                });
            links.push(TextLink {
                label,
                target,
                source,
            });
        }
        Ok(Some(links))
    }
    pub(super) fn set_text_link(
        &mut self,
        id: Id,
        index: usize,
        target: Option<&Link>,
    ) -> Result<(), String> {
        let links = self
            .text_links(id)?
            .ok_or("This control does not have text links")?;
        let link = links.get(index).ok_or("The text link no longer exists")?;
        let (_, p) = self
            .control_properties(id)
            .ok_or("Control no longer exists")?;
        let Some(Value::Text(text)) = p.get("text") else {
            return Err("Control text no longer exists".into());
        };
        let target = target.map(Link::target);
        let text = rich_text::replace_source_link_target(text, &link.source, target.as_deref())?;
        self.set_control_property(id, "text", Value::Text(text))
    }
    /// A multi-selection exposes the union of inline link positions. Controls
    /// without that position remain unchanged; every selected kind must support it.
    pub fn edit_text_links(
        &mut self,
        nodes: &[Id],
        index: usize,
        action: LinkAction,
    ) -> Result<Option<Id>, String> {
        let mut targets = vec![];
        for &node in nodes {
            let links = self
                .text_links(node)?
                .ok_or("A selected control does not support text links")?;
            if index < links.len() {
                targets.push(LinkTarget::text(node, index));
            }
        }
        self.edit_links_from(
            &targets,
            action,
            nodes.first().map(|&node| LinkTarget::text(node, index)),
        )
    }
}
