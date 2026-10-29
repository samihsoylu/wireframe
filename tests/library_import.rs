// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::collections::{BTreeMap, HashSet};
use wireframe::{
    document::*,
    workspace::{
        Clipboard, Editor, LibraryImportAction as Action, LibraryImportChoice as Choice,
        LibraryImportCounts, LibraryImportPlan,
    },
};
fn instance(id: Id, definition: Id) -> Node {
    let mut n = Node::group(id, Point::new(40., 50.), vec![]);
    n.content = NodeContent::Instance {
        definition,
        overrides: vec![],
    };
    n
}
fn fixture() -> Editor {
    let mut p = Project {
        font: None,
        next_id: 31,
        ..Project::default()
    };
    let mut button = Node::control(23, "button", Point::default(), Size::default());
    button
        .properties
        .insert("icon".into(), Value::Icon(IconSource::Asset(30)));
    p.definitions = vec![
        Definition {
            id: 11,
            name: "Container".into(),
            content: Content {
                nodes: vec![instance(13, 21)],
                paint_order: vec![],
            },
        },
        Definition {
            id: 21,
            name: "Button".into(),
            content: Content {
                nodes: vec![button],
                paint_order: vec![],
            },
        },
    ];
    p.libraries = [(10, 11, 12, "Forms"), (20, 21, 22, "Actions")]
        .into_iter()
        .map(|(id, definition, node, name)| ComponentLibrary {
            id,
            trashed: false,
            name: name.into(),
            notes: format!("Notes for {name}"),
            content: Content {
                nodes: vec![instance(node, definition)],
                paint_order: vec![],
            },
            sources: vec![ComponentSource {
                definition,
                node,
                notes: "Source notes".into(),
            }],
            imports: vec![],
        })
        .collect();
    let mut label = Node::control(14, "label", Point::new(300., 200.), Size::default());
    label
        .properties
        .insert("link".into(), Value::Link(Link::Page(1)));
    label.locked = true;
    p.libraries[0].content.nodes.push(label);
    p.libraries[0].content.paint_order = vec![vec![14], vec![12]];
    p.libraries[0].content.nodes[0].crop = Some(Crop {
        width: 0.8,
        ..Crop::ALL
    });
    p.default_library = Some(10);
    p.assets.push(Asset {
        attribution: None,
        trashed: false,
        id: 30,
        name: "Action icon".into(),
        media_type: "image/png".into(),
        bytes: vec![1, 2, 3],
    });
    p.validate().unwrap();
    Editor::new(p)
}
fn copy() -> Clipboard {
    fixture().copy_libraries(&[10, 20], "source").unwrap()
}
fn choices(plan: &LibraryImportPlan, choice: Choice) -> BTreeMap<Id, Choice> {
    plan.entries()
        .iter()
        .filter(|e| e.action == Action::Conflict)
        .map(|e| (e.incoming, choice))
        .collect()
}
fn apply(
    e: &mut Editor,
    clipboard: &Clipboard,
    origin: &str,
    choice: Choice,
) -> LibraryImportCounts {
    let plan = e.plan_library_import(clipboard, origin).unwrap();
    let prepared = plan.prepare(&choices(&plan, choice)).unwrap();
    e.apply_library_import(prepared, origin).unwrap()
}
fn mapped(e: &Editor, source: Id) -> Id {
    e.document
        .libraries
        .iter()
        .find_map(|l| {
            l.imports
                .iter()
                .find(|i| i.project == "source" && i.library == source)
                .map(|_| l.id)
        })
        .unwrap()
}
fn definition(e: &Editor, library: Id) -> Id {
    e.document
        .libraries
        .iter()
        .find(|l| l.id == library)
        .unwrap()
        .sources[0]
        .definition
}
fn refers(node: &Node) -> Id {
    let NodeContent::Instance { definition, .. } = node.content else {
        panic!()
    };
    definition
}
#[test]
fn same_project_copy_is_independent_preserves_target_and_undoes_exactly() {
    let mut e = fixture();
    e.edit_library(10).unwrap();
    let before = e.document.clone();
    let selected = e.selection.clone();
    let c = copy();
    let plan = e.plan_library_import(&c, "source").unwrap();
    assert!(
        plan.entries()
            .iter()
            .all(|entry| entry.action == Action::Add)
    );
    assert_eq!(e.document, before);
    let counts = e
        .apply_library_import(plan.prepare(&BTreeMap::new()).unwrap(), "source")
        .unwrap();
    assert_eq!(
        counts,
        LibraryImportCounts {
            added: 2,
            replaced: 0,
            reused: 0
        }
    );
    assert_eq!(e.library, Some(10));
    assert_eq!(e.selection, selected);
    assert_eq!(e.document.pages, before.pages);
    assert_eq!(e.document.libraries[..2], before.libraries);
    let a = &e.document.libraries[2];
    let b = &e.document.libraries[3];
    assert_eq!(a.name, "Forms (1)");
    assert_eq!(b.name, "Actions (1)");
    assert_eq!(a.content.nodes[1], before.libraries[0].content.nodes[1]);
    assert_eq!(
        a.content.paint_order,
        before.libraries[0].content.paint_order
    );
    assert_eq!(
        a.content.nodes[0].crop,
        before.libraries[0].content.nodes[0].crop
    );
    let copied_a = e
        .document
        .definitions
        .iter()
        .find(|d| d.id == a.sources[0].definition)
        .unwrap();
    assert_eq!(refers(&copied_a.content.nodes[0]), b.sources[0].definition);
    assert_ne!(b.sources[0].definition, 21);
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
}
#[test]
fn same_project_reuses_current_dependency_and_restores_missing_resource_identity() {
    let mut e = fixture();
    let c = e.copy_libraries(&[10], "source").unwrap();
    e.document.definitions[1].name = "Current action".into();
    let before = e.document.clone();
    let counts = apply(&mut e, &c, "source", Choice::UseNew);
    assert_eq!(
        counts,
        LibraryImportCounts {
            added: 1,
            replaced: 0,
            reused: 1
        }
    );
    assert_eq!(e.document.libraries[1], before.libraries[1]);
    assert_eq!(e.document.definitions[1], before.definitions[1]);
    let a = e.document.definitions.last().unwrap();
    assert_eq!(refers(&a.content.nodes[0]), 21);
    // Simulate a removed library. Existing dependent references intentionally remain.
    let mut e = fixture();
    let c = e.copy_libraries(&[20], "source").unwrap();
    e.document.libraries.retain(|l| l.id != 20);
    e.document.definitions.retain(|d| d.id != 21);
    e.document.validate().unwrap();
    let before = e.document.clone();
    apply(&mut e, &c, "source", Choice::UseNew);
    assert!(e.document.libraries.iter().any(|l| l.id == 20));
    assert!(e.document.definitions.iter().any(|d| d.id == 21));
    e.undo();
    assert_eq!(e.document, before);
}
#[test]
fn foreign_repeated_import_survives_serialization_without_new_history_or_allocator_changes() {
    let mut e = Editor::default();
    let c = copy();
    let before = e.document.clone();
    assert_eq!(apply(&mut e, &c, "destination", Choice::UseNew).added, 2);
    assert_eq!(e.document.pages, before.pages);
    let a = mapped(&e, 10);
    let b = mapped(&e, 20);
    assert_ne!(a, 10);
    assert_ne!(b, 20);
    let d = e
        .document
        .definitions
        .iter()
        .find(|d| d.id == definition(&e, a))
        .unwrap();
    assert_eq!(refers(&d.content.nodes[0]), definition(&e, b));
    let library = e.document.libraries.iter().find(|l| l.id == a).unwrap();
    assert_eq!(
        library.content.nodes[1].properties.get("link"),
        Some(&Value::Null)
    );
    assert_eq!(e.document.assets.len(), 1);
    let encoded = e.document.encode().unwrap();
    let mut reopened = Editor::new(decode(&encoded).unwrap());
    let before = reopened.document.clone();
    assert_eq!(
        apply(&mut reopened, &c, "destination", Choice::UseNew),
        LibraryImportCounts {
            added: 0,
            replaced: 0,
            reused: 2
        }
    );
    assert_eq!(reopened.document, before);
    assert!(reopened.undo_name().is_none());
}
#[test]
fn all_conflict_choices_are_atomic_and_replacement_undo_restores_local_edits() {
    let c = copy();
    let mut e = Editor::default();
    apply(&mut e, &c, "destination", Choice::UseNew);
    let a = mapped(&e, 10);
    let adef = definition(&e, a);
    let b = mapped(&e, 20);
    let bdef = definition(&e, b);
    let id = e.document.allocate_id().unwrap();
    let mut dependent = instance(id, adef);
    if let NodeContent::Instance { overrides, .. } = &mut dependent.content {
        let mut patch = Patch::default();
        patch
            .properties
            .insert("text".into(), Value::Text("Local override".into()));
        overrides.push(Override {
            path: vec![13, 23],
            patch,
        });
    }
    e.document.pages[0].content.nodes.push(dependent);
    e.begin("Local edit");
    e.document
        .definitions
        .iter_mut()
        .find(|d| d.id == bdef)
        .unwrap()
        .name = "Local name".into();
    e.commit();
    let before = e.document.clone();
    let undo = e.undo_name().map(str::to_owned);
    let plan = e.plan_library_import(&c, "destination").unwrap();
    assert_eq!(
        plan.entries()
            .iter()
            .filter(|e| e.action == Action::Conflict)
            .count(),
        2
    );
    assert!(plan.prepare(&BTreeMap::new()).is_err());
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), undo.as_deref());
    let kept = plan.prepare(&choices(&plan, Choice::UseExisting)).unwrap();
    assert_eq!(
        e.apply_library_import(kept, "destination").unwrap().reused,
        2
    );
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), undo.as_deref());
    assert_eq!(apply(&mut e, &c, "destination", Choice::KeepBoth).added, 2);
    assert_eq!(e.document.libraries[..2], before.libraries);
    assert_eq!(e.document.pages, before.pages);
    assert_eq!(e.document.libraries[2].name, "Forms (1)");
    assert_eq!(e.document.libraries[3].name, "Actions (1)");
    let new_b = e.document.libraries[3].sources[0].definition;
    let new_a = e.document.libraries[2].sources[0].definition;
    assert_eq!(
        refers(
            &e.document
                .definitions
                .iter()
                .find(|d| d.id == new_a)
                .unwrap()
                .content
                .nodes[0]
        ),
        new_b
    );
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(
        apply(&mut e, &c, "destination", Choice::UseNew),
        LibraryImportCounts {
            added: 0,
            replaced: 1,
            reused: 1
        }
    );
    assert_eq!(mapped(&e, 10), a);
    assert_eq!(mapped(&e, 20), b);
    assert_eq!(definition(&e, a), adef);
    assert_eq!(definition(&e, b), bdef);
    assert_eq!(e.document.pages, before.pages);
    assert_eq!(
        e.document
            .definitions
            .iter()
            .find(|d| d.id == bdef)
            .unwrap()
            .name,
        "Button"
    );
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
}
#[test]
fn stale_document_or_destination_origin_cannot_apply_a_prepared_import() {
    let c = copy();
    let mut e = Editor::default();
    let plan = e.plan_library_import(&c, "destination").unwrap();
    let prepared = plan.prepare(&BTreeMap::new()).unwrap();
    let before = e.document.clone();
    assert!(
        e.apply_library_import(prepared, "saved-as-new-origin")
            .is_err()
    );
    assert_eq!(e.document, before);
    assert!(e.undo_name().is_none());
    let prepared = plan.prepare(&BTreeMap::new()).unwrap();
    e.document.title = "Edited while choosing".into();
    let edited = e.document.clone();
    assert!(e.apply_library_import(prepared, "destination").is_err());
    assert_eq!(e.document, edited);
}
#[test]
fn exact_origins_win_over_names_and_distinct_inputs_cannot_claim_one_target() {
    let c = copy();
    let mut e = Editor::default();
    apply(&mut e, &c, "destination", Choice::UseNew);
    let b = mapped(&e, 20);
    let mut incoming = c.clone();
    let r = incoming.resources.as_mut().unwrap();
    r.libraries[0].id = 90;
    r.requested[0] = 90;
    r.libraries[0].name = "Actions".into();
    let plan = e.plan_library_import(&incoming, "destination").unwrap();
    assert_eq!(plan.entries()[0].existing, None);
    assert_eq!(plan.entries()[1].existing, Some(b));
    let mut e = Editor::default();
    let only = e.create_library("Same").unwrap();
    let mut incoming = c.clone();
    for l in &mut incoming.resources.as_mut().unwrap().libraries {
        l.name = "Same".into();
    }
    let plan = e.plan_library_import(&incoming, "destination").unwrap();
    assert_eq!(
        plan.entries()
            .iter()
            .filter(|e| e.existing == Some(only))
            .count(),
        1
    );
    assert_eq!(
        plan.entries()
            .iter()
            .filter(|e| e.action == Action::Add)
            .count(),
        1
    );
    e.apply_library_import(
        plan.prepare(&choices(&plan, Choice::UseNew)).unwrap(),
        "destination",
    )
    .unwrap();
    assert_eq!(e.document.libraries.len(), 2);
    assert_eq!(
        e.document
            .definitions
            .iter()
            .map(|d| d.id)
            .collect::<HashSet<_>>()
            .len(),
        2
    );
}
#[test]
fn missing_foreign_source_reuses_its_placeholder_identity_and_later_restores_it() {
    let mut source = Editor::default();
    source.document.font = None;
    let lib = source.create_library("Missing source").unwrap();
    let id = source.document.allocate_id().unwrap();
    source.document.libraries[0]
        .content
        .nodes
        .push(instance(id, 100));
    source.document.next_id = 101;
    source.document.validate().unwrap();
    let c = source.copy_libraries(&[lib], "source").unwrap();
    let mut e = Editor::default();
    apply(&mut e, &c, "destination", Choice::UseNew);
    let missing = refers(&e.document.libraries[0].content.nodes[0]);
    assert_ne!(missing, 100);
    let before = e.document.clone();
    assert_eq!(apply(&mut e, &c, "destination", Choice::UseNew).reused, 1);
    assert_eq!(e.document, before);
    let node = source.document.allocate_id().unwrap();
    source.document.definitions.push(Definition {
        id: 100,
        name: "Restored".into(),
        content: Content::default(),
    });
    source.document.libraries[0]
        .content
        .nodes
        .push(instance(node, 100));
    source.document.libraries[0].sources.push(ComponentSource {
        definition: 100,
        node,
        notes: String::new(),
    });
    source.document.validate().unwrap();
    let c = source.copy_libraries(&[lib], "source").unwrap();
    assert_eq!(apply(&mut e, &c, "destination", Choice::UseNew).replaced, 1);
    assert_eq!(e.document.definitions[0].id, missing);
    assert_eq!(refers(&e.document.libraries[0].content.nodes[0]), missing);
    e.undo();
    assert_eq!(e.document, before);
}
#[test]
fn shared_historical_targets_do_not_collapse_distinct_restored_sources() {
    let c = copy();
    let mut e = Editor::default();
    let a = e.create_library("Forms").unwrap();
    let b = e.create_library("Actions").unwrap();
    for (local, foreign, definition) in [(a, 10, 11), (b, 20, 21)] {
        e.document
            .libraries
            .iter_mut()
            .find(|l| l.id == local)
            .unwrap()
            .imports
            .push(LibraryImport {
                project: "source".into(),
                library: foreign,
                definitions: BTreeMap::from([(definition, 999)]),
            });
    }
    e.document.next_id = 1000;
    e.document.validate().unwrap();
    let before = e.document.clone();
    apply(&mut e, &c, "destination", Choice::UseNew);
    assert_ne!(definition(&e, a), definition(&e, b));
    let def = e
        .document
        .definitions
        .iter()
        .find(|d| d.id == definition(&e, a))
        .unwrap();
    assert_eq!(refers(&def.content.nodes[0]), definition(&e, b));
    e.undo();
    assert_eq!(e.document, before);
}
#[test]
fn keeping_changed_dependency_does_not_create_an_empty_parent_replacement() {
    let c = copy();
    let mut e = Editor::default();
    apply(&mut e, &c, "destination", Choice::UseNew);
    let b = definition(&e, mapped(&e, 20));
    e.begin("Local edit");
    e.document
        .definitions
        .iter_mut()
        .find(|d| d.id == b)
        .unwrap()
        .name = "Local action".into();
    e.commit();
    let before = e.document.clone();
    let undo = e.undo_name().map(str::to_owned);
    let plan = e.plan_library_import(&c, "destination").unwrap();
    let prepared = plan
        .prepare(&BTreeMap::from([
            (10, Choice::UseNew),
            (20, Choice::UseExisting),
        ]))
        .unwrap();
    let counts = e.apply_library_import(prepared, "destination").unwrap();
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), undo.as_deref());
    assert_eq!(
        counts,
        LibraryImportCounts {
            added: 0,
            replaced: 0,
            reused: 2
        }
    );
}
#[test]
fn empty_library_import_is_real_work_and_repeating_it_is_not() {
    let mut source = Editor::default();
    let id = source.create_library("Empty").unwrap();
    let c = source.copy_libraries(&[id], "source").unwrap();
    let mut target = Editor::default();
    let before = target.document.clone();
    assert_eq!(
        apply(&mut target, &c, "destination", Choice::UseNew).added,
        1
    );
    assert_eq!(target.document.libraries.len(), 1);
    assert!(target.document.libraries[0].sources.is_empty());
    let after = target.document.clone();
    let undo = target.undo_name().map(str::to_owned);
    assert_eq!(
        apply(&mut target, &c, "destination", Choice::UseNew).reused,
        1
    );
    assert_eq!(target.document, after);
    assert_eq!(target.undo_name(), undo.as_deref());
    target.undo();
    assert_eq!(target.document, before);
    target.redo();
    assert_eq!(target.document, after);
}

