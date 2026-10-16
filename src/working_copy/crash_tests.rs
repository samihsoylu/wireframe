// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

//! Real process termination tests. The writer cannot run destructors or finish
//! its journal after the parent kills it. All files belong to the test TempDir.
use super::*;
use crate::storage::ProjectStore;
use std::{
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn project(title: &str) -> Project {
    let mut project = Project {
        title: title.into(),
        ..Project::default()
    };
    project.pages[0].notes = "Retain accents é, العربية, 日本語 and 🧭\nSecond line".into();
    project
}

fn wait_for_kill(root: &Path, id: &str) -> ! {
    write_atomic(&root.join("ready"), id.as_bytes()).unwrap();
    loop {
        thread::park();
    }
}

#[test]
#[ignore = "Helper executed only by the crash tests in a separate process"]
fn child_process() {
    let Some(root) = std::env::var_os("WIREFRAME_CRASH_TEST_DIR") else {
        return;
    };
    let root = PathBuf::from(root);
    let stage: usize = std::env::var("WIREFRAME_CRASH_TEST_STAGE")
        .unwrap()
        .parse()
        .unwrap();
    let copies = WorkingCopies::new(root.join("copies")).unwrap();
    let mut copy = copies.open_named(&root.join("saved.wireframe")).unwrap();
    let latest = project("Latest local edit");
    if stage == 0 {
        wait_for_kill(&root, copy.id());
    }
    copy.checkpoint(&latest).unwrap();
    if stage == 1 {
        wait_for_kill(&root, copy.id());
    }
    let id = copy.id().to_owned();
    let mut writes = 0;
    copy.save_named_using(&project("Older save snapshot"), |path, bytes| {
        write_atomic(path, bytes)?;
        writes += 1;
        if stage == writes + 1 {
            wait_for_kill(&root, &id);
        }
        Ok(())
    })
    .unwrap();
    panic!("The requested crash boundary was not reached");
}

struct Writer(Child);
impl Drop for Writer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn stop_writer(stage: usize) -> (tempfile::TempDir, WorkingCopies, String, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let named = dir.path().join("saved.wireframe");
    ProjectStore
        .save(&named, &project("Original saved"))
        .unwrap();
    let mut writer = Writer(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "working_copy::crash_tests::child_process",
                "--ignored",
                "--nocapture",
            ])
            .env("WIREFRAME_CRASH_TEST_DIR", dir.path())
            .env("WIREFRAME_CRASH_TEST_STAGE", stage.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    let ready = dir.path().join("ready");
    let deadline = Instant::now() + Duration::from_secs(30);
    while !ready.exists() {
        assert!(
            writer.0.try_wait().unwrap().is_none(),
            "Writer exited early"
        );
        assert!(
            Instant::now() < deadline,
            "Writer did not reach stage {stage}"
        );
        thread::sleep(Duration::from_millis(10));
    }
    let id = fs::read_to_string(ready).unwrap();
    let copies = WorkingCopies::new(dir.path().join("copies")).unwrap();
    assert!(matches!(copies.resume(&id), Err(CopyError::Busy)));
    writer.0.kill().unwrap();
    assert!(!writer.0.wait().unwrap().success());
    eprintln!("Abrupt writer termination at stage {stage}; live ownership verified");
    (dir, copies, id, named)
}

#[test]
fn killed_writer_retains_last_checkpoint_at_each_save_boundary() {
    for stage in 0..=4 {
        let (_dir, copies, id, named) = stop_writer(stage);
        let local = project(if stage == 0 {
            "Original saved"
        } else {
            "Latest local edit"
        });
        let saved = project(if stage < 3 {
            "Original saved"
        } else {
            "Older save snapshot"
        });
        assert_eq!(fs::read(&named).unwrap(), saved.encode().unwrap());
        let mut copy = copies.resume(&id).unwrap();
        assert_eq!(copy.project(), &local);
        assert!(matches!(
            copy.named_status().unwrap(),
            NamedStatus::Unchanged
        ));
        assert_eq!(
            copy.record.named.as_ref().unwrap().pending.is_some(),
            stage == 2
        );
        copy.save_named(&local).unwrap();
        assert_eq!(fs::read(&named).unwrap(), local.encode().unwrap());
        assert!(copy.record.named.as_ref().unwrap().pending.is_none());
        drop(copy);
        assert_eq!(copies.resume(&id).unwrap().project(), &local);
    }
}

#[test]
fn interrupted_acknowledgement_does_not_overwrite_external_changes() {
    for change in ["modified", "missing", "unreadable"] {
        let (_dir, copies, id, named) = stop_writer(3);
        match change {
            "modified" => ProjectStore.save(&named, &project("Outside edit")).unwrap(),
            "missing" => fs::remove_file(&named).unwrap(),
            "unreadable" => fs::write(&named, b"not a project").unwrap(),
            _ => unreachable!(),
        }
        let before = fs::read(&named).ok();
        let mut copy = copies.resume(&id).unwrap();
        assert_eq!(copy.project(), &project("Latest local edit"));
        let status = copy.named_status().unwrap();
        match change {
            "modified" => {
                assert!(matches!(status, NamedStatus::Changed(p) if p == project("Outside edit")))
            }
            "missing" => assert!(matches!(status, NamedStatus::Missing)),
            "unreadable" => assert!(matches!(status, NamedStatus::Unreadable(_))),
            _ => unreachable!(),
        }
        let expected = if change == "missing" {
            CopyError::NamedMissing
        } else {
            CopyError::NamedChanged
        };
        assert_eq!(
            copy.save_named(&project("Latest local edit")),
            Err(expected)
        );
        assert_eq!(fs::read(&named).ok(), before);
        drop(copy);
        assert_eq!(
            copies.resume(&id).unwrap().project(),
            &project("Latest local edit")
        );
        eprintln!("Preserved {change} destination after interrupted acknowledgement");
    }
}
