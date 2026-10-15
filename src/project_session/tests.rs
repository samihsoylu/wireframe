// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::storage::ProjectStore;
use std::sync::{
    Barrier,
    atomic::{AtomicBool, Ordering},
};

#[test]
fn rapid_edits_coalesce_without_crossing_an_explicit_save_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let copies = WorkingCopies::new(dir.path().join("copies")).unwrap();
    let named = dir.path().join("saved.wireframe");
    ProjectStore.save(&named, &Project::default()).unwrap();
    let copy = copies.open_named(&named).unwrap();
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let first = Arc::new(AtomicBool::new(true));
    let wake = {
        let entered = entered.clone();
        let release = release.clone();
        Arc::new(move || {
            if first.swap(false, Ordering::SeqCst) {
                entered.wait();
                release.wait();
            }
        })
    };
    let mut session = ProjectSession::start(copies.clone(), copy, false, wake).unwrap();
    entered.wait();
    let mut references = vec![];
    for i in 0..100 {
        let p = Project {
            title: format!("A{i}"),
            ..Project::default()
        };
        session.observe(&p).unwrap();
        references.push(Arc::downgrade(&session.observed));
    }
    let first_snapshot = session.observed.clone();
    session.flush(&first_snapshot).unwrap();
    drop(first_snapshot);
    for i in 0..100 {
        let p = Project {
            title: format!("B{i}"),
            ..Project::default()
        };
        session.observe(&p).unwrap();
        references.push(Arc::downgrade(&session.observed));
    }
    // Release the actor before asserting so a failed test cannot strand its join.
    let retained = references.iter().filter(|r| r.upgrade().is_some()).count();
    release.wait();
    session.finish().unwrap();
    assert_eq!(retained, 2);
    assert_eq!(ProjectStore.load(&named).unwrap().title, "A99");
    assert_eq!(copies.open_named(&named).unwrap().project().title, "B99");
}
