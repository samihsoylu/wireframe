// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::collections::BTreeMap;
use wireframe::{
    document::*,
    model as old,
    storage::{DocumentStore, JsonStore, ProjectStore},
};
fn leaf(id: Id, x: f32, y: f32) -> Node {
    Node::control(id, "label", Point::new(x, y), Size::default())
}
fn instance(id: Id, definition: Id, overrides: Vec<Override>) -> Node {
    Node {
        content: NodeContent::Instance {
            definition,
            overrides,
        },
        ..leaf(id, 0., 0.)
    }
}
fn text_patch(path: &[Id], text: &str) -> Override {
    Override {
        path: path.into(),
        patch: Patch {
            properties: BTreeMap::from([("text".into(), Value::Text(text.into()))]),
            ..Patch::default()
        },
    }
}
fn component_project() -> Project {
    let mut p = Project::default();
    let mut a = leaf(2, 10., 20.);
    a.properties
        .insert("text".into(), Value::Text("shared".into()));
    p.definitions = vec![
        Definition {
            id: 10,
            name: "Inner".into(),
            content: Content {
                nodes: vec![a],
                ..Content::default()
            },
        },
        Definition {
            id: 11,
            name: "Outer".into(),
            content: Content {
                nodes: vec![instance(3, 10, vec![text_patch(&[2], "inner override")])],
                ..Content::default()
            },
        },
    ];
    p.pages[0]
        .content
        .nodes
        .push(instance(4, 11, vec![text_patch(&[3, 2], "outer override")]));
    p.next_id = 20;
    p
}
fn only_text(root: Node) -> String {
    let c = Content {
        nodes: vec![root],
        ..Content::default()
    };
    match &c.leaves().unwrap()[0].node.properties["text"] {
        Value::Text(s) => s.clone(),
        _ => panic!(),
    }
}
#[test]
fn migration_preserves_every_legacy_control_and_interleaved_group_layering() {
    let mut old = old::Document {
        title: "Existing project".into(),
        ..old::Document::default()
    };
    old.pages[0].notes = "Keep notes".into();
    for kind in old::Kind::ALL {
        let id = old.allocate_id();
        let mut c = old::Control::new(id, kind, id as f32 * 2., id as f32 * 3.);
        c.fill = [1, 20, 250];
        c.bold = true;
        c.checked = true;
        c.link = Some(1);
        old.pages[0].controls.push(c);
    }
    let group = old.allocate_id();
    old.pages[0].controls[0].group = Some(group);
    old.pages[0].controls[2].group = Some(group);
    old.pages[0].controls[2].locked = true;
    let bytes = serde_json::to_vec(&old).unwrap();
    let new = decode(&bytes).unwrap();
    assert_eq!(new.title, old.title);
    assert_eq!(new.pages[0].notes, old.pages[0].notes);
    assert_eq!(new.pages[0].content.paint_order.len(), 24);
    let leaves = new.pages[0].content.leaves().unwrap();
    for (a, b) in leaves.iter().zip(&old.pages[0].controls) {
        assert_eq!(a.node.id, b.id);
        assert_eq!(a.world_origin, Point::new(b.bounds.x, b.bounds.y));
        assert_eq!(a.locked, b.locked);
        assert_eq!(a.node.properties["text"], Value::Text(b.text.clone()));
        assert_eq!(
            a.node.properties["font_size"],
            Value::Number(b.font_size as f64)
        );
        assert_eq!(a.node.properties["fill"], Value::Color([1, 20, 250, 255]));
        assert_eq!(a.node.properties["link"], Value::Link(Link::Page(1)));
        assert_eq!(a.node.properties["bold"], Value::Bool(true));
        assert_eq!(a.node.properties["checked"], Value::Bool(true));
        match a.node.content {
            NodeContent::Control { size, .. } => {
                assert_eq!(size, Size::fixed(b.bounds.w, b.bounds.h))
            }
            _ => panic!(),
        }
    }
    assert_eq!(decode(&new.encode().unwrap()).unwrap(), new);
}
#[test]
fn nested_move_and_ungroup_preserve_descendant_world_positions() {
    let mut outer = Node::group(
        2,
        Point::new(50., 60.),
        vec![Node::group(3, Point::new(10., 20.), vec![leaf(4, 5., 8.)])],
    );
    outer.origin = outer.origin.translated(Point::new(100., 200.));
    let before = Content {
        nodes: vec![outer.clone()],
        ..Content::default()
    };
    let expected = before.leaves().unwrap()[0].world_origin;
    assert_eq!(expected, Point::new(165., 288.));
    let after = Content {
        nodes: outer.ungroup().unwrap(),
        ..Content::default()
    };
    assert_eq!(after.leaves().unwrap()[0].world_origin, expected);
    let inner = after.nodes.into_iter().next().unwrap().ungroup().unwrap();
    assert_eq!(inner[0].origin, expected);
}
#[test]
fn nested_instance_overrides_reset_propagation_and_detach_are_independent() {
    let mut p = component_project();
    p.validate().unwrap();
    let original = p.pages[0].content.nodes[0].clone();
    let resolved = p.resolve(&original).unwrap();
    assert!(resolved.orphan_overrides.is_empty());
    assert_eq!(only_text(resolved.root.clone()), "outer override");
    let mut reset = original.clone();
    if let NodeContent::Instance { overrides, .. } = &mut reset.content {
        overrides.clear();
    }
    assert_eq!(only_text(p.resolve(&reset).unwrap().root), "inner override");
    if let NodeContent::Instance { overrides, .. } = &mut p.definitions[1].content.nodes[0].content
    {
        overrides.clear();
    }
    p.definitions[0].content.nodes[0]
        .properties
        .insert("text".into(), Value::Text("edited source".into()));
    assert_eq!(only_text(p.resolve(&reset).unwrap().root), "edited source");
    assert_eq!(
        only_text(p.resolve(&original).unwrap().root),
        "outer override"
    );
    assert_eq!(only_text(resolved.root), "outer override"); // detached tree survives source edits
}
#[test]
fn orphan_overrides_are_reported_and_do_not_overwrite_a_different_path() {
    let mut p = component_project();
    if let NodeContent::Instance { overrides, .. } = &mut p.pages[0].content.nodes[0].content {
        overrides.push(text_patch(&[9, 2], "wrong"));
    }
    let resolved = p.resolve(&p.pages[0].content.nodes[0]).unwrap();
    assert_eq!(resolved.orphan_overrides, vec![(vec![4], vec![9, 2])]);
    assert_eq!(only_text(resolved.root), "outer override");
    assert_eq!(decode(&p.encode().unwrap()).unwrap(), p);
}
#[test]
fn cycles_missing_references_and_expansion_growth_fail_without_mutation() {
    let mut p = component_project();
    p.definitions[0].content.nodes.push(instance(5, 11, vec![]));
    let before = p.clone();
    assert!(p.validate().unwrap_err().contains("Cyclic"));
    assert!(p.resolve(&p.pages[0].content.nodes[0]).is_err());
    assert_eq!(p, before);
    p.definitions[0].content.nodes.pop();
    p.definitions.remove(0);
    assert!(p.validate().unwrap_err().contains("Missing component"));
    let mut p = Project::default();
    p.definitions.push(Definition {
        id: 10,
        name: "Base".into(),
        content: Content {
            nodes: vec![leaf(2, 0., 0.)],
            ..Content::default()
        },
    });
    for id in 11..27 {
        p.definitions.push(Definition {
            id,
            name: "Double".into(),
            content: Content {
                nodes: vec![instance(2, id - 1, vec![]), instance(3, id - 1, vec![])],
                ..Content::default()
            },
        });
    }
    p.next_id = 100;
    p.validate().unwrap();
    assert!(
        p.resolve(&instance(4, 26, vec![]))
            .unwrap_err()
            .contains("limit")
    );
}
#[test]
fn assets_alternates_and_sparse_context_values_round_trip() {
    let mut p = component_project();
    p.assets.push(Asset {
        id: 12,
        name: "sample".into(),
        media_type: "image/png".into(),
        bytes: vec![0, 1, 2, 255],
    });
    let mut image = Node::control(
        5,
        "image",
        Point::new(50., 60.),
        Size {
            width: Dimension::Auto,
            height: Dimension::Fixed(64.),
        },
    );
    image.crop = Some(Crop {
        x: 0.,
        y: 0.5,
        width: 0.8,
        height: 0.5,
    });
    image.properties.insert("image".into(), Value::Asset(12));
    image
        .properties
        .insert("rotation".into(), Value::Number(270.));
    p.pages[0].content.nodes.push(image);
    let alternate_content = p.pages[0].content.clone();
    p.pages[0].alternates.push(Alternate {
        id: 13,
        name: "Alternate".into(),
        content: alternate_content,
    });
    p.pages[0].alternates[0].content.nodes[1].origin.x = 123.;
    assert_eq!(p.pages[0].content.nodes[1].origin.x, 50.);
    assert_eq!(decode(&p.encode().unwrap()).unwrap(), p);
    let mut node = leaf(6, 0., 0.);
    let spec = wireframe::catalog::get("label").unwrap();
    let mut context = wireframe::catalog::Context {
        font_size: p.settings.font_size,
        ..Default::default()
    };
    assert_eq!(
        spec.effective(&node, &context).unwrap()["font_size"],
        Value::Number(13.)
    );
    node.properties
        .insert("font_size".into(), Value::Number(24.));
    node.properties.insert("link".into(), Value::Null);
    assert_eq!(
        spec.effective(&node, &context).unwrap()["link"],
        Value::Null
    );
    assert_eq!(
        spec.effective(&node, &context).unwrap()["font_size"],
        Value::Number(24.)
    );
    node.properties.remove("font_size");
    context.font_size = 18.;
    assert_eq!(
        spec.effective(&node, &context).unwrap()["font_size"],
        Value::Number(18.)
    );
}
#[test]
fn storage_migrates_without_overwriting_source_and_failed_save_preserves_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.wireframe");
    let old = old::Document::default();
    JsonStore.save(&path, &old).unwrap();
    let before = std::fs::read(&path).unwrap();
    let mut p = ProjectStore.load(&path).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before);
    ProjectStore.save(&path, &p).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(ProjectStore.load(&path).unwrap(), p);
    p.pages[0].content.nodes.push(leaf(2, f32::NAN, 0.));
    p.next_id = 3;
    assert!(ProjectStore.save(&path, &p).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert!(
        decode(br#"{"format_version":999}"#)
            .unwrap_err()
            .contains("Unsupported")
    );
    assert!(decode(b"broken").is_err());
}
#[test]
fn invalid_hierarchies_sizes_crop_and_order_are_rejected() {
    let mut p = Project {
        next_id: 5,
        ..Project::default()
    };
    p.pages[0].content.nodes = vec![leaf(2, 0., 0.), leaf(2, 10., 10.)];
    assert!(p.validate().is_err());
    p.pages[0].content.nodes.pop();
    p.pages[0].content.paint_order = vec![vec![99]];
    assert!(p.validate().is_err());
    p.pages[0].content.paint_order.clear();
    p.pages[0].content.nodes[0].crop = Some(Crop {
        x: 0.8,
        y: 0.,
        width: 0.5,
        height: 1.,
    });
    assert!(p.validate().is_err());
    p.pages[0].content.nodes[0].crop = None;
    p.pages[0].parent = Some(1);
    assert!(p.validate().is_err());
    p.pages[0].parent = None;
    p.pages[0].content.nodes[0].content = NodeContent::Control {
        kind: "label".into(),
        size: Size::fixed(-1., 10.),
    };
    assert!(p.validate().is_err());
}

#[test]
fn depth_limit_matches_file_round_trip_and_does_not_depend_on_definition_order() {
    let mut node = leaf(30, 0., 0.);
    for id in (2..2 + MAX_DEPTH as u64).rev() {
        node = Node::group(id, Point::default(), vec![node]);
    }
    let mut p = Project {
        next_id: 100,
        ..Project::default()
    };
    p.pages[0].content.nodes = vec![node];
    p.validate().unwrap();
    let bytes = p.encode().unwrap();
    assert_eq!(decode(&bytes).unwrap(), p);
    let node = p.pages[0].content.nodes.remove(0);
    p.pages[0]
        .content
        .nodes
        .push(Node::group(50, Point::default(), vec![node]));
    assert!(p.validate().is_err());
    let mut p = Project::default();
    for id in 10..10 + MAX_DEPTH as u64 + 2 {
        let nodes = if id == 10 {
            vec![leaf(2, 0., 0.)]
        } else {
            vec![instance(2, id - 1, vec![])]
        };
        p.definitions.push(Definition {
            id,
            name: "Depth".into(),
            content: Content {
                nodes,
                ..Content::default()
            },
        });
    }
    p.next_id = 100;
    assert!(p.validate().is_err());
    p.definitions.reverse();
    assert!(p.validate().is_err());
}

#[test]
fn a_page_cannot_multiply_the_expansion_budget_across_instances() {
    let mut p = Project {
        next_id: 100,
        ..Project::default()
    };
    p.definitions.push(Definition {
        id: 10,
        name: "Base".into(),
        content: Content {
            nodes: vec![leaf(2, 0., 0.)],
            ..Content::default()
        },
    });
    for id in 11..24 {
        p.definitions.push(Definition {
            id,
            name: "Pair".into(),
            content: Content {
                nodes: vec![instance(2, id - 1, vec![]), instance(3, id - 1, vec![])],
                ..Content::default()
            },
        });
    }
    let nodes: Vec<_> = (30..34).map(|id| instance(id, 23, vec![])).collect();
    assert!(p.resolve(&nodes[0]).is_ok());
    assert!(p.resolve_all(&nodes).unwrap_err().contains("limit"));
}
