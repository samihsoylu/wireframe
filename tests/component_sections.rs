// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    workspace::{Editor, PropertySection},
};

fn specimen() -> Editor {
    specimen_of("button")
}
fn specimen_of(kind: &str) -> Editor {
    let mut p = Project {
        next_id: 10,
        ..Project::default()
    };
    p.pages[0].content.nodes = (0..2)
        .map(|i| {
            Node::control(
                2 + i,
                kind,
                Point::new(50. + i as f32 * 210., 70.),
                Size::fixed(160., 60.),
            )
        })
        .collect();
    let mut e = Editor::new(p);
    for id in [2, 3] {
        e.set_control_property(id, "text", Value::Text(format!("Source {}", id - 1)))
            .unwrap();
        e.set_control_property(id, "font_size", Value::Number(17.))
            .unwrap();
        e.set_control_property(id, "bold", Value::Bool(true))
            .unwrap();
        e.set_control_property(id, "fill", Value::Color([238, 238, 238, 255]))
            .unwrap();
    }
    e.select_all();
    e.group();
    let source = e.create_component("Two buttons".into()).unwrap();
    let instance = *e.selection.iter().next().unwrap();
    e.insert_component(source, 50., 230.).unwrap();
    e.enter(instance);
    e.select_all();
    for (id, key, value) in [
        (2, "text", Value::Text("Local One".into())),
        (2, "font_size", Value::Number(25.)),
        (2, "bold", Value::Bool(false)),
        (2, "italic", Value::Bool(true)),
        (2, "fill", Value::Color([255, 170, 0, 255])),
        (3, "text", Value::Text("Local Two".into())),
        (3, "font_size", Value::Number(21.)),
        (3, "underline", Value::Bool(true)),
        (3, "fill", Value::Color([153, 204, 255, 255])),
    ] {
        e.set_control_property(id, key, value).unwrap();
    }
    e
}
#[test]
fn section_reset_restores_text_and_formatting_but_preserves_colors_other_instance_and_source() {
    let mut e = specimen();
    let before = e.document.clone();
    e.reset_component_section(&[2, 3], PropertySection::Text)
        .unwrap();
    let after = e.document.clone();
    for id in [2, 3] {
        let p = e.control_properties(id).unwrap().1;
        assert_eq!(p["text"], Value::Text(format!("Source {}", id - 1)));
        assert_eq!(p["font_size"], Value::Number(17.));
        assert_eq!(p["bold"], Value::Bool(true));
        assert_eq!(p["italic"], Value::Bool(false));
        assert_eq!(p["underline"], Value::Bool(false));
        let local = e.explicit_control_properties(id);
        assert_eq!(local.len(), 1);
        assert!(local.contains_key("fill"));
    }
    assert_eq!(after.definitions, before.definitions);
    assert_eq!(after.libraries, before.libraries);
    assert_eq!(
        after.pages[0].content.nodes[1],
        before.pages[0].content.nodes[1]
    );
    assert!(!e.has_section_overrides(&[2, 3], PropertySection::Text));
    assert!(e.has_section_overrides(&[2, 3], PropertySection::Color));
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
}
#[test]
fn separate_color_reset_keeps_text_and_noop_text_reset_preserves_redo() {
    let mut e = specimen();
    e.reset_component_section(&[2, 3], PropertySection::Text)
        .unwrap();
    let text = e.document.clone();
    e.reset_component_section(&[3, 2], PropertySection::Color)
        .unwrap();
    let all = e.document.clone();
    for id in [2, 3] {
        assert!(e.explicit_control_properties(id).is_empty());
    }
    e.undo();
    assert_eq!(e.document, text);
    e.reset_component_section(&[2, 3], PropertySection::Text)
        .unwrap();
    assert_eq!(e.document, text);
    e.redo();
    assert_eq!(e.document, all);
}
#[test]
fn section_target_validation_is_atomic_and_ordinary_controls_are_rejected() {
    let mut e = specimen();
    let before = e.document.clone();
    for ids in [&[][..], &[2, 2], &[2, 999]] {
        assert!(
            e.reset_component_section(ids, PropertySection::Text)
                .is_err()
        );
        assert_eq!(e.document, before);
    }
    e.leave();
    let before = e.document.clone();
    assert!(
        e.reset_component_section(&[2], PropertySection::Text)
            .is_err()
    );
    assert_eq!(e.document, before);
}
#[test]
fn partial_override_and_locked_control_reset_preserve_geometry_and_other_fields() {
    let mut e = specimen();
    e.reset_component_section(&[3], PropertySection::Text)
        .unwrap();
    e.set_control_property(2, "text_color", Value::Color([200, 0, 0, 255]))
        .unwrap();
    // Lock is local editing state and must survive an appearance reset.
    e.select(2, false);
    e.lock(true);
    let before = e.nodes()[0].clone();
    e.reset_component_section(&[2, 3], PropertySection::Text)
        .unwrap();
    let after = e.nodes()[0].clone();
    assert_eq!(before.origin, after.origin);
    assert_eq!(before.content, after.content);
    assert_eq!(before.locked, after.locked);
    assert!(e.explicit_control_properties(2).contains_key("text_color"));
    assert!(!e.has_section_overrides(&[2, 3], PropertySection::Text));
}

