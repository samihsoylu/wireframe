// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, workspace::Editor};
fn node(id: Id, kind: &str, text: &str) -> Node {
    let mut node = Node::control(id, kind, Point::default(), Size::default());
    node.properties
        .insert("text".into(), Value::Text(text.into()));
    node
}
fn fixture() -> Project {
    let mut p = Project {
        next_id: 100,
        font: None,
        ..Project::default()
    };
    p.pages[0].name = "Landing".into();
    p.pages[0].notes = "Literal [note](page:2)".into();
    p.pages[0].content.nodes = vec![
        node(
            10,
            "paragraph",
            r"Before [*Contact*](page:2) and \[literal](page:2)",
        ),
        node(11, "table", r"[One](page:1),[\,Two](page:2), [x]"),
        Node::group(
            12,
            Point::default(),
            vec![node(13, "paragraph", "[Missing](page:90)")],
        ),
    ];
    let mut contact = p.pages[0].clone();
    contact.id = 2;
    contact.name = "Contact".into();
    contact.content.nodes.clear();
    p.pages.push(contact);
    p.removed_pages.insert(90);
    p
}
fn text(node: &Node) -> &str {
    match &node.properties["text"] {
        Value::Text(s) => s,
        _ => panic!(),
    }
}
#[test]
fn project_import_remaps_inline_page_targets_and_preserves_literal_bytes_and_history() {
    let source = fixture();
    let mut e = Editor::new(Project {
        font: None,
        ..Project::default()
    });
    let before = e.document.clone();
    let prepared = e.prepare_project_copy(&source, "source", &[1, 2]).unwrap();
    let ids = prepared.imported_pages().to_vec();
    let after = prepared.preview();
    let nodes = &after.pages[1].content.nodes;
    assert_eq!(
        text(&nodes[0]),
        format!(
            r"Before [*Contact*](page:{}) and \[literal](page:2)",
            ids[1]
        )
    );
    assert_eq!(
        text(&nodes[1]),
        format!(r"[One](page:{}),[\,Two](page:{}), [x]", ids[0], ids[1])
    );
    assert_eq!(after.pages[1].notes, "Literal [note](page:2)");
    let NodeContent::Group { children } = &nodes[2].content else {
        panic!()
    };
    let missing = wireframe::rich_text::source_links(text(&children[0])).unwrap()[0]
        .target
        .clone()
        .unwrap();
    let missing = missing
        .strip_prefix("page:")
        .unwrap()
        .parse::<Id>()
        .unwrap();
    assert_ne!(missing, 90);
    assert!(after.removed_pages.contains(&missing));
    e.apply_project_copy(prepared).unwrap();
    let after = e.document.clone();
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
}
#[test]
fn omitted_inline_destination_cannot_capture_a_destination_page_with_the_same_id() {
    let source = fixture();
    let e = Editor::new(fixture());
    let prepared = e.prepare_project_copy(&source, "other", &[1]).unwrap();
    let imported = prepared.imported_pages()[0];
    let after = prepared.preview();
    let text = text(&after.live_page(imported).unwrap().content.nodes[0]);
    let target = wireframe::rich_text::source_links(text).unwrap()[0]
        .target
        .clone()
        .unwrap();
    let target = target.strip_prefix("page:").unwrap().parse::<Id>().unwrap();
    assert_ne!(target, 2);
    assert!(after.removed_pages.contains(&target));
    assert!(after.live_page(target).is_none());
}
#[test]
fn foreign_clipboard_page_links_do_not_capture_equal_numbered_destinations() {
    let mut source = Editor::new(fixture());
    source.select(10, false);
    source.set_node_link(10, Some(Link::Page(2))).unwrap();
    let copy = source.copy_with_origin("source").unwrap();
    let mut target = Editor::new(fixture());
    target.paste_from(&copy, 10., Some("destination"));
    assert!(target.error.is_none(), "{:?}", target.error);
    let nodes = target.nodes();
    let pasted = nodes.last().unwrap();
    assert_eq!(text(pasted), r"Before [*Contact*] and \[literal](page:2)");
    assert!(
        pasted
            .properties
            .get("link")
            .is_none_or(|v| *v == Value::Null)
    );
}

