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
fn line_recovery_migration_preserves_clean_status_dirty_changes_and_input_bytes() {
    for changed in [false, true] {
        let (_dir, copies, named) = fixture();
        let mut old = project("Line project");
        old.format_version = 13;
        old.next_id = 101;
        old.pages[0].content.nodes.push(document::Node::control(
            100,
            "line",
            document::Point::new(20., 30.),
            document::Size::fixed(180., 60.),
        ));
        let bytes = serde_json::to_vec_pretty(&old).unwrap();
        fs::write(&named, &bytes).unwrap();
        let copy = copies.open_named(&named).unwrap();
        let id = copy.id().to_owned();
        let path = copy.local_path().to_owned();
        let mut envelope: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        drop(copy);
        envelope["project"] = serde_json::from_slice(&bytes).unwrap();
        envelope["named"]["base_content"] = serde_json::to_value(fingerprint(&bytes)).unwrap();
        if changed {
            envelope["project"]["title"] = "Unsaved line project".into();
        }
        let record = serde_json::to_vec(&envelope).unwrap();
        fs::write(&path, &record).unwrap();
        let resumed = copies.resume(&id).unwrap();
        assert_eq!(resumed.has_local_changes().unwrap(), changed);
        let node = &resumed.project().pages[0].content.nodes[0];
        assert_eq!(
            node.properties["legacy_rule_frame"],
            document::Value::Bool(true)
        );
        assert_eq!(node.origin, document::Point::new(20., 30.));
        assert!(
            matches!(&node.content, document::NodeContent::Control {size,..} if *size == document::Size::fixed(180.,60.))
        );
        assert_eq!(fs::read(&named).unwrap(), bytes);
        assert_eq!(fs::read(&path).unwrap(), record);
    }
}

#[test]
fn horizontal_arrow_recovery_migrates_without_changing_clean_or_dirty_state() {
    for changed in [false, true] {
        let (_dir, copies, named) = fixture();
        let mut old = project("Arrow project");
        old.format_version = 12;
        old.next_id = 101;
        old.pages[0].content.nodes.push(
            crate::catalog::get("arrow")
                .unwrap()
                .create(100, document::Point::new(20., 30.)),
        );
        let bytes = serde_json::to_vec_pretty(&old).unwrap();
        fs::write(&named, &bytes).unwrap();
        let copy = copies.open_named(&named).unwrap();
        let id = copy.id().to_owned();
        let path = copy.local_path().to_owned();
        let mut envelope: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        drop(copy);
        envelope["project"] = serde_json::from_slice(&bytes).unwrap();
        envelope["named"]["base_content"] = serde_json::to_value(fingerprint(&bytes)).unwrap();
        if changed {
            envelope["project"]["title"] = "Unsaved arrow project".into();
        }
        let record = serde_json::to_vec(&envelope).unwrap();
        fs::write(&path, &record).unwrap();
        let resumed = copies.resume(&id).unwrap();
        assert_eq!(resumed.has_local_changes().unwrap(), changed);
        let node = &resumed.project().pages[0].content.nodes[0];
        let arrow = crate::arrows::node_geometry(node).unwrap().0;
        assert_eq!(arrow.start, document::Point::new(0., 50.));
        assert_eq!(arrow.end, document::Point::new(150., 50.));
        assert_eq!(node.properties["start_head"], document::Value::Bool(false));
        assert_eq!(fs::read(&named).unwrap(), bytes);
        assert_eq!(fs::read(&path).unwrap(), record);
    }
}

