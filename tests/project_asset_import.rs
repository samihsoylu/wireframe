// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::{collections::BTreeMap, io::Cursor};
use wireframe::{
    document::*,
    render::Drawing,
    storage::ProjectStore,
    working_copy::WorkingCopies,
    workspace::{
        AssetImportAction as Action, AssetImportChoice as Choice, Editor, PageImportChoice,
    },
};
fn image(id: Id, asset: Id) -> Node {
    let mut n = Node::control(id, "image", Point::default(), Size::fixed(20., 20.));
    n.properties.insert("image".into(), Value::Asset(asset));
    n
}
fn asset(id: Id, name: &str, color: u8) -> Asset {
    let pixels = image::RgbaImage::from_pixel(2, 2, image::Rgba([color, 0, 0, 255]));
    let mut bytes = Cursor::new(vec![]);
    pixels
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    Asset {
        trashed: false,
        id,
        name: name.into(),
        media_type: "image/png".into(),
        bytes: bytes.into_inner(),
    }
}
fn base() -> Project {
    let mut p = Project {
        font: None,
        next_id: 200,
        ..Project::default()
    };
    p.pages[0].name = "Landing".into();
    p.pages[0].content.nodes = vec![image(80, 10)];
    p.assets = vec![
        asset(10, "Hero", 10),
        asset(11, "Portrait", 20),
        asset(12, "Badge", 30),
    ];
    p
}
fn source() -> Project {
    let mut p = base();
    p.assets = vec![
        asset(50, "Hero", 110),
        asset(51, "Portrait", 120),
        asset(52, "Badge", 130),
    ];
    p.pages[0].content.nodes = vec![image(80, 50)];
    let mut contact = p.pages[0].clone();
    contact.id = 2;
    contact.name = "Contact".into();
    let mut instance = Node::group(81, Point::default(), vec![]);
    instance.content = NodeContent::Instance {
        definition: 41,
        overrides: vec![Override {
            path: vec![43],
            patch: Patch {
                properties: BTreeMap::from([("image".into(), Value::Asset(51))]),
                ..Patch::default()
            },
        }],
    };
    contact.content.nodes = vec![instance];
    contact.alternates.push(Alternate {
        id: 3,
        version: None,
        name: "Phone".into(),
        notes: String::new(),
        content: Content {
            nodes: vec![image(82, 52)],
            paint_order: vec![],
        },
    });
    p.pages.push(contact);
    p.definitions = vec![Definition {
        id: 41,
        name: "Photo".into(),
        content: Content {
            nodes: vec![image(43, 50)],
            paint_order: vec![],
        },
    }];
    let mut placement = Node::group(42, Point::default(), vec![]);
    placement.content = NodeContent::Instance {
        definition: 41,
        overrides: vec![],
    };
    let mut button = Node::control(44, "button", Point::default(), Size::default());
    button
        .properties
        .insert("icon".into(), Value::Icon(IconSource::Asset(52)));
    button.properties.insert(
        "icons".into(),
        Value::Icons(vec![
            Some(IconSource::Asset(51)),
            None,
            Some(IconSource::Asset(52)),
        ]),
    );
    p.libraries = vec![ComponentLibrary {
        id: 40,
        trashed: false,
        name: "Photos".into(),
        notes: String::new(),
        content: Content {
            nodes: vec![placement, button],
            paint_order: vec![],
        },
        sources: vec![ComponentSource {
            definition: 41,
            node: 42,
            notes: String::new(),
        }],
        imports: vec![],
    }];
    p.default_library = Some(40);
    p.validate().unwrap();
    p
}
fn pixel(e: &Editor) -> [u8; 4] {
    e.rendered_scene(e.page)
        .unwrap()
        .items
        .iter()
        .find_map(|item| match &item.drawing {
            Drawing::Image { data, .. } => Some(data.pixels.pixels[0].to_array()),
            _ => None,
        })
        .unwrap()
}
#[test]
fn mixed_asset_and_page_choices_share_one_mapping_and_exact_history() {
    let source = source();
    let source_before = source.clone();
    let mut e = Editor::new(base());
    let before = e.document.clone();
    assert_eq!(pixel(&e), [10, 0, 0, 255]);
    let plan = e
        .plan_project_page_import(&source, "source", &[1, 2])
        .unwrap();
    assert!(
        plan.asset_entries()
            .iter()
            .all(|a| a.action == Action::Conflict)
    );
    let prepared = plan
        .prepare_with_assets(
            &BTreeMap::from([(1, PageImportChoice::UseExisting)]),
            &BTreeMap::from([
                (50, Choice::UseNew),
                (51, Choice::UseExisting),
                (52, Choice::KeepBoth),
            ]),
        )
        .unwrap();
    assert_eq!(prepared.counts().assets_added, 1);
    assert_eq!(prepared.counts().assets_replaced, 1);
    assert_eq!(prepared.counts().assets_reused, 1);
    assert_eq!(e.document, before);
    assert_eq!(source, source_before);
    let new_badge = prepared
        .preview()
        .assets
        .iter()
        .find(|a| a.name == "Badge (2)")
        .unwrap()
        .id;
    let contact_id = prepared.imported_pages()[1];
    e.apply_project_copy(prepared).unwrap();
    assert_eq!(e.document.pages[0], before.pages[0]);
    assert_eq!(pixel(&e), [110, 0, 0, 255]);
    assert_eq!(e.document.assets[0].id, 10);
    assert_eq!(e.document.assets[0].name, "Hero");
    assert_eq!(e.document.assets[1], before.assets[1]);
    assert_eq!(e.document.assets[2], before.assets[2]);
    let contact = e.document.live_page(contact_id).unwrap();
    let NodeContent::Instance { overrides, .. } = &contact.content.nodes[0].content else {
        panic!()
    };
    assert_eq!(overrides[0].patch.properties["image"], Value::Asset(11));
    assert_eq!(
        contact.alternates[0].content.nodes[0].properties["image"],
        Value::Asset(new_badge)
    );
    assert_eq!(
        e.document.definitions[0].content.nodes[0].properties["image"],
        Value::Asset(10)
    );
    let button = &e.document.libraries[0].content.nodes[1];
    assert_eq!(
        button.properties["icon"],
        Value::Icon(IconSource::Asset(new_badge))
    );
    assert_eq!(
        button.properties["icons"],
        Value::Icons(vec![
            Some(IconSource::Asset(11)),
            None,
            Some(IconSource::Asset(new_badge))
        ])
    );
    let scene = e.rendered_scene(contact_id).unwrap();
    assert!(scene.items.iter().any(|i|matches!(&i.drawing,Drawing::Image{data,..} if data.pixels.pixels[0].to_array()==[20,0,0,255])));
    let after = e.document.clone();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("import.wireframe");
    ProjectStore.save(&path, &after).unwrap();
    assert_eq!(ProjectStore.load(&path).unwrap(), after);
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(pixel(&e), [10, 0, 0, 255]);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(pixel(&e), [110, 0, 0, 255]);
}
#[test]
fn asset_only_reuse_survives_recovery_and_repeated_import_is_a_noop() {
    let mut source = Project {
        font: None,
        next_id: 51,
        ..Project::default()
    };
    source.assets = vec![asset(50, "Hero", 10)];
    let mut e = Editor::new(base());
    let before = e.document.clone();
    let plan = e.plan_project_page_import(&source, "source", &[]).unwrap();
    assert_eq!(plan.asset_entries()[0].action, Action::Reuse);
    e.apply_project_copy(
        plan.prepare_with_assets(&BTreeMap::new(), &BTreeMap::new())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(e.document.assets, before.assets);
    let after = e.document.clone();
    assert_eq!(after.asset_imports[&10][0].asset, 50);
    let dir = tempfile::tempdir().unwrap();
    let copies = WorkingCopies::new(dir.path()).unwrap();
    let copy = copies.create_unnamed(&after).unwrap();
    let id = copy.id().to_owned();
    drop(copy);
    assert_eq!(copies.resume(&id).unwrap().project(), &after);
    source.assets[0].name = "Renamed upstream".into();
    let plan = e.plan_project_page_import(&source, "source", &[]).unwrap();
    assert_eq!(plan.asset_entries()[0].existing, Some(10));
    e.apply_project_copy(
        plan.prepare_with_assets(&BTreeMap::new(), &BTreeMap::new())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(e.document, after);
    e.undo();
    assert_eq!(e.document, before);
}
#[test]
fn strong_asset_origins_are_reserved_before_name_matches() {
    let mut p = base();
    p.asset_imports.insert(
        10,
        vec![AssetImport {
            project: "source".into(),
            asset: 51,
        }],
    );
    let mut source = Project {
        font: None,
        next_id: 53,
        ..Project::default()
    };
    source.assets = vec![
        asset(50, "Hero", 110),
        asset(51, "Renamed", 120),
        asset(52, "Portrait", 130),
    ];
    let e = Editor::new(p);
    let plan = e.plan_project_page_import(&source, "source", &[]).unwrap();
    assert_eq!(plan.asset_entries()[0].existing, None);
    assert_eq!(plan.asset_entries()[1].existing, Some(10));
    assert_eq!(plan.asset_entries()[2].existing, Some(11));
}
#[test]
fn reviewed_assets_keep_distinct_names_while_legacy_copy_deduplicates_bytes() {
    let mut source = Project {
        font: None,
        next_id: 52,
        ..Project::default()
    };
    source.assets = vec![asset(50, "Other", 10), asset(51, "Other", 10)];
    let e = Editor::new(base());
    let plan = e.plan_project_page_import(&source, "source", &[]).unwrap();
    let prepared = plan
        .prepare_with_assets(&BTreeMap::new(), &BTreeMap::new())
        .unwrap();
    assert_eq!(prepared.counts().assets_added, 2);
    assert_eq!(prepared.preview().assets[3].name, "Other");
    assert_eq!(prepared.preview().assets[4].name, "Other (2)");
    let legacy = e.prepare_project_copy(&source, "source", &[]).unwrap();
    assert_eq!(legacy.counts().assets_reused, 2);
    assert_eq!(legacy.preview().assets.len(), 3);
    assert_eq!(legacy.preview().asset_imports[&10].len(), 2);
}
#[test]
fn missing_unknown_and_stale_decisions_never_apply_partial_imports() {
    let source = source();
    let mut e = Editor::new(base());
    let before = e.document.clone();
    let plan = e.plan_project_page_import(&source, "source", &[]).unwrap();
    assert!(
        plan.prepare_with_assets(&BTreeMap::new(), &BTreeMap::new())
            .is_err()
    );
    assert!(
        plan.prepare_with_assets(&BTreeMap::new(), &BTreeMap::from([(999, Choice::UseNew)]))
            .is_err()
    );
    assert_eq!(e.document, before);
    let prepared = plan
        .prepare_with_assets(
            &BTreeMap::new(),
            &[
                (50, Choice::UseNew),
                (51, Choice::UseNew),
                (52, Choice::UseNew),
            ]
            .into(),
        )
        .unwrap();
    e.rename_page(e.page, "Edited").unwrap();
    let edited = e.document.clone();
    assert!(e.apply_project_copy(prepared).is_err());
    assert_eq!(e.document, edited);
    e.undo();
    assert_eq!(e.document, before);
}
#[test]
fn metadata_is_optional_bounded_and_validated_without_trusting_foreign_ids() {
    let base = base();
    assert!(
        decode(&base.encode().unwrap())
            .unwrap()
            .asset_imports
            .is_empty()
    );
    for case in 0..6 {
        let mut p = base.clone();
        let mut records = vec![AssetImport {
            project: "source".into(),
            asset: 50,
        }];
        let mut owner = 10;
        match case {
            0 => owner = 999,
            1 => records[0].project.clear(),
            2 => records[0].asset = 0,
            3 => records[0].project = "bad\norigin".into(),
            4 => records.push(records[0].clone()),
            _ => {
                records = (1..=50_001)
                    .map(|id| AssetImport {
                        project: "source".into(),
                        asset: id,
                    })
                    .collect()
            }
        };
        p.asset_imports.insert(owner, records);
        assert!(p.validate().is_err(), "case {case}");
    }
    let mut p = base;
    p.asset_imports.insert(
        10,
        vec![AssetImport {
            project: "source".into(),
            asset: u64::MAX,
        }],
    );
    p.validate().unwrap(); // Foreign IDs do not reserve local allocator space.
}
#[test]
fn replacement_checks_final_file_limits_instead_of_temporary_duplicates() {
    let mut p = Project {
        font: None,
        next_id: 11,
        ..Project::default()
    };
    p.assets = vec![Asset {
        trashed: false,
        id: 10,
        name: "Large".into(),
        media_type: "image/png".into(),
        bytes: vec![0; 4 * 1024 * 1024],
    }];
    let mut source = p.clone();
    source.assets[0].id = 50;
    source.assets[0].bytes.fill(1);
    source.next_id = 51;
    p.encode().unwrap();
    source.encode().unwrap();
    let e = Editor::new(p.clone());
    let plan = e.plan_project_page_import(&source, "source", &[]).unwrap();
    assert!(
        plan.prepare_with_assets(&BTreeMap::new(), &[(50, Choice::KeepBoth)].into())
            .is_err()
    );
    let replaced = plan
        .prepare_with_assets(&BTreeMap::new(), &[(50, Choice::UseNew)].into())
        .unwrap();
    assert_eq!(replaced.preview().assets.len(), 1);
    assert_eq!(replaced.preview().assets[0].id, 10);
    assert_eq!(replaced.preview().assets[0].bytes[0], 1);
    assert_eq!(e.document, p);
}