#[test]
fn local_library_notes_source_placement_lock_and_root_id_do_not_conflict() {
    for change in 0..4 {
        let c = copy();
        let mut e = Editor::new(Project::default());
        apply(&mut e, &c, "destination", Choice::UseNew);
        let id = mapped(&e, 20);
        let fresh = e.document.allocate_id().unwrap();
        let l = e
            .document
            .libraries
            .iter_mut()
            .find(|l| l.id == id)
            .unwrap();
        match change {
            0 => l.notes = "Local library notes".into(),
            1 => l.content.nodes[0].origin.x += 10.,
            2 => l.content.nodes[0].locked = true,
            3 => {
                let old = l.sources[0].node;
                l.sources[0].node = fresh;
                l.content.nodes[0].id = fresh;
                for path in &mut l.content.paint_order {
                    if path[0] == old {
                        path[0] = fresh;
                    }
                }
            }
            _ => unreachable!(),
        }
        e.document.validate().unwrap();
        // Reopen removes prior history and proves correspondence persistence.
        let mut e = Editor::new(decode(&e.document.encode().unwrap()).unwrap());
        let before = e.document.clone();
        let plan = e.plan_library_import(&c, "destination").unwrap();
        assert!(
            plan.entries().iter().all(|e| e.action == Action::Reuse),
            "change {change}: {:?}",
            plan.entries()
        );
        let counts = e
            .apply_library_import(plan.prepare(&BTreeMap::new()).unwrap(), "destination")
            .unwrap();
        assert_eq!(
            counts,
            LibraryImportCounts {
                added: 0,
                replaced: 0,
                reused: 2
            }
        );
        assert_eq!(e.document, before);
        e.undo();
        assert_eq!(e.document, before);
    }
}

