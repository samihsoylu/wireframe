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

#[test]
fn alternate_divider_tracks_slow_and_batched_drags_without_editing_the_project() {
    for schedule in 0..3 {
        let (ctx, mut app) = setup();
        app.file = Some(std::path::PathBuf::from("divider-study.wireframe"));
        let original = app.editor.document.clone();
        let key = app.version_preferences_key();
        for (drag_height, target_height) in [(250_u16, 250_u16), (500, 400), (40, 130)] {
            for _ in 0..3 {
                frame(&mut app, &ctx, vec![]);
            }
            let height = app.page_inspector.preferences.get(key.as_deref());
            let id = egui::Id::new(("page-alternates-panel", &key, height));
            let rect = egui::containers::panel::PanelState::load(&ctx, id)
                .unwrap()
                .outer_rect;
            let start = egui::pos2(rect.center().x, rect.min.y);
            let end = start + Vec2::new(0., height as f32 - drag_height as f32);
            let button = |pos, pressed| egui::Event::PointerButton {
                pos,
                pressed,
                button: egui::PointerButton::Primary,
                modifiers: Modifiers::NONE,
            };
            frame(&mut app, &ctx, vec![egui::Event::PointerMoved(start)]);
            if schedule == 0 {
                frame(&mut app, &ctx, vec![button(start, true)]);
                frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
                frame(&mut app, &ctx, vec![button(end, false)]);
            } else if schedule == 1 {
                frame(
                    &mut app,
                    &ctx,
                    vec![
                        button(start, true),
                        egui::Event::PointerMoved(end),
                        button(end, false),
                    ],
                );
            } else {
                frame(&mut app, &ctx, vec![button(start, true)]);
                frame(
                    &mut app,
                    &ctx,
                    vec![egui::Event::PointerMoved(end), button(end, false)],
                );
            }
            for _ in 0..3 {
                frame(&mut app, &ctx, vec![]);
            }
            assert_eq!(
                app.page_inspector.preferences.get(key.as_deref()),
                target_height,
                "schedule {schedule}"
            );
            let id = egui::Id::new(("page-alternates-panel", &key, target_height));
            let after = egui::containers::panel::PanelState::load(&ctx, id)
                .unwrap()
                .outer_rect;
            assert!((after.height() - target_height as f32).abs() <= 1.);
            assert_eq!(app.editor.document, original);
            assert!(app.editor.undo_name().is_none());
            assert!(app.editor.selection.is_empty());
        }
    }
}

#[test]
fn alternate_panel_preferences_reopen_per_project_and_survive_transient_reset() {
    let dir = tempfile::tempdir().unwrap();
    let (_, mut app) = setup();
    app.load_panel_preferences(dir.path());
    assert_eq!(app.page_inspector.preferences.get(Some("new")), 200);
    app.page_inspector
        .preferences
        .remember(Some("first"), 250)
        .unwrap();
    app.page_inspector
        .preferences
        .remember(Some("second"), 400)
        .unwrap();
    app.page_inspector.clear();
    assert_eq!(app.page_inspector.preferences.get(Some("first")), 250);
    let mut reopened = preferences::Preferences::default();
    reopened.load(dir.path()).unwrap();
    assert_eq!(reopened.get(Some("first")), 250);
    assert_eq!(reopened.get(Some("second")), 400);
    assert_eq!(reopened.get(Some("new")), 400);
    assert!(reopened.remember(Some("first"), 129).is_err());
    assert_eq!(reopened.get(Some("first")), 250);
}

#[test]
fn alternate_divider_has_a_five_point_hit_area_and_restores_the_surrounding_style() {
    for radius in [3., 7.] {
        let (ctx, mut app) = setup();
        let key = app.version_preferences_key();
        let id = egui::Id::new(("page-alternates-panel", &key, 200_u16));
        for _ in 0..3 {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800., 800.))),
                    ..Default::default()
                },
                |ui| {
                    ui.style_mut().interaction.resize_grab_radius_side = radius;
                    app.alternate_panel(ui);
                    assert_eq!(ui.style().interaction.resize_grab_radius_side, radius);
                    let handle = ui.read_response(id.with("__resize")).unwrap();
                    assert_eq!(handle.rect.height(), 5.);
                },
            )
            .drop_without_applying_deltas();
        }
    }
}

