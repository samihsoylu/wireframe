// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
mod support;
use wireframe::{
    document::{self, Project, Value},
    linear_controls::{self, Axis},
    model::Kind,
    workspace::Editor,
};
const KINDS: [Kind; 4] = [
    Kind::HorizontalScrollbar,
    Kind::VerticalScrollbar,
    Kind::HorizontalSplitter,
    Kind::VerticalSplitter,
];
#[test]
fn library_defaults_fixed_thickness_and_small_saved_lengths_round_trip() {
    for kind in KINDS {
        let mut e = Editor::new(Project::default());
        e.add(kind, 20., 30.);
        let id = *e.selection.iter().next().unwrap();
        let axis = Axis::for_kind(kind).unwrap();
        let b = e.selected_views()[0].bounds;
        assert_eq!(
            (b.w, b.h),
            if axis.vertical {
                (axis.thickness, 100.)
            } else {
                (100., axis.thickness)
            }
        );
        support::stored_size(&mut e, 1., 1.);
        let b = e.selected_views()[0].bounds;
        assert_eq!(
            (b.w, b.h),
            if axis.vertical {
                (axis.thickness, 1.)
            } else {
                (1., axis.thickness)
            }
        );
        let saved = e.document.encode().unwrap();
        let mut reopened = Editor::new(document::decode(&saved).unwrap());
        reopened.select(id, false);
        assert_eq!(reopened.selected_views()[0].bounds, b);
        assert_eq!(reopened.document.encode().unwrap(), saved);
    }
}
#[test]
fn scrollbar_value_is_validated_undoable_and_saved() {
    for kind in [Kind::HorizontalScrollbar, Kind::VerticalScrollbar] {
        let mut e = Editor::new(Project::default());
        e.add(kind, 0., 0.);
        let id = *e.selection.iter().next().unwrap();
        assert_eq!(
            e.control_properties(id).unwrap().1["value"],
            Value::Number(0.)
        );
        let before = e.document.clone();
        e.begin("Set position");
        e.set_control_property(id, "value", Value::Number(0.75))
            .unwrap();
        e.commit();
        let after = e.document.clone();
        e.undo();
        assert_eq!(e.document, before);
        e.redo();
        assert_eq!(e.document, after);
        assert!(
            e.set_control_property(id, "value", Value::Number(1.01))
                .is_err()
        );
        assert_eq!(e.document, after);
        assert_eq!(
            document::decode(&e.document.encode().unwrap()).unwrap(),
            after
        );
    }
}
#[test]
fn thumb_threshold_endpoints_and_short_negative_travel_are_retained() {
    for vertical in [false, true] {
        for sketch in [false, true] {
            for length in [1., 10., 24., 44., 45., 46., 47., 48., 100.] {
                for value in [0., 0.5, 1.] {
                    let l = linear_controls::scrollbar(vertical, length, value, sketch);
                    assert_eq!(l.thumb.is_some(), length > 45.);
                    if let Some(t) = l.thumb {
                        assert_eq!(
                            if vertical { t.y } else { t.x },
                            13. + (length - 48.) * value
                        );
                        assert_eq!((t.w, t.h), if vertical { (8., 21.) } else { (21., 8.) });
                    }
                }
            }
        }
    }
}
#[test]
fn splitter_grips_are_centered_with_style_specific_width() {
    for vertical in [false, true] {
        for sketch in [false, true] {
            for length in [1., 8., 16., 17., 100., 300.] {
                let l = linear_controls::splitter(vertical, length, sketch);
                let t = l.thumb.unwrap();
                assert!(l.background.is_none());
                assert_eq!(t.x + t.w / 2., l.frame.w / 2.);
                assert_eq!(t.y + t.h / 2., l.frame.h / 2.);
                assert_eq!(
                    (t.w, t.h),
                    if vertical {
                        (4., 16.)
                    } else if sketch {
                        (17., 4.)
                    } else {
                        (16., 4.)
                    }
                );
            }
        }
    }
}
#[test]
fn rendering_clips_small_geometry_and_exports_both_styles() {
    for kind in KINDS {
        for sketch in [false, true] {
            let mut e = Editor::new(Project::default());
            e.document.settings.sketch = sketch;
            e.add(kind, 20., 30.);
            support::stored_size(&mut e, 1., 1.);
            let b = e.selected_views()[0].bounds;
            let s = e.rendered_scene(e.page).unwrap();
            assert!(!s.items.is_empty());
            assert!(s.items.iter().all(|i| i.clip == Some(b)));
            assert!(s.links.is_empty());
            assert!(s.svg().contains("clipPath"));
            assert!(s.png().unwrap().starts_with(b"\x89PNG"));
        }
    }
}
#[test]
fn component_scroll_position_is_local_and_resettable() {
    let mut e = Editor::new(Project::default());
    e.add(Kind::VerticalScrollbar, 10., 10.);
    let id = *e.selection.iter().next().unwrap();
    e.create_component("Scroll indicator".into()).unwrap();
    let defs = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    e.begin("Position");
    e.set_control_property(child, "value", Value::Number(0.75))
        .unwrap();
    e.commit();
    assert_eq!(e.document.definitions, defs);
    assert_eq!(
        e.control_properties(child).unwrap().1["value"],
        Value::Number(0.75)
    );
    e.undo();
    assert_eq!(
        e.control_properties(child).unwrap().1["value"],
        Value::Number(0.)
    );
    e.redo();
    assert_eq!(e.document.definitions, defs);
    assert_eq!(
        document::decode(&e.document.encode().unwrap()).unwrap(),
        e.document
    );
}
