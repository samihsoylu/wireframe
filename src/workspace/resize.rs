// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::layout::LayoutNode;

impl Editor {
    /// Selection limits include fixed axes of every resolved descendant,
    /// even when locked or hidden by a crop. Each axis has its own limit.
    pub fn resize_minimum(&self, ids: &[Id]) -> (f32, f32) {
        let Ok(layout) = self.layout_page(self.surface_id()) else {
            return (10., 10.);
        };
        ids.iter()
            .filter_map(|id| {
                let mut path = self.context.clone();
                path.push(*id);
                view::find_layout(&layout.nodes, &path).map(minimum)
            })
            .reduce(combine)
            .unwrap_or((10., 10.))
    }
}

fn combine(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    (a.0.max(b.0), a.1.max(b.1))
}

fn minimum(node: &LayoutNode) -> (f32, f32) {
    if let Some(size) = node.children.iter().map(minimum).reduce(combine) {
        return size;
    }
    let fixed = (node.bounds.width, node.bounds.height);
    let Some(spec) = node.kind.as_deref().and_then(catalog::get) else {
        return fixed;
    };
    let limit = spec.minimum_resize_dimension();
    match spec.resize {
        catalog::ResizeAxes::Both => (limit, limit),
        catalog::ResizeAxes::Horizontal => (limit, fixed.1),
        catalog::ResizeAxes::Vertical => (fixed.0, limit),
        catalog::ResizeAxes::Computed => fixed,
    }
}