fn component_fixture() -> Editor {
    let mut e = Editor::new(fixture());
    e.select(11, false);
    e.create_component("Table".into()).unwrap();
    assert!(e.enter(11));
    let child = e.nodes()[0].id;
    e.set_control_property(
        child,
        "text",
        Value::Text(
            "[Head](page:1),[Local](page:2),[Web](https://example.org),[Back](back:)".into(),
        ),
    )
    .unwrap();
    e.leave();
    e
}
#[test]
fn component_override_cells_and_source_text_remap_before_path_correspondence() {
    let mut source = component_fixture();
    source.document.pages[0].content.nodes[2]
        .properties
        .insert(GROUP_NAME.into(), Value::Text("[literal](page:2)".into()));
    source.document.pages[0].content.nodes[2]
        .properties
        .insert(GROUP_NOTES.into(), Value::Text("[notes](page:2)".into()));
    if let NodeContent::Instance { overrides, .. } =
        &mut source.document.pages[0].content.nodes[1].content
    {
        let mut patch = Patch::default();
        patch.properties.insert(
            "text".into(),
            Value::Text("prefix,[orphan](page:90)".into()),
        );
        overrides.push(Override {
            path: vec![999],
            patch,
        });
    } else {
        panic!()
    }
    let mut alternate = source.document.pages[0].content.clone();
    alternate.nodes[0]
        .properties
        .insert("text".into(), Value::Text("[Phone](page:2)".into()));
    source.document.pages[0].alternates.push(Alternate {
        id: 1000,
        name: "Phone".into(),
        version: None,
        notes: String::new(),
        content: alternate,
    });
    source.document.next_id = 1001;
    let e = Editor::new(Project {
        font: None,
        ..Project::default()
    });
    let prepared = e
        .prepare_project_copy(&source.document, "source", &[1, 2])
        .unwrap();
    let ids = prepared.imported_pages();
    let after = prepared.preview();
    let imported = after.live_page(ids[0]).unwrap();
    assert_eq!(
        text(&imported.alternates[0].content.nodes[0]),
        format!("[Phone](page:{})", ids[1])
    );
    assert_eq!(
        imported.content.nodes[2].properties,
        source.document.pages[0].content.nodes[2].properties
    );
    let NodeContent::Instance {
        definition,
        overrides,
    } = &imported.content.nodes[1].content
    else {
        panic!()
    };
    let local = match &overrides[0].patch.properties["text"] {
        Value::Text(s) => s,
        _ => panic!(),
    };
    assert_eq!(
        local,
        &format!(
            "[Head](page:{}),[Local](page:{}),[Web](https://example.org),[Back](back:)",
            ids[0], ids[1]
        )
    );
    let def = after
        .definitions
        .iter()
        .find(|d| d.id == *definition)
        .unwrap();
    assert_eq!(
        text(&def.content.nodes[0]),
        format!(r"[One](page:{}),[\,Two](page:{}), [x]", ids[0], ids[1])
    );
    let orphan = match &overrides[1].patch.properties["text"] {
        Value::Text(s) => s,
        _ => panic!(),
    };
    assert!(!orphan.contains("page:90)"));
    assert!(orphan.starts_with("prefix,[orphan](page:"));
}
#[test]
fn local_clipboard_retains_links_foreign_and_legacy_payloads_clear_only_page_destinations() {
    let mut source = component_fixture();
    source.select(11, false);
    let copy = source.copy_with_origin("same").unwrap();
    let roundtrip: wireframe::workspace::Clipboard =
        serde_json::from_str(&serde_json::to_string(&copy).unwrap()).unwrap();
    for (origin, unknown, retained) in [
        ("same", false, true),
        ("different", false, false),
        ("same", true, false),
    ] {
        let mut target = Editor::new(source.document.clone());
        let before = target.document.clone();
        let mut copy = roundtrip.clone();
        if unknown {
            copy.origin = None;
        }
        target.paste_from(&copy, 10., Some(origin));
        assert!(target.error.is_none(), "{:?}", target.error);
        let nodes = target.nodes();
        let pasted = nodes.last().unwrap();
        let NodeContent::Instance { overrides, .. } = &pasted.content else {
            panic!()
        };
        let value = match &overrides[0].patch.properties["text"] {
            Value::Text(s) => s,
            _ => panic!(),
        };
        assert_eq!(value.contains("page:"), retained);
        assert!(value.contains("[Web](https://example.org),[Back](back:)"));
        let after = target.document.clone();
        target.undo();
        assert_eq!(target.document, before);
        target.redo();
        assert_eq!(target.document, after);
    }
    let copy = source.copy();
    source.paste(&copy, 10.);
    assert!(source.error.is_none());
    let nodes = source.nodes();
    let NodeContent::Instance { overrides, .. } = &nodes.last().unwrap().content else {
        panic!()
    };
    assert!(
        matches!(&overrides[0].patch.properties["text"],Value::Text(s) if s.contains("page:2"))
    );
}
#[test]
fn library_preview_and_foreign_import_clear_inline_targets_without_mutating_original() {
    let source = component_fixture();
    let library = source.document.default_library.unwrap();
    let copy = source.copy_libraries(&[library], "source").unwrap();
    let encoded = serde_json::to_string(&copy).unwrap();
    let preview = copy.resources.as_ref().unwrap().preview_project().unwrap();
    assert_eq!(
        text(&preview.definitions[0].content.nodes[0]),
        r"[One],[\,Two], [x]"
    );
    let mut target = Editor::new(fixture());
    let before = target.document.clone();
    let plan = target.plan_library_import(&copy, "foreign").unwrap();
    let prepared = plan.prepare(&Default::default()).unwrap();
    target.apply_library_import(prepared, "foreign").unwrap();
    assert_eq!(target.document.pages, before.pages);
    assert_eq!(
        text(&target.document.definitions[0].content.nodes[0]),
        r"[One],[\,Two], [x]"
    );
    assert_eq!(serde_json::to_string(&copy).unwrap(), encoded);
    let after = target.document.clone();
    target.undo();
    assert_eq!(target.document, before);
    target.redo();
    assert_eq!(target.document, after);
}
#[test]
fn repeated_page_import_reuses_destinations_for_inline_targets() {
    use std::collections::BTreeMap;
    use wireframe::workspace::PageImportChoice;
    let source = fixture();
    let mut target = Editor::new(Project {
        font: None,
        ..Project::default()
    });
    let copy = target
        .prepare_project_copy(&source, "source", &[1, 2])
        .unwrap();
    let ids = copy.imported_pages().to_vec();
    target.apply_project_copy(copy).unwrap();
    let plan = target
        .plan_project_page_import(&source, "source", &[1, 2])
        .unwrap();
    let prepared = plan
        .prepare(&BTreeMap::from([
            (1, PageImportChoice::KeepBoth),
            (2, PageImportChoice::UseExisting),
        ]))
        .unwrap();
    let landing = prepared.imported_pages()[0];
    let after = prepared.preview();
    assert_eq!(
        text(&after.live_page(landing).unwrap().content.nodes[0]),
        format!(
            r"Before [*Contact*](page:{}) and \[literal](page:2)",
            ids[1]
        )
    );
}
#[test]
fn unsafe_text_complexity_rejects_transfer_without_partial_document_or_history_changes() {
    let mut source = fixture();
    let nested = format!("{} [Contact](page:2)", "{".repeat(20_000));
    assert!(wireframe::rich_text::source_links(&nested).is_err());
    source.pages[0].content.nodes[0]
        .properties
        .insert("text".into(), Value::Text(nested));
    let mut target = Editor::new(fixture());
    let before = target.document.clone();
    assert!(
        target
            .prepare_project_copy(&source, "source", &[1, 2])
            .is_err()
    );
    assert_eq!(target.document, before);
    let mut source = Editor::new(source);
    source.select(10, false);
    let copy = source.copy();
    target.paste(&copy, 0.);
    assert!(target.error.is_some());
    assert_eq!(target.document, before);
}