#[test]
fn old_recovery_keeps_clean_status_and_unsaved_edits() {
    for (version, changed) in [
        (3, false),
        (3, true),
        (4, false),
        (4, true),
        (5, false),
        (5, true),
        (6, false),
        (6, true),
        (7, false),
        (7, true),
        (8, false),
        (8, true),
        (9, false),
        (9, true),
        (10, false),
        (10, true),
        (11, false),
        (11, true),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let copies = WorkingCopies::new(dir.path().join("copies")).unwrap();
        let named = dir.path().join("Existing.wireframe");
        let mut old = project("Version three");
        old.format_version = version;
        old.next_id = 101;
        old.pages[0].content.nodes.push(
            crate::catalog::get("toggle")
                .unwrap()
                .create(100, document::Point::default()),
        );
        let bytes = serde_json::to_vec_pretty(&old).unwrap();
        fs::write(&named, &bytes).unwrap();
        let copy = copies.open_named(&named).unwrap();
        let id = copy.id().to_owned();
        let path = copy.local_path().to_owned();
        let mut envelope: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        drop(copy);
        envelope["project"] = serde_json::from_slice(&bytes).unwrap();
        envelope["named"]["base_content"] = serde_json::to_value(fingerprint(&bytes)).unwrap();
        if changed {
            envelope["project"]["title"] = "Unsaved change".into();
        }
        let record = serde_json::to_vec(&envelope).unwrap();
        fs::write(&path, &record).unwrap();
        let resumed = copies.resume(&id).unwrap();
        assert_eq!(resumed.project().format_version, document::FORMAT_VERSION);
        assert_eq!(
            resumed.project().pages[0].content.nodes[0].properties["checked"],
            document::Value::Bool(false)
        );
        assert_eq!(resumed.has_local_changes().unwrap(), changed);
        assert_eq!(
            resumed.project().title,
            if changed {
                "Unsaved change"
            } else {
                "Version three"
            }
        );
        assert_eq!(fs::read(&path).unwrap(), record);
        assert_eq!(fs::read(&named).unwrap(), bytes);
    }
}

#[test]
fn legacy_component_recovery_migrates_clean_fingerprint_without_touching_files() {
    for (version, changed) in [
        (5, false),
        (5, true),
        (6, false),
        (6, true),
        (7, false),
        (7, true),
        (8, false),
        (8, true),
        (9, false),
        (9, true),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let copies = WorkingCopies::new(dir.path().join("copies")).unwrap();
        let named = dir.path().join("Components.wireframe");
        let mut editor = crate::workspace::Editor::default();
        editor.add(crate::model::Kind::Button, 30., 40.);
        editor.create_component("Shared".into()).unwrap();
        let mut old = editor.document;
        if version == 5 {
            old.libraries.clear();
            old.default_library = None;
        } else {
            old.libraries[0].content.nodes[0].markup = true;
        }
        old.format_version = version;
        let bytes = serde_json::to_vec_pretty(&old).unwrap();
        fs::write(&named, &bytes).unwrap();
        let copy = copies.open_named(&named).unwrap();
        let id = copy.id().to_owned();
        let path = copy.local_path().to_owned();
        let mut envelope: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        drop(copy);
        envelope["project"] = serde_json::from_slice(&bytes).unwrap();
        envelope["named"]["base_content"] = serde_json::to_value(fingerprint(&bytes)).unwrap();
        if changed {
            envelope["project"]["title"] = "Unsaved change".into();
        }
        let record = serde_json::to_vec(&envelope).unwrap();
        fs::write(&path, &record).unwrap();
        let resumed = copies.resume(&id).unwrap();
        assert_eq!(resumed.has_local_changes().unwrap(), changed);
        assert_eq!(resumed.project().definitions, old.definitions);
        if version == 5 {
            assert_eq!(resumed.project().pages, old.pages);
        } else {
            let root = resumed
                .project()
                .resolve(&resumed.project().pages[0].content.nodes[0])
                .unwrap()
                .root;
            assert_eq!(root.markup, version >= 7);
            assert!(resumed.project().libraries[0].content.nodes[0].markup);
        }
        assert_eq!(resumed.project().libraries.len(), 1);
        assert_eq!(fs::read(&path).unwrap(), record);
        assert_eq!(fs::read(&named).unwrap(), bytes);
    }
}

