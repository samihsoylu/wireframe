// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::document::{Node, Point, Size};
use wireframe::images::{Prepared, decoded_styled};
fn fixture(ctx: &egui::Context) -> WireframeApp {
    let mut app = WireframeApp::workspace(ctx, None);
    let mut project = Project {
        next_id: 4,
        ..Project::default()
    };
    project.pages[0].content.nodes = vec![
        Node::control(2, "image", Point::new(50., 50.), Size::default()),
        Node::control(3, "image", Point::new(250., 50.), Size::default()),
    ];
    app.editor = Editor::new(project);
    app.editor.select(2, false);
    app
}
fn preview(color: u8) -> Preview {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(12, 8, image::Rgba([color, 40, 90, 128]))
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let image = Prepared::from_bytes("picture.png", &bytes.into_inner()).unwrap();
    Preview {
        original: decoded_styled(&image.bytes, 0, false).unwrap(),
        sketch: decoded_styled(&image.bytes, 0, true).unwrap(),
        image,
    }
}
fn frame(
    app: &mut WireframeApp,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900., 800.))),
            events,
            ..Default::default()
        },
        |ui| {
            app.shortcuts(ui.ctx());
            app.web_image_dialog(ui.ctx());
        },
    );
    out.textures_delta.clear();
    out
}
fn click(app: &mut WireframeApp, ctx: &egui::Context, label: &str) {
    let _ = frame(app, ctx, vec![]);
    let out = frame(app, ctx, vec![]);
    let bounds = out
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, n)| n.role() == egui::accesskit::Role::Button && n.label() == Some(label))
        .unwrap_or_else(|| panic!("missing {label}"))
        .1
        .bounds()
        .unwrap();
    let pos = Pos2::new(
        (bounds.x0 + bounds.x1) as f32 / 2.,
        (bounds.y0 + bounds.y1) as f32 / 2.,
    );
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
fn cancellation_blocks_delete_and_preserves_redo_and_late_result() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = fixture(&ctx);
    app.editor
        .import_web_image(vec![2], preview(30).image, ImportChoice::KeepBoth, false)
        .unwrap();
    app.editor.undo();
    let before = app.editor.document.clone();
    let redo = app.editor.redo_name().map(str::to_owned);
    app.start_web_image(vec![2]);
    let cancel = app.web_image.as_ref().unwrap().cancel.clone();
    let (sender, receiver) = mpsc::channel();
    app.web_image.as_mut().unwrap().receiver = Some(receiver);
    frame(&mut app, &ctx, vec![key(Key::Delete)]);
    assert_eq!(app.editor.document, before);
    frame(&mut app, &ctx, vec![key(Key::Escape)]);
    assert!(app.web_image.is_none());
    assert!(cancel.load(Ordering::Relaxed));
    assert!(sender.send(Ok(preview(80))).is_err());
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.redo_name(), redo.as_deref());
}
#[test]
fn preview_load_is_one_atomic_history_entry_for_captured_targets() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = fixture(&ctx);
    let before = app.editor.document.clone();
    app.start_web_image(vec![2, 3]);
    app.editor.select(3, false);
    let (sender, receiver) = mpsc::channel();
    app.web_image.as_mut().unwrap().receiver = Some(receiver);
    sender.send(Ok(preview(20))).unwrap();
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.editor.document, before);
    app.web_image.as_mut().unwrap().sketch = true;
    click(&mut app, &ctx, "Load");
    assert!(app.web_image.is_none());
    let after = app.editor.document.clone();
    assert_eq!(after.assets.len(), 1);
    for node in &after.pages[0].content.nodes {
        assert_eq!(
            node.properties.get("image"),
            Some(&wireframe::document::Value::Asset(after.assets[0].id))
        );
        assert_eq!(
            node.properties.get("sketch_filter"),
            Some(&wireframe::document::Value::Bool(true))
        );
    }
    assert_eq!(app.editor.undo_name(), Some("Import image"));
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    assert!(app.editor.undo_name().is_none());
    app.editor.redo();
    assert_eq!(app.editor.document, after);
    assert_eq!(
        wireframe::document::decode(&after.encode().unwrap()).unwrap(),
        after
    );
}
#[test]
fn conflict_cancel_and_all_choices_keep_atomic_history() {
    for choice in ["Cancel", "Use new", "Use existing", "Keep both"] {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = fixture(&ctx);
        app.editor
            .import_web_image(vec![2], preview(30).image, ImportChoice::KeepBoth, false)
            .unwrap();
        let before = app.editor.document.clone();
        app.start_web_image(vec![3]);
        app.web_image.as_mut().unwrap().preview = Some(preview(200));
        click(&mut app, &ctx, "Load");
        assert!(app.web_image.as_ref().unwrap().conflict);
        assert_eq!(app.editor.document, before);
        click(&mut app, &ctx, choice);
        if choice == "Cancel" {
            assert_eq!(app.editor.document, before);
            continue;
        }
        let after = app.editor.document.clone();
        assert_eq!(
            after.assets.len(),
            if choice == "Keep both" { 2 } else { 1 }
        );
        if choice == "Use existing" {
            assert_eq!(after.assets, before.assets);
        }
        if choice == "Use new" {
            assert_ne!(after.assets[0].bytes, before.assets[0].bytes);
        }
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        app.editor.redo();
        assert_eq!(app.editor.document, after);
    }
}
#[test]
fn stale_project_or_surface_drops_pending_preview_without_mutation() {
    for change in 0..3 {
        let ctx = egui::Context::default();
        let mut app = fixture(&ctx);
        app.start_web_image(vec![2]);
        app.web_image.as_mut().unwrap().preview = Some(preview(20));
        match change {
            0 => app.editor.document.title = "Changed".into(),
            1 => app.editor = Editor::new(app.editor.document.clone()),
            _ => app.editor.context.push(99),
        }
        let before = app.editor.document.clone();
        frame(&mut app, &ctx, vec![]);
        assert!(app.web_image.is_none());
        assert_eq!(app.editor.document, before);
        assert!(app.error.as_ref().unwrap().contains("changed"));
    }
}
#[test]
fn cancelled_search_cannot_deliver_late_results_or_change_redo() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = fixture(&ctx);
    app.editor
        .import_web_image(vec![2], preview(30).image, ImportChoice::KeepBoth, false)
        .unwrap();
    app.editor.undo();
    let before = app.editor.document.clone();
    let redo = app.editor.redo_name().map(str::to_owned);
    app.start_image_search(vec![2]);
    let (sender, receiver) = mpsc::channel();
    let review = app.web_image.as_mut().unwrap();
    let cancelled = review.search_cancel.clone();
    review.search_receiver = Some(receiver);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![key(Key::Escape)]);
    assert!(app.web_image.is_none());
    assert!(cancelled.load(Ordering::Relaxed));
    assert!(
        sender
            .send(Ok(Results {
                hits: vec![],
                next: None
            }))
            .is_err()
    );
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.redo_name(), redo.as_deref());
}
#[test]
fn search_preview_credit_load_save_and_history_are_one_atomic_change() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = fixture(&ctx);
    let before = app.editor.document.clone();
    app.start_image_search(vec![2, 3]);
    let credit = ImageAttribution {
        title: "Canal.jpg".into(),
        creator: "Example creator".into(),
        license: "CC BY 4.0".into(),
        license_url: "https://creativecommons.org/licenses/by/4.0/".into(),
        source_url: "https://commons.wikimedia.org/wiki/File:Canal.jpg".into(),
    };
    let (sender, receiver) = mpsc::channel();
    let review = app.web_image.as_mut().unwrap();
    review.attribution = Some(credit.clone());
    review.receiver = Some(receiver);
    sender.send(Ok(preview(40))).unwrap();
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.editor.document, before);
    click(&mut app, &ctx, "Copy image credit");
    assert_eq!(app.editor.document, before);
    click(&mut app, &ctx, "Load");
    let after = app.editor.document.clone();
    assert_eq!(after.assets[0].attribution, Some(credit));
    assert_eq!(
        wireframe::document::decode(&after.encode().unwrap()).unwrap(),
        after
    );
    assert_eq!(app.editor.undo_name(), Some("Import image"));
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    app.editor.redo();
    assert_eq!(app.editor.document, after);
}
#[test]
fn returning_to_address_mode_discards_search_download_and_credit() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = fixture(&ctx);
    let before = app.editor.document.clone();
    app.start_image_search(vec![2]);
    let review = app.web_image.as_mut().unwrap();
    let (sender, receiver) = mpsc::channel();
    review.receiver = Some(receiver);
    let cancel = review.cancel.clone();
    let _ = frame(&mut app, &ctx, vec![]);
    // Selectable tabs expose a button role through egui AccessKit.
    click(&mut app, &ctx, "Image address");
    let review = app.web_image.as_ref().unwrap();
    assert!(!review.search_mode);
    assert!(review.receiver.is_none());
    assert!(review.preview.is_none());
    assert!(review.attribution.is_none());
    assert!(cancel.load(Ordering::Relaxed));
    assert!(sender.send(Ok(preview(80))).is_err());
    assert_eq!(app.editor.document, before);
}

