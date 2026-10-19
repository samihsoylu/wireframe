// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

fn app() -> (egui::Context, WireframeApp) {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.canvas_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1062., 890.));
    app.sync_viewport();
    (ctx, app)
}

#[test]
fn page_alternate_and_presentation_views_are_independent() {
    let (_, mut app) = app();
    let page = app.editor.page;
    app.set_viewport(View {
        zoom: 1.5,
        scroll: [266., 223.],
    });
    let alternate = app.editor.create_version("Option").unwrap();
    app.sync_viewport();
    assert_eq!(app.viewport(), View::default());
    app.zoom_step(false);
    app.zoom_step(false);
    assert_eq!(app.zoom, 0.66);
    app.editor.switch_version(page, None).unwrap();
    app.sync_viewport();
    assert_eq!(
        app.viewport(),
        View {
            zoom: 1.5,
            scroll: [266., 223.]
        }
    );
    app.start_presentation();
    app.zoom_step(true);
    assert_eq!(app.zoom, 2.);
    app.editor.switch_version(page, Some(alternate)).unwrap();
    app.sync_viewport();
    assert_eq!(app.zoom, 0.66);
    app.actual_size();
    app.present = false;
    app.sync_viewport();
    assert_eq!(app.zoom, 0.66);
    app.editor.switch_version(page, None).unwrap();
    app.sync_viewport();
    assert_eq!(app.zoom, 1.5);
}

#[test]
fn rename_promotion_discard_and_undo_keep_surface_identity() {
    let (_, mut app) = app();
    let page = app.editor.page;
    app.zoom_step(true);
    let alternate = app.editor.create_version("First").unwrap();
    app.sync_viewport();
    app.zoom_step(false);
    app.editor.rename_version(alternate, "Second").unwrap();
    app.editor.promote_version(alternate).unwrap();
    app.sync_viewport();
    assert_eq!(app.zoom, 0.75);
    let former = app.editor.document.pages[0]
        .alternates
        .iter()
        .find(|a| a.id != alternate)
        .unwrap()
        .id;
    app.editor.switch_version(page, None).unwrap();
    app.sync_viewport();
    assert_eq!(app.zoom, 1.5); // The official surface retains its own view after content replacement.
    app.editor.switch_version(page, Some(former)).unwrap();
    app.sync_viewport();
    assert_eq!(app.viewport(), View::default()); // A newly allocated surface starts at the origin.
    app.editor.switch_version(page, Some(alternate)).unwrap();
    app.sync_viewport();
    app.editor.discard_version(alternate).unwrap();
    app.sync_viewport();
    assert_eq!(app.zoom, 1.5);
    app.editor.undo();
    app.sync_viewport();
    assert_eq!(app.zoom, 0.75);
}

#[test]
fn all_reset_persists_every_branch_without_changing_document_or_history() {
    let dir = tempfile::tempdir().unwrap();
    let (_, mut app) = app();
    app.file = Some(dir.path().join("test.wireframe"));
    app.load_viewport_preferences(dir.path());
    let page = app.editor.page;
    app.zoom_step(true);
    let alternate = app.editor.create_version("Option").unwrap();
    app.sync_viewport();
    app.zoom_step(false);
    let document = app.editor.document.clone();
    let undo = app.editor.undo_name().map(str::to_owned);
    app.remember_viewports(true);
    app.zoom_all(false);
    let mut saved = preferences::Preferences::default();
    saved.load(dir.path()).unwrap();
    let views = saved.get(&app.version_preferences_key().unwrap());
    assert_eq!(views[&page], View::default());
    assert_eq!(views[&alternate], View::default());
    assert_eq!(app.editor.document, document);
    assert_eq!(app.editor.undo_name(), undo.as_deref());
}

#[test]
fn fit_empty_content_and_locked_selection_do_not_invent_anchors() {
    let (_, mut app) = app();
    app.fit();
    assert_eq!(app.viewport(), View::default());
    app.editor.add(Kind::Button, 65., 375.);
    app.editor.selected_mut(|c| {
        c.bounds.w = 210.;
        c.bounds.h = 44.;
    });
    app.zoom_step(true);
    assert_eq!(app.pan, Vec2::new(0., -151.));
    app.set_viewport(View::default());
    app.editor.lock(true);
    app.zoom_step(true);
    assert_eq!(app.pan, Vec2::new(-266., -223.));
}

