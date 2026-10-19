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
fn hidden_fixture(ctx: &egui::Context) -> WireframeApp {
    let mut app = fixture(ctx);
    app.image_crop = None;
    let mut p = app.editor.document.clone();
    let wireframe::document::NodeContent::Group { children } =
        &mut p.pages[0].content.nodes[0].content
    else {
        panic!("Expected group");
    };
    children[0] = Node::control(3, "rectangle", Point::default(), Size::fixed(81., 40.));
    children[1] = Node::control(4, "rectangle", Point::new(299., 0.), Size::fixed(81., 40.));
    children[1].markup = true;
    app.editor = Editor::new(p);
    app.editor.select(2, false);
    app.editor.set_markup_visible(false);
    app.start_group_crop(2).unwrap();
    app
}
#[test]
fn hidden_group_crop_preview_commit_reentry_and_undo_preserve_full_descendants() {
    let ctx = egui::Context::default();
    let mut app = hidden_fixture(&ctx);
    let before = app.editor.document.clone();
    let edit = app.image_crop.as_ref().unwrap();
    assert_eq!(edit.full, Bounds::new(100., 100., 81., 40.));
    assert_eq!(edit.bounds(), Bounds::new(100., 100., 65., 32.));
    let CropPreview::Group(scene) = &edit.preview else {
        panic!("Expected group preview")
    };
    assert_eq!(scene.bounds, Bounds::new(100., 100., 81., 40.));
    frame(&ctx, &mut app, vec![key(Key::Enter)]);
    assert!(app.error.is_none(), "{:?}", app.error);
    let (full, visible, crop) = app.editor.group_crop_geometry(2).unwrap();
    assert_eq!(full, Bounds::new(100., 100., 380., 40.));
    assert_eq!(visible, Bounds::new(100., 100., 305., 32.));
    assert_eq!(
        crop,
        Some(Crop {
            x: 0.,
            y: 0.,
            width: 0.802,
            height: 0.8
        })
    );
    assert_eq!(
        app.editor.selection_geometry().unwrap().bounds,
        Bounds::new(100., 100., 81., 32.)
    );
    assert_eq!(
        app.editor.nodes()[0].content,
        before.pages[0].content.nodes[0].content
    );
    let after = app.editor.document.clone();
    app.start_group_crop(2).unwrap();
    assert_eq!(
        app.image_crop.as_ref().unwrap().full,
        Bounds::new(100., 100., 101., 40.)
    );
    frame(&ctx, &mut app, vec![key(Key::Escape)]);
    assert_eq!(app.editor.document, after);
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    app.editor.redo();
    assert_eq!(app.editor.document, after);
    assert!(!app.editor.markup_visible());
}
#[test]
fn hidden_crop_preview_escape_preserves_redo_and_source_local_commit() {
    let ctx = egui::Context::default();
    let mut app = hidden_fixture(&ctx);
    app.image_crop = None;
    app.editor.create_component("Shared frame".into()).unwrap();
    app.editor.set_markup_visible(false);
    let definitions = app.editor.document.definitions.clone();
    let before = app.editor.document.clone();
    app.start_group_crop(2).unwrap();
    frame(&ctx, &mut app, vec![key(Key::Enter)]);
    assert!(app.error.is_none(), "{:?}", app.error);
    assert_eq!(app.editor.document.definitions, definitions);
    let after = app.editor.document.clone();
    assert_ne!(after, before);
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    let redo = app.editor.redo_name().map(str::to_owned);
    app.start_group_crop(2).unwrap();
    frame(&ctx, &mut app, vec![key(Key::Escape)]);
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.redo_name(), redo.as_deref());
    app.editor.redo();
    assert_eq!(app.editor.document, after);
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
fn split_fixture(ctx: &egui::Context) -> WireframeApp {
    let mut app = WireframeApp::workspace(ctx, None);
    let mut project = Project {
        next_id: 3,
        ..Project::default()
    };
    project.pages[0].content.nodes = vec![Node::control(
        2,
        "image",
        Point::new(100., 100.),
        Size::fixed(120., 80.),
    )];
    app.editor = Editor::new(project);
    app.editor.select(2, false);
    app.pan = Vec2::ZERO;
    app.start_image_split(2).unwrap();
    app
}
#[test]
fn split_pointer_preview_cancel_commit_and_history_at_multiple_zooms() {
    for zoom in [0.5, 1., 2.] {
        let ctx = egui::Context::default();
        let mut app = split_fixture(&ctx);
        app.zoom = zoom;
        frame(&ctx, &mut app, vec![]);
        let before = app.editor.document.clone();
        let visible = app.screen_rect(app.image_crop.as_ref().unwrap().bounds());
        let start = Pos2::new(visible.center().x, visible.top() + visible.height() * 0.3);
        let end = Pos2::new(start.x, visible.center().y);
        drag(&ctx, &mut app, start, end);
        close(
            app.image_crop
                .as_ref()
                .unwrap()
                .split
                .as_ref()
                .unwrap()
                .fraction,
            0.5,
        );
        assert_eq!(app.editor.document, before);
        frame(&ctx, &mut app, vec![key(Key::Escape)]);
        assert!(app.image_crop.is_none());
        assert_eq!(app.editor.document, before);
        assert!(app.editor.undo_name().is_none());
        app.start_image_split(2).unwrap();
        close(
            app.image_crop
                .as_ref()
                .unwrap()
                .split
                .as_ref()
                .unwrap()
                .fraction,
            0.5,
        );
        frame(&ctx, &mut app, vec![key(Key::Enter)]);
        assert!(app.image_crop.is_none());
        assert!(app.error.is_none(), "{:?}", app.error);
        assert_eq!(app.editor.nodes().len(), 2);
        let lower = *app.editor.selection.iter().next().unwrap();
        assert_eq!(
            app.editor.image_geometry(lower).unwrap().1,
            Bounds::new(100., 160., 120., 40.)
        );
        let after = app.editor.document.clone();
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        app.editor.redo();
        assert_eq!(app.editor.document, after);
    }
}
#[test]
fn split_click_clamps_to_ten_units_and_does_not_edit_on_pointer_release() {
    let ctx = egui::Context::default();
    let mut app = split_fixture(&ctx);
    frame(&ctx, &mut app, vec![]);
    let before = app.editor.document.clone();
    let rect = app.screen_rect(app.image_crop.as_ref().unwrap().bounds());
    let point = Pos2::new(rect.center().x, rect.top() + 1.);
    frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerMoved(point),
            pointer(point, true),
            pointer(point, false),
        ],
    );
    close(
        app.image_crop
            .as_ref()
            .unwrap()
            .split
            .as_ref()
            .unwrap()
            .fraction,
        0.125,
    );
    assert_eq!(app.editor.document, before);
}
#[test]
fn split_preview_rejects_changed_image_or_replaced_session_without_applying() {
    for replaced in [false, true] {
        let ctx = egui::Context::default();
        let mut app = split_fixture(&ctx);
        frame(&ctx, &mut app, vec![]);
        if replaced {
            app.editor = Editor::new(app.editor.document.clone());
        } else {
            app.editor.current_mut().content.nodes[0].origin.x += 1.;
        }
        let before = app.editor.document.clone();
        frame(&ctx, &mut app, vec![key(Key::Enter)]);
        assert!(app.image_crop.is_none());
        assert_eq!(app.editor.document, before);
        assert!(
            app.error
                .as_deref()
                .is_some_and(|s| s.contains("image changed"))
        );
    }
}

#[test]
fn crop_panel_quick_resize_preserves_crop_draft_and_drawing() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    let document = app.editor.document.clone();
    for _ in 0..3 {
        frame(&ctx, &mut app, vec![]);
    }
    let id = egui::Id::new("crop-inspector");
    let rect = egui::containers::panel::PanelState::load(&ctx, id)
        .unwrap()
        .outer_rect;
    let start = Pos2::new(rect.min.x, 100.);
    let end = start - Vec2::new(50., 0.);
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(start), pointer(start, true)],
    );
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(end), pointer(end, false)],
    );
    frame(&ctx, &mut app, vec![]);
    let after = egui::containers::panel::PanelState::load(&ctx, id)
        .unwrap()
        .outer_rect;
    assert!(
        (after.width() - rect.width() - 50.).abs() < 1.,
        "before={rect:?}, after={after:?}"
    );
    assert_eq!(app.editor.document, document);
    assert!(app.image_crop.is_some());
}
