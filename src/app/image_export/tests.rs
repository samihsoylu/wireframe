// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

#[test]
fn image_export_rejects_the_open_project_before_starting_a_worker() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("editable.wireframe");
    ProjectStore.save(&project, &app.editor.document).unwrap();
    let before = std::fs::read(&project).unwrap();
    app.file = Some(project.clone());
    let result = app.start_image_export(Request::new(1), Destination::File(project.clone()));
    assert!(result.unwrap_err().contains("open project file"));
    assert!(app.image_export.job.is_none());
    assert_eq!(std::fs::read(project).unwrap(), before);
}
use std::time::Instant;
use wireframe::document::{Content, Definition, Node, Point, Size};

fn wait(app: &mut WireframeApp, ctx: &egui::Context) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !app.image_export.job.as_ref().unwrap().is_finished() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    app.poll_image_export(ctx);
}
#[test]
fn options_follow_valid_scope_and_component_workspace() {
    let mut editor = Editor::default();
    editor.add(Kind::Button, 10., 20.);
    let mut options = Options {
        range: Range::Current,
        scope: Scope::Selection,
        ..Default::default()
    };
    let draft = Draft::new(&editor, None, options.clone());
    assert!(matches!(
        draft.request().unwrap().scope,
        SceneScope::Selection(_)
    ));
    options.range = Range::Chosen;
    options.chosen.clear();
    let draft = Draft::new(&editor, None, options.clone());
    assert!(draft.request().is_err());
    assert_eq!(draft.options.scope, Scope::Whole);
    editor.document.definitions.push(Definition {
        id: 100,
        name: "Card".into(),
        content: Content::default(),
    });
    editor.document.next_id = 101;
    editor.document.organize_components().unwrap();
    editor.definition = Some(100);
    editor.selection.clear();
    let draft = Draft::new(&editor, None, options);
    assert!(draft.component);
    assert_eq!(draft.selection(), Selection::Current(100));
    assert_eq!(draft.chosen()[0].page_name, "Card");
    assert!(draft.request().is_ok());
}
#[test]
fn preferences_roundtrip_separate_project_ids_and_preserve_unreadable_file() {
    let dir = tempfile::tempdir().unwrap();
    let mut preferences = preferences::Preferences::default();
    preferences.load(dir.path()).unwrap();
    let options = Options {
        range: Range::Chosen,
        chosen: [4, 5].into(),
        transparent: true,
        ..Default::default()
    };
    preferences.remember(Some("project-a"), options).unwrap();
    let mut restored = preferences::Preferences::default();
    restored.load(dir.path()).unwrap();
    assert_eq!(restored.get(Some("project-a")).chosen, [4, 5].into());
    assert!(restored.get(Some("project-b")).chosen.is_empty());
    assert!(restored.get(Some("project-b")).transparent);
    let file = dir.path().join("image-export-settings.json");
    std::fs::write(&file, b"invalid settings").unwrap();
    assert!(restored.load(dir.path()).is_err());
    assert!(
        restored
            .remember(Some("project-a"), Options::default())
            .is_err()
    );
    assert_eq!(std::fs::read(&file).unwrap(), b"invalid settings");
}
#[test]
fn options_block_document_shortcuts_and_render_without_mutating_project() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 10., 20.);
    let before = app.editor.document.clone();
    app.open_image_export();
    assert!(app.canvas_input_blocked());
    let event = egui::Event::Key {
        key: Key::D,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    };
    let output = ctx.run_ui(
        egui::RawInput {
            events: vec![event],
            ..Default::default()
        },
        |ui| {
            app.shortcuts(ui.ctx());
            app.image_export_dialogs(ui.ctx());
        },
    );
    output.drop_without_applying_deltas();
    assert_eq!(app.editor.document, before);
    app.image_export.draft = None;
    assert!(!app.canvas_input_blocked());
}
#[test]
fn failed_export_preserves_output_and_cancels_deferred_quit() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prior.png");
    std::fs::write(&path, b"prior image").unwrap();
    let request = Request {
        selection: Selection::Set(Default::default()),
        ..Request::new(1)
    };
    app.start_image_export(request, Destination::File(path.clone()))
        .unwrap();
    app.request(Pending::Quit, &ctx);
    assert!(app.image_export.quit_after);
    wait(&mut app, &ctx);
    assert!(!app.image_export.quit_after);
    assert!(!app.quit_allowed);
    assert!(app.pending.is_none());
    assert!(app.image_export.show_report);
    assert!(matches!(
        app.image_export.report.as_ref().unwrap().outcome,
        Outcome::Failed(_)
    ));
    assert_eq!(std::fs::read(path).unwrap(), b"prior image");
}
#[test]
fn conflict_cancel_keeps_existing_file_and_reports_no_completed_output() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prior.png");
    std::fs::write(&path, b"prior image").unwrap();
    app.start_image_export(Request::new(1), Destination::File(path.clone()))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        app.poll_image_export(&ctx);
        if matches!(
            app.image_export.status.as_ref().map(|s| &s.phase),
            Some(Phase::Conflict { .. })
        ) {
            break;
        }
        assert!(app.image_export.job.is_some() && Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(app.canvas_input_blocked());
    app.image_export.job.as_ref().unwrap().cancel();
    wait(&mut app, &ctx);
    let report = app.image_export.report.as_ref().unwrap();
    assert_eq!(report.outcome, Outcome::Cancelled);
    assert_eq!(report.exported_pages, 0);
    assert_eq!(report.remaining_pages(), 1);
    assert_eq!(std::fs::read(path).unwrap(), b"prior image");
}
#[test]
fn component_export_saves_its_surface_and_keeps_document_unchanged() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.document.definitions.push(Definition {
        id: 100,
        name: "Card".into(),
        content: Content {
            nodes: vec![Node::control(
                101,
                "rectangle",
                Point::new(20., 30.),
                Size::fixed(80., 40.),
            )],
            paint_order: vec![],
        },
    });
    app.editor.document.next_id = 102;
    app.editor.document.organize_components().unwrap();
    app.editor.definition = Some(100);
    let before = app.editor.document.clone();
    let draft = Draft::new(&app.editor, None, Options::default());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Card.png");
    app.start_image_export(draft.request().unwrap(), Destination::File(path.clone()))
        .unwrap();
    wait(&mut app, &ctx);
    assert_eq!(
        app.image_export.report.as_ref().unwrap().outcome,
        Outcome::Completed
    );
    assert_eq!(app.editor.document, before);
    let image = image::open(path).unwrap();
    assert_eq!((image.width(), image.height()), (160, 80));
}

