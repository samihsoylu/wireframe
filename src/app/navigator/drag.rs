// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::page_tree::PageTree;

pub(crate) struct PageDrag {
    pages: Vec<Id>,
    tree: Vec<(Id, Option<Id>)>,
    session: String,
    caption: String,
}

pub(crate) struct PagePress {
    owner: Id,
    point: Pos2,
    drag: PageDrag,
}

fn identity(tree: &PageTree) -> Vec<(Id, Option<Id>)> {
    tree.rows.iter().map(|r| (r.id, r.parent)).collect()
}

fn primary_press(events: &[egui::Event]) -> Option<usize> {
    events.iter().rposition(|event| {
        matches!(
            event,
            egui::Event::PointerButton {
                button: egui::PointerButton::Primary,
                pressed: true,
                ..
            }
        )
    })
}

fn primary_release(events: &[egui::Event]) -> Option<(usize, Pos2)> {
    let first = primary_press(events).map_or(0, |n| n + 1);
    events[first..]
        .iter()
        .enumerate()
        .find_map(|(index, event)| match event {
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                ..
            } => Some((first + index, *pos)),
            _ => None,
        })
}

/// A visible gap after a closed branch follows all its hidden descendants.
fn drop_gap(tree: &PageTree, rows: &[(Id, Rect)], point: Pos2) -> Option<(usize, f32)> {
    for &(id, rect) in rows {
        if point.y < rect.center().y {
            return Some((tree.rows.iter().position(|r| r.id == id)?, rect.top()));
        }
    }
    rows.last().map(|(_, r)| (tree.rows.len(), r.bottom()))
}

impl WireframeApp {
    pub(super) fn navigator_drag_start(
        &mut self,
        ui: &egui::Ui,
        tree: &PageTree,
        owner: Id,
        caption: &str,
        response: &egui::Response,
    ) {
        if !ui.is_enabled()
            || self.canvas_input_blocked()
            || ui.input(|i| !i.focused || i.key_pressed(Key::Escape))
        {
            self.navigator.press = None;
            return;
        }
        let events = ui.input(|i| {
            i.events
                .iter()
                .filter(|event| {
                    matches!(
                        event,
                        egui::Event::PointerMoved(_) | egui::Event::PointerButton { .. }
                    )
                })
                .cloned()
                .collect::<Vec<_>>()
        });
        let press = primary_press(&events);
        if let Some(index) = press {
            if self
                .navigator
                .press
                .as_ref()
                .is_some_and(|p| p.owner == owner)
            {
                self.navigator.press = None;
            }
            if let egui::Event::PointerButton { pos, .. } = events[index]
                && response.rect.intersect(ui.clip_rect()).contains(pos)
                && ui.ctx().layer_id_at(pos) == Some(ui.layer_id())
            {
                self.navigator.press = Some(PagePress {
                    owner,
                    point: pos,
                    drag: PageDrag {
                        pages: self.navigator.context(tree, owner),
                        tree: identity(tree),
                        session: self.editor.clipboard_session().into(),
                        caption: caption.into(),
                    },
                });
            }
        }
        // Framework drag flags omit a gesture whose movement and release are
        // delivered together. Keep its visible source press until that release.
        let first = press.map_or(0, |i| i + 1);
        let release = primary_release(&events).map(|(index, _)| index);
        let threshold = ui.ctx().options(|o| o.input_options.max_click_dist);
        let coalesced = release.is_some_and(|end| {
            self.navigator.press.as_ref().is_some_and(|p| {
                p.owner == owner
                    && p.drag.tree == identity(tree)
                    && p.drag.session == self.editor.clipboard_session()
                    && events[first..=end].iter().any(|event| match event {
                        egui::Event::PointerMoved(pos) | egui::Event::PointerButton { pos, .. } => {
                            pos.distance(p.point) > threshold
                        }
                        _ => false,
                    })
            })
        });
        if !response.drag_started_by(egui::PointerButton::Primary) && !coalesced {
            return;
        }
        self.end_pointer_gestures();
        response.request_focus();
        let mut drag = self
            .navigator
            .press
            .take()
            .filter(|p| p.owner == owner)
            .map(|p| p.drag)
            .unwrap_or_else(|| PageDrag {
                pages: self.navigator.context(tree, owner),
                tree: identity(tree),
                session: self.editor.clipboard_session().into(),
                caption: caption.into(),
            });
        if drag.pages.len() != 1 {
            drag.caption = format!("{} wireframes", drag.pages.len());
        }
        self.navigator.drag = Some(drag);
    }