#[test]
fn source_notes_child_layout_and_crop_still_conflict_and_replace_exactly() {
    for change in 0..3 {
        let c = copy();
        let mut e = Editor::new(Project::default());
        apply(&mut e, &c, "destination", Choice::UseNew);
        let id = mapped(&e, 20);
        let source = definition(&e, id);
        let l = e
            .document
            .libraries
            .iter_mut()
            .find(|l| l.id == id)
            .unwrap();
        // Source artwork is replaced, while resource-level local notes survive.
        l.notes = "Local note".into();
        l.content.nodes[0].origin.x += 10.;
        l.content.nodes[0].locked = true;
        match change {
            0 => l.sources[0].notes = "Changed source note".into(),
            1 => {
                e.document
                    .definitions
                    .iter_mut()
                    .find(|d| d.id == source)
                    .unwrap()
                    .content
                    .nodes[0]
                    .origin
                    .x += 10.
            }
            2 => {
                l.content.nodes[0].crop = Some(Crop {
                    width: 0.5,
                    ..Crop::ALL
                })
            }
            _ => unreachable!(),
        }
        let before = e.document.clone();
        let plan = e.plan_library_import(&c, "destination").unwrap();
        assert!(plan.entries().iter().all(|e| e.action == Action::Conflict));
        let prepared = plan.prepare(&choices(&plan, Choice::UseNew)).unwrap();
        let counts = e.apply_library_import(prepared, "destination").unwrap();
        assert_eq!(
            counts,
            LibraryImportCounts {
                added: 0,
                replaced: 1,
                reused: 1
            }
        );
        let replaced = e.document.clone();
        assert_ne!(replaced, before);
        let l = e.document.libraries.iter().find(|l| l.id == id).unwrap();
        assert_eq!(l.notes, "Local note");
        assert!(!l.content.nodes[0].locked);
        assert_eq!(l.content.nodes[0].origin, Point::new(40., 50.));
        e.undo();
        assert_eq!(e.document, before);
        e.redo();
        assert_eq!(e.document, replaced);
    }
}