fn credit(creator: &str) -> ImageAttribution {
    ImageAttribution {
        title: "Picture".into(),
        creator: creator.into(),
        license: "CC BY 4.0".into(),
        license_url: "https://creativecommons.org/licenses/by/4.0/".into(),
        source_url: "https://commons.wikimedia.org/wiki/File:Picture.jpg".into(),
    }
}
#[test]
fn same_pixels_with_different_credits_require_choice_and_preserve_exact_history() {
    for choice in ["Cancel", "Use new", "Use existing", "Keep both"] {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = fixture(&ctx);
        let mut original = preview(30).image;
        original.attribution = Some(credit("Original"));
        app.editor
            .import_web_image(vec![2], original, ImportChoice::KeepBoth, false)
            .unwrap();
        let before = app.editor.document.clone();
        app.start_image_search(vec![3]);
        let mut incoming = preview(30);
        incoming.image.attribution = Some(credit("Selected"));
        app.web_image.as_mut().unwrap().attribution = Some(credit("Selected"));
        app.web_image.as_mut().unwrap().preview = Some(incoming);
        click(&mut app, &ctx, "Load");
        assert!(app.web_image.as_ref().unwrap().conflict);
        assert_eq!(app.editor.document, before);
        click(&mut app, &ctx, choice);
        if choice == "Cancel" {
            assert_eq!(app.editor.document, before);
            continue;
        }
        let after = app.editor.document.clone();
        match choice {
            "Use new" => assert_eq!(after.assets[0].attribution, Some(credit("Selected"))),
            "Use existing" => assert_eq!(after.assets[0].attribution, Some(credit("Original"))),
            _ => {
                assert_eq!(after.assets[0].attribution, Some(credit("Original")));
                assert_eq!(after.assets[1].attribution, Some(credit("Selected")));
            }
        }
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        app.editor.redo();
        assert_eq!(app.editor.document, after);
    }
}
#[test]
fn cancelled_generations_and_reopened_dialogs_keep_a_fixed_live_request_bound() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        time::Duration,
    };
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.editor
        .import_web_image(vec![2], preview(10).image, ImportChoice::KeepBoth, false)
        .unwrap();
    app.editor.undo();
    let before = app.editor.document.clone();
    let redo = app.editor.redo_name().map(str::to_owned);
    assert_eq!(NETWORK_WORKERS.load(Ordering::Relaxed), 0);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let (arrived_tx, arrived_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let mut connections = vec![];
        for _ in 0..MAX_NETWORK_WORKERS {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut headers = vec![];
            while !headers.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                headers.push(byte[0]);
            }
            connections.push(stream);
            arrived_tx.send(()).unwrap();
        }
        release_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        for mut stream in connections {
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .unwrap();
        }
    });
    let mut workers = vec![];
    let mut cancelled = vec![];
    for name in ["search", "thumbnail", "thumbnail", "preview"] {
        let slot = WorkerSlot::acquire(&NETWORK_WORKERS).unwrap();
        let address = address.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        cancelled.push(cancel.clone());
        let (sender, receiver) = mpsc::channel();
        drop(receiver); // Closing a generation discards its receiver.
        workers.push(
            std::thread::Builder::new()
                .name(name.into())
                .spawn(move || {
                    let _slot = slot;
                    let agent: ureq::Agent = ureq::Agent::config_builder()
                        .timeout_global(Some(Duration::from_secs(3)))
                        .build()
                        .into();
                    let result = agent
                        .get(&address)
                        .call()
                        .map(|_| ())
                        .map_err(|e| e.to_string());
                    if !cancel.load(Ordering::Relaxed) {
                        let _ = sender.send(result);
                    }
                })
                .unwrap(),
        );
    }
    for _ in 0..MAX_NETWORK_WORKERS {
        arrived_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    }
    for cancel in cancelled {
        cancel.store(true, Ordering::Relaxed);
    }
    for _ in 0..32 {
        app.start_image_search(vec![2]);
        app.web_image = None;
        assert!(WorkerSlot::acquire(&NETWORK_WORKERS).is_none());
        assert_eq!(NETWORK_WORKERS.load(Ordering::Relaxed), MAX_NETWORK_WORKERS);
    }
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.redo_name(), redo.as_deref());
    release_tx.send(()).unwrap();
    for worker in workers {
        worker.join().unwrap();
    }
    server.join().unwrap();
    assert_eq!(NETWORK_WORKERS.load(Ordering::Relaxed), 0);
    assert!(WorkerSlot::acquire(&NETWORK_WORKERS).is_some());
}
