// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{self, Link, Value},
    model::Kind,
    rich_text::{self, structured::Grid},
    workspace::{Editor, LinkAction, LinkTarget},
};
fn control(e: &mut Editor, kind: Kind, text: &str) -> u64 {
    e.add(kind, 20., 30.);
    let id = *e.selection.iter().next().unwrap();
    e.set_control_property(id, "text", Value::Text(text.into()))
        .unwrap();
    id
}
fn text(e: &Editor, id: u64) -> String {
    match e.control_properties(id).unwrap().1.get("text").unwrap() {
        Value::Text(s) => s.clone(),
        _ => panic!(),
    }
}
#[test]
fn inline_destination_preserves_formatting_context_history_and_rendered_link() {
    let mut e = Editor::default();
    let id = control(
        &mut e,
        Kind::Paragraph,
        "Before [*Café*] after [Café](back:)",
    );
    let source = e.page;
    let before = e.document.clone();
    let dest = e
        .edit_text_links(&[id], 0, LinkAction::NewPage)
        .unwrap()
        .unwrap();
    assert_eq!(e.document.live_page(dest).unwrap().name, "*Café*");
    assert_eq!(e.text_links(id).unwrap().unwrap()[0].label, "Café");
    assert_eq!(e.page, source);
    assert_eq!(
        text(&e, id),
        format!("Before [*Café*](page:{dest}) after [Café](back:)")
    );
    assert_eq!(e.link_target(LinkTarget::text(id, 1)), Some(Link::Back));
    let after = e.document.clone();
    assert_eq!(document::decode(&after.encode().unwrap()).unwrap(), after);
    assert!(
        e.rendered_scene(source)
            .unwrap()
            .links
            .iter()
            .any(|l| l.target == format!("page:{dest}"))
    );
    e.undo();
    assert_eq!(e.document, before);
    e.redo();
    assert_eq!(e.document, after);
    e.edit_text_links(&[id], 0, LinkAction::Set(None)).unwrap();
    assert_eq!(text(&e, id), "Before [*Café*] after [Café](back:)");
}
#[test]
fn escaped_url_targets_round_trip_in_plain_and_table_text() {
    let target = "https://example.org/a(b)?q=c,d";
    for (kind, source) in [
        (Kind::Paragraph, "[Café] end"),
        (Kind::Table, " [Café] \\, end, [x]\n{70,30}"),
    ] {
        let mut e = Editor::default();
        let id = control(&mut e, kind, source);
        e.edit_text_links(&[id], 0, LinkAction::Set(Some(Link::Url(target.into()))))
            .unwrap();
        assert_eq!(
            e.text_links(id).unwrap().unwrap()[0].target,
            Some(Link::Url(target.into()))
        );
        assert!(
            e.rendered_scene(e.page)
                .unwrap()
                .links
                .iter()
                .any(|l| l.target == target)
        );
        e.edit_text_links(&[id], 0, LinkAction::Set(None)).unwrap();
        assert_eq!(text(&e, id), source);
    }
}