#[test]
fn local_component_locks_survive_recovery_and_named_save() {
    let (_dir, copies, named) = fixture();
    let mut editor = crate::workspace::Editor::default();
    editor.add(crate::model::Kind::TextInput, 30., 40.);
    let root = *editor.selection.iter().next().unwrap();
    editor.create_component("Field".into()).unwrap();
    assert!(editor.enter(root));
    let child = editor.nodes()[0].id;
    editor.select(child, false);
    editor.lock(true);
    let mut copy = copies.open_named(&named).unwrap();
    copy.checkpoint(&editor.document).unwrap();
    let id = copy.id().to_owned();
    drop(copy);
    let mut resumed = copies.resume(&id).unwrap();
    assert_eq!(resumed.project(), &editor.document);
    resumed.save_named(&editor.document).unwrap();
    let mut reopened = crate::workspace::Editor::new(ProjectStore.load(&named).unwrap());
    assert!(reopened.enter(root));
    assert!(reopened.own_locked(child));
    assert!(!reopened.document.definitions[0].content.nodes[0].locked);
}
#[test]
fn library_import_correspondence_survives_recovery_and_named_reopen() {
    let (_dir, copies, named) = fixture();
    let mut editor = crate::workspace::Editor::default();
    editor.add(crate::model::Kind::Button, 30., 40.);
    let definition = editor.create_component("Imported action".into()).unwrap();
    let historical = editor.document.allocate_id().unwrap();
    editor.document.libraries[0]
        .imports
        .push(document::LibraryImport {
            project: "foreign-project".into(),
            library: 90,
            definitions: std::collections::BTreeMap::from([(91, definition), (92, historical)]),
        });
    let mut copy = copies.open_named(&named).unwrap();
    copy.checkpoint(&editor.document).unwrap();
    let id = copy.id().to_owned();
    drop(copy);
    let mut resumed = copies.resume(&id).unwrap();
    assert!(resumed.has_local_changes().unwrap());
    assert_eq!(resumed.project(), &editor.document);
    resumed.save_named(&editor.document).unwrap();
    assert!(!resumed.has_local_changes().unwrap());
    drop(resumed);
    assert_eq!(copies.resume(&id).unwrap().project(), &editor.document);
    assert_eq!(ProjectStore.load(&named).unwrap(), editor.document);
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
        matches!(copy.named_status().unwrap(),NamedStatus::Changed(p) if *p==project("External"))
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

#[test]
fn save_as_reserves_new_destination_and_keeps_source_until_success() {
    let (dir, copies, _) = fixture();
    let source = copies.create_unnamed(&project("Source")).unwrap();
    let destination = dir.path().join("new.wireframe");
    let mut reserved = copies
        .prepare_save_as(&destination, source.project())
        .unwrap();
    assert!(!destination.exists());
    assert!(source.local_path().exists());
    assert!(matches!(
        copies.prepare_save_as(&destination, &project("Other")),
        Err(CopyError::Busy)
    ));
    reserved.save_named(source.project()).unwrap();
    assert_eq!(ProjectStore.load(&destination).unwrap(), project("Source"));
    assert!(source.local_path().exists());
    assert!(!reserved.has_local_changes().unwrap());
    source.discard().unwrap();
    assert_eq!(reserved.project(), &project("Source"));
}
#[test]
fn save_as_preserves_previous_local_work_and_checks_destination_after_reservation() {
    let (_dir, copies, named) = fixture();
    let mut old = copies.open_named(&named).unwrap();
    old.checkpoint(&project("Previous local edits")).unwrap();
    let original = fs::read(old.local_path()).unwrap();
    drop(old);
    let mut reserved = copies
        .prepare_save_as(&named, &project("Replacement"))
        .unwrap();
    assert_eq!(
        fs::read(reserved.preserved_copy().unwrap()).unwrap(),
        original
    );
    assert_eq!(ProjectStore.load(&named).unwrap(), project("Saved"));
    ProjectStore
        .save(&named, &project("External after selection"))
        .unwrap();
    assert_eq!(
        reserved.save_named(&project("Replacement")),
        Err(CopyError::NamedChanged)
    );
    assert_eq!(
        ProjectStore.load(&named).unwrap(),
        project("External after selection")
    );
    assert_eq!(reserved.project(), &project("Replacement"));
}
#[test]
fn new_destination_created_by_someone_else_is_not_overwritten() {
    let (dir, copies, _) = fixture();
    let destination = dir.path().join("new.wireframe");
    let mut copy = copies
        .prepare_save_as(&destination, &project("Mine"))
        .unwrap();
    ProjectStore.save(&destination, &project("Theirs")).unwrap();
    assert_eq!(
        copy.save_named(&project("Mine")),
        Err(CopyError::NamedChanged)
    );
    assert_eq!(ProjectStore.load(&destination).unwrap(), project("Theirs"));
}
#[test]
fn save_as_can_explicitly_replace_unreadable_named_bytes_but_not_future_local_metadata() {
    let (_dir, copies, named) = fixture();
    fs::write(&named, b"invalid selected file").unwrap();
    let mut copy = copies
        .prepare_save_as(&named, &project("Replacement"))
        .unwrap();
    copy.save_named(&project("Replacement")).unwrap();
    let local = copy.local_path().to_owned();
    drop(copy);
    fs::write(&local, br#"{"version":99}"#).unwrap();
    assert!(matches!(
        copies.prepare_save_as(&named, &project("Another")),
        Err(CopyError::UnsupportedVersion(99))
    ));
    assert_eq!(ProjectStore.load(&named).unwrap(), project("Replacement"));
}

#[test]
fn missing_fingerprint_fields_are_not_treated_as_new_file_reservations() {
    let (_dir, copies, named) = fixture();
    let copy = copies.open_named(&named).unwrap();
    let path = copy.local_path().to_owned();
    let id = copy.id().to_owned();
    let original: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    drop(copy);
    for field in ["base", "base_content", "pending"] {
        let mut value = original.clone();
        value["named"].as_object_mut().unwrap().remove(field);
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(
            matches!(copies.resume(&id), Err(CopyError::Invalid(_))),
            "Missing {field} must be rejected"
        );
    }
}

// Exact pretty-printed version-2 project bytes, before independent alternate notes.
const V2_PROJECT: &[u8] = br#"{
  "format_version": 2,
  "title": "Existing work",
  "next_id": 3,
  "settings": {
    "sketch": true,
    "font_size": 13.0
  },
  "pages": [
    {
      "id": 1,
      "name": "Page",
      "notes": "Official notes",
      "parent": null,
      "content": {
        "nodes": []
      },
      "alternates": [
        {
          "id": 2,
          "name": "Compact",
          "content": {
            "nodes": []
          }
        }
      ]
    }
  ],
  "definitions": [],
  "assets": []
}"#;

#[test]
fn version_two_recovery_records_migrate_without_losing_edits_or_changing_clean_status() {
    for changed in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let copies = WorkingCopies::new(dir.path().join("copies")).unwrap();
        let named = dir.path().join("Existing.wireframe");
        fs::write(&named, V2_PROJECT).unwrap();
        let copy = copies.open_named(&named).unwrap();
        let id = copy.id().to_owned();
        let path = copy.local_path().to_owned();
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        drop(copy);
        value["version"] = 1.into();
        value["project"] = serde_json::from_slice(V2_PROJECT).unwrap();
        value["named"]["base_content"] = serde_json::to_value(fingerprint(V2_PROJECT)).unwrap();
        if changed {
            value["project"]["title"] = "Unsaved changes".into();
        }
        let original = serde_json::to_vec(&value).unwrap();
        fs::write(&path, &original).unwrap();
        let resumed = copies.resume(&id).unwrap();
        assert_eq!(resumed.project().format_version, document::FORMAT_VERSION);
        assert_eq!(
            resumed.project().pages[0].alternates[0].notes,
            "Official notes"
        );
        assert_eq!(
            resumed.project().title,
            if changed {
                "Unsaved changes"
            } else {
                "Existing work"
            }
        );
        assert_eq!(resumed.has_local_changes().unwrap(), changed);
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::read(&named).unwrap(), V2_PROJECT);
        drop(resumed);
        let listed = copies.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(
            listed[0].summary.as_ref().unwrap().has_local_changes,
            changed
        );
    }
}

