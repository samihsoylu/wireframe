// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{self, Link, Value},
    model::Kind,
    workspace::{Editor, LinkAction, LinkTarget},
};
fn button(e: &mut Editor, text: &str) -> u64 {
    e.add(Kind::Button, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    e.set_control_property(id, "text", Value::Text(text.into()))
        .unwrap();
    id
}
#[test]
fn new_destination_links_all_controls_stays_on_source_and_undoes_as_one_action() {
    let mut e = Editor::default();
    let a = button(&mut e, "Contact");
    let b = button(&mut e, "Details");
    e.select_all();
    let source = e.page;
    let selection = e.selection.clone();
    let before = e.document.clone();
    let destination = e
        .edit_links(
            &[LinkTarget::whole(a), LinkTarget::whole(b)],
            LinkAction::NewPage,
        )
        .unwrap()
        .unwrap();
    let page = e.document.live_page(destination).unwrap();
    assert_eq!(page.name, "Contact");
    assert!(page.content.nodes.is_empty());
    assert_eq!(e.page, source);
    assert_eq!(e.selection, selection);
    for id in [a, b] {
        assert_eq!(e.node_link(id), Some(Link::Page(destination)));
    }
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(document::decode(&after.encode().unwrap()).unwrap(), after);
    assert!(
        e.rendered_scene(source)
            .unwrap()
            .links
            .iter()
            .all(|l| l.target == format!("page:{destination}"))
    );
}
#[test]
fn duplicate_uses_active_version_before_linking_and_preserves_source_context() {
    let mut e = Editor::default();
    let id = button(&mut e, "Official");
    let alt = e.create_version("Proposal").unwrap();
    e.set_control_property(id, "text", Value::Text("Proposal contact".into()))
        .unwrap();
    let source = e.page;
    let contents = e.document.pages[0]
        .alternates
        .iter()
        .find(|a| a.id == alt)
        .unwrap()
        .content
        .clone();
    let before = e.document.clone();
    let dest = e
        .edit_links(&[LinkTarget::whole(id)], LinkAction::DuplicatePage)
        .unwrap()
        .unwrap();
    let page = e.document.live_page(dest).unwrap();
    assert_eq!(page.name, "Proposal contact");
    assert_eq!(page.content, contents);
    assert!(page.alternates.is_empty());
    assert_eq!(e.page, source);
    assert_eq!(e.active_alternate(source), Some(alt));
    assert_eq!(e.node_link(id), Some(Link::Page(dest)));
    let after = e.document.clone();
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(e.active_alternate(source), Some(alt));
    e.redo();
    assert_eq!(e.document, after);
    assert_eq!(e.page, source);
}
#[test]
fn destination_is_last_sibling_after_descendants_without_copying_page_tree() {
    let mut e = Editor::default();
    let parent = e.page;
    e.add_page();
    let source = e.page;
    let id = button(&mut e, "Child target");
    e.document.pages.last_mut().unwrap().parent = Some(parent);
    e.add_page();
    let sibling = e.page;
    e.document.pages.last_mut().unwrap().parent = Some(parent);
    e.add_page();
    let grandchild = e.page;
    e.document.pages.last_mut().unwrap().parent = Some(sibling);
    e.add_page();
    let unrelated = e.page;
    e.switch_page(source);
    e.select(id, false);
    let dest = e
        .edit_links(&[LinkTarget::whole(id)], LinkAction::NewPage)
        .unwrap()
        .unwrap();
    assert_eq!(
        e.document.pages.iter().map(|p| p.id).collect::<Vec<_>>(),
        vec![parent, source, sibling, grandchild, dest, unrelated]
    );
    assert_eq!(e.document.live_page(dest).unwrap().parent, Some(parent));
    // A retained parent no longer establishes a visible nesting level.
    e.document.pages[0].trashed = true;
    let next = e
        .edit_links(&[LinkTarget::whole(id)], LinkAction::NewPage)
        .unwrap()
        .unwrap();
    assert_eq!(e.document.live_page(next).unwrap().parent, None);
    assert_eq!(e.document.pages.last().unwrap().id, next);
}
#[test]
fn row_actions_share_one_destination_keep_other_rows_and_rollback_invalid_targets() {
    let mut e = Editor::default();
    let mut ids = vec![];
    for _ in 0..2 {
        e.add(Kind::ButtonBar, 20., 30.);
        let id = *e.selection.iter().next().unwrap();
        e.set_control_property(id, "text", Value::Text("Home,Contact,Order".into()))
            .unwrap();
        e.set_control_property(id, "row_links", Value::Links(vec![Some(Link::Back)]))
            .unwrap();
        ids.push(id);
    }
    let targets: Vec<_> = ids.iter().map(|&id| LinkTarget::row(id, 1)).collect();
    let before = e.document.clone();
    let dest = e
        .edit_links(&targets, LinkAction::NewPage)
        .unwrap()
        .unwrap();
    assert_eq!(e.document.live_page(dest).unwrap().name, "Contact");
    for &id in &ids {
        assert_eq!(e.link_target(LinkTarget::row(id, 0)), Some(Link::Back));
        assert_eq!(
            e.link_target(LinkTarget::row(id, 1)),
            Some(Link::Page(dest))
        );
    }
    e.undo();
    assert_eq!(e.document, before);
    for bad in [LinkTarget::row(ids[1], 99), LinkTarget::whole(99999)] {
        assert!(
            e.edit_links(&[targets[0], bad], LinkAction::NewPage)
                .is_err()
        );
        assert_eq!(e.document, before);
    }
    for invalid in [Link::Url("javascript:bad".into()), Link::Page(99999)] {
        assert!(
            e.edit_links(&targets, LinkAction::Set(Some(invalid)))
                .is_err()
        );
        assert_eq!(e.document, before);
    }
    e.set_control_property(ids[0], "text", Value::Text("Only".into()))
        .unwrap();
    let first = e.nodes()[0].clone();
    let before = e.document.clone();
    let dest = e
        .edit_row_links(&ids, 1, LinkAction::NewPage)
        .unwrap()
        .unwrap();
    assert_eq!(e.nodes()[0], first);
    assert_eq!(e.document.live_page(dest).unwrap().name, "Only");
    assert_eq!(
        e.link_target(LinkTarget::row(ids[1], 1)),
        Some(Link::Page(dest))
    );
    e.undo();
    assert_eq!(e.document, before);
}
#[test]
fn component_local_link_creation_preserves_definitions_and_nested_editing_context() {
    let mut e = Editor::default();
    let root = button(&mut e, "Contact");
    e.create_component("Navigation".into()).unwrap();
    let definitions = e.document.definitions.clone();
    assert!(e.enter(root));
    let id = e.nodes()[0].id;
    e.select(id, false);
    let context = e.context.clone();
    let selection = e.selection.clone();
    let before = e.document.clone();
    let dest = e
        .edit_links(&[LinkTarget::whole(id)], LinkAction::NewPage)
        .unwrap()
        .unwrap();
    assert_eq!(e.document.definitions, definitions);
    assert_eq!(e.context, context);
    assert_eq!(e.selection, selection);
    assert_eq!(e.node_link(id), Some(Link::Page(dest)));
    e.undo();
    assert_eq!(e.document, before);
    assert_eq!(e.context, context);
}
#[test]
fn common_targets_apply_even_when_one_control_already_matches_and_clear_atomically() {
    let mut e = Editor::default();
    let a = button(&mut e, "A");
    let b = button(&mut e, "B");
    e.set_node_link(a, Some(Link::Back)).unwrap();
    let before = e.document.clone();
    let targets = [
        LinkTarget::whole(a),
        LinkTarget::whole(b),
        LinkTarget::whole(a),
    ];
    e.edit_links(&targets, LinkAction::Set(Some(Link::Back)))
        .unwrap();
    assert_eq!(e.node_link(a), Some(Link::Back));
    assert_eq!(e.node_link(b), Some(Link::Back));
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    let linked = e.document.clone();
    e.edit_links(&targets, LinkAction::Set(None)).unwrap();
    assert_eq!(e.node_link(a), None);
    assert_eq!(e.node_link(b), None);
    e.undo();
    assert_eq!(e.document, linked);
}

#[test]
fn failed_creation_restores_allocated_page_links_and_history_without_losing_prior_edits() {
    let mut e = Editor::default();
    let id = button(&mut e, "Short");
    e.begin("Type caption");
    e.set_control_property(id, "text", Value::Text("x".repeat(10001)))
        .unwrap();
    let before = e.document.clone();
    assert!(
        e.edit_links(&[LinkTarget::whole(id)], LinkAction::NewPage)
            .is_err()
    );
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), Some("Type caption"));
    e.undo();
    assert_eq!(
        e.control_properties(id).unwrap().1["text"],
        Value::Text("Short".into())
    );
}

