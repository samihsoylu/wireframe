// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
mod support;
use wireframe::{
    catalog,
    document::*,
    model::{Bounds, Kind},
    render::Drawing,
    scene::Primitive,
    workspace::{CatalogCategory, CatalogItem, CatalogPreset, Editor},
};

#[test]
fn quick_add_reaches_every_control_without_duplicate_legacy_choices() {
    let editor = Editor::default();
    for spec in catalog::CONTROLS.iter() {
        let kinds: Vec<_> = Kind::ALL
            .into_iter()
            .filter(|kind| catalog::get(kind.catalog_id()).is_some_and(|c| c.id == spec.id))
            .collect();
        if kinds.is_empty() {
            continue;
        }
        let results = editor.search_catalog(spec.name);
        let controls: Vec<_> = results
            .iter()
            .filter_map(|entry| match entry.item {
                CatalogItem::Control(kind) if kinds.contains(&kind) => Some(kind),
                _ => None,
            })
            .collect();
        assert_eq!(controls.len(), 1, "{}: {controls:?}", spec.name);
        if let Some(canonical) = kinds.into_iter().find(|kind| kind.catalog_id() == spec.id) {
            assert_eq!(controls[0], canonical);
        }
    }
    for query in ["horizontal rule", "horizontal", "divider", "line"] {
        assert!(
            editor
                .search_catalog(query)
                .iter()
                .any(|entry| entry.item == CatalogItem::Control(Kind::Line)),
            "{query}"
        );
    }
}

#[test]
fn category_presets_are_distinct_with_complete_membership_and_stable_unions() {
    let e = Editor::new(Project::default());
    assert_eq!(e.catalog_entries().len(), 94);
    for (category, count) in [
        (CatalogCategory::Android, 12),
        (CatalogCategory::Ios, 9),
        (CatalogCategory::Big, 24),
        (CatalogCategory::Buttons, 16),
        (CatalogCategory::Common, 23),
        (CatalogCategory::Containers, 10),
        (CatalogCategory::Forms, 27),
        (CatalogCategory::Layout, 13),
        (CatalogCategory::Markup, 7),
        (CatalogCategory::Media, 12),
        (CatalogCategory::Text, 22),
    ] {
        assert_eq!(
            e.catalog_categories(&[category]).len(),
            count,
            "{category:?}"
        );
    }
    let union = e.catalog_categories(&[
        CatalogCategory::Android,
        CatalogCategory::Forms,
        CatalogCategory::Android,
    ]);
    for item in [
        CatalogItem::Control(Kind::HorizontalSlider),
        CatalogItem::Preset(CatalogPreset::AndroidHorizontalSlider),
        CatalogItem::Control(Kind::CircleButton),
    ] {
        assert_eq!(
            union.iter().filter(|e| e.item == item).count(),
            1,
            "{item:?}"
        );
    }
    for preset in CatalogPreset::ALL {
        assert!(
            e.search_catalog(preset.name())
                .iter()
                .any(|e| e.item == CatalogItem::Preset(preset))
        );
    }
    assert!(
        CatalogCategory::visible(&[CatalogCategory::Navigation, CatalogCategory::Charts])
            .ends_with(&[CatalogCategory::Navigation, CatalogCategory::Charts])
    );
}

