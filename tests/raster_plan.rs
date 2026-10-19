// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    export_plan::{self, Selection},
    model::Bounds,
    raster::{
        BoundsRounding, Geometry,
        plan::{Prepared, Request},
    },
    render::{Drawing, Scene},
    scene::Primitive,
    workspace::{Editor, SceneScope},
};
fn rect(id: Id, x: f32, y: f32, color: u32) -> Node {
    let mut node = Node::control(id, "rectangle", Point::new(x, y), Size::fixed(80., 40.));
    node.properties.insert(
        "fill".into(),
        Value::Color([(color >> 16) as u8, (color >> 8) as u8, color as u8, 255]),
    );
    node
}
fn fixture() -> Project {
    let mut group = Node::group(
        2,
        Point::new(50., 60.),
        vec![rect(3, 0., 0., 0xff8000), rect(4, 100., 0., 0x0080ff)],
    );
    group.crop = Some(Crop {
        x: 0.25,
        y: 0.,
        width: 0.5,
        height: 1.,
    });
    let mut p = Project {
        next_id: 100,
        ..Default::default()
    };
    p.pages[0].name = "Landing".into();
    p.pages[0].content = Content {
        nodes: vec![group, rect(7, 400., 60., 0xff0000)],
        paint_order: vec![vec![2, 4], vec![7], vec![2, 3]],
    };
    p.pages[0].alternates.push(Alternate {
        version: None,
        notes: String::new(),
        id: 20,
        name: "Compact".into(),
        content: Content {
            nodes: vec![rect(21, 10., 15., 0x00ff00)],
            paint_order: vec![],
        },
    });
    p.pages.push(Page {
        trashed: false,
        id: 30,
        name: "Contact".into(),
        notes: String::new(),
        parent: None,
        content: Content::default(),
        alternates: vec![],
    });
    p.validate().unwrap();
    p
}
fn fills(scene: &Scene) -> Vec<[u8; 3]> {
    scene
        .items
        .iter()
        .filter_map(|item| match item.drawing {
            Drawing::Primitive(Primitive::Rect { fill, .. }) => Some(fill),
            _ => None,
        })
        .collect()
}
#[test]
fn explicit_choices_keep_document_order_and_one_alternate_suffix() {
    let mut p = fixture();
    let before = p.encode().unwrap();
    let request = Request {
        selection: Selection::Set([30, 20, 1].into()),
        ..Request::new(1)
    };
    let plan = Prepared::new(&p, &request).unwrap();
    assert_eq!(
        plan.entries().iter().map(|e| e.id).collect::<Vec<_>>(),
        vec![1, 20, 30]
    );
    assert_eq!(plan.entries()[1].display_name(), "Landing — Compact");
    assert_eq!(plan.entries()[1].file_stem(), "Landing (Compact)");
    assert_eq!(
        plan.scene(1).unwrap().bounds,
        Bounds::new(10., 15., 80., 40.)
    );
    p.pages[0].alternates[0].content.nodes.clear();
    assert_eq!(
        plan.scene(1).unwrap().bounds,
        Bounds::new(10., 15., 80., 40.)
    );
    assert_ne!(p.encode().unwrap(), before);
    assert!(plan.scene(3).is_err());
    let original = fixture();
    assert_eq!(
        export_plan::select(&original, &Selection::All, false)
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        Prepared::new(&original, &Request::new(1))
            .unwrap()
            .entries()
            .len(),
        3
    );
    assert_eq!(
        export_plan::select(&original, &Selection::Current(20), false).unwrap()[0].id,
        20
    );
}
#[test]
fn stale_and_empty_choices_fail_before_rendering() {
    let p = fixture();
    for selection in [
        Selection::Set([].into()),
        Selection::Current(999),
        Selection::Set([1, 999].into()),
    ] {
        assert!(
            Prepared::new(
                &p,
                &Request {
                    selection,
                    ..Request::new(1)
                }
            )
            .is_err()
        );
    }
    let mut request = Request::new(1);
    request.scope = SceneScope::Group(vec![2]);
    assert!(Prepared::new(&p, &request).is_err());
    request.selection = Selection::Current(30);
    assert!(Prepared::new(&p, &request).is_err());
    request.selection = Selection::Current(1);
    for scope in [
        SceneScope::Group(vec![]),
        SceneScope::Group(vec![7]),
        SceneScope::Selection(vec![]),
        SceneScope::Selection(vec![vec![2, 999]]),
    ] {
        request.scope = scope;
        assert!(Prepared::new(&p, &request).is_err());
    }
}
#[test]
fn selected_child_preserves_group_geometry_crop_and_editor_state() {
    let p = fixture();
    let mut e = Editor::new(p.clone());
    let canonical = e.rendered_scene(1).unwrap().svg();
    e.enter(2);
    e.select(3, false);
    let context = e.context.clone();
    let selection = e.selection.clone();
    let scene = e
        .export_scene(1, &SceneScope::Selection(vec![vec![2, 3]]))
        .unwrap();
    assert_eq!(scene.bounds, Bounds::new(95., 60., 35., 40.));
    assert_eq!(fills(&scene), vec![[255, 128, 0]]);
    assert!(scene.items.iter().all(|i| i.opacity == 1.));
    assert!(
        scene
            .items
            .iter()
            .any(|i| i.clip == Some(Bounds::new(95., 60., 90., 40.)))
    );
    assert_eq!(e.rendered_scene(1).unwrap().svg(), canonical);
    assert_eq!(e.document, p);
    assert_eq!(e.context, context);
    assert_eq!(e.selection, selection);
    assert!(!e.dirty());
    assert!(e.undo_name().is_none());
}
#[test]
fn group_and_reversed_selection_keep_interleaved_paint_order_without_duplicates() {
    let e = Editor::new(fixture());
    let all = e.export_scene(1, &SceneScope::Whole).unwrap();
    assert_eq!(fills(&all), vec![[0, 128, 255], [255, 0, 0], [255, 128, 0]]);
    let group = e.export_scene(1, &SceneScope::Group(vec![2])).unwrap();
    assert_eq!(fills(&group), vec![[0, 128, 255], [255, 128, 0]]);
    assert_eq!(group.bounds, Bounds::new(95., 60., 90., 40.));
    let selected = e
        .export_scene(
            1,
            &SceneScope::Selection(vec![vec![2, 3], vec![2], vec![2, 4]]),
        )
        .unwrap();
    assert_eq!(group.svg(), selected.svg());
}
#[test]
fn empty_page_and_group_have_minimal_export_extent() {
    let mut p = fixture();
    p.pages[1].content.nodes = vec![Node::group(31, Point::new(30., 40.), vec![])];
    let e = {
        p.organize_components().unwrap();
        Editor::new(p)
    };
    let scene = e.export_scene(30, &SceneScope::Group(vec![31])).unwrap();
    assert!(scene.items.is_empty());
    assert_eq!(scene.bounds, Bounds::new(0., 0., 0., 0.));
    let blank = Prepared::new(
        &fixture(),
        &Request {
            selection: Selection::Current(30),
            ..Request::new(30)
        },
    )
    .unwrap();
    let scene = blank.scene(0).unwrap();
    assert_eq!(scene.bounds, Bounds::new(0., 0., 0., 0.));
    let size = Geometry::new(scene.bounds, blank.options()).unwrap();
    assert_eq!((size.width, size.height), (2, 2));
}
#[test]
fn whole_page_ceil_and_group_selection_round_are_distinct() {
    let mut p = fixture();
    let mut child = rect(3, 0., 0., 0xff8000);
    let NodeContent::Control { size, .. } = &mut child.content else {
        unreachable!()
    };
    *size = Size::fixed(80.25, 40.25);
    p.pages[0].content = Content {
        nodes: vec![Node::group(2, Point::new(50., 60.), vec![child])],
        paint_order: vec![],
    };
    let mut request = Request {
        selection: Selection::Current(1),
        ..Request::new(1)
    };
    let whole = Prepared::new(&p, &request).unwrap();
    assert_eq!(
        Geometry::new(whole.scene(0).unwrap().bounds, whole.options())
            .unwrap()
            .width,
        162
    );
    for scope in [
        SceneScope::Group(vec![2]),
        SceneScope::Selection(vec![vec![2, 3]]),
    ] {
        request.scope = scope;
        let plan = Prepared::new(&p, &request).unwrap();
        assert_eq!(plan.options().rounding, BoundsRounding::Selection);
        assert_eq!(
            Geometry::new(plan.scene(0).unwrap().bounds, plan.options())
                .unwrap()
                .width,
            160
        );
    }
}
#[test]
fn instance_scope_keeps_local_overrides_and_source_geometry() {
    let mut p = fixture();
    p.definitions.push(Definition {
        id: 40,
        name: "Card".into(),
        content: p.pages[0].content.clone(),
    });
    let mut instance = Node::group(41, Point::new(300., 200.), vec![]);
    instance.content = NodeContent::Instance {
        definition: 40,
        overrides: vec![],
    };
    p.pages[0].content = Content {
        nodes: vec![instance],
        paint_order: vec![],
    };
    // Components require contiguous descendants in their source paint order.
    p.definitions[0].content.paint_order.clear();
    let mut e = {
        p.organize_components().unwrap();
        Editor::new(p)
    };
    e.enter(41);
    e.enter(2);
    e.set_control_property(3, "fill", Value::Color([0, 255, 0, 255]))
        .unwrap();
    let before = e.document.clone();
    let scene = e
        .export_scene(1, &SceneScope::Selection(vec![vec![41, 2, 3]]))
        .unwrap();
    assert_eq!(fills(&scene), vec![[0, 255, 0]]);
    assert!(scene.bounds.x >= 300. && scene.bounds.y >= 200.);
    assert_eq!(e.document, before);
}
#[test]
fn scoped_render_does_not_evict_other_controls_frozen_clocks() {
    use std::sync::atomic::{AtomicU8, Ordering};
    static MINUTE: AtomicU8 = AtomicU8::new(1);
    fn now() -> wireframe::tablet::Clock {
        wireframe::tablet::Clock::new(13, MINUTE.load(Ordering::SeqCst)).unwrap()
    }
    let mut p = Project {
        next_id: 10,
        ..Default::default()
    };
    p.pages[0].content.nodes = vec![
        wireframe::catalog::get("smartphone")
            .unwrap()
            .create(2, Point::new(0., 0.)),
        wireframe::catalog::get("smartphone")
            .unwrap()
            .create(3, Point::new(400., 0.)),
    ];
    let e = Editor::with_clock(p, now);
    let whole = e.rendered_scene(1).unwrap().svg();
    MINUTE.store(25, Ordering::SeqCst);
    e.export_scene(1, &SceneScope::Selection(vec![vec![2]]))
        .unwrap();
    let after = e.export_scene(1, &SceneScope::Whole).unwrap();
    assert_eq!(after.svg(), whole);
}

