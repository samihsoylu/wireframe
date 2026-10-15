// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
mod support;
use wireframe::{
    catalog,
    document::{self, Dimension, Link, NodeContent, Value},
    icons,
    linear_controls::Axis,
    model::{Bounds, Kind},
    render::Drawing,
    scene::Primitive,
    static_layouts as layouts,
    workspace::Editor,
};
const KINDS: [Kind; 4] = [
    Kind::FormattingToolbar,
    Kind::StreetMap,
    Kind::ModalOverlay,
    Kind::VerticalRule,
];
fn fixture(kind: Kind) -> (Editor, u64) {
    let mut e = Editor::default();
    e.add(kind, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
#[test]
fn fixed_axes_defaults_tiny_storage_and_supported_properties() {
    for (kind, key, default, fields) in [
        (KINDS[0], "formatting_toolbar", (261., 29.), vec![]),
        (KINDS[1], "map", (252., 222.), vec![]),
        (KINDS[2], "modal_overlay", (318., 240.), vec!["link"]),
        (
            KINDS[3],
            "vertical_rule",
            (10., 100.),
            vec!["stroke", "border_color", "background_opacity"],
        ),
    ] {
        let (mut e, id) = fixture(kind);
        let b = e.selected_views()[0].bounds;
        assert_eq!((b.w, b.h), default);
        assert!(e.prepare_text_edit(id).unwrap().is_none());
        let actual: std::collections::BTreeSet<_> = catalog::get(key)
            .unwrap()
            .properties
            .iter()
            .filter(|p| p.visible)
            .map(|p| p.key)
            .collect();
        assert_eq!(actual, fields.into_iter().collect());
        if let NodeContent::Control { size, .. } = &mut e.document.pages[0].content.nodes[0].content
        {
            size.width = Dimension::Fixed(1.);
            size.height = Dimension::Fixed(1.);
        }
        let b = e.selected_views()[0].bounds;
        assert_eq!(
            (b.w, b.h),
            match kind {
                Kind::FormattingToolbar => (1., 29.),
                Kind::VerticalRule => (10., 1.),
                _ => (1., 1.),
            }
        );
        let saved = e.document.encode().unwrap();
        assert_eq!(document::decode(&saved).unwrap(), e.document);
        e.edit("Fit", |e| {
            let result = e.auto_size_control(id);
            assert_eq!(result.is_ok(), kind == Kind::VerticalRule);
        });
        if kind == Kind::VerticalRule {
            assert_eq!(e.selected_views()[0].bounds, b);
            e.undo();
            assert_eq!(e.document.encode().unwrap(), saved);
        }
    }
}
#[test]
fn toolbar_strict_thresholds_and_licensed_icons() {
    for (index, limit) in [23., 41., 59., 77., 149., 167., 190., 208., 231., 249.]
        .into_iter()
        .enumerate()
    {
        assert!(
            !layouts::toolbar(limit)
                .icons
                .iter()
                .any(|(i, _)| *i == index)
        );
        assert!(
            layouts::toolbar(limit + 0.125)
                .icons
                .iter()
                .any(|(i, _)| *i == index)
        );
    }
    assert!(layouts::toolbar(131.).combo.is_none());
    assert!(layouts::toolbar(131.125).combo.is_some());
    for (limit, count) in [(170., 0), (170.125, 1), (211., 1), (211.125, 2)] {
        assert_eq!(layouts::toolbar(limit).separators.len(), count);
    }
    for id in layouts::TOOLBAR_ICONS {
        assert!(icons::get(id).is_some(), "{id}");
    }
    let (e, _) = fixture(Kind::FormattingToolbar);
    let scene = e.rendered_scene(e.page).unwrap();
    assert_eq!(
        scene
            .items
            .iter()
            .filter(|i| matches!(i.drawing, Drawing::Icon { .. }))
            .count(),
        10
    );
    assert!(scene.svg().contains("Font Awesome Free"));
    assert!(scene.links.is_empty());
}
#[test]
fn rule_pattern_clipping_color_opacity_and_history() {
    for sketch in [false, true] {
        for dotted in [false, true] {
            for h in [1., 10., 17., 100., 240.] {
                let r = layouts::rule(h, dotted, sketch);
                assert_eq!(r.thickness, if sketch { 3. } else { 2. });
                assert_eq!(r.segments[0][0], 0.);
                assert!(
                    r.segments
                        .iter()
                        .all(|s| s[0] >= 0. && s[1] <= h && s[1] > s[0])
                );
                for pair in r.segments.windows(2) {
                    assert_eq!(pair[1][0] - pair[0][1], r.gap);
                }
            }
        }
    }
    let (mut e, id) = fixture(Kind::VerticalRule);
    let original = e.document.clone();
    e.edit("Rule appearance", |e| {
        for (k, v) in [
            ("stroke", Value::Choice("dotted".into())),
            ("border_color", Value::Color([255, 0, 0, 255])),
            ("background_opacity", Value::Number(0.4)),
        ] {
            e.set_control_property(id, k, v).unwrap();
        }
    });
    let edited = e.document.clone();
    e.undo();
    assert_eq!(e.document, original);
    e.redo();
    assert_eq!(e.document, edited);
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(e.page).unwrap();
        assert!(
            scene
                .items
                .iter()
                .all(|i| i.opacity == 0.4 && i.clip == Some(Bounds::new(20., 30., 10., 100.)))
        );
        assert!(scene.items.iter().all(|i|matches!(&i.drawing,Drawing::Primitive(Primitive::Line{color,..})if *color==[255,0,0])));
    }
    for kind in [Kind::FormattingToolbar, Kind::VerticalRule] {
        let axis = Axis::for_kind(kind).unwrap();
        let b = Bounds::new(20., 30., 100., 100.);
        for edge in [-1, 1] {
            let r = axis.resize(b, edge, -200. * edge as f32);
            if axis.vertical {
                assert_eq!((r.w, r.h), (10., 10.));
                if edge < 0 {
                    assert_eq!(r.y + r.h, b.y + b.h);
                }
            } else {
                assert_eq!((r.w, r.h), (10., 29.));
                if edge < 0 {
                    assert_eq!(r.x + r.w, b.x + b.w);
                }
            }
        }
    }
}
#[test]
fn original_map_blocks_are_convex_finite_and_leave_open_roads() {
    let blocks = layouts::map_blocks();
    assert!(blocks.len() > 10);
    let mut area = 0.;
    for p in &blocks {
        let mut cross = vec![];
        for i in 0..p.len() {
            let a = p[i];
            let b = p[(i + 1) % p.len()];
            let c = p[(i + 2) % p.len()];
            assert!(a.iter().all(|v| v.is_finite()));
            assert!(a[0] >= -0.001 && a[0] <= 252.001 && a[1] >= -0.001 && a[1] <= 222.001);
            cross.push((b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]));
            area += (a[0] * b[1] - b[0] * a[1]) / 2.;
        }
        assert!(cross.iter().all(|v| *v >= -0.01) || cross.iter().all(|v| *v <= 0.01));
    }
    assert!(
        area > 252. * 222. * 0.7 && area < 252. * 222. * 0.95,
        "area={area}"
    );
    let (mut e, _) = fixture(Kind::StreetMap);
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        let mut s = e.rendered_scene(e.page).unwrap();
        assert_eq!(s.items.iter().any(|i|matches!(&i.drawing,Drawing::Primitive(Primitive::Rect{fill,..})if *fill==[255;3])),sketch);
        assert!(s.links.is_empty());
        s.items.insert(
            0,
            wireframe::render::Item {
                opacity: 1.,
                clip: None,
                drawing: Drawing::Primitive(Primitive::Rect {
                    b: e.selected_views()[0].bounds,
                    fill: [30, 170, 80],
                    stroke: false,
                    radius: 0.,
                }),
            },
        );
        let image = image::load_from_memory(&s.png().unwrap())
            .unwrap()
            .to_rgba8();
        assert_eq!(
            image.get_pixel(24 + 64, 24 + 10).0,
            if sketch { [255; 4] } else { [30, 170, 80, 255] }
        );
    }
}
#[test]
fn overlay_alpha_links_and_component_overrides_round_trip() {
    let (mut e, id) = fixture(Kind::ModalOverlay);
    e.set_control_property(
        id,
        "link",
        Value::Link(Link::Url("https://example.com".into())),
    )
    .unwrap();
    let s = e.rendered_scene(e.page).unwrap();
    assert_eq!(s.items.len(), 1);
    assert_eq!(s.items[0].opacity, 0.4);
    assert_eq!(s.hit_link(21., 31.), Some("https://example.com"));
    assert_eq!(s.hit_link(339., 31.), None);
    e.create_component("Overlay".into()).unwrap();
    let defs = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    e.edit("Link", |e| {
        e.set_control_property(child, "link", Value::Link(Link::Back))
            .unwrap()
    });
    assert_eq!(e.document.definitions, defs);
    assert_eq!(
        e.control_properties(child).unwrap().1["link"],
        Value::Link(Link::Back)
    );
    e.undo();
    assert_eq!(
        e.control_properties(child).unwrap().1["link"],
        Value::Link(Link::Url("https://example.com".into()))
    );
    e.redo();
    assert_eq!(
        document::decode(&e.document.encode().unwrap()).unwrap(),
        e.document
    );
}
#[test]
fn solid_rule_mesh_has_no_folded_triangles_at_short_or_tall_sizes() {
    let (mut e, _) = fixture(Kind::VerticalRule);
    e.document.settings.sketch = true;
    for h in [1., 10., 17., 67., 100., 240.] {
        let width = e.selected_views()[0].bounds.w;
        support::stored_size(&mut e, width, h);
        let s = e.rendered_scene(e.page).unwrap();
        let Drawing::Primitive(Primitive::Mesh {
            points, indices, ..
        }) = &s.items[0].drawing
        else {
            panic!("expected mesh")
        };
        assert!(points.iter().all(|p| p[1] >= 30. && p[1] <= 30. + h));
        for tri in indices.as_chunks::<3>().0 {
            let [a, b, c] = [
                points[tri[0] as usize],
                points[tri[1] as usize],
                points[tri[2] as usize],
            ];
            let cross = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            assert!(cross >= -0.001, "h={h},triangle={tri:?},cross={cross}");
        }
    }
}
