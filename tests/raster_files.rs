// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::{collections::HashSet, fs, io::Write, path::Path};
use wireframe::raster::{
    names::{self, FileName, Format},
    publication::{Attempt, PendingOutput},
};

fn pending(path: &Path, bytes: &[u8]) -> PendingOutput {
    PendingOutput::prepare(path.to_owned(), |file| {
        file.write_all(bytes).map_err(|e| e.to_string())
    })
    .unwrap()
}
fn files(dir: &Path) -> Vec<String> {
    let mut paths: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    paths.sort();
    paths
}
#[test]
fn names_sanitize_before_deduplication_and_preserve_real_numbered_names() {
    let labels = [
        "Contact/Phone",
        "Contact:Phone",
        "Contact_Phone_2",
        "contact_phone",
    ]
    .map(String::from);
    let names = names::unique(&labels, Format::Png).unwrap();
    assert_eq!(
        names.iter().map(FileName::text).collect::<Vec<_>>(),
        [
            "Contact_Phone.png",
            "Contact_Phone_3.png",
            "Contact_Phone_2.png",
            "contact_phone_4.png",
        ]
    );
    assert_eq!(
        names
            .iter()
            .map(FileName::key)
            .collect::<HashSet<_>>()
            .len(),
        names.len()
    );
}
#[test]
fn portable_names_handle_unicode_devices_controls_and_traversal() {
    let labels = [
        "cafe\u{301}",
        "Café",
        "CON",
        "con.txt",
        "LPT²",
        "COM9",
        "..",
        "   ",
        "  résumé. ",
        "a\0b\nc%?",
    ]
    .map(String::from);
    let out = names::unique(&labels, Format::Png).unwrap();
    assert_eq!(
        out.iter().map(FileName::text).collect::<Vec<_>>(),
        [
            "café.png",
            "Café_2.png",
            "_CON.png",
            "_con.txt.png",
            "_LPT².png",
            "_COM9.png",
            "Untitled.png",
            "Untitled_2.png",
            "  résumé.png",
            "a_b_c__.png",
        ]
    );
    let zip = names::zip_entry(
        "../../C:\\folder",
        &FileName::new("../Landing\\contact", Format::Png),
    );
    let parts: Vec<_> = zip.split('/').collect();
    assert_eq!(parts.len(), 2);
    assert!(
        parts
            .iter()
            .all(|p| !matches!(*p, "." | "..") && !p.contains('\\'))
    );
    assert_eq!(names::collision_key("σ.png"), names::collision_key("ς.PNG"));
}
#[test]
fn long_multibyte_names_truncate_without_splitting_characters_or_losing_suffixes() {
    let label = "😀".repeat(100);
    let out = names::unique(
        &[label.clone(), format!("{label}other"), label],
        Format::Png,
    )
    .unwrap();
    assert_eq!(
        out.iter().map(FileName::key).collect::<HashSet<_>>().len(),
        3
    );
    for name in out {
        let value = name.text();
        assert!(value.len() <= 184);
        assert!(value.encode_utf16().count() <= 184);
        assert!(value.ends_with(".png"));
        assert!(!value.contains('\u{fffd}'));
    }
}
#[test]
fn new_destination_is_published_only_after_complete_encoding() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("page.png");
    let output = pending(&target, b"complete output");
    assert!(!target.exists());
    let Attempt::Saved(saved) = output.publish_new().unwrap() else {
        panic!("unexpected conflict")
    };
    assert_eq!(saved.path, target);
    assert_eq!(fs::read(&target).unwrap(), b"complete output");
    assert_eq!(files(dir.path()), vec!["page.png"]);
}
#[test]
fn a_destination_created_after_encoding_is_never_silently_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("page.png");
    let output = pending(&target, b"new content");
    fs::write(&target, b"another writer").unwrap();
    let Attempt::Conflict(output) = output.publish_new().unwrap() else {
        panic!("overwrote another writer")
    };
    assert_eq!(output.target(), target);
    assert_eq!(fs::read(&target).unwrap(), b"another writer");
    output.discard().unwrap();
    assert_eq!(files(dir.path()), vec!["page.png"]);
}
#[test]
fn failed_encoding_and_cancellation_preserve_existing_files_and_remove_temporaries() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("page.png");
    fs::write(&target, b"original").unwrap();
    let error = PendingOutput::prepare(target.clone(), |file| {
        file.write_all(b"partial").unwrap();
        Err("encoder failed".into())
    })
    .err()
    .unwrap();
    assert_eq!(error, "encoder failed");
    let cancelled = pending(&target, b"finished but cancelled").keep_both(
        &FileName::new("page", Format::Png),
        &HashSet::new(),
        || Err("Cancelled".into()),
    );
    assert_eq!(cancelled.unwrap_err(), "Cancelled");
    assert_eq!(fs::read(&target).unwrap(), b"original");
    assert_eq!(files(dir.path()), vec!["page.png"]);
}
#[test]
fn explicit_replace_swaps_complete_content_and_rejects_folders() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("page.png");
    fs::write(&target, b"original").unwrap();
    pending(&target, b"replacement").replace().unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"replacement");
    let folder = dir.path().join("folder.png");
    fs::create_dir(&folder).unwrap();
    let error = pending(&folder, b"not a folder").replace().unwrap_err();
    assert!(error.contains("folder"));
    assert!(folder.is_dir());
    assert_eq!(files(dir.path()), vec!["folder.png", "page.png"]);
}
#[test]
fn keep_both_respects_case_unicode_and_names_reserved_for_later_pages() {
    let dir = tempfile::tempdir().unwrap();
    let base = FileName::new("Café", Format::Png);
    let target = dir.path().join(base.text());
    fs::write(&target, b"original").unwrap();
    fs::write(dir.path().join("CAFE\u{301}_1.PNG"), b"existing equivalent").unwrap();
    let planned = HashSet::from([base.key(), base.numbered(2).key()]);
    let saved = pending(&target, b"copy")
        .keep_both(&base, &planned, || Ok(()))
        .unwrap();
    assert_eq!(saved.path.file_name().unwrap(), "Café_3.png");
    assert_eq!(fs::read(&target).unwrap(), b"original");
    assert_eq!(fs::read(saved.path).unwrap(), b"copy");
    assert_eq!(
        fs::read(dir.path().join("CAFE\u{301}_1.PNG")).unwrap(),
        b"existing equivalent"
    );
}
#[test]
fn simultaneous_keep_both_publishers_preserve_every_output() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("page.png");
    fs::write(&target, b"original").unwrap();
    let gate = std::sync::Arc::new(std::sync::Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|n| {
            let target = target.clone();
            let gate = gate.clone();
            std::thread::spawn(move || {
                let bytes = format!("worker {n}");
                let output = pending(&target, bytes.as_bytes());
                gate.wait();
                let saved = output
                    .keep_both(&FileName::new("page", Format::Png), &HashSet::new(), || {
                        Ok(())
                    })
                    .unwrap();
                (saved.path, bytes)
            })
        })
        .collect();
    let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(
        results.iter().map(|(p, _)| p).collect::<HashSet<_>>().len(),
        8
    );
    for (path, bytes) in results {
        assert_eq!(fs::read(path).unwrap(), bytes.as_bytes());
    }
    assert_eq!(fs::read(&target).unwrap(), b"original");
    assert_eq!(files(dir.path()).len(), 9);
}
#[cfg(unix)]
#[test]
fn symbolic_links_are_conflicts_and_never_redirect_writes() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original.png");
    let link = dir.path().join("link.png");
    fs::write(&original, b"original").unwrap();
    std::os::unix::fs::symlink(&original, &link).unwrap();
    let Attempt::Conflict(output) = pending(&link, b"new").publish_new().unwrap() else {
        panic!("followed link")
    };
    assert!(output.replace().is_err());
    assert_eq!(fs::read(&original).unwrap(), b"original");
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    let dangling = dir.path().join("dangling.png");
    std::os::unix::fs::symlink(dir.path().join("absent"), &dangling).unwrap();
    let Attempt::Conflict(output) = pending(&dangling, b"new").publish_new().unwrap() else {
        panic!("overwrote dangling link")
    };
    output.discard().unwrap();
    assert!(!dir.path().join("absent").exists());
    assert_eq!(
        files(dir.path()),
        vec!["dangling.png", "link.png", "original.png"]
    );
}
