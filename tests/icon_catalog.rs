// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::*,
    icons,
    model::Kind,
    workspace::{CatalogCategory as Category, CatalogItem as Item, Editor},
};

#[test]
fn individual_icons_have_distinct_styles_and_explicit_category_membership() {
    let editor = Editor::new(Project::default());
    let entries = editor.catalog_categories(&[Category::Icons]);
    assert_eq!(entries.len(), icons::all().len());
    for (id, name) in [
        ("star-regular", "Star Regular"),
        ("star-solid", "Star Solid"),
    ] {
        let entry = entries.iter().find(|e| e.item == Item::Icon(id)).unwrap();
        assert_eq!(entry.name, name);
        assert!(entry.enabled);
    }
    for categories in [vec![], vec![Category::All], vec![Category::Common]] {
        let ordinary = editor.catalog_categories(&categories);
        assert!(ordinary.iter().all(|e| !matches!(e.item, Item::Icon(_))));
        assert!(ordinary.iter().any(|e| e.item == Item::Control(Kind::Icon)));
    }
    let union = editor.catalog_categories(&[Category::Icons, Category::Icons, Category::Common]);
    assert_eq!(
        union
            .iter()
            .filter(|e| matches!(e.item, Item::Icon(_)))
            .count(),
        entries.len()
    );
    assert!(
        union
            .windows(2)
            .all(|w| w[0].name.to_lowercase() <= w[1].name.to_lowercase())
    );
}
#[test]
fn quick_add_matches_style_identity_and_keywords_with_existing_controls_retained() {
    let editor = Editor::new(Project::default());
    assert_eq!(
        editor.search_catalog("Star Regular")[0].item,
        Item::Icon("star-regular")
    );
    assert_eq!(
        editor.search_catalog("star-solid")[0].item,
        Item::Icon("star-solid")
    );
    assert!(
        editor
            .search_catalog("favorite")
            .iter()
            .any(|e| e.item == Item::Icon("star-regular"))
    );
    assert!(
        editor
            .search_catalog("button")
            .iter()
            .any(|e| e.item == Item::Control(Kind::Button))
    );
    assert!(editor.catalog_entry(Item::Icon("missing-icon")).is_none());
}
#[test]
fn icon_insertion_is_a_fresh_ordinary_control_and_roundtrips_with_one_step_history() {
    for icon in ["star-regular", "star-solid", "0-solid"] {
        let mut editor = Editor::new(Project::default());
        let before = editor.document.clone();
        let id = editor.insert_catalog_icon(icon, 50., 100.).unwrap();
        let node = &editor.nodes()[0];
        assert!(matches!(&node.content, NodeContent::Control{kind, ..} if kind == "icon"));
        assert_eq!(
            node.properties.get("icon"),
            Some(&Value::Icon(IconSource::Catalog(icon.into())))
        );
        assert_eq!(
            node.properties.get("icon_size"),
            Some(&Value::Choice("24".into()))
        );
        assert_eq!(node.origin, Point::new(50., 100.));
        let control = editor
            .current()
            .controls
            .into_iter()
            .find(|c| c.id == id)
            .unwrap();
        assert_eq!((control.bounds.w, control.bounds.h), (24., 24.));
        let saved = editor.document.clone();
        assert_eq!(decode(&saved.encode().unwrap()).unwrap(), saved);
        assert_eq!(editor.undo_name(), Some("Add icon"));
        editor.undo();
        assert_eq!(editor.document, before);
        editor.redo();
        assert_eq!(editor.document, saved);
    }
}
#[test]
fn invalid_or_exhausted_insertion_preserves_pending_edits_and_history() {
    for mode in 0..3 {
        let mut p = Project::default();
        if mode == 2 {
            p.next_id = Id::MAX;
        }
        let mut editor = Editor::new(p);
        editor.begin("Pending title");
        editor.document.title = "Still editing".into();
        let before = editor.document.clone();
        assert!(
            editor
                .insert_catalog_icon(
                    if mode == 0 { "missing" } else { "star-solid" },
                    if mode == 1 { f32::NAN } else { 0. },
                    0.
                )
                .is_err()
        );
        assert_eq!(editor.document, before);
        assert_eq!(editor.undo_name(), None);
        editor.cancel();
        assert_eq!(editor.document.title, "Untitled project");
    }
}
#[test]
fn nested_icon_insertion_uses_local_coordinates_and_locked_context_disables_search() {
    let mut p = Project::default();
    let id = p.allocate_id().unwrap();
    p.pages[0]
        .content
        .nodes
        .push(Node::group(id, Point::new(40., 70.), vec![]));
    let mut editor = Editor::new(p);
    editor.context = vec![id];
    editor
        .insert_catalog_icon("star-regular", 90., 110.)
        .unwrap();
    assert_eq!(editor.nodes()[0].origin, Point::new(50., 40.));
    editor.document.pages[0].content.nodes[0].locked = true;
    let before = editor.document.clone();
    assert!(
        !editor
            .catalog_entry(Item::Icon("star-regular"))
            .unwrap()
            .enabled
    );
    assert!(editor.search_catalog("star regular").is_empty());
    assert!(
        editor
            .insert_catalog_icon("star-regular", 90., 110.)
            .is_err()
    );
    assert_eq!(editor.document, before);
}
