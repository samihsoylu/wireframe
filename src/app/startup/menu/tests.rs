// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::app::startup::tests::{drain, frame, setup};

fn project(title: &str) -> Project {
    Project {
        title: title.into(),
        ..Default::default()
    }
}
fn persistent() -> (tempfile::TempDir, egui::Context, WireframeApp, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Original – 世界.wireframe");
    ProjectStore.save(&path, &project("Original")).unwrap();
    let (ctx, mut app) = setup();
    app.enable_persistence(&ctx, dir.path().to_owned());
    app.set_save_mode(false);
    app.start_startup_open(path.clone(), &ctx);
    drain(&mut app);
    assert!(app.error.is_none(), "{:?}", app.error);
    (dir, ctx, app, path)
}

#[test]
fn clean_menu_open_changes_file_only_after_async_preparation_and_handoff() {
    let (dir, ctx, mut app, original) = persistent();
    let path = dir.path().join("Next.wireframe");
    let next = project("Next");
    ProjectStore.save(&path, &next).unwrap();
    app.request(Pending::Open(path.clone()), &ctx);
    assert!(app.startup_open.is_some());
    assert_eq!(app.editor.document.title, "Original");
    assert_eq!(app.file, Some(original.canonicalize().unwrap()));
    drain(&mut app);
    assert!(app.error.is_none(), "{:?}", app.error);
    assert_eq!(app.editor.document, next);
    assert_eq!(app.file, Some(path.canonicalize().unwrap()));
    assert!(!app.editor.dirty());
    assert!(app.finish_session(false));
}

#[test]
fn discard_open_failure_keeps_document_history_owner_and_named_bytes() {
    for variant in ["missing", "invalid", "locked"] {
        let (dir, ctx, mut app, original) = persistent();
        let bytes = std::fs::read(&original).unwrap();
        app.editor.add(Kind::Label, 20., 30.);
        let changed = app.editor.document.clone();
        app.observe_project();
        let id = app.open_source_id();
        let path = dir.path().join(format!("{variant}.wireframe"));
        let owner = match variant {
            "invalid" => {
                std::fs::write(&path, "invalid").unwrap();
                None
            }
            "locked" => {
                ProjectStore.save(&path, &project("Other owner")).unwrap();
                Some(app.startup_copies().unwrap().open_named(&path).unwrap())
            }
            _ => None,
        };
        app.discard_then(Pending::Open(path), &ctx);
        drain(&mut app);
        assert!(app.error.is_some(), "{variant}");
        assert!(matches!(app.pending, Some(Pending::Open(_))));
        assert_eq!(app.open_source_id(), id);
        assert_eq!(app.editor.document, changed);
        assert_eq!(std::fs::read(&original).unwrap(), bytes);
        assert!(app.startup_copies().unwrap().open_named(&original).is_err());
        app.editor.undo();
        assert_eq!(app.editor.document.title, "Original");
        assert!(app.editor.current().controls.is_empty());
        drop(owner);
        assert!(app.finish_session(false));
    }
}

#[test]
fn cancelled_discard_preparation_keeps_live_source_and_drops_late_target_owner() {
    let (dir, ctx, mut app, original) = persistent();
    app.editor.add(Kind::Button, 20., 30.);
    let changed = app.editor.document.clone();
    let target = dir.path().join("Slow.wireframe");
    ProjectStore.save(&target, &project("Slow")).unwrap();
    let copies = app.startup_copies().unwrap();
    let worker_copies = copies.clone();
    let worker_target = target.clone();
    let (release, wait) = mpsc::channel();
    let (started, ready) = mpsc::channel();
    let mut job = Opening::spawn(target.clone(), ctx.clone(), move || {
        let prepared = persistence::PreparedOpen::read(&worker_copies, &worker_target)?;
        started.send(()).unwrap();
        wait.recv().unwrap();
        Ok(Prepared::Persistent(Box::new(prepared)))
    })
    .unwrap();
    job.phase = Phase::MenuRead { discard: true };
    app.startup_open = Some(job);
    ready.recv_timeout(Duration::from_secs(3)).unwrap();
    assert!(copies.open_named(&original).is_err(), "source stays owned");
    assert!(copies.open_named(&target).is_err(), "worker holds target");
    frame(&mut app, &ctx, vec![]);
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert!(app.startup_open.is_none());
    assert_eq!(app.editor.document, changed);
    app.editor.add(Kind::Label, 50., 60.);
    app.observe_project();
    let after = app.editor.document.clone();
    assert!(app.error.is_none(), "{:?}", app.error);
    release.send(()).unwrap();
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        if copies.open_named(&target).is_ok() {
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(2));
    }
    app.poll_startup_open();
    assert_eq!(app.editor.document, after);
    assert!(app.finish_session(false));
    assert_eq!(copies.open_named(&original).unwrap().project(), &after);
}

