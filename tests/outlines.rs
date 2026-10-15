// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    catalog, document::*, model::Kind, render::Drawing, scene::Primitive, workspace::Editor,
};

fn rectangle(sketch: bool, border: &str, w: f32, h: f32) -> Editor {
    let mut p = Project::default();
    p.settings.sketch = sketch;
    let id = p.allocate_id().unwrap();
    let mut n = catalog::get("rectangle")
        .unwrap()
        .create(id, Point::new(20., 20.));
    n.content = NodeContent::Control {
        kind: "rectangle".into(),
        size: Size::fixed(w, h),
    };
    n.properties
        .insert("border".into(), Value::Choice(border.into()));
    p.pages[0].content.nodes.push(n);
    Editor::new(p)
}
#[test]
fn small_outlines_are_finite_bounded_and_do_not_paint_the_transparent_interior() {
    for sketch in [false, true] {
        for border in ["solid", "rounded", "dashed", "none"] {
            for (w, h) in [(1., 1.), (3., 8.), (8., 3.), (20., 20.), (180., 40.)] {
                let mut e = rectangle(sketch, border, w, h);
                let id = e.nodes()[0].id;
                e.set_control_property(id, "background_opacity", Value::Number(0.))
                    .unwrap();
                let scene = e.rendered_scene(e.page).unwrap();
                for item in &scene.items {
                    if let Drawing::Primitive(Primitive::Mesh {
                        points, indices, ..
                    }) = &item.drawing
                    {
                        assert!(indices.iter().all(|i| (*i as usize) < points.len()));
                        assert!(points.iter().flatten().all(|x| x.is_finite()));
                        for tri in indices
                            .as_chunks::<3>()
                            .0
                            .iter()
                            .filter(|_| border != "dashed")
                        {
                            // Solid rings must have an interior diagonal. A stroked
                            // dashed path can overlap itself at tight turns.
                            let [a, b, c] = [
                                points[tri[0] as usize],
                                points[tri[1] as usize],
                                points[tri[2] as usize],
                            ];
                            let area =
                                (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
                            assert!(area >= -0.0001, "{border} {w} x {h} must not fold: {area}");
                        }
                        // Stroke fragments may extend beyond a tiny logical frame but must be clipped.
                        assert!(item.clip.is_some());
                    }
                }
                if border == "none" {
                    assert!(scene.items.iter().all(|i| i.opacity == 0.
                        || matches!(i.drawing, Drawing::Primitive(Primitive::Text { .. }))));
                }
            }
        }
    }
    for border in ["solid", "rounded", "dashed"] {
        let mut e = rectangle(true, border, 70., 40.);
        let id = e.nodes()[0].id;
        e.set_control_property(id, "background_opacity", Value::Number(0.))
            .unwrap();
        e.set_control_property(id, "border_color", Value::Color([120, 30, 200, 255]))
            .unwrap();
        let bytes = e.rendered_scene(e.page).unwrap().png().unwrap();
        let img = image::load_from_memory(&bytes).unwrap().to_rgb8();
        assert_eq!(img.get_pixel(59, 44).0, [255; 3]);
        assert!(img.pixels().any(|p| p.0 == [120, 30, 200]));
    }
}
#[test]
fn outline_options_survive_undo_storage_and_deterministic_rendering() {
    let mut e = rectangle(true, "solid", 70., 40.);
    let id = e.nodes()[0].id;
    let original = e.document.encode().unwrap();
    let before = e.rendered_scene(e.page).unwrap().svg();
    e.begin("Border");
    e.set_control_property(id, "border", Value::Choice("rounded".into()))
        .unwrap();
    e.commit();
    let changed = e.document.encode().unwrap();
    let after = e.rendered_scene(e.page).unwrap().svg();
    assert_ne!(before, after);
    e.undo();
    assert!(
        e.document.encode().unwrap() == original,
        "Undo must restore the original project"
    );
    assert_eq!(e.rendered_scene(e.page).unwrap().svg(), before);
    e.redo();
    assert!(
        e.document.encode().unwrap() == changed,
        "Redo must restore the edited project"
    );
    let reopened = Editor::new(decode(&changed).unwrap());
    assert_eq!(reopened.rendered_scene(reopened.page).unwrap().svg(), after);
}
#[test]
fn sketch_corner_geometry_is_stable_when_only_the_center_stretches() {
    for kind in [Kind::Button, Kind::TextArea, Kind::Rectangle] {
        let mut e = Editor::default();
        e.add(kind, 20., 20.);
        e.selected_mut(|c| {
            c.bounds.w = 70.;
            c.bounds.h = 40.;
        });
        let points = |e: &Editor| {
            let scene = e.rendered_scene(e.page).unwrap();
            scene
                .items
                .into_iter()
                .filter_map(|i| match i.drawing {
                    Drawing::Primitive(Primitive::Mesh { points, .. }) => Some(points),
                    _ => None,
                })
                .flatten()
                .filter(|p| p[0] < 25. && p[1] < 24.)
                .collect::<Vec<_>>()
        };
        let normal = points(&e);
        assert!(!normal.is_empty());
        e.selected_mut(|c| {
            c.bounds.w = 180.;
            c.bounds.h = 120.;
        });
        assert_eq!(normal, points(&e));
    }
}

#[test]
fn button_shadow_steps_outside_the_frame_without_folded_small_geometry() {
    for (w, h) in [
        (1., 1.),
        (3., 8.),
        (8., 3.),
        (8., 8.),
        (20., 20.),
        (70., 40.),
        (180., 40.),
    ] {
        let mut p = Project::default();
        let id = p.allocate_id().unwrap();
        let mut n = catalog::get("button")
            .unwrap()
            .create(id, Point::new(20., 20.));
        n.properties
            .insert("text".into(), Value::Text(String::new()));
        n.content = NodeContent::Control {
            kind: "button".into(),
            size: Size::fixed(w, h),
        };
        p.pages[0].content.nodes.push(n);
        let e = Editor::new(p);
        let scene = e.rendered_scene(e.page).unwrap();
        assert_eq!(
            (scene.bounds.w, scene.bounds.h),
            (w, h),
            "Stored dimensions must not pass through an interactive resize helper"
        );
        for item in &scene.items {
            if let Drawing::Primitive(Primitive::Mesh {
                points, indices, ..
            }) = &item.drawing
            {
                assert!(points.iter().flatten().all(|v| v.is_finite()));
                assert!(
                    points
                        .iter()
                        .all(|p| (20. ..=20. + w).contains(&p[0])
                            && (20. ..=20. + h).contains(&p[1]))
                );
                for tri in indices.as_chunks::<3>().0 {
                    let [a, b, c] = [
                        points[tri[0] as usize],
                        points[tri[1] as usize],
                        points[tri[2] as usize],
                    ];
                    let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
                    assert!(
                        area >= -0.0001,
                        "A {w} x {h} border must not fold over itself: {area}; {a:?} {b:?} {c:?}"
                    );
                }
            }
        }
        if w >= 8. && h >= 8. {
            let image = image::load_from_memory(&scene.png().unwrap())
                .unwrap()
                .to_rgb8();
            assert_eq!(
                image.get_pixel(24 + w as u32 - 1, 24).0,
                [255; 3],
                "The shadow starts below the top edge"
            );
            assert_eq!(
                image.get_pixel(24, 24 + h as u32 - 1).0,
                [255; 3],
                "The shadow starts right of the left edge"
            );
            assert!(
                image
                    .get_pixel(24 + w as u32 - 2, 24 + h as u32 - 2)
                    .0
                    .iter()
                    .all(|v| *v < 100)
            );
        }
    }
}
