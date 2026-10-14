// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    model::{Bounds, Kind},
    render::{Drawing, Scene},
    workspace::Editor,
};
fn edit(e: &mut Editor, id: Id, key: &str, value: Value) {
    e.begin("Edit multiline button");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
fn fixture() -> (Editor, Id) {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    e.add(Kind::MultilineButton, 50., 50.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
fn bounds(e: &Editor) -> Bounds {
    e.selected_views()[0].bounds
}
fn fixed(e: &mut Editor, w: f32, h: f32) {
    e.begin("Resize");
    e.selected_mut(|c| {
        c.bounds.w = w;
        c.bounds.h = h;
    });
    e.commit();
}
fn scene(e: &Editor) -> Scene {
    e.rendered_scene(e.page).unwrap()
}
#[test]
fn multiline_title_based_size_wrapping_fixed_dimensions_and_history() {
    let (mut e, id) = fixture();
    assert_eq!(bounds(&e).h, 66.);
    edit(&mut e, id, "text", Value::Text("".into()));
    assert_eq!(bounds(&e), Bounds::new(50., 50., 44., 66.));
    edit(&mut e, id, "text", Value::Text("Go".into()));
    assert_eq!(bounds(&e).w, 44.);
    edit(
        &mut e,
        id,
        "text",
        Value::Text("Go\nA longer second line wraps".into()),
    );
    let clean = bounds(&e);
    assert_eq!(clean.w, 44.);
    assert!(clean.h > 66.);
    e.document.settings.sketch = true;
    let sketch = bounds(&e);
    assert!(sketch.w > clean.w);
    assert!(sketch.h < clean.h);
    fixed(&mut e, 130., 80.);
    assert_eq!(bounds(&e), Bounds::new(50., 50., 130., 80.));
    let saved = e.document.clone();
    e.undo();
    assert_eq!(bounds(&e), sketch);
    e.redo();
    assert_eq!(e.document, saved);
    e.begin("Auto-size");
    e.auto_size_control(id).unwrap();
    e.commit();
    assert_eq!(bounds(&e), sketch);
    e.undo();
    assert_eq!(e.document, saved);
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), saved);
    let s = scene(&e);
    assert!(
        s.items
            .iter()
            .any(|i| matches!(&i.drawing,Drawing::Glyph(g) if g.size==10.))
    );
    edit(&mut e, id, "font_size", Value::Number(28.));
    assert!(
        scene(&e)
            .items
            .iter()
            .any(|i| matches!(&i.drawing,Drawing::Glyph(g) if g.size==20.))
    );
}
#[test]
fn multiline_icon_uses_both_natural_text_widths_for_fit_and_clips_text() {
    let (mut e, id) = fixture();
    edit(
        &mut e,
        id,
        "text",
        Value::Text("Go\nA long explanation".into()),
    );
    edit(
        &mut e,
        id,
        "icon",
        Value::Icon(IconSource::Catalog("arrow-right-solid".into())),
    );
    edit(&mut e, id, "icon_size", Value::Choice("48".into()));
    assert!(bounds(&e).w < 110.);
    assert!(
        !scene(&e)
            .items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Icon { .. }))
    );
    fixed(&mut e, 250., 80.);
    edit(&mut e, id, "icon_rotation", Value::Choice("90".into()));
    let s = scene(&e);
    assert!(
        s.items
            .iter()
            .any(|i| matches!(i.drawing,Drawing::Icon{turns:1,bounds:b,..} if b.w==48.))
    );
    assert!(s.items.iter().all(|i| i.clip == Some(bounds(&e))));
    edit(&mut e, id, "icon", Value::Null);
    assert!(
        !scene(&e)
            .items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Icon { .. }))
    );
    e.undo();
    assert!(
        scene(&e)
            .items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Icon { .. }))
    );
    fixed(&mut e, 20., 8.);
    let s = scene(&e);
    assert!(
        !s.items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Glyph(_) | Drawing::Icon { .. }))
    );
}
#[test]
fn multiline_opacity_threshold_preserves_ink_border_and_inline_link_priority() {
    let (mut e, id) = fixture();
    edit(
        &mut e,
        id,
        "text",
        Value::Text("Go\n[More](https://example.org/more)".into()),
    );
    edit(
        &mut e,
        id,
        "link",
        Value::Link(Link::Url("https://example.org/control".into())),
    );
    edit(&mut e, id, "fill", Value::Color([0, 0, 0, 255]));
    fixed(&mut e, 130., 80.);
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        for alpha in [0., 0.5, 0.749, 0.75, 1.] {
            edit(&mut e, id, "background_opacity", Value::Number(alpha));
            let s = scene(&e);
            assert_eq!(s.items[0].opacity, alpha as f32);
            assert!(s.items[1..].iter().all(|i| i.opacity == 1.));
            let colors = s
                .items
                .iter()
                .filter_map(|i| {
                    if let Drawing::Glyph(g) = &i.drawing {
                        Some(g.color)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            let normal = if sketch {
                [0, 0, 0, 255]
            } else {
                [51, 51, 51, 255]
            };
            assert_eq!(colors[0], if alpha >= 0.75 { [255; 4] } else { normal });
            assert_eq!(
                *colors.last().unwrap(),
                if alpha >= 0.75 {
                    [255; 4]
                } else {
                    [8, 83, 148, 255]
                }
            );
            let inline = s
                .links
                .iter()
                .find(|l| l.target.ends_with("/more"))
                .unwrap();
            assert_eq!(
                s.hit_link(inline.bounds.x + 1., inline.bounds.y + 1.),
                Some("https://example.org/more")
            );
            assert_eq!(s.hit_link(51., 51.), Some("https://example.org/control"));
            assert_eq!(s.hit_link(49., 51.), None);
            assert!(s.png().is_ok());
            assert!(s.svg().contains("https://example.org/more"));
        }
    }
    let saved = e.document.clone();
    assert!(
        e.set_control_property(id, "background_opacity", Value::Number(1.1))
            .is_err()
    );
    assert_eq!(e.document, saved);
}
