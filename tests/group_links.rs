// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{document::*, workspace::Editor};

fn fixture() -> Editor {
    let first = Node::control(4, "button", Point::default(), Size::fixed(100., 40.));
    let mut second = Node::control(5, "button", Point::new(100., 80.), Size::fixed(100., 40.));
    second
        .properties
        .insert("link".into(), Value::Link(Link::Page(22)));
    let inner = Node::group(3, Point::default(), vec![first, second]);
    let mut root = Node::group(6, Point::new(100., 100.), vec![inner]);
    root.properties
        .insert("link".into(), Value::Link(Link::Page(20)));
    let mut project = Project {
        next_id: 100,
        ..Project::default()
    };
    project.pages[0].content.nodes = vec![root];
    for (id, name) in [(20, "Contact"), (22, "Order")] {
        let mut page = project.pages[0].clone();
        page.id = id;
        page.name = name.into();
        page.content = Content::default();
        project.pages.push(page);
    }
    Editor::new(project)
}
#[test]
fn group_link_owns_child_and_blank_regions_and_clearing_restores_child() {
    let mut e = fixture();
    let before = e.document.clone();
    let scene = e.rendered_scene(e.page).unwrap();
    assert_eq!(scene.hit_link(250., 200.), Some("page:20"));
    assert_eq!(scene.hit_link(150., 160.), Some("page:20"));
    assert_eq!(scene.hit_link(99., 160.), None);
    let mut c = e.current().controls[0].clone();
    assert_eq!(c.link, Some(20));
    c.link = None;
    e.begin("Clear group link");
    e.update_control(c);
    e.commit();
    assert_eq!(e.current().controls[0].link, None);
    let scene = e.rendered_scene(e.page).unwrap();
    assert_eq!(scene.hit_link(250., 200.), Some("page:22"));
    assert_eq!(scene.hit_link(150., 160.), None);
    let cleared = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, cleared);
    let reopened = Editor::new(decode(&e.document.encode().unwrap()).unwrap());
    assert_eq!(
        reopened.rendered_scene(reopened.page).unwrap().svg(),
        scene.svg()
    );
}

#[test]
fn nested_group_regions_respect_masks_parent_priority_and_later_occlusion() {
    let mut e = fixture();
    let root = &mut e.document.pages[0].content.nodes[0];
    root.crop = Some(Crop {
        x: 0.25,
        y: 0.,
        width: 0.5,
        height: 1.,
    });
    let NodeContent::Group { children } = &mut root.content else {
        panic!()
    };
    children[0]
        .properties
        .insert("link".into(), Value::Link(Link::Back));
    e.document.pages[0].content.nodes.push(Node::control(
        7,
        "button",
        Point::new(190., 150.),
        Size::fixed(20., 40.),
    ));
    let s = e.rendered_scene(e.page).unwrap();
    assert_eq!(s.hit_link(149., 160.), None);
    assert_eq!(s.hit_link(151., 160.), Some("page:20"));
    assert_eq!(s.hit_link(200., 160.), None);
    assert_eq!(s.hit_link(249., 200.), Some("page:20"));
    assert_eq!(s.hit_link(251., 200.), None);
    assert!(
        s.links.iter().all(|l| l.target == "page:20"
            && l.bounds.x >= 150.
            && l.bounds.x + l.bounds.w <= 250.)
    );
    let canonical = s.svg();
    e.enter(6);
    e.enter(3);
    assert_eq!(e.rendered_scene(e.page).unwrap().svg(), canonical);
    e.leave();
    e.leave();
    let mut c = e.current().controls[0].clone();
    c.link = None;
    e.begin("Clear parent");
    e.update_control(c);
    e.commit();
    assert_eq!(
        e.rendered_scene(e.page).unwrap().hit_link(160., 160.),
        Some("back:")
    );
    assert_eq!(e.rendered_scene(e.page).unwrap().hit_link(200., 160.), None);
}

