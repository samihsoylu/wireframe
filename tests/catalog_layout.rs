// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use eframe::egui;
use std::collections::{BTreeMap, HashSet};
use wireframe::{
    catalog::{self, Context, PropertyType},
    document::*,
    layout::{self, TextMeasurer},
};

#[test]
fn catalog_has_unique_identities_valid_defaults_and_all_migrated_types() {
    assert_eq!(catalog::CONTROLS.len(), 86);
    let mut ids = HashSet::new();
    for spec in catalog::CONTROLS.iter() {
        assert!(ids.insert(spec.id));
        let mut keys = HashSet::new();
        for prop in &spec.properties {
            assert!(keys.insert(prop.key), "{} {}", spec.id, prop.key);
            prop.validate(&prop.default_value(&Context::default()))
                .unwrap();
        }
    }
    let mut old = wireframe::model::Document::default();
    for kind in wireframe::model::Kind::ALL {
        let id = old.allocate_id();
        old.pages[0]
            .controls
            .push(wireframe::model::Control::new(id, kind, 0., 0.));
    }
    let p = Project::try_from(old).unwrap();
    for leaf in p.pages[0].content.leaves().unwrap() {
        if let NodeContent::Control { kind, .. } = &leaf.node.content {
            catalog::get(kind)
                .unwrap()
                .effective(leaf.node, &Context::default())
                .unwrap();
        }
    }
}
#[test]
fn search_prefers_exact_names_and_aliases_and_handles_multiple_words() {
    assert_eq!(catalog::search("button")[0].id, "button");
    assert_eq!(catalog::search("table")[0].id, "table");
    assert_eq!(catalog::search("check box")[0].id, "checkbox");
    assert_eq!(catalog::search("  TEXT INPUT  ")[0].id, "text_input");
    assert!(
        catalog::search("vertical slider")
            .iter()
            .any(|s| s.id == "vertical_slider")
    );
    assert!(catalog::search("no such control xyz").is_empty());
}
#[test]
fn property_edits_normalize_defaults_but_component_overrides_use_inherited_values() {
    let spec = catalog::get("button").unwrap();
    let context = Context::default();
    let mut node = spec.create(2, Point::default());
    spec.set_property(&mut node, "font_size", Value::Number(24.), &context)
        .unwrap();
    assert_eq!(
        spec.effective(&node, &context).unwrap()["font_size"],
        Value::Number(24.)
    );
    spec.set_property(&mut node, "font_size", Value::Number(13.), &context)
        .unwrap();
    assert!(!node.properties.contains_key("font_size"));
    let mut changed = context;
    changed.font_size = 18.;
    assert_eq!(
        spec.effective(&node, &changed).unwrap()["font_size"],
        Value::Number(18.)
    );
    let mut inherited = spec.defaults(&context);
    inherited.insert("font_size".into(), Value::Number(24.));
    let mut patch = Properties::new();
    spec.set_override(&mut patch, "font_size", Value::Number(13.), &inherited)
        .unwrap();
    assert_eq!(patch["font_size"], Value::Number(13.));
    spec.set_override(&mut patch, "font_size", Value::Number(24.), &inherited)
        .unwrap();
    assert!(patch.is_empty());
    let before = node.clone();
    assert!(
        spec.set_property(
            &mut node,
            "state",
            Value::Choice("indeterminate".into()),
            &context
        )
        .is_err()
    );
    assert_eq!(node, before);
    assert!(
        spec.set_property(&mut node, "font_size", Value::Bool(true), &context)
            .is_err()
    );
    assert_eq!(node, before);
}
#[test]
fn state_support_and_legacy_checked_values_remain_distinct() {
    let spec = catalog::get("checkbox").unwrap();
    let context = Context::default();
    let mut node = spec.create(2, Point::default());
    let PropertyType::Choice(states) = spec.property("state").unwrap().kind else {
        panic!()
    };
    assert_eq!(states.len(), 6);
    node.properties.insert("checked".into(), Value::Bool(true));
    assert_eq!(
        spec.effective(&node, &context).unwrap()["state"],
        Value::Choice("selected".into())
    );
    spec.set_property(
        &mut node,
        "state",
        Value::Choice("disabled_indeterminate".into()),
        &context,
    )
    .unwrap();
    assert!(!node.properties.contains_key("checked"));
    spec.reset_property(&mut node, "state").unwrap();
    assert_eq!(
        spec.effective(&node, &context).unwrap()["state"],
        Value::Choice("normal".into())
    );
    let button = catalog::get("button").unwrap();
    assert!(
        button
            .property("state")
            .unwrap()
            .validate(&Value::Choice("disabled_indeterminate".into()))
            .is_err()
    );
}
#[test]
fn schema_errors_block_saving_while_unknown_values_are_preserved() {
    let mut p = Project {
        next_id: 3,
        ..Project::default()
    };
    let mut node = catalog::get("checkbox")
        .unwrap()
        .create(2, Point::default());
    node.properties
        .insert("future_setting".into(), Value::Text("retained".into()));
    p.pages[0].content.nodes.push(node);
    assert_eq!(decode(&p.encode().unwrap()).unwrap(), p);
    p.pages[0].content.nodes[0]
        .properties
        .insert("state".into(), Value::Choice("invalid".into()));
    assert!(p.encode().is_err());
}
#[test]
fn resize_axes_are_independent_of_persisted_dimensions() {
    assert_eq!(
        catalog::get("label")
            .unwrap()
            .resized((50., 20.), (100., 90.)),
        (100., 20.)
    );
    assert_eq!(
        catalog::get("vertical_slider")
            .unwrap()
            .resized((12., 33.), (100., 90.)),
        (12., 90.)
    );
    assert_eq!(
        catalog::get("button")
            .unwrap()
            .resized((50., 20.), (100., 90.)),
        (100., 90.)
    );
    assert_eq!(
        catalog::get("icon")
            .unwrap()
            .resized((32., 32.), (100., 90.)),
        (32., 32.)
    );
    assert_eq!(
        catalog::get("button")
            .unwrap()
            .resized((50., 20.), (f32::NAN, -10.)),
        (50., 1.)
    );
}
fn with_fonts(test: impl FnOnce(&layout::EguiMeasurer<'_>)) {
    let context = egui::Context::default();
    let mut test = Some(test);
    let output = context.run_ui(egui::RawInput::default(), |ui| {
        let context = ui.ctx();
        if let Some(test) = test.take() {
            test(&layout::EguiMeasurer {
                context,
                regular: egui::FontFamily::Proportional,
                bold: egui::FontFamily::Proportional,
            });
        }
    });
    output.drop_without_applying_deltas();
}
#[test]
fn real_font_metrics_distinguish_glyph_width_and_wrap_without_mutating_text() {
    with_fonts(|fonts| {
        let style = layout::TextStyle {
            size: 13.,
            bold: false,
            italic: false,
        };
        let narrow = fonts.measure("iiii", style, None);
        let wide = fonts.measure("WWWW", style, None);
        assert!(wide.width > narrow.width * 2.);
        let text = "Several words that should wrap across lines";
        assert!(
            fonts.measure(text, style, Some(45.)).rows
                > fonts.measure(text, style, Some(300.)).rows
        );
        let mut p = Project {
            next_id: 3,
            ..Project::default()
        };
        let mut node = catalog::get("paragraph")
            .unwrap()
            .create(2, Point::new(50., 50.));
        node.properties
            .insert("text".into(), Value::Text(text.into()));
        node.content = NodeContent::Control {
            kind: "paragraph".into(),
            size: Size {
                width: Dimension::Fixed(80.),
                height: Dimension::Auto,
            },
        };
        p.pages[0].content.nodes.push(node);
        let before = p.clone();
        let engine = layout::Engine {
            project: &p,
            measurer: fonts,
            context: Context::default(),
        };
        let result = engine.layout(&p.pages[0].content).unwrap();
        assert_eq!(result.nodes[0].bounds.width, 80.);
        assert!(result.nodes[0].bounds.height > 20.);
        assert_eq!(p, before);
    });
}
#[test]
fn group_layout_tracks_child_edits_overrides_and_clip_intersection() {
    with_fonts(|fonts| {
        let mut p = Project {
            next_id: 20,
            ..Project::default()
        };
        let mut leaf = catalog::get("label")
            .unwrap()
            .create(2, Point::new(5., 10.));
        leaf.properties
            .insert("text".into(), Value::Text("Short".into()));
        p.definitions.push(Definition {
            id: 10,
            name: "Shared".into(),
            content: Content {
                nodes: vec![leaf],
                ..Content::default()
            },
        });
        let mut node = Node::group(3, Point::new(100., 200.), vec![]);
        node.content = NodeContent::Instance {
            definition: 10,
            overrides: vec![Override {
                path: vec![2],
                patch: Patch {
                    properties: BTreeMap::from([(
                        "text".into(),
                        Value::Text("Much longer override".into()),
                    )]),
                    ..Patch::default()
                },
            }],
        };
        node.crop = Some(Crop {
            x: 0.,
            y: 0.,
            width: 0.5,
            height: 1.,
        });
        p.pages[0].content.nodes.push(node);
        let engine = layout::Engine {
            project: &p,
            measurer: fonts,
            context: Context::default(),
        };
        let result = engine.layout(&p.pages[0].content).unwrap();
        let root = &result.nodes[0];
        assert_eq!(root.bounds.x, 105.);
        assert_eq!(root.bounds.y, 210.);
        assert_eq!(root.bounds.width, root.uncropped.width * 0.5);
        let leaves = result.leaves();
        assert_eq!(leaves.len(), 1);
        assert_eq!(leaves[0].0.path, vec![3, 2]);
        assert_eq!(leaves[0].1.width, root.bounds.width);
        let original_width = fonts
            .measure(
                "Short",
                layout::TextStyle {
                    size: 13.,
                    bold: false,
                    italic: false,
                },
                None,
            )
            .width;
        assert!(root.uncropped.width > original_width);
    });
}
#[test]
fn unsupported_automatic_layout_is_explicit_not_a_fake_completed_renderer() {
    with_fonts(|fonts| {
        let p = Project::default();
        let mut node = catalog::get("map").unwrap().create(2, Point::default());
        if let NodeContent::Control { size, .. } = &mut node.content {
            size.width = Dimension::Auto;
        }
        let content = Content {
            nodes: vec![node],
            ..Content::default()
        };
        let error = layout::Engine {
            project: &p,
            measurer: fonts,
            context: Context::default(),
        }
        .layout(&content)
        .unwrap_err();
        assert!(error.contains("not implemented for map"));
    });
}

#[test]
fn inspector_fields_and_legacy_flags_do_not_leak_between_control_types() {
    let checkbox = catalog::get("checkbox").unwrap();
    assert!(checkbox.property("text_color").unwrap().visible);
    assert!(!checkbox.property("fill").unwrap().visible);
    assert!(!checkbox.property("checked").unwrap().visible);
    let button = catalog::get("button").unwrap();
    assert!(button.property("fill").unwrap().visible);
    let mut node = button.create(2, Point::default());
    node.properties.insert("checked".into(), Value::Bool(true));
    assert_eq!(
        button.effective(&node, &Context::default()).unwrap()["state"],
        Value::Choice("normal".into())
    );
}

#[test]
fn structured_grid_and_formatted_text_drive_automatic_layout() {
    with_fonts(|fonts| {
        let p = Project::default();
        let mut node = catalog::get("table").unwrap().create(2, Point::default());
        node.properties.insert(
            "text".into(),
            Value::Text("Title, Count\nA, 42\nB, [x]".into()),
        );
        node.properties
            .insert("row_height".into(), Value::Number(40.));
        let content = Content {
            nodes: vec![node],
            ..Content::default()
        };
        let engine = layout::Engine {
            project: &p,
            measurer: fonts,
            context: Context::default(),
        };
        let first = engine.layout(&content).unwrap();
        assert!(first.nodes[0].bounds.height >= 80.);
        let mut changed = content.clone();
        changed.nodes[0].properties.insert(
            "text".into(),
            Value::Text("Title, Count\n{size:48}Wide body text{size}, 42\nB, [x]".into()),
        );
        let second = engine.layout(&changed).unwrap();
        assert!(second.nodes[0].bounds.width > first.nodes[0].bounds.width);
        assert!(second.nodes[0].bounds.height > first.nodes[0].bounds.height);
        assert_eq!(
            content.nodes[0].properties["text"],
            Value::Text("Title, Count\nA, 42\nB, [x]".into())
        );
    });
}