#[test]
fn version_two_pending_save_migrates_its_content_baseline_after_reconciling_raw_bytes() {
    for changed in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let copies = WorkingCopies::new(dir.path().join("copies")).unwrap();
        let named = dir.path().join("Existing.wireframe");
        fs::write(&named, V2_PROJECT).unwrap();
        let copy = copies.open_named(&named).unwrap();
        let id = copy.id().to_owned();
        let path = copy.local_path().to_owned();
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        drop(copy);
        value["version"] = 1.into();
        value["project"] = serde_json::from_slice(V2_PROJECT).unwrap();
        value["named"]["base"] = serde_json::to_value([0u8; 32]).unwrap();
        value["named"]["base_content"] = serde_json::to_value([0u8; 32]).unwrap();
        value["named"]["pending"] = serde_json::to_value(fingerprint(V2_PROJECT)).unwrap();
        if changed {
            value["project"]["title"] = "Newer local changes".into();
        }
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        let copy = copies.resume(&id).unwrap();
        assert_eq!(copy.has_local_changes().unwrap(), changed);
        assert!(matches!(
            copy.named_status().unwrap(),
            NamedStatus::Unchanged
        ));
        assert!(copy.record.named.as_ref().unwrap().pending.is_none());
        assert_eq!(copy.record.version, VERSION);
        assert_eq!(fs::read(&named).unwrap(), V2_PROJECT);
        drop(copy);
        assert_eq!(
            copies.resume(&id).unwrap().has_local_changes().unwrap(),
            changed
        );
    }
}

