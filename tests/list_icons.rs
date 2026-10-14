// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    icons,
    model::{Bounds, Kind},
    render::Drawing,
    workspace::Editor,
};
fn icon(id: &str) -> Option<IconSource> {
    Some(IconSource::Catalog(id.into()))
}
fn set(e: &mut Editor, id: Id, key: &str, value: Value) {
    e.begin("List edit");
    e.set_control_property(id, key, value).unwrap();
    e.commit();
}
fn fixture() -> (Editor, Id) {
    let mut e = Editor::default();
    e.document.settings.sketch = false;
    e.add(Kind::List, 50., 50.);
    let id = *e.selection.iter().next().unwrap();
    (e, id)
}
fn values(e: &Editor, id: Id) -> Vec<Option<IconSource>> {
    match &e.control_properties(id).unwrap().1["row_icons"] {
        Value::Icons(v) => v.clone(),
        _ => panic!(),
    }
}

#[test]
fn public_catalog_is_complete_searchable_and_renderable_offline() {
    assert_eq!(icons::all().len(), 2060);
    assert!(icons::all().windows(2).all(|pair| pair[0].id < pair[1].id));
    assert_eq!(icons::search("camera", "")[0].id, "camera-solid");
    assert!(
        icons::search("photo", "solid")
            .iter()
            .any(|i| i.id == "camera-solid")
    );
    assert!(
        icons::search("heart", "regular")
            .iter()
            .all(|i| i.style == "regular")
    );
    assert!(icons::search("no-matching-icon-4321", "").is_empty());
    for i in icons::all() {
        assert!(i.width > 0 && i.height > 0);
        let image = i.raster(16).unwrap_or_else(|e| panic!("{}: {e}", i.id));
        assert!(image.pixels.iter().any(|p| p.a() > 0), "{}", i.id);
        assert!(image.size[0] <= 16 && image.size[1] <= 16);
        let fitted = i.fit(Bounds::new(10., 20., 24., 24.));
        assert!(fitted.w <= 24.001 && fitted.h <= 24.001);
        assert!((fitted.w / fitted.h - i.width as f32 / i.height as f32).abs() < 0.001);
    }
}
#[test]
fn icons_follow_duplicate_rows_transaction_baselines_and_history() {
    let (mut e, id) = fixture();
    set(&mut e, id, "text", Value::Text("Same\nSame\nOther".into()));
    let original = vec![icon("camera-solid"), icon("heart-regular"), None];
    set(&mut e, id, "row_icons", Value::Icons(original.clone()));
    let baseline = e.document.clone();
    e.begin("Edit text");
    e.set_control_property(id, "text", Value::Text(String::new()))
        .unwrap();
    assert!(values(&e, id).is_empty());
    e.set_control_property(id, "text", Value::Text("Same\nOther\nSame".into()))
        .unwrap();
    e.commit();
    assert_eq!(
        values(&e, id),
        vec![original[0].clone(), None, original[1].clone()]
    );
    e.undo();
    assert_eq!(e.document, baseline);
    e.redo();
    set(&mut e, id, "text", Value::Text("Same\nOther".into()));
    assert_eq!(values(&e, id), vec![original[0].clone(), None]);
    assert_eq!(
        wireframe::document::decode(&e.document.encode().unwrap()).unwrap(),
        e.document
    );
    let before = e.document.clone();
    assert!(
        e.set_control_property(id, "row_icons", Value::Icons(vec![icon("https://bad")]))
            .is_err()
    );
    assert!(
        e.set_control_property(id, "row_icons", Value::Icons(vec![None; 10002]))
            .is_err()
    );
    assert_eq!(before, e.document);
    set(
        &mut e,
        id,
        "row_icons",
        Value::Icons(vec![icon("future-icon-solid")]),
    );
    let scene = e.rendered_scene(e.page).unwrap();
    assert!(scene.svg().contains("stroke="));
    assert!(scene.png().is_ok());
}
#[test]
fn list_icon_geometry_color_clipping_and_vector_exports_agree() {
    let (mut e, id) = fixture();
    set(
        &mut e,
        id,
        "text",
        Value::Text("A long title\n-Disabled-\n*Bravo*\nLast".into()),
    );
    set(&mut e, id, "font_size", Value::Number(24.));
    set(&mut e, id, "header", Value::Bool(true));
    let width = e.current().controls[0].bounds.w;
    let assigned = vec![
        icon("camera-solid"),
        icon("asterisk-solid"),
        None,
        icon("cloud-solid"),
    ];
    set(&mut e, id, "row_icons", Value::Icons(assigned));
    assert!((e.current().controls[0].bounds.w - width - 40.).abs() < 0.001);
    let scene = e.rendered_scene(e.page).unwrap();
    let glyph = scene
        .items
        .iter()
        .find_map(|i| match &i.drawing {
            Drawing::Glyph(g) => Some(g),
            _ => None,
        })
        .unwrap();
    assert_eq!(glyph.x, 90.);
    let painted: Vec<_> = scene
        .items
        .iter()
        .filter_map(|i| match &i.drawing {
            Drawing::Icon {
                bounds, id, color, ..
            } => Some((bounds, id, color, i.clip)),
            _ => None,
        })
        .collect();
    assert_eq!(painted.len(), 4usize - 1);
    assert_eq!(painted[0].0.w, 24.);
    assert_eq!(painted[0].0.x, 58.);
    assert_eq!(*painted[1].2, [51, 51, 51, 255]);
    assert!(
        scene
            .items
            .iter()
            .any(|i| matches!(&i.drawing,Drawing::Glyph(g) if g.color == [153,153,153,255]))
    );
    assert!(painted.iter().all(|(_, _, _, clip)| clip.is_some()));
    let svg = scene.svg();
    assert!(svg.contains("CC BY 4.0"));
    assert!(svg.contains("<path d="));
    assert!(!svg.contains("data:image/png"));
    let png = scene.png().unwrap();
    let reader = png::Decoder::new(std::io::Cursor::new(&png))
        .read_info()
        .unwrap();
    assert!(reader.info().utf8_text.iter().any(
        |text| text.keyword == "Attribution" && text.get_text().unwrap().contains("CC BY 4.0")
    ));
    let pixels = image::load_from_memory(&png).unwrap();
    assert!(pixels.width() > 0 && pixels.height() > 0);
    let mut c = e.current().controls[0].clone();
    c.bounds.w = 20.;
    c.bounds.h = 25.;
    e.begin("Clip");
    e.update_control(c);
    e.commit();
    let scene = e.rendered_scene(e.page).unwrap();
    assert!(
        scene
            .items
            .iter()
            .filter(|i| matches!(i.drawing, Drawing::Icon { .. }))
            .all(|i| i.clip.unwrap().w == 20.)
    );
    set(&mut e, id, "row_icons", Value::Icons(vec![]));
    e.begin("Auto-size");
    e.auto_size_control(id).unwrap();
    e.commit();
    assert_eq!(e.current().controls[0].bounds.w, width);
}
#[test]
fn component_icon_overrides_reset_copy_and_storage_preserve_source() {
    let (mut e, id) = fixture();
    set(
        &mut e,
        id,
        "row_icons",
        Value::Icons(vec![icon("heart-regular")]),
    );
    e.create_component("Menu".into()).unwrap();
    let source = e.document.definitions.clone();
    assert!(e.enter(id));
    let child = e.nodes()[0].id;
    set(
        &mut e,
        child,
        "row_icons",
        Value::Icons(vec![icon("camera-solid")]),
    );
    assert_eq!(e.document.definitions, source);
    e.reset_local_changes(child).unwrap();
    assert_eq!(values(&e, child)[0], icon("heart-regular"));
    set(&mut e, child, "row_icons", Value::Icons(vec![]));
    assert!(values(&e, child).is_empty());
    assert_eq!(e.document.definitions, source);
    e.leave();
    e.select(id, false);
    let clipboard = e.copy();
    let mut target = Editor::default();
    target.paste(&clipboard, 0.);
    assert!(target.error.is_none());
    assert_eq!(
        wireframe::document::decode(&target.document.encode().unwrap()).unwrap(),
        target.document
    );
}