fn input(ctx: &egui::Context, app: &mut WireframeApp, time: f64, events: Vec<egui::Event>) {
    let mut raw = egui::RawInput {
        time: Some(time),
        events,
        ..Default::default()
    };
    app.prepare_viewport_input(ctx, &mut raw);
    let mut output = ctx.run_ui(raw, |ui| {
        app.viewport_input(ui.ctx());
        // An extra layout pass must not consume the same input twice.
        app.viewport_input(ui.ctx());
    });
    output.textures_delta.clear();
}

#[test]
fn raw_command_wheel_is_applied_once_without_synthesized_residual_zoom() {
    let (ctx, mut app) = app();
    app.editor.add(Kind::Button, 2000., 2000.);
    app.set_viewport(View {
        zoom: 1.5,
        scroll: [100., 100.],
    });
    let point = app.viewport().point_at([400., 300.]);
    input(
        &ctx,
        &mut app,
        0.,
        vec![
            egui::Event::ModifiersChanged(Modifiers::COMMAND),
            egui::Event::PointerMoved(Pos2::new(400., 300.)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                phase: egui::TouchPhase::Move,
                delta: Vec2::new(0., 12.),
                modifiers: Modifiers::COMMAND,
            },
        ],
    );
    assert!((app.zoom - 1.59).abs() < 0.000001);
    let after = app.viewport();
    for frame in 1..8 {
        input(&ctx, &mut app, f64::from(frame) / 60., vec![]);
        assert_eq!(app.viewport(), after);
    }
    for (actual, expected) in after.point_at([400., 300.]).into_iter().zip(point) {
        assert!((actual - expected).abs() <= 0.5 / after.zoom);
    }
}

#[test]
fn raw_pinch_recovers_signed_magnification_and_plain_wheel_scrolls() {
    let (ctx, mut app) = app();
    app.editor.add(Kind::Button, 2000., 2000.);
    app.zoom = 2.;
    input(
        &ctx,
        &mut app,
        0.,
        vec![
            egui::Event::PointerMoved(Pos2::new(300., 250.)),
            egui::Event::Zoom(0.1_f32.exp()),
        ],
    );
    assert!((app.zoom - 2.2).abs() < 0.000001);
    let pan = app.pan;
    input(
        &ctx,
        &mut app,
        1.,
        vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            phase: egui::TouchPhase::Move,
            delta: Vec2::new(0., -12.),
            modifiers: Modifiers::NONE,
        }],
    );
    assert!((app.zoom - 2.2).abs() < 0.000001);
    assert!(app.pan.y < pan.y);
}

#[test]
fn mixed_command_and_plain_wheel_do_not_share_scroll_residuals() {
    let (mixed_ctx, mut mixed) = app();
    let (plain_ctx, mut plain) = app();
    for app in [&mut mixed, &mut plain] {
        app.editor.add(Kind::Button, 2000., 2000.);
        app.set_viewport(View {
            zoom: 1.5,
            scroll: [100., 100.],
        });
    }
    let pointer = Pos2::new(400., 300.);
    plain.set_viewport(plain.viewport().anchored(1.59, pointer.to_vec2().into()));
    let wheel = |y, modifiers| egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        phase: egui::TouchPhase::Move,
        delta: Vec2::new(0., y),
        modifiers,
    };
    input(
        &mixed_ctx,
        &mut mixed,
        0.,
        vec![
            egui::Event::PointerMoved(pointer),
            wheel(12., Modifiers::COMMAND),
            wheel(-7., Modifiers::NONE),
        ],
    );
    input(
        &plain_ctx,
        &mut plain,
        0.,
        vec![
            egui::Event::PointerMoved(pointer),
            wheel(-7., Modifiers::NONE),
        ],
    );
    for frame in 1..8 {
        assert!(
            (mixed.pan - plain.pan).length() < 0.001,
            "mixed: {:?}, plain: {:?}",
            mixed.pan,
            plain.pan
        );
        input(&mixed_ctx, &mut mixed, f64::from(frame) / 60., vec![]);
        input(&plain_ctx, &mut plain, f64::from(frame) / 60., vec![]);
    }
}

