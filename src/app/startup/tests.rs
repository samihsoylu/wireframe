// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::working_copy::WorkingCopies;

pub(super) fn setup() -> (egui::Context, WireframeApp) {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let app = WireframeApp::workspace(&ctx, None);
    (ctx, app)
}
pub(super) fn drain(app: &mut WireframeApp) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.startup_open.is_some() {
        app.poll_startup_open();
        assert!(Instant::now() < deadline, "opening did not complete");
        std::thread::sleep(Duration::from_millis(2));
    }
}
pub(super) fn frame(
    app: &mut WireframeApp,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
) -> Option<Pos2> {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(940., 620.))),
            events,
            ..Default::default()
        },
        |ui| {
            app.poll_startup_open();
            app.shortcuts(ctx);
            ui.label("Workspace remains drawable");
            app.dialogs(ctx);
        },
    );
    output.textures_delta.clear();
    output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find_map(|(_, node)| {
            if node.label() != Some("Cancel") {
                return None;
            }
            let r = node.bounds()?;
            Some(egui::pos2(
                ((r.x0 + r.x1) / 2.) as f32,
                ((r.y0 + r.y1) / 2.) as f32,
            ))
        })
}
#[test]
fn stalled_read_keeps_cancel_reachable_and_never_applies_a_late_result() {
    let (ctx, mut app) = setup();
    let original = app.editor.document.clone();
    let (release, wait) = mpsc::channel();
    let (started, ready) = mpsc::channel();
    let (finished, done) = mpsc::channel();
    app.startup_open = Some(
        Opening::spawn("Slow project.wireframe".into(), ctx.clone(), move || {
            started.send(()).unwrap();
            wait.recv().unwrap();
            let p = Project {
                title: "Late result".into(),
                ..Default::default()
            };
            finished.send(()).unwrap();
            Ok(Prepared::Plain(Box::new(prepare_editor(p)?)))
        })
        .unwrap(),
    );
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    let key = egui::Event::Key {
        key: Key::N,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    };
    frame(&mut app, &ctx, vec![key]);
    assert!(app.canvas_input_blocked());
    assert!(!app.command_enabled(&ctx, menus::Command::Save));
    assert!(app.command_enabled(&ctx, menus::Command::Quit));
    assert!(app.editor.document == original);
    let pos = frame(&mut app, &ctx, vec![]).expect("Cancel is exposed while read is blocked");
    frame(&mut app, &ctx, vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            }],
        );
    }
    assert!(app.startup_open.is_none());
    app.editor.add(Kind::Button, 40., 50.);
    let after_cancel = app.editor.document.clone();
    release.send(()).unwrap();
    done.recv_timeout(Duration::from_secs(2)).unwrap();
    app.poll_startup_open();
    assert!(app.editor.document == after_cancel);
}
#[test]
fn persistent_startup_recovers_local_changes_without_replacing_saved_file_or_font() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Project – 世界.wireframe");
    let original = Project {
        title: "Saved".into(),
        ..Default::default()
    };
    ProjectStore.save(&path, &original).unwrap();
    let copies = WorkingCopies::new(dir.path().join("working-copies")).unwrap();
    let mut copy = copies.open_named(&path).unwrap();
    let mut changed = original.clone();
    changed.title = "Local change".into();
    copy.checkpoint(&changed).unwrap();
    drop(copy);
    let (ctx, mut app) = setup();
    app.enable_persistence(&ctx, dir.path().to_owned());
    app.start_startup_open(path.clone(), &ctx);
    drain(&mut app);
    assert!(app.error.is_none(), "{:?}", app.error);
    assert!(app.editor.document == changed);
    assert!(app.editor.dirty());
    assert!(ProjectStore.load(&path).unwrap() == original);
    assert_eq!(app.file, Some(path.canonicalize().unwrap()));
    assert!(!app.recovery_chooser_open());
    assert!(app.finish_session(false));
}
#[test]
fn failed_open_preserves_document_and_recovery_offer() {
    let dir = tempfile::tempdir().unwrap();
    for contents in [None, Some(b"not a project".as_slice())] {
        let path = dir.path().join(if contents.is_some() {
            "broken.wireframe"
        } else {
            "missing.wireframe"
        });
        if let Some(bytes) = contents {
            std::fs::write(&path, bytes).unwrap();
        }
        let (ctx, mut app) = setup();
        app.editor.add(Kind::Label, 20., 30.);
        let original = app.editor.document.clone();
        let recovery = Project {
            title: "Recover me".into(),
            ..Default::default()
        };
        app.recovery_offer = Some(recovery.clone());
        app.start_startup_open(path, &ctx);
        drain(&mut app);
        assert!(app.error.as_ref().unwrap().contains("Could not open"));
        assert!(app.editor.document == original);
        assert!(app.recovery_offer.as_ref() == Some(&recovery));
        assert!(app.file.is_none());
    }
}

