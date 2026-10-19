// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::io::Cursor;
use wireframe::{
    document::*,
    images::Prepared,
    workspace::{Editor, ImageImportTarget, ImportChoice},
};
fn picture(name: &str, color: u8) -> Prepared {
    let img = image::RgbaImage::from_fn(120, 80, |x, y| {
        image::Rgba([
            color,
            (x * 2) as u8,
            (y * 3) as u8,
            if x < 60 { 255 } else { 180 },
        ])
    });
    let mut out = Cursor::new(vec![]);
    img.write_to(&mut out, image::ImageFormat::Png).unwrap();
    Prepared::from_bytes(name, &out.into_inner()).unwrap()
}
fn selection() -> Editor {
    let mut p = Project {
        next_id: 5,
        ..Project::default()
    };
    p.pages[0].content.nodes = ["image", "image", "cover_flow"]
        .into_iter()
        .enumerate()
        .map(|(i, k)| {
            Node::control(
                i as u64 + 2,
                k,
                Point::new(50. + i as f32 * 250., 50.),
                Size::fixed(160., 100.),
            )
        })
        .collect();
    let mut e = Editor::new(p);
    e.select_all();
    e
}
#[test]
fn shared_import_source_removal_replacement_and_noop_have_atomic_history() {
    let mut e = selection();
    let before = e.document.clone();
    let a = e
        .import_image_at(
            ImageImportTarget::Controls(vec![2, 3, 4]),
            picture("one.png", 20),
            ImportChoice::KeepBoth,
        )
        .unwrap();
    assert_eq!(e.document.assets.len(), 1);
    for id in [2, 3, 4] {
        assert_eq!(
            e.control_properties(id).unwrap().1["image"],
            Value::Asset(a)
        );
    }
    let imported = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, imported);
    e.assign_images(&[2, 3, 4], None).unwrap();
    let removed = e.document.clone();
    assert_eq!(removed.assets, imported.assets);
    e.undo();
    assert_eq!(e.document, imported);
    e.assign_images(&[2, 3, 4], Some(a)).unwrap();
    e.redo();
    assert_eq!(e.document, removed);
    e.undo();
    let b = e
        .import_image_at(
            ImageImportTarget::Controls(vec![2, 3, 4]),
            picture("two.png", 120),
            ImportChoice::KeepBoth,
        )
        .unwrap();
    assert_ne!(a, b);
    let second = e.document.clone();
    e.import_image_at(
        ImageImportTarget::Controls(vec![2, 3, 4]),
        picture("two.png", 200),
        ImportChoice::Replace,
    )
    .unwrap();
    assert_eq!(e.document.assets.len(), 2);
    e.undo();
    assert_eq!(e.document, second);
    e.undo();
    assert_eq!(e.document, imported);
    assert_eq!(decode(&second.encode().unwrap()).unwrap(), second);
}
#[test]
fn mixed_image_rotation_uses_visible_crop_centers_and_one_undo() {
    let mut e = selection();
    e.import_image_at(
        ImageImportTarget::Controls(vec![2, 3, 4]),
        picture("one.png", 20),
        ImportChoice::KeepBoth,
    )
    .unwrap();
    let n = &mut e.current_mut().content.nodes[1];
    n.content = NodeContent::Control {
        kind: "image".into(),
        size: Size::fixed(320., 125.),
    };
    n.crop = Some(Crop {
        x: 0.,
        y: 0.,
        width: 0.5,
        height: 0.8,
    });
    n.properties.insert("rotation".into(), Value::Number(90.));
    e.set_control_property(4, "rotation", Value::Number(180.))
        .unwrap();
    let before = e.document.clone();
    let assets = e.document.assets.clone();
    e.rotate_images(&[2, 3, 4]).unwrap();
    assert_eq!(
        e.image_geometry(2).unwrap().1,
        wireframe::model::Bounds::new(80., 20., 100., 160.)
    );
    assert_eq!(
        e.image_geometry(3).unwrap().1,
        wireframe::model::Bounds::new(330., 20., 100., 160.)
    );
    assert_eq!(
        e.image_geometry(3).unwrap().2,
        Some(Crop {
            x: 0.,
            y: 0.5,
            width: 0.8,
            height: 0.5
        })
    );
    assert_eq!(e.nodes()[2].origin, before.pages[0].content.nodes[2].origin);
    assert_eq!(
        e.nodes()[2].content,
        before.pages[0].content.nodes[2].content
    );
    for (id, angle) in [(2, 270.), (3, 0.), (4, 90.)] {
        assert_eq!(
            e.control_properties(id).unwrap().1["rotation"],
            Value::Number(angle)
        );
    }
    assert_eq!(e.document.assets, assets);
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
}
#[test]
fn shared_image_validation_and_late_rotation_failure_keep_document_and_history() {
    let mut e = selection();
    let before = e.document.clone();
    for ids in [vec![], vec![2, 2], vec![2, 999]] {
        assert!(e.rotate_images(&ids).is_err());
        assert!(
            e.import_image_at(
                ImageImportTarget::Controls(ids),
                picture("one.png", 20),
                ImportChoice::KeepBoth
            )
            .is_err()
        );
        assert_eq!(e.document, before);
    }
    assert!(e.assign_images(&[2, 3], Some(999)).is_err());
    assert_eq!(e.document, before);
    e.import_image_at(
        ImageImportTarget::Controls(vec![2, 3, 4]),
        picture("one.png", 20),
        ImportChoice::KeepBoth,
    )
    .unwrap();
    e.current_mut().content.nodes[1].origin = Point::new(100000., 100000.);
    let before = e.document.clone();
    assert!(e.rotate_images(&[2, 3]).is_err());
    assert_eq!(e.document, before);
    e.current_mut().content.nodes[1].content = NodeContent::Control {
        kind: "button".into(),
        size: Size::default(),
    };
    assert!(e.selected_image_ids().is_none());
    assert!(
        !e.selection_properties()
            .iter()
            .any(|p| p.key == "sketch_filter")
    );
}
#[test]
fn shared_image_changes_stay_local_inside_component_and_keep_filter_mixed_state() {
    let mut e = selection();
    e.current_mut().content.nodes.pop();
    e.select_all();
    let a = e
        .import_image_at(
            ImageImportTarget::Controls(vec![2, 3]),
            picture("one.png", 20),
            ImportChoice::KeepBoth,
        )
        .unwrap();
    e.set_control_property(3, "sketch_filter", Value::Bool(true))
        .unwrap();
    assert!(
        e.selection_properties()
            .iter()
            .find(|p| p.key == "sketch_filter")
            .unwrap()
            .mixed
    );
    e.group();
    let definition = e.create_component("Pictures".into()).unwrap();
    let instance = e.selection.iter().copied().next().unwrap();
    let other = e.insert_component(definition, 700., 50.).unwrap();
    e.enter(instance);
    e.select_all();
    let ids = e.ordered_selection();
    assert_eq!(ids.len(), 2);
    let before = e.document.clone();
    let b = e
        .import_image_at(
            ImageImportTarget::Controls(ids.clone()),
            picture("two.png", 120),
            ImportChoice::KeepBoth,
        )
        .unwrap();
    assert_ne!(a, b);
    e.rotate_images(&ids).unwrap();
    assert_eq!(e.document.definitions, before.definitions);
    let after = e.document.clone();
    e.undo();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    e.redo();
    assert_eq!(e.document, after);
    e.leave();
    e.enter(other);
    for n in e.nodes().iter() {
        assert_eq!(
            e.control_properties(n.id).unwrap().1["image"],
            Value::Asset(a)
        );
    }
}
