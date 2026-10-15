// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use crate::storage::ProjectStore;
fn project(title: &str) -> Project {
    Project {
        title: title.into(),
        ..Project::default()
    }
}
fn fixture() -> (tempfile::TempDir, WorkingCopies, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let copies = WorkingCopies::new(dir.path().join("copies")).unwrap();
    let named = dir.path().join("saved.wireframe");
    ProjectStore.save(&named, &project("Saved")).unwrap();
    (dir, copies, named)
}
#[test]
fn independent_projects_and_unnamed_work_survive_release_and_resume() {
    let (_dir, copies, named) = fixture();
    let mut first = copies.open_named(&named).unwrap();
    let second = copies.create_unnamed(&project("Untitled A")).unwrap();
    let third = copies.create_unnamed(&project("Untitled B")).unwrap();
    assert_ne!(second.id(), third.id());
    first.checkpoint(&project("Local edits")).unwrap();
    let first_id = first.id().to_owned();
    let second_id = second.id().to_owned();
    drop(first);
    drop(second);
    drop(third);
    assert_eq!(copies.list().unwrap().len(), 3);
    assert_eq!(
        copies.resume(&first_id).unwrap().project(),
        &project("Local edits")
    );
    assert_eq!(
        copies.resume(&second_id).unwrap().project(),
        &project("Untitled A")
    );
    assert_eq!(ProjectStore.load(&named).unwrap(), project("Saved"));
}
#[test]
fn canonical_aliases_share_one_owner_and_lock_is_released_on_drop() {
    let (_dir, copies, named) = fixture();
    let copy = copies.open_named(&named).unwrap();
    let alias = named
        .parent()
        .unwrap()
        .join(".")
        .join(named.file_name().unwrap());
    assert!(matches!(copies.open_named(&alias), Err(CopyError::Busy)));
    assert!(matches!(copies.resume(copy.id()), Err(CopyError::Busy)));
    let id = copy.id().to_owned();
    drop(copy);
    assert_eq!(copies.open_named(&alias).unwrap().id(), id);
}
#[test]
fn older_named_snapshot_does_not_replace_the_latest_local_document() {
    let (_dir, copies, named) = fixture();
    let mut copy = copies.open_named(&named).unwrap();
    let first = project("Write in progress");
    let latest = project("Newer local edit");
    copy.checkpoint(&latest).unwrap();
    copy.save_named(&first).unwrap();
    assert_eq!(ProjectStore.load(&named).unwrap(), first);
    assert_eq!(copy.project(), &latest);
    let id = copy.id().to_owned();
    drop(copy);
    let mut copy = copies.resume(&id).unwrap();
    assert_eq!(copy.project(), &latest);
    assert!(matches!(
        copy.named_status().unwrap(),
        NamedStatus::Unchanged
    ));
    copy.save_named(&latest).unwrap();
    assert_eq!(ProjectStore.load(&named).unwrap(), latest);
}
#[test]
fn external_changes_are_available_for_selection_but_never_overwritten() {
    let (_dir, copies, named) = fixture();
    let mut copy = copies.open_named(&named).unwrap();
    copy.checkpoint(&project("Local")).unwrap();
    ProjectStore.save(&named, &project("External")).unwrap();
    assert!(
        matches!(copy.named_status().unwrap(),NamedStatus::Changed(p) if p==project("External"))
    );
    assert_eq!(
        copy.save_named(&project("Local")),
        Err(CopyError::NamedChanged)
    );
    assert_eq!(ProjectStore.load(&named).unwrap(), project("External"));
    let id = copy.id().to_owned();
    drop(copy);
    assert_eq!(copies.resume(&id).unwrap().project(), &project("Local"));
}
#[test]
fn missing_and_unreadable_named_files_leave_local_edits_recoverable() {
    let (_dir, copies, named) = fixture();
    let mut copy = copies.open_named(&named).unwrap();
    copy.checkpoint(&project("Retained")).unwrap();
    drop(copy);
    fs::remove_file(&named).unwrap();
    let mut copy = copies.open_named(&named).unwrap();
    assert_eq!(copy.project(), &project("Retained"));
    assert!(matches!(copy.named_status().unwrap(), NamedStatus::Missing));
    assert_eq!(
        copy.save_named(&project("Retained")),
        Err(CopyError::NamedMissing)
    );
    fs::write(&named, b"broken named file").unwrap();
    assert!(matches!(
        copy.named_status().unwrap(),
        NamedStatus::Unreadable(_)
    ));
    assert_eq!(
        copy.save_named(&project("Retained")),
        Err(CopyError::NamedChanged)
    );
    assert_eq!(fs::read(&named).unwrap(), b"broken named file");
}
#[test]
fn corrupt_local_records_are_visible_and_preserved_before_named_fallback() {
    let (_dir, copies, named) = fixture();
    let copy = copies.open_named(&named).unwrap();
    let path = copy.local_path().to_owned();
    drop(copy);
    let broken = b"incomplete local record";
    fs::write(&path, broken).unwrap();
    assert!(copies.list().unwrap()[0].summary.is_err());
    let copy = copies.open_named(&named).unwrap();
    assert_eq!(copy.project(), &project("Saved"));
    let archive = copy.preserved_copy().unwrap();
    assert_eq!(fs::read(archive).unwrap(), broken);
    assert_eq!(copies.list().unwrap().len(), 1);
}
#[test]
fn both_unreadable_candidates_and_future_local_versions_are_left_intact() {
    let (_dir, copies, named) = fixture();
    let copy = copies.open_named(&named).unwrap();
    let path = copy.local_path().to_owned();
    drop(copy);
    fs::write(&path, b"local invalid").unwrap();
    fs::write(&named, b"named invalid").unwrap();
    assert!(copies.open_named(&named).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"local invalid");
    assert_eq!(fs::read(&named).unwrap(), b"named invalid");
    ProjectStore.save(&named, &project("Saved")).unwrap();
    let future = br#"{"version":99,"future_data":"retain"}"#;
    fs::write(&path, future).unwrap();
    assert!(matches!(
        copies.open_named(&named),
        Err(CopyError::UnsupportedVersion(99))
    ));
    assert_eq!(fs::read(&path).unwrap(), future);
}
#[test]
fn failed_local_checkpoint_keeps_memory_and_previous_bytes() {
    let (_dir, copies, named) = fixture();
    let mut copy = copies.open_named(&named).unwrap();
    let path = copy.local_path().to_owned();
    let parked = path.with_extension("retained");
    let original = fs::read(&path).unwrap();
    fs::rename(&path, &parked).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(copy.checkpoint(&project("New")).is_err());
    assert_eq!(copy.project(), &project("Saved"));
    assert_eq!(fs::read(&parked).unwrap(), original);
    assert!(copy.save_named(&project("New")).is_err());
    assert_eq!(ProjectStore.load(&named).unwrap(), project("Saved"));
    fs::remove_dir(&path).unwrap();
    fs::rename(&parked, &path).unwrap();
    copy.checkpoint(&project("New")).unwrap();
    assert_eq!(copy.project(), &project("New"));
}
#[test]
fn failed_named_write_retains_a_retryable_journal_and_local_document() {
    let (_dir, copies, named) = fixture();
    let mut copy = copies.open_named(&named).unwrap();
    copy.checkpoint(&project("Local")).unwrap();
    let destination = copy.named_path().unwrap().unwrap();
    let result = copy.save_named_using(&project("Local"), |p, b| {
        if p == destination {
            Err("write unavailable".into())
        } else {
            write_atomic(p, b)
        }
    });
    assert!(result.is_err());
    assert!(copy.record.named.as_ref().unwrap().pending.is_some());
    assert_eq!(ProjectStore.load(&named).unwrap(), project("Saved"));
    let id = copy.id().to_owned();
    drop(copy);
    let mut copy = copies.resume(&id).unwrap();
    assert_eq!(copy.project(), &project("Local"));
    copy.save_named(&project("Local")).unwrap();
    assert_eq!(ProjectStore.load(&named).unwrap(), project("Local"));
    assert!(copy.record.named.as_ref().unwrap().pending.is_none());
}
#[test]
fn named_success_with_failed_acknowledgement_is_recognized_on_reopen() {
    let (_dir, copies, named) = fixture();
    let mut copy = copies.open_named(&named).unwrap();
    copy.checkpoint(&project("Newer local")).unwrap();
    let path = copy.local_path().to_owned();
    let mut local_writes = 0;
    let result = copy.save_named_using(&project("Older snapshot"), |p, b| {
        if p == path {
            local_writes += 1;
            if local_writes == 2 {
                return Err("acknowledgement unavailable".into());
            }
        }
        write_atomic(p, b)
    });
    assert!(result.is_err());
    assert_eq!(
        ProjectStore.load(&named).unwrap(),
        project("Older snapshot")
    );
    let disk = read_record(&path, copy.id()).unwrap();
    assert!(disk.named.unwrap().pending.is_some());
    let id = copy.id().to_owned();
    drop(copy);
    let mut copy = copies.resume(&id).unwrap();
    assert_eq!(copy.project(), &project("Newer local"));
    assert!(copy.record.named.as_ref().unwrap().pending.is_none());
    assert!(matches!(
        copy.named_status().unwrap(),
        NamedStatus::Unchanged
    ));
    copy.save_named(&project("Newer local")).unwrap();
    assert_eq!(ProjectStore.load(&named).unwrap(), project("Newer local"));
}
#[test]
fn an_external_edit_after_interrupted_acknowledgement_is_still_a_conflict() {
    let (_dir, copies, named) = fixture();
    let mut copy = copies.open_named(&named).unwrap();
    copy.checkpoint(&project("Local")).unwrap();
    let path = copy.local_path().to_owned();
    let mut writes = 0;
    assert!(
        copy.save_named_using(&project("Local"), |p, b| {
            writes += 1;
            if writes == 3 {
                Err("ack failed".into())
            } else {
                write_atomic(p, b)
            }
        })
        .is_err()
    );
    ProjectStore
        .save(&named, &project("External after write"))
        .unwrap();
    let id = copy.id().to_owned();
    drop(copy);
    let mut copy = copies.resume(&id).unwrap();
    assert_eq!(
        copy.save_named(&project("Local")),
        Err(CopyError::NamedChanged)
    );
    assert_eq!(
        ProjectStore.load(&named).unwrap(),
        project("External after write")
    );
    assert_eq!(read_record(&path, &id).unwrap().project, project("Local"));
}
#[test]
fn external_change_between_staging_and_write_is_rechecked() {
    let (_dir, copies, named) = fixture();
    let mut copy = copies.open_named(&named).unwrap();
    copy.checkpoint(&project("Local")).unwrap();
    let local = copy.local_path().to_owned();
    let result = copy.save_named_using(&project("Local"), |p, b| {
        write_atomic(p, b)?;
        if p == local {
            ProjectStore.save(&named, &project("External during staging"))?;
        }
        Ok(())
    });
    assert_eq!(result, Err(CopyError::NamedChanged));
    assert_eq!(
        ProjectStore.load(&named).unwrap(),
        project("External during staging")
    );
}
#[test]
fn discard_removes_only_the_selected_local_record_and_retains_lock_identity() {
    let (_dir, copies, named) = fixture();
    let copy = copies.open_named(&named).unwrap();
    let path = copy.local_path().to_owned();
    let other = copies.create_unnamed(&project("Other")).unwrap();
    copy.discard().unwrap();
    assert!(!path.exists());
    assert!(path.with_extension("lock").exists());
    assert_eq!(ProjectStore.load(&named).unwrap(), project("Saved"));
    assert_eq!(copies.list().unwrap().len(), 1);
    assert_eq!(other.project(), &project("Other"));
    assert!(copies.open_named(&named).is_ok());
}
#[test]
fn identifiers_cannot_escape_the_store_and_identity_changes_are_rejected() {
    let (_dir, copies, named) = fixture();
    for id in [
        "../escape",
        "named-../../escape",
        "untitled-../escape",
        "",
        "named-aa",
    ] {
        assert!(copies.resume(id).is_err());
    }
    let copy = copies.open_named(&named).unwrap();
    let path = copy.local_path().to_owned();
    let id = copy.id().to_owned();
    let mut record = copy.record.clone();
    drop(copy);
    record.named.as_mut().unwrap().path = NativePath::capture(Path::new("/other-file"));
    write_record(&path, &record).unwrap();
    assert!(matches!(copies.resume(&id), Err(CopyError::Invalid(_))));
    record.named.as_mut().unwrap().path = NativePath::capture(Path::new("relative-file"));
    let relative_id = record.named.as_ref().unwrap().path.key();
    write_record(&copies.root.join(format!("{relative_id}.json")), &record).unwrap();
    assert!(matches!(
        copies.resume(&relative_id),
        Err(CopyError::Invalid(_))
    ));
}
#[cfg(unix)]
#[test]
fn native_path_units_round_trip_and_filesystem_aliases_share_identity() {
    use std::os::unix::{ffi::OsStringExt, fs::symlink};
    let (dir, copies, _) = fixture();
    let native_units = dir.path().join(std::ffi::OsString::from_vec(
        b"name-\xff.wireframe".to_vec(),
    ));
    assert_eq!(
        NativePath::capture(&native_units).restore().unwrap(),
        native_units
    );
    let named = dir.path().join("name-ü.wireframe");
    ProjectStore.save(&named, &project("Native path")).unwrap();
    let copy = copies.open_named(&named).unwrap();
    assert_eq!(
        copy.named_path().unwrap().unwrap(),
        fs::canonicalize(&named).unwrap()
    );
    let alias = dir.path().join("alias.wireframe");
    symlink(&named, &alias).unwrap();
    assert!(matches!(copies.open_named(&alias), Err(CopyError::Busy)));
    let id = copy.id().to_owned();
    drop(copy);
    let mut copy = copies.resume(&id).unwrap();
    copy.save_named(&project("New")).unwrap();
    assert_eq!(ProjectStore.load(&named).unwrap(), project("New"));
}
#[cfg(unix)]
#[test]
fn local_record_symlinks_are_never_replaced_during_fallback() {
    use std::os::unix::fs::symlink;
    let (dir, copies, named) = fixture();
    let copy = copies.open_named(&named).unwrap();
    let path = copy.local_path().to_owned();
    drop(copy);
    fs::remove_file(&path).unwrap();
    let absent = dir.path().join("absent");
    symlink(&absent, &path).unwrap();
    assert!(matches!(
        copies.open_named(&named),
        Err(CopyError::Invalid(_))
    ));
    assert_eq!(fs::read_link(&path).unwrap(), absent);
}

