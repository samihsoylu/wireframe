// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::document::{Asset, Node, Point, Project, Size, Value};

fn fixture(ctx: &egui::Context) -> WireframeApp {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !screenshot_text::available() {
        assert!(
            std::time::Instant::now() < deadline,
            "previous owned recognition must finish"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let mut app = WireframeApp::workspace(ctx, None);
    let mut project = Project {
        font: None,
        next_id: 5,
        ..Project::default()
    };
    let mut image = Node::control(2, "image", Point::new(20., 20.), Size::default());
    image.properties.insert("image".into(), Value::Asset(3));
    project.pages[0].content.nodes = vec![
        image,
        Node::control(4, "paragraph", Point::new(200., 20.), Size::default()),
    ];
    project.assets.push(Asset {
        id: 3,
        trashed: false,
        attribution: None,
        name: "Screenshot".into(),
        media_type: "image/png".into(),
        bytes: include_bytes!("../../../vendor/wireframe-ocr/tests/fixtures/screenshot.png")
            .to_vec(),
    });
    app.editor = Editor::new(project);
    app.editor.edit("Change title", |editor| {
        editor.document.title = "A later title".into()
    });
    app.editor.undo();
    app
}
fn frame(
    app: &mut WireframeApp,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900., 700.))),
            events,
            ..Default::default()
        },
        |ui| {
            app.shortcuts(ui.ctx());
            app.screenshot_text_dialog(ui.ctx());
        },
    );
    output.textures_delta.clear();
    output
}
fn key(key: Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}
#[test]
fn asset_extraction_and_cancel_preserve_complete_project_and_pending_redo() {
    let _guard = screenshot_text::TEST_RECOGNITION.lock().unwrap();
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.editor.select(2, false);
    let before = app.editor.document.clone();
    let redo = app.editor.redo_name().map(str::to_owned);
    app.extract_asset_text(&ctx, 3);
    assert!(app.screenshot_text.is_some());
    assert!(app.canvas_input_blocked());
    frame(&mut app, &ctx, vec![key(Key::Delete)]);
    frame(&mut app, &ctx, vec![key(Key::Escape)]);
    assert!(app.screenshot_text.is_none());
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.redo_name(), redo.as_deref());
    assert!(app.editor.undo_name().is_none());
}
#[test]
fn unfinished_canvas_text_disables_extraction_without_accepting_or_losing_draft() {
    let _guard = screenshot_text::TEST_RECOGNITION.lock().unwrap();
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.begin_text_edit(4);
    app.text_edit.as_mut().unwrap().1 = "Unfinished draft".into();
    let before = app.editor.document.clone();
    let redo = app.editor.redo_name().map(str::to_owned);
    assert!(!app.can_extract_screenshot());
    assert!(!app.command_enabled(&ctx, menus::Command::ExtractText));
    app.extract_asset_text(&ctx, 3);
    assert!(app.screenshot_text.is_none());
    assert_eq!(app.text_edit.as_ref().unwrap().1, "Unfinished draft");
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.redo_name(), redo.as_deref());
}
#[test]
fn session_reset_discards_preview_and_missing_asset_does_not_start_it() {
    let _guard = screenshot_text::TEST_RECOGNITION.lock().unwrap();
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.extract_asset_text(&ctx, 999);
    assert!(app.screenshot_text.is_none());
    app.extract_asset_text(&ctx, 3);
    assert!(app.screenshot_text.is_some());
    app.reset_session_identity();
    assert!(app.screenshot_text.is_none());
}
#[test]
fn screenshot_file_read_is_bounded_and_never_creates_a_project_asset() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.png");
    std::fs::write(&path, b"test image bytes").unwrap();
    assert_eq!(read_screenshot(&path).unwrap(), b"test image bytes");
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(wireframe_ocr::MAX_BYTES as u64 + 1)
        .unwrap();
    assert!(read_screenshot(&path).unwrap_err().contains("32 MB"));
    assert!(
        read_screenshot(&dir.path().join("missing.png"))
            .unwrap_err()
            .contains("Couldn't open")
    );
}

#[test]
fn completed_local_recognition_copy_and_close_leave_project_and_redo_exact() {
    let _guard = screenshot_text::TEST_RECOGNITION.lock().unwrap();
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = fixture(&ctx);
    let before = app.editor.document.clone();
    let redo = app.editor.redo_name().map(str::to_owned);
    app.extract_asset_text(&ctx, 3);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let bounds = loop {
        let output = frame(&mut app, &ctx, vec![]);
        if let Some(bounds) = output
            .platform_output
            .accesskit_update
            .as_ref()
            .and_then(|update| {
                update
                    .nodes
                    .iter()
                    .find(|(_, node)| node.label() == Some("Copy text") && !node.is_disabled())
                    .and_then(|(_, node)| node.bounds())
            })
        {
            break bounds;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "recognition should finish for the readable fixture"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let pos = Pos2::new(
        (bounds.x0 + bounds.x1) as f32 / 2.,
        (bounds.y0 + bounds.y1) as f32 / 2.,
    );
    let mut copied = None;
    for pressed in [true, false] {
        let output = frame(
            &mut app,
            &ctx,
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
        for command in output.platform_output.commands {
            if let egui::OutputCommand::CopyText(text) = command {
                copied = Some(text);
            }
        }
    }
    let text = copied.expect("Copy text publishes the actual recognized text");
    for expected in [
        "Project overview",
        "Welcome to Wireframe",
        "Draft screens: 12",
        "Ready for review",
        "Create project",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.redo_name(), redo.as_deref());
    frame(&mut app, &ctx, vec![key(Key::Escape)]);
    assert!(app.screenshot_text.is_none());
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.redo_name(), redo.as_deref());
}

#[derive(Debug)]
struct ProjectDrop(std::path::PathBuf);
impl egui::DroppedFile for ProjectDrop {
    fn path(&self) -> &std::path::Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|error| error.to_string())
    }
}
#[test]
fn production_drop_dispatch_refuses_active_extraction_and_keeps_normal_open_available() {
    let _guard = screenshot_text::TEST_RECOGNITION.lock().unwrap();
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dropped.wireframe");
    let target = Project {
        font: None,
        title: "Dropped example".into(),
        ..Project::default()
    };
    std::fs::write(&path, target.encode().unwrap()).unwrap();
    let before = app.editor.document.clone();
    let redo = app.editor.redo_name().map(str::to_owned);
    let session = app.editor.clipboard_session().to_owned();
    app.extract_asset_text(&ctx, 3);
    assert!(app.screenshot_text.is_some());
    let input = || egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900., 700.))),
        dropped_files: vec![std::sync::Arc::new(ProjectDrop(path.clone()))],
        ..Default::default()
    };
    let mut output = ctx.run_ui(input(), |ui| {
        app.image_import_dialog(ui.ctx());
        app.prepare_page_notes(ui.ctx());
        app.open_dropped_project(ui.ctx());
    });
    output.textures_delta.clear();
    assert!(app.screenshot_text.is_some());
    assert!(app.startup_open.is_none());
    assert!(app.pending.is_none());
    assert!(app.file.is_none());
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.redo_name(), redo.as_deref());
    assert_eq!(app.editor.clipboard_session(), session);
    app.screenshot_text = None;
    let mut output = ctx.run_ui(input(), |ui| app.open_dropped_project(ui.ctx()));
    output.textures_delta.clear();
    assert!(
        app.startup_open.is_some()
            || app.pending.is_some()
            || app.file.as_deref() == Some(path.as_path())
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while app.startup_open.is_some() {
        app.poll_startup_open();
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(app.error.is_none(), "{:?}", app.error);
}
