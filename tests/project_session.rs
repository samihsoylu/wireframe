// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use wireframe::{
    document::Project,
    project_session::{ProjectSession, RequestId, SaveReceipt, SessionEvent},
    storage::ProjectStore,
    working_copy::WorkingCopies,
    workspace::Editor,
};
fn project(title: &str) -> Project {
    Project {
        title: title.into(),
        ..Project::default()
    }
}
fn fixture(automatic: bool) -> (tempfile::TempDir, WorkingCopies, PathBuf, ProjectSession) {
    let dir = tempfile::tempdir().unwrap();
    let copies = WorkingCopies::new(dir.path().join("working")).unwrap();
    let named = dir.path().join("saved.wireframe");
    ProjectStore.save(&named, &project("Original")).unwrap();
    let copy = copies.open_named(&named).unwrap();
    let session = ProjectSession::start(copies.clone(), copy, automatic, Arc::new(|| {})).unwrap();
    (dir, copies, named, session)
}
fn until(
    session: &ProjectSession,
    mut matches: impl FnMut(&SessionEvent) -> bool,
) -> Vec<SessionEvent> {
    let deadline = Instant::now() + Duration::from_secs(12);
    let mut events = vec![];
    loop {
        if let Some(event) = session.try_event().unwrap() {
            let found = matches(&event);
            events.push(event);
            if found {
                return events;
            }
        } else {
            assert!(
                Instant::now() < deadline,
                "Timed out waiting for worker event: {events:?}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
fn request(session: &ProjectSession, id: RequestId) -> Result<SaveReceipt, String> {
    let events = until(
        session,
        |e| matches!(e,SessionEvent::RequestFinished{request,..}if *request==id),
    );
    match events.into_iter().last().unwrap() {
        SessionEvent::RequestFinished { result, .. } => result,
        _ => unreachable!(),
    }
}
#[test]
fn finishing_manual_session_checkpoints_queued_edits_without_named_save() {
    let (_dir, copies, named, mut session) = fixture(false);
    session.observe(&project("Local only")).unwrap();
    session.finish().unwrap();
    assert_eq!(ProjectStore.load(&named).unwrap(), project("Original"));
    let resumed = copies.open_named(&named).unwrap();
    assert_eq!(resumed.project(), &project("Local only"));
    assert!(resumed.has_local_changes().unwrap());
}
#[test]
fn automatic_worker_checkpoints_then_saves_after_the_debounce() {
    let (_dir, copies, named, mut session) = fixture(true);
    let started = Instant::now();
    session.observe(&project("Automatic")).unwrap();
    let events = until(
        &session,
        |e| matches!(e,SessionEvent::Saved{snapshot,..}if snapshot.title=="Automatic"),
    );
    assert!(started.elapsed() >= Duration::from_millis(5900));
    let local = events
        .iter()
        .position(
            |e| matches!(e,SessionEvent::Checkpointed{snapshot,..}if snapshot.title=="Automatic"),
        )
        .unwrap();
    let saved = events
        .iter()
        .position(|e| matches!(e, SessionEvent::Saved { .. }))
        .unwrap();
    assert!(local < saved);
    session.finish().unwrap();
    assert_eq!(ProjectStore.load(&named).unwrap(), project("Automatic"));
    assert!(
        !copies
            .open_named(&named)
            .unwrap()
            .has_local_changes()
            .unwrap()
    );
}
#[test]
fn save_as_keeps_source_when_busy_and_transfers_ownership_after_success() {
    let dir = tempfile::tempdir().unwrap();
    let copies = WorkingCopies::new(dir.path().join("working")).unwrap();
    let source = copies.create_unnamed(&project("Source")).unwrap();
    let source_path = source.local_path().to_owned();
    let named = dir.path().join("destination.wireframe");
    ProjectStore.save(&named, &project("Existing")).unwrap();
    let destination_owner = copies.open_named(&named).unwrap();
    let mut session =
        ProjectSession::start(copies.clone(), source, false, Arc::new(|| {})).unwrap();
    let id = session
        .save_as(named.clone(), &project("New contents"))
        .unwrap();
    assert!(request(&session, id).unwrap_err().contains("in use"));
    assert!(source_path.exists());
    assert_eq!(ProjectStore.load(&named).unwrap(), project("Existing"));
    drop(destination_owner);
    let id = session
        .save_as(named.clone(), &project("New contents"))
        .unwrap();
    let receipt = request(&session, id).unwrap();
    assert_eq!(receipt.snapshot.as_ref(), &project("New contents"));
    assert!(receipt.preserved.is_some());
    assert!(!source_path.exists());
    session.observe(&project("Later local edit")).unwrap();
    session.finish().unwrap();
    let recovered = copies.open_named(&named).unwrap();
    assert_eq!(recovered.project(), &project("Later local edit"));
    assert_eq!(ProjectStore.load(&named).unwrap(), project("New contents"));
}
#[test]
fn save_as_to_current_destination_is_a_guarded_flush_without_relocking() {
    let (_dir, copies, named, mut session) = fixture(false);
    let id = session
        .save_as(named.clone(), &project("Same location"))
        .unwrap();
    request(&session, id).unwrap();
    session.finish().unwrap();
    assert_eq!(ProjectStore.load(&named).unwrap(), project("Same location"));
    assert_eq!(copies.list().unwrap().len(), 1);
}
#[test]
fn failed_manual_flush_does_not_resurrect_on_later_local_edits() {
    let (_dir, copies, named, mut session) = fixture(false);
    ProjectStore.save(&named, &project("External")).unwrap();
    let id = session.flush(&project("Attempted")).unwrap();
    assert!(request(&session, id).is_err());
    ProjectStore.save(&named, &project("Original")).unwrap();
    session.observe(&project("Later edit")).unwrap();
    until(
        &session,
        |e| matches!(e,SessionEvent::Checkpointed{snapshot,..}if snapshot.title=="Later edit"),
    );
    session.set_automatic(false).unwrap();
    session.finish().unwrap();
    assert_eq!(ProjectStore.load(&named).unwrap(), project("Original"));
    assert_eq!(
        copies.open_named(&named).unwrap().project(),
        &project("Later edit")
    );
}
#[test]
fn asynchronous_save_acknowledges_its_snapshot_without_clearing_newer_editor_changes() {
    let (_dir, copies, named, mut session) = fixture(false);
    let mut editor = Editor::new(project("Original"));
    editor.edit("First", |e| e.document.title = "First".into());
    let first = editor.document.clone();
    let id = session.flush(&first).unwrap();
    editor.edit("Later", |e| e.document.title = "Later".into());
    session.observe(&editor.document).unwrap();
    let receipt = request(&session, id).unwrap();
    assert_eq!(receipt.snapshot.as_ref(), &first);
    editor.mark_saved_snapshot(&receipt.snapshot);
    assert!(editor.dirty());
    editor.undo();
    assert!(!editor.dirty());
    editor.redo();
    assert!(editor.dirty());
    session.finish().unwrap();
    assert_eq!(ProjectStore.load(&named).unwrap(), first);
    assert_eq!(
        copies.open_named(&named).unwrap().project(),
        &project("Later")
    );
}
#[test]
fn explicit_discard_does_not_recreate_a_record_during_worker_shutdown() {
    let dir = tempfile::tempdir().unwrap();
    let copies = WorkingCopies::new(dir.path().join("working")).unwrap();
    let copy = copies.create_unnamed(&project("Original")).unwrap();
    let path = copy.local_path().to_owned();
    let mut session = ProjectSession::start(copies.clone(), copy, true, Arc::new(|| {})).unwrap();
    session.observe(&project("Pending")).unwrap();
    session.discard().unwrap();
    assert!(!path.exists());
    assert!(path.with_extension("lock").exists());
    assert!(copies.list().unwrap().is_empty());
}
#[test]
fn enabling_automatic_mode_flushes_pending_local_edits_without_waiting_five_seconds() {
    let (_dir, _copies, named, mut session) = fixture(false);
    let started = Instant::now();
    session.observe(&project("Enabled")).unwrap();
    session.set_automatic(true).unwrap();
    until(
        &session,
        |e| matches!(e,SessionEvent::Saved{snapshot,..}if snapshot.title=="Enabled"),
    );
    assert!(started.elapsed() < Duration::from_secs(4));
    session.finish().unwrap();
    assert_eq!(ProjectStore.load(&named).unwrap(), project("Enabled"));
}
