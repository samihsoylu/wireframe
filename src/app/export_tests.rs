// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

#[test]
fn page_export_cannot_replace_its_open_project() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("editable.wireframe");
    ProjectStore.save(&path, &app.editor.document).unwrap();
    app.file = Some(path.clone());
    let before = std::fs::read(&path).unwrap();
    for png in [false, true] {
        let result = app.write_page_export(&path, png);
        assert!(
            result.is_err(),
            "export must refuse its editable project destination"
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(ProjectStore.load(&path).unwrap(), app.editor.document);
    }
}

#[test]
fn page_export_preserves_aliases_and_allows_separate_outputs() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("editable.wireframe");
    ProjectStore.save(&path, &app.editor.document).unwrap();
    app.file = Some(path.clone());
    let before = std::fs::read(&path).unwrap();
    let unchanged = app.editor.document.clone();
    let sub = dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    let alias = sub.join("..").join("editable.wireframe");
    assert!(app.write_page_export(&alias, false).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    #[cfg(unix)]
    {
        let link = dir.path().join("alias.svg");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(app.write_page_export(&link, false).is_err());
        assert!(
            std::fs::symlink_metadata(link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }
    for (png, name) in [(false, "drawing.svg"), (true, "drawing.png")] {
        let output = dir.path().join(name);
        std::fs::write(&output, b"previous export").unwrap();
        app.write_page_export(&output, png).unwrap();
        let bytes = std::fs::read(output).unwrap();
        if png {
            assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        } else {
            assert!(std::str::from_utf8(&bytes).unwrap().contains("<svg"));
        }
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(app.editor.document, unchanged);
    }
}
