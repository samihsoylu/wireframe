// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use egui::containers::panel::PanelState;
use std::ops::RangeInclusive;

#[derive(Clone, Copy)]
struct Press {
    start: Pos2,
    frame: u64,
}

#[derive(Clone, Copy)]
pub(in crate::app) enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

// Framework drag flags can omit movement and release delivered in one redraw.
// Preserve the visible handle's press, then let Panel apply its usual size limits.
pub(in crate::app) fn prepare(
    ui: &egui::Ui,
    id: egui::Id,
    edge: Edge,
    range: RangeInclusive<f32>,
    enabled: bool,
) -> bool {
    let key = id.with("wireframe-panel-press");
    let frame = ui.ctx().cumulative_frame_nr();
    let mut press = ui
        .ctx()
        .data(|d| d.get_temp::<Press>(key))
        .filter(|p| p.frame.saturating_add(1) >= frame);
    let events = ui.input(|i| i.events.clone());
    let cancelled = !enabled
        || !ui.is_enabled()
        || !ui.input(|i| i.focused)
        || events.iter().any(|e| {
            matches!(
                e,
                egui::Event::PointerGone
                    | egui::Event::Key {
                        key: Key::Escape,
                        pressed: true,
                        ..
                    }
            )
        });
    let handle = ui
        .read_response(id.with("__resize"))
        .filter(|r| r.enabled());
    let panel = PanelState::load(ui.ctx(), id);
    let mut finished = false;
    let mut consumed = Vec::new();
    if !cancelled && let (Some(handle), Some(mut panel)) = (handle, panel) {
        let axis = usize::from(matches!(edge, Edge::Top | Edge::Bottom));
        let threshold = ui.ctx().options(|o| o.input_options.max_click_dist);
        for (index, event) in events.into_iter().enumerate() {
            if press.is_some() && matches!(event, egui::Event::PointerMoved(_)) {
                consumed.push(index);
            }
            if let egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                ..
            } = event
            {
                if pressed {
                    press = (handle.rect.contains(pos)
                        && ui.ctx().layer_id_at(pos) == Some(handle.layer_id))
                    .then_some(Press { start: pos, frame });
                    if press.is_some() {
                        consumed.push(index);
                    }
                } else if let Some(start) = press.take() {
                    consumed.push(index);
                    if (pos[axis] - start.start[axis]).abs() <= threshold {
                        continue;
                    }
                    let from_max = matches!(edge, Edge::Right | Edge::Bottom);
                    let fixed = if from_max {
                        panel.outer_rect.max[axis]
                    } else {
                        panel.outer_rect.min[axis]
                    };
                    let size = (if from_max {
                        fixed - pos[axis]
                    } else {
                        pos[axis] - fixed
                    })
                    .clamp(*range.start(), *range.end());
                    if from_max {
                        panel.outer_rect.min[axis] = fixed - size;
                    } else {
                        panel.outer_rect.max[axis] = fixed + size;
                    }
                    ui.ctx().data_mut(|d| d.insert_persisted(id, panel));
                    finished = true;
                }
            }
        }
    } else {
        press = None;
    }
    // Panel already received egui's processed pointer state. Hide only its
    // raw gesture events from custom canvas, crop and page-drag handlers.
    // Keep later clicks in this batch available to their own targets.
    if press.is_some() || !consumed.is_empty() {
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new("panel-pointer-owner"), frame));
    }
    if !consumed.is_empty() {
        ui.input_mut(|input| {
            let mut index = 0;
            input.events.retain(|_| {
                let keep = !consumed.contains(&index);
                index += 1;
                keep
            });
        });
    }
    if let Some(press) = press.as_mut() {
        press.frame = frame;
    }
    ui.ctx().data_mut(|d| {
        if let Some(press) = press {
            d.insert_temp(key, press);
        } else {
            d.remove::<Press>(key);
        }
    });
    finished
}

pub(in crate::app) fn owns_pointer(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<u64>(egui::Id::new("panel-pointer-owner")))
        == Some(ctx.cumulative_frame_nr())
}
