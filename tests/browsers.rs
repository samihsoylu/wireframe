// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use eframe::egui::{self, Color32, Pos2, Rect, Vec2, epaint::Mesh};
use wireframe::{
    browser::{Browser, Chrome, Style, torn_edge},
    document::*,
    model::{Bounds, Kind},
    render::{Drawing, mask::BottomMask},
    rich_text::Inline,
    workspace::Editor,
};
fn fixture() -> (Editor, u64) {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    e.add(Kind::Browser, 0., 0.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
fn set(e: &mut Editor, id: u64, key: &str, value: Value) {
    e.begin("Edit browser");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
fn visible(text: &wireframe::rich_text::RichText) -> String {
    text.items
        .iter()
        .filter_map(|i| {
            if let Inline::Span { text, .. } = i {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect()
}
#[test]
fn source_split_preserves_empty_extra_lines_and_literal_page_names() {
    for (raw, title, address) in [
        ("", "", ""),
        ("Title", "Title", ""),
        ("\nURL", "", "URL"),
        ("Title\nfirst\nsecond", "Title", "first\nsecond"),
        (
            "Title\\rcontinued\nURL\\rcontinued",
            "Title\\rcontinued",
            "URL\\rcontinued",
        ),
    ] {
        let b = Browser::parse(raw, "Page").unwrap();
        assert_eq!(b.title.source, title);
        assert_eq!(b.address.source, address);
    }
    let b = Browser::parse("{mockup-name}\n{project-name} {date}", "*Page_One* [draft]").unwrap();
    assert_eq!(visible(&b.title), "*Page_One* [draft]");
    assert_eq!(visible(&b.address), "{project-name} {date}");
    assert!(
        b.title
            .items
            .iter()
            .all(|i| !matches!(i,Inline::Span{style,..} if style.bold||style.italic||style.link))
    );
    assert!(Browser::parse(&"x".repeat(100_001), "").is_err());
}
#[test]
fn boundary_layout_keeps_text_offsets_and_original_fill_gate() {
    let b = Browser::parse("Title\nURL", "").unwrap();
    for sketch in [false, true] {
        let s = Style::new(sketch);
        let cutoff = s.address_left + s.address_right;
        assert!(
            b.layout_with_heights(cutoff, 240., sketch, false, false, 13., 13.)
                .address
                .is_none()
        );
        assert_eq!(
            b.layout_with_heights(cutoff + 1., 240., sketch, false, false, 13., 13.)
                .address
                .unwrap()
                .w,
            1.
        );
        let h = s.content_top + s.bottom;
        assert!(
            b.layout_with_heights(300., h, sketch, true, true, 13., 13.)
                .content
                .is_none()
        );
        assert_eq!(
            b.layout_with_heights(300., h + 1., sketch, true, true, 13., 13.)
                .content
                .unwrap()
                .h,
            s.bottom + 1.
        );
        assert!(
            !b.layout_with_heights(67., 38., sketch, true, true, 13., 13.)
                .mask
        );
        assert!(
            b.layout_with_heights(67., 39., sketch, true, true, 13., 13.)
                .mask
        );
        assert!(
            !b.layout_with_heights(66., 180., sketch, true, true, 13., 13.)
                .mask
        );
        let c = Chrome {
            width: 50.,
            height: 38.,
            style: s,
        };
        assert!(c.x(20., false) < 20.);
        assert!(c.y(30., false) < 30.);
        assert_eq!(
            b.layout_with_heights(50., 38., sketch, false, false, 13., 13.)
                .title
                .y,
            8.
        );
    }
}
#[test]
fn fixed_dimensions_history_component_overrides_and_saved_values() {
    let (mut e, id) = fixture();
    let original = e.document.clone();
    let (spec, p) = e.control_properties(id).unwrap();
    assert_eq!(
        e.selected_views()[0].bounds,
        Bounds::new(0., 0., 450., 400.)
    );
    assert_eq!(p["text"], Value::Text(wireframe::browser::DEFAULT.into()));
    assert_eq!(spec.autosize, wireframe::catalog::AutoSize::None);
    for key in ["text", "border", "fill", "vertical_scrollbar", "value"] {
        assert!(spec.property(key).unwrap().visible);
    }
    for key in ["bold", "font_size", "italic", "underline"] {
        assert!(!spec.property(key).unwrap().visible);
    }
    set(
        &mut e,
        id,
        "text",
        Value::Text("Title\nURL\nadditional".into()),
    );
    assert_eq!(e.selected_views()[0].bounds.w, 450.);
    e.undo();
    assert_eq!(e.document, original);
    e.redo();
    set(&mut e, id, "border", Value::Choice("torn_bottom".into()));
    set(&mut e, id, "vertical_scrollbar", Value::Bool(true));
    set(&mut e, id, "value", Value::Number(0.5));
    let before = e.document.clone();
    assert!(e.auto_size_control(id).is_err());
    assert_eq!(e.document, before);
    e.create_component("Browser frame".into()).unwrap();
    let definitions = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    set(&mut e, child, "fill", Value::Color([140, 190, 220, 255]));
    assert_eq!(e.document.definitions, definitions);
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
    e.begin("Reset content color");
    e.reset_control_property(child, "fill").unwrap();
    e.commit();
    assert!(!e.explicit_control_properties(child).contains_key("fill"));
}
#[test]
fn thumb_thresholds_and_inline_links_follow_rendered_control_semantics() {
    let (mut e, id) = fixture();
    set(
        &mut e,
        id,
        "text",
        Value::Text("[Title](https://example.org)\n[Address](https://example.org/path)".into()),
    );
    set(&mut e, id, "vertical_scrollbar", Value::Bool(true));
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        let threshold = if sketch { 147. } else { 118. };
        for h in [threshold - 1., threshold] {
            e.begin("Resize");
            e.selected_mut(|c| {
                c.bounds.w = 300.;
                c.bounds.h = h;
            });
            e.commit();
            let scene = e.rendered_scene(e.page).unwrap();
            assert!(scene.links.is_empty());
            let thumbs=scene.items.iter().filter(|i|matches!(&i.drawing,Drawing::Primitive(wireframe::scene::Primitive::Rect{b,..})if b.w==8.&&b.h==21.)).count();
            assert_eq!(thumbs, usize::from(h == threshold));
            let image = image::load_from_memory(&scene.png().unwrap())
                .unwrap()
                .to_rgba8();
            let style = Style::new(sketch);
            let x = 24 + (300. - style.scrollbar_right - 8.) as u32;
            let track = (style.content_top + 13.) as u32..(h - style.bottom - 14.) as u32;
            let dark = track
                .filter(|y| {
                    let pixel = image.get_pixel(x, 24 + y).0;
                    pixel[..3].iter().all(|channel| *channel < 110)
                })
                .count();
            assert_eq!(dark > 0, h == threshold, "style={sketch}, height={h}");
        }
    }
}
#[test]
fn tiny_frames_do_not_export_negative_fill_dimensions() {
    let (mut e, _) = fixture();
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        for width in [1., 2., 3., 4.] {
            e.selected_mut(|c| {
                c.bounds.w = width;
                c.bounds.h = 180.;
            });
            let scene = e.rendered_scene(e.page).unwrap();
            for item in &scene.items {
                if let Drawing::Primitive(wireframe::scene::Primitive::Rect { b, .. }) =
                    &item.drawing
                {
                    assert!(b.w >= 0. && b.h >= 0., "invalid fill {b:?}");
                }
            }
            assert!(!scene.png().unwrap().is_empty());
        }
    }
}
#[test]
fn torn_export_reveals_underlying_control_instead_of_white_cover() {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    e.add(Kind::Browser, 0., 0.);
    let back = *e.selection.iter().next().unwrap();
    set(&mut e, back, "fill", Value::Color([30, 170, 80, 255]));
    e.selected_mut(|c| c.bounds.h = 250.);
    e.add(Kind::Browser, 0., 0.);
    let front = *e.selection.iter().next().unwrap();
    e.selected_mut(|c| c.bounds.h = 180.);
    set(&mut e, front, "border", Value::Choice("torn_bottom".into()));
    set(&mut e, front, "fill", Value::Color([150, 200, 250, 255]));
    let scene = e.rendered_scene(e.page).unwrap();
    assert_eq!(
        scene
            .items
            .iter()
            .filter(|i| matches!(i.drawing, Drawing::BeginMask(_)))
            .count(),
        1
    );
    let svg = scene.svg();
    assert!(svg.contains("<polygon points="));
    let image = image::load_from_memory(&scene.png().unwrap())
        .unwrap()
        .to_rgba8();
    assert_eq!(image.get_pixel(24 + 10, 24 + 175).0, [30, 170, 80, 255]);
    assert_eq!(image.get_pixel(24 + 10, 24 + 130).0, [150, 200, 250, 255]);
}
#[test]
fn mesh_mask_preserves_texture_coordinates_and_canvas_order() {
    let bounds = Bounds::new(0., 0., 100., 100.);
    let mask = BottomMask {
        keep_above: true,
        bounds,
        edge: vec![[0., 70.], [50., 30.], [100., 70.]],
    };
    let mut mesh = Mesh::with_texture(egui::TextureId::User(4));
    mesh.add_rect_with_uv(
        Rect::from_min_max(Pos2::ZERO, Pos2::new(100., 100.)),
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
        Color32::WHITE,
    );
    let clipped = mask.clip_mesh(&mesh, Pos2::ZERO, 1.);
    assert_eq!(clipped.texture_id, mesh.texture_id);
    assert!(!clipped.indices.is_empty());
    for v in clipped.vertices {
        let bottom = 30. + (v.pos.x - 50.).abs() * 0.8;
        assert!(v.pos.y <= bottom + 0.001);
        assert!((v.uv.x - v.pos.x / 100.).abs() < 0.00001);
        assert!((v.uv.y - v.pos.y / 100.).abs() < 0.00001);
    }
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(Default::default(), |ui| {
        let ctx = ui.ctx();
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::new("mask test"),
        ));
        painter.rect_filled(
            Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.)),
            0.,
            Color32::GREEN,
        );
        let start = ctx.graphics(|g| g.get(painter.layer_id()).unwrap().next_idx().0);
        painter.rect_filled(
            Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.)),
            0.,
            Color32::RED,
        );
        wireframe::render::mask::finish(&painter, start, &mask, Pos2::ZERO, 1.);
    });
    assert!(
        out.shapes
            .iter()
            .any(|s| matches!(&s.shape,egui::Shape::Rect(r) if r.fill==Color32::GREEN))
    );
    assert!(
        !out.shapes
            .iter()
            .any(|s| matches!(&s.shape,egui::Shape::Rect(r) if r.fill==Color32::RED))
    );
    assert!(
        out.shapes
            .iter()
            .any(|s| matches!(s.shape, egui::Shape::Mesh(_)))
    );
    out.textures_delta.clear();
    let edge = torn_edge(448., 161., 1.);
    assert_eq!(edge.first(), Some(&[0., 161.]));
    assert_eq!(edge.last(), Some(&[448., 161.]));
    assert!(edge.windows(2).all(|p| p[0][0] <= p[1][0]));
}
