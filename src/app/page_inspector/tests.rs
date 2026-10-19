// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::collections::HashMap;

fn key(key: Key, modifiers: Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}
fn frame(
    app: &mut WireframeApp,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
) -> HashMap<String, Rect> {
    let mut raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 800.))),
        events,
        ..Default::default()
    };
    app.order_text_input(ctx, &mut raw);
    let mut out = ctx.run_ui(raw, |ui| {
        app.prepare_page_notes(ui.ctx());
        app.shortcuts(ui.ctx());
        app.inspector(ui);
        app.page_rename_dialog(ui.ctx());
    });
    let nodes = out
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter_map(|(_, n)| Some((n.label()?.to_owned(), n.bounds()?)))
        .map(|(s, b)| {
            (
                s,
                Rect::from_min_max(
                    egui::pos2(b.x0 as f32, b.y0 as f32),
                    egui::pos2(b.x1 as f32, b.y1 as f32),
                ),
            )
        })
        .collect();
    out.textures_delta.clear();
    nodes
}
fn setup() -> (egui::Context, WireframeApp) {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let app = WireframeApp::workspace(&ctx, None);
    (ctx, app)
}
fn click(app: &mut WireframeApp, ctx: &egui::Context, label: &str) {
    frame(app, ctx, vec![]);
    let nodes = frame(app, ctx, vec![]);
    let pos = nodes
        .get(label)
        .unwrap_or_else(|| panic!("Missing {label}: {:?}", nodes.keys()))
        .center();
    for pressed in [true, false] {
        frame(
            app,
            ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
    frame(app, ctx, vec![]);
}
fn begin(app: &mut WireframeApp, ctx: &egui::Context) {
    click(app, ctx, "Click here to edit notes.");
}

#[test]
fn notes_cancel_button_and_escape_discard_without_history() {
    let (ctx, mut app) = setup();
    let original = app.editor.document.clone();
    begin(&mut app, &ctx);
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Paste("Unsaved notes".into())],
    );
    assert_eq!(app.editor.document, original);
    click(&mut app, &ctx, "Cancel");
    assert!(!app.notes_editing());
    assert_eq!(app.editor.document, original);
    assert_eq!(app.editor.undo_name(), None);
    begin(&mut app, &ctx);
    frame(
        &mut app,
        &ctx,
        vec![
            egui::Event::Paste("Discard again".into()),
            key(Key::Escape, Modifiers::NONE),
        ],
    );
    frame(&mut app, &ctx, vec![]);
    assert!(!app.notes_editing());
    assert_eq!(app.editor.document, original);
}
#[test]
fn notes_submit_preserves_multiline_unicode_and_one_history_step() {
    let (ctx, mut app) = setup();
    let before = app.editor.document.clone();
    begin(&mut app, &ctx);
    frame(
        &mut app,
        &ctx,
        vec![
            egui::Event::Paste("Contact 👩‍💻 שלום".into()),
            key(Key::Enter, Modifiers::NONE),
            egui::Event::Text("Second line".into()),
        ],
    );
    frame(&mut app, &ctx, vec![key(Key::Enter, Modifiers::COMMAND)]);
    assert!(!app.notes_editing());
    assert_eq!(app.editor.page_data().notes, "Contact 👩‍💻 שלום\nSecond line");
    let after = app.editor.document.clone();
    assert_eq!(app.editor.undo_name(), Some("Edit notes"));
    frame(&mut app, &ctx, vec![key(Key::Z, Modifiers::COMMAND)]);
    assert_eq!(app.editor.document, before);
    frame(
        &mut app,
        &ctx,
        vec![key(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT)],
    );
    assert_eq!(app.editor.document, after);
}

