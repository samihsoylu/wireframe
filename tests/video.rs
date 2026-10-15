// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    catalog,
    document::{self, Project},
    model::{Bounds, Kind},
    video,
    workspace::Editor,
};
#[test]
fn video_defaults_and_static_inspector() {
    let mut e = Editor::new(Project::default());
    e.add(Kind::Video, 20., 30.);
    let c = &e.selected_views()[0];
    assert_eq!(c.bounds, Bounds::new(20., 30., 300., 200.));
    assert!(c.text.is_empty());
    assert!(
        catalog::get("video")
            .unwrap()
            .properties
            .iter()
            .all(|p| !p.visible)
    );
    assert!(e.rendered_scene(e.page).unwrap().links.is_empty());
}
#[test]
fn sizing_preserves_tiny_files_and_history() {
    let mut e = Editor::new(Project::default());
    e.add(Kind::Video, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    let before = e.document.clone();
    e.begin("Resize video");
    e.selected_mut(|c| {
        c.bounds.w = 1.;
        c.bounds.h = 1.;
    });
    e.commit();
    let after = e.document.clone();
    let bytes = e.document.encode().unwrap();
    let mut reopened = Editor::new(document::decode(&bytes).unwrap());
    reopened.select(id, false);
    assert_eq!(
        reopened.selected_views()[0].bounds,
        Bounds::new(20., 30., 1., 1.)
    );
    assert_eq!(reopened.document.encode().unwrap(), bytes);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
}
#[test]
fn center_collapse_changes_corner_overlap_at_exact_threshold() {
    for (sketch, w, h) in [(true, 220., 50.), (false, 208., 40.)] {
        let a = video::layout(w, h, sketch);
        let b = video::layout(w + 1., h + 1., sketch);
        let nw = |l: &video::Layout| {
            l.regions
                .iter()
                .find(|r| r.column == 0 && r.row == 0)
                .unwrap()
                .destination
        };
        assert_eq!(nw(&a).w, nw(&b).w + 0.5);
        assert_eq!(nw(&a).h, nw(&b).h + 0.5);
        let tiny = video::layout(1., 1., sketch);
        assert_eq!(tiny.background, Bounds::new(1., 1., 1., 1.));
        assert_eq!(a.ink, if sketch { [0; 3] } else { [103; 3] });
    }
}
#[test]
fn fixed_bottom_art_and_independent_compression() {
    for sketch in [false, true] {
        let a = video::layout(300., 200., sketch);
        let b = video::layout(600., 400., sketch);
        let sw = |l: &video::Layout| {
            l.regions
                .iter()
                .find(|r| r.column == 0 && r.row == 2)
                .unwrap()
                .destination
        };
        assert_eq!(sw(&a).w, sw(&b).w);
        assert_eq!(sw(&a).h, sw(&b).h);
        assert_eq!(sw(&b).y - sw(&a).y, 200.);
        let thin = video::layout(100., 400., sketch);
        assert!(sw(&thin).w < sw(&b).w);
        assert_eq!(sw(&thin).h, sw(&b).h);
    }
}
#[test]
fn convex_art_and_clipping_are_shared_by_canvas_and_exports() {
    use wireframe::{render::Drawing, scene::Primitive};
    for sketch in [false, true] {
        for (w, h) in [(1., 1.), (10., 10.), (300., 200.), (600., 100.)] {
            let mut e = Editor::new(Project::default());
            e.document.settings.sketch = sketch;
            e.add(Kind::Video, 20., 30.);
            e.selected_mut(|c| {
                c.bounds.w = w;
                c.bounds.h = h;
            });
            let scene = e.rendered_scene(e.page).unwrap();
            assert!(!scene.items.is_empty());
            for item in &scene.items {
                let clip = item.clip.unwrap();
                assert!(
                    clip.x >= 20.
                        && clip.y >= 30.
                        && clip.x + clip.w <= 20. + w + 0.001
                        && clip.y + clip.h <= 30. + h + 0.001
                );
                if let Drawing::Primitive(Primitive::Polygon { points, .. }) = &item.drawing {
                    let mut sign = 0.;
                    for i in 0..points.len() {
                        let a = points[i];
                        let b = points[(i + 1) % points.len()];
                        let c = points[(i + 2) % points.len()];
                        let cross = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
                        if cross.abs() > 0.001 {
                            if sign != 0. {
                                assert_eq!(cross.signum(), sign);
                            }
                            sign = cross.signum();
                        }
                    }
                }
            }
            assert!(scene.svg().contains("clipPath"));
            assert!(scene.png().unwrap().starts_with(b"\x89PNG"));
        }
    }
}
#[test]
fn component_video_resize_is_local_and_persistent() {
    let mut e = Editor::new(Project::default());
    e.add(Kind::Video, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    e.create_component("Preview".into()).unwrap();
    let defs = e.document.definitions.clone();
    assert!(e.enter(id));
    e.select(e.nodes()[0].id, false);
    e.begin("Resize video");
    e.selected_mut(|c| {
        c.bounds.w = 600.;
        c.bounds.h = 100.;
    });
    e.commit();
    assert_eq!(e.document.definitions, defs);
    let after = e.document.clone();
    e.undo();
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(
        document::decode(&e.document.encode().unwrap()).unwrap(),
        after
    );
}