#[test]
fn new_project_does_not_reuse_old_surface_ids_or_saved_views() {
    let (ctx, mut app) = app();
    app.zoom_step(true);
    app.sync_viewport();
    app.perform(Pending::New, &ctx);
    assert_eq!(app.viewport(), View::default());
    app.editor.add_page();
    app.sync_viewport();
    assert_eq!(app.viewport(), View::default());
}

#[test]
fn save_as_copies_views_while_reopening_restores_the_selected_projects_map() {
    let dir = tempfile::tempdir().unwrap();
    let (ctx, mut app) = app();
    app.load_viewport_preferences(dir.path());
    let original = dir.path().join("original.wireframe");
    let copy = dir.path().join("copy.wireframe");
    ProjectStore.save(&original, &app.editor.document).unwrap();
    ProjectStore.save(&copy, &app.editor.document).unwrap();
    app.file = Some(original.clone());
    app.zoom_step(true);
    app.remember_viewports(true);
    // Successful Save As changes the file identity without replacing the editor.
    app.file = Some(copy.clone());
    app.remember_viewports(true);
    app.zoom_step(true);
    app.remember_viewports(true);
    app.perform(Pending::Open(original), &ctx);
    finish_open(&mut app);
    assert_eq!(app.zoom, 1.5);
    app.perform(Pending::Open(copy), &ctx);
    finish_open(&mut app);
    assert_eq!(app.zoom, 2.);
}

#[test]
fn preferences_debounce_changes_flush_batches_and_preserve_unreadable_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("viewport-settings.json");
    let mut prefs = preferences::Preferences::default();
    prefs.load(dir.path()).unwrap();
    let start = Instant::now();
    let views = BTreeMap::from([
        (1, View::default()),
        (
            2,
            View {
                zoom: 0.66,
                scroll: [20., 10.],
            },
        ),
    ]);
    prefs.remember("project".into(), &views, true, start);
    prefs
        .flush(start + Duration::from_millis(499), false)
        .unwrap();
    assert!(!path.exists());
    prefs
        .flush(start + Duration::from_millis(500), false)
        .unwrap();
    let mut restored = preferences::Preferences::default();
    restored.load(dir.path()).unwrap();
    assert_eq!(restored.get("project"), views);
    for corrupt in [
        b"unreadable".as_slice(),
        br#"{"project":{"1":{"zoom":0,"scroll":[0,0]}}}"#,
    ] {
        std::fs::write(&path, corrupt).unwrap();
        let mut invalid = preferences::Preferences::default();
        assert!(invalid.load(dir.path()).is_err());
        invalid.remember("project".into(), &views, true, start);
        assert!(invalid.flush(start, true).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), corrupt);
    }
}

#[test]
fn ordinary_groups_share_views_and_all_pages_exclude_component_views() {
    let (_, mut app) = app();
    app.editor.add(Kind::Button, 100., 100.);
    app.editor.add(Kind::Button, 300., 100.);
    app.editor.select_all();
    app.editor.group();
    let group = *app.editor.selection.iter().next().unwrap();
    app.zoom_step(true);
    let group_view = app.viewport();
    assert!(app.editor.enter(group));
    app.sync_viewport();
    assert_eq!(app.viewport(), group_view);
    app.editor.leave();
    let component = app.editor.create_component("Buttons".into()).unwrap();
    app.editor.edit_definition(component).unwrap();
    app.sync_viewport();
    assert_eq!(app.viewport(), View::default());
    app.zoom_step(false);
    let component_view = app.viewport();
    app.editor.switch_page(app.editor.page);
    app.sync_viewport();
    assert_eq!(app.viewport(), group_view);
    app.zoom_all(false);
    app.editor.edit_definition(component).unwrap();
    app.sync_viewport();
    assert_eq!(app.viewport(), component_view);
}

