// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignmentAction {
    Left,
    Center,
    Right,
    Top,
    Middle,
    Bottom,
    DistributeHorizontal,
    DistributeVertical,
    SpaceHorizontal,
    SpaceVertical,
}
impl AlignmentAction {
    pub fn vertical(self) -> bool {
        matches!(
            self,
            Self::Top
                | Self::Middle
                | Self::Bottom
                | Self::DistributeVertical
                | Self::SpaceVertical
        )
    }
}
impl Editor {
    pub fn can_align_selection(&self) -> bool {
        self.selected_views().iter().filter(|c| !c.locked).count() >= 2
    }
    pub fn align_selection(&mut self, action: AlignmentAction) {
        use AlignmentAction::*;
        let mut controls: Vec<_> = self
            .selected_views()
            .into_iter()
            .filter(|c| !c.locked)
            .collect();
        if controls.len() < 2 {
            return;
        }
        let full_bounds: std::collections::HashMap<_, _> =
            controls.iter().map(|c| (c.id, c.bounds)).collect();
        if !self.markup_visible
            && let Ok(layout) = self.layout_page(self.surface_id())
        {
            for control in &mut controls {
                let mut path = self.context.clone();
                path.push(control.id);
                if let Some(node) = view::find_layout(&layout.nodes, &path) {
                    let b = node.without_markup;
                    control.bounds = Bounds::new(b.x, b.y, b.width, b.height);
                }
            }
        }
        let vertical = action.vertical();
        let start = |c: &Control| if vertical { c.bounds.y } else { c.bounds.x };
        let size = |c: &Control| if vertical { c.bounds.h } else { c.bounds.w };
        let center = |c: &Control| crate::model::round_coordinate(start(c) + size(c) / 2.);
        let min = controls.iter().map(start).reduce(f32::min).unwrap();
        let max = controls
            .iter()
            .map(|c| start(c) + size(c))
            .reduce(f32::max)
            .unwrap();
        let positions: Vec<_> = match action {
            Left | Top => controls.iter().map(|_| min).collect(),
            Right | Bottom => controls.iter().map(|c| max - size(c)).collect(),
            Center | Middle => {
                let middle = crate::model::round_coordinate((min + max) / 2.);
                controls
                    .iter()
                    .map(|c| start(c) + middle - center(c))
                    .collect()
            }
            DistributeHorizontal | DistributeVertical => {
                controls.sort_by(|a, b| center(a).total_cmp(&center(b)));
                let first = center(&controls[0]);
                let step = (center(controls.last().unwrap()) - first) / (controls.len() - 1) as f32;
                controls
                    .iter()
                    .enumerate()
                    .map(|(i, c)| first + step * i as f32 - size(c) / 2.)
                    .collect()
            }
            SpaceHorizontal | SpaceVertical => {
                controls.sort_by(|a, b| center(a).total_cmp(&center(b)));
                let gap = (max - min - controls.iter().map(size).sum::<f32>())
                    / (controls.len() - 1) as f32;
                let mut at = min;
                controls
                    .iter()
                    .map(|c| {
                        let p = at;
                        at += size(c) + gap;
                        p
                    })
                    .collect()
            }
        };
        self.edit("Align controls", |e| {
            for (mut control, position) in controls.into_iter().zip(positions) {
                let full = full_bounds[&control.id];
                // Arrangement measures visible bounds but writes a complete
                // control origin. Edge/spacing actions retain the visible
                // origin offset; center actions already include it.
                let offset = if matches!(action, Center | Middle) {
                    0.
                } else if vertical {
                    control.bounds.y - full.y
                } else {
                    control.bounds.x - full.x
                };
                let position = if matches!(
                    action,
                    DistributeHorizontal | DistributeVertical | SpaceHorizontal | SpaceVertical
                ) {
                    (position + offset).floor()
                } else {
                    position + offset
                };
                control.bounds = full;
                if vertical {
                    control.bounds.y = position;
                } else {
                    control.bounds.x = position;
                }
                e.update_control(control);
            }
        });
    }
    pub fn align(&mut self, axis: usize, edge: usize) {
        use AlignmentAction::*;
        self.align_selection(match (axis, edge) {
            (0, 0) => Left,
            (0, 1) => Center,
            (0, _) => Right,
            (_, 0) => Top,
            (_, 1) => Middle,
            _ => Bottom,
        });
    }
    pub fn distribute(&mut self, axis: usize) {
        self.align_selection(if axis == 0 {
            AlignmentAction::DistributeHorizontal
        } else {
            AlignmentAction::DistributeVertical
        });
    }
    pub fn supports_auto_size(&self, id: Id) -> bool {
        self.control_properties(id).is_some_and(|(spec, _)| {
            spec.autosize != catalog::AutoSize::None
                && !matches!(
                    spec.id,
                    "image" | "icon" | "breadcrumbs" | "link_bar" | "playback" | "toggle"
                )
        })
    }
    pub fn can_auto_size_selection(&self, all: bool) -> bool {
        !self.selection.is_empty()
            && if all {
                self.selection.iter().all(|id| self.supports_auto_size(*id))
            } else {
                self.selection.iter().any(|id| self.supports_auto_size(*id))
            }
    }
    pub fn auto_size_selection(&mut self) -> Result<(), String> {
        let ids: Vec<_> = self
            .selected_views()
            .iter()
            .filter(|c| self.supports_auto_size(c.id))
            .map(|c| c.id)
            .collect();
        self.begin("Auto-size controls");
        for id in ids {
            if let Err(error) = self.auto_size_control(id) {
                self.cancel();
                return Err(error);
            }
        }
        self.commit();
        Ok(())
    }
}