#[test]
fn imported_source_cut_paste_updates_bindings_and_reuses_current_identity() {
    let c = copy();
    let mut e = Editor::new(Project::default());
    apply(&mut e, &c, "destination", Choice::UseNew);
    let library = mapped(&e, 20);
    let old = definition(&e, library);
    let source = e.copy_source(old, "destination").unwrap();
    let before = e.document.clone();
    e.delete_definition(old).unwrap();
    let cut = e.document.clone();
    e.edit_library(library).unwrap();
    e.paste_from(&source, 10., Some("destination"));
    assert!(e.error.is_none(), "{:?}", e.error);
    let current = definition(&e, library);
    assert_ne!(current, old);
    let l = e
        .document
        .libraries
        .iter()
        .find(|l| l.id == library)
        .unwrap();
    assert_eq!(l.imports[0].definitions[&21], current);
    let pasted = e.document.clone();
    e.undo();
    assert_eq!(e.document, cut);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    e.redo();
    assert_eq!(e.document, pasted);
    let mut reopened = Editor::new(decode(&pasted.encode().unwrap()).unwrap());
    let plan = reopened.plan_library_import(&c, "destination").unwrap();
    assert!(
        plan.entries().iter().all(|e| e.action == Action::Reuse),
        "{:?}",
        plan.entries()
    );
    reopened
        .apply_library_import(plan.prepare(&BTreeMap::new()).unwrap(), "destination")
        .unwrap();
    assert_eq!(reopened.document, pasted);
    // A newly added dependent must use the current source, not its retired ID.
    let mut incoming = fixture();
    incoming.document.libraries[0].id = 40;
    incoming.document.libraries[0].name = "New Forms".into();
    incoming.document.default_library = Some(40);
    incoming.document.next_id = 41;
    let new_copy = incoming.copy_libraries(&[40], "source").unwrap();
    let counts = apply(&mut reopened, &new_copy, "destination", Choice::UseNew);
    assert_eq!(
        counts,
        LibraryImportCounts {
            added: 1,
            replaced: 0,
            reused: 1
        }
    );
    let parent = definition(&reopened, mapped(&reopened, 40));
    assert_eq!(
        refers(
            &reopened
                .document
                .definitions
                .iter()
                .find(|d| d.id == parent)
                .unwrap()
                .content
                .nodes[0]
        ),
        current
    );
    reopened.undo();
    assert_eq!(reopened.document, pasted);
}

#[test]
fn stale_import_binding_prefers_unique_current_source_for_new_dependents() {
    let c = copy();
    let mut e = Editor::new(Project::default());
    apply(&mut e, &c, "destination", Choice::UseNew);
    let library = mapped(&e, 20);
    let old = definition(&e, library);
    let source = e.copy_source(old, "destination").unwrap();
    e.delete_definition(old).unwrap();
    e.edit_library(library).unwrap();
    e.paste_from(&source, 0., Some("destination"));
    assert!(e.error.is_none(), "{:?}", e.error);
    let current = definition(&e, library);
    // Simulate a saved project from before source restoration updated bindings.
    e.document
        .libraries
        .iter_mut()
        .find(|l| l.id == library)
        .unwrap()
        .imports[0]
        .definitions
        .insert(21, old);
    let before = e.document.clone();
    assert!(
        e.plan_library_import(&c, "destination")
            .unwrap()
            .entries()
            .iter()
            .all(|e| e.action == Action::Reuse)
    );
    let mut incoming = fixture();
    incoming.document.libraries[0].id = 40;
    incoming.document.libraries[0].name = "New Forms".into();
    incoming.document.default_library = Some(40);
    incoming.document.next_id = 41;
    let new_copy = incoming.copy_libraries(&[40], "source").unwrap();
    let counts = apply(&mut e, &new_copy, "destination", Choice::UseNew);
    assert_eq!(
        counts,
        LibraryImportCounts {
            added: 1,
            replaced: 0,
            reused: 1
        }
    );
    let parent = definition(&e, mapped(&e, 40));
    assert_eq!(
        refers(
            &e.document
                .definitions
                .iter()
                .find(|d| d.id == parent)
                .unwrap()
                .content
                .nodes[0]
        ),
        current
    );
    assert!(!e.document.definitions.iter().any(|d| d.id == old));
    e.undo();
    assert_eq!(e.document, before);
}

#[test]
fn replacement_retains_resource_notes_and_counts_unchanged_parent_as_reused() {
    let c = copy();
    let mut e = Editor::new(Project::default());
    let initial = e.document.clone();
    apply(&mut e, &c, "destination", Choice::UseNew);
    let parent = mapped(&e, 10);
    let child = mapped(&e, 20);
    e.document
        .libraries
        .iter_mut()
        .find(|l| l.id == parent)
        .unwrap()
        .notes = "Local parent note".into();
    e.document
        .libraries
        .iter_mut()
        .find(|l| l.id == child)
        .unwrap()
        .sources[0]
        .notes = "Changed child".into();
    let before = e.document.clone();
    let plan = e.plan_library_import(&c, "destination").unwrap();
    assert!(plan.entries().iter().all(|e| e.action == Action::Conflict));
    // Keep the changed dependency and choose replacement for its parent.
    // Local resource notes survive, so the parent is also unchanged.
    let prepared = plan
        .prepare(&BTreeMap::from([
            (10, Choice::UseNew),
            (20, Choice::UseExisting),
        ]))
        .unwrap();
    assert_eq!(
        e.apply_library_import(prepared, "destination").unwrap(),
        LibraryImportCounts {
            added: 0,
            replaced: 0,
            reused: 2
        }
    );
    assert_eq!(
        e.document
            .libraries
            .iter()
            .find(|l| l.id == parent)
            .unwrap()
            .notes,
        "Local parent note"
    );
    assert_eq!(
        e.document.libraries.iter().find(|l| l.id == child),
        before.libraries.iter().find(|l| l.id == child)
    );
    assert_eq!(e.document, before);
    e.undo();
    assert_eq!(e.document, initial);
}

#[test]
fn root_normalization_preserves_ordinary_controls_and_effective_stacking_order() {
    for change in 0..3 {
        let c = copy();
        let mut e = Editor::new(Project::default());
        apply(&mut e, &c, "destination", Choice::UseNew);
        let id = mapped(&e, 10);
        let fresh = e.document.allocate_id().unwrap();
        let l = e
            .document
            .libraries
            .iter_mut()
            .find(|l| l.id == id)
            .unwrap();
        let old = l.sources[0].node;
        l.sources[0].node = fresh;
        l.content.nodes[0].id = fresh;
        for path in &mut l.content.paint_order {
            if path[0] == old {
                path[0] = fresh;
            }
        }
        match change {
            0 => (),
            1 => l.content.nodes[1].origin.x += 10.,
            2 => l.content.paint_order.reverse(),
            _ => unreachable!(),
        }
        let before = e.document.clone();
        let plan = e.plan_library_import(&c, "destination").unwrap();
        assert_eq!(
            plan.entries()
                .iter()
                .find(|e| e.incoming == 10)
                .unwrap()
                .action,
            if change == 0 {
                Action::Reuse
            } else {
                Action::Conflict
            }
        );
        let prepared = plan.prepare(&choices(&plan, Choice::UseExisting)).unwrap();
        e.apply_library_import(prepared, "destination").unwrap();
        assert_eq!(e.document, before);
    }
}

