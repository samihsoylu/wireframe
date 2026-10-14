// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, model::Kind, workspace::Editor};

fn specimen() -> (Editor, Id, Id) {
    let mut e = Editor::default();
    e.add(Kind::Button, 50., 60.);
    let instance = e.nodes()[0].id;
    let definition = e.create_component("Shared button".into()).unwrap();
    assert!(e.enter(instance));
    let child = e.nodes()[0].id;
    e.set_control_property(child, "text", Value::Text("Local value".into()))
        .unwrap();
    e.leave();
    e.insert_component(definition, 300., 60.).unwrap();
    (e, definition, instance)
}
#[test]
fn source_deletion_preserves_instances_overrides_history_storage_and_shared_drawing() {
    let (mut e, definition, instance) = specimen();
    let canonical = e.document.pages[0].content.clone();
    let before = e.rendered_scene(1).unwrap().svg();
    e.edit_definition(definition).unwrap();
    e.delete_definition(definition).unwrap();
    assert!(e.definition.is_none());
    assert!(e.document.definitions.is_empty());
    assert_eq!(e.document.pages[0].content, canonical);
    assert_eq!(e.undo_name(), Some("Delete component"));
    let layers = e.layers(1).unwrap();
    assert_eq!(layers.len(), 2);
    for l in layers {
        assert_eq!((l.control.bounds.w, l.control.bounds.h), (150., 150.));
    }
    let missing = e.rendered_scene(1).unwrap().svg();
    assert_eq!(missing.matches("Component not found").count(), 2);
    assert!(e.rendered_scene(1).unwrap().png().unwrap().len() > 100);
    let resolved = e
        .document
        .resolve(&e.document.pages[0].content.nodes[0])
        .unwrap();
    assert_eq!(resolved.orphan_overrides.len(), 1);
    assert!(e.missing_component(instance));
    assert!(!e.enter(instance));
    let saved = e.document.clone();
    assert!(e.detach_component(instance).is_err());
    assert!(e.reset_instance(instance).is_err());
    assert!(e.edit_definition(definition).is_err());
    assert_eq!(e.document, saved);
    let reopened = Editor::new(decode(&e.document.encode().unwrap()).unwrap());
    assert_eq!(reopened.document, saved);
    assert_eq!(reopened.rendered_scene(1).unwrap().svg(), missing);
    e.undo();
    assert_eq!(e.definition, Some(definition));
    assert_eq!(e.rendered_scene(1).unwrap().svg(), before);
    e.redo();
    assert_eq!(e.document, saved);
    // Moving preserves the orphaned local data; attempted resizing cannot replace the fallback.
    let mut c = e.current().controls[0].clone();
    c.bounds.x += 20.;
    c.bounds.w = 300.;
    e.begin("Move missing component");
    e.update_control(c);
    e.commit();
    let c = &e.current().controls[0];
    assert_eq!(
        (c.bounds.x, c.bounds.y, c.bounds.w, c.bounds.h),
        (70., 60., 150., 150.)
    );
    assert_eq!(
        e.document.pages[0].content.nodes[0].content,
        canonical.nodes[0].content
    );
}
#[test]
fn nested_missing_sources_alternates_crops_and_context_recovery_retain_canonical_data() {
    let (mut e, inner, first) = specimen();
    e.select_all();
    e.group();
    let outer = e.create_component("Outer".into()).unwrap();
    let root = e.nodes()[0].id;
    let alt_id = e.document.allocate_id().unwrap();
    let content = e.document.pages[0].content.clone();
    e.document.pages[0].alternates.push(Alternate {
        id: alt_id,
        name: "Variant".into(),
        content,
    });
    let original = e.document.clone();
    assert!(e.enter(root));
    assert!(e.enter(first));
    e.delete_definition(inner).unwrap();
    assert_eq!(e.context, [root]);
    assert_eq!(e.document.pages, original.pages);
    assert_eq!(e.document.definitions[0].id, outer);
    assert_eq!(e.layers(1).unwrap().len(), 2);
    assert!(!e.enter(first));
    e.leave();
    assert!(e.detach_component(root).is_err());
    // Missing instances discard only their derived crop, never the stored crop or outer clipping.
    e.edit_definition(outer).unwrap();
    let source = &mut e.document.definitions[0].content.nodes[0];
    source.crop = Some(Crop {
        x: 0.,
        y: 0.,
        width: 0.5,
        height: 0.5,
    });
    let crop = source.crop;
    let resolved = e
        .document
        .resolve(&e.document.definitions[0].content.nodes[0])
        .unwrap();
    assert!(resolved.root.crop.is_none());
    assert_eq!(e.document.definitions[0].content.nodes[0].crop, crop);
    e.document.validate().unwrap();
    assert_eq!(decode(&e.document.encode().unwrap()).unwrap(), e.document);
}
#[test]
fn missing_reference_ids_are_reserved_and_clipboard_cannot_rebind_them() {
    let (mut source, definition, _) = specimen();
    source.delete_definition(definition).unwrap();
    source.select_all();
    let copy = source.copy();
    let mut destination = Editor::default();
    destination.add(Kind::Label, 0., 0.);
    let collision = destination.create_component("Unrelated".into()).unwrap();
    assert_eq!(collision, definition);
    destination.paste(&copy, 20.);
    let pasted: Vec<_> = destination
        .nodes()
        .iter()
        .skip(1)
        .map(|n| match n.content {
            NodeContent::Instance { definition, .. } => definition,
            _ => panic!(),
        })
        .collect();
    assert_eq!(pasted.len(), 2);
    assert_eq!(pasted[0], pasted[1]);
    assert_ne!(pasted[0], collision);
    assert!(pasted[0] < destination.document.next_id);
    assert_eq!(
        destination
            .rendered_scene(1)
            .unwrap()
            .svg()
            .matches("Component not found")
            .count(),
        2
    );
    let mut p = source.document.clone();
    if let NodeContent::Instance { definition, .. } = &mut p.pages[0].content.nodes[0].content {
        *definition = 1000;
    }
    assert!(p.validate().unwrap_err().contains("next ID"));
    p.next_id = 1001;
    p.validate().unwrap();
    assert_eq!(p.allocate_id().unwrap(), 1001);
    for bad in [0, 1, u64::MAX] {
        if let NodeContent::Instance { definition, .. } = &mut p.pages[0].content.nodes[0].content {
            *definition = bad;
        }
        assert!(p.validate().is_err());
    }
    // A copied available parent with a missing child must clone its dependencies as a unit.
    let (mut source, inner, _) = specimen();
    source.select_all();
    source.group();
    let outer = source.create_component("Outer".into()).unwrap();
    source.delete_definition(inner).unwrap();
    source.select_all();
    let copy = source.copy();
    source.paste(&copy, 20.);
    assert_ne!(
        source.instance_definition(*source.selection.iter().next().unwrap()),
        Some(outer)
    );
    assert_eq!(
        source
            .rendered_scene(1)
            .unwrap()
            .svg()
            .matches("Component not found")
            .count(),
        4
    );
    source.document.validate().unwrap();
}

#[test]
fn ungroup_without_a_selected_group_is_a_true_no_op() {
    let (mut e, definition, _) = specimen();
    e.delete_definition(definition).unwrap();
    let mut e = Editor::new(e.document);
    e.ungroup();
    assert!(!e.dirty());
    assert!(e.undo_name().is_none());
    e.select_all();
    e.ungroup();
    assert!(!e.dirty());
    assert!(e.undo_name().is_none());
    assert!(e.document.pages[0].content.paint_order.is_empty());
}