#[test]
fn alternate_divider_accepts_inside_grabs_and_ignores_adjacent_notes_and_heading_drags() {
    for offset in [-3., -2.5, -2., 2., 2.5, 3.] {
        let (ctx, mut app) = setup();
        app.file = Some(std::path::PathBuf::from("divider-hit-area.wireframe"));
        let original = app.editor.document.clone();
        let key = app.version_preferences_key();
        let id = egui::Id::new(("page-alternates-panel", &key, 200_u16));
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let rect = egui::containers::panel::PanelState::load(&ctx, id)
            .unwrap()
            .outer_rect;
        let start = egui::pos2(rect.center().x, rect.min.y + offset);
        let end = start - Vec2::new(0., 40.);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            pressed,
            button: egui::PointerButton::Primary,
            modifiers: Modifiers::NONE,
        };
        frame(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(start),
                button(start, true),
                egui::Event::PointerMoved(end),
                button(end, false),
            ],
        );
        let height = app.page_inspector.preferences.get(key.as_deref());
        assert_eq!(
            height,
            if (-2. ..3.).contains(&offset) {
                240
            } else {
                200
            },
            "offset {offset}, height {height}"
        );
        assert_eq!(app.editor.document, original);
        assert!(app.editor.undo_name().is_none());
        assert!(!app.notes_editing());
    }
}

#[test]
fn held_divider_motion_keeps_the_grab_offset_and_saves_only_at_release() {
    for offset in [-2., 2.] {
        let (ctx, mut app) = setup();
        let original = app.editor.document.clone();
        let key = app.version_preferences_key();
        let id = egui::Id::new(("page-alternates-panel", &key, 200_u16));
        for _ in 0..3 {
            frame(&mut app, &ctx, vec![]);
        }
        let initial = egui::containers::panel::PanelState::load(&ctx, id)
            .unwrap()
            .outer_rect;
        let start = egui::pos2(initial.center().x, initial.top() + offset);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            pressed,
            button: egui::PointerButton::Primary,
            modifiers: Modifiers::NONE,
        };
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start), button(start, true)],
        );
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start - Vec2::new(0., 10.))],
        );
        assert_eq!(
            egui::containers::panel::PanelState::load(&ctx, id)
                .unwrap()
                .outer_rect
                .height(),
            210.
        );
        assert_eq!(app.page_inspector.preferences.get(key.as_deref()), 200);
        let end = start - Vec2::new(0., 20.);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(end), button(end, false)],
        );
        assert_eq!(app.page_inspector.preferences.get(key.as_deref()), 220);
        assert_eq!(app.editor.document, original);
        assert!(app.editor.undo_name().is_none());
        assert!(!app.notes_editing());
    }
}

#[test]
fn divider_release_at_the_press_point_restores_the_initial_height() {
    let (ctx, mut app) = setup();
    let original = app.editor.document.clone();
    let key = app.version_preferences_key();
    let id = egui::Id::new(("page-alternates-panel", &key, 200_u16));
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![]);
    }
    let rect = egui::containers::panel::PanelState::load(&ctx, id)
        .unwrap()
        .outer_rect;
    let start = egui::pos2(rect.center().x, rect.top() + 2.);
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        pressed,
        button: egui::PointerButton::Primary,
        modifiers: Modifiers::NONE,
    };
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::PointerMoved(start), button(start, true)],
    );
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::PointerMoved(start - Vec2::new(0., 20.))],
    );
    // Release can carry its final position without a separate move event.
    frame(&mut app, &ctx, vec![button(start, false)]);
    assert_eq!(
        egui::containers::panel::PanelState::load(&ctx, id)
            .unwrap()
            .outer_rect
            .height(),
        200.
    );
    assert_eq!(app.page_inspector.preferences.get(key.as_deref()), 200);
    assert_eq!(app.editor.document, original);
    assert!(app.editor.undo_name().is_none());
}