#[test]
fn worker_snapshot_preserves_displayed_clock_and_freezes_unseen_controls() {
    use std::sync::atomic::{AtomicU8, Ordering};
    static MINUTE: AtomicU8 = AtomicU8::new(1);
    fn now() -> wireframe::tablet::Clock {
        wireframe::tablet::Clock::new(13, MINUTE.load(Ordering::SeqCst)).unwrap()
    }
    let mut p = Project {
        next_id: 10,
        ..Default::default()
    };
    p.pages[0].content.nodes = vec![
        wireframe::catalog::get("smartphone")
            .unwrap()
            .create(2, Point::default()),
        wireframe::catalog::get("smartphone")
            .unwrap()
            .create(3, Point::new(400., 0.)),
    ];
    let mut editor = Editor::with_clock(p, now);
    let displayed = editor
        .export_scene(1, &SceneScope::Selection(vec![vec![2]]))
        .unwrap()
        .svg();
    MINUTE.store(25, Ordering::SeqCst);
    let snapshot = editor.export_snapshot();
    MINUTE.store(45, Ordering::SeqCst);
    editor.document.pages[0].content.nodes.clear();
    let (first, second) = std::thread::spawn(move || {
        let request = Request {
            selection: Selection::Current(1),
            scope: SceneScope::Selection(vec![vec![2]]),
            ..Request::new(1)
        };
        let prepared = Prepared::from_snapshot(snapshot, &request).unwrap();
        // This scoped export uses the displayed control's cached time.
        let first = prepared.scene(0).unwrap().svg();
        let again = prepared.scene(0).unwrap().svg();
        (first, again)
    })
    .join()
    .unwrap();
    assert_eq!(first, displayed);
    assert_eq!(first, second);
    // Unseen controls freeze the snapshot time rather than a later worker time.
    let mut p = Project {
        next_id: 10,
        ..Default::default()
    };
    p.pages[0].content.nodes = vec![
        wireframe::catalog::get("smartphone")
            .unwrap()
            .create(3, Point::default()),
    ];
    MINUTE.store(25, Ordering::SeqCst);
    let editor = Editor::with_clock(p, now);
    let snapshot = editor.export_snapshot();
    let expected = editor.export_scene(1, &SceneScope::Whole).unwrap().svg();
    MINUTE.store(45, Ordering::SeqCst);
    let prepared = Prepared::from_snapshot(snapshot, &Request::new(1)).unwrap();
    assert_eq!(prepared.scene(0).unwrap().svg(), expected);
}

