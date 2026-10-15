// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Hierarchical page diagrams, independent of widgets and rendering.
use crate::{
    document::{Link, Project},
    layout::{TextMeasurer, TextStyle},
    model::Bounds,
    rich_text::RichText,
};
use serde::Serialize;
pub const DEFAULT: &str = "Start\n- Explore\n-- Overview\n-- Details\n- Contact";
pub const ERROR: &str = "Use one starting point.\nIndent child pages with a dash.";
#[derive(Debug, Clone)]
pub struct Node {
    pub line: String,
    pub label: RichText,
    pub depth: usize,
    pub children: Vec<usize>,
}
#[derive(Debug, Clone)]
pub struct Tree {
    pub nodes: Vec<Node>,
    pub invalid: bool,
}
fn depth(s: &str) -> usize {
    s.chars().take_while(|c| matches!(c, '-' | '\t')).count()
}
pub fn normalize(source: &str) -> Vec<String> {
    let mut next = 0;
    let mut previous = None;
    let mut clipped = false;
    source
        .trim()
        .split('\n')
        .map(|line| {
            let d = depth(line);
            let remove = if clipped && previous == Some(d) {
                d.saturating_sub(next - 1)
            } else if d > next {
                let r = d - next;
                next += 1;
                clipped = true;
                r
            } else {
                next = d + 1;
                clipped = false;
                0
            };
            previous = Some(d);
            line[remove..].to_owned()
        })
        .collect()
}
impl Tree {
    pub fn parse(source: &str) -> Self {
        let mut nodes: Vec<Node> = vec![];
        let mut last: Vec<usize> = vec![];
        let mut roots = 0;
        for line in normalize(source) {
            let d = depth(&line);
            let index = nodes.len();
            if d == 0 {
                roots += 1;
            } else if let Some(&parent) = last.get(d - 1) {
                nodes[parent].children.push(index);
            }
            if last.len() <= d {
                last.resize(d + 1, 0);
            }
            last[d] = index;
            nodes.push(Node {
                label: RichText::parse(line[d..].trim(), false),
                line,
                depth: d,
                children: vec![],
            });
        }
        Self {
            nodes,
            invalid: roots != 1,
        }
    }
    pub fn layout(&self, fonts: &dyn TextMeasurer, style: TextStyle, horizontal: bool) -> Layout {
        if self.invalid {
            let label = RichText::parse(ERROR, false);
            let m = fonts.measure_rich(&label, style, None, true);
            let b = Bounds::new(0., 0., (m.width + 20.).max(100.), (m.height + 20.).max(70.));
            return Layout {
                width: b.w,
                height: b.h,
                nodes: vec![b],
                links: vec![],
                edges: vec![],
                error: true,
            };
        }
        let sizes: Vec<_> = self
            .nodes
            .iter()
            .map(|n| {
                let m = fonts.measure_rich(&n.label, style, None, true);
                (
                    (m.width + 20.).max(100.),
                    (m.height + 20.).max(if horizontal { 35. } else { 70. }),
                )
            })
            .collect();
        let levels = self.nodes.iter().map(|n| n.depth).max().unwrap_or(0) + 1;
        let mut level_size = vec![0_f32; levels];
        let mut level_height = vec![0_f32; levels];
        for (n, &(w, h)) in self.nodes.iter().zip(&sizes) {
            level_size[n.depth] = level_size[n.depth].max(if horizontal { w } else { h });
            level_height[n.depth] = level_height[n.depth].max(h);
        }
        let gap = if horizontal { 18.5 } else { 20. };
        let step = if horizontal { 70. } else { 50. };
        let mut spans = vec![0.; self.nodes.len()];
        for i in (0..self.nodes.len()).rev() {
            let children = &self.nodes[i].children;
            let needed = children.iter().map(|&c| spans[c]).sum::<f32>()
                + children.len().saturating_sub(1) as f32 * gap;
            spans[i] = needed.max(if horizontal { sizes[i].1 } else { sizes[i].0 });
        }
        let mut offsets = vec![0.; levels];
        for i in 1..levels {
            offsets[i] = offsets[i - 1] + level_size[i - 1] + step;
        }
        let extent = level_size.iter().sum::<f32>() + levels.saturating_sub(1) as f32 * step;
        let mut nodes = vec![Bounds::new(0., 0., 1., 1.); self.nodes.len()];
        let mut starts = vec![0.; self.nodes.len()];
        for i in 0..self.nodes.len() {
            let n = &self.nodes[i];
            let (w, h) = sizes[i];
            let along = starts[i] + (spans[i] - if horizontal { h } else { w }) / 2.;
            nodes[i] = if horizontal {
                Bounds::new(offsets[n.depth], along, w, h)
            } else {
                Bounds::new(along, offsets[n.depth], w, h)
            };
            let occupied = n.children.iter().map(|&c| spans[c]).sum::<f32>()
                + n.children.len().saturating_sub(1) as f32 * gap;
            let mut cursor = starts[i] + (spans[i] - occupied).max(0.) / 2.;
            for &child in &n.children {
                starts[child] = cursor;
                cursor += spans[child] + gap;
            }
        }
        let mut edges = vec![];
        for (i, n) in self.nodes.iter().enumerate() {
            if n.children.is_empty() {
                continue;
            }
            let b = nodes[i];
            if horizontal {
                let y = b.y + b.h / 2.;
                let trunk = b.x + b.w + 25.;
                edges.push([[b.x + b.w, y], [trunk, y]]);
                let first = nodes[n.children[0]];
                let last = nodes[*n.children.last().unwrap()];
                edges.push([
                    [trunk, first.y + first.h / 2.],
                    [trunk, last.y + last.h / 2.],
                ]);
                for &child in &n.children {
                    let c = nodes[child];
                    edges.push([[c.x, c.y + c.h / 2.], [trunk, c.y + c.h / 2.]]);
                }
            } else {
                let x = b.x + b.w / 2.;
                let trunk = b.y + b.h + 25.;
                edges.push([[x, b.y + b.h], [x, trunk]]);
                let first = nodes[n.children[0]];
                let last = nodes[*n.children.last().unwrap()];
                edges.push([
                    [first.x + first.w / 2., trunk],
                    [last.x + last.w / 2., trunk],
                ]);
                for &child in &n.children {
                    let c = nodes[child];
                    edges.push([[c.x + c.w / 2., c.y], [c.x + c.w / 2., trunk]]);
                }
            }
        }
        let links = nodes
            .iter()
            .zip(&self.nodes)
            .map(|(b, n)| Bounds::new(b.x, b.y, b.w, level_height[n.depth]))
            .collect();
        Layout {
            width: if horizontal { extent } else { spans[0] },
            height: if horizontal { spans[0] } else { extent },
            nodes,
            links,
            edges,
            error: false,
        }
    }
}
#[derive(Debug, Serialize)]
pub struct Layout {
    pub width: f32,
    pub height: f32,
    pub nodes: Vec<Bounds>,
    pub links: Vec<Bounds>,
    pub edges: Vec<[[f32; 2]; 2]>,
    pub error: bool,
}
pub fn labels(source: &str) -> Vec<String> {
    let t = Tree::parse(source);
    if t.invalid {
        vec![]
    } else {
        t.nodes.into_iter().map(|n| n.line).collect()
    }
}
pub fn from_project(project: &Project) -> (String, Vec<Option<Link>>) {
    let mut text = project.title.clone();
    let mut links = vec![None];
    let mut pending: Vec<_> = project
        .pages
        .iter()
        .filter(|p| p.parent.is_none())
        .rev()
        .map(|p| (p, 1usize))
        .collect();
    while let Some((page, level)) = pending.pop() {
        text.push('\n');
        text.push_str(&"-".repeat(level));
        text.push(' ');
        text.push_str(&page.name);
        links.push(Some(Link::Page(page.id)));
        pending.extend(
            project
                .pages
                .iter()
                .filter(|p| p.parent == Some(page.id))
                .rev()
                .map(|p| (p, level + 1)),
        );
    }
    (text, links)
}

/// Preserve per-node destinations when labels are reordered or inserted.
pub fn apply_text(
    node: &mut crate::document::Node,
    source: &str,
    baseline: &crate::document::Properties,
) -> Result<(), String> {
    use crate::document::Value;
    let old = match baseline.get("text") {
        Some(Value::Text(v)) => v.as_str(),
        _ => "",
    };
    let old = labels(old);
    let new = labels(source);
    if !old.is_empty()
        && !new.is_empty()
        && let Some(Value::Links(links)) = baseline.get("row_links")
    {
        let old: Vec<_> = old.iter().map(String::as_str).collect();
        let new: Vec<_> = new.iter().map(String::as_str).collect();
        node.properties.insert(
            "row_links".into(),
            Value::Links(crate::list::remap_labels(&old, &new, links)),
        );
    }
    node.properties
        .insert("text".into(), Value::Text(source.into()));
    Ok(())
}