#[test]
fn every_preset_inserts_ordinary_properties_as_one_undoable_saved_control() {
    for preset in CatalogPreset::ALL {
        let mut e = Editor::new(Project::default());
        let before = e.document.clone();
        e.add_preset(preset, 50., 70.);
        assert!(e.error.is_none(), "{preset:?}: {:?}", e.error);
        let id = *e.selection.iter().next().unwrap();
        let node = &e.nodes()[0];
        assert!(
            matches!(&node.content,NodeContent::Control{kind,..}if kind==preset.kind().catalog_id())
        );
        let (spec, props) = e.control_properties(id).unwrap();
        for (key, value) in &node.properties {
            if let Some(field) = spec.property(key) {
                field.validate(value).unwrap();
            }
        }
        match preset {
            CatalogPreset::AndroidAlert => {
                assert_eq!(props["button_style"], Value::Choice("inline".into()));
                assert_eq!(props["align"], Value::Choice("left".into()));
            }
            CatalogPreset::AndroidHorizontalSlider | CatalogPreset::AndroidVerticalSlider => {
                assert_eq!(props["slider_thumb"], Value::Choice("circle".into()))
            }
            CatalogPreset::IconList => {
                assert_eq!(props["border"], Value::Choice("none".into()));
                assert!(matches!(&props["row_icons"],Value::Icons(v)if v.len()==3));
            }
            CatalogPreset::AndroidProgress => {
                assert_eq!(props["progress_style"], Value::Choice("thin".into()))
            }
            CatalogPreset::AndroidSearch => {
                assert_eq!(props["shape"], Value::Choice("rectangle".into()));
                assert_eq!(props["microphone_icon"], Value::Bool(true));
            }
            CatalogPreset::AndroidTextInput => {
                assert_eq!(props["border"], Value::Choice("line".into()))
            }
            CatalogPreset::IosPointedButton => {
                assert_eq!(props["border"], Value::Choice("solid".into()))
            }
        }
        let after = e.document.clone();
        let saved = after.encode().unwrap();
        let reopened = wireframe::document::decode(&saved).unwrap();
        assert_eq!(reopened, after);
        e.undo();
        assert_eq!(e.document, before);
        e.redo();
        assert_eq!(e.document, after);
    }
}

fn rectangles(e: &Editor) -> Vec<(Bounds, [u8; 3])> {
    e.rendered_scene(e.page)
        .unwrap()
        .items
        .into_iter()
        .filter_map(|i| match i.drawing {
            Drawing::Primitive(Primitive::Rect { b, fill, .. }) => Some((b, fill)),
            _ => None,
        })
        .collect()
}
#[test]
fn progress_default_sizes_value_geometry_and_saved_dimensions_are_respected() {
    for thin in [false, true] {
        let mut e = Editor::new(Project::default());
        if thin {
            e.add_preset(CatalogPreset::AndroidProgress, 0., 0.);
        } else {
            e.add(Kind::Progress, 0., 0.);
        }
        let id = *e.selection.iter().next().unwrap();
        let b = e.selected_views()[0].bounds;
        assert_eq!((b.w, b.h), (100., if thin { 10. } else { 20. }));
        assert!(!Kind::Progress.text_editable());
        for (value, width) in [
            (0., 0.),
            (0.03, if thin { 3. } else { 0. }),
            (0.5, if thin { 50. } else { 44. }),
            (1., if thin { 100. } else { 94. }),
        ] {
            e.set_control_property(id, "value", Value::Number(value))
                .unwrap();
            let rects = rectangles(&e);
            let fills: Vec<_> = rects.iter().filter(|(_, c)| *c == [204; 3]).collect();
            assert_eq!(
                fills.len(),
                usize::from(width > 0.),
                "thin={thin},value={value}"
            );
            if width > 0. {
                assert!((fills[0].0.w - width).abs() < 0.001);
                assert_eq!(fills[0].0.y, 3.);
            }
            if thin {
                assert_eq!(rects[0], (Bounds::new(0., 3., 100., 4.), [232; 3]));
            }
        }
        support::stored_size(&mut e, 240., 24.);
        let p = wireframe::document::decode(&e.document.encode().unwrap()).unwrap();
        let mut reopened = Editor::new(p);
        reopened.select(id, false);
        assert_eq!(
            reopened.selected_views()[0].bounds,
            Bounds::new(0., 0., 240., 24.)
        );
        support::stored_size(&mut e, 2., 2.);
        assert!(rectangles(&e).iter().all(|(b, _)| b.w >= 0. && b.h >= 0.));
    }
}

#[test]
fn thin_indeterminate_uses_center_half_and_preserves_the_value() {
    let mut e = Editor::new(Project::default());
    e.add_preset(CatalogPreset::AndroidProgress, 0., 0.);
    let id = *e.selection.iter().next().unwrap();
    e.set_control_property(id, "value", Value::Number(0.9))
        .unwrap();
    e.set_control_property(id, "indeterminate", Value::Bool(true))
        .unwrap();
    assert!(rectangles(&e).contains(&(Bounds::new(25., 3., 50., 4.), [204; 3])));
    e.set_control_property(id, "indeterminate", Value::Bool(false))
        .unwrap();
    assert!(
        rectangles(&e)
            .iter()
            .any(|(b, c)| *c == [204; 3] && (b.w - 90.).abs() < 0.001)
    );
}