#[test]
fn color_section_clears_every_supported_color_override_without_resetting_text_or_geometry() {
    let mut covered = std::collections::HashSet::new();
    for kind in ["button", "shape"] {
        let mut e = specimen_of(kind);
        for (key, value) in [
            ("fill", Value::Color([10, 20, 30, 255])),
            ("border_color", Value::Color([40, 50, 60, 255])),
            ("text_color", Value::Color([70, 80, 90, 255])),
            ("background_opacity", Value::Number(0.4)),
        ] {
            if e.control_properties(2).unwrap().0.property(key).is_some() {
                e.set_control_property(2, key, value).unwrap();
                covered.insert(key);
            }
        }
        let before = e.document.clone();
        let node = e.nodes()[0].clone();
        e.reset_component_section(&[2], PropertySection::Color)
            .unwrap();
        let explicit = e.explicit_control_properties(2);
        assert!(
            PropertySection::Color
                .keys()
                .iter()
                .all(|key| !explicit.contains_key(*key))
        );
        assert_eq!(explicit["text"], Value::Text("Local One".into()));
        assert_eq!(e.nodes()[0].origin, node.origin);
        assert_eq!(e.nodes()[0].content, node.content);
        e.undo();
        assert_eq!(e.document, before);
    }
    assert_eq!(covered.len(), 4);
}

