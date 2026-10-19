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
    if let Ok(mode) = std::env::var("WIREFRAME_CRASH_TEST_MODE") {
        save_as_child(&root, stage, mode == "named");
    }
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

fn start_writer(root: &Path, stage: usize, mode: Option<&str>) -> Writer {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "working_copy::crash_tests::child_process",
            "--ignored",
            "--nocapture",
        ])
        .env("WIREFRAME_CRASH_TEST_DIR", root)
        .env("WIREFRAME_CRASH_TEST_STAGE", stage.to_string())
        .env_remove("WIREFRAME_CRASH_TEST_MODE")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    if let Some(mode) = mode {
        command.env("WIREFRAME_CRASH_TEST_MODE", mode);
    }
    Writer(command.spawn().unwrap())
}

fn marker(writer: &mut Writer, root: &Path, stage: usize) -> String {
    let ready = root.join("ready");
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
    fs::read_to_string(ready).unwrap()
}

fn stop_writer(stage: usize) -> (tempfile::TempDir, WorkingCopies, String, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let named = dir.path().join("saved.wireframe");
    ProjectStore
        .save(&named, &project("Original saved"))
        .unwrap();
    let mut writer = start_writer(dir.path(), stage, None);
    let id = marker(&mut writer, dir.path(), stage);
    let copies = WorkingCopies::new(dir.path().join("copies")).unwrap();
    assert!(matches!(copies.resume(&id), Err(CopyError::Busy)));
    writer.0.kill().unwrap();
    assert!(!writer.0.wait().unwrap().success());
    eprintln!("Abrupt writer termination at stage {stage}; live ownership verified");
    (dir, copies, id, named)
}

fn save_as_child(root: &Path, stage: usize, named_source: bool) -> ! {
    let copies = WorkingCopies::new(root.join("copies")).unwrap();
    let mut source = if named_source {
        copies.open_named(&root.join("saved.wireframe")).unwrap()
    } else {
        copies.create_unnamed(&project("Unnamed original")).unwrap()
    };
    let snapshot = project("Save As local edits");
    source.checkpoint(&snapshot).unwrap();
    let source_id = source.id().to_owned();
    let stop = |destination: Option<&str>| {
        wait_for_kill(
            root,
            &serde_json::to_string(&(&source_id, destination)).unwrap(),
        )
    };
    if stage == 0 {
        stop(None);
    }
    let mut destination = copies
        .prepare_save_as(&root.join("destination.wireframe"), &snapshot)
        .unwrap();
    let destination_id = destination.id().to_owned();
    if stage == 1 {
        stop(Some(&destination_id));
    }
    let mut writes = 0;
    destination
        .save_named_using(&snapshot, |path, bytes| {
            write_atomic(path, bytes)?;
            writes += 1;
            if stage == writes + 1 {
                stop(Some(&destination_id));
            }
            Ok(())
        })
        .unwrap();
    source.discard().unwrap();
    if stage == 5 {
        stop(Some(&destination_id));
    }
    panic!("Save As crash boundary was not reached");
}

#[test]
fn interrupted_save_as_preserves_source_destination_and_previous_local_edits() {
    for named_source in [false, true] {
        for existing_destination in [false, true] {
            for stage in 0..=5 {
                let dir = tempfile::tempdir().unwrap();
                let copies = WorkingCopies::new(dir.path().join("copies")).unwrap();
                let source_path = dir.path().join("saved.wireframe");
                let destination_path = dir.path().join("destination.wireframe");
                let original = project("Original source");
                ProjectStore.save(&source_path, &original).unwrap();
                let prior_local = project("Previous destination local edits");
                if existing_destination {
                    ProjectStore
                        .save(&destination_path, &project("Previous destination saved"))
                        .unwrap();
                    copies
                        .open_named(&destination_path)
                        .unwrap()
                        .checkpoint(&prior_local)
                        .unwrap();
                }
                let mut writer = start_writer(
                    dir.path(),
                    stage,
                    Some(if named_source { "named" } else { "unnamed" }),
                );
                let (source_id, destination_id): (String, Option<String>) =
                    serde_json::from_str(&marker(&mut writer, dir.path(), stage)).unwrap();
                if stage < 5 {
                    assert!(matches!(copies.resume(&source_id), Err(CopyError::Busy)));
                }
                if let Some(id) = &destination_id {
                    assert!(matches!(copies.resume(id), Err(CopyError::Busy)));
                }
                writer.0.kill().unwrap();
                assert!(!writer.0.wait().unwrap().success());
                let snapshot = project("Save As local edits");
                assert_eq!(fs::read(&source_path).unwrap(), original.encode().unwrap());
                if stage < 5 {
                    assert_eq!(copies.resume(&source_id).unwrap().project(), &snapshot);
                } else {
                    assert!(
                        !dir.path()
                            .join("copies")
                            .join(format!("{source_id}.json"))
                            .exists()
                    );
                }
                if stage >= 3 {
                    assert_eq!(
                        fs::read(&destination_path).unwrap(),
                        snapshot.encode().unwrap()
                    );
                } else if existing_destination {
                    assert_eq!(
                        fs::read(&destination_path).unwrap(),
                        project("Previous destination saved").encode().unwrap()
                    );
                } else {
                    assert!(!destination_path.exists());
                }
                if let Some(id) = destination_id {
                    let mut destination = copies.resume(&id).unwrap();
                    assert_eq!(destination.project(), &snapshot);
                    destination.save_named(&snapshot).unwrap();
                    assert_eq!(
                        fs::read(&destination_path).unwrap(),
                        snapshot.encode().unwrap()
                    );
                }
                let archives = copies.list_archives().unwrap();
                if existing_destination && stage > 0 {
                    assert_eq!(archives.len(), 1);
                    let name = archives[0].id.strip_prefix("archive:").unwrap();
                    let archive_path = dir.path().join("copies").join(name);
                    let bytes = fs::read(&archive_path).unwrap();
                    let restored = copies.restore_archive(name).unwrap();
                    assert_eq!(restored.project(), &prior_local);
                    assert!(restored.named_path().unwrap().is_none());
                    assert_eq!(fs::read(archive_path).unwrap(), bytes);
                } else {
                    assert!(archives.is_empty());
                    if existing_destination {
                        assert_eq!(
                            copies.open_named(&destination_path).unwrap().project(),
                            &prior_local
                        );
                    }
                }
                eprintln!(
                    "Save As termination: named_source={named_source}, existing_destination={existing_destination}, stage={stage}"
                );
            }
        }
    }
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
                assert!(matches!(status, NamedStatus::Changed(p) if *p == project("Outside edit")))
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
