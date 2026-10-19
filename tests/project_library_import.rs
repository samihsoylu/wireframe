// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::{collections::BTreeMap, io::Cursor};
use wireframe::{
    document::*,
    workspace::{
        AssetImportChoice as AssetChoice, Editor, LibraryImportAction as Action,
        LibraryImportChoice as Choice, PageImportChoice as PageChoice,
    },
};

fn fixture(offset: Id, label: &str) -> Project {
    let mut p = Project {
        font: None,
        next_id: 500,
        ..Project::default()
    };
    p.pages[0].name = "Landing".into();
    p.pages[0].notes = label.into();
    let pixels = image::RgbaImage::from_pixel(1, 1, image::Rgba([offset as u8, 0, 0, 255]));
    let mut bytes = Cursor::new(vec![]);
    pixels
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    p.assets.push(Asset {
        trashed: false,
        id: offset + 20,
        name: "Logo".into(),
        media_type: "image/png".into(),
        bytes: bytes.into_inner(),
    });
    let mut child = Node::control(offset + 12, "button", Point::default(), Size::default());
    child
        .properties
        .insert("text".into(), Value::Text(label.into()));
    child
        .properties
        .insert("link".into(), Value::Link(Link::Page(2)));
    child
        .properties
        .insert("icon".into(), Value::Icon(IconSource::Asset(offset + 20)));
    p.definitions.push(Definition {
        id: offset + 11,
        name: "Card".into(),
        content: Content {
            nodes: vec![child],
            paint_order: vec![],
        },
    });
    let mut placement = Node::group(offset + 13, Point::default(), vec![]);
    placement.content = NodeContent::Instance {
        definition: offset + 11,
        overrides: vec![],
    };
    p.libraries.push(ComponentLibrary {
        id: offset + 10,
        name: "Cards".into(),
        notes: format!("{label} library notes"),
        trashed: false,
        content: Content {
            nodes: vec![placement.clone()],
            paint_order: vec![],
        },
        sources: vec![ComponentSource {
            definition: offset + 11,
            node: offset + 13,
            notes: format!("{label} source notes"),
        }],
        imports: vec![],
    });
    p.default_library = Some(offset + 10);
    placement.id = 80;
    if let NodeContent::Instance { overrides, .. } = &mut placement.content {
        overrides.push(Override {
            path: vec![offset + 12],
            patch: Patch {
                properties: BTreeMap::from([
                    ("text".into(), Value::Text(format!("{label} override"))),
                    ("link".into(), Value::Link(Link::Page(2))),
                    ("icon".into(), Value::Icon(IconSource::Asset(offset + 20))),
                ]),
                ..Patch::default()
            },
        });
    }
    p.pages[0].content.nodes = vec![placement];
    let alternate_content = p.pages[0].content.clone();
    p.pages[0].alternates.push(Alternate {
        id: 3,
        name: "Phone".into(),
        notes: label.into(),
        version: None,
        content: alternate_content,
    });
    let mut contact = p.pages[0].clone();
    contact.id = 2;
    contact.name = "Contact".into();
    contact.alternates.clear();
    p.pages.push(contact);
    p.validate().unwrap();
    p
}
fn pages(choice: PageChoice) -> BTreeMap<Id, PageChoice> {
    BTreeMap::from([(1, choice), (2, choice)])
}