#[test]
fn source_duplicate_uses_whole_library_and_keeps_raw_caption_as_destination_name() {
    let mut e = Editor::default();
    button(&mut e, "*Contact*\nDetails");
    let definition = e.create_component("Navigation".into()).unwrap();
    let library = e
        .document
        .component_source(definition)
        .unwrap()
        .0
        .content
        .clone();
    e.edit_definition(definition).unwrap();
    let id = e.nodes()[0].id;
    let before = e.document.clone();
    let dest = e
        .edit_links(&[LinkTarget::whole(id)], LinkAction::DuplicatePage)
        .unwrap()
        .unwrap();
    let page = e.document.live_page(dest).unwrap();
    assert_eq!(page.name, "*Contact*\nDetails");
    assert_eq!(page.content, library);
    assert!(page.parent.is_none());
    assert_eq!(e.definition, Some(definition));
    e.undo();
    assert_eq!(e.document, before);
}

#[test]
fn common_app_bar_menu_targets_follow_the_trailing_slot_for_different_icon_counts() {
    let mut e = Editor::default();
    let mut ids = vec![];
    for icons in [vec![], vec![None, None]] {
        e.add(Kind::AppBar, 20., 30.);
        let id = *e.selection.iter().next().unwrap();
        e.set_control_property(id, "icons", Value::Icons(icons))
            .unwrap();
        ids.push(id);
    }
    assert_eq!(
        e.common_row_targets(&ids, 4).unwrap(),
        vec![LinkTarget::row(ids[0], 2), LinkTarget::row(ids[1], 4)]
    );
    let before = e.document.clone();
    let dest = e
        .edit_row_links(&ids, 4, LinkAction::NewPage)
        .unwrap()
        .unwrap();
    assert_eq!(e.document.live_page(dest).unwrap().name, "Menu");
    assert_eq!(
        e.link_target(LinkTarget::row(ids[0], 2)),
        Some(Link::Page(dest))
    );
    assert_eq!(
        e.link_target(LinkTarget::row(ids[1], 4)),
        Some(Link::Page(dest))
    );
    assert_eq!(e.link_target(LinkTarget::row(ids[1], 2)), None);
    e.undo();
    assert_eq!(e.document, before);
}
