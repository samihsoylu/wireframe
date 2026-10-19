// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, workspace::Editor};
fn selection(kind: &str) -> Editor {
    let mut p = Project {
        next_id: 4,
        ..Project::default()
    };
    p.pages[0].content.nodes = (2..4)
        .map(|id| {
            Node::control(
                id,
                kind,
                Point::new(50. + (id - 2) as f32 * 250., 50.),
                Size::fixed(180., 140.),
            )
        })
        .collect();
    let mut e = Editor::new(p);
    e.select_all();
    e
}
fn set(e: &mut Editor, key: &str, value: Value) {
    e.begin("Shared change");
    e.set_selection_property(key, value).unwrap();
    e.commit();
}
#[test]
fn shape_choices_relative_rotation_and_missing_eligibility_preserve_history() {
    let mut e = selection("shape");
    assert!(
        !e.selection_properties()
            .iter()
            .any(|p| p.key == "shape_rotation")
    );
    let before = e.document.clone();
    assert!(e.rotate_selection_shapes().is_err());
    assert_eq!(e.document, before);
    set(&mut e, "shape", Value::Choice("triangle".into()));
    e.set_control_property(3, "shape_rotation", Value::Number(90.))
        .unwrap();
    e.current_mut().content.nodes[1].locked = true;
    let before = e.document.clone();
    e.begin("Rotate shapes");
    e.rotate_selection_shapes().unwrap();
    e.commit();
    for (id, expected) in [(2, 90.), (3, 180.)] {
        assert_eq!(
            e.control_properties(id).unwrap().1["shape_rotation"],
            Value::Number(expected)
        );
    }
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
    set(&mut e, "shape", Value::Choice("ellipse".into()));
    assert!(
        !e.selection_properties()
            .iter()
            .any(|p| p.key == "shape_rotation")
    );
    assert_eq!(
        e.control_properties(3).unwrap().1["shape_rotation"],
        Value::Number(180.)
    );
    e.undo();
    assert_eq!(e.document, after);
    e.set_control_property(2, "shape", Value::Choice("rectangle".into()))
        .unwrap();
    let before = e.document.clone();
    assert!(e.rotate_selection_shapes().is_err());
    assert_eq!(e.document, before);
}
#[test]
fn shared_arrow_mode_matches_individual_geometry_conversion_and_is_atomic() {
    let mut e = selection("arrow");
    e.begin("Bent arrows");
    e.move_arrow_handle(
        2,
        wireframe::arrows::Handle::Middle,
        Point::new(140., 55.),
        false,
    )
    .unwrap();
    e.move_arrow_handle(
        3,
        wireframe::arrows::Handle::Middle,
        Point::new(320., 175.),
        false,
    )
    .unwrap();
    e.commit();
    let before = e.document.clone();
    let mut independent = Editor::new(before.clone());
    for id in [2, 3] {
        independent
            .set_control_property(id, "shape", Value::Choice("lines".into()))
            .unwrap();
    }
    set(&mut e, "shape", Value::Choice("lines".into()));
    assert_eq!(e.document, independent.document);
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    e.undo();
    e.begin("No op");
    e.set_selection_property("shape", Value::Choice("curve".into()))
        .unwrap();
    e.commit();
    e.redo();
    assert_eq!(e.document, after);
    e.undo();
    e.current_mut().content.nodes[1].locked = true;
    let locked = e.document.clone();
    assert!(
        e.set_selection_property("shape", Value::Choice("lines".into()))
            .is_err()
    );
    assert_eq!(e.document, locked);
}
#[test]
fn shared_heads_label_position_resets_and_family_guards() {
    let mut e = selection("arrow");
    e.set_control_property(2, "start_head", Value::Bool(false))
        .unwrap();
    e.set_control_property(3, "label_position", Value::Number(9.))
        .unwrap();
    e.select(3, false);
    e.select(2, true);
    let field = e
        .selection_properties()
        .into_iter()
        .find(|p| p.key == "label_position")
        .unwrap();
    assert_eq!(field.value, Value::Number(9.));
    assert!(field.mixed);
    for (key, value) in [
        ("start_head", Value::Bool(false)),
        ("end_head", Value::Bool(false)),
        ("label_position", Value::Number(7.)),
    ] {
        let before = e.document.clone();
        set(&mut e, key, value.clone());
        for id in [2, 3] {
            assert_eq!(e.control_properties(id).unwrap().1[key], value);
        }
        e.undo();
        assert_eq!(e.document, before);
    }
    let before = e.document.clone();
    assert!(
        e.set_selection_property("label_position", Value::Number(f64::NAN))
            .is_err()
    );
    assert_eq!(e.document, before);
    e.begin("Reset label position");
    e.reset_selection_property("label_position").unwrap();
    e.commit();
    assert_eq!(
        e.control_properties(3).unwrap().1["label_position"],
        Value::Number(5.)
    );
    e.undo();
    assert_eq!(e.document, before);
    e.current_mut().content.nodes[1] =
        Node::control(3, "shape", Point::new(300., 50.), Size::fixed(180., 140.));
    assert!(!e.selection_properties().iter().any(|p| matches!(
        p.key,
        "shape" | "label_position" | "start_head" | "end_head" | "shape_rotation"
    )));
}