    pub(super) fn navigator_drag_finish(
        &mut self,
        ui: &mut egui::Ui,
        tree: &PageTree,
        rows: &[(Id, Rect)],
    ) {
        if self.navigator.press.as_ref().is_some_and(|press| {
            !ui.is_enabled()
                || press.drag.tree != identity(tree)
                || press.drag.session != self.editor.clipboard_session()
                || ui.input(|i| !i.focused || !i.pointer.primary_down())
        }) {
            self.navigator.press = None;
        }
        let Some(drag) = &self.navigator.drag else {
            return;
        };
        let (focused, held, released, point) = ui.input(|i| {
            (
                i.focused,
                i.pointer.primary_down(),
                i.pointer.button_released(egui::PointerButton::Primary),
                primary_release(&i.events)
                    .map(|(_, point)| point)
                    .or_else(|| i.pointer.interact_pos()),
            )
        });
        if !ui.is_enabled()
            || self.canvas_input_blocked()
            || !focused
            || drag.session != self.editor.clipboard_session()
            || drag.tree != identity(tree)
            || (!held && !released)
        {
            self.navigator.drag = None;
            return;
        }
        let viewport = ui.clip_rect();
        let target = point
            .filter(|p| viewport.contains(*p))
            .and_then(|p| drop_gap(tree, rows, p));
        if released {
            let drag = self.navigator.drag.take().unwrap();
            if let Some((target, _)) = target {
                if let Err(error) = self.editor.reorder_pages(&drag.pages, target) {
                    self.error = Some(error);
                } else {
                    let tree = PageTree::new(&self.editor.document);
                    self.navigator.reveal(&tree, self.editor.page);
                    self.navigator.scroll_to = Some(self.editor.page);
                }
            }
            return;
        }
        ui.ctx().set_cursor_icon(if target.is_some() {
            egui::CursorIcon::Grabbing
        } else {
            egui::CursorIcon::NotAllowed
        });
        if let Some((_, y)) = target {
            ui.painter().line_segment(
                [
                    Pos2::new(viewport.left() + 5., y),
                    Pos2::new(viewport.right() - 5., y),
                ],
                Stroke::new(2., Color32::from_rgb(35, 135, 210)),
            );
        }
        if let Some(point) = point {
            let painter = ui.ctx().layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("page-drag-preview"),
            ));
            let galley = painter.layout_no_wrap(
                drag.caption.clone(),
                egui::FontId::proportional(12.),
                Color32::WHITE,
            );
            let rect = Rect::from_min_size(
                point + Vec2::new(12., 14.),
                galley.size() + Vec2::new(16., 10.),
            );
            painter.rect_filled(rect, 3., Color32::from_rgb(45, 90, 125));
            painter.galley(rect.min + Vec2::new(8., 5.), galley, Color32::WHITE);
            if viewport.contains(point) {
                let speed = if point.y < viewport.top() + 24. {
                    240.
                } else if point.y > viewport.bottom() - 24. {
                    -240.
                } else {
                    0.
                };
                if speed != 0. {
                    ui.scroll_with_delta(Vec2::new(
                        0.,
                        speed * ui.input(|i| i.stable_dt.min(0.05)),
                    ));
                    ui.ctx().request_repaint();
                }
            }
        }
    }
}
