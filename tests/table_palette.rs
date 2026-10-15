// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{self, Value},
    model::Kind,
    render::{Drawing, Scene},
    scene::Primitive,
    workspace::Editor,
};

fn fixture(sketch: bool, header: bool, selected: i32) -> (Editor, u64) {
    let mut e = Editor::default();
    e.document.settings.sketch = sketch;
    e.add(Kind::Table, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    for (key, value) in [
        ("text", Value::Text("Heading ^, [Link](https://example.test/), Ready\nFirst, [Open](https://example.test/first), [x]\nSecond, [Next](https://example.test/next), (o)\nThird, Plain, [-]\n{45L,35L,20C}".into())),
        ("font_size", Value::Number(16.)),
        ("header", Value::Bool(header)),
        ("selected_index", Value::Number(selected as f64)),
        ("row_height", Value::Number(36.)),
        ("horizontal_lines", Value::Bool(true)),
    ] { e.set_control_property(id, key, value).unwrap(); }
    e.selected_mut(|c| {
        c.bounds.w = 440.;
        c.bounds.h = 220.;
    });
    (e, id)
}
fn scene(e: &Editor) -> Scene {
    e.rendered_scene(e.page).unwrap()
}
fn ink(scene: &Scene, text: &str) -> [u8; 4] {
    scene
        .items
        .iter()
        .find_map(|item| match &item.drawing {
            Drawing::Glyph(g) if g.text == text => Some(g.color),
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing text {text}"))
}
fn rows(scene: &Scene) -> Vec<[u8; 3]> {
    scene
        .items
        .iter()
        .filter_map(|item| match &item.drawing {
            Drawing::Primitive(Primitive::Rect {
                b,
                fill,
                stroke: false,
                ..
            }) if b.w == 440. && b.h < 220. => Some(*fill),
            _ => None,
        })
        .collect()
}
#[test]
fn header_selection_offsets_and_empty_filler_rows_use_the_right_palette() {
    for sketch in [false, true] {
        for header in [false, true] {
            let (mut e, id) = fixture(sketch, header, 0);
            let s = scene(&e);
            let r = rows(&s);
            if header {
                assert_eq!(r[0], [204; 3]);
            }
            assert_eq!(r[usize::from(header)], [149, 188, 242]);
            assert_eq!(
                ink(&s, "First"),
                if sketch {
                    [0, 0, 0, 255]
                } else {
                    [51, 51, 51, 255]
                }
            );
            assert_eq!(ink(&s, "Open"), [8, 83, 148, 255]);
            e.set_control_property(id, "selected_index", Value::Number(5.))
                .unwrap();
            assert!(!rows(&scene(&e)).contains(&[149, 188, 242]));
        }
    }
}
#[test]
fn every_row_resolves_text_links_sort_and_choices_at_the_contrast_boundary() {
    for sketch in [false, true] {
        let (mut e, id) = fixture(sketch, false, -1);
        e.set_control_property(id, "fill", Value::Color([152, 152, 152, 255]))
            .unwrap();
        e.set_control_property(id, "alternate_fill", Value::Color([153, 153, 153, 255]))
            .unwrap();
        let s = scene(&e);
        let dark = if sketch {
            [0, 0, 0, 255]
        } else {
            [51, 51, 51, 255]
        };
        for text in ["Heading", "Link", "Second", "Next"] {
            assert_eq!(ink(&s, text), [255; 4]);
        }
        for text in ["First", "Third", "Plain"] {
            assert_eq!(ink(&s, text), dark);
        }
        assert_eq!(ink(&s, "Open"), [8, 83, 148, 255]);
        let icons: Vec<_> = s
            .items
            .iter()
            .filter_map(|i| match &i.drawing {
                Drawing::Icon {
                    id, color, bounds, ..
                } => Some((id.as_str(), *color, *bounds)),
                _ => None,
            })
            .collect();
        assert_eq!(icons.len(), 3);
        assert_eq!(icons[0].1, dark);
        assert_eq!(icons[1].1, [255; 4]);
        assert_eq!(icons[2].1, dark);
        assert!(s.items.iter().any(|i| matches!(&i.drawing,Drawing::Primitive(Primitive::Polygon{fill,points}) if *fill==[255;3] && points.len()==3)));
        let png = image::load_from_memory(&s.png().unwrap())
            .unwrap()
            .into_rgba8();
        // Inspect the actual radio icon region, including visible white ink and the
        // original row background through its transparent interior.
        let b = icons[1].2;
        let crop = image::imageops::crop_imm(
            &png,
            (b.x - s.bounds.x + 24.) as u32,
            (b.y - s.bounds.y + 24.) as u32,
            b.w as u32,
            b.h as u32,
        )
        .to_image();
        assert!(
            crop.pixels()
                .any(|p| p[0] > 230 && p[1] > 230 && p[2] > 230)
        );
        assert!(crop.pixels().any(|p| p.0 == [152, 152, 152, 255]));
        assert!(s.svg().contains("#ffffff"));
        assert_eq!(s.links.len(), 3);
    }
}
#[test]
fn explicit_colors_disabled_state_history_and_storage_remain_authoritative() {
    let (mut e, id) = fixture(false, false, -1);
    e.set_control_property(id, "fill", Value::Color([34, 34, 34, 255]))
        .unwrap();
    e.set_control_property(id, "alternate_fill", Value::Color([34, 34, 34, 255]))
        .unwrap();
    let before = e.document.clone();
    e.edit("Explicit table colors", |e| {
        e.set_control_property(id, "text_color", Value::Color([51, 51, 51, 255]))
            .unwrap();
        // Unknown persisted colors remain supported without introducing a new inspector field.
        e.document.pages[0].content.nodes[0]
            .properties
            .insert("link_color".into(), Value::Color([8, 83, 148, 255]));
    });
    let s = scene(&e);
    assert_eq!(ink(&s, "Heading"), [51, 51, 51, 255]);
    assert_eq!(ink(&s, "Link"), [8, 83, 148, 255]);
    e.edit("Change style", |e| e.document.settings.sketch = true);
    assert_eq!(ink(&scene(&e), "Heading"), [51, 51, 51, 255]);
    let bytes = e.document.encode().unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("__table_auto"));
    assert_eq!(document::decode(&bytes).unwrap(), e.document);
    e.undo();
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(ink(&scene(&e), "Heading"), [255; 4]);
    e.document.pages[0].content.nodes[0]
        .properties
        .insert("state".into(), Value::Choice("disabled".into()));
    let disabled = scene(&e);
    assert_eq!(ink(&disabled, "Heading"), [128, 128, 139, 255]);
    assert_eq!(ink(&disabled, "Link"), [128, 128, 139, 255]);
}
#[test]
fn custom_blue_background_is_not_mistaken_for_selection_and_rules_follow_style() {
    for sketch in [false, true] {
        let (mut e, id) = fixture(sketch, false, -1);
        e.set_control_property(id, "fill", Value::Color([0, 102, 204, 255]))
            .unwrap();
        e.set_control_property(
            id,
            "text",
            Value::Text(
                "{color:#dd2244}Accent{color}, [Link](https://example.test/)\nPlain, Next".into(),
            ),
        )
        .unwrap();
        assert_eq!(ink(&scene(&e), "Accent"), [221, 34, 68, 255]);
        e.set_control_property(id, "text_color", Value::Color([230, 230, 0, 255]))
            .unwrap();
        let s = scene(&e);
        assert_eq!(ink(&s, "Plain"), [230, 230, 0, 255]);
        let rule = if sketch { [0; 3] } else { [51; 3] };
        assert!(s.items.iter().any(|i|matches!(&i.drawing,Drawing::Primitive(Primitive::Line{points,color,..}) if points.len()==2 && (points[0][0]-points[1][0]).abs()==440. && *color==rule)));
    }
}