#[test]
fn version_two_archives_restore_independently_and_future_embedded_documents_are_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let copies = WorkingCopies::new(dir.path().join("copies")).unwrap();
    let name = "previous-working-legacy.json";
    let archive = copies.root.join(name);
    let value = serde_json::json!({"version":1,"named":null,"project":serde_json::from_slice::<serde_json::Value>(V2_PROJECT).unwrap()});
    let bytes = serde_json::to_vec(&value).unwrap();
    fs::write(&archive, &bytes).unwrap();
    assert!(copies.list_archives().unwrap()[0].summary.is_ok());
    let restored = copies.restore_archive(name).unwrap();
    assert_eq!(
        restored.project().pages[0].alternates[0].notes,
        "Official notes"
    );
    assert!(restored.named_path().unwrap().is_none());
    assert_eq!(fs::read(&archive).unwrap(), bytes);
    let id = restored.id().to_owned();
    let path = restored.local_path().to_owned();
    let mut future: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    drop(restored);
    future["project"]["format_version"] = 99.into();
    let bytes = serde_json::to_vec(&future).unwrap();
    fs::write(&path, &bytes).unwrap();
    assert!(matches!(
        copies.resume(&id),
        Err(CopyError::UnsupportedVersion(99))
    ));
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn retained_pages_survive_working_copy_restart_and_named_save() {
    let (_dir, copies, named) = fixture();
    let mut editor = crate::workspace::Editor::default();
    editor.add(crate::model::Kind::Button, 40., 50.);
    editor.create_version("Compact").unwrap();
    editor.set_notes("Hidden alternate notes".into());
    editor.commit();
    editor.add_page();
    editor.document.pages[0].trashed = true;
    editor.document.pages[1].parent = Some(1);
    editor.document.validate().unwrap();
    let mut copy = copies.open_named(&named).unwrap();
    copy.checkpoint(&editor.document).unwrap();
    let id = copy.id().to_owned();
    drop(copy);
    let mut resumed = copies.resume(&id).unwrap();
    assert!(resumed.has_local_changes().unwrap());
    assert_eq!(resumed.project(), &editor.document);
    resumed.save_named(&editor.document).unwrap();
    assert!(!resumed.has_local_changes().unwrap());
    assert_eq!(ProjectStore.load(&named).unwrap(), editor.document);
    drop(resumed);
    let reopened = copies.open_named(&named).unwrap();
    assert!(!reopened.has_local_changes().unwrap());
    assert_eq!(reopened.project(), &editor.document);
}

#[test]
fn retained_libraries_survive_recovery_save_and_restore_without_rewriting_instances() {
    let (_dir, copies, named) = fixture();
    let mut editor = crate::workspace::Editor::default();
    editor.add(crate::model::Kind::Button, 40., 50.);
    let source = editor.create_component("Action".into()).unwrap();
    let library = editor.document.component_source(source).unwrap().0.id;
    editor
        .edit_source_notes(source, "Preserved source notes")
        .unwrap();
    let original = editor.document.clone();
    editor.trash_libraries(&[library]).unwrap();
    let mut copy = copies.open_named(&named).unwrap();
    copy.checkpoint(&editor.document).unwrap();
    let id = copy.id().to_owned();
    drop(copy);
    let mut resumed = copies.resume(&id).unwrap();
    assert!(resumed.has_local_changes().unwrap());
    assert_eq!(resumed.project(), &editor.document);
    resumed.save_named(&editor.document).unwrap();
    assert!(!resumed.has_local_changes().unwrap());
    drop(resumed);
    let reopened = copies.open_named(&named).unwrap();
    assert!(!reopened.has_local_changes().unwrap());
    assert_eq!(reopened.project(), &editor.document);
    assert_eq!(ProjectStore.load(&named).unwrap(), editor.document);
    let mut restored = crate::workspace::Editor::new(reopened.project().clone());
    assert!(!restored.document.definition_available(source));
    restored.restore_libraries(&[library]).unwrap();
    assert_eq!(restored.document, original);
}