#[test]
fn scoped_control_id_cannot_alias_source_in_comparison_paint_order() {
    let mut incoming = fixture();
    let mut e = Editor::new(Project::default());
    apply(&mut e, &copy(), "destination", Choice::UseNew);
    let library = mapped(&e, 10);
    let source = definition(&e, library);
    for l in [
        &mut incoming.document.libraries[0],
        e.document
            .libraries
            .iter_mut()
            .find(|l| l.id == library)
            .unwrap(),
    ] {
        let old = l.content.nodes[1].id;
        l.content.nodes[1].id = source;
        for path in &mut l.content.paint_order {
            if path[0] == old {
                path[0] = source;
            }
        }
    }
    let c = incoming.copy_libraries(&[10, 20], "source").unwrap();
    assert!(
        e.plan_library_import(&c, "destination")
            .unwrap()
            .entries()
            .iter()
            .all(|e| e.action == Action::Reuse)
    );
    e.document
        .libraries
        .iter_mut()
        .find(|l| l.id == library)
        .unwrap()
        .content
        .paint_order
        .reverse();
    assert_eq!(
        e.plan_library_import(&c, "destination")
            .unwrap()
            .entries()
            .iter()
            .find(|e| e.incoming == 10)
            .unwrap()
            .action,
        Action::Conflict
    );
}

fn path_fixture() -> Editor {
    let mut e = fixture();
    let button = e.document.definitions[1].content.nodes.remove(0);
    e.document.definitions[1].content.nodes = vec![Node::group(24, Point::default(), vec![button])];
    if let NodeContent::Instance { overrides, .. } =
        &mut e.document.definitions[0].content.nodes[0].content
    {
        overrides.push(text_override(vec![24, 23], "Nested source override"));
    }
    e.document.validate().unwrap();
    e
}
fn text_override(path: NodePath, text: &str) -> Override {
    Override {
        path,
        patch: Patch {
            properties: Properties::from([("text".into(), Value::Text(text.into()))]),
            ..Patch::default()
        },
    }
}
fn renumber_source_children(project: &mut Project) {
    fn visit(nodes: &mut [Node]) {
        for n in nodes {
            n.id += 100;
            match &mut n.content {
                NodeContent::Group { children } => visit(children),
                NodeContent::Instance { overrides, .. } => {
                    for o in overrides {
                        for id in &mut o.path {
                            *id += 100;
                        }
                    }
                }
                _ => (),
            }
        }
    }
    for d in &mut project.definitions {
        visit(&mut d.content.nodes);
    }
    project.next_id = project.next_id.max(500);
    project.validate().unwrap();
}
fn resolved_texts(project: &Project, node: &Node) -> (Vec<String>, usize) {
    fn visit(n: &Node, out: &mut Vec<String>) {
        if let Some(Value::Text(text)) = n.properties.get("text") {
            out.push(text.clone());
        }
        if let NodeContent::Group { children } = &n.content {
            for n in children {
                visit(n, out);
            }
        }
    }
    let resolved = project.resolve(node).unwrap();
    let mut out = vec![];
    visit(&resolved.root, &mut out);
    (out, resolved.orphan_overrides.len())
}
#[test]
fn descendant_renumbering_and_nested_override_paths_reuse_without_history() {
    let c = path_fixture().copy_libraries(&[10, 20], "source").unwrap();
    let mut e = Editor::default();
    apply(&mut e, &c, "destination", Choice::UseNew);
    renumber_source_children(&mut e.document);
    let mut e = Editor::new(decode(&e.document.encode().unwrap()).unwrap());
    let before = e.document.clone();
    let counts = apply(&mut e, &c, "destination", Choice::UseNew);
    assert_eq!(
        counts,
        LibraryImportCounts {
            added: 0,
            replaced: 0,
            reused: 2
        }
    );
    assert_eq!(e.document, before);
    assert!(e.undo_name().is_none());
    let n = instance(800, definition(&e, mapped(&e, 10)));
    assert_eq!(
        resolved_texts(&e.document, &n),
        (vec!["Nested source override".into()], 0)
    );
}
#[test]
fn new_parent_override_paths_follow_reused_renumbered_dependency() {
    let mut incoming = path_fixture();
    let c = incoming.copy_libraries(&[10, 20], "source").unwrap();
    let mut e = Editor::default();
    apply(&mut e, &c, "destination", Choice::UseNew);
    renumber_source_children(&mut e.document);
    let before = e.document.clone();
    incoming.document.libraries[0].id = 40;
    incoming.document.libraries[0].name = "New parent".into();
    incoming.document.default_library = Some(40);
    incoming.document.next_id = 41;
    let c = incoming.copy_libraries(&[40], "source").unwrap();
    assert_eq!(
        apply(&mut e, &c, "destination", Choice::UseNew),
        LibraryImportCounts {
            added: 1,
            replaced: 0,
            reused: 1
        }
    );
    let n = instance(900, definition(&e, mapped(&e, 40)));
    assert_eq!(
        resolved_texts(&e.document, &n),
        (vec!["Nested source override".into()], 0)
    );
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
}
#[test]
fn changed_source_with_new_child_ids_preserves_existing_nested_local_override() {
    let mut incoming = path_fixture();
    let c = incoming.copy_libraries(&[10, 20], "source").unwrap();
    let mut e = Editor::default();
    apply(&mut e, &c, "destination", Choice::UseNew);
    renumber_source_children(&mut e.document);
    let mut n = instance(501, definition(&e, mapped(&e, 10)));
    if let NodeContent::Instance { overrides, .. } = &mut n.content {
        overrides.push(text_override(vec![113, 124, 123], "Local page text"));
    }
    e.document.next_id = 502;
    e.document.pages[0].content.nodes.push(n.clone());
    let local_library = e.create_library("Local consumers").unwrap();
    let outer = e
        .create_empty_source(local_library, "Outer", Point::default())
        .unwrap();
    e.document
        .definitions
        .iter_mut()
        .find(|d| d.id == outer)
        .unwrap()
        .content
        .nodes
        .push(n.clone());
    e.document
        .libraries
        .iter_mut()
        .find(|l| l.id == local_library)
        .unwrap()
        .content
        .nodes
        .push(n.clone());
    let alternate = e.document.allocate_id().unwrap();
    e.document.pages[0].alternates.push(Alternate {
        id: alternate,
        name: "Alternate".into(),
        version: None,
        notes: "Local notes".into(),
        content: Content {
            nodes: vec![n.clone()],
            paint_order: vec![],
        },
    });
    e.document.validate().unwrap();
    let before = e.document.clone();
    let NodeContent::Group { children } =
        &mut incoming.document.definitions[1].content.nodes[0].content
    else {
        panic!()
    };
    children[0]
        .properties
        .insert("text".into(), Value::Text("Changed incoming text".into()));
    let c = incoming.copy_libraries(&[10, 20], "source").unwrap();
    let counts = apply(&mut e, &c, "destination", Choice::UseNew);
    assert_eq!(
        counts,
        LibraryImportCounts {
            added: 0,
            replaced: 1,
            reused: 1
        }
    );
    assert_eq!(e.document.pages, before.pages);
    for content in [
        &e.document.pages[0].alternates[0].content,
        &e.document
            .definitions
            .iter()
            .find(|d| d.id == outer)
            .unwrap()
            .content,
    ] {
        assert_eq!(
            resolved_texts(&e.document, &content.nodes[0]),
            (vec!["Local page text".into()], 0)
        );
    }
    let ordinary = &e
        .document
        .libraries
        .iter()
        .find(|l| l.id == local_library)
        .unwrap()
        .content
        .nodes[1];
    assert_eq!(
        resolved_texts(&e.document, ordinary),
        (vec!["Local page text".into()], 0)
    );
    assert_eq!(
        resolved_texts(&e.document, &n),
        (vec!["Local page text".into()], 0)
    );
    let source = definition(&e, mapped(&e, 20));
    assert_eq!(
        resolved_texts(&e.document, &instance(800, source)),
        (vec!["Changed incoming text".into()], 0)
    );
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
}

