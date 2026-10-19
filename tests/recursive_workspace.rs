// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::collections::HashSet;
use wireframe::{
    document::*,
    model::{Bounds, Kind},
    storage::ProjectStore,
    workspace::{Clipboard, Editor},
};
fn populated() -> Editor {
    let mut e = Editor::default();
    e.add(Kind::Button, 20., 30.);
    e.add(Kind::Label, 180., 50.);
    e.add(Kind::Checkbox, 100., 150.);
    e
}
fn bounds(e: &Editor) -> Vec<Bounds> {
    e.layers(e.page)
        .unwrap()
        .into_iter()
        .map(|l| l.control.bounds)
        .collect()
}
#[test]
fn nested_group_entry_movement_undo_and_round_trip_preserve_the_tree() {
    let mut e = populated();
    let first = e.nodes()[0].id;
    let second = e.nodes()[1].id;
    let before = bounds(&e);
    e.selection = HashSet::from([first, second]);
    e.group();
    assert!(e.error.is_none(), "{:?}", e.error);
    let group = *e.selection.iter().next().unwrap();
    assert!(matches!(e.nodes()[0].content, NodeContent::Group { .. }));
    assert_eq!(bounds(&e), before);
    e.select_all();
    e.group();
    let outer = *e.selection.iter().next().unwrap();
    assert!(e.enter(outer));
    assert!(e.enter(group));
    assert_eq!(e.context, vec![outer, group]);
    e.select(first, false);
    e.move_by(12., 7.);
    let moved = bounds(&e);
    assert_eq!(moved[0].x, before[0].x + 12.);
    assert_eq!(moved[1], before[1]);
    e.undo();
    assert_eq!(bounds(&e), before);
    e.redo();
    assert_eq!(bounds(&e), moved);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested.wireframe");
    ProjectStore.save(&path, &e.document).unwrap();
    let reopened = Editor::new(ProjectStore.load(&path).unwrap());
    assert_eq!(bounds(&reopened), moved);
    assert_eq!(reopened.document, e.document);
    assert!(e.leave());
    assert_eq!(e.selection, HashSet::from([group]));
    e.ungroup();
    assert_eq!(bounds(&e), moved);
    assert!(e.error.is_none(), "{:?}", e.error);
    e.undo();
    assert!(e.is_group(group));
}
#[test]
fn a_group_drag_is_one_transaction_and_cancel_restores_every_descendant() {
    let mut e = populated();
    e.select_all();
    e.group();
    let before = e.document.clone();
    let original = e.selected_views()[0].clone();
    e.begin("Move group");
    for distance in [10., 30., 70.] {
        let mut c = original.clone();
        c.bounds.x += distance;
        c.bounds.y += distance / 2.;
        e.update_control(c);
    }
    e.commit();
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    e.begin("Move group");
    e.selected_mut(|c| c.bounds.x += 100.);
    e.cancel();
    assert_eq!(e.document, after);
}
#[test]
fn full_enclosure_selection_and_lock_exclusion_are_contextual() {
    let mut e = populated();
    e.select_area(Bounds::new(19., 29., 2., 2.), &HashSet::new());
    assert!(e.selection.is_empty());
    let b = e.current().controls[0].bounds;
    e.select_area(b, &HashSet::new());
    assert_eq!(e.selection.len(), 1);
    e.lock(true);
    let before = e.document.clone();
    e.move_by(10., 0.);
    assert_eq!(e.document, before);
    e.select_all();
    assert_eq!(e.selection.len(), 2);
}
#[test]
fn grouping_nonadjacent_controls_preserves_paint_order_and_scoped_ids() {
    let mut e = populated();
    let a = e.nodes()[0].id;
    let c = e.nodes()[2].id;
    let before = bounds(&e);
    e.selection = HashSet::from([a, c]);
    e.group();
    assert_eq!(bounds(&e), before);
    e.duplicate();
    assert!(e.error.is_none(), "{:?}", e.error);
    assert_eq!(e.layers(e.page).unwrap().len(), 5);
    e.document.validate().unwrap();
    e.ungroup();
    assert!(e.error.is_none(), "{:?}", e.error);
    e.document.validate().unwrap();
    assert_eq!(e.layers(e.page).unwrap().len(), 5);
}
#[test]
fn invalid_edits_roll_back_and_new_edits_discard_redo() {
    let mut e = populated();
    let before = e.document.clone();
    e.edit("Bad input", |e| e.document.pages.clear());
    assert_eq!(e.document, before);
    assert!(e.error.take().is_some());
    e.move_by(2., 0.);
    e.undo();
    assert!(e.redo_name().is_some());
    e.move_by(0., 3.);
    assert!(e.redo_name().is_none());
}
#[test]
fn clipboard_retains_component_dependencies_and_existing_data() {
    let mut e = populated();
    let node = e.nodes()[0].clone();
    let definition = e.document.allocate_id().unwrap();
    e.document.definitions.push(Definition {
        id: definition,
        name: "Shared button".into(),
        content: Content {
            nodes: vec![node],
            ..Content::default()
        },
    });
    let instance = e.document.allocate_id().unwrap();
    let mut n = Node::group(instance, Point::new(300., 300.), vec![]);
    n.content = NodeContent::Instance {
        definition,
        overrides: vec![],
    };
    e.current_mut().content.nodes.push(n);
    e.document.organize_components().unwrap();
    e.select(instance, false);
    let copy = e.copy();
    assert_eq!(copy.definitions.len(), 1);
    e.paste(&copy, 20.);
    assert_eq!(e.document.definitions.len(), 1);
    assert!(e.error.is_none(), "{:?}", e.error);
    let mut other = Editor::default();
    other.paste(&copy, 20.);
    assert!(other.error.is_none(), "{:?}", other.error);
    assert_eq!(other.document.definitions.len(), 1);
    other.document.validate().unwrap();
    let encoded = serde_json::to_string(&copy).unwrap();
    let decoded: Clipboard = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.nodes, copy.nodes);
}
#[test]
fn group_resize_respects_leaf_axes_and_crop_remains_nondestructive() {
    let mut e = populated();
    e.select_all();
    e.group();
    let mut group = e.selected_views()[0].clone();
    let before = bounds(&e);
    group.bounds.w *= 2.;
    group.bounds.h *= 2.;
    e.edit("Resize group", |e| e.update_control(group));
    assert!(e.error.is_none(), "{:?}", e.error);
    let after = bounds(&e);
    assert_eq!(after[0].w, before[0].w * 2.);
    assert_eq!(after[1].h, before[1].h);
    let n = &mut e.current_mut().content.nodes[0];
    n.crop = Some(Crop {
        x: 0.,
        y: 0.,
        width: 0.5,
        height: 1.,
    });
    let layers = e.layers(e.page).unwrap();
    assert_eq!(layers[0].control.bounds, after[0]);
    assert!(layers.iter().all(|l| l.clip.is_some()));
}