#[test]
fn all_fit_calculates_each_surface_and_retains_top_level_selection() {
    let (_, mut app) = app();
    let page = app.editor.page;
    app.editor.add(Kind::Browser, 20., 20.);
    app.editor.selected_mut(|c| {
        c.bounds.w = 960.;
        c.bounds.h = 700.;
    });
    let alternate = app.editor.create_version("Small").unwrap();
    app.editor.select_all();
    app.editor.selected_mut(|c| {
        c.bounds.w = 300.;
        c.bounds.h = 200.;
    });
    app.sync_viewport();
    let selected = app.editor.selection.clone();
    app.zoom_all(true);
    assert_eq!(app.editor.selection, selected);
    assert_eq!(app.viewports.editor[&alternate].zoom, 3.);
    assert!((app.viewports.editor[&page].zoom - 1.0836735).abs() < 0.000001);
}

fn drain(app: &mut WireframeApp, ctx: &egui::Context) {
    let deadline = Instant::now() + Duration::from_secs(12);
    while app.save_busy() {
        app.poll_saves(ctx);
        assert!(Instant::now() < deadline, "save timed out: {:?}", app.error);
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn working_copy_reopen_recovery_and_save_as_keep_surface_views() {
    let dir = tempfile::tempdir().unwrap();
    let (ctx, mut app) = app();
    app.enable_persistence(&ctx, dir.path().to_owned());
    app.editor.set_markup_visible(false);
    let page = app.editor.page;
    app.zoom_step(true);
    let alternate = app.editor.create_version("Alternative").unwrap();
    app.sync_viewport();
    app.zoom_step(false);
    let path = dir.path().join("Original.wireframe");
    assert!(app.queue_save(path.clone(), true));
    drain(&mut app, &ctx);
    app.remember_viewports(true);
    let key = app.version_preferences_key().unwrap();
    assert!(app.finish_session(false));
    let mut reopened = WireframeApp::workspace(&ctx, None);
    reopened.enable_persistence(&ctx, dir.path().to_owned());
    assert!(reopened.restore_copy(key));
    assert!(!reopened.editor.markup_visible());
    assert_eq!(reopened.editor.surface_id(), alternate);
    assert_eq!(reopened.zoom, 0.75);
    reopened.editor.switch_version(page, None).unwrap();
    reopened.sync_viewport();
    assert_eq!(reopened.zoom, 1.5);
    let copy = dir.path().join("Copy.wireframe");
    assert!(reopened.queue_save(copy.clone(), true));
    drain(&mut reopened, &ctx);
    assert!(reopened.finish_session(false));
    let mut copied = WireframeApp::workspace(&ctx, None);
    copied.enable_persistence(&ctx, dir.path().to_owned());
    assert!(copied.load_persistent(copy));
    assert!(!copied.editor.markup_visible());
    assert_eq!(copied.zoom, 1.5);
    copied.editor.switch_version(page, Some(alternate)).unwrap();
    copied.sync_viewport();
    assert_eq!(copied.zoom, 0.75);
    assert!(copied.finish_session(false));
}

#[test]
fn shifted_number_shortcut_uses_physical_digit_when_logical_key_is_punctuation() {
    let (ctx, mut app) = app();
    let page = app.editor.page;
    app.zoom_step(true);
    let alternate = app.editor.create_version("Option").unwrap();
    app.sync_viewport();
    app.zoom_step(false);
    let mut output = ctx.run_ui(
        egui::RawInput {
            events: vec![egui::Event::Key {
                key: Key::Exclamationmark,
                physical_key: Some(Key::Num1),
                pressed: true,
                repeat: false,
                modifiers: Modifiers::COMMAND | Modifiers::SHIFT,
            }],
            ..Default::default()
        },
        |ui| app.shortcuts(ui.ctx()),
    );
    output.textures_delta.clear();
    assert_eq!(app.viewport(), View::default());
    assert_eq!(app.viewports.editor[&page], View::default());
    assert_eq!(app.viewports.editor[&alternate], View::default());
}

fn fit_fixture(app: &mut WireframeApp) {
    app.editor.add(Kind::Browser, 20., 20.);
    app.editor.selected_mut(|control| {
        control.bounds.w = 960.;
        control.bounds.h = 700.;
    });
}

#[test]
fn smart_zoom_toggles_fit_and_actual_once_without_document_history_changes() {
    let (ctx, mut app) = app();
    fit_fixture(&mut app);
    let document = app.editor.document.clone();
    let undo = app.editor.undo_name().map(str::to_owned);
    input(
        &ctx,
        &mut app,
        0.,
        vec![
            egui::Event::PointerMoved(Pos2::new(400., 300.)),
            egui::Event::SmartZoom,
        ],
    );
    assert!((app.zoom - 1.0836735).abs() < 0.000001);
    assert_eq!(app.viewport().scroll, [11., 11.]);
    input(&ctx, &mut app, 0.1, vec![egui::Event::SmartZoom]);
    assert_eq!(app.viewport(), View::default());
    // Two gestures in one frame keep their ordering rather than collapsing.
    input(
        &ctx,
        &mut app,
        0.2,
        vec![egui::Event::SmartZoom, egui::Event::SmartZoom],
    );
    assert_eq!(app.viewport(), View::default());
    assert_eq!(app.editor.document, document);
    assert_eq!(app.editor.undo_name(), undo.as_deref());
}

#[test]
fn smart_zoom_uses_exact_actual_size_and_preserves_order_with_pinch() {
    let (ctx, mut app) = app();
    fit_fixture(&mut app);
    app.zoom = 1.0001;
    input(
        &ctx,
        &mut app,
        0.,
        vec![
            egui::Event::PointerMoved(Pos2::new(400., 300.)),
            egui::Event::SmartZoom,
        ],
    );
    assert_eq!(app.zoom, 1.);
    input(
        &ctx,
        &mut app,
        0.1,
        vec![egui::Event::SmartZoom, egui::Event::Zoom(0.2_f32.exp())],
    );
    assert!((app.zoom - 1.0836735 * 1.2).abs() < 0.000001);
    // With the opposite order the last action restores Actual Size instead.
    app.set_viewport(View::default());
    app.viewports.gesture.reset();
    input(
        &ctx,
        &mut app,
        0.2,
        vec![egui::Event::Zoom(0.2_f32.exp()), egui::Event::SmartZoom],
    );
    assert_eq!(app.zoom, 1.);
}

#[test]
fn smart_zoom_stays_inside_canvas_and_presentation_does_not_replace_editor_view() {
    let (ctx, mut app) = app();
    fit_fixture(&mut app);
    input(
        &ctx,
        &mut app,
        0.,
        vec![
            egui::Event::PointerMoved(Pos2::new(-1., 300.)),
            egui::Event::SmartZoom,
        ],
    );
    assert_eq!(app.viewport(), View::default());
    app.zoom_step(true);
    let editor_view = app.viewport();
    app.start_presentation();
    input(
        &ctx,
        &mut app,
        0.1,
        vec![
            egui::Event::PointerMoved(Pos2::new(400., 300.)),
            egui::Event::SmartZoom,
        ],
    );
    assert_eq!(app.zoom, 1.);
    app.present = false;
    app.sync_viewport();
    assert_eq!(app.viewport(), editor_view);
}

#[test]
fn library_source_and_alternate_views_keep_independent_owners_on_return() {
    let (_, mut app) = app();
    app.editor.add(Kind::Button, 100., 100.);
    let source = app.editor.create_component("Action".into()).unwrap();
    let library = app.editor.document.default_library.unwrap();
    let page = app.editor.page;
    let alternate = app.editor.create_version("Compact").unwrap();
    app.sync_viewport();
    let alternate_view = View {
        zoom: 1.5,
        scroll: [120., 90.],
    };
    app.set_viewport(alternate_view);
    app.editor.edit_library(library).unwrap();
    app.sync_viewport();
    let library_view = View {
        zoom: 0.5,
        scroll: [20., 70.],
    };
    app.set_viewport(library_view);
    app.editor.edit_library_source(source).unwrap();
    app.sync_viewport();
    let source_view = View {
        zoom: 2.,
        scroll: [330., 110.],
    };
    app.set_viewport(source_view);
    let document = app.editor.document.clone();
    app.editor.switch_page(page);
    app.sync_viewport();
    assert_eq!(app.editor.surface_id(), alternate);
    assert_eq!(app.viewport(), alternate_view);
    assert!(app.editor.selection.is_empty());
    app.editor.edit_library(library).unwrap();
    app.sync_viewport();
    assert_eq!(app.viewport(), library_view);
    app.editor.edit_library_source(source).unwrap();
    app.sync_viewport();
    assert_eq!(app.viewport(), source_view);
    assert_eq!(app.editor.document, document);
}

fn finish_open(app: &mut WireframeApp) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.startup_open.is_some() {
        app.poll_startup_open();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(app.error.is_none(), "{:?}", app.error);
}

#[test]
fn markup_preference_migrates_legacy_views_and_round_trips_without_touching_corrupt_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("viewport-settings.json");
    let legacy = br#"{"old":{"1":{"zoom":1.5,"scroll":[20,30]}}}"#;
    std::fs::write(&path, legacy).unwrap();
    let mut prefs = preferences::Preferences::default();
    prefs.load(dir.path()).unwrap();
    assert!(prefs.markup_visible("old"));
    assert!(prefs.markup_visible("missing"));
    let views = prefs.get("old");
    assert_eq!(views[&1].zoom, 1.5);
    let now = Instant::now();
    prefs.flush(now, true).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), legacy);
    prefs.remember("old".into(), &views, false, now);
    prefs
        .flush(now + Duration::from_millis(499), false)
        .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), legacy);
    prefs
        .flush(now + Duration::from_millis(500), false)
        .unwrap();
    let mut reopened = preferences::Preferences::default();
    reopened.load(dir.path()).unwrap();
    assert!(!reopened.markup_visible("old"));
    assert_eq!(reopened.get("old"), views);
    let corrupt = br#"{"old":{"views":{},"markup_visible":"false"}}"#;
    std::fs::write(&path, corrupt).unwrap();
    assert!(reopened.load(dir.path()).is_err());
    reopened.remember("old".into(), &views, true, now);
    assert!(reopened.flush(now, true).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), corrupt);
}

