// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    workspace::{AlignmentAction, Editor},
};

fn fixture() -> Editor {
    let mut p = Project {
        next_id: 10,
        ..Project::default()
    };
    let mut marked = Node::control(3, "rectangle", Point::default(), Size::fixed(81., 40.));
    marked.markup = true;
    let plain = Node::control(4, "rectangle", Point::new(299., 0.), Size::fixed(81., 40.));
    p.pages[0].content.nodes = vec![
        Node::group(2, Point::new(100., 100.), vec![marked, plain]),
        Node::control(
            5,
            "rectangle",
            Point::new(700., 100.),
            Size::fixed(81., 40.),
        ),
    ];
    let mut e = Editor::new(p);
    e.select_all();
    e.set_markup_visible(false);
    e
}
#[test]
fn hidden_group_alignment_matches_origin_offset_and_preserves_child_data() {
    use AlignmentAction::*;
    for (action, expected) in [
        (Left, [698., 399.]),
        (Center, [549., 549.]),
        (Right, [999., 700.]),
        (DistributeHorizontal, [698., 700.]),
        (SpaceHorizontal, [698., 700.]),
    ] {
        let mut e = fixture();
        let before = e.document.clone();
        e.align_selection(action);
        assert_eq!(
            e.nodes().iter().map(|n| n.origin.x).collect::<Vec<_>>(),
            expected,
            "{action:?}"
        );
        for (a, b) in e.nodes().iter().zip(&before.pages[0].content.nodes) {
            assert_eq!(a.content, b.content);
        }
        let after = e.document.clone();
        e.undo();
        assert_eq!(e.document, before);
        e.redo();
        assert_eq!(e.document, after);
        assert!(!e.markup_visible());
    }
}
#[test]
fn vertical_hidden_offsets_keep_alignment_axis_and_locked_targets_outside_transaction() {
    use AlignmentAction::*;
    fn transpose(node: &mut Node) {
        std::mem::swap(&mut node.origin.x, &mut node.origin.y);
        match &mut node.content {
            NodeContent::Control { size, .. } => std::mem::swap(&mut size.width, &mut size.height),
            NodeContent::Group { children } => children.iter_mut().for_each(transpose),
            _ => unreachable!(),
        }
    }
    for (action, expected) in [
        (Top, [698., 399.]),
        (Middle, [549., 549.]),
        (Bottom, [999., 700.]),
        (DistributeVertical, [698., 700.]),
        (SpaceVertical, [698., 700.]),
    ] {
        let mut p = fixture().document;
        p.pages[0].content.nodes.iter_mut().for_each(transpose);
        let mut e = Editor::new(p);
        e.select_all();
        e.set_markup_visible(false);
        let before = e.document.clone();
        e.align_selection(action);
        assert_eq!(
            e.nodes().iter().map(|n| n.origin.y).collect::<Vec<_>>(),
            expected
        );
        assert!(e.nodes().iter().all(|n| n.origin.x == 100.));
        e.undo();
        assert_eq!(e.document, before);
    }
    let mut e = fixture();
    e.select(2, false);
    e.lock(true);
    e.select(5, true);
    let before = e.document.clone();
    let undo = e.undo_name().map(str::to_owned);
    assert!(!e.can_align_selection());
    e.align_selection(Left);
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), undo.as_deref());
}
#[test]
fn local_component_alignment_changes_placement_without_rewriting_definition() {
    let mut e = fixture();
    e.select(2, false);
    e.create_component("Shared panel".into()).unwrap();
    let definitions = e.document.definitions.clone();
    e.set_markup_visible(false);
    e.select(5, true);
    let before = e.document.clone();
    e.align_selection(AlignmentAction::Left);
    assert_eq!(e.nodes()[0].origin.x, 698.);
    assert_eq!(e.document.definitions, definitions);
    e.undo();
    assert_eq!(e.document, before);
}
