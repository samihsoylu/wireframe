// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::io::Cursor;
use wireframe::{
    document::*,
    images::Prepared,
    model::Bounds,
    workspace::{Editor, ImportChoice},
};
fn fixture() -> Editor {
    let mut p = Project {
        next_id: 4,
        ..Project::default()
    };
    p.pages[0].content.nodes = vec![
        Node::control(2, "image", Point::new(50., 50.), Size::default()),
        Node::control(3, "button", Point::new(300., 50.), Size::default()),
    ];
    let mut e = Editor::new(p);
    let image = image::RgbaImage::from_fn(120, 80, |x, y| {
        // Keep this pixel oracle lossless through import normalization.
        image::Rgba(if x == 0 && y == 0 {
            [0, 0, 0, 0]
        } else if y < 40 {
            [255, 0, 0, 255]
        } else {
            [0, 0, 255, 255]
        })
    });
    let mut bytes = Cursor::new(vec![]);
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    e.import_image(
        2,
        Prepared::from_bytes("two colors.png", &bytes.into_inner()).unwrap(),
        ImportChoice::KeepBoth,
    )
    .unwrap();
    e
}
#[test]
fn split_keeps_shared_artwork_metadata_order_and_exact_history() {
    let mut e = fixture();
    e.set_control_property(2, "text", Value::Text("Caption".into()))
        .unwrap();
    let before = e.document.clone();
    let id = e.split_image(2, 0.5).unwrap();
    assert_eq!(
        e.image_geometry(2).unwrap().1,
        Bounds::new(50., 50., 120., 40.)
    );
    assert_eq!(
        e.image_geometry(id).unwrap().1,
        Bounds::new(50., 110., 120., 40.)
    );
    assert_eq!(e.document.assets, before.assets);
    let nodes = &e.document.pages[0].content.nodes;
    assert_eq!(
        nodes.iter().map(|n| n.id).collect::<Vec<_>>(),
        vec![2, id, 3]
    );
    assert_eq!(nodes[0].properties, nodes[1].properties);
    assert_eq!(e.selection.iter().copied().collect::<Vec<_>>(), vec![id]);
    let after = e.document.clone();
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
}
#[test]
fn split_composes_with_existing_crop_rotation_and_explicit_layer_order() {
    for turns in 0..4 {
        let mut e = fixture();
        for _ in 0..turns {
            e.rotate_image(2).unwrap();
        }
        e.crop_image(
            2,
            Some(Crop {
                x: 0.1,
                y: 0.2,
                width: 0.8,
                height: 0.6,
            }),
        )
        .unwrap();
        e.current_mut().content.paint_order = vec![vec![3], vec![2]];
        let before = e.document.clone();
        let (_, v, c) = e.image_geometry(2).unwrap();
        let c = c.unwrap();
        let id = e.split_image(2, 0.25).unwrap();
        let (_, a, ca) = e.image_geometry(2).unwrap();
        let (_, b, cb) = e.image_geometry(id).unwrap();
        for (actual, expected) in [
            (a.x, v.x),
            (a.y, v.y),
            (a.w, v.w),
            (a.h, v.h * 0.25),
            (b.x, v.x),
            (b.y, v.y + v.h * 0.25 + 20.),
            (b.h, v.h * 0.75),
        ] {
            assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
        }
        assert_eq!(ca.unwrap().x, c.x);
        assert_eq!(cb.unwrap().x, c.x);
        assert!((ca.unwrap().height + cb.unwrap().height - c.height).abs() < 0.0001);
        assert_eq!(
            e.document.pages[0].content.paint_order,
            vec![vec![3], vec![2], vec![id]]
        );
        assert_eq!(e.document.assets, before.assets);
        e.undo();
        assert_eq!(e.document, before);
    }
}
#[test]
fn invalid_split_and_locked_control_preserve_document_and_redo() {
    let mut e = fixture();
    e.split_image(2, 0.5).unwrap();
    e.undo();
    let before = e.document.clone();
    let redo = e.redo_name().map(str::to_owned);
    for f in [f32::NAN, f32::INFINITY, 0., 1., -0.1, 1.1] {
        assert!(e.split_image(2, f).is_err());
        assert_eq!(e.document, before);
        assert_eq!(e.redo_name(), redo.as_deref());
    }
    assert!(e.split_image(3, 0.5).is_err());
    assert!(e.split_image(999, 0.5).is_err());
    e.current_mut().content.nodes[0].locked = true;
    let locked = e.document.clone();
    assert!(e.split_image(2, 0.5).is_err());
    assert_eq!(e.document, locked);
}
#[test]
fn nested_split_stays_in_parent_space_and_locked_parent_rejects_it() {
    let mut e = fixture();
    let image = e.current_mut().content.nodes.remove(0);
    e.current_mut()
        .content
        .nodes
        .push(Node::group(8, Point::new(200., 100.), vec![image]));
    e.document.next_id = 9;
    e.context = vec![8];
    let before = e.document.clone();
    let id = e.split_image(2, 0.5).unwrap();
    assert_eq!(
        e.image_geometry(id).unwrap().1,
        Bounds::new(250., 210., 120., 40.)
    );
    assert_eq!(e.context, vec![8]);
    e.undo();
    assert_eq!(e.document, before);
    e.context.clear();
    e.current_mut()
        .content
        .nodes
        .iter_mut()
        .find(|n| n.id == 8)
        .unwrap()
        .locked = true;
    e.context = vec![8];
    let before = e.document.clone();
    assert!(e.split_image(2, 0.5).is_err());
    assert_eq!(e.document, before);
}
#[test]
fn split_rendering_preserves_upper_and_lower_pixels() {
    let mut e = fixture();
    e.current_mut().content.nodes.retain(|n| n.id == 2);
    e.split_image(2, 0.5).unwrap();
    let scene = e.rendered_scene(e.page).unwrap();
    let png = image::load_from_memory(&scene.png().unwrap())
        .unwrap()
        .to_rgba8();
    let colors: std::collections::HashSet<_> = png.pixels().map(|p| p.0).collect();
    assert!(colors.contains(&[255, 0, 0, 255]));
    assert!(colors.contains(&[0, 0, 255, 255]));
}
#[test]
fn component_instance_rejects_structural_split_and_source_allows_it() {
    let mut e = fixture();
    e.select_all();
    e.group();
    let definition = e.create_component("Pictures".into()).unwrap();
    let instance = *e.selection.iter().next().unwrap();
    e.enter(instance);
    let id = e
        .nodes()
        .iter()
        .find(|n| matches!(&n.content,NodeContent::Control{kind,..}if kind=="image"))
        .unwrap()
        .id;
    let before = e.document.clone();
    assert!(e.split_image(id, 0.5).is_err());
    assert_eq!(e.document, before);
    e.context.clear();
    e.definition = Some(definition);
    e.split_image(id, 0.5).unwrap();
    let changed = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, changed);
}
#[test]
fn pending_edit_and_allocator_survive_a_failed_split() {
    let mut e = fixture();
    e.current_mut().content.nodes[0].origin.y = 100000.;
    e.begin("Move");
    e.current_mut().content.nodes[0].origin.x += 1.;
    let before = e.document.clone();
    let history = e.undo_name().map(str::to_owned);
    assert!(e.split_image(2, 0.5).is_err());
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), history.as_deref());
    e.cancel();
}
