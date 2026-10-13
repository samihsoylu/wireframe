// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    catalog, document::*, model::Kind, render::Drawing, scene::Primitive, storage::ProjectStore,
    workspace::Editor,
};

#[test]
fn typed_state_edits_clear_legacy_flags_and_round_trip_with_undo() {
    let mut e = Editor::default();
    e.add(Kind::Checkbox, 20., 30.);
    let id = e.nodes()[0].id;
    e.document.pages[0].content.nodes[0]
        .properties
        .insert("checked".into(), Value::Bool(true));
    e.document.pages[0].content.nodes[0]
        .properties
        .insert("future_property".into(), Value::Text("Retain me".into()));
    let before = e.document.clone();
    e.begin("Change choice state");
    e.set_control_property(id, "state", Value::Choice("disabled_selected".into()))
        .unwrap();
    e.commit();
    assert!(e.current().controls[0].checked);
    assert!(!e.nodes()[0].properties.contains_key("checked"));
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("states.wireframe");
    ProjectStore.save(&path, &e.document).unwrap();
    let reopened = Editor::new(ProjectStore.load(&path).unwrap());
    assert_eq!(reopened.document, after);
    e.begin("Reset state");
    e.reset_control_property(id, "state").unwrap();
    e.commit();
    assert!(!e.current().controls[0].checked);
    assert!(!e.nodes()[0].properties.contains_key("state"));
    assert_eq!(
        e.nodes()[0].properties["future_property"],
        Value::Text("Retain me".into())
    );
}

#[test]
fn property_edits_respect_nested_context_lock_and_validation() {
    let mut e = Editor::default();
    e.add(Kind::Checkbox, 20., 30.);
    e.add(Kind::Label, 150., 30.);
    e.select_all();
    e.group();
    let group = *e.selection.iter().next().unwrap();
    assert!(e.enter(group));
    let id = e.nodes()[0].id;
    let before = e.document.clone();
    assert!(
        e.set_control_property(id, "state", Value::Choice("invalid".into()))
            .is_err()
    );
    assert_eq!(e.document, before);
    e.begin("Edit nested state");
    e.set_control_property(id, "state", Value::Choice("indeterminate".into()))
        .unwrap();
    e.commit();
    assert_eq!(
        e.control_properties(id).unwrap().1["state"],
        Value::Choice("indeterminate".into())
    );
    e.select(id, false);
    e.lock(true);
    let locked = e.document.clone();
    assert!(
        e.set_control_property(id, "text", Value::Text("Changed".into()))
            .is_err()
    );
    assert_eq!(e.document, locked);
}

#[test]
fn inherited_font_reset_and_auto_size_use_project_context_and_undo() {
    let mut e = Editor::default();
    e.document.settings.font_size = 21.;
    e.add(Kind::Checkbox, 20., 30.);
    let id = e.nodes()[0].id;
    assert_eq!(
        e.control_properties(id).unwrap().1["font_size"],
        Value::Number(21.)
    );
    assert_eq!(e.current().controls[0].font_size, 21.);
    e.begin("Font");
    e.set_control_property(id, "font_size", Value::Number(30.))
        .unwrap();
    e.commit();
    e.begin("Reset font");
    e.reset_control_property(id, "font_size").unwrap();
    e.commit();
    assert!(!e.nodes()[0].properties.contains_key("font_size"));
    assert_eq!(e.current().controls[0].font_size, 21.);
    let automatic = e.current().controls[0].bounds;
    e.edit("Resize", |e| {
        let mut c = e.current().controls[0].clone();
        c.bounds.w += 80.;
        e.update_control(c);
    });
    let fixed = e.document.clone();
    e.begin("Auto-size");
    e.auto_size_control(id).unwrap();
    e.commit();
    assert_eq!(e.current().controls[0].bounds, automatic);
    e.undo();
    assert_eq!(e.document, fixed);
}

#[test]
fn unsupported_auto_layout_rolls_back_requested_dimensions() {
    let mut p = Project {
        next_id: 3,
        ..Project::default()
    };
    let mut node = catalog::get("accordion")
        .unwrap()
        .create(2, Point::default());
    node.content = NodeContent::Control {
        kind: "accordion".into(),
        size: Size::fixed(200., 100.),
    };
    p.pages[0].content.nodes.push(node);
    let mut e = Editor::new(p);
    let before = e.document.clone();
    e.begin("Auto-size");
    assert!(e.auto_size_control(2).is_err());
    e.commit();
    assert_eq!(e.document, before);
    assert!(e.undo_name().is_none());
}

#[test]
fn all_choice_states_draw_their_mark_and_disabled_ink_without_changing_geometry() {
    for kind in ["checkbox", "radio"] {
        let mut normal_bounds = None;
        let mut normal_count = 0;
        for state in [
            "normal",
            "selected",
            "disabled",
            "disabled_selected",
            "indeterminate",
            "disabled_indeterminate",
        ] {
            let mut p = Project {
                next_id: 3,
                ..Project::default()
            };
            let mut n = catalog::get(kind).unwrap().create(2, Point::new(40., 50.));
            n.properties
                .insert("state".into(), Value::Choice(state.into()));
            n.properties
                .insert("text".into(), Value::Text("[Choice](page:9)".into()));
            p.pages[0].content.nodes.push(n);
            let e = Editor::new(p);
            let scene = e.rendered_scene(1).unwrap();
            let b = e.current().controls[0].bounds;
            assert_eq!(*normal_bounds.get_or_insert(b), b);
            let ink = if state.contains("disabled") {
                [128, 128, 139]
            } else {
                [0, 0, 0]
            };
            let outlines = scene
                .items
                .iter()
                .filter_map(|i| match &i.drawing {
                    Drawing::Primitive(Primitive::Line { points, color, .. })
                        if points.len() > 4 =>
                    {
                        Some((points, color))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(outlines.len(), 1);
            assert_eq!(*outlines[0].1, ink);
            let width = outlines[0]
                .0
                .iter()
                .map(|p| p[0])
                .fold(f32::NEG_INFINITY, f32::max)
                - outlines[0]
                    .0
                    .iter()
                    .map(|p| p[0])
                    .fold(f32::INFINITY, f32::min);
            assert_eq!(width, if kind == "checkbox" { 16. } else { 13. });
            let marks = scene
                .items
                .iter()
                .filter(|i| matches!(i.drawing, Drawing::Primitive(_)))
                .count();
            if state == "normal" {
                normal_count = marks;
            }
            if state.contains("selected") || state.contains("indeterminate") {
                assert!(marks > normal_count);
            }
            let glyphs = scene
                .items
                .iter()
                .filter_map(|i| {
                    if let Drawing::Glyph(g) = &i.drawing {
                        Some(g)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            assert_eq!(glyphs[0].x, b.x + 20.);
            if state.contains("disabled") {
                assert!(glyphs.iter().all(|g| g.color == [128, 128, 139, 255]));
            }
            assert_eq!(scene.links.len(), 1);
        }
    }
}