fn family_specimen(kind: &str) -> Editor {
    let mut p = Project {
        next_id: 10,
        ..Project::default()
    };
    p.pages[0].content.nodes = (0..2)
        .map(|i| {
            Node::control(
                2 + i,
                kind,
                Point::new(50. + i as f32 * 210., 70.),
                Size::fixed(160., 60.),
            )
        })
        .collect();
    let mut e = Editor::new(p);
    e.select_all();
    e.group();
    let source = e.create_component("Pair".into()).unwrap();
    let instance = *e.selection.iter().next().unwrap();
    e.insert_component(source, 50., 230.).unwrap();
    e.enter(instance);
    e.select_all();
    e
}
#[test]
fn scalar_sections_restore_supported_fields_atomically_and_preserve_other_edits() {
    use PropertySection::*;
    let cases: &[(&str, PropertySection, &[&str])] = &[
        ("horizontal_slider", Slider, &["value", "slider_thumb"]),
        ("vertical_slider", Slider, &["value", "slider_thumb"]),
        (
            "progress",
            Progress,
            &["value", "progress_style", "indeterminate"],
        ),
        ("text_area", Scrollbar, &["value", "vertical_scrollbar"]),
        ("vertical_scrollbar", Scrollbar, &["value"]),
        (
            "phone",
            Phone,
            &[
                "device_model",
                "orientation",
                "device_resizing",
                "background_pattern",
                "transparent_background",
                "status_bar",
            ],
        ),
        (
            "tablet",
            Tablet,
            &[
                "orientation",
                "device_resizing",
                "transparent_background",
                "status_bar",
            ],
        ),
        (
            "mobile_keyboard",
            Keyboard,
            &["keyboard_device", "orientation"],
        ),
        (
            "window",
            Window,
            &[
                "header_height",
                "footer_height",
                "minimize",
                "maximize",
                "close",
                "resize_handle",
            ],
        ),
        (
            "search",
            Search,
            &["shape", "search_icon", "microphone_icon"],
        ),
        ("shape", Shape, &["shape", "shape_rotation"]),
        ("popover", Popover, &["direction", "arrow_position"]),
        ("tooltip", Direction, &["direction"]),
        ("label", TextOrientation, &["text_orientation"]),
        ("date_picker", Date, &["date_mode"]),
        ("tabs", Tabs, &["tab_position"]),
        ("alert", Buttons, &["button_style"]),
        (
            "list",
            List,
            &["header", "alternate_fill", "fill", "row_height"],
        ),
        (
            "table",
            Table,
            &[
                "header",
                "horizontal_lines",
                "vertical_lines",
                "alternate_fill",
                "fill",
                "row_height",
            ],
        ),
        ("icon_label", LabelPosition, &["label_position"]),
        ("button", Menu, &["menu_icon"]),
        ("checkbox", State, &["state"]),
        ("list", Selection, &["selected_index"]),
        ("icon", Icon, &["icon_size", "icon_rotation"]),
    ];
    for &(kind, section, keys) in cases {
        let mut e = family_specimen(kind);
        let inherited = e.control_properties(2).unwrap().1;
        for id in [2, 3] {
            for &key in keys {
                let (spec, values) = e.control_properties(id).unwrap();
                let old = &values[key];
                use wireframe::catalog::PropertyType;
                let value = match (&spec.property(key).unwrap().kind, old) {
                    (PropertyType::Bool, Value::Bool(v)) => Value::Bool(!v),
                    (PropertyType::Number { min, max, .. }, Value::Number(v)) => {
                        Value::Number(if v == min { *max } else { *min })
                    }
                    (PropertyType::Choice(choices), Value::Choice(v)) => {
                        Value::Choice((*choices.iter().find(|c| **c != v).unwrap()).into())
                    }
                    (PropertyType::Color, _) => Value::Color([123, 45, 67, 255]),
                    _ => panic!("Unhandled fixture {kind}/{key}"),
                };
                e.set_control_property(id, key, value)
                    .unwrap_or_else(|error| panic!("{kind}/{key}: {error}"));
            }
            // Unsupported extension metadata must survive section resets too.
            if let NodeContent::Instance { overrides, .. } =
                &mut e.document.pages[0].content.nodes[0].content
            {
                overrides
                    .iter_mut()
                    .find(|o| o.path == [id])
                    .unwrap()
                    .patch
                    .properties
                    .insert("extension_marker".into(), Value::Text("keep".into()));
            }
        }
        let before = e.document.clone();
        assert!(e.has_section_overrides(&[2, 3], section), "{kind}");
        e.reset_component_section(&[3, 2], section).unwrap();
        let after = e.document.clone();
        for id in [2, 3] {
            for &key in keys {
                assert_eq!(
                    e.control_properties(id).unwrap().1[key],
                    inherited[key],
                    "{kind}/{key}"
                );
            }
            assert_eq!(e.explicit_control_properties(id).len(), 1, "{kind}");
        }
        assert!(!e.has_section_overrides(&[2, 3], section));
        assert_eq!(after.definitions, before.definitions, "{kind}");
        assert_eq!(after.libraries, before.libraries, "{kind}");
        assert_eq!(
            after.pages[0].content.nodes[1], before.pages[0].content.nodes[1],
            "{kind}"
        );
        e.undo();
        assert_eq!(e.document, before, "{kind}");
        e.redo();
        assert_eq!(e.document, after, "{kind}");
        assert_eq!(decode(&after.encode().unwrap()).unwrap(), after, "{kind}");
    }
}
#[test]
fn repeated_keys_do_not_create_cross_family_reset_actions() {
    let mut e = family_specimen("phone");
    // Replace one source child with a tablet before resolving the selection.
    for definition in &mut e.document.definitions {
        {
            definition.content.nodes[1].content = NodeContent::Control {
                kind: "tablet".into(),
                size: Size::fixed(160., 60.),
            };
        }
    }
    assert_eq!(
        e.selection_property_section(&[(2, "orientation"), (3, "orientation")]),
        None
    );
    assert_eq!(
        e.selection_property_section(&[(2, "orientation")]),
        Some(PropertySection::Phone)
    );
    let before = e.document.clone();
    assert!(
        e.reset_component_section(&[2, 3], PropertySection::Phone)
            .is_err()
    );
    assert_eq!(e.document, before);
}

