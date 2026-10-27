// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    catalog,
    document::{self, Node, Point, Project, Value},
    model::Kind,
    rich_text::structured::Grid,
    workspace::{CatalogPreset, Editor},
};

#[test]
fn new_starters_preserve_historical_fallbacks_and_saved_history_in_both_styles() {
    for sketch in [false, true] {
        for (kind, old) in [
            (
                Kind::Accordion,
                "Overview\nPages\n- Landing\n- Settings\nAssets\nNotes",
            ),
            (
                Kind::Alert,
                "Confirm changes\nApply these changes to the project?\nCancel, Apply",
            ),
            (Kind::Browser, "Page title\nhttps://example.com"),
            (Kind::Dropdown, "Dropdown"),
            (
                Kind::Table,
                "Name, Status\nItem one, Ready\nItem two, Draft",
            ),
        ] {
            let spec = catalog::get(kind.catalog_id()).unwrap();
            let old_node = Node::control(2, spec.id, Point::default(), spec.default_size);
            let context = catalog::Context::default();
            assert_eq!(
                spec.effective(&old_node, &context).unwrap()["text"],
                Value::Text(old.into())
            );
            let mut e = Editor::default();
            e.document.settings.sketch = sketch;
            let before = e.document.clone();
            e.add(kind, 40., 50.);
            assert!(e.error.is_none(), "{kind:?}: {:?}", e.error);
            let id = *e.selection.iter().next().unwrap();
            let (_, props) = e.control_properties(id).unwrap();
            let Value::Text(text) = &props["text"] else {
                panic!("missing starter")
            };
            assert_ne!(text, old);
            if kind == Kind::Table {
                let grid = Grid::parse(text);
                assert_eq!(grid.rows.len(), 6);
                assert!(grid.rows.iter().all(|row| row.len() == 4));
            }
            let scene = e.rendered_scene(e.page).unwrap();
            assert!(scene.bounds.w > 0. && scene.bounds.h > 0.);
            if kind == Kind::Alert {
                assert_eq!((scene.bounds.w, scene.bounds.h), (200., 102.));
            }
            let after = e.document.clone();
            assert_eq!(document::decode(&after.encode().unwrap()).unwrap(), after);
            e.undo();
            assert_eq!(e.document, before);
            e.redo();
            assert_eq!(e.document, after);
            assert_eq!(e.rendered_scene(e.page).unwrap().svg(), scene.svg());
        }
        let mut e = Editor::new(Project::default());
        e.document.settings.sketch = sketch;
        e.add_preset(CatalogPreset::AndroidAlert, 0., 0.);
        let id = *e.selection.iter().next().unwrap();
        let (_, props) = e.control_properties(id).unwrap();
        assert_eq!(
            props["text"],
            Value::Text("Notice\nMessage goes here\nCancel, OK".into())
        );
        assert_eq!(props["button_style"], Value::Choice("inline".into()));
        let scene = e.rendered_scene(e.page).unwrap();
        assert_eq!((scene.bounds.w, scene.bounds.h), (200., 102.));
    }
}