#[test]
fn failed_project_read_retains_exact_details_and_pending_history() {
    let dir = tempfile::tempdir().unwrap();
    for contents in [None, Some(b"not a project".as_slice())] {
        let path = dir.path().join("Unavailable project.wireframe");
        if let Some(bytes) = contents {
            std::fs::write(&path, bytes).unwrap();
        }
        let diagnostic = ProjectStore.load(&path).unwrap_err();
        let (ctx, mut app) = setup();
        app.editor.add(Kind::Label, 20., 30.);
        let edited = app.editor.document.clone();
        app.editor.undo();
        let baseline = app.editor.document.clone();
        let redo = app.editor.redo_name().map(str::to_owned);
        app.start_startup_open(path.clone(), &ctx);
        drain(&mut app);
        let error = app.error.as_deref().unwrap();
        let (primary, details) = error.split_once("\n\nDetails: ").unwrap();
        assert!(primary.contains("Try opening it again"));
        assert!(primary.contains("restore a backup"));
        assert_eq!(
            details,
            format!("Could not open {}.\n\n{diagnostic}", path.display())
        );
        assert_eq!(app.editor.document, baseline);
        assert_eq!(app.editor.redo_name(), redo.as_deref());
        app.editor.redo();
        assert_eq!(app.editor.document, edited);
        if let Some(bytes) = contents {
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
        } else {
            assert!(!path.exists());
        }
    }
}

#[test]
fn project_open_keeps_version_and_size_explanations_in_visible_details() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Newer project.wireframe");
    let mut project = Project::default();
    project.format_version += 1;
    std::fs::write(&path, serde_json::to_vec(&project).unwrap()).unwrap();
    let (ctx, mut app) = setup();
    app.start_startup_open(path.clone(), &ctx);
    drain(&mut app);
    assert!(app.error.as_deref().unwrap().ends_with(&format!(
        "Details: Could not open {}.\n\nUnsupported document version {}",
        path.display(),
        project.format_version
    )));
    let path = dir.path().join("Oversized project.wireframe");
    std::fs::File::create(&path)
        .unwrap()
        .set_len(wireframe::document::MAX_FILE_BYTES as u64 + 1)
        .unwrap();
    app.start_startup_open(path.clone(), &ctx);
    drain(&mut app);
    assert!(app.error.as_deref().unwrap().ends_with(&format!(
        "Details: Could not open {}.\n\nProject exceeds the 64 MB file limit",
        path.display()
    )));
}