#[test]
fn page_equivalence_uses_combined_asset_library_and_override_mapping() {
    let source = fixture(30, "Same");
    let mut e = Editor::default();
    let prepared = e
        .plan_project_page_import(&source, "source", &[1, 2])
        .unwrap()
        .review_libraries(&BTreeMap::new(), &BTreeMap::new())
        .unwrap()
        .prepare(&BTreeMap::new())
        .unwrap();
    e.apply_project_copy(prepared).unwrap();
    let before = e.document.clone();
    let plan = e
        .plan_project_page_import(&source, "source", &[1, 2])
        .unwrap();
    assert_eq!(
        plan.equivalent_pages(&BTreeMap::new(), &BTreeMap::new(), &BTreeMap::new())
            .unwrap(),
        vec![1, 2]
    );
    // A changed source creates a library conflict. Choosing a separate library
    // must invalidate automatic page reuse, including instance-local overrides.
    let mut changed = source.clone();
    changed.definitions[0].content.nodes[0]
        .properties
        .insert("text".into(), Value::Text("Updated source".into()));
    let plan = e
        .plan_project_page_import(&changed, "source", &[1, 2])
        .unwrap();
    assert_eq!(
        plan.equivalent_pages(&BTreeMap::new(), &BTreeMap::new(), &BTreeMap::new())
            .unwrap(),
        vec![1, 2]
    );
    assert!(
        plan.equivalent_pages(
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::from([(40, Choice::KeepBoth)])
        )
        .unwrap()
        .is_empty()
    );
    assert_eq!(e.document, before);
}
fn assets(choice: AssetChoice) -> BTreeMap<Id, AssetChoice> {
    BTreeMap::from([(50, choice)])
}
fn review(
    e: &Editor,
    source: &Project,
    page: PageChoice,
    asset: AssetChoice,
) -> wireframe::workspace::ProjectLibraryImportPlan {
    e.plan_project_page_import(source, "source", &[1, 2])
        .unwrap()
        .review_libraries(&pages(page), &assets(asset))
        .unwrap()
}
fn instance(node: &Node) -> (Id, &[Override]) {
    let NodeContent::Instance {
        definition,
        overrides,
    } = &node.content
    else {
        panic!("expected instance")
    };
    (*definition, overrides)
}
fn assert_mapped(p: &Project, definition: Id, child: Id, asset: Id) {
    for content in
        std::iter::once(&p.pages[0].content).chain(p.pages[0].alternates.iter().map(|a| &a.content))
    {
        let (d, overrides) = instance(&content.nodes[0]);
        assert_eq!(d, definition);
        assert_eq!(overrides[0].path, [child]);
        assert_eq!(
            overrides[0].patch.properties["link"],
            Value::Link(Link::Page(2))
        );
        assert_eq!(
            overrides[0].patch.properties["icon"],
            Value::Icon(IconSource::Asset(asset))
        );
    }
}

#[test]
fn reused_definition_translates_incoming_page_and_alternate_paths_and_assets() {
    let source = fixture(30, "Incoming");
    let mut e = Editor::new(fixture(0, "Local"));
    let before = e.document.clone();
    let plan = review(&e, &source, PageChoice::UseNew, AssetChoice::UseExisting);
    assert!(plan.entries().iter().any(|e| e.action == Action::Conflict));
    let prepared = plan
        .prepare(&BTreeMap::from([(40, Choice::UseExisting)]))
        .unwrap();
    assert_eq!(prepared.counts().libraries_reused, 1);
    assert_eq!(prepared.newly_orphaned_overrides(), 0);
    assert_mapped(prepared.preview(), 11, 12, 20);
    assert_eq!(prepared.preview().definitions, before.definitions);
    e.apply_project_copy(prepared).unwrap();
    let after = e.document.clone();
    assert_eq!(after.pages[0].notes, "Incoming");
    assert_eq!(after.pages[0].alternates[0].id, 3);
    assert_eq!(e.undo_name(), Some("Import project"));
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
}

#[test]
fn replacing_library_keeps_local_identity_metadata_and_existing_overrides() {
    let source = fixture(30, "Incoming");
    let e = Editor::new(fixture(0, "Local"));
    let p = review(&e, &source, PageChoice::UseExisting, AssetChoice::UseNew)
        .prepare(&BTreeMap::from([(40, Choice::UseNew)]))
        .unwrap();
    let after = p.preview();
    assert_eq!(p.counts().libraries_replaced, 1);
    assert_eq!(after.libraries.len(), 1);
    assert_eq!(after.libraries[0].id, 10);
    assert_eq!(after.libraries[0].name, "Cards");
    assert_eq!(after.libraries[0].notes, "Local library notes");
    assert_eq!(after.libraries[0].sources[0].notes, "Incoming source notes");
    assert_eq!(
        after.definitions[0].content.nodes[0].properties["text"],
        Value::Text("Incoming".into())
    );
    assert_eq!(
        after.definitions[0].content.nodes[0].properties["link"],
        Value::Link(Link::Page(2))
    );
    assert_mapped(after, 11, 12, 20);
    assert_eq!(after.assets[0].bytes, source.assets[0].bytes);
}

