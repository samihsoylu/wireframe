// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, model::Kind, pdf, workspace::Editor};

#[test]
fn clean_button_corners_reveal_underlying_artwork_without_changing_fill_or_bounds() {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    e.add(Kind::Rectangle, 0., 0.);
    let back = e.nodes()[0].id;
    let blue = [16, 140, 211];
    e.set_control_property(back, "fill", Value::Color([blue[0], blue[1], blue[2], 255]))
        .unwrap();
    e.add(Kind::Button, 20., 20.);
    let id = e.nodes()[1].id;
    e.set_control_property(id, "text", Value::Text(String::new()))
        .unwrap();
    e.selected_mut(|c| {
        c.bounds.w = 80.;
        c.bounds.h = 30.;
    });
    let bounds = e.current().controls[1].bounds;
    let image = image::load_from_memory(&e.rendered_scene(e.page).unwrap().png().unwrap())
        .unwrap()
        .to_rgba8();
    for (x, y) in [(44, 44), (123, 44), (44, 73), (123, 73)] {
        let corner = image.get_pixel(x, y).0;
        assert!(
            corner[..3]
                .iter()
                .zip(blue)
                .all(|(a, b)| a.abs_diff(b) <= 2),
            "{x},{y}: {corner:?}"
        );
    }
    assert_eq!(image.get_pixel(84, 44).0, [51, 51, 51, 255]);
    assert_eq!(image.get_pixel(50, 50).0, [221, 221, 221, 255]);
    assert_eq!(e.current().controls[1].bounds, bounds);
}

#[test]
fn tiny_saved_clean_buttons_render_without_negative_inner_geometry() {
    for (w, h) in [(1., 1.), (2., 2.), (3., 1.), (3., 3.), (6., 4.)] {
        let mut p = Project::default();
        p.settings.sketch = false;
        let id = p.allocate_id().unwrap();
        let mut n = wireframe::catalog::get("button")
            .unwrap()
            .create(id, Point::new(0., 0.));
        n.content = NodeContent::Control {
            kind: "button".into(),
            size: Size {
                width: Dimension::Fixed(w),
                height: Dimension::Fixed(h),
            },
        };
        n.properties
            .insert("text".into(), Value::Text(String::new()));
        p.pages[0].content.nodes.push(n);
        let e = Editor::new(p);
        let scene = e.rendered_scene(e.page).unwrap();
        assert!(scene.png().unwrap().starts_with(b"\x89PNG"));
        let pages = vec![pdf::Page {
            scene,
            decoration: None,
            size: [w, h],
            origin: [0., 0.],
            scale: 1.,
            links: vec![],
        }];
        assert!(
            pdf::encode("Small buttons", &pages)
                .unwrap()
                .starts_with(b"%PDF")
        );
    }
}
