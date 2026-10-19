// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::collections::HashSet;
use wireframe::{
    document::{Id, NodeContent, Size},
    model::Kind,
    storage::ProjectStore,
    workspace::{Editor, LayerAction::*},
};

fn specimen() -> (Editor, [Id; 4]) {
    let mut e = Editor::default();
    for (x, y) in [(20., 20.), (500., 20.), (70., 70.), (130., 130.)] {
        e.add(Kind::Button, x, y);
        if let NodeContent::Control { size, .. } =
            &mut e.current_mut().content.nodes.last_mut().unwrap().content
        {
            *size = Size::fixed(160., 100.);
        }
    }
    let ids = order(&e).try_into().unwrap();
    (e, ids)
}
fn order(e: &Editor) -> Vec<Id> {
    e.nodes().iter().map(|n| n.id).collect()
}

#[test]
fn steps_cross_overlaps_skip_gaps_and_have_exact_history() {
    let (mut e, [a, gap, b, c]) = specimen();
    e.select(a, false);
    let before = e.document.clone();
    assert!(e.can_arrange(Forward));
    assert!(!e.can_arrange(Backward));
    e.arrange(Forward);
    assert_eq!(order(&e), [gap, b, a, c]);
    assert!(!e.can_arrange(Forward));
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    e.arrange(Backward);
    assert_eq!(order(&e), [gap, a, b, c]);
    e.undo();
    assert_eq!(e.document, after);
}

#[test]
fn isolated_control_has_extreme_actions_but_no_step_or_spurious_history() {
    let (mut e, [a, gap, b, c]) = specimen();
    e.select(gap, false);
    assert!(!e.can_arrange(Forward));
    assert!(!e.can_arrange(Backward));
    assert!(e.can_arrange(Front));
    assert!(e.can_arrange(Back));
    let before = e.document.clone();
    let history = e.undo_name().map(str::to_owned);
    e.arrange(Forward);
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), history.as_deref());
    e.arrange(Front);
    assert_eq!(order(&e), [a, b, c, gap]);
    e.undo();
    assert_eq!(e.document, before);
    e.arrange(Backward);
    assert!(e.redo_name().is_some());
    e.redo();
    assert_eq!(order(&e), [a, b, c, gap]);
    e.arrange(Back);
    assert_eq!(order(&e), [gap, a, b, c]);
}

#[test]
fn separated_selection_is_gathered_at_the_crossing_with_directional_order() {
    let (mut e, [a, gap, b, c]) = specimen();
    e.selection = HashSet::from([a, c]);
    let before = e.document.clone();
    e.arrange(Backward);
    assert_eq!(order(&e), [gap, c, a, b]);
    e.undo();
    assert_eq!(e.document, before);
    e.arrange(Forward);
    assert_eq!(order(&e), [gap, b, a, c]);
}

#[test]
fn touching_edges_count_as_overlap() {
    let (mut e, [a, _, b, _]) = specimen();
    e.current_mut().content.nodes[2].origin.x = 180.;
    e.select(a, false);
    assert!(e.can_arrange(Forward));
    e.arrange(Forward);
    assert!(order(&e).iter().position(|id| *id == a) > order(&e).iter().position(|id| *id == b));
}

#[test]
fn selection_can_gather_beyond_the_last_sibling_without_losing_items() {
    let (mut e, [a, gap, b, c]) = specimen();
    e.current_mut().content.nodes[2].origin.y = 300.;
    e.current_mut().content.nodes[3].origin.y = 20.;
    e.selection = HashSet::from([a, b]);
    let before = e.document.clone();
    e.arrange(Forward);
    assert_eq!(order(&e), [gap, c, a, b]);
    e.document.validate().unwrap();
    e.undo();
    assert_eq!(e.document, before);
}

#[test]
fn nested_steps_preserve_other_parent_slots_and_round_trip() {
    let (mut e, [a, gap, b, c]) = specimen();
    e.selection = HashSet::from([a, b, c]);
    e.group();
    let group = *e.selection.iter().next().unwrap();
    assert!(e.enter(group));
    let before = e.document.clone();
    e.select(a, false);
    e.arrange(Forward);
    assert_eq!(order(&e), [b, a, c]);
    let paths = &e.current_mut().content.paint_order;
    assert_eq!(
        paths,
        &vec![vec![group, b], vec![gap], vec![group, a], vec![group, c]]
    );
    e.document.validate().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("layers.wireframe");
    ProjectStore.save(&file, &e.document).unwrap();
    assert_eq!(ProjectStore.load(&file).unwrap(), e.document);
    e.undo();
    assert_eq!(e.document, before);
}