#[test]
fn icon_sources_row_icons_and_legacy_state_restore_inheritance() {
    let icon = IconSource::Catalog("star-solid".into());
    for (kind, key, source, local, section) in [
        (
            "icon",
            "icon",
            Value::Icon(icon.clone()),
            Value::Null,
            PropertySection::Icon,
        ),
        (
            "list",
            "row_icons",
            Value::Icons(vec![Some(icon)]),
            Value::Icons(vec![]),
            PropertySection::List,
        ),
    ] {
        let mut e = family_specimen(kind);
        for n in &mut e.document.definitions[0].content.nodes {
            n.properties.insert(key.into(), source.clone());
        }
        for id in [2, 3] {
            e.set_control_property(id, key, local.clone()).unwrap();
        }
        let before = e.document.clone();
        e.reset_component_section(&[2, 3], section).unwrap();
        for id in [2, 3] {
            assert_eq!(e.control_properties(id).unwrap().1[key], source);
            assert!(e.explicit_control_properties(id).is_empty());
        }
        e.undo();
        assert_eq!(e.document, before);
    }
    let mut e = family_specimen("checkbox");
    if let NodeContent::Instance { overrides, .. } =
        &mut e.document.pages[0].content.nodes[0].content
    {
        overrides.push(Override {
            path: vec![2],
            patch: Patch {
                properties: [("checked".into(), Value::Bool(true))]
                    .into_iter()
                    .collect(),
                ..Default::default()
            },
        });
    }
    assert_eq!(
        e.control_properties(2).unwrap().1["state"],
        Value::Choice("selected".into())
    );
    assert!(e.has_section_overrides(&[2], PropertySection::State));
    let before = e.document.clone();
    e.reset_component_section(&[2], PropertySection::State)
        .unwrap();
    assert_eq!(
        e.control_properties(2).unwrap().1["state"],
        Value::Choice("normal".into())
    );
    assert!(e.explicit_control_properties(2).is_empty());
    e.undo();
    assert_eq!(e.document, before);
}