#[test]
fn image_selection_does_not_inherit_the_pdf_page_count_limit() {
    let mut project = Project::default();
    project.pages[0].alternates = (2..1002)
        .map(|id| Alternate {
            version: None,
            notes: String::new(),
            id,
            name: format!("Version {id}"),
            content: Content::default(),
        })
        .collect();
    project.next_id = 1002;
    let plan = Prepared::new(&project, &Request::new(1)).unwrap();
    assert_eq!(plan.entries().len(), 1001);
    assert_eq!(
        plan.scene(1000).unwrap().bounds,
        Bounds::new(0., 0., 0., 0.)
    );
    assert!(
        wireframe::pdf::plan(
            &project,
            &wireframe::pdf::Options {
                include_alternates: true,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn worker_snapshot_preserves_displayed_dates_and_freezes_unseen_calendars() {
    use std::sync::atomic::{AtomicU8, Ordering};
    static DAY: AtomicU8 = AtomicU8::new(28);
    fn today() -> wireframe::date_controls::Date {
        wireframe::date_controls::Date::new(2028, 2, DAY.load(Ordering::SeqCst) as u32).unwrap()
    }
    let mut p = Project {
        next_id: 10,
        ..Default::default()
    };
    p.pages[0].content.nodes = vec![
        wireframe::catalog::get("calendar")
            .unwrap()
            .create(2, Point::default()),
        wireframe::catalog::get("calendar")
            .unwrap()
            .create(3, Point::new(500., 0.)),
    ];
    let editor = Editor::with_date(p, today);
    editor
        .export_scene(1, &SceneScope::Selection(vec![vec![2]]))
        .unwrap();
    DAY.store(29, Ordering::SeqCst);
    let snapshot = editor.export_snapshot();
    let expected = editor.export_scene(1, &SceneScope::Whole).unwrap().svg();
    DAY.store(1, Ordering::SeqCst);
    let prepared = Prepared::from_snapshot(snapshot, &Request::new(1)).unwrap();
    assert_eq!(prepared.scene(0).unwrap().svg(), expected);
}

#[test]
fn prepared_names_disambiguate_alternate_labels_and_keep_archive_paths_relative() {
    let mut p = fixture();
    p.title = "../Project: draft".into();
    p.pages[1].name = "Landing (Compact)".into();
    let prepared = Prepared::new(&p, &Request::new(1)).unwrap();
    assert_eq!(
        prepared
            .file_names()
            .iter()
            .map(|n| n.text())
            .collect::<Vec<_>>(),
        [
            "Landing.png",
            "Landing (Compact).png",
            "Landing (Compact)_2.png",
        ]
    );
    assert_eq!(prepared.archive_name().text(), ".._Project_ draft.zip");
    assert_eq!(
        prepared.archive_entry(2).unwrap(),
        ".._Project_ draft/Landing (Compact)_2.png"
    );
    p.title = "changed".into();
    p.pages[0].name = "changed".into();
    assert_eq!(
        prepared.archive_entry(0).unwrap(),
        ".._Project_ draft/Landing.png"
    );
    assert!(prepared.archive_entry(3).is_err());
}