fn alternate_layout_frame(app: &mut WireframeApp, ctx: &egui::Context, extent: f32) -> f32 {
    let key = app.version_preferences_key();
    let mut visible = 0.;
    for _ in 0..3 {
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400., 1000.))),
                ..Default::default()
            },
            |ui| {
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(Rect::from_min_size(Pos2::ZERO, Vec2::new(300., extent))),
                    |ui| {
                        assert_eq!(ui.available_height(), extent);
                        let height = app.page_inspector.preferences.get(key.as_deref());
                        let id = egui::Id::new(("page-alternates-panel", &key, height));
                        app.alternate_panel(ui);
                        visible = egui::containers::panel::PanelState::load(ctx, id)
                            .unwrap()
                            .outer_rect
                            .height();
                    },
                );
            },
        )
        .drop_without_applying_deltas();
    }
    visible
}

#[test]
fn alternate_divider_resizes_proportionally_and_persists_layout_changes() {
    for steps in [
        [(737., 400_u16), (857., 400), (737., 344)],
        [(737., 300_u16), (857., 349), (737., 300)],
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (ctx, mut app) = setup();
        app.file = Some(dir.path().join("layout-size.wireframe"));
        app.load_panel_preferences(dir.path());
        let key = app.version_preferences_key();
        app.page_inspector
            .preferences
            .remember(key.as_deref(), steps[0].1)
            .unwrap();
        let original = app.editor.document.clone();
        for (extent, expected) in steps {
            let visible = alternate_layout_frame(&mut app, &ctx, extent);
            assert_eq!(visible, expected as f32, "extent {extent}");
            assert_eq!(app.page_inspector.preferences.get(key.as_deref()), expected);
            let mut reopened = preferences::Preferences::default();
            reopened.load(dir.path()).unwrap();
            assert_eq!(reopened.get(key.as_deref()), expected);
            assert_eq!(app.editor.document, original);
            assert!(app.editor.undo_name().is_none());
        }
    }
}

#[test]
fn alternate_layout_keeps_project_sizes_independent_and_survives_unusable_heights() {
    let dir = tempfile::tempdir().unwrap();
    let (ctx, mut app) = setup();
    app.load_panel_preferences(dir.path());
    let first = dir.path().join("first.wireframe");
    let second = dir.path().join("second.wireframe");
    app.file = Some(first.clone());
    let first_key = app.version_preferences_key();
    app.page_inspector
        .preferences
        .remember(first_key.as_deref(), 300)
        .unwrap();
    assert_eq!(alternate_layout_frame(&mut app, &ctx, 737.), 300.);
    assert_eq!(alternate_layout_frame(&mut app, &ctx, 200.), 40.);
    assert_eq!(
        app.page_inspector.preferences.get(first_key.as_deref()),
        300
    );
    assert_eq!(alternate_layout_frame(&mut app, &ctx, 737.), 300.);
    app.file = Some(second.clone());
    let second_key = app.version_preferences_key();
    app.page_inspector
        .preferences
        .remember(second_key.as_deref(), 200)
        .unwrap();
    assert_eq!(alternate_layout_frame(&mut app, &ctx, 857.), 200.);
    app.file = Some(first);
    assert_eq!(alternate_layout_frame(&mut app, &ctx, 737.), 300.);
    app.page_inspector.clear();
    assert_eq!(alternate_layout_frame(&mut app, &ctx, 737.), 300.);
    app.file = Some(second);
    app.page_inspector.clear();
    assert_eq!(alternate_layout_frame(&mut app, &ctx, 200.), 40.);
    assert_eq!(alternate_layout_frame(&mut app, &ctx, 857.), 200.);
    let mut reopened = preferences::Preferences::default();
    reopened.load(dir.path()).unwrap();
    assert_eq!(reopened.get(first_key.as_deref()), 300);
    assert_eq!(reopened.get(second_key.as_deref()), 200);
    assert!(app.editor.undo_name().is_none());
}