#[test]
fn formatted_notes_reopen_exact_markup_and_cancel_without_navigation_or_history() {
    let (ctx, mut app) = setup();
    let text = "*Welcome* שלום\n[Website](https://example.com)\n:star-solid: :unknown-icon:\n- literal item";
    begin(&mut app, &ctx);
    frame(&mut app, &ctx, vec![egui::Event::Paste(text.into())]);
    frame(&mut app, &ctx, vec![key(Key::Enter, Modifiers::COMMAND)]);
    let saved = app.editor.document.clone();
    assert_eq!(app.editor.page_data().notes, text);
    assert_eq!(app.editor.undo_name(), Some("Edit notes"));
    click(&mut app, &ctx, text);
    assert_eq!(app.page_inspector.draft.as_ref().unwrap().text, text);
    frame(&mut app, &ctx, vec![key(Key::Escape, Modifiers::NONE)]);
    assert_eq!(app.editor.document, saved);
    app.editor.undo();
    assert_eq!(app.editor.page_data().notes, "");
    app.editor.redo();
    assert_eq!(app.editor.document, saved);
}
#[test]
fn text_then_new_page_and_pointer_departure_commit_to_old_owner() {
    let (ctx, mut app) = setup();
    let page = app.editor.page;
    begin(&mut app, &ctx);
    frame(
        &mut app,
        &ctx,
        vec![
            egui::Event::Paste("Original page".into()),
            key(Key::N, Modifiers::COMMAND | Modifiers::SHIFT),
        ],
    );
    frame(&mut app, &ctx, vec![]);
    assert_ne!(app.editor.page, page);
    assert_eq!(
        app.editor.document.live_page(page).unwrap().notes,
        "Original page"
    );
    assert_eq!(app.editor.page_data().notes, "");
    app.editor.undo();
    assert_eq!(app.editor.document.live_pages().count(), 1);
    assert_eq!(
        app.editor.document.live_page(page).unwrap().notes,
        "Original page"
    );
    app.editor.undo();
    assert_eq!(app.editor.document.live_page(page).unwrap().notes, "");
    begin(&mut app, &ctx);
    frame(&mut app, &ctx, vec![egui::Event::Paste("Blur".into())]);
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::PointerButton {
            pos: egui::pos2(20., 400.),
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        }],
    );
    assert!(!app.notes_editing());
    assert_eq!(app.editor.page_data().notes, "Blur");
}
#[test]
fn notes_save_command_captures_preceding_input_and_alternate_only() {
    let (ctx, mut app) = setup();
    let dir = tempfile::tempdir().unwrap();
    app.file = Some(dir.path().join("notes.wireframe"));
    let alternate = app.editor.create_version("Mobile").unwrap();
    begin(&mut app, &ctx);
    frame(
        &mut app,
        &ctx,
        vec![
            egui::Event::Paste("Alternate notes".into()),
            key(Key::S, Modifiers::COMMAND),
        ],
    );
    frame(&mut app, &ctx, vec![]);
    let saved = ProjectStore.load(app.file.as_ref().unwrap()).unwrap();
    assert_eq!(saved.surface_notes(alternate), "Alternate notes");
    assert_eq!(saved.pages[0].notes, "");
    assert!(!app.notes_editing());
    assert_eq!(saved, app.editor.document);
}
#[test]
fn project_rename_modal_preserves_filename_and_can_cancel() {
    let (ctx, mut app) = setup();
    let file = PathBuf::from("/tmp/rename-keeps-path.wireframe");
    app.file = Some(file.clone());
    let before = app.editor.document.clone();
    app.run_command(&ctx, menus::Command::RenameProject);
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![]);
    }
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Paste("Landing workspace".into())],
    );
    frame(&mut app, &ctx, vec![key(Key::Enter, Modifiers::NONE)]);
    assert_eq!(app.editor.document.title, "Landing workspace");
    assert_eq!(app.file, Some(file));
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    app.run_command(&ctx, menus::Command::RenameProject);
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![]);
    }
    frame(&mut app, &ctx, vec![egui::Event::Paste("Cancelled".into())]);
    frame(&mut app, &ctx, vec![key(Key::Escape, Modifiers::NONE)]);
    assert_eq!(app.editor.document, before);
}
#[test]
fn explicit_notes_owner_and_validation_are_atomic() {
    let mut editor = Editor::new(Project::default());
    let page = editor.page;
    let alt = editor.create_version("Compact").unwrap();
    editor.add_page();
    let active = editor.page;
    editor
        .edit_page_notes(page, Some(alt), "Compact notes")
        .unwrap();
    assert_eq!(editor.page, active);
    assert_eq!(editor.document.surface_notes(alt), "Compact notes");
    assert_eq!(editor.document.surface_notes(page), "");
    let before = editor.document.clone();
    let undo = editor.undo_name().map(str::to_owned);
    assert!(
        editor
            .edit_page_notes(page, Some(u64::MAX), "stale")
            .is_err()
    );
    assert!(
        editor
            .edit_page_notes(page, None, &"a".repeat(1_000_001))
            .is_err()
    );
    assert!(editor.rename_project(" ").is_err());
    assert_eq!(editor.document, before);
    assert_eq!(editor.undo_name(), undo.as_deref());
}

