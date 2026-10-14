// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{self, Project, Value},
    model::Kind,
    render::Drawing,
    scene::Primitive,
    sliders::{self, Options, Thumb},
    workspace::Editor,
};
fn set(e: &mut Editor, id: u64, key: &str, v: Value) {
    e.begin("Edit slider");
    e.set_control_property(id, key, v).unwrap();
    e.commit();
}
#[test]
fn defaults_and_fixed_thickness_survive_edits_and_reload() {
    for (kind, expected) in [
        (Kind::HorizontalSlider, (33., 12.)),
        (Kind::VerticalSlider, (12., 33.)),
    ] {
        let mut e = Editor::new(Project::default());
        e.add(kind, 20., 30.);
        let id = *e.selection.iter().next().unwrap();
        let b = e.selected_views()[0].bounds;
        assert_eq!((b.w, b.h), expected);
        let (_, p) = e.control_properties(id).unwrap();
        assert_eq!(p["value"], Value::Number(0.05));
        assert_eq!(p["slider_thumb"], Value::Choice("pointy".into()));
        e.selected_mut(|c| {
            c.bounds.w = 200.;
            c.bounds.h = 180.;
        });
        let b = e.selected_views()[0].bounds;
        assert_eq!(
            (b.w, b.h),
            if kind == Kind::HorizontalSlider {
                (200., 12.)
            } else {
                (12., 180.)
            }
        );
        let saved = e.document.encode().unwrap();
        let mut reopened = Editor::new(document::decode(&saved).unwrap());
        reopened.select(id, false);
        assert_eq!(reopened.selected_views()[0].bounds, b);
        assert_eq!(saved, reopened.document.encode().unwrap());
    }
}
#[test]
fn property_changes_are_atomic_and_undoable_and_defaults_follow_style() {
    let mut e = Editor::new(Project::default());
    e.add(Kind::HorizontalSlider, 0., 0.);
    let id = *e.selection.iter().next().unwrap();
    let before = e.document.clone();
    set(&mut e, id, "value", Value::Number(0.8));
    let after = e.document.clone();
    e.undo();
    assert_eq!(before, e.document);
    e.redo();
    assert_eq!(after, e.document);
    for (key, v) in [
        ("value", Value::Number(1.1)),
        ("slider_thumb", Value::Choice("invalid".into())),
        ("state", Value::Choice("focused".into())),
    ] {
        assert!(e.set_control_property(id, key, v).is_err());
        assert_eq!(after, e.document);
    }
    for sketch in [true, false] {
        e.document.settings.sketch = sketch;
        let p = e.control_properties(id).unwrap().1;
        assert_eq!(
            p["fill"],
            Value::Color(if sketch {
                [0, 0, 0, 255]
            } else {
                [102, 102, 102, 255]
            })
        );
    }
}
#[test]
fn endpoints_use_thumb_travel_and_keep_cross_axis_padding() {
    for vertical in [false, true] {
        for sketch in [false, true] {
            for thumb in [Thumb::Circle, Thumb::Pointy, Thumb::Rectangle] {
                for value in [0., 0.5, 1.] {
                    let o = Options {
                        vertical,
                        sketch,
                        thumb,
                        value,
                        length: 120.,
                        disabled: false,
                        color: [51, 102, 153],
                    };
                    let l = sliders::layout(o);
                    let intrinsic = if !sketch && thumb == Thumb::Circle {
                        12.
                    } else {
                        14.
                    };
                    assert_eq!(
                        if vertical { l.thumb.y } else { l.thumb.x },
                        (120. - intrinsic) * value
                    );
                    assert_eq!(l.thumb.w, 12.);
                    assert_eq!(l.thumb.h, 12.);
                    assert_eq!(
                        if vertical {
                            l.tracks[0].h
                        } else {
                            l.tracks[0].w
                        },
                        (120. - intrinsic) * value + intrinsic / 2.
                    );
                    assert_eq!(l.leading_color, [51, 102, 153]);
                    assert_eq!(l.rest_color, if sketch { [0; 3] } else { [102; 3] });
                }
            }
        }
    }
}
#[test]
fn rendered_disabled_sliders_ignore_custom_color_and_clip_narrow_shapes() {
    for kind in [Kind::HorizontalSlider, Kind::VerticalSlider] {
        for sketch in [false, true] {
            for thumb in ["pointy", "rectangle", "circle"] {
                let mut e = Editor::new(Project::default());
                e.document.settings.sketch = sketch;
                e.add(kind, 20., 30.);
                let id = *e.selection.iter().next().unwrap();
                set(&mut e, id, "fill", Value::Color([51, 102, 153, 255]));
                set(&mut e, id, "state", Value::Choice("disabled".into()));
                set(&mut e, id, "slider_thumb", Value::Choice(thumb.into()));
                set(&mut e, id, "value", Value::Number(1.));
                e.selected_mut(|c| {
                    if kind == Kind::HorizontalSlider {
                        c.bounds.w = 1.
                    } else {
                        c.bounds.h = 1.
                    }
                });
                let b = e.selected_views()[0].bounds;
                let scene = e.rendered_scene(e.page).unwrap();
                assert!(!scene.items.is_empty());
                for item in &scene.items {
                    assert_eq!(item.clip, Some(b));
                    if let Drawing::Primitive(Primitive::Polygon { fill, .. }) = &item.drawing {
                        assert_eq!(*fill, if sketch { [137; 3] } else { [204; 3] });
                    } else {
                        panic!("unexpected slider drawing");
                    }
                }
                assert!(scene.links.is_empty());
                let png = scene.png().unwrap();
                assert!(png.starts_with(b"\x89PNG"));
                assert!(scene.svg().contains("clipPath"));
            }
        }
    }
}
#[test]
fn local_component_value_and_shape_changes_do_not_modify_source() {
    let mut e = Editor::new(Project::default());
    e.add(Kind::VerticalSlider, 10., 10.);
    let id = *e.selection.iter().next().unwrap();
    e.create_component("Level".into()).unwrap();
    let defs = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    set(&mut e, child, "value", Value::Number(0.75));
    set(
        &mut e,
        child,
        "slider_thumb",
        Value::Choice("circle".into()),
    );
    assert_eq!(e.document.definitions, defs);
    assert_eq!(
        e.control_properties(child).unwrap().1["value"],
        Value::Number(0.75)
    );
    e.undo();
    assert_eq!(
        e.control_properties(child).unwrap().1["slider_thumb"],
        Value::Choice("pointy".into())
    );
    e.redo();
    assert_eq!(e.document.definitions, defs);
    assert_eq!(
        document::decode(&e.document.encode().unwrap()).unwrap(),
        e.document
    );
}

#[test]
fn resizing_either_end_keeps_opposite_edge_and_allows_one_unit_length() {
    use wireframe::model::Bounds;
    for vertical in [false, true] {
        let b = if vertical {
            Bounds::new(20., 30., 12., 33.)
        } else {
            Bounds::new(20., 30., 33., 12.)
        };
        let shrink = sliders::resize(b, vertical, -1, 40.);
        assert_eq!(if vertical { shrink.h } else { shrink.w }, 1.);
        assert_eq!(
            if vertical {
                shrink.y + shrink.h
            } else {
                shrink.x + shrink.w
            },
            if vertical { b.y + b.h } else { b.x + b.w }
        );
        let grow = sliders::resize(b, vertical, 1, 70.);
        assert_eq!((grow.x, grow.y), (20., 30.));
        assert_eq!(if vertical { grow.h } else { grow.w }, 103.);
    }
}