#[test]
fn clipboard_links_survive_saving_or_renaming_the_same_open_document() {
    let mut e = Editor::new(fixture());
    e.select(10, false);
    let copy = e.copy_with_origin("untitled").unwrap();
    e.paste_from(&copy, 10., Some("saved-project"));
    assert!(e.error.is_none());
    let nodes = e.nodes();
    assert_eq!(
        text(nodes.last().unwrap()),
        r"Before [*Contact*](page:2) and \[literal](page:2)"
    );
}

#[test]
fn combined_resource_import_preserves_page_links_in_local_component_overrides() {
    use std::collections::BTreeMap;
    let source = component_fixture();
    let target = Editor::new(Project {
        font: None,
        ..Project::default()
    });
    let plan = target
        .plan_project_page_import(&source.document, "source", &[1, 2])
        .unwrap();
    let libraries = plan
        .review_libraries(&BTreeMap::new(), &BTreeMap::new())
        .unwrap();
    let prepared = libraries.prepare(&BTreeMap::new()).unwrap();
    let ids = prepared.imported_pages();
    let project = prepared.preview();
    let NodeContent::Instance { overrides, .. } =
        &project.live_page(ids[0]).unwrap().content.nodes[1].content
    else {
        panic!()
    };
    assert_eq!(
        overrides[0].patch.properties["text"],
        Value::Text(format!(
            "[Head](page:{}),[Local](page:{}),[Web](https://example.org),[Back](back:)",
            ids[0], ids[1]
        ))
    );
    assert_eq!(decode(&project.encode().unwrap()).unwrap(), *project);
}
#[test]
fn nested_library_override_text_uses_the_underlying_table_kind() {
    let mut source = component_fixture();
    source.add(wireframe::model::Kind::Label, 400., 200.);
    source.select(11, true);
    source.group();
    source.reorder(true);
    source.create_component("Nested".into()).unwrap();
    let library = source.document.default_library.unwrap();
    let copy = source.copy_libraries(&[library], "source").unwrap();
    let preview = copy.resources.as_ref().unwrap().preview_project().unwrap();
    assert!(
        !serde_json::to_string(&preview.definitions)
            .unwrap()
            .contains("page:")
    );
    let mut target = Editor::new(fixture());
    let plan = target.plan_library_import(&copy, "foreign").unwrap();
    let prepared = plan.prepare(&Default::default()).unwrap();
    target.apply_library_import(prepared, "foreign").unwrap();
    assert!(
        !serde_json::to_string(&target.document.definitions)
            .unwrap()
            .contains("page:")
    );
    assert!(
        serde_json::to_string(&target.document.definitions)
            .unwrap()
            .contains("https://example.org")
    );
}