#[test]
fn layered_component_instances_remain_atomic_and_valid_after_paste() {
    let mut e = populated();
    e.selection = HashSet::from([e.nodes()[0].id, e.nodes()[2].id]);
    e.group();
    let d = e.document.allocate_id().unwrap();
    let child = Node::control(90, "button", Point::default(), Size::fixed(70., 30.));
    e.document.next_id = 100;
    e.document.definitions.push(Definition {
        id: d,
        name: "Button component".into(),
        content: Content {
            nodes: vec![child],
            ..Content::default()
        },
    });
    let mut instance = Node::group(91, Point::new(500., 100.), vec![]);
    instance.content = NodeContent::Instance {
        definition: d,
        overrides: vec![],
    };
    e.document.organize_components().unwrap();
    let copy = Clipboard {
        pages: None,
        origin: None,
        session: None,
        nodes: vec![instance],
        definitions: e.document.definitions.clone(),
        assets: vec![],
        removed_assets: Default::default(),
        libraries: vec![],
        source: None,
        sources: vec![],
        resources: None,
    };
    e.paste(&copy, 0.);
    assert!(e.error.is_none(), "{:?}", e.error);
    e.document.validate().unwrap();
    let last = e.layers(e.page).unwrap().last().unwrap().control.bounds;
    assert_eq!(last.x, 500.);
    e.reorder(false);
    assert_eq!(e.layers(e.page).unwrap()[0].control.bounds.x, 500.);
    e.undo();
    assert_eq!(
        e.layers(e.page).unwrap().last().unwrap().control.bounds.x,
        500.
    );
}
#[test]
fn nested_reordering_does_not_cross_other_parent_paint_slots() {
    let mut e = populated();
    let a = e.nodes()[0].id;
    let c = e.nodes()[2].id;
    e.selection = HashSet::from([a, c]);
    e.group();
    let group = *e.selection.iter().next().unwrap();
    let middle = bounds(&e)[1];
    e.enter(group);
    e.select(a, false);
    e.reorder(true);
    let order = bounds(&e);
    assert_eq!(order[1], middle);
    assert_eq!(order[2].x, 20.);
    assert!(e.error.is_none(), "{:?}", e.error);
}
#[test]
fn raster_and_vector_exports_apply_the_same_group_crop() {
    let mut e = populated();
    e.select_all();
    e.group();
    e.current_mut().content.nodes[0].crop = Some(Crop {
        x: 0.25,
        y: 0.,
        width: 0.5,
        height: 1.,
    });
    let layers: Vec<_> = e
        .layers(e.page)
        .unwrap()
        .into_iter()
        .map(|l| (l.control, l.clip))
        .collect();
    let svg = wireframe::scene::svg_layers("Cropped", &layers, false);
    assert_eq!(svg.matches("<clipPath ").count(), 3);
    let png = wireframe::scene::png_svg(&svg).unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).unwrap();
    assert!(tree.size().width() < 400.);
    let mut red = wireframe::model::Control::new(1, Kind::Rectangle, 0., 0.);
    red.bounds = Bounds::new(0., 0., 100., 100.);
    red.fill = [200, 0, 0];
    red.text.clear();
    let clipped = wireframe::scene::svg_layers(
        "Pixels",
        &[(red, Some(Bounds::new(25., 0., 50., 100.)))],
        false,
    );
    let bytes = wireframe::scene::png_svg(&clipped).unwrap();
    let pixels = resvg::tiny_skia::Pixmap::decode_png(&bytes).unwrap();
    assert_eq!((pixels.width(), pixels.height()), (98, 148));
    assert_eq!(pixels.pixel(30, 50).unwrap().red(), 200);
    assert_eq!(pixels.pixel(30, 50).unwrap().green(), 0);
    assert_eq!(pixels.pixel(10, 50).unwrap().green(), 255);
    assert_eq!(pixels.pixel(87, 50).unwrap().green(), 255);
}

#[test]
fn hit_testing_uses_visible_leaf_order_and_direct_resize_obeys_axes() {
    let mut e = Editor::default();
    for _ in 0..3 {
        e.add(Kind::Button, 10., 10.);
    }
    let first = e.nodes()[0].id;
    let middle = e.nodes()[1].id;
    let last = e.nodes()[2].id;
    e.selection = HashSet::from([first, last]);
    e.group();
    let group = *e.selection.iter().next().unwrap();
    assert_eq!(e.hit(30., 25.).unwrap().id, group);
    e.select(middle, false);
    e.reorder(true);
    assert_eq!(e.hit(30., 25.).unwrap().id, middle);
    e.add(Kind::Label, 300., 300.);
    let old = e.selected_views()[0].clone();
    let mut resized = old.clone();
    resized.bounds.y -= 20.;
    resized.bounds.h += 20.;
    e.edit("Resize label", |e| e.update_control(resized));
    assert_eq!(e.selected_views()[0].bounds, old.bounds);
}
