// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, workspace::Editor};

fn fixture() -> (Editor, Id, Id, Id) {
    let mut project = Project {
        font: None,
        next_id: 4,
        ..Project::default()
    };
    project.pages[0].content.nodes.push(Node::group(
        2,
        Point::new(40., 60.),
        vec![Node::control(
            3,
            "button",
            Point::new(10., 15.),
            Size::fixed(80., 40.),
        )],
    ));
    let mut e = Editor::new(project);
    e.select(2, false);
    let source = e.create_component("Shared source".into()).unwrap();
    let origin = e.document.default_library.unwrap();
    e.edit_source_notes(source, "Source note – 世界").unwrap();
    e.document.libraries[0].content.nodes[0].locked = true;
    e.document.libraries[0].content.nodes[0].crop = Some(Crop {
        width: 0.8,
        ..Crop::ALL
    });
    let target = e.create_library("Destination").unwrap();
    e.create_empty_source(target, "Shared source", Point::new(6., 6.))
        .unwrap();
    e.edit_library_source(source).unwrap();
    // Drop fixture construction history, retaining a valid current document.
    let mut e = Editor::new(e.document);
    e.edit_library_source(source).unwrap();
    (e, source, origin, target)
}
#[test]
fn move_preserves_shared_content_references_notes_appearance_and_exact_history() {
    let (mut e, source, origin, target) = fixture();
    let before = e.document.clone();
    let resolved = before.resolve(&before.pages[0].content.nodes[0]).unwrap();
    let old = before.component_source(source).unwrap();
    let placement = old
        .0
        .content
        .nodes
        .iter()
        .find(|n| n.id == old.1.node)
        .unwrap();
    e.move_sources_to_library(&[source], target).unwrap();
    let (owner, moved) = e.document.component_source(source).unwrap();
    assert_eq!(owner.id, target);
    assert_eq!(moved.notes, old.1.notes);
    let mut expected = placement.clone();
    expected.id = moved.node;
    assert_ne!(expected.id, placement.id);
    assert_eq!(owner.content.nodes.last(), Some(&expected));
    assert!(
        e.document
            .libraries
            .iter()
            .find(|l| l.id == origin)
            .unwrap()
            .sources
            .is_empty()
    );
    assert_eq!(e.document.definitions, before.definitions);
    assert_eq!(e.document.pages, before.pages);
    assert_eq!(
        e.document
            .resolve(&e.document.pages[0].content.nodes[0])
            .unwrap(),
        resolved
    );
    assert_eq!(e.library, Some(target));
    assert_eq!(e.definition, Some(source));
    assert_eq!(e.undo_name(), Some("Move components to library"));
    let after = e.document.clone();
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(e.library, Some(origin));
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(e.library, Some(target));
}
#[test]
fn multi_owner_move_skips_resident_sources_preserves_order_and_has_one_undo() {
    let (mut e, first, origin, target) = fixture();
    let resident = e
        .document
        .libraries
        .iter()
        .find(|l| l.id == target)
        .unwrap()
        .sources[0]
        .definition;
    let third = e.create_library("Third").unwrap();
    let second = e
        .create_empty_source(third, "Second", Point::new(200., 300.))
        .unwrap();
    for l in &mut e.document.libraries {
        l.content.paint_order = l.content.paint_paths().unwrap();
    }
    let before = e.document.clone();
    e.move_sources_to_library(&[resident, first, second], target)
        .unwrap();
    let destination = e
        .document
        .libraries
        .iter()
        .find(|l| l.id == target)
        .unwrap();
    assert_eq!(
        destination
            .sources
            .iter()
            .map(|s| s.definition)
            .collect::<Vec<_>>(),
        vec![resident, first, second]
    );
    assert_eq!(e.definition, Some(first));
    assert_eq!(e.document.definitions, before.definitions);
    e.document.validate().unwrap();
    assert!(
        e.document
            .libraries
            .iter()
            .filter(|l| [origin, third].contains(&l.id))
            .all(|l| l.sources.is_empty() && l.content.paint_order.is_empty())
    );
    e.undo();
    assert_eq!(e.document, before);
}
#[test]
fn invalid_noop_and_exhausted_moves_preserve_document_context_and_redo() {
    for case in 0..5 {
        let (mut e, source, origin, target) = fixture();
        e.move_sources_to_library(&[source], target).unwrap();
        e.undo();
        if case == 4 {
            e.document.next_id = u64::MAX;
        }
        let before = e.document.clone();
        let result = match case {
            0 => e.move_sources_to_library(&[source], origin),
            1 => e.move_sources_to_library(&[source, 999999], target),
            2 => e.move_sources_to_library(&[source, source], target),
            3 => e.move_sources_to_library(&[source], 999999),
            _ => e.move_sources_to_library(&[source], target),
        };
        assert_eq!(result.is_ok(), case == 0);
        assert_eq!(e.document, before);
        assert_eq!(e.library, Some(origin));
        assert!(e.undo_name().is_none());
        assert!(e.redo_name().is_some());
    }
}