#[test]
fn keep_both_maps_incoming_instances_to_the_independent_copy() {
    let source = fixture(30, "Incoming");
    let e = Editor::new(fixture(0, "Local"));
    let p = review(&e, &source, PageChoice::UseNew, AssetChoice::KeepBoth)
        .prepare(&BTreeMap::from([(40, Choice::KeepBoth)]))
        .unwrap();
    let after = p.preview();
    assert_eq!(p.counts().libraries, 1);
    assert_eq!(after.libraries.len(), 2);
    let new = after.libraries.iter().find(|l| l.id != 10).unwrap();
    assert_eq!(new.name, "Cards (1)");
    let asset = after.assets.iter().find(|a| a.id != 20).unwrap().id;
    assert_mapped(after, new.sources[0].definition, 42, asset);
    assert_eq!(after.definitions[0], e.document.definitions[0]);
}

#[test]
fn orphan_on_incoming_page_cannot_capture_a_reused_child() {
    let mut source = fixture(30, "Incoming");
    for page in &mut source.pages {
        for content in std::iter::once(&mut page.content)
            .chain(page.alternates.iter_mut().map(|a| &mut a.content))
        {
            if let NodeContent::Instance { overrides, .. } = &mut content.nodes[0].content {
                overrides[0].path = vec![12];
            }
        }
    }
    let e = Editor::new(fixture(0, "Local"));
    let plan = review(&e, &source, PageChoice::UseNew, AssetChoice::UseExisting);
    assert!(plan.entries()[0].has_unmapped_overrides);
    assert!(
        plan.prepare(&BTreeMap::from([(40, Choice::UseExisting)]))
            .is_err()
    );
    let copied = plan
        .prepare(&BTreeMap::from([(40, Choice::KeepBoth)]))
        .unwrap();
    assert!(copied.newly_orphaned_overrides() > 0);
    // Discarded incoming page branches must not impose their orphan restriction.
    let discarded = review(
        &e,
        &source,
        PageChoice::UseExisting,
        AssetChoice::UseExisting,
    );
    assert!(!discarded.entries()[0].has_unmapped_overrides);
    discarded
        .prepare(&BTreeMap::from([(40, Choice::UseExisting)]))
        .unwrap();
}

#[test]
fn reuse_persists_origin_and_repeats_without_allocation_or_history() {
    let mut source = fixture(30, "Incoming");
    let mut e = Editor::new(fixture(0, "Local"));
    let p = review(
        &e,
        &source,
        PageChoice::UseExisting,
        AssetChoice::UseExisting,
    )
    .prepare(&BTreeMap::from([(40, Choice::UseExisting)]))
    .unwrap();
    e.apply_project_copy(p).unwrap();
    let before = e.document.clone();
    let history = e.undo_name().map(str::to_owned);
    source.libraries[0].name = "Renamed source".into();
    let plan = review(
        &e,
        &source,
        PageChoice::UseExisting,
        AssetChoice::UseExisting,
    );
    assert_eq!(plan.entries()[0].existing, Some(10));
    e.apply_project_copy(
        plan.prepare(&BTreeMap::from([(40, Choice::UseExisting)]))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), history.as_deref());
}