#[test]
fn inline_page_names_keep_formatting_and_duplicate_label_positions() {
    for (kind, source, first_name) in [
        (Kind::Paragraph, "[*Café*] then [Café]", "*Café*"),
        (Kind::Table, "[*É\\,cho*], [Écho]\n{50,50}", "*É,cho*"),
    ] {
        let mut e = Editor::default();
        let id = control(&mut e, kind, source);
        let first = e
            .edit_text_links(&[id], 0, LinkAction::NewPage)
            .unwrap()
            .unwrap();
        assert_eq!(e.document.live_page(first).unwrap().name, first_name);
        let before = e.document.clone();
        let second = e
            .edit_text_links(&[id], 1, LinkAction::DuplicatePage)
            .unwrap()
            .unwrap();
        assert_eq!(
            e.document.live_page(second).unwrap().name,
            if kind == Kind::Table {
                "Écho"
            } else {
                "Café"
            }
        );
        assert_eq!(
            e.link_target(LinkTarget::text(id, 0)),
            Some(Link::Page(first))
        );
        assert_eq!(
            e.link_target(LinkTarget::text(id, 1)),
            Some(Link::Page(second))
        );
        let after = e.document.clone();
        assert!(
            !e.document
                .live_page(second)
                .unwrap()
                .content
                .nodes
                .is_empty()
        );
        e.undo();
        assert_eq!(e.document, before);
        e.redo();
        assert_eq!(e.document, after);
    }
}
#[test]
fn table_links_use_rendered_cells_not_choice_markers_or_widths() {
    let source = " [É\\,cho] ^ , [x], [*], [-]\n[Same](back:), [Same], []\n{40L,20C,20R,20}";
    let links = Grid::source_links(source).unwrap();
    assert_eq!(links.len(), 3);
    assert_eq!(&source[links[0].label.clone()], "É\\,cho");
    let updated =
        rich_text::replace_source_link_target(source, &links[1], Some("page:99")).unwrap();
    assert_eq!(
        updated,
        source.replacen("[Same](back:)", "[Same](page:99)", 1)
    );
    assert_eq!(Grid::parse(source).rows.len(), 2);
}
#[test]
fn common_inline_indices_skip_missing_slots_and_reject_unsupported_controls() {
    let mut e = Editor::default();
    let a = control(&mut e, Kind::Paragraph, "[First]");
    let b = control(&mut e, Kind::Label, "[Other] and [Second]");
    let before = e.document.clone();
    let dest = e
        .edit_text_links(&[a, b], 1, LinkAction::DuplicatePage)
        .unwrap()
        .unwrap();
    assert_eq!(text(&e, a), "[First]");
    assert_eq!(
        e.link_target(LinkTarget::text(b, 1)),
        Some(Link::Page(dest))
    );
    assert_eq!(
        e.document.live_page(dest).unwrap().content,
        before.pages[0].content
    );
    e.undo();
    assert_eq!(e.document, before);
    let unsupported = control(&mut e, Kind::Button, "[Button]");
    let before = e.document.clone();
    assert!(
        e.edit_text_links(&[a, unsupported], 0, LinkAction::NewPage)
            .is_err()
    );
    assert_eq!(e.document, before);
}
#[test]
fn inline_component_override_preserves_source_and_undo() {
    let mut e = Editor::default();
    let root = control(&mut e, Kind::Paragraph, "[*Contact*]");
    e.create_component("Navigation".into()).unwrap();
    let definitions = e.document.definitions.clone();
    assert!(e.enter(root));
    let id = e.nodes()[0].id;
    let context = e.context.clone();
    let before = e.document.clone();
    e.edit_text_links(&[id], 0, LinkAction::NewPage).unwrap();
    assert_eq!(e.document.definitions, definitions);
    assert_eq!(e.context, context);
    assert!(text(&e, id).contains("(page:"));
    e.undo();
    assert_eq!(e.document, before);
}
#[test]
fn invalid_index_and_destination_are_atomic() {
    let mut e = Editor::default();
    let id = control(&mut e, Kind::Paragraph, "[First]");
    let before = e.document.clone();
    for (index, action) in [
        (2, LinkAction::NewPage),
        (0, LinkAction::Set(Some(Link::Page(u64::MAX)))),
        (0, LinkAction::Set(Some(Link::Url("invalid".into())))),
    ] {
        assert!(e.edit_text_links(&[id], index, action).is_err());
        assert_eq!(e.document, before);
    }
    let malformed = LinkTarget {
        node: id,
        row: Some(0),
        text: Some(0),
    };
    assert!(e.edit_links(&[malformed], LinkAction::NewPage).is_err());
    assert_eq!(e.document, before);
}
#[test]
fn source_replacement_rejects_invalid_ranges_and_ignores_literal_markup() {
    let source = r"\[literal] [*É*](back:)";
    let links = rich_text::source_links(source).unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(
        rich_text::replace_source_link_target(source, &links[0], None).unwrap(),
        r"\[literal] [*É*]"
    );
    for (range, label) in [
        (usize::MAX..usize::MAX, usize::MAX..usize::MAX),
        (0..999, 1..2),
        (0..3, std::ops::Range { start: 2, end: 1 }),
    ] {
        let invalid = rich_text::SourceLink {
            range,
            label,
            target: None,
        };
        assert!(rich_text::replace_source_link_target(source, &invalid, None).is_err());
    }
}

#[test]
fn escaped_comma_at_table_label_boundary_keeps_its_source_range() {
    for source in [r"[\,start], next", r"[end\,], next", r"[\,], next"] {
        let links = Grid::source_links(source).unwrap();
        let updated =
            rich_text::replace_source_link_target(source, &links[0], Some("back:")).unwrap();
        assert_eq!(updated, source.replacen(']', "](back:)", 1));
        let links = Grid::source_links(&updated).unwrap();
        assert_eq!(
            rich_text::replace_source_link_target(&updated, &links[0], None).unwrap(),
            source
        );
    }
}
