// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::layout::LayoutNode;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectionGeometry {
    /// Position is relative to the visible editing context's uncropped origin.
    pub bounds: Bounds,
    pub locked: bool,
    pub width_editable: bool,
    pub height_editable: bool,
}

impl Editor {
    fn geometry_origin(&self) -> Point {
        if self.context.is_empty() {
            return Point::default();
        }
        self.layout_page(self.surface_id())
            .ok()
            .and_then(|layout| {
                view::find_layout(&layout.nodes, &self.context)
                    .map(|node| Point::new(node.uncropped.x, node.uncropped.y))
            })
            .unwrap_or_else(|| self.context_origin())
    }

    pub fn selection_geometry(&self) -> Option<SelectionGeometry> {
        let controls = self.selected_views();
        let locked = controls.iter().all(|c| c.locked);
        let layout = (!self.markup_visible)
            .then(|| self.layout_page(self.surface_id()).ok())
            .flatten();
        let mut bounds = controls
            .iter()
            .filter(|c| locked || !c.locked)
            .map(|c| {
                let mut path = self.context.clone();
                path.push(c.id);
                layout
                    .as_ref()
                    .and_then(|layout| view::find_layout(&layout.nodes, &path))
                    .map_or(c.bounds, |node| {
                        let b = node.without_markup;
                        Bounds::new(b.x, b.y, b.width, b.height)
                    })
            })
            .reduce(Bounds::union)?;
        let origin = self.geometry_origin();
        bounds.x -= origin.x;
        bounds.y -= origin.y;
        let axes: Vec<_> = controls
            .iter()
            .map(|c| {
                if self.empty_container(c.id) || self.missing_component(c.id) {
                    return (false, false);
                }
                if c.group.is_some() {
                    return (c.bounds.w > 0., c.bounds.h > 0.);
                }
                if let Some(axis) = self.linear_axis(c.id) {
                    return (!axis.vertical, axis.vertical);
                }
                let resize = self.control_properties(c.id).map(|(spec, _)| spec.resize);
                use catalog::ResizeAxes::*;
                (
                    c.bounds.w > 0. && matches!(resize, Some(Both | Horizontal)),
                    c.bounds.h > 0. && matches!(resize, Some(Both | Vertical)),
                )
            })
            .collect();
        Some(SelectionGeometry {
            bounds,
            locked,
            width_editable: axes.iter().any(|axes| axes.0),
            height_editable: axes.iter().any(|axes| axes.1),
        })
    }

    /// Position edits move the complete selection without resizing it, even
    /// when inspector dimensions exclude hidden descendants.
    pub fn update_selection_position(&mut self, position: Point) {
        let Some(state) = self.selection_geometry() else {
            return;
        };
        if state.locked {
            return;
        }
        let Some(mut bounds) = self.movable_bounds() else {
            return;
        };
        let origin = self.geometry_origin();
        bounds.x = position.x + origin.x;
        bounds.y = position.y + origin.y;
        self.update_selection_bounds(bounds);
    }

    /// Update within a caller-owned transaction. Inspector positions use the
    /// current group's uncropped origin. Size edits target the complete group
    /// frame even when the inspector displays bounds excluding hidden markup.
    pub fn update_selection_geometry(&mut self, mut bounds: Bounds) {
        let Some(state) = self.selection_geometry() else {
            return;
        };
        if state.locked {
            return;
        }
        if !state.width_editable {
            bounds.w = state.bounds.w;
        }
        if !state.height_editable {
            bounds.h = state.bounds.h;
        }
        let origin = self.geometry_origin();
        bounds.x += origin.x;
        bounds.y += origin.y;
        self.update_selection_bounds(bounds);
    }

    /// Apply a world-space selection rectangle using each descendant's resize
    /// axes. Fixed dimensions retain their edge anchoring within the selection.
    /// Snapshot history belongs to the caller, including continuous previews.
    pub fn update_selection_bounds(&mut self, bounds: Bounds) {
        if ![bounds.x, bounds.y, bounds.w, bounds.h]
            .iter()
            .all(|v| v.is_finite())
            || bounds.w < 0.
            || bounds.h < 0.
        {
            return;
        }
        let Some(from) = self.movable_bounds() else {
            return;
        };
        if from == bounds {
            return;
        }
        let Ok(layout) = self.layout_page(self.surface_id()) else {
            return;
        };
        let ids: HashSet<_> = self
            .selected_views()
            .iter()
            .filter(|c| !c.locked)
            .map(|c| c.id)
            .collect();
        let parent = self.context_origin();
        let mut updates = Vec::new();
        for before in self.nodes().iter().filter(|n| ids.contains(&n.id)) {
            let mut path = self.context.clone();
            path.push(before.id);
            let Some(geometry) = view::find_layout(&layout.nodes, &path) else {
                return;
            };
            let mut after = before.clone();
            if let Err(error) = transform(
                &self.document,
                &mut after,
                geometry,
                parent,
                parent,
                from,
                bounds,
            ) {
                self.error = Some(error);
                return;
            }
            updates.push((before.clone(), after));
        }
        for (before, after) in updates {
            if self.editing_instance() {
                if let Err(error) = self.write_local_node(before.id, &before, &after) {
                    self.error = Some(error);
                    return;
                }
            } else if let Some(node) = self.nodes_mut().iter_mut().find(|n| n.id == before.id) {
                *node = after;
            }
        }
    }

    pub fn linear_axis(&self, id: Id) -> Option<crate::linear_controls::Axis> {
        let control = self.current().controls.into_iter().find(|c| c.id == id)?;
        if control.group.is_some() {
            return None;
        }
        if control.kind == Kind::Line {
            let (_, properties) = self.control_properties(id)?;
            return (!crate::rules::legacy_frame(&properties)).then(|| {
                crate::linear_controls::Axis {
                    vertical: false,
                    thickness: crate::rules::height(&properties),
                }
            });
        }
        crate::linear_controls::Axis::for_kind(control.kind)
    }
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