#[test]
fn identity_permutations_of_identical_siblings_preserve_destination_and_local_edits() {
    let mut incoming = path_fixture();
    let NodeContent::Group { children } =
        &mut incoming.document.definitions[1].content.nodes[0].content
    else {
        panic!()
    };
    let mut duplicate = children[0].clone();
    duplicate.id = 25;
    children.push(duplicate);
    let c = incoming.copy_libraries(&[10, 20], "source").unwrap();
    let mut e = Editor::default();
    apply(&mut e, &c, "destination", Choice::UseNew);
    let id = definition(&e, mapped(&e, 20));
    let d = e
        .document
        .definitions
        .iter_mut()
        .find(|d| d.id == id)
        .unwrap();
    let NodeContent::Group { children } = &mut d.content.nodes[0].content else {
        panic!()
    };
    children[0].id = 25;
    children[1].id = 23;
    // Update the source's own referring override to the same structural child.
    let parent = definition(&e, mapped(&e, 10));
    let NodeContent::Instance { overrides, .. } = &mut e
        .document
        .definitions
        .iter_mut()
        .find(|d| d.id == parent)
        .unwrap()
        .content
        .nodes[0]
        .content
    else {
        panic!()
    };
    overrides[0].path = vec![24, 25];
    let before = e.document.clone();
    let plan = e.plan_library_import(&c, "destination").unwrap();
    assert!(plan.entries().iter().all(|e| e.action == Action::Reuse));
    let prepared = plan.prepare(&BTreeMap::new()).unwrap();
    assert_eq!(prepared.newly_orphaned_overrides(), 0);
    e.apply_library_import(prepared, "destination").unwrap();
    assert_eq!(e.document, before);
}
#[test]
fn unrelated_same_name_and_ambiguous_children_cannot_capture_existing_overrides() {
    let mut incoming = path_fixture();
    let NodeContent::Group { children } =
        &mut incoming.document.definitions[1].content.nodes[0].content
    else {
        panic!()
    };
    let mut duplicate = children[0].clone();
    duplicate.id = 25;
    duplicate.origin.x = 200.;
    children.push(duplicate);
    let c = incoming.copy_libraries(&[10, 20], "source").unwrap();
    let mut e = Editor::default();
    apply(&mut e, &c, "destination", Choice::UseNew);
    let id = definition(&e, mapped(&e, 20));
    // No ancestry: equal numeric child IDs alone do not identify a control.
    for l in &mut e.document.libraries {
        l.imports.clear();
    }
    let mut page = instance(501, id);
    if let NodeContent::Instance { overrides, .. } = &mut page.content {
        overrides.push(text_override(vec![24, 23], "Local first child"));
    }
    e.document.pages[0].content.nodes.push(page.clone());
    e.document.next_id = 502;
    let before = e.document.clone();
    let NodeContent::Group { children } =
        &mut incoming.document.definitions[1].content.nodes[0].content
    else {
        panic!()
    };
    for n in children {
        n.properties
            .insert("text".into(), Value::Text("Different content".into()));
    }
    let c = incoming.copy_libraries(&[10, 20], "unrelated").unwrap();
    let plan = e.plan_library_import(&c, "destination").unwrap();
    let kept = plan.prepare(&choices(&plan, Choice::KeepBoth)).unwrap();
    assert_eq!(kept.newly_orphaned_overrides(), 0);
    let prepared = plan.prepare(&choices(&plan, Choice::UseNew)).unwrap();
    assert!(prepared.newly_orphaned_overrides() > 0);
    e.apply_library_import(prepared, "destination").unwrap();
    let (texts, orphans) = resolved_texts(&e.document, &page);
    assert_eq!(texts, vec!["Different content", "Different content"]);
    assert_eq!(orphans, 1);
    assert_eq!(e.document.pages, before.pages);
    e.undo();
    assert_eq!(e.document, before);
}
#[test]
fn retained_child_id_tracks_reordering_while_new_children_avoid_deleted_paths() {
    let mut incoming = path_fixture();
    let NodeContent::Group { children } =
        &mut incoming.document.definitions[1].content.nodes[0].content
    else {
        panic!()
    };
    let mut extra = Node::control(25, "label", Point::new(300., 0.), Size::default());
    extra
        .properties
        .insert("text".into(), Value::Text("Label".into()));
    children.push(extra);
    let c = incoming.copy_libraries(&[10, 20], "source").unwrap();
    let mut e = Editor::default();
    apply(&mut e, &c, "destination", Choice::UseNew);
    let id = definition(&e, mapped(&e, 20));
    let mut page = instance(501, id);
    if let NodeContent::Instance { overrides, .. } = &mut page.content {
        overrides.push(text_override(vec![24, 23], "Button override"));
    }
    e.document.pages[0].content.nodes.push(page.clone());
    e.document.next_id = 502;
    let before = e.document.clone();
    let NodeContent::Group { children } =
        &mut incoming.document.definitions[1].content.nodes[0].content
    else {
        panic!()
    };
    children.reverse();
    let c = incoming.copy_libraries(&[10, 20], "source").unwrap();
    let plan = e.plan_library_import(&c, "destination").unwrap();
    let prepared = plan.prepare(&choices(&plan, Choice::UseNew)).unwrap();
    assert_eq!(prepared.newly_orphaned_overrides(), 0);
    e.apply_library_import(prepared, "destination").unwrap();
    assert_eq!(
        resolved_texts(&e.document, &page),
        (vec!["Label".into(), "Button override".into()], 0)
    );
    e.undo();
    assert_eq!(e.document, before);
    let NodeContent::Group { children } =
        &mut incoming.document.definitions[1].content.nodes[0].content
    else {
        panic!()
    };
    children[1] = Node::control(23, "image", Point::default(), Size::default());
    let c = incoming.copy_libraries(&[10, 20], "source").unwrap();
    let plan = e.plan_library_import(&c, "destination").unwrap();
    let prepared = plan.prepare(&choices(&plan, Choice::UseNew)).unwrap();
    assert!(prepared.newly_orphaned_overrides() > 0);
    e.apply_library_import(prepared, "destination").unwrap();
    assert_eq!(
        resolved_texts(&e.document, &page),
        (vec!["Label".into()], 1)
    );
    e.undo();
    assert_eq!(e.document, before);
}

