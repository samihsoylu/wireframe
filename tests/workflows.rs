// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

use wireframe::{
    editor::Editor,
    model::*,
    scene,
    storage::{DocumentStore, JsonStore},
};

#[test]
fn add_move_undo_redo_and_new_edit_branch() {
    let mut e = Editor::default();
    e.add(Kind::Button, 50., 50.);
    let original = e.document.clone();
    e.move_by(1., 0.);
    e.move_by(10., 0.);
    assert_eq!(e.current().controls[0].bounds.x, 61.);
    e.undo();
    assert_eq!(e.current().controls[0].bounds.x, 51.);
    e.undo();
    assert_eq!(e.document, original);
    e.redo();
    assert_eq!(e.current().controls[0].bounds.x, 51.);
    e.move_by(0., 10.);
    assert!(e.redo_name().is_none());
    e.undo();
    e.undo();
    assert_eq!(e.document, original);
}
#[test]
fn one_drag_transaction_is_one_undo_and_cancel_restores() {
    let mut e = Editor::default();
    e.add(Kind::TextInput, 20., 20.);
    let before = e.document.clone();
    e.begin("Move");
    for _ in 0..100 {
        e.selected_mut(|c| c.bounds.x += 1.);
    }
    e.commit();
    assert_eq!(e.current().controls[0].bounds.x, 120.);
    e.undo();
    assert_eq!(e.document, before);
    e.begin("Resize");
    e.selected_mut(|c| c.bounds.w = 400.);
    e.cancel();
    assert_eq!(e.document, before);
}
#[test]
fn groups_paste_and_lock_preserve_ids_and_geometry() {
    let mut e = Editor::default();
    e.add(Kind::Button, 50., 50.);
    e.add(Kind::Label, 50., 100.);
    e.select_all();
    e.group();
    let ids: Vec<_> = e.current().controls.iter().map(|c| c.id).collect();
    let group = e.current().controls[0].group.unwrap();
    e.select(ids[0], false);
    assert_eq!(e.selection.len(), 2);
    e.move_by(10., 20.);
    assert_eq!(
        e.current().controls[1].bounds.y - e.current().controls[0].bounds.y,
        50.
    );
    e.duplicate();
    let pasted = &e.current().controls[2..];
    assert_ne!(pasted[0].group, Some(group));
    assert_eq!(pasted[0].group, pasted[1].group);
    assert!(!ids.contains(&pasted[0].id));
    e.document.validate().unwrap();
    e.lock(true);
    let before = e.document.clone();
    e.move_by(20., 20.);
    e.delete();
    assert_eq!(e.document, before);
}
#[test]
fn save_reopen_and_failed_save_preserve_valid_file() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("project.wireframe");
    let mut e = Editor::default();
    e.add(Kind::Table, 50., 50.);
    e.add_page();
    e.add(Kind::Checkbox, 50., 50.);
    JsonStore.save(&p, &e.document).unwrap();
    let bytes = std::fs::read(&p).unwrap();
    assert_eq!(JsonStore.load(&p).unwrap(), e.document);
    let mut bad = e.document.clone();
    bad.pages[0].controls[0].bounds.w = -1.;
    assert!(JsonStore.save(&p, &bad).is_err());
    assert_eq!(std::fs::read(&p).unwrap(), bytes);
    e.mark_saved();
    assert!(!e.dirty());
    e.move_by(10., 0.);
    assert!(e.dirty());
    e.undo();
    assert!(!e.dirty());
}
#[test]
fn validation_rejects_future_version_duplicate_ids_and_dangling_links() {
    let mut d = Document {
        format_version: 999,
        ..Document::default()
    };
    assert!(d.validate().is_err());
    d.format_version = FORMAT_VERSION;
    d.pages.push(d.pages[0].clone());
    assert!(d.validate().is_err());
    d.pages.pop();
    let id = d.allocate_id();
    let mut c = Control::new(id, Kind::Button, 0., 0.);
    c.link = Some(999);
    d.pages[0].controls.push(c);
    assert!(d.validate().is_err());
}
#[test]
fn delete_page_repairs_links_and_undo_restores() {
    let mut e = Editor::default();
    e.add(Kind::Button, 50., 50.);
    let first = e.page;
    e.add_page();
    let second = e.page;
    e.switch_page(first);
    e.edit("Link", |e| e.current_mut().controls[0].link = Some(second));
    e.switch_page(second);
    e.delete_page();
    assert_eq!(e.current().controls[0].link, None);
    e.document.validate().unwrap();
    e.undo();
    assert_eq!(e.document.pages.len(), 2);
    assert_eq!(e.document.pages[0].controls[0].link, Some(second));
}
#[test]
fn every_control_exports_valid_svg_and_png_escapes_markup() {
    let mut e = Editor::default();
    for (i, k) in Kind::ALL.into_iter().enumerate() {
        e.add(k, (i % 4) as f32 * 800., (i / 4) as f32 * 600.);
    }
    e.current_mut().controls[0].text = "<script>&\"".into();
    let svg = scene::svg(e.current(), true);
    assert!(!svg.contains("<script>"));
    assert!(svg.contains("&lt;script&gt;"));
    let options = resvg::usvg::Options::default();
    resvg::usvg::Tree::from_str(&svg, &options).unwrap();
    let page = Page {
        id: 1,
        name: "Export".into(),
        notes: String::new(),
        controls: vec![Control::new(2, Kind::Button, 50., 50.)],
    };
    let png = scene::png(&page, true).unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
}
#[test]
fn alignment_distribution_and_noop_history() {
    let mut e = Editor::default();
    for x in [0., 80., 300.] {
        e.add(Kind::Label, x, 40.);
    }
    e.select_all();
    e.distribute(0);
    assert_eq!(e.current().controls[1].bounds.x, 150.);
    e.align(1, 0);
    assert_eq!(e.undo_name(), Some("Distribute"));
    e.undo();
    assert_eq!(e.current().controls[1].bounds.x, 80.);
}

#[test]
fn marquee_includes_group_members_and_respects_locked_controls() {
    let mut e = Editor::default();
    e.add(Kind::Button, 10., 10.);
    e.add(Kind::Label, 500., 400.);
    e.select_all();
    e.group();
    e.selection.clear();
    e.select_area(Bounds::new(0., 0., 30., 30.), &Default::default());
    assert_eq!(e.selection.len(), 2);
    e.lock(true);
    e.select_area(Bounds::new(0., 0., 800., 800.), &Default::default());
    assert!(e.selection.is_empty());
}

#[test]
fn recovery_has_one_owner_and_releases_on_exit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("recovery.wireframe");
    let first = wireframe::storage::acquire_recovery(&path).unwrap();
    assert!(wireframe::storage::acquire_recovery(&path).is_err());
    drop(first);
    assert!(wireframe::storage::acquire_recovery(&path).is_ok());
}

#[test]
fn resize_edges_keep_opposite_edge_and_aspect() {
    let b = Bounds::new(100., 100., 120., 60.);
    assert_eq!(
        b.resized((-1, -1), (20., 10.), false),
        Bounds::new(120., 110., 100., 50.)
    );
    assert_eq!(
        b.resized((0, -1), (100., 10.), false),
        Bounds::new(100., 110., 120., 50.)
    );
    let r = b.resized((1, 0), (60., 0.), true);
    assert_eq!(r, Bounds::new(100., 85., 180., 90.));
    let tiny = b.resized((-1, -1), (1000., 1000.), false);
    assert_eq!(tiny.w, 8.);
    assert_eq!(tiny.h, 8.);
    assert_eq!(tiny.x + tiny.w, b.x + b.w);
}