#[test]
fn cancellation_invalid_choices_and_stale_destination_are_atomic() {
    let source = fixture(30, "Incoming");
    let mut e = Editor::new(fixture(0, "Local"));
    let before = e.document.clone();
    let plan = review(&e, &source, PageChoice::UseNew, AssetChoice::UseNew);
    assert!(plan.prepare(&BTreeMap::new()).is_err());
    assert!(
        plan.prepare(&BTreeMap::from([(999, Choice::UseNew)]))
            .is_err()
    );
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), None);
    let prepared = plan
        .prepare(&BTreeMap::from([(40, Choice::UseNew)]))
        .unwrap();
    e.document.pages[0].notes = "Later local edit".into();
    let edited = e.document.clone();
    assert!(!plan.matches_destination(&e.document));
    assert!(e.apply_project_copy(prepared).is_err());
    assert_eq!(e.document, edited);
    assert_eq!(e.undo_name(), None);
}

#[test]
fn omitted_page_links_remain_missing_instead_of_targeting_equal_local_ids() {
    let source = fixture(30, "Incoming");
    let e = Editor::new(fixture(0, "Local"));
    let plan = e
        .plan_project_page_import(&source, "source", &[1])
        .unwrap()
        .review_libraries(
            &BTreeMap::from([(1, PageChoice::UseNew)]),
            &assets(AssetChoice::UseExisting),
        )
        .unwrap();
    let p = plan
        .prepare(&BTreeMap::from([(40, Choice::UseNew)]))
        .unwrap();
    let after = p.preview();
    let Value::Link(Link::Page(id)) = after.definitions[0].content.nodes[0].properties["link"]
    else {
        panic!()
    };
    assert_ne!(id, 2);
    assert!(after.removed_pages.contains(&id));
    assert_eq!(
        instance(&after.pages[0].content.nodes[0]).1[0]
            .patch
            .properties["link"],
        Value::Link(Link::Page(id))
    );
}

#[test]
fn projects_without_libraries_keep_the_same_atomic_import_path() {
    let mut source = Project {
        font: None,
        ..Project::default()
    };
    source.pages[0].name = "New page".into();
    let e = Editor::new(Project {
        font: None,
        ..Project::default()
    });
    let plan = e
        .plan_project_page_import(&source, "source", &[1])
        .unwrap()
        .review_libraries(&BTreeMap::new(), &BTreeMap::new())
        .unwrap();
    assert!(plan.entries().is_empty());
    let p = plan.prepare(&BTreeMap::new()).unwrap();
    assert_eq!(p.counts().pages, 1);
    assert_eq!(p.preview().pages.len(), 2);
}

fn with_parent_library(mut p: Project) -> Project {
    let mut instance = Node::group(72, Point::default(), vec![]);
    instance.content = NodeContent::Instance {
        definition: 41,
        overrides: vec![],
    };
    p.definitions.push(Definition {
        id: 71,
        name: "Panel".into(),
        content: Content {
            nodes: vec![instance],
            paint_order: vec![],
        },
    });
    let mut root = Node::group(73, Point::default(), vec![]);
    root.content = NodeContent::Instance {
        definition: 71,
        overrides: vec![],
    };
    p.libraries.push(ComponentLibrary {
        id: 70,
        name: "Panels".into(),
        notes: String::new(),
        trashed: false,
        content: Content {
            nodes: vec![root],
            paint_order: vec![],
        },
        sources: vec![ComponentSource {
            definition: 71,
            node: 73,
            notes: String::new(),
        }],
        imports: vec![],
    });
    p.default_library = Some(70);
    p.validate().unwrap();
    p
}

