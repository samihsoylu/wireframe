// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::collections::HashSet;
use wireframe::{document::*, model::Bounds, workspace::Editor};

fn specimen(fixed_height: bool) -> Editor {
    let mut project = Project {
        next_id: 20,
        ..Project::default()
    };
    project.pages[0].content.nodes = vec![
        Node::control(2, "button", Point::new(50., 50.), Size::fixed(160., 100.)),
        Node::control(3, "button", Point::new(400., 50.), Size::fixed(160., 100.)),
        Node::control(
            4,
            if fixed_height { "label" } else { "button" },
            Point::new(160., 160.),
            Size {
                width: Dimension::Fixed(160.),
                height: if fixed_height {
                    Dimension::Auto
                } else {
                    Dimension::Fixed(100.)
                },
            },
        ),
    ];
    let mut e = Editor::new(project);
    e.selection = HashSet::from([2, 4]);
    e
}
fn edit(e: &mut Editor, bounds: Bounds) {
    e.edit("Edit geometry", |e| e.update_selection_geometry(bounds));
    assert!(e.error.is_none(), "{:?}", e.error);
    e.document.validate().unwrap();
}
fn bounds(e: &Editor, id: Id) -> Bounds {
    e.current()
        .controls
        .iter()
        .find(|c| c.id == id)
        .unwrap()
        .bounds
}

#[test]
fn aggregate_position_size_and_history_preserve_unselected_controls() {
    let mut e = specimen(false);
    assert_eq!(
        e.selection_geometry().unwrap().bounds,
        Bounds::new(50., 50., 270., 210.)
    );
    let before = e.document.clone();
    edit(&mut e, Bounds::new(70., 50., 270., 210.));
    assert_eq!(bounds(&e, 2).x, 70.);
    assert_eq!(bounds(&e, 4).x, 180.);
    assert_eq!(e.nodes()[1], before.pages[0].content.nodes[1]);
    e.undo();
    assert_eq!(e.document, before);
    edit(&mut e, Bounds::new(50., 50., 540., 210.));
    assert_eq!(bounds(&e, 2), Bounds::new(50., 50., 320., 100.));
    assert_eq!(bounds(&e, 4), Bounds::new(270., 160., 320., 100.));
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    let reopened = Editor::new(decode(&after.encode().unwrap()).unwrap());
    assert_eq!(reopened.document, after);
}

#[test]
fn fixed_height_bottom_anchor_and_extreme_shrink_keep_exact_undo() {
    let mut e = specimen(true);
    let initial = e.selection_geometry().unwrap().bounds;
    let height = bounds(&e, 4).h;
    let before = e.document.clone();
    edit(
        &mut e,
        Bounds {
            h: initial.h * 2.,
            ..initial
        },
    );
    assert_eq!(bounds(&e, 2).h, 200.);
    assert_eq!(bounds(&e, 4).h, height);
    assert_eq!(bounds(&e, 4).y, 50. + initial.h * 2. - height);
    e.undo();
    assert_eq!(e.document, before);
    edit(&mut e, Bounds { h: 10., ..initial });
    assert_eq!(bounds(&e, 2).h, 10.);
    assert_eq!(bounds(&e, 4).y, 60. - height);
    let shrunk = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, shrunk);
}

#[test]
fn mixed_lock_uses_only_movable_bounds_and_all_locked_is_read_only() {
    let mut e = specimen(false);
    e.current_mut().content.nodes[0].locked = true;
    assert_eq!(
        e.selection_geometry().unwrap().bounds,
        Bounds::new(160., 160., 160., 100.)
    );
    let locked = e.nodes()[0].clone();
    edit(&mut e, Bounds::new(200., 160., 160., 100.));
    assert_eq!(e.nodes()[0], locked);
    assert_eq!(bounds(&e, 4).x, 200.);
    e.current_mut().content.nodes[2].locked = true;
    let state = e.selection_geometry().unwrap();
    assert!(state.locked);
    assert_eq!(state.bounds, Bounds::new(50., 50., 310., 210.));
    let before = e.document.clone();
    edit(&mut e, Bounds::new(10., 10., 50., 50.));
    assert_eq!(e.document, before);
}

#[test]
fn group_child_positions_follow_the_current_uncropped_context_origin() {
    let mut e = specimen(false);
    e.group();
    let group = *e.selection.iter().next().unwrap();
    assert!(e.enter(group));
    e.select(2, false);
    assert_eq!(
        e.selection_geometry().unwrap().bounds,
        Bounds::new(0., 0., 160., 100.)
    );
    let before = e.document.clone();
    edit(&mut e, Bounds::new(20., 0., 160., 100.));
    assert_eq!(bounds(&e, 2).x, 70.);
    assert_eq!(bounds(&e, 4).x, 160.);
    assert_eq!(e.selection_geometry().unwrap().bounds.x, 0.);
    e.select(4, false);
    assert_eq!(e.selection_geometry().unwrap().bounds.x, 90.);
    e.undo();
    assert_eq!(e.document, before);
}

#[test]
fn preview_reset_and_cancel_restore_the_entire_selection() {
    let mut e = specimen(true);
    let before = e.document.clone();
    let initial = e.movable_bounds().unwrap();
    e.begin("Resize");
    for h in [250., 280., 10.] {
        e.reset_preview();
        e.update_selection_bounds(Bounds { h, ..initial });
    }
    e.cancel();
    assert_eq!(e.document, before);
}

#[test]
fn instance_child_geometry_is_local_and_source_lock_is_respected() {
    let mut e = specimen(false);
    e.select_all();
    e.group();
    let definition = e.create_component("Contact controls".into()).unwrap();
    let instance = *e.selection.iter().next().unwrap();
    assert!(e.enter(instance));
    e.select_all();
    let before = e.document.clone();
    let state = e.selection_geometry().unwrap();
    assert_eq!(state.bounds.x, 0.);
    edit(
        &mut e,
        Bounds {
            w: state.bounds.w * 2.,
            ..state.bounds
        },
    );
    assert_eq!(e.document.definitions, before.definitions);
    assert_eq!(e.document.libraries, before.libraries);
    assert_eq!(
        e.selection_geometry().unwrap().bounds.w,
        state.bounds.w * 2.
    );
    e.undo();
    assert_eq!(e.document, before);
    e.leave();
    let (library, source) = e.document.component_source(definition).unwrap();
    let (library, owner) = (library.id, source.node);
    e.edit_library(library).unwrap();
    e.select(owner, false);
    e.lock(true);
    e.edit_library_source(definition).unwrap();
    e.select_all();
    let state = e.selection_geometry().unwrap();
    assert!(state.locked);
    let before = e.document.clone();
    edit(
        &mut e,
        Bounds {
            x: 70.,
            w: state.bounds.w * 2.,
            ..state.bounds
        },
    );
    assert_eq!(e.document, before);
}