#[test]
fn image_section_reset_reverses_frame_axes_and_transforms_crop_without_discarding_it() {
    use wireframe::{
        images::Prepared,
        workspace::{ImageImportTarget, ImportChoice},
    };
    let picture = |name: &str, c: u8| {
        let image = image::RgbaImage::from_pixel(120, 80, image::Rgba([c, 50, 100, 255]));
        let mut bytes = std::io::Cursor::new(vec![]);
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        Prepared::from_bytes(name, &bytes.into_inner()).unwrap()
    };
    let mut p = Project {
        next_id: 10,
        ..Project::default()
    };
    p.pages[0].content.nodes = ["image", "image", "cover_flow"]
        .into_iter()
        .enumerate()
        .map(|(i, kind)| {
            Node::control(
                2 + i as u64,
                kind,
                Point::new(50. + i as f32 * 250., 80.),
                Size::fixed(160., 100.),
            )
        })
        .collect();
    let mut e = Editor::new(p);
    let ids = [2, 3, 4];
    let source = e
        .import_image_at(
            ImageImportTarget::Controls(ids.to_vec()),
            picture("source.png", 20),
            ImportChoice::KeepBoth,
        )
        .unwrap();
    for (id, angle) in [(2, 0.), (3, 90.), (4, 180.)] {
        e.set_control_property(id, "rotation", Value::Number(angle))
            .unwrap();
    }
    e.select_all();
    e.group();
    let def = e.create_component("Images".into()).unwrap();
    let root = *e.selection.iter().next().unwrap();
    e.insert_component(def, 50., 380.).unwrap();
    e.enter(root);
    e.select_all();
    e.import_image_at(
        ImageImportTarget::Controls(ids.to_vec()),
        picture("local.png", 180),
        ImportChoice::KeepBoth,
    )
    .unwrap();
    for id in ids {
        e.set_control_property(id, "sketch_filter", Value::Bool(true))
            .unwrap();
    }
    e.crop_image(
        3,
        Some(Crop {
            x: 0.1,
            y: 0.1,
            width: 0.5,
            height: 0.6,
        }),
    )
    .unwrap();
    let original = e.image_geometry(3).unwrap();
    e.rotate_images(&ids).unwrap();
    let before = e.document.clone();
    e.reset_component_section(&ids, PropertySection::Image)
        .unwrap();
    let after = e.document.clone();
    for (id, angle) in [(2, 0.), (3, 90.), (4, 180.)] {
        let values = e.control_properties(id).unwrap().1;
        assert_eq!(values["image"], Value::Asset(source));
        assert_eq!(values["rotation"], Value::Number(angle));
        assert_eq!(values["sketch_filter"], Value::Bool(false));
    }
    let (full, visible, crop) = e.image_geometry(3).unwrap();
    assert_eq!((full.w, full.h), (original.0.w, original.0.h));
    for (a, b) in [
        (visible.x, original.1.x),
        (visible.y, original.1.y),
        (visible.w, original.1.w),
        (visible.h, original.1.h),
    ] {
        assert!((a - b).abs() < 0.001, "{a} != {b}");
    }
    let c = crop.unwrap();
    assert!((c.x - 0.4).abs() < 0.001);
    assert!((c.y - 0.3).abs() < 0.001);
    assert!((c.width - 0.5).abs() < 0.001);
    assert!((c.height - 0.6).abs() < 0.001);
    assert_eq!(before.definitions, after.definitions);
    assert_eq!(before.libraries, after.libraries);
    assert_eq!(before.assets, after.assets);
    assert_eq!(
        before.pages[0].content.nodes[1],
        after.pages[0].content.nodes[1]
    );
    let cover = e.nodes().iter().find(|n| n.id == 4).unwrap().clone();
    assert_eq!(cover.origin, Point::new(500., 0.));
    assert!(matches!(cover.content,NodeContent::Control{size,..}if size==Size::fixed(160.,100.)));
    assert!(!e.has_section_overrides(&ids, PropertySection::Image));
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
}
#[test]
fn image_reset_without_rotation_preserves_crop_and_geometry_exactly() {
    let mut e = family_specimen("image");
    e.crop_image(
        2,
        Some(Crop {
            x: 0.1,
            y: 0.2,
            width: 0.5,
            height: 0.6,
        }),
    )
    .unwrap();
    e.set_control_property(2, "sketch_filter", Value::Bool(true))
        .unwrap();
    let geometry = e.image_geometry(2).unwrap();
    let before = e.document.clone();
    e.reset_component_section(&[2], PropertySection::Image)
        .unwrap();
    assert_eq!(e.image_geometry(2).unwrap(), geometry);
    e.undo();
    assert_eq!(e.document, before);
}
#[test]
fn arrow_section_restores_points_and_heads_together_but_retains_explicit_size_position_text_and_color()
 {
    let mut e = family_specimen("arrow");
    let source_values = [
        ("shape", Value::Choice("curve".into())),
        ("start_x", Value::Number(0.)),
        ("start_y", Value::Number(0.)),
        ("end_x", Value::Number(180.)),
        ("end_y", Value::Number(140.)),
        ("bend_along", Value::Number(0.5)),
        ("bend_across", Value::Number(0.2)),
        ("start_head", Value::Bool(false)),
        ("end_head", Value::Bool(true)),
        ("label_position", Value::Number(5.)),
    ];
    for n in &mut e.document.definitions[0].content.nodes {
        for (key, value) in &source_values {
            n.properties.insert((*key).into(), value.clone());
        }
    }
    for id in [2, 3] {
        for (key, value) in [
            ("shape", Value::Choice("lines".into())),
            ("start_x", Value::Number(14.)),
            ("start_y", Value::Number(20.)),
            ("end_x", Value::Number(265.)),
            ("end_y", Value::Number(100.)),
            ("bend_along", Value::Number(0.55)),
            ("bend_across", Value::Number(-0.195)),
            ("start_head", Value::Bool(true)),
            ("end_head", Value::Bool(false)),
            ("label_position", Value::Number(3.)),
            ("text", Value::Text("Local".into())),
            ("border_color", Value::Color([255, 170, 0, 255])),
        ] {
            e.set_control_property(id, key, value).unwrap();
        }
    }
    if let NodeContent::Instance { overrides, .. } =
        &mut e.document.pages[0].content.nodes[0].content
    {
        for o in overrides {
            o.patch.size = Some(Size::fixed(251., 140.));
            o.patch.origin = Some(Point::new(if o.path == [2] { 0. } else { 349. }, 0.));
        }
    }
    let before = e.document.clone();
    e.reset_component_section(&[2, 3], PropertySection::Arrow)
        .unwrap();
    let after = e.document.clone();
    for id in [2, 3] {
        let values = e.control_properties(id).unwrap().1;
        for (key, value) in &source_values {
            assert_eq!(values[*key], *value, "{key}");
        }
        assert_eq!(values["text"], Value::Text("Local".into()));
        assert_eq!(values["border_color"], Value::Color([255, 170, 0, 255]));
        let n = e.nodes().iter().find(|n| n.id == id).unwrap().clone();
        assert!(matches!(n.content,NodeContent::Control{size,..}if size==Size::fixed(251.,140.)));
        assert_eq!(n.origin, Point::new(if id == 2 { 0. } else { 349. }, 0.));
    }
    assert_eq!(after.definitions, before.definitions);
    assert_eq!(
        after.pages[0].content.nodes[1],
        before.pages[0].content.nodes[1]
    );
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    e.reset_component_section(&[2, 3], PropertySection::Arrow)
        .unwrap();
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
}

