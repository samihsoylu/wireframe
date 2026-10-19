// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
#[test]
fn queued_commands_recheck_modal_boundaries() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    let before = app.editor.document.clone();
    app.run_command(&ctx, Command::Font);
    app.run_command(&ctx, Command::NewPage);
    app.run_command(&ctx, Command::Quit);
    assert_eq!(app.editor.document, before);
    assert!(app.pending.is_none());
    app.font_import = None;
    app.run_command(&ctx, Command::NewPage);
    assert_eq!(app.editor.document.pages.len(), before.pages.len() + 1);
    app.run_command(&ctx, Command::Quit);
    assert!(matches!(app.pending, Some(Pending::Quit)));
    app.run_command(&ctx, Command::NewPage);
    assert_eq!(app.editor.document.pages.len(), before.pages.len() + 1);
}
#[test]
fn held_insertion_rejects_stale_menu_actions() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    let before = app.editor.document.clone();
    app.library_drag = Some(CatalogItem::Control(Kind::Button));
    for command in [
        Command::NewPage,
        Command::Quit,
        Command::Sketch,
        Command::Grid,
    ] {
        assert!(!app.command_enabled(&ctx, command));
        app.run_command(&ctx, command);
    }
    assert_eq!(app.editor.document, before);
    assert!(app.pending.is_none());
}

#[test]
fn explicit_canvas_commands_keep_their_owner_while_a_field_has_focus() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 50., 50.);
    let mut field = "Draft".to_owned();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        ui.add(TextInput::singleline(&mut field).autofocus(true));
    });
    output.textures_delta.clear();
    assert!(wireframe::text_entry::is_focused(&ctx));
    assert!(app.command_enabled(&ctx, Command::Duplicate));
    app.run_command(&ctx, Command::Duplicate);
    assert_eq!(app.editor.current().controls.len(), 2);
    assert_eq!(field, "Draft");
    app.font_import = Some(fonts::FontImport::default());
    assert!(!app.command_enabled(&ctx, Command::Duplicate));
    app.run_command(&ctx, Command::Duplicate);
    assert_eq!(app.editor.current().controls.len(), 2);
}

#[test]
fn locked_source_canvas_commands_protect_document_without_disabling_copy_or_text() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 50., 50.);
    let source = app.editor.create_component("Action".into()).unwrap();
    let (library, owner) = app.editor.document.component_source(source).unwrap();
    let (library, owner) = (library.id, owner.node);
    app.editor.edit_library(library).unwrap();
    app.editor.select(owner, false);
    app.editor.lock(true);
    app.editor.edit_library_source(source).unwrap();
    app.editor.select_all();
    let before = app.editor.document.clone();
    for command in [
        Command::Cut,
        Command::Delete,
        Command::Group,
        Command::Ungroup,
    ] {
        assert!(!app.command_enabled(&ctx, command));
        app.run_command(&ctx, command);
    }
    assert_eq!(app.editor.document, before);
    assert!(app.clipboard.is_empty());
    for command in [Command::Copy, Command::Duplicate, Command::SelectAll] {
        assert!(app.command_enabled(&ctx, command));
    }
    let mut output = ctx.run_ui(
        egui::RawInput {
            events: vec![egui::Event::Cut],
            ..Default::default()
        },
        |ui| app.shortcuts(ui.ctx()),
    );
    output.textures_delta.clear();
    assert!(app.clipboard.is_empty());
    assert_eq!(app.editor.document, before);
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 800.))),
            ..Default::default()
        },
        |ui| app.inspector(ui),
    );
    let nodes = &output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes;
    for name in ["Group", "Ungroup", "Delete", "Make component"] {
        assert!(
            nodes
                .iter()
                .any(|(_, n)| n.label() == Some(name) && n.is_disabled()),
            "{name}"
        );
    }
    assert!(
        nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Duplicate") && !n.is_disabled())
    );
    output.textures_delta.clear();
    let mut draft = "Contact".to_owned();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        ui.add(TextInput::singleline(&mut draft).autofocus(true));
    });
    output.textures_delta.clear();
    assert!(app.command_enabled(&ctx, Command::Cut));
    assert!(app.command_enabled(&ctx, Command::Delete));
}

