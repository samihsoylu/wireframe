// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    workspace::{Editor, ImageImportTarget, ImportChoice},
};
fn icon(name: &str) -> Option<IconSource> {
    Some(IconSource::Catalog(name.into()))
}
fn fixture() -> Editor {
    let mut p = Project {
        next_id: 4,
        ..Project::default()
    };
    p.pages[0].content.nodes = (0..2)
        .map(|i| {
            let mut n = Node::control(
                i + 2,
                "app_bar",
                Point::new(50., 60. + i as f32 * 160.),
                Size::fixed(500., 48.),
            );
            let names = if i == 0 {
                vec!["asterisk-solid", "heart-solid", "star-solid"]
            } else {
                vec!["circle-solid"]
            };
            n.properties.insert(
                "icons".into(),
                Value::Icons(names.iter().map(|s| icon(s)).collect()),
            );
            let destinations = if i == 0 {
                vec![1, 2, 3, 4, 5, 6]
            } else {
                vec![1, 2, 7, 6]
            };
            n.properties.insert(
                "row_links".into(),
                Value::Links(
                    destinations
                        .into_iter()
                        .map(|id| Some(Link::Url(format!("https://example.org/{id}"))))
                        .collect(),
                ),
            );
            for key in ["nav_icon", "filter_icon", "menu_icon"] {
                n.properties.insert(key.into(), Value::Bool(i == 0));
            }
            n
        })
        .collect();
    let mut e = Editor::new(p);
    e.select_all();
    e
}
fn values(e: &Editor, id: Id) -> Properties {
    e.control_properties(id).unwrap().1
}
fn urls(ids: &[Option<u32>]) -> Value {
    Value::Links(
        ids.iter()
            .map(|id| id.map(|id| Link::Url(format!("https://example.org/{id}"))))
            .collect(),
    )
}
#[test]
fn compact_unequal_lists_preserve_destinations_and_exact_history() {
    let mut e = fixture();
    let before = e.document.clone();
    let slots = e.app_bar_icon_slots(&[2, 3]).unwrap();
    assert_eq!(slots.len(), 3);
    assert!(slots[2].label.ends_with(" / No Icon"));
    assert_eq!(
        e.app_bar_icon_slots(&[3, 2]).unwrap()[0].icon,
        icon("circle-solid")
    );
    e.begin("Change app bar icon");
    e.set_app_bar_icon(&[2, 3], 2, icon("bag-shopping-solid"))
        .unwrap();
    e.commit();
    assert_eq!(
        values(&e, 2)["icons"],
        Value::Icons(vec![
            icon("asterisk-solid"),
            icon("heart-solid"),
            icon("bag-shopping-solid")
        ])
    );
    assert_eq!(
        values(&e, 3)["icons"],
        Value::Icons(vec![icon("circle-solid"), icon("bag-shopping-solid")])
    );
    assert_eq!(
        values(&e, 2)["row_links"],
        urls(&[Some(1), Some(2), Some(3), Some(4), None, Some(6)])
    );
    assert_eq!(
        values(&e, 3)["row_links"],
        urls(&[Some(1), Some(2), Some(7), None, Some(6)])
    );
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
    e.undo();
    e.begin("Remove icon");
    e.set_app_bar_icon(&[2, 3], 2, None).unwrap();
    e.commit();
    assert_eq!(
        e.document.pages[0].content.nodes[1],
        before.pages[0].content.nodes[1]
    );
    assert_eq!(
        values(&e, 2)["row_links"],
        urls(&[Some(1), Some(2), Some(3), Some(4), Some(6)])
    );
}
#[test]
fn full_collection_changes_preserve_builtin_destinations_and_do_not_reuse_removed_links() {
    let mut e = fixture();
    let before = e.document.clone();
    let mut icons = e
        .app_bar_icon_slots(&[2, 3])
        .unwrap()
        .into_iter()
        .map(|s| s.icon)
        .collect::<Vec<_>>();
    icons.push(icon("plus-solid"));
    e.begin("Shared icons");
    e.replace_app_bar_icons(&[2, 3], icons.clone()).unwrap();
    e.commit();
    for id in [2, 3] {
        assert_eq!(values(&e, id)["icons"], Value::Icons(icons.clone()));
    }
    assert_eq!(
        values(&e, 2)["row_links"],
        urls(&[Some(1), Some(2), Some(3), Some(4), Some(5), None, Some(6)])
    );
    assert_eq!(
        values(&e, 3)["row_links"],
        urls(&[Some(1), Some(2), None, None, None, None, Some(6)])
    );
    e.undo();
    assert_eq!(e.document, before);
}
#[test]
fn invalid_batches_are_atomic_and_missing_removal_is_noop() {
    let mut e = fixture();
    let before = e.document.clone();
    for (ids, index, value) in [
        (vec![2, 999], 0, icon("star-solid")),
        (vec![2, 3], 99, icon("star-solid")),
        (vec![2, 3], 0, icon("missing-icon")),
        (vec![2, 3], 0, Some(IconSource::Asset(999))),
    ] {
        assert!(e.set_app_bar_icon(&ids, index, value).is_err());
        assert_eq!(e.document, before);
    }
    assert!(
        e.replace_app_bar_icons(&[2, 999], vec![icon("star-solid")])
            .is_err()
    );
    assert_eq!(e.document, before);
    e.begin("Nothing");
    e.set_app_bar_icon(&[2, 3], 3, None).unwrap();
    e.commit();
    assert_eq!(e.document, before);
    assert!(e.undo_name().is_none());
    e.begin("Add");
    e.set_app_bar_icon(&[2, 3], 3, icon("plus-solid")).unwrap();
    e.commit();
    assert_eq!(
        values(&e, 2)["icons"],
        Value::Icons(vec![
            icon("asterisk-solid"),
            icon("heart-solid"),
            icon("star-solid"),
            icon("plus-solid")
        ])
    );
    assert_eq!(
        values(&e, 3)["icons"],
        Value::Icons(vec![icon("circle-solid"), icon("plus-solid")])
    );
}
#[test]
fn shared_toggles_keep_links_and_components_keep_sources() {
    let mut e = fixture();
    for key in ["nav_icon", "filter_icon", "menu_icon"] {
        let f = e
            .selection_properties()
            .into_iter()
            .find(|f| f.key == key)
            .unwrap();
        assert!(f.mixed);
        assert_eq!(f.value, Value::Bool(true));
        let before = e.document.clone();
        e.begin("Toggle");
        e.set_selection_property(key, Value::Bool(false)).unwrap();
        e.commit();
        for id in [2, 3] {
            assert_eq!(values(&e, id)[key], Value::Bool(false));
        }
        e.undo();
        assert_eq!(e.document, before);
    }
    e.group();
    let def = e.create_component("Bars".into()).unwrap();
    let root = *e.selection.iter().next().unwrap();
    e.enter(root);
    e.select_all();
    let before = e.document.clone();
    let ids = e.ordered_selection();
    e.begin("Local icons");
    e.set_app_bar_icon(&ids, 2, icon("plus-solid")).unwrap();
    e.commit();
    assert_eq!(e.document.definitions, before.definitions);
    assert_eq!(
        e.document.component_source(def),
        before.component_source(def)
    );
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
}
#[test]
fn imported_icon_is_one_asset_and_one_atomic_edit_for_both_bars() {
    let mut e = fixture();
    let before = e.document.clone();
    let mut bytes = std::io::Cursor::new(vec![]);
    image::RgbaImage::from_pixel(20, 10, image::Rgba([0, 0, 0, 255]))
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let image = wireframe::images::Prepared::from_bytes("Shared.png", bytes.get_ref()).unwrap();
    let asset = e
        .import_image_at(
            ImageImportTarget::AppBarIcons {
                controls: vec![2, 3],
                index: 2,
            },
            image,
            ImportChoice::UseExisting,
        )
        .unwrap();
    assert!(e.document.live_asset(asset).is_some());
    for (id, index) in [(2, 2), (3, 1)] {
        let Value::Icons(icons) = values(&e, id)["icons"].clone() else {
            panic!()
        };
        assert_eq!(icons[index], Some(IconSource::Asset(asset)));
    }
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
}