#[test]
fn unchanged_whole_project_reuses_libraries_and_assets_after_reopen() {
    let source = with_parent_library(fixture(30, "Incoming"));
    let mut e = Editor::new(Project {
        font: None,
        ..Project::default()
    });
    let plan = e
        .plan_project_page_import(&source, "source", &[1, 2])
        .unwrap()
        .review_libraries(&BTreeMap::new(), &BTreeMap::new())
        .unwrap();
    let p = plan.prepare(&BTreeMap::new()).unwrap();
    assert_eq!(p.counts().libraries, 2);
    e.apply_project_copy(p).unwrap();
    let preferred = e
        .document
        .libraries
        .iter()
        .find(|l| l.name == "Panels")
        .unwrap()
        .id;
    assert_eq!(e.document.default_library, Some(preferred));
    let mut reopened = Editor::new(decode(&e.document.encode().unwrap()).unwrap());
    let before = reopened.document.clone();
    let plan = reopened
        .plan_project_page_import(&source, "source", &[1, 2])
        .unwrap()
        .review_libraries(&pages(PageChoice::UseExisting), &BTreeMap::new())
        .unwrap();
    assert!(plan.entries().iter().all(|e| e.action == Action::Reuse));
    let p = plan.prepare(&BTreeMap::new()).unwrap();
    assert_eq!(p.counts().libraries_reused, 2);
    reopened.apply_project_copy(p).unwrap();
    assert_eq!(reopened.document, before);
    assert_eq!(reopened.undo_name(), None);
}

#[test]
fn parent_replacement_uses_the_selected_independent_dependency() {
    let mut source = with_parent_library(fixture(30, "Incoming"));
    let mut e = Editor::new(Project {
        font: None,
        ..Project::default()
    });
    let plan = e
        .plan_project_page_import(&source, "source", &[1, 2])
        .unwrap()
        .review_libraries(&BTreeMap::new(), &BTreeMap::new())
        .unwrap();
    e.apply_project_copy(plan.prepare(&BTreeMap::new()).unwrap())
        .unwrap();
    let before = e.document.clone();
    let old_child = e
        .document
        .libraries
        .iter()
        .find(|l| l.name == "Cards")
        .unwrap()
        .sources[0]
        .definition;
    let parent = e
        .document
        .libraries
        .iter()
        .find(|l| l.name == "Panels")
        .unwrap()
        .sources[0]
        .definition;
    source.definitions[0].content.nodes[0]
        .properties
        .insert("text".into(), Value::Text("Changed source".into()));
    let plan = e
        .plan_project_page_import(&source, "source", &[1, 2])
        .unwrap()
        .review_libraries(&pages(PageChoice::UseExisting), &BTreeMap::new())
        .unwrap();
    assert!(plan.entries().iter().all(|e| e.action == Action::Conflict));
    let p = plan
        .prepare(&BTreeMap::from([
            (40, Choice::KeepBoth),
            (70, Choice::UseNew),
        ]))
        .unwrap();
    let new_child = p
        .preview()
        .libraries
        .iter()
        .find(|l| l.name == "Cards (1)")
        .unwrap()
        .sources[0]
        .definition;
    assert_ne!(old_child, new_child);
    let definition = p
        .preview()
        .definitions
        .iter()
        .find(|d| d.id == parent)
        .unwrap();
    assert_eq!(instance(&definition.content.nodes[0]).0, new_child);
    e.apply_project_copy(p).unwrap();
    e.undo();
    assert_eq!(e.document, before);
}

#[test]
fn replacement_reports_existing_edits_that_become_unresolved() {
    let mut destination = fixture(0, "Local");
    let mut source = fixture(30, "Incoming");
    for (p, id, text) in [
        (&mut destination, 14, "Other local"),
        (&mut source, 44, "Other incoming"),
    ] {
        let mut node = Node::control(id, "button", Point::default(), Size::default());
        node.properties
            .insert("text".into(), Value::Text(text.into()));
        p.definitions[0].content.nodes.push(node);
    }
    let e = Editor::new(destination);
    let p = review(
        &e,
        &source,
        PageChoice::UseExisting,
        AssetChoice::UseExisting,
    )
    .prepare(&BTreeMap::from([(40, Choice::UseNew)]))
    .unwrap();
    assert!(p.newly_orphaned_overrides() > 0);
    assert_eq!(
        instance(&p.preview().pages[0].content.nodes[0]).1[0].path,
        [12]
    );
    assert!(
        !p.preview().definitions[0]
            .content
            .nodes
            .iter()
            .any(|n| n.id == 12)
    );
}

