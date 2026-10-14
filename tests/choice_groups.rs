// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    choice_group::{ChoiceGroup, Mark, parse_row},
    document::*,
    model::{Bounds, Kind},
    workspace::Editor,
};

fn edit(e: &mut Editor, id: Id, key: &str, value: Value) {
    e.begin("Edit choice rows");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
fn fixture(kind: Kind) -> (Editor, Id, Id) {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    let first = e.page;
    e.add_page();
    let dest = e.page;
    e.switch_page(first);
    e.add(kind, 50., 50.);
    let id = *e.selection.iter().next().unwrap();
    (e, id, dest)
}
fn resize(e: &mut Editor, height: f32) {
    e.begin("Resize choice group");
    e.selected_mut(|c| c.bounds = Bounds::new(50., 50., 100., height));
    e.commit();
}
#[test]
fn marker_states_require_the_correct_kind_and_preserve_formatted_labels() {
    for radio in [false, true] {
        let (l, r) = if radio { ('(', ')') } else { ('[', ']') };
        for (marker, mark) in [
            ("", Mark::Empty),
            (" ", Mark::Empty),
            ("x", Mark::Selected),
            ("X", Mark::Selected),
            ("o", Mark::Selected),
            ("O", Mark::Selected),
            ("v", Mark::Selected),
            ("V", Mark::Selected),
            ("*", Mark::Selected),
            ("-", Mark::Mixed),
            ("=", Mark::Mixed),
        ] {
            for disabled in [false, true] {
                let source = format!("{l}{marker}{r}  *Label*  ");
                let source = if disabled {
                    format!("-{source}-")
                } else {
                    source
                };
                let row = parse_row(&source, radio);
                assert_eq!(row.mark, Some(mark), "{source}");
                assert_eq!(row.disabled, disabled);
                assert_eq!(row.text.display_text(true), "Label");
            }
        }
        for input in [" [x] label", "[  ] label", "Plain", ""] {
            assert_eq!(parse_row(input, radio).mark, None);
        }
        assert_eq!(
            parse_row(if radio { "[x] label" } else { "(o) label" }, radio).mark,
            None
        );
    }
    let red = parse_row("{color:red}[x] Red{color}", false);
    assert_eq!(red.symbol_color.as_deref(), Some("red"));
    assert_eq!(red.text.display_text(true), "Red");
    let blue = parse_row("{colour:blue}[ ]{colour} Label", false);
    assert_eq!(blue.symbol_color.as_deref(), Some("blue"));
    assert_eq!(blue.text.source, "Label");
    assert_eq!(
        parse_row("(o) First\\rsecond", true)
            .text
            .display_text(true),
        "First\nsecond"
    );
    assert!(ChoiceGroup::parse(&"a".repeat(100001), false).is_err());
    assert!(ChoiceGroup::parse(&"\n".repeat(10000), true).is_err());
}
#[test]
fn complete_rows_and_links_share_height_boundaries_in_both_styles() {
    for kind in [Kind::CheckboxGroup, Kind::RadioGroup] {
        let (mut e, id, dest) = fixture(kind);
        let source = if kind == Kind::RadioGroup {
            "( ) First\n-(o) Second-\n(-) Third"
        } else {
            "[ ] First\n-[x] Second-\n[-] Third"
        };
        edit(&mut e, id, "text", Value::Text(source.into()));
        edit(
            &mut e,
            id,
            "row_links",
            Value::Links(vec![
                Some(Link::Back),
                Some(Link::Page(dest)),
                Some(Link::Url("https://example.org/third".into())),
            ]),
        );
        assert_eq!(e.selected_views()[0].bounds.h, 61.);
        for sketch in [false, true] {
            e.document.settings.sketch = sketch;
            for (height, count) in [(36., 1), (37., 2), (58., 2), (59., 3)] {
                resize(&mut e, height);
                let s = e.rendered_scene(e.page).unwrap();
                assert_eq!(s.links.len(), count);
                assert_eq!(s.hit_link(145., 55.), Some("back:"));
                assert_eq!(s.hit_link(55., 72.), None); // Nine-unit gap has no target.
                if count > 1 {
                    assert_eq!(s.hit_link(145., 80.), Some(format!("page:{dest}").as_str()));
                }
                if count < 3 {
                    assert!(!s.svg().contains("https://example.org/third"));
                }
            }
        }
        e.begin("Auto-size");
        e.auto_size_control(id).unwrap();
        e.commit();
        assert_eq!(e.selected_views()[0].bounds.h, 61.);
        let before = e.document.clone();
        e.undo();
        assert_eq!(e.selected_views()[0].bounds.h, 59.);
        e.redo();
        assert_eq!(e.document, before);
        assert_eq!(decode(&before.encode().unwrap()).unwrap(), before);
    }
}
#[test]
fn small_empty_labels_plain_rows_and_soft_breaks_keep_distinct_heights() {
    let (mut e, id, _) = fixture(Kind::CheckboxGroup);
    assert_eq!(e.selected_views()[0].bounds.h, 149.);
    edit(&mut e, id, "text", Value::Text("[ ]\n[x]\n[-]".into()));
    edit(&mut e, id, "font_size", Value::Number(8.));
    assert_eq!(e.selected_views()[0].bounds.h, 58.);
    assert_eq!(e.selected_views()[0].bounds.w, 20.);
    edit(&mut e, id, "font_size", Value::Number(13.));
    edit(&mut e, id, "text", Value::Text("Plain\n\n[x] Last".into()));
    assert_eq!(e.selected_views()[0].bounds.h, 61.);
    let (mut e, id, _) = fixture(Kind::RadioGroup);
    assert_eq!(e.selected_views()[0].bounds.h, 181.);
    edit(&mut e, id, "font_size", Value::Number(24.));
    edit(
        &mut e,
        id,
        "text",
        Value::Text("(o) First\\rsecond\n( ) Last".into()),
    );
    assert_eq!(e.selected_views()[0].bounds.h, 90.);
}
#[test]
fn edits_preserve_row_links_through_history_components_and_storage() {
    let (mut e, id, dest) = fixture(Kind::CheckboxGroup);
    edit(
        &mut e,
        id,
        "text",
        Value::Text("[ ] A\n[x] B\nPlain".into()),
    );
    edit(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![Some(Link::Page(dest)), None, Some(Link::Back)]),
    );
    let before = e.document.clone();
    e.begin("Edit text");
    e.selected_mut(|c| c.text = "Plain\n[ ] A\n[x] B".into());
    e.commit();
    assert_eq!(
        e.control_properties(id).unwrap().1["row_links"],
        Value::Links(vec![Some(Link::Back), Some(Link::Page(dest)), None])
    );
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    e.create_component("Choices".into()).unwrap();
    let definitions = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    edit(
        &mut e,
        child,
        "text",
        Value::Text("[ ] A\nPlain\n[x] B".into()),
    );
    assert_eq!(e.document.definitions, definitions);
    assert_eq!(
        e.control_properties(child).unwrap().1["row_links"],
        Value::Links(vec![Some(Link::Page(dest)), Some(Link::Back), None])
    );
    e.reset_local_changes(child).unwrap();
    assert_eq!(
        e.control_properties(child).unwrap().1["text"],
        Value::Text("Plain\n[ ] A\n[x] B".into())
    );
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
}