fn orphan_import_fixture(path: NodePath, size: bool) -> (Editor, Editor, Clipboard) {
    let mut incoming = path_fixture();
    let NodeContent::Instance { overrides, .. } =
        &mut incoming.document.definitions[0].content.nodes[0].content
    else {
        panic!()
    };
    let mut edit = text_override(path, "Orphaned edit");
    if size {
        edit.patch.size = Some(Size::default());
    }
    overrides.push(edit);
    incoming.document.next_id = 1000;
    let c = incoming.copy_libraries(&[10, 20], "source").unwrap();
    let mut e = Editor::default();
    apply(&mut e, &c, "destination", Choice::UseNew);
    renumber_source_children(&mut e.document);
    e.document.next_id = 2000;
    (incoming, e, c)
}
#[test]
fn unmappable_orphan_reaches_safe_conflict_choices_without_mutation() {
    let (_, mut e, c) = orphan_import_fixture(vec![24, 123], false);
    let before = e.document.clone();
    let history = e.undo_name().map(str::to_owned);
    let plan = e.plan_library_import(&c, "destination").unwrap();
    assert!(
        plan.entries()
            .iter()
            .all(|entry| entry.action == Action::Conflict)
    );
    assert!(
        plan.entries()
            .iter()
            .find(|entry| entry.incoming == 20)
            .unwrap()
            .has_unmapped_overrides
    );
    assert!(
        !plan
            .entries()
            .iter()
            .find(|entry| entry.incoming == 10)
            .unwrap()
            .has_unmapped_overrides
    );
    assert!(
        plan.prepare(&choices(&plan, Choice::UseNew))
            .err()
            .unwrap()
            .contains("Keep Both")
    );
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), history.as_deref());
    let existing = plan.prepare(&choices(&plan, Choice::UseExisting)).unwrap();
    assert_eq!(existing.newly_orphaned_overrides(), 0);
    assert_eq!(
        e.apply_library_import(existing, "destination").unwrap(),
        LibraryImportCounts {
            added: 0,
            replaced: 0,
            reused: 2
        }
    );
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), history.as_deref());
    // The warning is relational: replacing only the dependency does not import
    // the referring orphan and can be a complete no-op.
    let mixed = plan
        .prepare(&BTreeMap::from([
            (10, Choice::UseExisting),
            (20, Choice::UseNew),
        ]))
        .unwrap();
    assert_eq!(
        e.apply_library_import(mixed, "destination").unwrap(),
        LibraryImportCounts {
            added: 0,
            replaced: 0,
            reused: 2
        }
    );
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), history.as_deref());
    let mixed = plan
        .prepare(&BTreeMap::from([
            (10, Choice::UseNew),
            (20, Choice::KeepBoth),
        ]))
        .unwrap();
    assert_eq!(
        e.apply_library_import(mixed, "destination").unwrap(),
        LibraryImportCounts {
            added: 1,
            replaced: 1,
            reused: 0
        }
    );
    let parent = instance(3000, definition(&e, mapped(&e, 10)));
    assert_eq!(
        resolved_texts(&e.document, &parent),
        (vec!["Nested source override".into()], 1)
    );
    let dependency = mapped(&e, 20);
    assert_eq!(
        e.document.libraries.iter().find(|l| l.id == dependency),
        before.libraries.iter().find(|l| l.id == dependency)
    );
    e.undo();
    assert_eq!(e.document, before);
    let both = plan.prepare(&choices(&plan, Choice::KeepBoth)).unwrap();
    assert_eq!(
        e.apply_library_import(both, "destination").unwrap(),
        LibraryImportCounts {
            added: 2,
            replaced: 0,
            reused: 0
        }
    );
    let added = e
        .document
        .libraries
        .iter()
        .find(|l| {
            !before.libraries.iter().any(|b| b.id == l.id)
                && l.imports
                    .iter()
                    .any(|i| i.library == 10 && i.project == "source")
        })
        .unwrap();
    let copied = instance(3000, added.sources[0].definition);
    assert_eq!(
        resolved_texts(&e.document, &copied),
        (vec!["Nested source override".into()], 1)
    );
    let d = e
        .document
        .definitions
        .iter()
        .find(|d| d.id == added.sources[0].definition)
        .unwrap();
    let NodeContent::Instance { overrides, .. } = &d.content.nodes[0].content else {
        panic!()
    };
    assert_eq!(overrides[1].path, vec![24, 123]);
    let after = e.document.clone();
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
}
#[test]
fn new_parent_with_orphan_can_copy_dependency_instead_of_rejecting_planning() {
    let (mut incoming, mut e, _) = orphan_import_fixture(vec![24, 123], false);
    incoming.document.libraries[0].id = 40;
    incoming.document.libraries[0].name = "New parent".into();
    incoming.document.default_library = Some(40);
    let c = incoming.copy_libraries(&[40], "source").unwrap();
    let before = e.document.clone();
    let plan = e.plan_library_import(&c, "destination").unwrap();
    assert_eq!(
        plan.entries()
            .iter()
            .find(|entry| entry.incoming == 40)
            .unwrap()
            .action,
        Action::Add
    );
    assert_eq!(
        plan.entries()
            .iter()
            .find(|entry| entry.incoming == 20)
            .unwrap()
            .action,
        Action::Conflict
    );
    assert!(
        plan.prepare(&BTreeMap::from([(20, Choice::UseExisting)]))
            .is_err()
    );
    assert!(
        plan.prepare(&BTreeMap::from([(20, Choice::UseNew)]))
            .is_err()
    );
    assert_eq!(e.document, before);
    let prepared = plan
        .prepare(&BTreeMap::from([(20, Choice::KeepBoth)]))
        .unwrap();
    assert_eq!(
        e.apply_library_import(prepared, "destination").unwrap(),
        LibraryImportCounts {
            added: 2,
            replaced: 0,
            reused: 0
        }
    );
    let n = instance(3000, definition(&e, mapped(&e, 40)));
    assert_eq!(
        resolved_texts(&e.document, &n),
        (vec!["Nested source override".into()], 1)
    );
    e.undo();
    assert_eq!(e.document, before);
}
#[test]
fn orphan_colliding_with_current_group_cannot_apply_an_invalid_size_patch() {
    let (_, mut e, c) = orphan_import_fixture(vec![124], true);
    let before = e.document.clone();
    let plan = e.plan_library_import(&c, "destination").unwrap();
    assert!(
        plan.entries()
            .iter()
            .any(|entry| entry.has_unmapped_overrides)
    );
    assert!(plan.prepare(&choices(&plan, Choice::UseNew)).is_err());
    let prepared = plan.prepare(&choices(&plan, Choice::UseExisting)).unwrap();
    e.apply_library_import(prepared, "destination").unwrap();
    assert_eq!(e.document, before);
    let prepared = plan.prepare(&choices(&plan, Choice::KeepBoth)).unwrap();
    e.apply_library_import(prepared, "destination").unwrap();
    e.document.validate().unwrap();
    e.undo();
    assert_eq!(e.document, before);
}