#[test]
fn pointer_option_changes_survive_following_frames_and_reopening() {
    fn frame(
        ctx: &egui::Context,
        app: &mut WireframeApp,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1360., 900.))),
            events,
            ..Default::default()
        };
        app.order_text_input(ctx, &mut raw);
        ctx.run_ui(raw, |ui| {
            app.shortcuts(ui.ctx());
            app.toolbar(ui);
            app.sidebar(ui);
            app.inspector(ui);
            app.canvas(ui);
            app.image_export_dialogs(ui.ctx());
        })
    }
    fn label(shape: &egui::epaint::Shape, wanted: &str) -> Option<Pos2> {
        match shape {
            egui::epaint::Shape::Text(text) if text.galley.job.text == wanted => {
                Some(text.visual_bounding_rect().center())
            }
            egui::epaint::Shape::Vec(shapes) => {
                shapes.iter().find_map(|shape| label(shape, wanted))
            }
            _ => None,
        }
    }
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 10., 20.);
    app.image_export
        .preferences
        .remember(
            None,
            Options {
                double: false,
                transparent: true,
                ..Default::default()
            },
        )
        .unwrap();
    for _ in 0..2 {
        app.open_image_export();
        for _ in 0..3 {
            frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
        }
        for text in ["2×", "White"] {
            let output = frame(&ctx, &mut app, vec![]);
            let pos = output
                .shapes
                .iter()
                .find_map(|shape| label(&shape.shape, text))
                .unwrap();
            output.drop_without_applying_deltas();
            for pressed in [true, false] {
                frame(
                    &ctx,
                    &mut app,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Modifiers::NONE,
                        },
                    ],
                )
                .drop_without_applying_deltas();
            }
        }
        for _ in 0..3 {
            frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
        }
        let options = &app.image_export.draft.as_ref().unwrap().options;
        assert!(options.double, "Resolution click must select 2x");
        assert!(!options.transparent, "Background click must select White");
        app.image_export.draft = None;
        frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
    }
}

#[test]
fn remembered_versions_are_captured_for_active_and_current_image_export() {
    let mut editor = Editor::default();
    let first = editor.page;
    let alt = editor.create_version("Compact").unwrap();
    editor.add_page();
    let second = editor.page;
    editor.create_version("Detailed").unwrap();
    editor.switch_version(second, None).unwrap();
    editor.switch_page(first);
    let draft = Draft::new(
        &editor,
        None,
        Options {
            range: Range::Current,
            ..Default::default()
        },
    );
    assert_eq!(draft.selection(), Selection::Current(alt));
    assert_eq!(draft.active, [alt, second].into());
    editor.switch_version(first, None).unwrap();
    assert_eq!(draft.active, [alt, second].into());
    assert_eq!(draft.current, alt);
    let plan = raster::plan::Prepared::new(&editor.document, &draft.request().unwrap()).unwrap();
    assert_eq!(plan.entries().len(), 1);
}
