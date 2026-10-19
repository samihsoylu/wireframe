// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerAction {
    Back,
    Backward,
    Forward,
    Front,
}
impl LayerAction {
    fn forward(self) -> bool {
        matches!(self, Self::Forward | Self::Front)
    }
    fn extreme(self) -> bool {
        matches!(self, Self::Back | Self::Front)
    }
}

struct Plan {
    order: Vec<Id>,
    anchor: Option<Id>,
}

fn relocate(order: &mut Vec<Id>, id: Id, slot: isize) {
    let old = order.iter().position(|&item| item == id).unwrap();
    order.remove(old);
    order.insert((slot.max(0) as usize).min(order.len()), id);
}

fn plan(items: &[(Id, Bounds)], selected: &HashSet<Id>, action: LayerAction) -> Option<Plan> {
    let original: Vec<_> = items.iter().map(|&(id, _)| id).collect();
    let chosen: Vec<_> = original
        .iter()
        .copied()
        .filter(|id| selected.contains(id))
        .collect();
    if chosen.is_empty() {
        return None;
    }
    let mut order = original.clone();
    let mut anchor = None;
    if action.extreme() {
        order.sort_by_key(|id| selected.contains(id) == action.forward());
    } else {
        let mut candidates: Vec<_> = (0..items.len())
            .filter(|&i| selected.contains(&items[i].0))
            .collect();
        if action.forward() {
            candidates.reverse();
        }
        let (pivot, target) = candidates.into_iter().find_map(|i| {
            let mut range: Box<dyn Iterator<Item = usize>> = if action.forward() {
                Box::new(i + 1..items.len())
            } else {
                Box::new((0..i).rev())
            };
            range
                .find(|&j| !selected.contains(&items[j].0) && items[i].1.intersects(items[j].1))
                .map(|j| (items[i].0, j))
        })?;
        anchor = Some(items[target].0);
        let pivot_rank = chosen.iter().position(|&id| id == pivot).unwrap();
        relocate(&mut order, pivot, target as isize);
        for (offset, &id) in chosen[pivot_rank + 1..].iter().enumerate() {
            relocate(&mut order, id, (target + offset + 1) as isize);
        }
        // Earlier selected siblings are gathered toward the crossing point.
        // Backward movement starts at the target slot; its pivot can therefore
        // precede selected siblings that originally lay below that target.
        let first = target as isize - isize::from(action.forward());
        for (offset, &id) in chosen[..pivot_rank].iter().rev().enumerate() {
            relocate(&mut order, id, first - offset as isize);
        }
    }
    // Legacy leaf paint order can still change when sibling order is already
    // at the requested extreme. The edit transaction suppresses true no-ops.
    (action.extreme() || order != original).then_some(Plan { order, anchor })
}

impl Editor {
    fn layer_plan(&self, action: LayerAction) -> Option<Plan> {
        if self.selection.is_empty() || self.require_structure_edit().is_err() {
            return None;
        }
        let views = self.current().controls;
        let items: Option<Vec<_>> = self
            .nodes()
            .iter()
            .map(|n| {
                views
                    .iter()
                    .find(|c| c.id == n.id)
                    .map(|c| (n.id, c.bounds))
            })
            .collect();
        let items = items?;
        plan(&items, &self.selection, action)
    }

    pub fn can_arrange(&self, action: LayerAction) -> bool {
        self.layer_plan(action).is_some()
    }

    pub fn arrange(&mut self, action: LayerAction) {
        if !self.structure_edit_allowed() {
            return;
        }
        let Some(plan) = self.layer_plan(action) else {
            return;
        };
        self.edit("Change layering", |e| {
            let rank: HashMap<_, _> = plan
                .order
                .iter()
                .enumerate()
                .map(|(i, &id)| (id, i))
                .collect();
            e.nodes_mut().sort_by_key(|n| rank[&n.id]);
            let context = e.context.clone();
            let selected = e.selection.clone();
            let paint_order = &mut e.active_content_mut().paint_order;
            let scoped: Vec<_> = paint_order
                .iter()
                .filter(|path| path.starts_with(&context))
                .cloned()
                .collect();
            if scoped.is_empty() {
                return;
            }
            let (mut moving, mut remaining): (Vec<_>, Vec<_>) =
                scoped.into_iter().partition(|path| {
                    path.get(context.len())
                        .is_some_and(|id| selected.contains(id))
                });
            if !action.extreme() {
                moving.sort_by_key(|path| rank[&path[context.len()]]);
            }
            let slot = match plan.anchor {
                Some(anchor) if action.forward() => remaining
                    .iter()
                    .rposition(|path| path.get(context.len()) == Some(&anchor))
                    .map_or(remaining.len(), |i| i + 1),
                Some(anchor) => remaining
                    .iter()
                    .position(|path| path.get(context.len()) == Some(&anchor))
                    .unwrap_or(0),
                None if action.forward() => remaining.len(),
                None => 0,
            };
            remaining.splice(slot..slot, moving);
            let mut reordered = remaining.into_iter();
            for path in paint_order
                .iter_mut()
                .filter(|path| path.starts_with(&context))
            {
                *path = reordered.next().unwrap();
            }
        });
    }
}
