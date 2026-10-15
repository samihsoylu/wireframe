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
