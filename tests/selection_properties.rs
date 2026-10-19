// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    workspace::{Editor, SelectionProperty},
};
fn specimen() -> Editor {
    let mut p = Project {
        next_id: 10,
        ..Project::default()
    };
    p.pages[0].content.nodes = ["button", "button", "text_input", "label"]
        .into_iter()
        .enumerate()
        .map(|(i, kind)| {
            Node::control(
                i as u64 + 2,
                kind,
                Point::new(50. + (i % 2) as f32 * 250., 50. + (i / 2) as f32 * 170.),
                Size::fixed(150., 60.),
            )
        })
        .collect();
    let mut e = Editor::new(p);
    for (id, key, value) in [
        (2, "bold", Value::Bool(true)),
        (2, "font_size", Value::Number(17.)),
        (3, "font_size", Value::Number(25.)),
        (2, "fill", Value::Color([255, 0, 0, 255])),
        (3, "fill", Value::Color([0, 0, 255, 255])),
    ] {
        e.set_control_property(id, key, value).unwrap();
    }
    e.select(2, false);
    e.select(3, true);
    e
}
fn field(e: &Editor, key: &str) -> SelectionProperty {
    e.selection_properties()
        .into_iter()
        .find(|p| p.key == key)
        .unwrap()
}
fn apply(e: &mut Editor, key: &str, value: Value) {
    e.begin("Edit shared property");
    e.set_selection_property(key, value).unwrap();
    e.commit();
}
#[test]
fn first_selected_value_and_mixed_flags_follow_click_order_and_history() {
    let mut e = specimen();
    assert_eq!(field(&e, "font_size").value, Value::Number(17.));
    assert!(field(&e, "font_size").mixed);
    e.select(3, false);
    e.select(2, true);
    assert_eq!(field(&e, "font_size").value, Value::Number(25.));
    assert_eq!(field(&e, "bold").value, Value::Bool(false));
    let before = e.document.clone();
    apply(&mut e, "bold", Value::Bool(true));
    assert!(!field(&e, "bold").mixed);
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(e.ordered_selection(), [3, 2]);
    assert_eq!(field(&e, "bold").value, Value::Bool(false));
    e.redo();
    assert_eq!(e.ordered_selection(), [3, 2]);
    e.select_all();
    assert_eq!(e.ordered_selection(), [2, 3, 4, 5]);
    e.select(3, true);
    e.select(3, true);
    assert_eq!(e.ordered_selection(), [2, 4, 5, 3]);
}
#[test]
fn shared_font_color_and_alignment_include_locked_targets_with_one_exact_undo() {
    let mut e = specimen();
    e.current_mut().content.nodes[1].locked = true;
    for (key, value) in [
        ("font_size", Value::Number(20.)),
        ("align", Value::Choice("right".into())),
        ("fill", Value::Color([102, 51, 153, 255])),
    ] {
        let before = e.document.clone();
        apply(&mut e, key, value.clone());
        let after = e.document.clone();
        for id in [2, 3] {
            assert_eq!(e.control_properties(id).unwrap().1.get(key), Some(&value));
        }
        assert!(e.nodes()[1].locked);
        e.undo();
        assert_eq!(e.document, before);
        e.redo();
        assert_eq!(e.document, after);
        assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
        e.undo();
    }
}
#[test]
fn heterogeneous_state_union_applies_only_compatible_targets() {
    let mut e = specimen();
    e.select(2, false);
    e.select(4, true);
    assert!(field(&e, "state").choices.contains(&"selected"));
    let before = e.document.clone();
    apply(&mut e, "state", Value::Choice("selected".into()));
    assert_eq!(
        e.control_properties(2).unwrap().1.get("state"),
        Some(&Value::Choice("selected".into()))
    );
    assert_eq!(e.nodes()[2], before.pages[0].content.nodes[2]);
    e.undo();
    assert_eq!(e.document, before);
    assert!(
        e.set_selection_property("state", Value::Choice("impossible".into()))
            .is_err()
    );
    assert_eq!(e.document, before);
}
#[test]
fn main_color_maps_to_each_controls_own_color_and_resets_atomically() {
    let mut e = specimen();
    e.select(2, false);
    e.select(5, true);
    let p = field(&e, "main_color");
    assert_eq!(p.targets, [(2, "fill"), (5, "text_color")]);
    let before = e.document.clone();
    let color = Value::Color([102, 51, 153, 255]);
    apply(&mut e, "main_color", color.clone());
    for (id, key) in p.targets {
        assert_eq!(e.control_properties(id).unwrap().1.get(key), Some(&color));
    }
    let after = e.document.clone();
    e.begin("Reset colors");
    e.reset_selection_property("main_color").unwrap();
    e.commit();
    e.undo();
    assert_eq!(e.document, after);
    e.undo();
    assert_eq!(e.document, before);
}
#[test]
fn unsupported_selection_hides_shared_fields_and_invalid_write_is_atomic() {
    let mut e = specimen();
    let before = e.document.clone();
    assert!(
        e.set_selection_property("font_size", Value::Number(f64::NAN))
            .is_err()
    );
    assert_eq!(e.document, before);
    e.current_mut().content.nodes[1].content = NodeContent::Control {
        kind: "browser".into(),
        size: Size::default(),
    };
    let fields = e.selection_properties();
    assert!(!fields.iter().any(|p| p.key == "bold"));
    assert!(!fields.iter().any(|p| p.key == "state"));
    assert!(fields.iter().any(|p| p.key == "fill"));
}
#[test]
fn icon_sizes_skip_empty_targets_rotation_is_relative_and_removal_preserves_sizes() {
    let mut e = specimen();
    let icon = Value::Icon(IconSource::Catalog("star-solid".into()));
    e.set_control_property(2, "icon", icon.clone()).unwrap();
    e.set_control_property(2, "icon_rotation", Value::Choice("90".into()))
        .unwrap();
    let before = e.document.clone();
    apply(&mut e, "icon_size", Value::Choice("48".into()));
    assert_eq!(
        e.control_properties(2).unwrap().1.get("icon_size"),
        Some(&Value::Choice("48".into()))
    );
    assert_eq!(e.nodes()[1], before.pages[0].content.nodes[1]);
    e.undo();
    assert_eq!(e.document, before);
    apply(&mut e, "icon", icon);
    e.begin("Rotate icons");
    e.rotate_selection_icons().unwrap();
    e.commit();
    assert_eq!(
        e.control_properties(2).unwrap().1.get("icon_rotation"),
        Some(&Value::Choice("180".into()))
    );
    assert_eq!(
        e.control_properties(3).unwrap().1.get("icon_rotation"),
        Some(&Value::Choice("90".into()))
    );
    e.undo();
    apply(&mut e, "icon", Value::Null);
    assert_eq!(field(&e, "icon").value, Value::Null);
}
#[test]
fn component_local_formatting_keeps_sources_and_unselected_content_unchanged() {
    let mut e = specimen();
    e.group();
    let definition = e.create_component("Buttons".into()).unwrap();
    let instance = *e.selection.iter().next().unwrap();
    e.enter(instance);
    e.select_all();
    let before = e.document.clone();
    apply(&mut e, "font_size", Value::Number(20.));
    assert_eq!(e.document.definitions, before.definitions);
    assert_eq!(e.document.libraries, before.libraries);
    assert_eq!(
        e.document.component_source(definition).unwrap(),
        before.component_source(definition).unwrap()
    );
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
}