#[test]
fn new_child_allocation_does_not_revive_an_existing_orphan_override() {
    let mut incoming = path_fixture();
    let c = incoming.copy_libraries(&[10, 20], "source").unwrap();
    let mut e = Editor::default();
    apply(&mut e, &c, "destination", Choice::UseNew);
    let id = definition(&e, mapped(&e, 20));
    let mut page = instance(501, id);
    if let NodeContent::Instance { overrides, .. } = &mut page.content {
        overrides.push(text_override(vec![24, 502], "Must stay orphaned"));
    }
    e.document.pages[0].content.nodes.push(page.clone());
    e.document.next_id = 502;
    e.document.validate().unwrap();
    let before = e.document.clone();
    assert_eq!(resolved_texts(&e.document, &page).1, 1);
    let NodeContent::Group { children } =
        &mut incoming.document.definitions[1].content.nodes[0].content
    else {
        panic!()
    };
    children[0] = Node::control(23, "image", Point::default(), Size::default());
    let c = incoming.copy_libraries(&[10, 20], "source").unwrap();
    apply(&mut e, &c, "destination", Choice::UseNew);
    assert_eq!(resolved_texts(&e.document, &page), (vec![], 1));
    e.undo();
    assert_eq!(e.document, before);
}

#[test]
fn renumbered_children_compare_after_foreign_page_link_policy() {
    let mut incoming = path_fixture();
    let NodeContent::Group { children } =
        &mut incoming.document.definitions[1].content.nodes[0].content
    else {
        panic!()
    };
    children[0]
        .properties
        .insert("link".into(), Value::Link(Link::Page(1)));
    let mut second = children[0].clone();
    second.id = 25;
    second.origin.x = 200.;
    children.push(second);
    let c = incoming.copy_libraries(&[10, 20], "source").unwrap();
    let mut e = Editor::default();
    apply(&mut e, &c, "destination", Choice::UseNew);
    renumber_source_children(&mut e.document);
    let before = e.document.clone();
    assert_eq!(
        apply(&mut e, &c, "destination", Choice::UseNew),
        LibraryImportCounts {
            added: 0,
            replaced: 0,
            reused: 2
        }
    );
    assert_eq!(e.document, before);
}

#[test]
fn unchanged_shared_ids_do_not_let_incoming_orphan_capture_extra_destination_child() {
    let (incoming, mut e, _) = orphan_import_fixture(vec![24, 999], false);
    // Restore shared child IDs, but retain an extra destination-only child at
    // the incoming orphan path. The copy has no correspondence for that child.
    fn restore(nodes: &mut [Node]) {
        for n in nodes {
            n.id -= 100;
            match &mut n.content {
                NodeContent::Group { children } => restore(children),
                NodeContent::Instance { overrides, .. } => {
                    for o in overrides {
                        for id in &mut o.path {
                            *id -= 100;
                        }
                    }
                }
                _ => (),
            }
        }
    }
    for d in &mut e.document.definitions {
        restore(&mut d.content.nodes);
    }
    let child = definition(&e, mapped(&e, 20));
    let NodeContent::Group { children } = &mut e
        .document
        .definitions
        .iter_mut()
        .find(|d| d.id == child)
        .unwrap()
        .content
        .nodes[0]
        .content
    else {
        panic!()
    };
    let mut extra = Node::control(999, "label", Point::new(300., 0.), Size::default());
    extra
        .properties
        .insert("text".into(), Value::Text("Destination only".into()));
    children.push(extra);
    // Use a new parent so no existing local edit can account for a text change.
    let mut incoming = incoming;
    incoming.document.libraries[0].id = 40;
    incoming.document.libraries[0].name = "Extra parent".into();
    incoming.document.default_library = Some(40);
    let c = incoming.copy_libraries(&[40], "source").unwrap();
    let before = e.document.clone();
    let plan = e.plan_library_import(&c, "destination").unwrap();
    assert!(
        !plan
            .entries()
            .iter()
            .find(|e| e.incoming == 20)
            .unwrap()
            .has_unmapped_overrides
    );
    assert!(
        plan.prepare(&BTreeMap::from([(20, Choice::UseExisting)]))
            .is_err()
    );
    assert_eq!(e.document, before);
    let replacement = plan
        .prepare(&BTreeMap::from([(20, Choice::UseNew)]))
        .unwrap();
    assert!(replacement.newly_orphaned_overrides() > 0);
    e.apply_library_import(replacement, "destination").unwrap();
    let parent = instance(3000, definition(&e, mapped(&e, 40)));
    assert_eq!(
        resolved_texts(&e.document, &parent),
        (vec!["Nested source override".into()], 1)
    );
    e.undo();
    assert_eq!(e.document, before);
    let prepared = plan
        .prepare(&BTreeMap::from([(20, Choice::KeepBoth)]))
        .unwrap();
    e.apply_library_import(prepared, "destination").unwrap();
    let parent = instance(3000, definition(&e, mapped(&e, 40)));
    assert_eq!(
        resolved_texts(&e.document, &parent),
        (vec!["Nested source override".into()], 1)
    );
    e.undo();
    assert_eq!(e.document, before);
}

#[test]
fn noncapturing_orphan_translates_known_prefix_and_repeats_without_new_history() {
    let (_, mut e, c) = orphan_import_fixture(vec![24, 999], false);
    let before = e.document.clone();
    let plan = e.plan_library_import(&c, "destination").unwrap();
    assert!(
        plan.entries()
            .iter()
            .all(|entry| !entry.has_unmapped_overrides)
    );
    let prepared = plan.prepare(&choices(&plan, Choice::UseNew)).unwrap();
    e.apply_library_import(prepared, "destination").unwrap();
    let parent = instance(3000, definition(&e, mapped(&e, 10)));
    assert_eq!(
        resolved_texts(&e.document, &parent),
        (vec!["Nested source override".into()], 1)
    );
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    let mut reopened = Editor::new(decode(&after.encode().unwrap()).unwrap());
    let plan = reopened.plan_library_import(&c, "destination").unwrap();
    assert!(
        plan.entries()
            .iter()
            .all(|entry| entry.action == Action::Reuse)
    );
    reopened
        .apply_library_import(plan.prepare(&BTreeMap::new()).unwrap(), "destination")
        .unwrap();
    assert_eq!(reopened.document, after);
    assert!(reopened.undo_name().is_none());
}

#[test]
fn distinct_orphans_remain_distinct_through_conflict_planning() {
    let (mut incoming, mut e, _) = orphan_import_fixture(vec![24, 123], false);
    let NodeContent::Instance { overrides, .. } =
        &mut incoming.document.definitions[0].content.nodes[0].content
    else {
        panic!()
    };
    overrides.push(text_override(vec![24, 999], "Separate orphan"));
    let c = incoming.copy_libraries(&[10, 20], "source").unwrap();
    let before = e.document.clone();
    let plan = e.plan_library_import(&c, "destination").unwrap();
    assert!(plan.prepare(&choices(&plan, Choice::UseNew)).is_err());
    assert_eq!(e.document, before);
    let prepared = plan.prepare(&choices(&plan, Choice::KeepBoth)).unwrap();
    e.apply_library_import(prepared, "destination").unwrap();
    let parent_library = e
        .document
        .libraries
        .iter()
        .find(|l| {
            !before.libraries.iter().any(|old| old.id == l.id)
                && l.imports
                    .iter()
                    .any(|i| i.library == 10 && i.project == "source")
        })
        .unwrap();
    let parent = instance(3000, definition(&e, parent_library.id));
    assert_eq!(
        resolved_texts(&e.document, &parent),
        (vec!["Nested source override".into()], 2)
    );
    let after = e.document.clone();
    assert_eq!(decode(&after.encode().unwrap()).unwrap(), after);
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
}
