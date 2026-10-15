// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::document::{Node, Size};
fn fixture(ctx: &egui::Context) -> WireframeApp {
    let mut app = WireframeApp::workspace(ctx, None);
    let mut p = Project {
        next_id: 10,
        ..Project::default()
    };
    p.pages[0].content.nodes = vec![Node::group(
        2,
        Point::new(100., 100.),
        vec![
            Node::control(3, "rectangle", Point::default(), Size::fixed(80., 40.)),
            Node::control(
                4,
                "rectangle",
                Point::new(120., 110.),
                Size::fixed(80., 40.),
            ),
        ],
    )];
    app.editor = Editor::new(p);
    app.editor.select(2, false);
    app.pan = Vec2::ZERO;
    app.start_group_crop(2).unwrap();
    app
}
fn frame(ctx: &egui::Context, app: &mut WireframeApp, events: Vec<egui::Event>) {
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1400., 1000.))),
            events,
            ..Default::default()
        },
        |ui| {
            app.shortcuts(ui.ctx());
            if app.image_crop.is_some() {
                app.crop_controls(ui);
            }
            app.canvas(ui);
        },
    );
    out.textures_delta.clear();
}
fn pointer(pos: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}
fn key(key: Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}
fn drag(ctx: &egui::Context, app: &mut WireframeApp, start: Pos2, end: Pos2) {
    frame(
        ctx,
        app,
        vec![egui::Event::PointerMoved(start), pointer(start, true)],
    );
    frame(ctx, app, vec![egui::Event::PointerMoved(end)]);
    frame(ctx, app, vec![pointer(end, false)]);
}
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 0.001, "{a} != {b}");
}
#[test]
fn region_drag_at_multiple_zooms_is_transient_and_escape_discards_it() {
    for zoom in [0.5, 1., 2.] {
        let ctx = egui::Context::default();
        let mut app = fixture(&ctx);
        app.zoom = zoom;
        frame(&ctx, &mut app, vec![]);
        let before = app.editor.document.clone();
        let b = app.image_crop.as_ref().unwrap().bounds();
        let start = app.screen_rect(b).center();
        drag(&ctx, &mut app, start, start + Vec2::new(20., 15.) * zoom);
        let after = app.image_crop.as_ref().unwrap().bounds();
        close(after.x, b.x + 20.);
        close(after.y, b.y + 15.);
        assert_eq!(app.editor.document, before);
        assert!(app.editor.undo_name().is_none());
        frame(&ctx, &mut app, vec![key(Key::Escape)]);
        assert!(app.image_crop.is_none());
        assert_eq!(app.editor.document, before);
        assert!(!app.editor.dirty());
    }
}
#[test]
fn outside_drag_keeps_frame_fixed_and_apply_is_one_undo_step() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    frame(&ctx, &mut app, vec![]);
    let before = app.editor.document.clone();
    let b = app.image_crop.as_ref().unwrap().bounds();
    let full = app.image_crop.as_ref().unwrap().full;
    let start = app.screen_rect(full).right_bottom() + Vec2::new(30., 30.);
    drag(&ctx, &mut app, start, start - Vec2::new(20., 15.));
    let edit = app.image_crop.as_ref().unwrap();
    assert_eq!(edit.bounds(), b);
    close(edit.full.x, full.x - 20.);
    close(edit.full.y, full.y - 15.);
    assert_eq!(app.editor.document, before);
    frame(&ctx, &mut app, vec![key(Key::Enter)]);
    assert!(app.image_crop.is_none());
    assert!(app.error.is_none(), "{:?}", app.error);
    let (actual, frame, _) = app.editor.group_crop_geometry(2).unwrap();
    assert_eq!(frame, b);
    close(actual.x, full.x - 20.);
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    assert!(app.editor.undo_name().is_none());
}
#[test]
fn all_resize_handles_clamp_inside_artwork_and_keep_a_ten_unit_minimum() {
    for (x, y) in [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ] {
        let ctx = egui::Context::default();
        let mut app = fixture(&ctx);
        let edit = app.image_crop.as_mut().unwrap();
        edit.drag = Some(CropDrag {
            gesture: Gesture::Resize(x, y),
            start: Pos2::ZERO,
            crop: edit.crop,
            full: edit.full,
        });
        edit.update_drag(
            Pos2::new(-f32::from(x) * 10000., -f32::from(y) * 10000.),
            2.,
        );
        assert!(edit.crop.valid());
        let b = edit.bounds();
        if x != 0 {
            close(b.w, 10.);
        }
        if y != 0 {
            close(b.h, 10.);
        }
        edit.update_drag(Pos2::new(f32::from(x) * 10000., f32::from(y) * 10000.), 0.5);
        assert!(edit.crop.valid());
        let b = edit.bounds();
        assert!(b.x >= edit.full.x && b.y >= edit.full.y);
        assert!(
            b.x + b.w <= edit.full.x + edit.full.w + 0.001
                && b.y + b.h <= edit.full.y + edit.full.h + 0.001
        );
    }
}
#[test]
fn held_crop_preview_blocks_save_and_undo_commands() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("preview.wireframe");
    app.file = Some(path.clone());
    let before = app.editor.document.clone();
    let mut save = key(Key::S);
    if let egui::Event::Key { modifiers, .. } = &mut save {
        *modifiers = Modifiers::COMMAND;
    }
    frame(&ctx, &mut app, vec![save]);
    assert!(!path.exists());
    assert!(app.image_crop.is_some());
    assert_eq!(app.editor.document, before);
}

#[test]
fn crop_drag_preserves_press_when_native_move_and_release_span_two_frames() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    frame(&ctx, &mut app, vec![]);
    let b = app.image_crop.as_ref().unwrap().bounds();
    let start = app.screen_rect(b).center();
    let middle = start + Vec2::new(12.5, 10.);
    let end = start + Vec2::new(25., 20.);
    frame(&ctx, &mut app, vec![egui::Event::PointerMoved(start)]);
    frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerMoved(start),
            pointer(start, true),
            egui::Event::PointerMoved(middle),
        ],
    );
    frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerMoved(end),
            egui::Event::PointerMoved(end),
            pointer(end, false),
        ],
    );
    let after = app.image_crop.as_ref().unwrap().bounds();
    close(after.x, b.x + 25.);
    close(after.y, b.y + 20.);
}