#[test]
fn project_error_card_wraps_primary_and_details_at_normal_window_sizes() {
    for size in [Vec2::new(920., 620.), Vec2::new(1360., 900.)] {
        let (ctx, mut app) = setup();
        let path = PathBuf::from(format!(
            "/unavailable/{}/Project.wireframe",
            "long-folder-name/".repeat(25)
        ));
        app.start_startup_open(path, &ctx);
        drain(&mut app);
        let error = app.error.clone().unwrap();
        let mut observed = std::collections::HashMap::new();
        for _ in 0..3 {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                    ..Default::default()
                },
                |ui| app.dialogs(ui.ctx()),
            );
            out.textures_delta.clear();
            if let Some(update) = out.platform_output.accesskit_update {
                for (_, node) in update.nodes {
                    let text = node.label().or_else(|| node.value());
                    if matches!(text, Some("Couldn't complete that action" | "OK"))
                        || text == Some(error.as_str())
                    {
                        observed.insert(text.unwrap().to_owned(), node.bounds().unwrap());
                    }
                }
            }
        }
        for label in ["Couldn't complete that action", error.as_str(), "OK"] {
            let bounds = observed
                .get(label)
                .unwrap_or_else(|| panic!("missing visible error content {label}"));
            assert!(
                bounds.x0 >= 0.
                    && bounds.y0 >= 0.
                    && bounds.x1 <= size.x as f64
                    && bounds.y1 <= size.y as f64,
                "error content outside {size:?}: {label:?} {bounds:?}"
            );
        }
    }
}
#[test]
fn fallback_startup_loads_the_project_and_worker_disconnect_reports_failure() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Fallback.wireframe");
    let p = Project {
        title: "Loaded".into(),
        ..Default::default()
    };
    ProjectStore.save(&path, &p).unwrap();
    let (ctx, mut app) = setup();
    app.start_startup_open(path.clone(), &ctx);
    drain(&mut app);
    assert!(app.editor.document == p);
    assert_eq!(app.file, Some(path));
    let (sender, receiver) = mpsc::channel();
    drop(sender);
    app.startup_open = Some(Opening {
        path: "Disconnected".into(),
        receiver,
        phase: Phase::Startup,
        quit_after: false,
        ctx: ctx.clone(),
    });
    app.poll_startup_open();
    let error = app.error.as_ref().unwrap();
    let (primary, details) = error.split_once("\n\nDetails: ").unwrap();
    assert!(!primary.contains("worker"));
    assert!(details.ends_with("The file-opening worker stopped unexpectedly."));
    assert!(app.editor.document == p);
}
#[test]
fn quit_does_not_join_a_stalled_startup_read() {
    let (ctx, mut app) = setup();
    let (sender, receiver) = mpsc::channel();
    app.startup_open = Some(Opening {
        path: "Waiting".into(),
        receiver,
        phase: Phase::Startup,
        quit_after: false,
        ctx: ctx.clone(),
    });
    app.request(Pending::Quit, &ctx);
    assert!(app.quit_allowed);
    assert!(app.startup_open.is_none());
    assert!(sender.send(Ok(Prepared::Plain(Box::default()))).is_err());
}

#[test]
fn cancelled_persistent_result_releases_its_lock_without_writing_the_named_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Cancelled.wireframe");
    let original = Project::default();
    ProjectStore.save(&path, &original).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let copies = WorkingCopies::new(dir.path().join("copies")).unwrap();
    let worker_copies = copies.clone();
    let worker_path = path.clone();
    let (ctx, mut app) = setup();
    let (release, wait) = mpsc::channel();
    let (started, ready) = mpsc::channel();
    app.startup_open = Some(
        Opening::spawn(path.clone(), ctx, move || {
            let prepared = persistence::PreparedOpen::read(&worker_copies, &worker_path)?;
            started.send(()).unwrap();
            wait.recv().unwrap();
            Ok(Prepared::Persistent(Box::new(prepared)))
        })
        .unwrap(),
    );
    ready.recv_timeout(Duration::from_secs(3)).unwrap();
    assert!(
        copies.open_named(&path).is_err(),
        "Worker owns the local copy lock"
    );
    app.cancel_startup_open();
    release.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Ok(copy) = copies.open_named(&path) {
            assert!(copy.project() == &original);
            break;
        }
        assert!(
            Instant::now() < deadline,
            "Cancelled result retained its lock"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert!(app.file.is_none());
}

#[test]
fn changed_named_file_keeps_earlier_local_work_recoverable_and_reports_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("External change.wireframe");
    let original = Project::default();
    ProjectStore.save(&path, &original).unwrap();
    let copies = WorkingCopies::new(dir.path().join("working-copies")).unwrap();
    let mut copy = copies.open_named(&path).unwrap();
    let mut local = original.clone();
    local.title = "Earlier local work".into();
    copy.checkpoint(&local).unwrap();
    drop(copy);
    let mut external = original;
    external.title = "External version".into();
    ProjectStore.save(&path, &external).unwrap();
    let (ctx, mut app) = setup();
    app.enable_persistence(&ctx, dir.path().to_owned());
    app.start_startup_open(path.clone(), &ctx);
    drain(&mut app);
    assert!(app.error.is_none(), "{:?}", app.error);
    assert!(app.editor.document == external);
    assert!(app.status.contains("Earlier local work was preserved"));
    let entry = copies
        .list()
        .unwrap()
        .into_iter()
        .filter_map(|e| e.summary.ok())
        .find(|entry| entry.title == "Earlier local work")
        .unwrap();
    let recovered = copies.resume(&entry.id).unwrap();
    assert!(recovered.project() == &local);
    assert!(ProjectStore.load(&path).unwrap() == external);
    assert!(app.finish_session(false));
}
