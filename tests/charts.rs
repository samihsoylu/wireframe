// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    catalog,
    charts::{self, Family},
    document::{self, Value},
    model::{Bounds, Kind},
    render::Drawing,
    scene::Primitive,
    workspace::Editor,
};
const KINDS: [Kind; 4] = [
    Kind::ColumnChart,
    Kind::BarChart,
    Kind::LineChart,
    Kind::PieChart,
];
#[test]
fn chart_defaults_static_fields_and_no_content() {
    for k in KINDS {
        let mut e = Editor::default();
        e.add(k, 20., 30.);
        let c = &e.selected_views()[0];
        let (w, h) = charts::Family::for_kind(k).unwrap().default_size();
        assert_eq!(c.bounds, Bounds::new(20., 30., w, h));
        assert!(c.text.is_empty());
        let id = c.id;
        let (spec, _) = e.control_properties(id).unwrap();
        assert_eq!(spec.autosize, catalog::AutoSize::None);
        assert!(spec.properties.iter().all(|p| !p.visible));
        e.set_control_property(id, "text", Value::Text("Retained old text".into()))
            .unwrap();
        let scene = e.rendered_scene(e.page).unwrap();
        assert!(scene.links.is_empty());
        assert!(
            scene
                .items
                .iter()
                .all(|i| !matches!(i.drawing, Drawing::Glyph(_)))
        );
    }
}
#[test]
fn independently_resized_and_tiny_charts_survive_history_and_storage() {
    for k in KINDS {
        let mut e = Editor::default();
        e.add(k, 20., 30.);
        let id = *e.selection.iter().next().unwrap();
        let before = e.document.clone();
        for (w, h) in [(300., 80.), (80., 260.), (1., 1.)] {
            e.begin("Resize chart");
            e.selected_mut(|c| {
                c.bounds.w = w;
                c.bounds.h = h;
            });
            e.commit();
            let after = e.document.clone();
            e.undo();
            e.redo();
            assert_eq!(e.document, after);
            let bytes = e.document.encode().unwrap();
            let mut reopened = Editor::new(document::decode(&bytes).unwrap());
            reopened.select(id, false);
            let actual = reopened.selected_views()[0].bounds;
            assert_eq!((actual.x, actual.y), (20., 30.));
            assert!((actual.w - w).abs() < 0.001 && (actual.h - h).abs() < 0.001);
            assert_eq!(reopened.document.encode().unwrap(), bytes);
            assert!(
                !reopened
                    .rendered_scene(reopened.page)
                    .unwrap()
                    .items
                    .is_empty()
            );
        }
        for _ in 0..3 {
            e.undo();
        }
        assert_eq!(e.document, before);
    }
}
#[test]
fn bars_keep_absolute_lengths_and_rounding_boundary() {
    for sketch in [false, true] {
        let l = charts::layout(Family::Column, 187., 175., sketch);
        assert_eq!(l.bars.len(), 4);
        assert_eq!(l.bars[0].frame.w, 28.);
        assert_eq!(l.bars[0].frame.h, 86.);
        let doubled = charts::layout(Family::Column, 374., 350., sketch);
        assert_eq!(doubled.bars[0].frame.h, 261.);
        assert_eq!(doubled.bars[0].frame.w, 56.);
        assert!(
            charts::layout(Family::Column, 3.33, 175., sketch)
                .bars
                .is_empty()
        );
        let thin = charts::layout(Family::Column, 3.34, 175., sketch);
        assert_eq!(thin.bars.len(), 4);
        assert_eq!(thin.bars[0].fill.w, 1.);
        assert_eq!(thin.bars[0].fill.x, thin.bars[0].frame.x + 1.);
        let h = charts::layout(Family::Bar, 80., 260., sketch);
        assert_eq!(h.bars[0].frame.w, 10.);
        assert_eq!(h.bars[1].frame.w, 10.);
        assert_eq!(h.bars[2].frame.w, 35.);
        assert_eq!(h.bars[3].frame.w, 20.);
    }
}
#[test]
fn line_thickness_clamps_and_series_paint_in_order() {
    for (sketch, w, want) in [
        (true, 1., 4.),
        (true, 160., 4.),
        (true, 187., 4.675),
        (true, 320., 8.),
        (true, 640., 8.),
        (false, 640., 2.),
    ] {
        let l = charts::layout(Family::Line, w, 10., sketch);
        assert_eq!(l.series[0].width, want);
        assert_eq!(l.series[0].points[3], [w - 15., 25.]);
        assert_eq!(l.series[1].color, [167; 3]);
        let mut e = Editor::default();
        e.document.settings.sketch = sketch;
        e.add(Kind::LineChart, 0., 0.);
        let s = e.rendered_scene(e.page).unwrap();
        let colors: Vec<_> = s
            .items
            .iter()
            .filter_map(|i| {
                if let Drawing::Primitive(Primitive::Line { color, .. }) = i.drawing {
                    Some(color)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            colors,
            [[230; 3], [167; 3], if sketch { [0; 3] } else { [51; 3] }]
        );
    }
}
#[test]
fn pie_remains_centered_in_rectangular_frames() {
    for sketch in [false, true] {
        for (w, h) in [(300., 80.), (80., 260.), (180., 180.), (1., 1.)] {
            let p = charts::layout(Family::Pie, w, h, sketch).pie.unwrap();
            assert_eq!(p.center, [w / 2., h / 2.]);
            assert_eq!(p.background.w, p.background.h);
            assert_eq!(p.radius + p.width / 2., w.min(h) / 2.);
            if w.min(h) > 10. {
                assert_eq!(p.background.x + p.background.w / 2., w / 2.);
                assert_eq!(p.background.y + p.background.h / 2., h / 2.);
            }
        }
    }
}
#[test]
fn all_chart_art_is_finite_convex_or_mesh_and_clipped() {
    for k in KINDS {
        for sketch in [false, true] {
            for (w, h) in [(1., 1.), (10., 10.), (187., 175.), (80., 260.)] {
                let mut e = Editor::default();
                e.document.settings.sketch = sketch;
                e.add(k, 20., 30.);
                e.selected_mut(|c| {
                    c.bounds.w = w;
                    c.bounds.h = h;
                });
                let s = e.rendered_scene(e.page).unwrap();
                for item in &s.items {
                    assert_eq!(item.clip, Some(Bounds::new(20., 30., w, h)));
                    if let Drawing::Primitive(Primitive::Polygon { points, .. }) = &item.drawing {
                        let mut sign = 0.;
                        for i in 0..points.len() {
                            let a = points[i];
                            let b = points[(i + 1) % points.len()];
                            let c = points[(i + 2) % points.len()];
                            assert!(a.iter().all(|v| v.is_finite()));
                            let cross =
                                (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
                            if cross.abs() > 0.001 {
                                if sign != 0. {
                                    assert_eq!(cross.signum(), sign);
                                }
                                sign = cross.signum();
                            }
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
fn component_chart_size_edits_are_local_and_undoable() {
    for k in KINDS {
        let mut e = Editor::default();
        e.add(k, 20., 30.);
        let id = *e.selection.iter().next().unwrap();
        e.create_component("Chart".into()).unwrap();
        let defs = e.document.definitions.clone();
        assert!(e.enter(id));
        e.select(e.nodes()[0].id, false);
        let before = e.document.clone();
        e.begin("Resize chart");
        e.selected_mut(|c| {
            c.bounds.w = 300.;
            c.bounds.h = 80.;
        });
        e.commit();
        let after = e.document.clone();
        assert_eq!(e.document.definitions, defs);
        e.undo();
        assert_eq!(e.document, before);
        e.redo();
        assert_eq!(e.document, after);
        assert_eq!(
            document::decode(&e.document.encode().unwrap()).unwrap(),
            after
        );
    }
}
