// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Exercise scheduling with real serialized projects and the atomic disk writer.
//! The desktop UI does not use this coordinator yet.
use std::{collections::BTreeMap, time::Duration};
use wireframe::{document::Project, save_coordinator::SaveCoordinator, storage::ProjectStore};

#[test]
fn delayed_named_write_cannot_erase_the_newer_local_checkpoint() {
    let dir = tempfile::tempdir().unwrap();
    let local = dir.path().join("working.wireframe");
    let named = dir.path().join("project.wireframe");
    let store = ProjectStore;
    let initial = Project::default();
    store.save(&named, &initial).unwrap();
    store.save(&local, &initial).unwrap();
    let mut c = SaveCoordinator::new(true);
    let mut snapshots = BTreeMap::new();
    let first = Project {
        title: "First edit".into(),
        ..initial.clone()
    };
    let r1 = c.edited().unwrap();
    snapshots.insert(r1, first.clone());
    store.save(&local, &first).unwrap();
    c.local_committed(r1, Duration::ZERO).unwrap();
    let pending = c.poll(Duration::from_secs(5)).unwrap().unwrap();

    let latest = Project {
        title: "Edit while saving".into(),
        ..first.clone()
    };
    let r2 = c.edited().unwrap();
    snapshots.insert(r2, latest.clone());
    store.save(&local, &latest).unwrap();
    c.local_committed(r2, Duration::from_secs(6)).unwrap();
    let closing_revision = c.request_flush();
    assert_eq!(closing_revision, r2);

    // The first write finishes after the second local checkpoint. Its snapshot
    // must come from the ticket, never from the mutable current working file.
    store.save(&named, &snapshots[&pending.revision()]).unwrap();
    c.complete(pending, Ok(()), Duration::from_secs(7)).unwrap();
    assert_eq!(store.load(&named).unwrap(), first);
    assert_eq!(store.load(&local).unwrap(), latest);
    assert!(c.named_revision() < closing_revision);
    assert!(!c.is_clean());

    let followup = c.poll(Duration::from_secs(7)).unwrap().unwrap();
    store
        .save(&named, &snapshots[&followup.revision()])
        .unwrap();
    c.complete(followup, Ok(()), Duration::from_secs(8))
        .unwrap();
    assert!(c.named_revision() >= closing_revision);
    assert!(c.is_clean());
    assert_eq!(
        std::fs::read(&local).unwrap(),
        std::fs::read(&named).unwrap()
    );
    assert_eq!(store.load(&named).unwrap(), latest);
}

#[test]
fn real_write_failure_keeps_local_data_and_retries_the_captured_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let local = dir.path().join("working.wireframe");
    let named = dir.path().join("project.wireframe");
    // A directory at the destination makes the atomic rename fail on each OS.
    std::fs::create_dir(&named).unwrap();
    std::fs::write(named.join("retained"), b"do not replace").unwrap();
    let store = ProjectStore;
    let doc = Project {
        title: "Durable local edit".into(),
        ..Project::default()
    };
    let mut c = SaveCoordinator::new(false);
    let revision = c.edited().unwrap();
    store.save(&local, &doc).unwrap();
    c.local_committed(revision, Duration::ZERO).unwrap();
    c.request_flush();
    let first = c.poll(Duration::ZERO).unwrap().unwrap();
    let failure = store.save(&named, &doc);
    assert!(failure.is_err());
    c.complete(first, failure, Duration::ZERO).unwrap();
    assert_eq!(store.load(&local).unwrap(), doc);
    assert_eq!(
        std::fs::read(named.join("retained")).unwrap(),
        b"do not replace"
    );
    assert!(!c.is_clean());
    assert!(c.poll(Duration::from_millis(999)).unwrap().is_none());

    let obstruction = dir.path().join("retained-directory");
    std::fs::rename(&named, &obstruction).unwrap();
    let retry = c.poll(Duration::from_secs(1)).unwrap().unwrap();
    assert!(retry.is_retry());
    assert_eq!(retry.revision(), revision);
    let result = store.save(&named, &doc);
    assert!(result.is_ok());
    c.complete(retry, result, Duration::from_secs(1)).unwrap();
    assert!(c.is_clean());
    assert_eq!(store.load(&named).unwrap(), doc);
    assert_eq!(
        std::fs::read(obstruction.join("retained")).unwrap(),
        b"do not replace"
    );
    // Failed temporary writes leave no stray files behind.
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 3);
}

#[test]
fn worker_keeps_immutable_save_snapshots_and_releases_project_ownership_on_exit() {
    use std::sync::mpsc;
    use wireframe::{
        save_coordinator::WriteTicket,
        working_copy::{CopyError, WorkingCopies},
    };
    let dir = tempfile::tempdir().unwrap();
    let named = dir.path().join("project.wireframe");
    ProjectStore.save(&named, &Project::default()).unwrap();
    let copies = WorkingCopies::new(dir.path().join("working")).unwrap();
    let mut copy = copies.open_named(&named).unwrap();
    let id = copy.id().to_owned();
    let (send, commands) = mpsc::sync_channel::<(Project, Option<WriteTicket>)>(1);
    let (ack, results) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        for (snapshot, ticket) in commands {
            let result = if ticket.is_some() {
                copy.save_named(&snapshot)
            } else {
                copy.checkpoint(&snapshot)
            };
            ack.send((ticket, result.map_err(|e| e.to_string())))
                .unwrap();
        }
        copy
    });
    let first = Project {
        title: "Captured for saving".into(),
        ..Project::default()
    };
    let latest = Project {
        title: "Edited before that save completed".into(),
        ..first.clone()
    };
    let mut c = SaveCoordinator::default();
    let r1 = c.edited().unwrap();
    send.send((first.clone(), None)).unwrap();
    let (_, result) = results.recv().unwrap();
    result.unwrap();
    c.local_committed(r1, Duration::ZERO).unwrap();
    let older = c.poll(Duration::from_secs(5)).unwrap().unwrap();
    let r2 = c.edited().unwrap();
    send.send((latest.clone(), None)).unwrap();
    let (_, result) = results.recv().unwrap();
    result.unwrap();
    c.local_committed(r2, Duration::from_secs(6)).unwrap();
    let target = c.request_flush();
    send.send((first.clone(), Some(older))).unwrap();
    let (ticket, result) = results.recv().unwrap();
    c.complete(ticket.unwrap(), result, Duration::from_secs(7))
        .unwrap();
    assert_eq!(ProjectStore.load(&named).unwrap(), first);
    assert!(c.named_revision() < target);
    assert!(!c.is_clean());
    let next = c.poll(Duration::from_secs(7)).unwrap().unwrap();
    send.send((latest.clone(), Some(next))).unwrap();
    let (ticket, result) = results.recv().unwrap();
    c.complete(ticket.unwrap(), result, Duration::from_secs(8))
        .unwrap();
    assert!(c.is_clean());
    drop(send);
    let copy = worker.join().unwrap();
    assert_eq!(copy.project(), &latest);
    assert!(!copy.has_local_changes().unwrap());
    assert!(matches!(copies.resume(&id), Err(CopyError::Busy)));
    drop(copy);
    let reopened = copies.resume(&id).unwrap();
    assert_eq!(reopened.project(), &latest);
    assert_eq!(ProjectStore.load(&named).unwrap(), latest);
}
