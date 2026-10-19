// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{catalog, document::*, render::Scene, workspace::Editor};

fn fixture(overlay: Node) -> Editor {
    let mut card = Node::group(
        10,
        Point::new(100., 100.),
        vec![Node::control(
            11,
            "button",
            Point::default(),
            Size::fixed(240., 140.),
        )],
    );
    card.properties
        .insert("link".into(), Value::Link(Link::Page(20)));
    let mut project = Project {
        next_id: 100,
        ..Project::default()
    };
    project.pages[0].content.nodes = vec![card, overlay];
    for (id, name) in [(20, "Contact"), (22, "Order")] {
        let mut page = project.pages[0].clone();
        page.id = id;
        page.name = name.into();
        page.content = Content::default();
        project.pages.push(page);
    }
    Editor::new(project)
}

fn overlay(kind: &str) -> Node {
    let mut node = catalog::get(kind)
        .unwrap()
        .create(30, Point::new(130., 135.));
    let NodeContent::Control { size, .. } = &mut node.content else {
        unreachable!()
    };
    *size = Size::fixed(180., 60.);
    node
}

fn area(scene: &Scene, target: &str) -> f32 {
    scene
        .links
        .iter()
        .filter(|r| r.target == target)
        .map(|r| r.bounds.w * r.bounds.h)
        .sum()
}

#[test]
fn text_overlays_preserve_card_links_but_fields_and_placeholder_strokes_block() {
    for kind in [
        "label",
        "heading",
        "subtitle",
        "paragraph",
        "link",
        "tag_cloud",
        "breadcrumbs",
        "link_bar",
        "icon_label",
    ] {
        let mut node = overlay(kind);
        node.properties
            .insert("text".into(), Value::Text("Plain overlay".into()));
        let e = fixture(node);
        let before = e.document.clone();
        let scene = e.rendered_scene(e.page).unwrap();
        assert_eq!(area(&scene, "page:20"), 240. * 140., "{kind}");
        assert_eq!(e.document, before);
    }
    for kind in [
        "text_area",
        "text_input",
        "field_set",
        "text_line",
        "text_block",
        "rectangle",
    ] {
        let e = fixture(overlay(kind));
        let scene = e.rendered_scene(e.page).unwrap();
        assert!(area(&scene, "page:20") < 240. * 140., "{kind}");
        assert_eq!(scene.hit_link(110., 110.), Some("page:20"));
    }
}

#[test]
fn markup_is_local_to_the_covering_control_not_inherited_from_its_group() {
    for kind in ["sticky_note", "rectangle"] {
        for marked in [false, true] {
            for parent_marked in [false, true] {
                let mut child = overlay(kind);
                child.origin = Point::default();
                child.markup = marked;
                let mut group = Node::group(31, Point::new(130., 135.), vec![child]);
                group.markup = parent_marked;
                let e = fixture(group);
                let scene = e.rendered_scene(e.page).unwrap();
                assert_eq!(
                    scene.hit_link(150., 150.),
                    marked.then_some("page:20"),
                    "{kind}, child={marked}, parent={parent_marked}"
                );
                let reopened = Editor::new(decode(&e.document.encode().unwrap()).unwrap());
                assert_eq!(
                    reopened
                        .rendered_scene(reopened.page)
                        .unwrap()
                        .hit_link(150., 150.),
                    scene.hit_link(150., 150.)
                );
            }
        }
    }
}

#[test]
fn overlay_links_replace_only_their_own_regions_without_duplicate_export_targets() {
    for kind in ["label", "sticky_note"] {
        let mut node = overlay(kind);
        node.markup = true;
        node.properties
            .insert("text".into(), Value::Text("Order".into()));
        node.properties
            .insert("link".into(), Value::Link(Link::Page(22)));
        let e = fixture(node);
        let scene = e.rendered_scene(e.page).unwrap();
        assert_eq!(scene.hit_link(150., 145.), Some("page:22"));
        assert_eq!(scene.hit_link(110., 110.), Some("page:20"));
        assert_eq!(
            area(&scene, "page:20") + area(&scene, "page:22"),
            240. * 140.
        );
        for (i, a) in scene.links.iter().enumerate() {
            for b in &scene.links[i + 1..] {
                let w = (a.bounds.x + a.bounds.w).min(b.bounds.x + b.bounds.w)
                    - a.bounds.x.max(b.bounds.x);
                let h = (a.bounds.y + a.bounds.h).min(b.bounds.y + b.bounds.h)
                    - a.bounds.y.max(b.bounds.y);
                assert!(w <= 0. || h <= 0., "overlapping exported link regions");
            }
        }
    }
}

#[test]
fn inline_target_leaves_the_rest_of_a_cropped_card_clickable() {
    let mut node = overlay("paragraph");
    node.properties.insert(
        "text".into(),
        Value::Text("[Order](page:22) and ordinary text".into()),
    );
    let mut e = fixture(node);
    e.document.pages[0].content.nodes[0].crop = Some(Crop {
        x: 0.1,
        y: 0.,
        width: 0.8,
        height: 1.,
    });
    let before = e.document.clone();
    let scene = e.rendered_scene(e.page).unwrap();
    let order = scene.links.iter().find(|l| l.target == "page:22").unwrap();
    assert_eq!(
        scene.hit_link(
            order.bounds.x + order.bounds.w / 2.,
            order.bounds.y + order.bounds.h / 2.
        ),
        Some("page:22")
    );
    assert_eq!(scene.hit_link(290., 150.), Some("page:20"));
    assert_eq!(scene.hit_link(110., 150.), None);
    assert!((area(&scene, "page:20") + area(&scene, "page:22") - 192. * 140.).abs() < 0.02);
    assert_eq!(e.document, before);
    assert!(e.undo_name().is_none());
}