#[test]
fn visible_alternate_divider_double_click_preserves_size_preferences_and_history() {
    let dir = tempfile::tempdir().unwrap();
    let (ctx, mut app) = setup();
    app.file = Some(std::path::PathBuf::from("divider-viewport-study.wireframe"));
    app.load_panel_preferences(dir.path());
    let key = app.version_preferences_key();
    app.page_inspector
        .preferences
        .remember(key.as_deref(), 300)
        .unwrap();
    let original = app.editor.document.clone();
    let id = egui::Id::new(("page-alternates-panel", &key, 300_u16));
    for _ in 0..3 {
        frame(&mut app, &ctx, vec![]);
    }
    let initial = egui::containers::panel::PanelState::load(&ctx, id)
        .unwrap()
        .outer_rect;
    assert!((initial.height() - 300.).abs() <= 1.);
    let boundary = egui::pos2(initial.center().x, initial.min.y);
    for _ in 0..2 {
        for pressed in [true, false] {
            frame(
                &mut app,
                &ctx,
                vec![
                    egui::Event::PointerMoved(boundary),
                    egui::Event::PointerButton {
                        pos: boundary,
                        pressed,
                        button: egui::PointerButton::Primary,
                        modifiers: Modifiers::NONE,
                    },
                ],
            );
        }
    }
    assert!(ctx.input(|i| {
        i.pointer
            .button_double_clicked(egui::PointerButton::Primary)
    }));
    let after_click = egui::containers::panel::PanelState::load(&ctx, id)
        .unwrap()
        .outer_rect;
    assert!((after_click.height() - 300.).abs() <= 1.);
    assert_eq!(app.page_inspector.preferences.get(key.as_deref()), 300);
    assert_eq!(app.editor.document, original);
    assert!(app.editor.undo_name().is_none());
    let mut reopened = preferences::Preferences::default();
    reopened.load(dir.path()).unwrap();
    assert_eq!(reopened.get(key.as_deref()), 300);
}

#[test]
fn invalid_panel_settings_are_preserved_while_session_sizing_remains_available() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("panel-settings.json");
    for contents in [
        "unreadable",
        r#"{"default_height":200,"projects":{"first":401}}"#,
    ] {
        std::fs::write(&path, contents).unwrap();
        let mut prefs = preferences::Preferences::default();
        assert!(prefs.load(dir.path()).is_err());
        assert!(prefs.remember(Some("first"), 300).is_err());
        assert_eq!(prefs.get(Some("first")), 300);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), contents);
    }
}

#[test]
fn extract_text_native_command_preserves_pending_page_and_source_notes_and_redo() {
    let _guard = crate::app::screenshot_text::TEST_RECOGNITION
        .lock()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !crate::app::screenshot_text::available() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    for source_notes in [false, true] {
        let (ctx, mut app) = setup();
        if source_notes {
            let library = app.editor.create_library("Shared controls").unwrap();
            app.editor
                .create_empty_source(library, "Source", wireframe::document::Point::default())
                .unwrap();
        }
        app.editor.edit("Change title", |editor| {
            editor.document.title = "Later title".into()
        });
        app.editor.undo();
        app.begin_page_notes(Rect::from_min_size(Pos2::ZERO, Vec2::new(500., 500.)));
        app.page_inspector.draft.as_mut().unwrap().text =
            "Pending notes that must stay editable".into();
        let before = app.editor.document.clone();
        let redo = app.editor.redo_name().map(str::to_owned);
        assert!(!app.can_extract_screenshot());
        assert!(!app.command_enabled(&ctx, menus::Command::ExtractText));
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::AppCommand(menus::Command::ExtractText as u64)],
        );
        assert_eq!(
            app.page_inspector.draft.as_ref().unwrap().text,
            "Pending notes that must stay editable"
        );
        assert_eq!(app.editor.document, before);
        assert_eq!(app.editor.redo_name(), redo.as_deref());
        assert!(app.screenshot_text.is_none());
        assert!(app.error.is_none());
    }
}