#[test]
fn tab_alignment_does_not_join_text_alignment_and_border_editors_keep_their_family() {
    fn pair(a: &str, b: &str) -> Editor {
        let mut p = Project {
            next_id: 10,
            ..Project::default()
        };
        p.pages[0].content.nodes = vec![
            Node::control(2, a, Point::new(0., 0.), Size::fixed(450., 150.)),
            Node::control(3, b, Point::new(0., 200.), Size::fixed(450., 150.)),
        ];
        let mut e = Editor::new(p);
        e.select_all();
        e
    }
    let mut e = pair("tabs", "paragraph");
    assert_eq!(field(&e, "align").targets, vec![(3, "align")]);
    apply(&mut e, "align", Value::Choice("right".into()));
    assert_eq!(
        e.control_properties(2).unwrap().1["align"],
        Value::Choice("left".into())
    );
    assert_eq!(
        e.control_properties(3).unwrap().1["align"],
        Value::Choice("right".into())
    );
    assert_eq!(field(&pair("tabs", "tabs"), "align").targets.len(), 2);
    assert!(
        !pair("rectangle", "tabs")
            .selection_properties()
            .iter()
            .any(|f| f.key == "border")
    );
    assert!(
        pair("tabs", "table")
            .selection_properties()
            .iter()
            .any(|f| f.key == "border")
    );
    assert!(
        field(&pair("rectangle", "browser"), "border")
            .choices
            .contains(&"torn_bottom")
    );
}
