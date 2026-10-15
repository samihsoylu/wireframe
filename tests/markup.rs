// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    catalog,
    document::{self, Dimension, Value},
    markup::{self, Family},
    model::{Bounds, Kind},
    render::Drawing,
    scene::Primitive,
    workspace::Editor,
};
const KINDS: [Kind; 4] = [
    Kind::HorizontalBrace,
    Kind::VerticalBrace,
    Kind::RedCross,
    Kind::ScratchOut,
];
fn fixture(k: Kind) -> (Editor, u64) {
    let mut e = Editor::default();
    e.add(k, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
#[test]
fn default_sizes_markup_and_field_contracts() {
    for k in KINDS {
        let (mut e, id) = fixture(k);
        let f = Family::for_kind(k).unwrap();
        assert_eq!(
            (
                e.selected_views()[0].bounds.w,
                e.selected_views()[0].bounds.h
            ),
            f.default_size()
        );
        assert!(e.document.pages[0].content.nodes[0].markup);
        let spec = e.control_properties(id).unwrap().0;
        assert_eq!(spec.autosize, catalog::AutoSize::None);
        assert_eq!(spec.resize, catalog::ResizeAxes::Both);
        let fields: Vec<_> = spec
            .properties
            .iter()
            .filter(|p| p.visible)
            .map(|p| p.key)
            .collect();
        assert!(!fields.contains(&"link") && !fields.contains(&"align"));
        assert_eq!(fields.contains(&"text"), f.brace());
        assert_eq!(fields.contains(&"direction"), f.brace());
        assert_eq!(fields.contains(&"fill"), f == Family::ScratchOut);
        assert_eq!(
            fields.contains(&"background_opacity"),
            f == Family::ScratchOut
        );
        if f.brace() {
            e.set_control_property(id, "text", Value::Text("A very long line\n".repeat(30)))
                .unwrap();
            assert_eq!(
                (
                    e.selected_views()[0].bounds.w,
                    e.selected_views()[0].bounds.h
                ),
                f.default_size()
            );
            if let document::NodeContent::Control { size, .. } =
                &mut e.document.pages[0].content.nodes[0].content
            {
                size.width = Dimension::Auto;
                size.height = Dimension::Auto;
            }
            assert_eq!(
                (
                    e.selected_views()[0].bounds.w,
                    e.selected_views()[0].bounds.h
                ),
                f.default_size()
            );
        }
    }
}
#[test]
fn brace_placement_and_strict_label_threshold() {
    for (f, d, w, h, art, label) in [
        (
            Family::HorizontalBrace,
            "top",
            200.,
            80.,
            Bounds::new(0., 0., 200., 15.),
            Bounds::new(0., 20., 200., 60.),
        ),
        (
            Family::HorizontalBrace,
            "bottom",
            200.,
            80.,
            Bounds::new(0., 65., 200., 15.),
            Bounds::new(0., 0., 200., 60.),
        ),
        (
            Family::VerticalBrace,
            "left",
            180.,
            140.,
            Bounds::new(0., 0., 15., 140.),
            Bounds::new(20., 0., 160., 140.),
        ),
        (
            Family::VerticalBrace,
            "right",
            180.,
            140.,
            Bounds::new(165., 0., 15., 140.),
            Bounds::new(0., 0., 160., 140.),
        ),
    ] {
        let l = markup::layout(f, w, h, d);
        assert_eq!(l.art, art);
        assert_eq!(l.label, Some(label));
        for small in [1., 10., 19., 20.] {
            assert!(markup::layout(f, small, small, d).label.is_none());
        }
        assert!(markup::layout(f, 20.001, 20.001, d).label.is_some());
    }
}
#[test]
fn both_axes_history_components_and_storage() {
    for k in KINDS {
        let (mut e, id) = fixture(k);
        e.create_component("Markup".into()).unwrap();
        let defs = e.document.definitions.clone();
        assert!(e.enter(id));
        e.select(e.nodes()[0].id, false);
        for (w, h) in [(300., 60.), (60., 160.), (1., 1.)] {
            let before = e.document.clone();
            e.begin("Resize markup");
            e.selected_mut(|c| {
                c.bounds.w = w;
                c.bounds.h = h;
            });
            e.commit();
            let after = e.document.clone();
            assert_eq!(e.document.definitions, defs);
            let actual = e.selected_views()[0].bounds;
            assert!((actual.w - w).abs() < 0.001 && (actual.h - h).abs() < 0.001);
            e.undo();
            assert_eq!(e.document, before);
            e.redo();
            assert_eq!(e.document, after);
            let encoded = e.document.encode().unwrap();
            assert_eq!(
                document::decode(&encoded).unwrap().encode().unwrap(),
                encoded
            );
        }
    }
}
#[test]
fn directional_rich_text_keeps_bottom_and_center_anchors() {
    for (kind, dir) in [
        (Kind::HorizontalBrace, "bottom"),
        (Kind::VerticalBrace, "right"),
    ] {
        let (mut e, id) = fixture(kind);
        e.document.settings.sketch = false;
        e.document.pages[0].name = "Page *literal*".into();
        e.set_control_property(
            id,
            "text",
            Value::Text("*Bold* [link](https://example.com)\n{mockup-name}".into()),
        )
        .unwrap();
        e.set_control_property(id, "direction", Value::Choice(dir.into()))
            .unwrap();
        e.set_control_property(id, "text_color", Value::Color([0, 51, 204, 255]))
            .unwrap();
        let scene = e.rendered_scene(e.page).unwrap();
        let glyphs: Vec<_> = scene
            .items
            .iter()
            .filter_map(|i| {
                if let Drawing::Glyph(g) = &i.drawing {
                    Some(g)
                } else {
                    None
                }
            })
            .collect();
        let text = glyphs.iter().map(|g| g.text.as_str()).collect::<String>();
        assert!(
            text.contains("Bold") && text.contains("Page *literal*"),
            "{text}"
        );
        assert!(glyphs.iter().any(|g| g.bold));
        assert_eq!(scene.links.len(), 1);
        assert_eq!(scene.links[0].target, "https://example.com");
        assert!(scene.links[0].bounds.w > 0.);
        assert!(glyphs.iter().any(|g| g.color == [0, 51, 204, 255]));
        let art = scene.items.iter().find_map(|i| {
            if let Drawing::Primitive(Primitive::Mesh { fill, .. }) = i.drawing {
                Some(fill)
            } else {
                None
            }
        });
        assert_eq!(art, Some([0, 51, 204]));
        let first = glyphs[0].baseline;
        e.selected_mut(|c| {
            if kind == Kind::HorizontalBrace {
                c.bounds.h += 40.
            } else {
                c.bounds.h += 40.;
                c.bounds.w += 40.;
            }
        });
        let taller = e.rendered_scene(e.page).unwrap();
        let next = taller
            .items
            .iter()
            .find_map(|i| {
                if let Drawing::Glyph(g) = &i.drawing {
                    Some(g.baseline)
                } else {
                    None
                }
            })
            .unwrap();
        assert!(
            (next
                - first
                - if kind == Kind::HorizontalBrace {
                    40.
                } else {
                    20.
                })
            .abs()
                < 0.001
        );
    }
}
#[test]
fn static_marks_colors_opacity_and_clipping() {
    for sketch in [false, true] {
        for k in [Kind::RedCross, Kind::ScratchOut] {
            let (mut e, id) = fixture(k);
            e.document.settings.sketch = sketch;
            e.set_control_property(id, "text", Value::Text("Ignored".into()))
                .unwrap();
            e.set_control_property(id, "fill", Value::Color([0, 51, 204, 255]))
                .unwrap();
            if k == Kind::ScratchOut {
                e.set_control_property(id, "background_opacity", Value::Number(0.5))
                    .unwrap();
            }
            for (w, h) in [(1., 1.), (10., 10.), (300., 60.), (60., 300.)] {
                e.selected_mut(|c| {
                    c.bounds.w = w;
                    c.bounds.h = h;
                });
                let s = e.rendered_scene(e.page).unwrap();
                assert!(s.links.is_empty());
                for item in &s.items {
                    assert_eq!(item.clip, Some(Bounds::new(20., 30., w, h)));
                    assert_eq!(item.opacity, if k == Kind::ScratchOut { 0.5 } else { 1. });
                    let Drawing::Primitive(Primitive::Mesh {
                        points,
                        indices,
                        fill,
                    }) = &item.drawing
                    else {
                        panic!("Unexpected text or drawing")
                    };
                    assert_eq!(
                        *fill,
                        if k == Kind::ScratchOut {
                            [0, 51, 204]
                        } else {
                            markup::cross_color(sketch)
                        }
                    );
                    assert!(points.iter().flatten().all(|v| v.is_finite()));
                    // All triangles have the same winding; a ribbon must not fold over at bends.
                    let mut sign = 0.;
                    for triangle in indices.as_chunks::<3>().0 {
                        let [a, b, c] = [
                            points[triangle[0] as usize],
                            points[triangle[1] as usize],
                            points[triangle[2] as usize],
                        ];
                        let cross = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
                        if cross.abs() > 0.00001 {
                            if sign != 0. {
                                assert_eq!(cross.signum(), sign, "Folded mesh for {k:?}");
                            }
                            sign = cross.signum();
                        }
                    }
                }
                assert!(s.svg().contains("clipPath"));
                assert!(s.png().unwrap().starts_with(b"\x89PNG"));
            }
        }
    }
}
#[test]
fn tiny_braces_omit_text_and_keep_saved_bounds() {
    for k in [Kind::HorizontalBrace, Kind::VerticalBrace] {
        let (mut e, _) = fixture(k);
        for (w, h) in [(1., 1.), (20., 20.), (20., 80.), (80., 20.)] {
            e.selected_mut(|c| {
                c.bounds.w = w;
                c.bounds.h = h;
            });
            let before = e.document.clone();
            let s = e.rendered_scene(e.page).unwrap();
            assert_eq!(e.document, before);
            if (k == Kind::HorizontalBrace && h <= 20.) || (k == Kind::VerticalBrace && w <= 20.) {
                assert!(
                    s.items
                        .iter()
                        .all(|i| !matches!(i.drawing, Drawing::Glyph(_)))
                );
                assert!(s.links.is_empty());
            }
            assert!(
                s.items
                    .iter()
                    .all(|i| i.clip == Some(Bounds::new(20., 30., w, h)))
            );
        }
    }
}
