// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    catalog, document::*, model::Kind, render::Drawing, scene::Primitive, storage::ProjectStore,
    workspace::Editor,
};

#[test]
fn form_insertion_defaults_and_style_specific_dimensions_survive_reopen() {
    for sketch in [true, false] {
        let mut e = Editor::default();
        e.document.settings.sketch = sketch;
        for kind in [Kind::Button, Kind::TextInput, Kind::TextArea] {
            e.add(kind, 10., 20.);
        }
        let controls = e.current().controls;
        assert_eq!(
            controls.iter().map(|c| c.kind).collect::<Vec<_>>(),
            [Kind::Button, Kind::TextInput, Kind::TextArea]
        );
        assert_eq!(controls[0].bounds.h, if sketch { 27. } else { 24. });
        assert_eq!(
            (controls[1].bounds.w, controls[1].bounds.h),
            (79., if sketch { 27. } else { 24. })
        );
        assert!(controls[1].text.is_empty());
        assert_eq!(
            controls[0].fill,
            if sketch {
                [255, 255, 255]
            } else {
                [221, 221, 221]
            }
        );
        let button = controls[0].id;
        e.set_control_property(button, "fill", Value::Color([10, 20, 30, 255]))
            .unwrap();
        assert_eq!(e.current().controls[0].fill, [10, 20, 30]);
        e.reset_control_property(button, "fill").unwrap();
        assert_eq!(e.current().controls[0].fill, controls[0].fill);
        assert_eq!((controls[2].bounds.w, controls[2].bounds.h), (200., 140.));
        let id = controls[1].id;
        e.begin("Enter input text");
        e.set_control_property(id, "text", Value::Text("Account".into()))
            .unwrap();
        e.set_control_property(id, "font_size", Value::Number(24.))
            .unwrap();
        e.commit();
        assert_eq!(
            e.current().controls[1].bounds.h,
            if sketch { 38. } else { 35. }
        );
        let bytes = e.document.encode().unwrap();
        let reopened = Editor::new(wireframe::document::decode(&bytes).unwrap());
        assert_eq!(reopened.current().controls, e.current().controls);
        e.undo();
        assert!(e.current().controls[1].text.is_empty());
    }
}

#[test]
fn form_states_change_border_and_fill_without_moving_the_text() {
    for sketch in [true, false] {
        let accent = if sketch {
            [149, 188, 242]
        } else {
            [151, 189, 240]
        };
        for kind in [Kind::Button, Kind::TextInput, Kind::TextArea] {
            let mut e = Editor::default();
            e.document.settings.sketch = sketch;
            e.add(kind, 20., 30.);
            let id = e.nodes()[0].id;
            e.set_control_property(id, "text", Value::Text("Account".into()))
                .unwrap();
            let mut baseline = None;
            for state in ["normal", "focused", "disabled", "selected"] {
                if state == "selected" && kind != Kind::Button {
                    continue;
                }
                e.set_control_property(id, "state", Value::Choice(state.into()))
                    .unwrap();
                let scene = e.rendered_scene(1).unwrap();
                let color = match state {
                    "focused" => accent,
                    "disabled" if kind == Kind::Button && !sketch => [204, 204, 204],
                    "disabled" => [128, 128, 139],
                    _ if sketch => [0, 0, 0],
                    _ => [51, 51, 51],
                };
                let border = scene
                    .items
                    .iter()
                    .find_map(|i| match &i.drawing {
                        Drawing::Primitive(Primitive::Line { color, .. }) => Some(*color),
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(border, color, "{kind:?} {state}");
                let fill = scene
                    .items
                    .iter()
                    .find_map(|i| match &i.drawing {
                        Drawing::Primitive(Primitive::Rect { fill, .. }) => Some(*fill),
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(
                    fill,
                    if state == "selected" {
                        accent
                    } else if kind == Kind::Button && !sketch {
                        [221, 221, 221]
                    } else {
                        [255, 255, 255]
                    }
                );
                let glyph = scene
                    .items
                    .iter()
                    .find_map(|i| match &i.drawing {
                        Drawing::Glyph(g) => Some(g),
                        _ => None,
                    })
                    .unwrap();
                let pos = (glyph.x, glyph.baseline);
                assert_eq!(*baseline.get_or_insert(pos), pos);
                if state == "disabled" {
                    assert_eq!(glyph.color, [128, 128, 139, 255]);
                }
            }
        }
    }
}

#[test]
fn dark_button_fill_uses_readable_text_and_selected_state_uses_its_own_fill() {
    let mut e = Editor::default();
    e.add(Kind::Button, 10., 20.);
    let id = e.nodes()[0].id;
    e.set_control_property(id, "fill", Value::Color([0, 0, 0, 255]))
        .unwrap();
    let text_color = |e: &Editor| {
        e.rendered_scene(1)
            .unwrap()
            .items
            .iter()
            .find_map(|i| match &i.drawing {
                Drawing::Glyph(g) => Some(g.color),
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(text_color(&e), [255, 255, 255, 255]);
    e.set_control_property(id, "state", Value::Choice("selected".into()))
        .unwrap();
    assert_eq!(text_color(&e), [0, 0, 0, 255]);
}

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
    let mut node = catalog::get("app_bar").unwrap().create(2, Point::default());
    node.content = NodeContent::Control {
        kind: "app_bar".into(),
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
            let symbols = scene
                .items
                .iter()
                .filter_map(|i| match &i.drawing {
                    Drawing::Icon {
                        bounds, id, color, ..
                    } => Some((bounds, id.as_str(), color)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(symbols.len(), 1);
            let (symbol, id, color) = symbols[0];
            assert_eq!(*color, [ink[0], ink[1], ink[2], 255]);
            assert_eq!(
                (symbol.w, symbol.h),
                if kind == "checkbox" {
                    (16., 16.)
                } else {
                    (13., 13.)
                }
            );
            let mixed = state.contains("indeterminate");
            let selected = state.contains("selected");
            let expected = if kind == "checkbox" {
                if mixed {
                    "square-minus-regular"
                } else if selected {
                    "square-check-regular"
                } else {
                    "square-regular"
                }
            } else if selected {
                "circle-dot-regular"
            } else {
                "circle-regular"
            };
            assert_eq!(id, expected);
            let bars = scene
                .items
                .iter()
                .filter_map(|i| match &i.drawing {
                    Drawing::Primitive(Primitive::Line {
                        points,
                        color,
                        width,
                    }) if points
                        .iter()
                        .all(|p| p[0] >= symbol.x && p[0] <= symbol.x + symbol.w) =>
                    {
                        Some((points, color, width))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            if kind == "radio" && mixed {
                assert_eq!(bars.len(), 1);
                assert_eq!(*bars[0].1, ink);
                assert_eq!(*bars[0].2, 13. * 3. / 32.);
            } else {
                assert!(bars.is_empty());
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
