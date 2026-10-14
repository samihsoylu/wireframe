// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::{Caret, State};
use eframe::egui::{
    self, Id, Rect, Response, Ui, Vec2,
    accesskit::{self, Action, ActionData, NodeId, Role, TextPosition, TextSelection},
};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone)]
pub(super) struct Run {
    node: NodeId,
    source: Range<usize>,
    boundaries: Vec<usize>,
    hard_break: bool,
}

pub(super) fn action(state: &mut State, id: Id, request: &accesskit::ActionRequest) -> bool {
    if request.target_node != id.accesskit_id() {
        return false;
    }
    match &request.data {
        Some(ActionData::Value(value)) if request.action == Action::SetValue => {
            state.composition = None;
            state.draft.select_all();
            state.draft.insert(value);
            true
        }
        Some(ActionData::SetTextSelection(selection)) => {
            if state.composition.is_some() {
                return false;
            }
            let resolve = |position: &TextPosition| {
                state
                    .ax_runs
                    .iter()
                    .find(|r| r.node == position.node)
                    .and_then(|r| r.boundaries.get(position.character_index))
                    .copied()
            };
            if let (Some(anchor), Some(focus)) =
                (resolve(&selection.anchor), resolve(&selection.focus))
            {
                state.draft.select(Caret::new(anchor), Caret::new(focus));
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

struct Unit {
    source: Range<usize>,
    rect: Rect,
    rtl: bool,
}

pub(super) fn update(ui: &mut Ui, response: &Response, state: &mut State, inner: Rect) {
    if ui
        .ctx()
        .accesskit_node_builder(response.id, |_| ())
        .is_none()
    {
        return;
    }
    let (source, caret, composition) = state.display();
    let Some(layout) = &state.layout else { return };
    let mut spans: Vec<_> = layout
        .rows
        .iter()
        .flat_map(|row| {
            let mut spans: Vec<_> = row
                .cells
                .iter()
                .map(|c| (c.source.clone(), c.rect, c.rtl))
                .collect();
            if !row.ending.is_empty() {
                let rect = layout
                    .caret_rect(Caret::new(row.ending.start))
                    .unwrap_or(row.rect);
                spans.push((row.ending.clone(), rect, false));
            }
            if row.cells.is_empty() && row.ending.is_empty() {
                spans.push((row.source.clone(), row.rect, false));
            }
            spans
        })
        .collect();
    spans.sort_by_key(|s| (s.0.start, s.0.end));
    spans.dedup_by(|a, b| a.0 == b.0);
    let mut groups: Vec<Vec<Unit>> = Vec::new();
    for (range, rect, rtl) in spans {
        // AccessKit lengths are u8. Only unusually long combining sequences
        // need splitting; normal units are whole extended graphemes or CRLF.
        let mut start = range.start;
        let ends = source[range.clone()]
            .char_indices()
            .map(|(i, c)| range.start + i + c.len_utf8())
            .collect::<Vec<_>>();
        let mut chunks = Vec::new();
        while range.end - start > 255 {
            let end = *ends
                .iter()
                .skip_while(|&&end| end <= start)
                .take_while(|&&end| end - start <= 255)
                .last()
                .unwrap();
            chunks.push(start..end);
            start = end;
        }
        chunks.push(start..range.end);
        for source_range in chunks {
            let unit = Unit {
                source: source_range,
                rect,
                rtl,
            };
            let merge = groups.last().is_some_and(|group| {
                let last = group.last().unwrap();
                !unit.source.is_empty()
                    && !last.source.is_empty()
                    && group.len() < 255
                    && last.source.end == unit.source.start
                    && last.rtl == rtl
                    && last.rect.top() == rect.top()
                    && if rtl {
                        (last.rect.left() - rect.right()).abs() < 0.1
                    } else {
                        (last.rect.right() - rect.left()).abs() < 0.1
                    }
            });
            if merge {
                groups.last_mut().unwrap().push(unit);
            } else {
                groups.push(vec![unit]);
            }
        }
    }
    let word_starts: Vec<_> = source.unicode_word_indices().map(|(i, _)| i).collect();
    state.ax_runs.clear();
    let offset = inner.min.to_vec2() - Vec2::new(0., state.scroll);
    for units in groups {
        let range = units[0].source.start..units.last().unwrap().source.end;
        let rtl = units[0].rtl;
        let rect = units.iter().fold(Rect::NOTHING, |r, u| r.union(u.rect));
        let child = ui.new_child(
            egui::UiBuilder::new()
                .id_salt((response.id, "source-run", range.start))
                .accessibility_parent(response.id),
        );
        let id = child.unique_id();
        drop(child);
        let value = &source[range.clone()];
        let screen = rect.translate(offset);
        let mut boundaries = units
            .iter()
            .filter(|u| !u.source.is_empty())
            .map(|u| u.source.start)
            .collect::<Vec<_>>();
        boundaries.push(range.end);
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_role(Role::TextRun);
            node.set_value(value);
            node.set_character_lengths(
                units
                    .iter()
                    .filter(|u| !u.source.is_empty())
                    .map(|u| u.source.len() as u8)
                    .collect::<Vec<_>>(),
            );
            node.set_character_positions(
                units
                    .iter()
                    .filter(|u| !u.source.is_empty())
                    .map(|u| {
                        if rtl {
                            rect.right() - u.rect.right()
                        } else {
                            u.rect.left() - rect.left()
                        }
                    })
                    .collect::<Vec<_>>(),
            );
            node.set_character_widths(
                units
                    .iter()
                    .filter(|u| !u.source.is_empty())
                    .map(|u| u.rect.width())
                    .collect::<Vec<_>>(),
            );
            node.set_word_starts(
                units
                    .iter()
                    .enumerate()
                    .filter_map(|(i, u)| {
                        word_starts
                            .binary_search(&u.source.start)
                            .is_ok()
                            .then_some(i as u8)
                    })
                    .collect::<Vec<_>>(),
            );
            node.set_text_direction(if rtl {
                accesskit::TextDirection::RightToLeft
            } else {
                accesskit::TextDirection::LeftToRight
            });
            node.set_bounds(accesskit::Rect {
                x0: screen.left() as f64,
                y0: screen.top() as f64,
                x1: screen.right() as f64,
                y1: screen.bottom() as f64,
            });
        });
        state.ax_runs.push(Run {
            node: id.accesskit_id(),
            source: range,
            boundaries,
            hard_break: value.ends_with(['\r', '\n']),
        });
    }
    let selection = if composition.is_some() {
        super::Selection {
            anchor: caret,
            focus: caret,
        }
    } else {
        state.draft.selection()
    };
    let position = |caret: Caret| {
        let preferred = match caret.affinity {
            super::super::Affinity::Upstream => state
                .ax_runs
                .iter()
                .find(|r| r.source.end == caret.byte && !r.hard_break),
            super::super::Affinity::Downstream => {
                state.ax_runs.iter().find(|r| r.source.start == caret.byte)
            }
        };
        let run = preferred
            .or_else(|| {
                state
                    .ax_runs
                    .iter()
                    .find(|r| r.source.start <= caret.byte && caret.byte < r.source.end)
            })
            .or_else(|| state.ax_runs.last())?;
        let character_index = run
            .boundaries
            .partition_point(|&i| i < caret.byte)
            .min(run.boundaries.len() - 1);
        Some(TextPosition {
            node: run.node,
            character_index,
        })
    };
    let positions = position(selection.anchor).zip(position(selection.focus));
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(Role::MultilineTextInput);
        node.set_value(source.as_str());
        node.add_action(Action::SetTextSelection);
        node.add_action(Action::SetValue);
        if let Some((anchor, focus)) = positions {
            node.set_text_selection(TextSelection { anchor, focus });
        }
    });
}