#[test]
fn tab_placement_and_text_sections_reset_independently() {
    let mut e = family_specimen("tabs");
    for id in [2, 3] {
        e.set_control_property(id, "tab_position", Value::Choice("bottom".into()))
            .unwrap();
        e.set_control_property(id, "align", Value::Choice("right".into()))
            .unwrap();
    }
    assert!(!e.has_section_overrides(&[2, 3], PropertySection::Text));
    assert!(e.has_section_overrides(&[2, 3], PropertySection::Tabs));
    e.set_control_property(2, "text", Value::Text("Local, Second".into()))
        .unwrap();
    e.set_control_property(2, "fill", Value::Color([255, 170, 0, 255]))
        .unwrap();
    let before = e.document.clone();
    e.reset_component_section(&[2, 3], PropertySection::Text)
        .unwrap();
    assert_eq!(
        e.control_properties(2).unwrap().1["align"],
        Value::Choice("right".into())
    );
    assert_eq!(
        e.control_properties(2).unwrap().1["tab_position"],
        Value::Choice("bottom".into())
    );
    e.undo();
    assert_eq!(e.document, before);
    e.reset_component_section(&[2, 3], PropertySection::Tabs)
        .unwrap();
    let after = e.document.clone();
    for id in [2, 3] {
        let v = e.control_properties(id).unwrap().1;
        assert_eq!(v["align"], Value::Choice("left".into()));
        assert_eq!(v["tab_position"], Value::Choice("top".into()));
    }
    assert_eq!(
        e.control_properties(2).unwrap().1["text"],
        Value::Text("Local, Second".into())
    );
    assert_eq!(
        e.control_properties(2).unwrap().1["fill"],
        Value::Color([255, 170, 0, 255])
    );
    assert_eq!(after.definitions, before.definitions);
    assert_eq!(
        after.pages[0].content.nodes[1],
        before.pages[0].content.nodes[1]
    );
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
}

