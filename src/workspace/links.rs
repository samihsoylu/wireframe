// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LinkTarget {
    pub node: Id,
    pub row: Option<usize>,
    pub text: Option<usize>,
}
impl LinkTarget {
    pub fn whole(node: Id) -> Self {
        Self {
            node,
            row: None,
            text: None,
        }
    }
    pub fn text(node: Id, index: usize) -> Self {
        Self {
            node,
            row: None,
            text: Some(index),
        }
    }
    pub fn row(node: Id, row: usize) -> Self {
        Self {
            node,
            row: Some(row),
            text: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum LinkAction {
    Set(Option<Link>),
    NewPage,
    DuplicatePage,
}

impl Editor {
    /// Labels use the same row indexing as saved links, including list headers.
    pub fn link_row_labels(&self, id: Id) -> Option<Vec<String>> {
        let (spec, p) = self.control_properties(id)?;
        if !spec.property("row_links").is_some_and(|f| f.visible) {
            return None;
        }
        let source = match p.get("text") {
            Some(Value::Text(s)) => s.as_str(),
            _ => "",
        };
        Some(match spec.id {
            "date_picker" | "time_picker" => crate::pickers::BUTTONS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            "app_bar" => crate::app_bar::labels(&p),
            "alert" => crate::alert::labels(source),
            "accordion" => crate::accordion::labels(source),
            "site_map" => crate::site_map::labels(source),
            "tree" => crate::tree::labels(source),
            "menu" | "menu_bar" => crate::menu::labels(source, spec.id == "menu_bar"),
            "tabs" | "vertical_tabs" => crate::tabs::labels(source, spec.id == "vertical_tabs"),
            "breadcrumbs" | "link_bar" => {
                crate::navigation::labels(source, spec.id == "breadcrumbs")
            }
            "button_bar" => crate::button_bar::labels(source)
                .into_iter()
                .map(str::to_owned)
                .collect(),
            _ => source.split('\n').map(str::to_owned).collect(),
        })
    }
    pub fn link_target(&self, target: LinkTarget) -> Option<Link> {
        if let Some(index) = target.text {
            return self
                .text_links(target.node)
                .ok()??
                .get(index)?
                .target
                .clone();
        }
        let Some(row) = target.row else {
            return self.node_link(target.node);
        };
        let (_, p) = self.control_properties(target.node)?;
        match p.get("row_links") {
            Some(Value::Links(links)) => links.get(row).cloned().flatten(),
            _ => None,
        }
    }
    /// One transaction creates a destination and links every target without navigating.
    /// Existing field edits are committed separately; any failure rolls this action back.
    pub fn edit_links(
        &mut self,
        targets: &[LinkTarget],
        action: LinkAction,
    ) -> Result<Option<Id>, String> {
        self.edit_links_from(targets, action, targets.first().copied())
    }
    /// Common row inspectors include rows that only some selected controls own.
    /// Missing rows stay untouched; destination naming still follows the first control.
    pub fn edit_row_links(
        &mut self,
        nodes: &[Id],
        row: usize,
        action: LinkAction,
    ) -> Result<Option<Id>, String> {
        let targets = self.common_row_targets(nodes, row)?;
        let name_source = nodes.first().map(|&node| {
            targets
                .iter()
                .find(|t| t.node == node)
                .copied()
                .unwrap_or(LinkTarget::row(node, row))
        });
        self.edit_links_from(&targets, action, name_source)
    }
    pub fn common_row_targets(&self, nodes: &[Id], row: usize) -> Result<Vec<LinkTarget>, String> {
        let app_bars = nodes
            .iter()
            .filter(|&&id| {
                self.control_properties(id)
                    .is_some_and(|(spec, _)| spec.id == "app_bar")
            })
            .count();
        if app_bars > 0 && app_bars != nodes.len() {
            return Err("Select matching controls to edit icon links".into());
        }
        let labels: Vec<_> = nodes
            .iter()
            .map(|&node| {
                self.link_row_labels(node)
                    .ok_or("A row-link control no longer exists")
            })
            .collect::<Result<_, _>>()?;
        let count = labels.iter().map(Vec::len).max().unwrap_or(0);
        if row >= count {
            return Ok(vec![]);
        }
        let mut targets = vec![];
        for (&node, labels) in nodes.iter().zip(labels) {
            let index = if app_bars > 0 && row + 1 == count {
                labels.len() - 1
            } else {
                row
            };
            if index < labels.len()
                && (app_bars == 0 || row + 1 == count || index + 1 < labels.len())
            {
                targets.push(LinkTarget::row(node, index));
            }
        }
        Ok(targets)
    }
    pub(super) fn edit_links_from(
        &mut self,
        targets: &[LinkTarget],
        action: LinkAction,
        name_source: Option<LinkTarget>,
    ) -> Result<Option<Id>, String> {
        if targets.is_empty() {
            return Err("Select a control to link".into());
        }
        for target in targets {
            if target.text.is_some() && target.row.is_some() {
                return Err("Choose one kind of link target".into());
            }
            if let Some(index) = target.text
                && !self
                    .text_links(target.node)?
                    .is_some_and(|links| index < links.len())
            {
                return Err("The text link no longer exists".into());
            }
            if !self.nodes().iter().any(|n| n.id == target.node) {
                return Err("A link control no longer exists".into());
            }
            if let Some(row) = target.row
                && !self
                    .link_row_labels(target.node)
                    .is_some_and(|labels| row < labels.len())
            {
                return Err("A link row no longer exists".into());
            }
        }
        if let LinkAction::Set(Some(link)) = &action {
            if !link.valid() {
                return Err("Enter a valid web address or choose a page".into());
            }
            if let Link::Page(page) = link
                && self.document.live_page(*page).is_none()
            {
                return Err("The destination page is no longer available".into());
            }
        }
        self.commit();
        self.begin(match action {
            LinkAction::Set(_) => "Edit links",
            LinkAction::NewPage => "Link to new wireframe",
            LinkAction::DuplicatePage => "Link to duplicate wireframe",
        });
        let result = (|| {
            let (link, created) = match action {
                LinkAction::Set(link) => (link, None),
                LinkAction::NewPage | LinkAction::DuplicatePage => {
                    let id = self.create_link_destination(
                        name_source.unwrap_or(targets[0]),
                        matches!(action, LinkAction::DuplicatePage),
                    )?;
                    (Some(Link::Page(id)), Some(id))
                }
            };
            let mut seen = HashSet::new();
            for &target in targets {
                if !seen.insert(target) {
                    continue;
                }
                if let Some(index) = target.text {
                    self.set_text_link(target.node, index, link.as_ref())?;
                } else if let Some(row) = target.row {
                    let count = self
                        .link_row_labels(target.node)
                        .ok_or("Rows no longer available")?
                        .len();
                    let (_, p) = self
                        .control_properties(target.node)
                        .ok_or("Control no longer exists")?;
                    let mut links = match p.get("row_links") {
                        Some(Value::Links(v)) => v.clone(),
                        _ => vec![],
                    };
                    links.resize(count, None);
                    links[row] = link.clone();
                    self.set_control_property(target.node, "row_links", Value::Links(links))?;
                } else {
                    self.set_node_link(target.node, link.clone())?;
                }
            }
            self.document.encode()?;
            Ok(created)
        })();
        match result {
            Ok(created) => {
                self.commit();
                Ok(created)
            }
            Err(error) => {
                self.cancel();
                Err(error)
            }
        }
    }
    fn create_link_destination(
        &mut self,
        target: LinkTarget,
        duplicate: bool,
    ) -> Result<Id, String> {
        let mut name = target
            .text
            .and_then(|index| {
                let links = self.text_links(target.node).ok()??;
                let source = links.get(index)?.source.label.clone();
                let (spec, properties) = self.control_properties(target.node)?;
                let Value::Text(text) = properties.get("text")? else {
                    return None;
                };
                let label = text.get(source)?;
                // Page names retain the link's formatting markers. A table's
                // escaped comma belongs to cell syntax, not to its label.
                Some(if spec.id == "table" {
                    label.replace("\\,", ",")
                } else {
                    label.to_owned()
                })
            })
            .or_else(|| {
                target
                    .row
                    .and_then(|row| self.link_row_labels(target.node)?.get(row).cloned())
            })
            .unwrap_or_default();
        if name.is_empty() {
            name = self
                .control_properties(target.node)
                .and_then(|(_, p)| match p.get("text") {
                    Some(Value::Text(s)) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default();
        }
        let name = if name.is_empty() {
            format!("Wireframe {}", self.document.live_pages().count() + 1)
        } else {
            name
        };
        let tree = crate::page_tree::PageTree::new(&self.document);
        let parent = if self.in_source_workspace() {
            None
        } else {
            tree.row(self.page).and_then(|row| row.parent)
        };
        let insertion = if let Some(parent) = parent {
            tree.subtree(parent)
                .iter()
                .map(|r| r.position)
                .max()
                .ok_or("Page parent no longer exists")?
                + 1
        } else {
            self.document.pages.len()
        };
        let content = if duplicate {
            // A source editor is a view into its owning library; duplicate that
            // complete surface, just as a nested group duplicates its whole page.
            self.definition
                .and_then(|id| self.document.component_source(id))
                .map_or_else(
                    || self.active_content().clone(),
                    |(library, _)| library.content.clone(),
                )
        } else {
            Content::default()
        };
        let id = self.document.allocate_id()?;
        self.document.pages.insert(
            insertion,
            Page {
                id,
                trashed: false,
                name,
                notes: String::new(),
                parent,
                content,
                alternates: vec![],
            },
        );
        Ok(id)
    }
}