#[test]
fn permanently_removed_destinations_survive_recovery_and_named_reopen() {
    let (_dir, copies, named) = fixture();
    let mut editor = crate::workspace::Editor::default();
    editor.add_page();
    editor.add(crate::model::Kind::Button, 40., 50.);
    editor.document.pages[1].content.nodes[0].properties.insert(
        "link".into(),
        document::Value::Link(document::Link::Page(1)),
    );
    editor.trash_pages(&[1]).unwrap();
    editor.permanently_remove_pages(&[1]).unwrap();
    let expected = editor.document;
    let mut copy = copies.open_named(&named).unwrap();
    copy.checkpoint(&expected).unwrap();
    let id = copy.id().to_owned();
    drop(copy);
    let mut resumed = copies.resume(&id).unwrap();
    assert!(resumed.has_local_changes().unwrap());
    assert_eq!(resumed.project(), &expected);
    resumed.save_named(&expected).unwrap();
    assert!(!resumed.has_local_changes().unwrap());
    assert_eq!(ProjectStore.load(&named).unwrap(), expected);
    drop(resumed);
    let reopened = copies.open_named(&named).unwrap();
    assert!(!reopened.has_local_changes().unwrap());
    assert_eq!(reopened.project(), &expected);
}

#[test]
fn asset_retention_migration_preserves_old_clean_and_dirty_recovery_baselines() {
    for changed in [false, true] {
        let (_dir, copies, named) = fixture();
        let mut old = project("Image project");
        old.format_version = 14;
        old.next_id = 11;
        old.assets.push(document::Asset {
            id: 10,
            trashed: false,
            name: "Photo".into(),
            media_type: "image/png".into(),
            bytes: vec![1, 2, 3],
        });
        let bytes = serde_json::to_vec_pretty(&old).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("trashed"));
        fs::write(&named, &bytes).unwrap();
        let copy = copies.open_named(&named).unwrap();
        let id = copy.id().to_owned();
        let path = copy.local_path().to_owned();
        let mut envelope: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        drop(copy);
        envelope["project"] = serde_json::from_slice(&bytes).unwrap();
        envelope["named"]["base_content"] = serde_json::to_value(fingerprint(&bytes)).unwrap();
        if changed {
            envelope["project"]["title"] = "Unsaved image project".into();
        }
        let record = serde_json::to_vec(&envelope).unwrap();
        fs::write(&path, &record).unwrap();
        let resumed = copies.resume(&id).unwrap();
        assert_eq!(resumed.has_local_changes().unwrap(), changed);
        assert_eq!(resumed.project().format_version, document::FORMAT_VERSION);
        assert!(!resumed.project().assets[0].trashed);
        assert_eq!(resumed.project().assets, old.assets);
        assert_eq!(fs::read(&named).unwrap(), bytes);
        assert_eq!(fs::read(&path).unwrap(), record);
    }
}

#[test]
fn dropped_owner_releases_lock_even_while_a_duplicate_descriptor_survives() {
    let (_dir, copies, named) = fixture();
    let copy = copies.open_named(&named).unwrap();
    let id = copy.id().to_owned();
    // A process spawn can temporarily inherit the same open file description.
    let inherited_descriptor = copy._lock.file.try_clone().unwrap();
    assert!(matches!(copies.resume(&id), Err(CopyError::Busy)));
    drop(copy);
    let reopened = copies.resume(&id).expect("the original owner has finished");
    assert_eq!(reopened.project().title, "Saved");
    drop(inherited_descriptor);
    // Closing the old descriptor cannot release a new owner's separate lock.
    assert!(matches!(copies.resume(&id), Err(CopyError::Busy)));
    drop(reopened);
    assert!(copies.resume(&id).is_ok());
}