#[test]
fn alternate_rows_and_notes_editor_do_not_expand_the_panel() {
    let (ctx, mut app) = setup();
    app.editor.create_version("Compact").unwrap();
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    let width = || {
        egui::containers::panel::PanelState::load(&ctx, egui::Id::new("inspector"))
            .unwrap()
            .outer_rect
            .width()
    };
    let first = width();
    let mut last = first;
    for _ in 0..20 {
        frame(&mut app, &ctx, vec![]);
        last = width();
    }
    assert!(
        (last - first).abs() < 1.,
        "Inspector grew while idle: {first} -> {last}"
    );
    begin(&mut app, &ctx);
    for _ in 0..20 {
        frame(&mut app, &ctx, vec![]);
        last = width();
    }
    assert!(
        (last - first).abs() < 1.,
        "Editor expanded inspector: {first} -> {last}"
    );
}

#[test]
fn source_and_library_notes_keep_explicit_owners_on_departure_and_undo() {
    let (ctx, mut app) = setup();
    let page = app.editor.page;
    let library = app.editor.create_library("Shared controls").unwrap();
    let source = app
        .editor
        .create_empty_source(
            library,
            "Contact button",
            wireframe::document::Point { x: 6., y: 6. },
        )
        .unwrap();
    let before = app.editor.document.clone();
    begin(&mut app, &ctx);
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Paste("Source 👩‍💻 notes".into())],
    );
    // A model-level navigation between frames still saves to the original owner.
    app.editor.edit_library(library).unwrap();
    frame(&mut app, &ctx, vec![]);
    assert!(!app.notes_editing());
    assert_eq!(app.editor.document.surface_notes(source), "Source 👩‍💻 notes");
    assert_eq!(app.editor.document.surface_notes(library), "");
    assert_eq!(app.editor.document.surface_notes(page), "");
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    app.editor.edit_library(library).unwrap();
    begin(&mut app, &ctx);
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Paste("Library notes".into())],
    );
    app.back_to_wireframes();
    assert!(!app.editor.in_source_workspace());
    assert_eq!(app.editor.document.surface_notes(library), "Library notes");
    assert_eq!(app.editor.document.surface_notes(source), "");
    assert_eq!(app.editor.document.surface_notes(page), "");
}

#[test]
fn source_notes_cancel_and_usage_link_do_not_mutate_page_notes() {
    let (ctx, mut app) = setup();
    let page = app.editor.page;
    let library = app.editor.create_library("Shared controls").unwrap();
    let source = app
        .editor
        .create_empty_source(
            library,
            "Contact button",
            wireframe::document::Point { x: 6., y: 6. },
        )
        .unwrap();
    app.editor.switch_page(page);
    app.editor.insert_component(source, 80., 80.).unwrap();
    app.editor.edit_library_source(source).unwrap();
    let before = app.editor.document.clone();
    begin(&mut app, &ctx);
    frame(&mut app, &ctx, vec![egui::Event::Paste("Discarded".into())]);
    click(&mut app, &ctx, "Cancel");
    assert_eq!(app.editor.document, before);
    let page_name = app.editor.page_data().name.clone();
    click(&mut app, &ctx, &page_name);
    assert!(!app.editor.in_source_workspace());
    assert_eq!(app.editor.page, page);
    assert!(app.editor.selection.is_empty());
    assert_eq!(app.editor.document, before);
}

#[test]
fn source_notes_validation_failure_keeps_draft_and_blocks_back() {
    let (ctx, mut app) = setup();
    let library = app.editor.create_library("Shared controls").unwrap();
    let source = app
        .editor
        .create_empty_source(
            library,
            "Contact button",
            wireframe::document::Point { x: 6., y: 6. },
        )
        .unwrap();
    begin(&mut app, &ctx);
    app.page_inspector.draft.as_mut().unwrap().text = "x".repeat(1_000_001);
    app.back_to_wireframes();
    assert!(app.notes_editing());
    assert_eq!(app.editor.definition, Some(source));
    assert_eq!(app.editor.document.surface_notes(source), "");
    assert!(app.error.is_some());
}