#[test]
fn border_reset_preserves_color_and_image_filter_and_exact_history() {
    for (kind, key, value) in [
        ("rectangle", "border", Value::Choice("rounded".into())),
        ("image", "image_border", Value::Bool(true)),
    ] {
        let mut e = family_specimen(kind);
        for id in [2, 3] {
            e.set_control_property(id, key, value.clone()).unwrap();
            if kind == "image" {
                e.set_control_property(id, "sketch_filter", Value::Bool(true))
                    .unwrap();
            } else {
                e.set_control_property(id, "fill", Value::Color([255, 170, 0, 255]))
                    .unwrap();
            }
        }
        let before = e.document.clone();
        let untouched = e.explicit_control_properties(2);
        e.reset_component_section(&[2, 3], PropertySection::Border)
            .unwrap();
        let after = e.document.clone();
        let local = e.explicit_control_properties(2);
        assert!(!local.contains_key(key));
        for (k, v) in untouched.iter().filter(|(k, _)| k.as_str() != key) {
            assert_eq!(local.get(k), Some(v));
        }
        assert_eq!(after.definitions, before.definitions);
        assert_eq!(
            after.pages[0].content.nodes[1],
            before.pages[0].content.nodes[1]
        );
        e.undo();
        assert_eq!(e.document, before);
        e.redo();
        assert_eq!(e.document, after);
    }
}

#[test]
fn app_bar_icon_reset_restores_fill_but_keeps_toggles_text_and_links() {
    let mut e = family_specimen("app_bar");
    for id in [2, 3] {
        e.set_control_property(id, "fill", Value::Color([255, 170, 0, 255]))
            .unwrap();
        for key in ["nav_icon", "filter_icon", "menu_icon"] {
            e.set_control_property(id, key, Value::Bool(false)).unwrap();
        }
        e.set_control_property(id, "text", Value::Text(format!("Local {id}")))
            .unwrap();
    }
    assert!(!e.has_section_overrides(&[2, 3], PropertySection::AppBarIcons));
    for id in [2, 3] {
        e.set_control_property(
            id,
            "icons",
            Value::Icons(vec![Some(IconSource::Catalog("star-solid".into()))]),
        )
        .unwrap();
    }
    e.set_control_property(2, "row_links", Value::Links(vec![Some(Link::Page(1))]))
        .unwrap();
    let before = e.document.clone();
    let old = e.control_properties(2).unwrap().1;
    e.reset_component_section(&[2, 3], PropertySection::AppBarIcons)
        .unwrap();
    let after = e.document.clone();
    let values = e.control_properties(2).unwrap().1;
    assert_eq!(values["icons"], Value::Icons(vec![]));
    assert!(!e.explicit_control_properties(2).contains_key("fill"));
    for key in ["nav_icon", "filter_icon", "menu_icon", "text", "row_links"] {
        assert_eq!(values[key], old[key]);
    }
    assert_eq!(after.definitions, before.definitions);
    assert_eq!(
        after.pages[0].content.nodes[1],
        before.pages[0].content.nodes[1]
    );
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
}

#[test]
fn vertical_tabs_reset_does_not_clear_retained_horizontal_alignment() {
    let mut e = family_specimen("vertical_tabs");
    e.set_control_property(2, "align", Value::Choice("right".into()))
        .unwrap();
    assert!(!e.has_section_overrides(&[2], PropertySection::Tabs));
    e.set_control_property(2, "tab_position", Value::Choice("right".into()))
        .unwrap();
    e.reset_component_section(&[2], PropertySection::Tabs)
        .unwrap();
    assert_eq!(
        e.control_properties(2).unwrap().1["tab_position"],
        Value::Choice("left".into())
    );
    assert_eq!(
        e.explicit_control_properties(2)["align"],
        Value::Choice("right".into())
    );
}