#[test]
fn nested_incoming_page_paths_follow_reused_parent_and_child_definitions() {
    let mut source = with_parent_library(fixture(30, "Incoming"));
    let mut e = Editor::new(Project {
        font: None,
        ..Project::default()
    });
    let initial = e
        .plan_project_page_import(&source, "source", &[1, 2])
        .unwrap()
        .review_libraries(&BTreeMap::new(), &BTreeMap::new())
        .unwrap();
    e.apply_project_copy(initial.prepare(&BTreeMap::new()).unwrap())
        .unwrap();
    let parent = e
        .document
        .libraries
        .iter()
        .find(|l| l.name == "Panels")
        .unwrap()
        .sources[0]
        .definition;
    let parent_node = &e
        .document
        .definitions
        .iter()
        .find(|d| d.id == parent)
        .unwrap()
        .content
        .nodes[0];
    let nested_id = parent_node.id;
    let child = instance(parent_node).0;
    let child_id = e
        .document
        .definitions
        .iter()
        .find(|d| d.id == child)
        .unwrap()
        .content
        .nodes[0]
        .id;
    source.definitions[0].content.nodes[0].id = 142;
    source.definitions[0].content.nodes[0]
        .properties
        .insert("text".into(), Value::Text("Changed".into()));
    source.definitions[1].content.nodes[0].id = 172;
    for page in &mut source.pages {
        for content in std::iter::once(&mut page.content)
            .chain(page.alternates.iter_mut().map(|a| &mut a.content))
        {
            content.nodes[0].content = NodeContent::Instance {
                definition: 71,
                overrides: vec![Override {
                    path: vec![172, 142],
                    patch: Patch {
                        properties: BTreeMap::from([(
                            "text".into(),
                            Value::Text("Nested page edit".into()),
                        )]),
                        ..Patch::default()
                    },
                }],
            };
        }
    }
    source.validate().unwrap();
    let plan = e
        .plan_project_page_import(&source, "source", &[1, 2])
        .unwrap()
        .review_libraries(&pages(PageChoice::UseNew), &BTreeMap::new())
        .unwrap();
    let choices = plan
        .entries()
        .iter()
        .filter(|entry| entry.action == Action::Conflict)
        .map(|entry| (entry.incoming, Choice::UseExisting))
        .collect();
    let p = plan.prepare(&choices).unwrap();
    for page in p
        .preview()
        .live_pages()
        .filter(|page| page.name == "Landing" || page.name == "Contact")
    {
        for content in
            std::iter::once(&page.content).chain(page.alternates.iter().map(|a| &a.content))
        {
            let (definition, overrides) = instance(&content.nodes[0]);
            assert_eq!(definition, parent);
            assert_eq!(overrides[0].path, vec![nested_id, child_id]);
        }
    }
    assert_eq!(p.newly_orphaned_overrides(), 0);
}

#[test]
fn combined_choices_validate_final_file_size_without_mutation() {
    let mut base = fixture(0, "Local");
    let mut source = fixture(30, "Incoming");
    base.assets[0].bytes = vec![0; 4 * 1024 * 1024];
    source.assets[0].bytes = vec![1; 4 * 1024 * 1024];
    base.encode().unwrap();
    source.encode().unwrap();
    let e = Editor::new(base.clone());
    let choices = BTreeMap::from([(40, Choice::UseExisting)]);
    let oversized = review(&e, &source, PageChoice::UseExisting, AssetChoice::KeepBoth);
    assert!(oversized.prepare(&choices).is_err());
    let replacement = review(&e, &source, PageChoice::UseExisting, AssetChoice::UseNew);
    let prepared = replacement.prepare(&choices).unwrap();
    assert_eq!(prepared.preview().assets.len(), 1);
    assert_eq!(prepared.preview().assets[0].id, 20);
    assert_eq!(prepared.preview().assets[0].bytes[0], 1);
    assert_eq!(e.document, base);
}