#[test]
fn component_group_links_are_local_resettable_locked_and_persistent() {
    let mut e = fixture();
    let root = &mut e.document.pages[0].content.nodes[0];
    root.properties.remove("link");
    let NodeContent::Group { children } = &mut root.content else {
        panic!()
    };
    children[0]
        .properties
        .insert("link".into(), Value::Link(Link::Page(20)));
    e.select(6, false);
    let definition = e.create_component("Actions".into()).unwrap();
    let instance = e.nodes()[0].id;
    e.insert_component(definition, 400., 100.).unwrap();
    let source = e.document.definitions.clone();
    assert!(e.enter(instance));
    let group = e.current().controls[0].id;
    let before = e.document.clone();
    let mut c = e.current().controls[0].clone();
    c.link = Some(22);
    e.begin("Local group link");
    e.update_control(c);
    e.commit();
    assert_eq!(e.document.definitions, source);
    assert_eq!(
        e.rendered_scene(e.page).unwrap().hit_link(150., 160.),
        Some("page:22")
    );
    assert_eq!(
        e.rendered_scene(e.page).unwrap().hit_link(450., 160.),
        Some("page:20")
    );
    let linked = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, linked);
    e.reset_local_changes(group).unwrap();
    assert_eq!(
        e.rendered_scene(e.page).unwrap().hit_link(150., 160.),
        Some("page:20")
    );
    e.leave();
    e.select(instance, false);
    e.lock(true);
    let locked = e.document.clone();
    let mut c = e.selected_views()[0].clone();
    c.link = Some(22);
    e.begin("Locked link");
    e.update_control(c);
    e.commit();
    assert_eq!(e.nodes()[0].properties["link"], Value::Link(Link::Page(22)));
    assert!(e.own_locked(instance));
    assert_eq!(e.nodes()[0].origin, locked.pages[0].content.nodes[0].origin);
    e.undo();
    assert_eq!(e.document, locked);
    e.redo();
    let reopened = Editor::new(decode(&e.document.encode().unwrap()).unwrap());
    assert_eq!(
        reopened.rendered_scene(reopened.page).unwrap().svg(),
        e.rendered_scene(e.page).unwrap().svg()
    );
}

#[test]
fn full_target_editing_on_groups_keeps_history_reopen_and_scene_links() {
    let mut e = fixture();
    for target in [
        Some(Link::Back),
        Some(Link::Url("https://example.com/contact".into())),
        None,
        Some(Link::Page(22)),
    ] {
        let before = e.document.clone();
        e.begin("Edit group link");
        e.set_node_link(6, target.clone()).unwrap();
        e.commit();
        assert_eq!(e.node_link(6), target);
        let after = e.document.clone();
        let scene = e.rendered_scene(e.page).unwrap();
        let expected = match &target {
            Some(Link::Back) => Some("back:"),
            Some(Link::Url(s)) => Some(s.as_str()),
            Some(Link::Page(_)) => Some("page:22"),
            None => None,
        };
        assert_eq!(scene.hit_link(150., 160.), expected);
        e.undo();
        assert_eq!(e.document, before);
        e.redo();
        assert_eq!(e.document, after);
        let reopened = Editor::new(decode(&after.encode().unwrap()).unwrap());
        assert_eq!(reopened.node_link(6), target);
        assert_eq!(
            reopened.rendered_scene(reopened.page).unwrap().svg(),
            scene.svg()
        );
    }
}
#[test]
fn invalid_link_is_rejected_atomically_and_component_children_use_local_overrides() {
    let mut e = fixture();
    let before = e.document.clone();
    assert!(
        e.set_node_link(6, Some(Link::Url("javascript:alert(1)".into())))
            .is_err()
    );
    assert_eq!(e.document, before);
    e.select(6, false);
    e.create_component("Linked card".into()).unwrap();
    let sources = e.document.definitions.clone();
    assert!(e.enter(6));
    let id = e.nodes()[0].id;
    let before = e.document.clone();
    e.begin("Local group target");
    e.set_node_link(id, Some(Link::Back)).unwrap();
    e.commit();
    assert_eq!(e.node_link(id), Some(Link::Back));
    assert_eq!(e.document.definitions, sources);
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
}