#[test]
fn discard_handoff_removes_only_source_after_target_is_ready() {
    let (dir, ctx, mut app, original) = persistent();
    let copies = app.startup_copies().unwrap();
    let old = app.open_source_id().unwrap();
    let original_bytes = std::fs::read(&original).unwrap();
    app.editor.add(Kind::Button, 20., 30.);
    let target = dir.path().join("Next.wireframe");
    ProjectStore.save(&target, &project("Next")).unwrap();
    app.discard_then(Pending::Open(target), &ctx);
    drain(&mut app);
    assert!(app.error.is_none(), "{:?}", app.error);
    assert_eq!(app.editor.document.title, "Next");
    assert!(
        copies.resume(&old).is_err(),
        "discard removed source local record"
    );
    assert_eq!(std::fs::read(&original).unwrap(), original_bytes);
    assert!(app.finish_session(false));
}

#[test]
fn same_identity_reopen_with_and_without_discard_preserves_correct_version() {
    for discard in [false, true] {
        let (dir, ctx, mut app, original) = persistent();
        let old = app.open_source_id().unwrap();
        app.editor.add(Kind::Button, 20., 30.);
        let changed = app.editor.document.clone();
        let alias_dir = dir.path().join("aliases");
        std::fs::create_dir(&alias_dir).unwrap();
        let alias = alias_dir.join("..").join(original.file_name().unwrap());
        assert_eq!(WorkingCopies::named_id(&alias).unwrap(), old);
        app.start_menu_open(alias, discard, &ctx);
        drain(&mut app);
        assert!(app.error.is_none(), "{:?}", app.error);
        assert_eq!(app.open_source_id().as_ref(), Some(&old));
        assert_eq!(
            app.editor.document,
            if discard {
                project("Original")
            } else {
                changed
            }
        );
        assert_eq!(ProjectStore.load(&original).unwrap(), project("Original"));
        assert_eq!(app.editor.dirty(), !discard);
        assert!(app.finish_session(false));
    }
}

