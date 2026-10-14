// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    app_bar::{self, AppBar},
    document::*,
    layout::{TextMeasurer, TextMetrics, TextStyle},
    model::Kind,
    render::Drawing,
    workspace::{Editor, ImageTarget, ImportChoice},
};
struct Metrics;
impl TextMeasurer for Metrics {
    fn measure(&self, text: &str, style: TextStyle, _: Option<f32>) -> TextMetrics {
        assert!(!style.bold && !style.italic);
        TextMetrics {
            width: text
                .split('\n')
                .map(|s| s.chars().count() as f32 * 8.)
                .fold(0., f32::max),
            height: text.split('\n').count() as f32 * style.size,
            rows: text.split('\n').count(),
        }
    }
}
fn icons(names: &[&str]) -> Value {
    Value::Icons(
        names
            .iter()
            .map(|s| Some(IconSource::Catalog((*s).into())))
            .collect(),
    )
}
fn fixture() -> (Editor, Id) {
    let mut e = Editor::default();
    e.add(Kind::AppBar, 50., 50.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
fn set(e: &mut Editor, id: Id, key: &str, value: Value) {
    e.begin("Edit app bar");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
    assert!(e.error.is_none(), "{:?}", e.error);
}
#[test]
fn layout_strict_thresholds_keep_source_slots_and_unclipped_title_bounds() {
    let mut p = Properties::new();
    p.insert("icons".into(), icons(&["star-regular", "heart-solid"]));
    p.insert("nav_icon".into(), Value::Bool(false));
    let bar = AppBar::parse("Title", &p).unwrap();
    let m = bar.measure(&Metrics, 16.);
    assert_eq!((m.width, m.height), (320., 44.));
    assert_eq!(
        bar.layout(&m, 140., 44.)
            .icons
            .iter()
            .map(|i| i.index)
            .collect::<Vec<_>>(),
        [1, 4]
    );
    let layout = bar.layout(&m, 141., 44.);
    assert_eq!(
        layout.icons.iter().map(|i| i.index).collect::<Vec<_>>(),
        [1, 3, 4]
    );
    assert_eq!(layout.icons[1].bounds.x, 83.);
    assert_eq!(layout.icons[2].link.x, 110.);
    assert!(bar.layout(&m, 320., 30.).icons.is_empty());
    assert!(bar.layout(&m, 320., 30.).title.is_none());
    assert_eq!(bar.layout(&m, 320., 31.).icons.len(), 4);
    let bar = AppBar::parse("An overlong title", &Properties::new()).unwrap();
    let m = bar.measure(&Metrics, 16.);
    let narrow = bar.layout(&m, 80., 44.);
    assert_eq!(narrow.icons.len(), 1);
    assert_eq!(narrow.icons[0].index, 0);
    let title = narrow.title.unwrap();
    assert!(title.x + title.w > 80.);
    assert!(bar.layout(&m, 29., 44.).icons.iter().all(|i| i.index != 0));
    assert!(bar.layout(&m, 30., 44.).icons.iter().any(|i| i.index == 0));
}
#[test]
fn intrinsic_size_retains_soft_breaks_and_fixed_font_style() {
    let (e, id) = fixture();
    let (spec, p) = e.control_properties(id).unwrap();
    assert_eq!(p["font_size"], Value::Number(16.));
    assert_eq!(p["fill"], Value::Color([221, 221, 221, 255]));
    for key in ["bold", "italic", "underline", "selected_index", "border"] {
        assert!(spec.property(key).is_none_or(|p| !p.visible));
    }
    for key in ["icons", "row_links", "nav_icon", "filter_icon", "menu_icon"] {
        assert!(spec.property(key).unwrap().visible);
    }
    for source in ["Title\ncontinued", "Title\\rcontinued"] {
        let bar = AppBar::parse(source, &p).unwrap();
        assert_eq!(bar.measure(&Metrics, 16.).height, 47.);
    }
    let bar = AppBar::parse(&"x".repeat(100), &p).unwrap();
    assert_eq!(bar.measure(&Metrics, 16.).width, 919.);
    assert!(AppBar::parse(&"x".repeat(100_001), &p).is_err());
    assert_eq!(app_bar::foreground([152, 152, 152, 255]), [255; 4]);
    assert_eq!(app_bar::foreground([153, 153, 153, 255]), [0, 0, 0, 255]);
    assert_eq!(app_bar::foreground([255, 0, 0, 255]), [255; 4]);
    assert_eq!(app_bar::foreground([255, 255, 0, 255]), [0, 0, 0, 255]);
}
#[test]
fn duplicate_icon_destinations_follow_occurrences_and_title_edits_leave_them_alone() {
    let (mut e, id) = fixture();
    set(
        &mut e,
        id,
        "icons",
        icons(&["star-regular", "star-regular", "heart-solid"]),
    );
    assert!(!e.nodes()[0].properties.contains_key("row_links"));
    let links = vec![
        Some(Link::Back),
        None,
        Some(Link::Url("https://example.org/first".into())),
        Some(Link::Url("https://example.org/second".into())),
        Some(Link::Url("https://example.org/heart".into())),
        Some(Link::Page(e.page)),
    ];
    set(&mut e, id, "row_links", Value::Links(links.clone()));
    let before = e.document.clone();
    e.begin("Reorder icons");
    e.set_control_property(id, "icons", icons(&[])).unwrap();
    e.set_control_property(
        id,
        "icons",
        icons(&["heart-solid", "star-regular", "star-regular", "plus-solid"]),
    )
    .unwrap();
    e.commit();
    assert_eq!(
        e.control_properties(id).unwrap().1["row_links"],
        Value::Links(vec![
            links[0].clone(),
            links[1].clone(),
            links[4].clone(),
            links[2].clone(),
            links[3].clone(),
            None,
            links[5].clone()
        ])
    );
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    let p = e.control_properties(id).unwrap().1;
    set(&mut e, id, "text", Value::Text("Different\nheading".into()));
    let changed = e.control_properties(id).unwrap().1;
    assert_eq!(changed["row_links"], p["row_links"]);
    assert_eq!(changed["icons"], p["icons"]);
    e.begin("Reset icons");
    e.reset_control_property(id, "icons").unwrap();
    e.commit();
    assert_eq!(
        e.control_properties(id).unwrap().1["row_links"],
        Value::Links(vec![links[0].clone(), links[1].clone(), links[5].clone()])
    );
    assert!(!e.nodes()[0].properties.contains_key("icons"));
    e.undo();
    assert_eq!(e.control_properties(id).unwrap().1["icons"], p["icons"]);
}
#[test]
fn component_icon_edits_and_reset_are_local_and_invalid_edits_are_atomic() {
    let (mut e, id) = fixture();
    let page = e.page;
    set(&mut e, id, "icons", icons(&["star-regular", "heart-solid"]));
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(vec![
            None,
            None,
            Some(Link::Back),
            Some(Link::Page(page)),
            None,
        ]),
    );
    e.create_component("Toolbar".into()).unwrap();
    let definitions = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    set(
        &mut e,
        child,
        "icons",
        icons(&["heart-solid", "star-regular"]),
    );
    assert_eq!(e.document.definitions, definitions);
    assert_eq!(
        e.control_properties(child).unwrap().1["row_links"],
        Value::Links(vec![
            None,
            None,
            Some(Link::Page(page)),
            Some(Link::Back),
            None
        ])
    );
    e.begin("Reset collection");
    e.reset_control_property(child, "icons").unwrap();
    e.commit();
    assert_eq!(
        e.control_properties(child).unwrap().1["icons"],
        icons(&["star-regular", "heart-solid"])
    );
    assert!(!e.explicit_control_properties(child).contains_key("icons"));
    assert_eq!(
        e.control_properties(child).unwrap().1["row_links"],
        Value::Links(vec![
            None,
            None,
            Some(Link::Back),
            Some(Link::Page(page)),
            None
        ])
    );
    let before = e.document.clone();
    assert!(
        e.set_control_property(child, "icons", icons(&["bad id"]))
            .is_err()
    );
    assert!(
        e.set_control_property(child, "icons", Value::Bool(true))
            .is_err()
    );
    assert_eq!(e.document, before);
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
}
#[test]
fn presentation_links_exclude_titles_and_hidden_icons_in_both_styles() {
    let (mut e, id) = fixture();
    set(
        &mut e,
        id,
        "text",
        Value::Text("[Title](https://example.org)".into()),
    );
    assert!(e.rendered_scene(e.page).unwrap().links.is_empty());
    set(&mut e, id, "text", Value::Text("Title".into()));
    set(&mut e, id, "nav_icon", Value::Bool(false));
    set(&mut e, id, "icons", icons(&["star-regular", "heart-solid"]));
    set(
        &mut e,
        id,
        "row_links",
        Value::Links(
            (0..5)
                .map(|i| Some(Link::Url(format!("https://example.org/{i}"))))
                .collect(),
        ),
    );
    e.begin("Narrow bar");
    e.selected_mut(|c| {
        c.bounds.w = 140.;
        c.bounds.h = 44.;
    });
    e.commit();
    for sketch in [false, true] {
        e.document.settings.sketch = sketch;
        let scene = e.rendered_scene(e.page).unwrap();
        let mut targets: Vec<_> = scene.links.iter().map(|l| l.target.as_str()).collect();
        targets.sort();
        assert_eq!(
            targets,
            [
                "https://example.org/1",
                "https://example.org/3",
                "https://example.org/4"
            ]
        );
        assert!(scene.png().is_ok());
        assert!(scene.svg().contains("clipPath"));
    }
    let before = e.document.clone();
    e.begin("Auto-size");
    e.auto_size_control(id).unwrap();
    e.commit();
    assert_eq!(e.selected_views()[0].bounds.w, 320.);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.selected_views()[0].bounds.w, 320.);
    set(&mut e, id, "bold", Value::Bool(true));
    set(&mut e, id, "italic", Value::Bool(true));
    set(&mut e, id, "underline", Value::Bool(true));
    let styled = e.rendered_scene(e.page).unwrap().svg();
    set(&mut e, id, "bold", Value::Bool(false));
    set(&mut e, id, "italic", Value::Bool(false));
    set(&mut e, id, "underline", Value::Bool(false));
    assert_eq!(e.rendered_scene(e.page).unwrap().svg(), styled);
}
#[test]
fn imported_icon_assets_have_atomic_history_and_survive_storage() {
    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
    use std::io::Cursor;
    let (mut e, id) = fixture();
    let before = e.document.clone();
    let img = RgbaImage::from_pixel(20, 10, Rgba([0, 0, 0, 255]));
    let mut bytes = Cursor::new(vec![]);
    DynamicImage::ImageRgba8(img)
        .write_to(&mut bytes, ImageFormat::Png)
        .unwrap();
    let asset = e
        .import_image_at(
            ImageTarget::AppBarIcon {
                control: id,
                index: 0,
            },
            wireframe::images::Prepared::from_bytes("Tool.png", bytes.get_ref()).unwrap(),
            ImportChoice::KeepBoth,
        )
        .unwrap();
    assert_eq!(
        e.control_properties(id).unwrap().1["icons"],
        Value::Icons(vec![Some(IconSource::Asset(asset))])
    );
    assert!(
        e.rendered_scene(e.page)
            .unwrap()
            .items
            .iter()
            .any(|i| matches!(i.drawing, Drawing::Image { .. }))
    );
    let saved = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, saved);
    assert_eq!(decode(&saved.encode().unwrap()).unwrap(), saved);
    let error = e.import_image_at(
        ImageTarget::AppBarIcon {
            control: id,
            index: 4,
        },
        wireframe::images::Prepared::from_bytes("Other.png", bytes.get_ref()).unwrap(),
        ImportChoice::KeepBoth,
    );
    assert!(error.is_err());
    assert_eq!(e.document, saved);
}