#[test]
fn markup_visibility_fit_and_project_switching_keep_independent_preferences() {
    let dir = tempfile::tempdir().unwrap();
    let (ctx, mut app) = app();
    app.load_viewport_preferences(dir.path());
    app.editor.add(Kind::Button, 20., 30.);
    let plain_fit = app.fitted_view(app.editor.page).unwrap();
    app.editor.add(Kind::Callout, 4000., 4000.);
    let full_fit = app.fitted_view(app.editor.page).unwrap();
    assert_ne!(plain_fit, full_fit);
    let original = dir.path().join("original.wireframe");
    let copy = dir.path().join("copy.wireframe");
    ProjectStore.save(&original, &app.editor.document).unwrap();
    ProjectStore.save(&copy, &app.editor.document).unwrap();
    app.file = Some(original.clone());
    app.editor.set_markup_visible(false);
    app.remember_viewports(true);
    assert_eq!(app.fitted_view(app.editor.page).unwrap(), plain_fit);
    app.file = Some(copy.clone());
    app.remember_viewports(true);
    app.editor.set_markup_visible(true);
    app.remember_viewports(true);
    app.perform(Pending::Open(original), &ctx);
    finish_open(&mut app);
    assert!(!app.editor.markup_visible());
    assert_eq!(app.fitted_view(app.editor.page).unwrap(), plain_fit);
    app.perform(Pending::Open(copy), &ctx);
    finish_open(&mut app);
    assert!(app.editor.markup_visible());
    assert_eq!(app.fitted_view(app.editor.page).unwrap(), full_fit);
}
