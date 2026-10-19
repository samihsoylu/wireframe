// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::collections::HashSet;
use wireframe::{document::*, model::Bounds, workspace::Editor};
fn leaf(id: Id, x: f32, y: f32, markup: bool) -> Node {
    let mut n = Node::control(id, "rectangle", Point::new(x, y), Size::fixed(80., 40.));
    n.markup = markup;
    n
}
fn fixture() -> Editor {
    let mut p = Project {
        next_id: 100,
        ..Project::default()
    };
    p.pages[0].content.nodes = vec![Node::group(
        2,
        Point::new(50., 60.),
        vec![leaf(3, 20., 30., false), leaf(4, 300., 200., true)],
    )];
    Editor::new(p)
}
#[test]
fn hidden_markup_changes_snap_frame_but_keeps_selection_marquee_and_hit_frame() {
    let mut e = fixture();
    let full = Bounds::new(70., 90., 360., 210.);
    let visible = Bounds::new(70., 90., 80., 40.);
    e.set_markup_visible(false);
    e.select(2, false);
    assert_eq!(e.bounds(), Some(full));
    assert_eq!(e.hit(400., 280.).unwrap().id, 2);
    e.select_area(visible, &HashSet::new());
    assert!(e.selection.is_empty());
    e.select_area(full, &HashSet::new());
    assert_eq!(e.selection, HashSet::from([2]));
    let hidden = e.alignment_targets_with_selection(4., true).unwrap();
    assert!(hidden.matches(visible).iter().any(|g| g.position == 150.));
    assert!(!hidden.matches(full).iter().any(|g| g.position == 430.));
    e.set_markup_visible(true);
    let shown = e.alignment_targets_with_selection(4., true).unwrap();
    assert!(shown.matches(full).iter().any(|g| g.position == 430.));
}
#[test]
fn hiding_does_not_change_move_resize_or_history_of_group_descendants() {
    for resize in [false, true] {
        let mut shown = fixture();
        let mut hidden = fixture();
        hidden.set_markup_visible(false);
        for editor in [&mut shown, &mut hidden] {
            editor.select(2, false);
            let mut target = editor.bounds().unwrap();
            target.x += 30.;
            target.y += 20.;
            if resize {
                target.w *= 2.;
                target.h *= 2.;
            }
            editor.begin("Change group geometry");
            editor.update_selection_bounds(target);
            editor.commit();
        }
        assert_eq!(shown.document, hidden.document);
        let after = hidden.document.clone();
        hidden.undo();
        assert_eq!(hidden.document, fixture().document);
        hidden.redo();
        assert_eq!(hidden.document, after);
        assert!(!hidden.markup_visible());
    }
}
#[test]
fn nested_hidden_bounds_intersect_existing_masks_without_repositioning_artwork() {
    let mut e = fixture();
    e.document.pages[0].content.nodes[0].crop = Some(Crop {
        x: 0.1,
        y: 0.,
        width: 0.5,
        height: 1.,
    });
    let before = e.document.clone();
    let layout = e.layout_page(1).unwrap();
    let n = &layout.nodes[0];
    assert_eq!(
        n.bounds,
        wireframe::layout::Rect {
            x: 106.,
            y: 90.,
            width: 180.,
            height: 210.
        }
    );
    assert_eq!(
        n.without_markup,
        wireframe::layout::Rect {
            x: 106.,
            y: 90.,
            width: 44.,
            height: 40.
        }
    );
    e.set_markup_visible(false);
    assert_eq!(e.document, before);
    assert_eq!(
        e.editing_scene().unwrap().bounds,
        Bounds::new(106., 90., 44., 40.)
    );
    e.enter(2);
    assert_eq!(
        e.current().controls[0].bounds,
        Bounds::new(70., 90., 80., 40.)
    );
}
#[test]
fn all_marked_descendants_have_zero_snap_extent_and_instances_resolve_local_flags() {
    let mut e = fixture();
    e.enter(2);
    e.set_markup(3, true).unwrap();
    e.leave();
    let n = e.layout_page(1).unwrap().nodes.remove(0);
    assert_eq!(
        n.without_markup,
        wireframe::layout::Rect {
            x: 50.,
            y: 60.,
            width: 0.,
            height: 0.
        }
    );
    e.select(2, false);
    e.create_component("Shared annotations".into()).unwrap();
    let instance = e.nodes()[0].id;
    assert!(e.enter(instance));
    e.set_markup(3, false).unwrap();
    assert!(e.leave());
    let n = e.layout_page(1).unwrap().nodes.remove(0);
    assert_eq!(
        (n.without_markup.width, n.without_markup.height),
        (80., 40.)
    );
}