#[test]
fn dirty_state_uses_project_content_and_clears_only_for_the_matching_snapshot() {
    let (_dir, copies, named) = fixture();
    // Formatting changes in the saved representation are not document edits.
    let compact = serde_json::to_vec(&project("Saved")).unwrap();
    fs::write(&named, compact).unwrap();
    let mut copy = copies.open_named(&named).unwrap();
    assert!(!copy.has_local_changes().unwrap());
    copy.checkpoint(&project("Older")).unwrap();
    copy.checkpoint(&project("Latest")).unwrap();
    assert!(copy.has_local_changes().unwrap());
    copy.save_named(&project("Older")).unwrap();
    assert!(copy.has_local_changes().unwrap());
    assert!(
        copies.list().unwrap()[0]
            .summary
            .as_ref()
            .unwrap()
            .has_local_changes
    );
    copy.save_named(&project("Latest")).unwrap();
    assert!(!copy.has_local_changes().unwrap());
    let id = copy.id().to_owned();
    drop(copy);
    assert!(!copies.resume(&id).unwrap().has_local_changes().unwrap());
}

#[test]
fn an_identifier_collision_cannot_replace_live_or_released_unnamed_work() {
    let (_dir, copies, _) = fixture();
    let id = "untitled-collision";
    let first = copies
        .create_unnamed_at(id, &project("Original"))
        .unwrap()
        .unwrap();
    let path = first.local_path().to_owned();
    let original = fs::read(&path).unwrap();
    assert!(
        copies
            .create_unnamed_at(id, &project("Other"))
            .unwrap()
            .is_none()
    );
    drop(first);
    assert!(
        copies
            .create_unnamed_at(id, &project("Other"))
            .unwrap()
            .is_none()
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(copies.resume(id).unwrap().project(), &project("Original"));
}
