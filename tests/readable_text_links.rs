// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{Link, Value},
    model::Kind,
    workspace::{Editor, LinkTarget},
};

fn page(e: &mut Editor, name: &str) -> u64 {
    e.add_page();
    let id = e.page;
    e.rename_page(id, name).unwrap();
    id
}
fn control(e: &mut Editor, kind: Kind, value: &str) -> u64 {
    e.add(kind, 50., 50.);
    let id = *e.selection.iter().next().unwrap();
    e.set_control_property(id, "text", Value::Text(value.into()))
        .unwrap();
    id
}
#[test]
fn readable_names_round_trip_escaped_text_and_follow_renames() {
    for kind in [Kind::Paragraph, Kind::Table] {
        let mut e = Editor::default();
        let source = e.page;
        let dest = page(&mut e, "Contact (EU), café");
        e.switch_page(source);
        let stored = if kind == Kind::Table {
            format!(" [*É\\,cho*](page:{dest}), [Home](back:)\n{{60,40}}")
        } else {
            format!("Before [*Café*](page:{dest}) then [Home](back:)")
        };
        let id = control(&mut e, kind, &stored);
        let before = e.document.clone();
        let shown = e.prepare_text_edit(id).unwrap().unwrap().1;
        assert!(shown.contains(r"(Contact \(EU\)\, café)"));
        assert_eq!(e.control_text_from_edit(id, &shown).unwrap(), stored);
        assert_eq!(
            e.control_text_from_edit(id, &format!("Prefix {shown}"))
                .unwrap(),
            format!("Prefix {stored}")
        );
        assert_eq!(e.document, before);
        e.rename_page(dest, "*Renamed* 世界").unwrap();
        let renamed = e.prepare_text_edit(id).unwrap().unwrap().1;
        assert!(renamed.contains("(*Renamed* 世界)"));
        assert_eq!(e.control_text_from_edit(id, &renamed).unwrap(), stored);
    }
}
#[test]
fn duplicate_page_names_keep_existing_targets_through_edits_and_reorder() {
    let mut e = Editor::default();
    let source = e.page;
    let first = page(&mut e, "Same");
    let second = page(&mut e, "Same");
    e.switch_page(source);
    let stored = format!("[A](page:{first}) [B](page:{second}) [B](page:{first})");
    let id = control(&mut e, Kind::Paragraph, &stored);
    let shown = e.prepare_text_edit(id).unwrap().unwrap().1;
    assert_eq!(shown, "[A](Same) [B](Same) [B](Same)");
    assert_eq!(e.control_text_from_edit(id, &shown).unwrap(), stored);
    assert_eq!(
        e.control_text_from_edit(id, &format!("New {shown}"))
            .unwrap(),
        format!("New {stored}")
    );
    assert_eq!(
        e.control_text_from_edit(id, "[B](Same) [B](Same) [B](Same)")
            .unwrap(),
        format!("[B](page:{first}) [B](page:{second}) [B](page:{first})")
    );
    assert_eq!(
        e.control_text_from_edit(id, "[B](Same) [A](Same) [B](Same)")
            .unwrap(),
        format!("[B](page:{second}) [A](page:{first}) [B](page:{first})")
    );
    assert_eq!(
        e.control_text_from_edit(id, "[A](Same) [Renamed label](Same) [B](Same)")
            .unwrap(),
        format!("[A](page:{first}) [Renamed label](page:{second}) [B](page:{first})")
    );
    assert_eq!(
        e.control_text_from_edit(id, "[A](Same) [B](Same) [B](Same) [New](Same)")
            .unwrap(),
        format!("{stored} [New](page:{first})")
    );
}
#[test]
fn names_resolve_only_in_supported_links_and_preserve_unresolved_targets() {
    let mut e = Editor::default();
    let source = e.page;
    let dest = page(&mut e, "Contact");
    page(&mut e, "https://example.org");
    e.switch_page(source);
    let stored = "[Web](https://example.org) [Missing](page:999999) [Back](back:)";
    let id = control(&mut e, Kind::Paragraph, stored);
    let shown = e.control_text_for_edit(id, stored).unwrap();
    assert_eq!(shown, stored);
    assert_eq!(
        e.control_text_from_edit(id, &format!("New {shown} [Go](Contact) [Unknown](Other)"))
            .unwrap(),
        format!("New {stored} [Go](page:{dest}) [Unknown](Other)")
    );
    let button = control(&mut e, Kind::Button, "[Go](Contact)");
    assert_eq!(
        e.control_text_from_edit(button, "[Go](Contact)").unwrap(),
        "[Go](Contact)"
    );
}
#[test]
fn preview_uses_stable_destinations_without_changing_saved_document_or_history() {
    let mut e = Editor::default();
    let source = e.page;
    let dest = page(&mut e, "Contact");
    e.switch_page(source);
    let id = control(&mut e, Kind::Paragraph, &format!("[Go](page:{dest})"));
    let before = e.document.clone();
    let history = e.undo_name().map(str::to_owned);
    let scene = e
        .text_preview_scene(id, "Changed [Go](Contact) [Back](back:)")
        .unwrap();
    assert!(
        scene
            .links
            .iter()
            .any(|l| l.target == format!("page:{dest}"))
    );
    assert!(scene.links.iter().any(|l| l.target == "back:"));
    assert_eq!(e.document, before);
    assert_eq!(e.undo_name(), history.as_deref());
    assert_eq!(
        e.link_target(LinkTarget::text(id, 0)),
        Some(Link::Page(dest))
    );
}