#[test]
fn automatic_open_flushes_current_edits_before_switching() {
    let (dir, ctx, mut app, original) = persistent();
    app.set_save_mode(true);
    app.editor.add(Kind::Button, 20., 30.);
    let expected = app.editor.document.clone();
    let next = dir.path().join("Next.wireframe");
    ProjectStore.save(&next, &project("Next")).unwrap();
    app.request(Pending::Open(next), &ctx);
    let until = Instant::now() + Duration::from_secs(10);
    while app.editor.document.title != "Next" {
        app.poll_saves(&ctx);
        app.poll_startup_open();
        assert!(app.error.is_none(), "{:?}", app.error);
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(ProjectStore.load(&original).unwrap(), expected);
    assert!(app.finish_session(false));
}

#[test]
fn handoff_ignores_escape_and_defers_quit_until_ownership_is_settled() {
    let (ctx, mut app) = setup();
    let (sender, receiver) = mpsc::channel();
    app.startup_open = Some(Opening {
        path: "Ready.wireframe".into(),
        receiver,
        phase: Phase::Handoff,
        quit_after: false,
        ctx: ctx.clone(),
    });
    frame(&mut app, &ctx, vec![]);
    app.cancel_startup_open();
    assert!(app.startup_open.is_some());
    app.request(Pending::Quit, &ctx);
    assert!(!app.quit_allowed);
    assert!(app.startup_open.as_ref().unwrap().quit_after);
    sender
        .send(Ok(Prepared::Committed {
            result: Ok(Box::new(Prepared::Plain(Box::new(
                prepare_editor(project("Ready")).unwrap(),
            )))),
            discarded: false,
        }))
        .unwrap();
    drain(&mut app);
    assert_eq!(app.editor.document.title, "Ready");
    assert!(app.quit_allowed);
}

#[test]
fn failure_after_same_file_discard_retains_visible_document_and_can_save_as() {
    let (dir, ctx, mut app, original) = persistent();
    app.editor.add(Kind::Button, 20., 30.);
    let changed = app.editor.document.clone();
    // The valid preflight has completed, but the destination disappears before
    // handoff. The old visible document must survive even after explicit discard.
    let preflight = Prepared::Current(Some(Box::new(ProjectStore.load(&original).unwrap())));
    std::fs::remove_file(&original).unwrap();
    app.begin_open_handoff(original.clone(), preflight, true, &ctx);
    drain(&mut app);
    assert!(app.error.is_some());
    assert_eq!(app.editor.document, changed);
    assert_eq!(
        app.file.as_ref(),
        Some(&original.canonicalize().unwrap_or_else(|_| {
            original
                .parent()
                .unwrap()
                .canonicalize()
                .unwrap()
                .join(original.file_name().unwrap())
        }))
    );
    assert!(app.open_source_id().is_none());
    assert!(
        !original.exists(),
        "failure must not silently recreate the named file"
    );
    let copy = dir.path().join("Retained.wireframe");
    app.error = None;
    assert!(app.queue_save(copy.clone(), true));
    let until = Instant::now() + Duration::from_secs(10);
    while app.save_busy() {
        app.poll_saves(&ctx);
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(app.error.is_none(), "{:?}", app.error);
    assert_eq!(ProjectStore.load(&copy).unwrap(), changed);
    assert!(app.finish_session(false));
}

#[cfg(unix)]
#[test]
fn symlink_reopen_uses_existing_owner_instead_of_contending_with_it() {
    let (dir, ctx, mut app, original) = persistent();
    let alias = dir.path().join("Alias.wireframe");
    std::os::unix::fs::symlink(&original, &alias).unwrap();
    let id = app.open_source_id();
    app.request(Pending::Open(alias), &ctx);
    drain(&mut app);
    assert!(app.error.is_none(), "{:?}", app.error);
    assert_eq!(app.open_source_id(), id);
    assert_eq!(app.file, Some(original.canonicalize().unwrap()));
    assert!(app.finish_session(false));
}

#[test]
fn nonactive_page_layout_failure_keeps_source_and_rejects_discard() {
    for discard in [false, true] {
        let (dir, ctx, mut app, original) = persistent();
        app.editor.add(Kind::Button, 20., 30.);
        let changed = app.editor.document.clone();
        let owner = app.open_source_id();
        let original_bytes = std::fs::read(&original).unwrap();
        let mut target = project("Layout failure on second page");
        let mut second = target.pages[0].clone();
        second.id = 2;
        let mut label = wireframe::catalog::get("label")
            .unwrap()
            .create(3, wireframe::document::Point::default());
        label.properties.insert(
            "text".into(),
            wireframe::document::Value::Text("W".repeat(2000)),
        );
        label
            .properties
            .insert("font_size".into(), wireframe::document::Value::Number(120.));
        second.content.nodes.push(label);
        target.pages.push(second);
        target.next_id = 4;
        target.validate().unwrap();
        let path = dir.path().join("Unlayoutable.wireframe");
        ProjectStore.save(&path, &target).unwrap();
        let target_bytes = std::fs::read(&path).unwrap();
        app.start_menu_open(path.clone(), discard, &ctx);
        drain(&mut app);
        assert!(
            app.error.as_ref().unwrap().contains("Layout dimensions"),
            "{:?}",
            app.error
        );
        assert_eq!(app.editor.document, changed);
        assert_eq!(app.open_source_id(), owner);
        assert_eq!(std::fs::read(&original).unwrap(), original_bytes);
        assert_eq!(std::fs::read(&path).unwrap(), target_bytes);
        assert!(app.startup_copies().unwrap().open_named(&path).is_ok());
        app.editor.undo();
        assert!(app.editor.current().controls.is_empty());
        assert!(app.finish_session(false));
    }
}