#[test]
fn layering_commands_obey_selection_text_and_modal_ownership() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    for _ in 0..3 {
        app.editor.add(Kind::Button, 20., 20.);
    }
    let first = app.editor.nodes()[0].id;
    app.editor.select(first, false);
    assert!(app.command_enabled(&ctx, Command::BringForward));
    assert!(!app.command_enabled(&ctx, Command::SendBackward));
    let before = app.editor.document.clone();
    app.run_command(&ctx, Command::BringForward);
    assert_eq!(app.editor.nodes()[1].id, first);
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    app.font_import = Some(fonts::FontImport::default());
    app.run_command(&ctx, Command::BringToFront);
    assert_eq!(app.editor.document, before);
    app.font_import = None;
    let mut field = "Draft".to_owned();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        ui.add(TextInput::singleline(&mut field).autofocus(true));
    });
    output.textures_delta.clear();
    assert!(wireframe::text_entry::is_focused(&ctx));
    app.run_command(&ctx, Command::BringToFront);
    assert_eq!(app.editor.document, before);
}

#[test]
fn markup_command_updates_mixed_labels_and_obeys_modal_and_text_focus() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    assert!(!app.command_enabled(&ctx, Command::TreatAsMarkup));
    app.editor.add(Kind::Button, 50., 50.);
    app.editor.add(Kind::Callout, 150., 50.);
    app.editor.select_all();
    app.editor.document.pages[0].content.nodes[0].locked = true;
    let before = app.editor.document.clone();
    assert_eq!(app.command_label(Command::TreatAsMarkup), "Treat as Markup");
    app.run_command(&ctx, Command::TreatAsMarkup);
    assert!(app.editor.nodes().iter().all(|n| n.markup));
    assert_eq!(
        app.command_label(Command::TreatAsMarkup),
        "Do not Treat as Markup"
    );
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    let mut field = "Draft".to_owned();
    let output = ctx.run_ui(egui::RawInput::default(), |ui| {
        let response = ui.add(TextInput::singleline(&mut field));
        response.request_focus();
    });
    output.drop_without_applying_deltas();
    assert!(!app.command_enabled(&ctx, Command::TreatAsMarkup));
    app.run_command(&ctx, Command::TreatAsMarkup);
    assert_eq!(app.editor.document, before);
    if let Some(id) = ctx.memory(|m| m.focused()) {
        ctx.memory_mut(|m| m.surrender_focus(id));
    }
    app.error = Some("Example error".into());
    assert!(!app.command_enabled(&ctx, Command::TreatAsMarkup));
    app.run_command(&ctx, Command::TreatAsMarkup);
    assert_eq!(app.editor.document, before);
}

#[test]
fn visibility_command_is_transient_and_respects_text_modal_gesture_and_presentation_owners() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Callout, 40., 50.);
    let before = app.editor.document.clone();
    let history = app.editor.undo_name().unwrap().to_owned();
    assert_eq!(app.command_checked(Command::ShowMarkup), Some(true));
    app.run_command(&ctx, Command::ShowMarkup);
    assert_eq!(app.command_checked(Command::ShowMarkup), Some(false));
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.undo_name(), Some(history.as_str()));
    let mut field = "Draft".to_owned();
    ctx.run_ui(egui::RawInput::default(), |ui| {
        ui.add(TextInput::singleline(&mut field)).request_focus();
    })
    .drop_without_applying_deltas();
    app.run_command(&ctx, Command::ShowMarkup);
    assert!(!app.editor.markup_visible());
    if let Some(id) = ctx.memory(|m| m.focused()) {
        ctx.memory_mut(|m| m.surrender_focus(id));
    }
    app.error = Some("Example".into());
    app.run_command(&ctx, Command::ShowMarkup);
    assert!(!app.editor.markup_visible());
    app.error = None;
    app.library_drag = Some(CatalogItem::Control(Kind::Button));
    app.run_command(&ctx, Command::ShowMarkup);
    assert!(!app.editor.markup_visible());
    app.library_drag = None;
    app.present = true;
    app.run_command(&ctx, Command::ShowMarkup);
    assert!(!app.editor.markup_visible());
    app.present = false;
    app.run_command(&ctx, Command::ShowMarkup);
    assert!(app.editor.markup_visible());
}

#[test]
fn pdf_draft_captures_editor_markup_preference_without_changing_document() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    let before = app.editor.document.clone();
    app.editor.set_markup_visible(false);
    app.run_command(&ctx, Command::ExportPdf);
    assert!(!app.pdf_options.as_ref().unwrap().markup_visible);
    app.pdf_options = None;
    app.editor.set_markup_visible(true);
    app.run_command(&ctx, Command::ExportPdf);
    assert!(app.pdf_options.as_ref().unwrap().markup_visible);
    assert_eq!(app.editor.document, before);
}
