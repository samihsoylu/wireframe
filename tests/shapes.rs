// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
mod support;
use wireframe::{
    catalog,
    document::{self, IconSource, Project, Value},
    model::Kind,
    render::{Drawing, mask::BottomMask},
    shapes,
    workspace::Editor,
};
const SHAPES: [&str; 10] = [
    "ellipse",
    "rectangle",
    "rounded_rectangle",
    "dotted_rectangle",
    "diamond",
    "star",
    "parallelogram",
    "triangle",
    "torn_bottom",
    "torn_top",
];
fn choice(s: &str) -> Value {
    Value::Choice(s.into())
}

#[test]
fn shape_geometry_has_size_dependent_ink_closed_fill_and_bounded_tears() {
    for sketch in [true, false] {
        for shape in SHAPES {
            for (w, h) in [
                (8., 8.),
                (32., 32.),
                (64., 40.),
                (65., 39.),
                (65., 40.),
                (100., 80.),
                (200., 180.),
            ] {
                let g = shapes::geometry(shape, 0., w, h, sketch);
                assert_eq!(
                    g.thickness,
                    if w < 10. || h < 10. {
                        0.
                    } else if sketch {
                        (w.min(h) / 40.).clamp(2., 4.).floor()
                    } else {
                        1.
                    }
                );
                assert_eq!(g.dashed, shape == "dotted_rectangle");
                assert_eq!(
                    g.tear.is_some(),
                    shape.starts_with("torn_") && w >= 65. && h >= 40.
                );
                assert!(
                    g.outline.iter().all(|p| p[0] >= -0.001
                        && p[0] <= w + 0.001
                        && p[1] >= -0.001
                        && p[1] <= h + 0.001),
                    "{shape} {w}x{h}"
                );
                let triangles = wireframe::popover::triangles(&g.outline);
                let area = triangles
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|i| {
                        let a = g.outline[i[0] as usize];
                        let b = g.outline[i[1] as usize];
                        let c = g.outline[i[2] as usize];
                        ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() / 2.
                    })
                    .sum::<f32>();
                let polygon = g
                    .outline
                    .iter()
                    .zip(g.outline.iter().cycle().skip(1))
                    .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
                    .sum::<f32>()
                    .abs()
                    / 2.;
                assert!(
                    area > 0. && (area - polygon).abs() < 0.1,
                    "Incomplete fill: {shape} {w}x{h}: {area}/{polygon}"
                );
            }
        }
    }
}
#[test]
fn only_triangle_rotates_in_quarter_turns_without_swapping_bounds() {
    let mut variants = std::collections::HashSet::new();
    for degrees in [0., 90., 180., 270.] {
        let g = shapes::geometry("triangle", degrees, 120., 60., false);
        variants.insert(format!("{:?}", g.outline));
        assert!(
            g.outline
                .iter()
                .all(|p| p[0] >= 0. && p[0] <= 120. && p[1] >= 0. && p[1] <= 60.)
        );
    }
    assert_eq!(variants.len(), 4);
    for shape in SHAPES.into_iter().filter(|s| *s != "triangle") {
        assert_eq!(
            shapes::geometry(shape, 0., 120., 60., false).outline,
            shapes::geometry(shape, 90., 120., 60., false).outline
        );
    }
}
#[test]
fn new_shape_and_all_variants_render_edit_undo_and_round_trip() {
    let mut e = Editor::default();
    e.add(Kind::Shape, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    assert_eq!(e.control_properties(id).unwrap().0.id, "shape");
    assert_eq!(
        e.control_properties(id).unwrap().1["align"],
        choice("center")
    );
    assert_eq!(
        (
            e.selected_views()[0].bounds.w,
            e.selected_views()[0].bounds.h
        ),
        (32., 32.)
    );
    assert!(Kind::ALL.contains(&Kind::Shape));
    assert!(!Kind::ALL.contains(&Kind::Ellipse));
    support::stored_size(&mut e, 160., 100.);
    for sketch in [true, false] {
        e.document.settings.sketch = sketch;
        let mut output = std::collections::HashSet::new();
        for shape in SHAPES {
            e.begin("Change shape");
            e.set_control_property(id, "shape", choice(shape)).unwrap();
            e.commit();
            let scene = e.rendered_scene(e.page).unwrap();
            output.insert(scene.svg());
            let mut png = vec![];
            wireframe::raster::write_png(&scene, Default::default(), &mut png, |_, _| Ok(()))
                .unwrap();
            assert!(png.starts_with(b"\x89PNG"));
        }
        assert_eq!(
            output.len(),
            10,
            "Each full-size shape must have a distinct contour"
        );
    }
    let before = e.document.clone();
    e.begin("Style shape");
    for (k, v) in [
        ("fill", Value::Color([0, 0, 0, 255])),
        ("border_color", Value::Color([255, 0, 0, 255])),
        ("background_opacity", Value::Number(0.4)),
        (
            "text",
            Value::Text("[Details](https://example.com)\nNext line".into()),
        ),
        (
            "icon",
            Value::Icon(IconSource::Catalog("star-solid".into())),
        ),
        ("icon_rotation", choice("90")),
    ] {
        e.set_control_property(id, k, v).unwrap();
    }
    e.commit();
    let after = e.document.clone();
    let scene = e.rendered_scene(e.page).unwrap();
    assert!(
        scene
            .items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Glyph(_)))
    );
    assert!(
        scene
            .items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Icon { turns: 1, .. }))
    );
    assert!(
        scene
            .items
            .iter()
            .any(|i| matches!(i.drawing,Drawing::BeginOpacity(a) if (a-0.4).abs()<0.001))
    );
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    let reopened = Editor::new(document::decode(&e.document.encode().unwrap()).unwrap());
    assert_eq!(reopened.rendered_scene(1).unwrap().svg(), scene.svg());
}
#[test]
fn old_shape_centered_caption_and_alias_remain_readable() {
    for kind in ["shape", "ellipse"] {
        let mut p = Project {
            next_id: 101,
            ..Project::default()
        };
        let mut n = catalog::get(kind).unwrap().create(100, Default::default());
        if let document::NodeContent::Control { kind: stored, .. } = &mut n.content {
            *stored = kind.into();
        }
        n.properties
            .insert("text".into(), Value::Text("Retained".into()));
        p.pages[0].content.nodes.push(n);
        let mut old = serde_json::to_value(p).unwrap();
        old["format_version"] = 11.into();
        let mut e = Editor::new(document::decode(&serde_json::to_vec(&old).unwrap()).unwrap());
        assert_eq!(
            e.control_properties(100).unwrap().1["align"],
            choice("center")
        );
        e.set_control_property(100, "shape", choice("diamond"))
            .unwrap();
        assert_eq!(
            e.control_properties(100).unwrap().1["shape"],
            choice("diamond")
        );
        assert!(e.rendered_scene(1).is_ok());
        e.reset_control_property(100, "align").unwrap();
        assert_eq!(
            e.control_properties(100).unwrap().1["align"],
            choice("center")
        );
    }
}
#[test]
fn top_and_bottom_masks_preserve_complementary_mesh_regions() {
    use eframe::egui::{Color32, Pos2, Rect, Vec2, epaint::Mesh};
    let b = wireframe::model::Bounds::new(0., 0., 80., 40.);
    let mut m = Mesh::default();
    m.add_colored_rect(
        Rect::from_min_size(Pos2::ZERO, Vec2::new(80., 40.)),
        Color32::WHITE,
    );
    for above in [true, false] {
        let mask = BottomMask {
            bounds: b,
            edge: vec![[0., 20.], [80., 20.]],
            keep_above: above,
        };
        let clipped = mask.clip_mesh(&m, Pos2::ZERO, 1.);
        assert!(!clipped.indices.is_empty());
        assert!(clipped.vertices.iter().all(|v| if above {
            v.pos.y <= 20.001
        } else {
            v.pos.y >= 19.999
        }));
        let area = clipped
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|i| {
                let a = clipped.vertices[i[0] as usize].pos;
                let b = clipped.vertices[i[1] as usize].pos;
                let c = clipped.vertices[i[2] as usize].pos;
                ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs() / 2.
            })
            .sum::<f32>();
        assert!((area - 1600.).abs() < 0.01);
        assert!(
            mask.polygon()
                .iter()
                .all(|p| if above { p[1] <= 20. } else { p[1] >= 20. })
        );
    }
}
#[test]
fn text_single_character_can_ignore_padding_but_too_short_frame_hides_text() {
    let mut e = Editor::default();
    e.add(Kind::Shape, 0., 0.);
    let id = *e.selection.iter().next().unwrap();
    support::stored_size(&mut e, 12., 40.);
    e.set_control_property(id, "text", Value::Text("A".into()))
        .unwrap();
    assert!(
        e.rendered_scene(1)
            .unwrap()
            .items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Glyph(_)))
    );
    e.set_control_property(id, "text", Value::Text("AB".into()))
        .unwrap();
    assert!(
        !e.rendered_scene(1)
            .unwrap()
            .items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Glyph(_)))
    );
    e.set_control_property(id, "text", Value::Text("😀".into()))
        .unwrap();
    assert!(
        !e.rendered_scene(1)
            .unwrap()
            .items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Glyph(_)))
    );
    support::stored_size(&mut e, 120., 8.);
    e.set_control_property(id, "text", Value::Text("A".into()))
        .unwrap();
    assert!(
        !e.rendered_scene(1)
            .unwrap()
            .items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Glyph(_)))
    );
}