#[test]
fn disabled_rows_keep_explicit_inline_colors_without_recoloring_the_symbol() {
    use wireframe::{render::Drawing, scene::Primitive};
    for kind in [Kind::CheckboxGroup, Kind::RadioGroup] {
        let (mut e, id, dest) = fixture(kind);
        let marker = if kind == Kind::RadioGroup {
            "(o)"
        } else {
            "[x]"
        };
        edit(
            &mut e,
            id,
            "text",
            Value::Text(format!(
                "-{marker} {{color:#ff0000}}Red{{color}}-\n-{marker} Plain-"
            )),
        );
        edit(
            &mut e,
            id,
            "row_links",
            Value::Links(vec![Some(Link::Page(dest)); 2]),
        );
        for sketch in [false, true] {
            e.document.settings.sketch = sketch;
            let scene = e.rendered_scene(e.page).unwrap();
            let colors = scene
                .items
                .iter()
                .filter_map(|item| match &item.drawing {
                    Drawing::Glyph(g) => Some(g.color),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(colors.contains(&[255, 0, 0, 255]));
            assert!(colors.contains(&[128, 128, 139, 255]));
            for item in &scene.items {
                match &item.drawing {
                    Drawing::Primitive(Primitive::Line { color, .. }) => {
                        assert_eq!(*color, [128, 128, 139])
                    }
                    Drawing::Icon { color, .. } => assert_eq!(*color, [128, 128, 139, 255]),
                    _ => {}
                }
            }
            assert_eq!(scene.links.len(), 2);
            let svg = scene.svg();
            assert!(svg.contains("fill=\"rgb(255,0,0)\"") || svg.contains("fill=\"#ff0000\""));
        }
    }
}

#[test]
fn choice_artwork_keeps_optical_bounds_and_negative_half_alignment() {
    use wireframe::render::Drawing;
    for (kind, marker, pixels) in [
        (Kind::CheckboxGroup, "[ ]", (14, 14)),
        (Kind::RadioGroup, "( )", (13, 13)),
    ] {
        let (mut e, id, _) = fixture(kind);
        edit(&mut e, id, "text", Value::Text(marker.into()));
        let scene = e.rendered_scene(e.page).unwrap();
        let data = scene.png().unwrap();
        let raster = image::load_from_memory(&data).unwrap().to_rgba8();
        let mut bounds = (u32::MAX, u32::MAX, 0, 0);
        for (x, y, p) in raster.enumerate_pixels() {
            if p[3] > 24 && p[0] < 230 && p[1] < 230 && p[2] < 230 {
                bounds = (
                    bounds.0.min(x),
                    bounds.1.min(y),
                    bounds.2.max(x + 1),
                    bounds.3.max(y + 1),
                );
            }
        }
        assert_eq!((bounds.2 - bounds.0, bounds.3 - bounds.1), pixels);
        assert!(scene.svg().contains("Font Awesome Free"));
        // Thirteen-point checkbox labels make the 16-unit icon taller than its row.
        edit(&mut e, id, "text", Value::Text(format!("{marker} First")));
        let scene = e.rendered_scene(e.page).unwrap();
        let symbol = scene
            .items
            .iter()
            .find_map(|i| match i.drawing {
                Drawing::Icon { bounds, .. } => Some(bounds),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            symbol.y,
            if kind == Kind::CheckboxGroup {
                51.
            } else {
                52.
            }
        );
    }
}

#[test]
fn choice_ink_tracks_style_without_replacing_explicit_colors() {
    use wireframe::render::Drawing;
    fn assert_ink(e: &Editor, expected: [u8; 4]) {
        let scene = e.rendered_scene(e.page).unwrap();
        let mut symbols = 0;
        let mut labels = 0;
        for item in scene.items {
            match item.drawing {
                Drawing::Icon { color, .. } => {
                    symbols += 1;
                    assert_eq!(color, expected);
                }
                Drawing::Glyph(g) => {
                    labels += 1;
                    assert_eq!(g.color, expected);
                }
                _ => {}
            }
        }
        assert_eq!(symbols, 1);
        assert!(labels > 0);
    }
    for (kind, text) in [
        (Kind::Checkbox, "Choice"),
        (Kind::Radio, "Choice"),
        (Kind::CheckboxGroup, "[x] Choice"),
        (Kind::RadioGroup, "(o) Choice"),
    ] {
        let mut e = Editor::default();
        e.add(kind, 50., 50.);
        let id = *e.selection.iter().next().unwrap();
        edit(&mut e, id, "text", Value::Text(text.into()));
        assert_ink(&e, [0, 0, 0, 255]);
        e.edit("Clean style", |e| e.document.settings.sketch = false);
        assert_ink(&e, [51, 51, 51, 255]);
        e.undo();
        assert_ink(&e, [0, 0, 0, 255]);
        e.redo();
        assert_ink(&e, [51, 51, 51, 255]);
        for color in [[0, 0, 0, 255], [30, 70, 110, 255]] {
            edit(&mut e, id, "text_color", Value::Color(color));
            for sketch in [true, false] {
                e.document.settings.sketch = sketch;
                assert_ink(&e, color);
                let restored = Editor::new(decode(&e.document.encode().unwrap()).unwrap());
                assert_ink(&restored, color);
            }
        }
        e.reset_control_property(id, "text_color").unwrap();
        assert_ink(&e, [51, 51, 51, 255]);
    }
}
