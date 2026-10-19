// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

const SIDE: Pos2 = Pos2::new(100., 300.);
const CANVAS: Pos2 = Pos2::new(600., 400.);
fn wheel(delta: Vec2) -> egui::Event {
    egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        phase: egui::TouchPhase::Move,
        delta,
        modifiers: Modifiers::NONE,
    }
}
fn frame(ctx: &egui::Context, app: &mut WireframeApp, n: usize, events: Vec<egui::Event>) {
    let mut raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1360., 932.))),
        time: Some(n as f64 / 60.),
        events,
        ..Default::default()
    };
    app.prepare_viewport_input(ctx, &mut raw);
    ctx.run_ui(raw, |ui| {
        app.sync_viewport();
        app.toolbar(ui);
        app.sidebar(ui);
        app.inspector(ui);
        app.canvas(ui);
    })
    .drop_without_applying_deltas();
}
fn fixture(tiles: bool) -> (egui::Context, WireframeApp) {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    for _ in 0..40 {
        app.editor.add_page();
    }
    let first = app.editor.document.pages[0].id;
    app.editor.switch_page(first);
    app.thumbnails = tiles;
    app.show_library = false;
    for n in 0..3 {
        frame(&ctx, &mut app, n, vec![]);
    }
    assert!(app.canvas_rect.contains(CANVAS));
    assert!(!app.canvas_rect.contains(SIDE));
    (ctx, app)
}
#[test]
fn sidebar_wheel_cannot_pan_canvas_after_pointer_transition() {
    for tiles in [true, false] {
        for same_frame in [true, false] {
            let (ctx, mut app) = fixture(tiles);
            let document = app.editor.document.clone();
            let mut events = vec![
                egui::Event::PointerMoved(SIDE),
                wheel(Vec2::new(-30., -120.)),
            ];
            if same_frame {
                events.push(egui::Event::PointerMoved(CANVAS));
            }
            frame(&ctx, &mut app, 3, events);
            for n in 4..20 {
                frame(&ctx, &mut app, n, vec![egui::Event::PointerMoved(CANVAS)]);
            }
            assert_eq!(
                app.pan,
                Vec2::ZERO,
                "tiles={tiles}, same_frame={same_frame}"
            );
            assert_eq!(app.editor.document, document);
        }
    }
}
#[test]
fn mixed_region_wheels_keep_their_event_time_targets() {
    let (ctx, mut app) = fixture(true);
    frame(
        &ctx,
        &mut app,
        3,
        vec![
            egui::Event::PointerMoved(CANVAS),
            wheel(Vec2::new(0., -2.)),
            egui::Event::PointerMoved(SIDE),
            wheel(Vec2::new(0., -3.)),
            egui::Event::PointerMoved(CANVAS),
        ],
    );
    assert_eq!(app.pan, Vec2::new(0., -2.));
}
#[test]
fn canvas_wheel_is_not_lost_when_pointer_leaves_in_same_frame() {
    let (ctx, mut app) = fixture(true);
    frame(
        &ctx,
        &mut app,
        3,
        vec![
            egui::Event::PointerMoved(CANVAS),
            wheel(Vec2::new(0., -2.)),
            egui::Event::PointerMoved(SIDE),
        ],
    );
    assert_eq!(app.pan, Vec2::new(0., -2.));
}

#[test]
fn canvas_scroll_retains_the_existing_curve_units_and_modifiers() {
    for (unit, modifiers) in [
        (egui::MouseWheelUnit::Point, Modifiers::NONE),
        (egui::MouseWheelUnit::Line, Modifiers::SHIFT),
        (egui::MouseWheelUnit::Page, Modifiers::NONE),
    ] {
        let (ctx, mut app) = fixture(true);
        let baseline = egui::Context::default();
        baseline.options_mut(|o| o.input_options = ctx.options(|o| o.input_options));
        let mut expected = Vec2::ZERO;
        for n in 0..24 {
            let events = if n == 3 {
                vec![
                    egui::Event::PointerMoved(CANVAS),
                    egui::Event::MouseWheel {
                        unit,
                        phase: egui::TouchPhase::Move,
                        delta: Vec2::new(-2., -12.),
                        modifiers,
                    },
                ]
            } else if n == 4 {
                // Residuals continue to their original owner, not the sidebar.
                vec![egui::Event::PointerMoved(SIDE)]
            } else {
                vec![]
            };
            baseline
                .run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1360., 932.))),
                        time: Some(n as f64 / 60.),
                        events: events.clone(),
                        ..Default::default()
                    },
                    |ui| {
                        let delta = ui.input(|i| i.smooth_scroll_delta);
                        expected += delta;
                        if delta != Vec2::ZERO {
                            ui.ctx().request_repaint();
                        }
                    },
                )
                .drop_without_applying_deltas();
            if n >= 3 {
                frame(&ctx, &mut app, n, events);
                assert!(
                    (app.pan - expected).length() < 0.01,
                    "{unit:?} at {n}: {:?} != {expected:?}",
                    app.pan
                );
                assert_eq!(
                    ctx.input(|i| i.smooth_scroll_delta),
                    Vec2::ZERO,
                    "canvas wheel leaked into shared input"
                );
            }
        }
    }
}

#[test]
fn sidebar_still_consumes_ordinary_scroll_without_changing_canvas() {
    for tiles in [true, false] {
        let (ctx, mut app) = fixture(tiles);
        frame(
            &ctx,
            &mut app,
            3,
            vec![egui::Event::PointerMoved(SIDE), wheel(Vec2::new(0., -2.))],
        );
        assert_eq!(app.pan, Vec2::ZERO);
        assert_eq!(
            ctx.input(|i| i.smooth_scroll_delta),
            Vec2::ZERO,
            "sidebar did not consume scroll"
        );
    }
}

#[test]
fn surface_change_and_modal_do_not_reuse_canvas_scroll_residuals() {
    for change_surface in [true, false] {
        let (ctx, mut app) = fixture(true);
        frame(
            &ctx,
            &mut app,
            3,
            vec![
                egui::Event::PointerMoved(CANVAS),
                wheel(Vec2::new(0., -120.)),
            ],
        );
        assert!(app.pan.y < 0.);
        if change_surface {
            let page = app.editor.document.pages[1].id;
            app.editor.switch_page(page);
            app.sync_viewport();
        } else {
            app.error = Some("Test modal".into());
        }
        let pan = app.pan;
        frame(&ctx, &mut app, 4, vec![]);
        app.error = None;
        for n in 5..12 {
            frame(&ctx, &mut app, n, vec![]);
        }
        assert_eq!(app.pan, pan);
    }
}

#[test]
fn focus_loss_cancels_residuals_and_pointer_exit_does_not_retarget_them() {
    let (ctx, mut app) = fixture(true);
    frame(
        &ctx,
        &mut app,
        3,
        vec![
            egui::Event::PointerMoved(CANVAS),
            wheel(Vec2::new(0., -120.)),
        ],
    );
    let initial = app.pan;
    frame(&ctx, &mut app, 4, vec![egui::Event::PointerGone]);
    assert!(
        app.pan.y < initial.y,
        "owned smoothing should finish without a pointer"
    );
    frame(&ctx, &mut app, 5, vec![egui::Event::WindowFocused(false)]);
    let stopped = app.pan;
    frame(
        &ctx,
        &mut app,
        6,
        vec![
            egui::Event::WindowFocused(true),
            egui::Event::PointerMoved(CANVAS),
        ],
    );
    frame(&ctx, &mut app, 7, vec![]);
    assert_eq!(app.pan, stopped);
}

#[test]
fn phased_touch_keeps_its_owner_until_end_or_cancel() {
    for terminal in [egui::TouchPhase::End, egui::TouchPhase::Cancel] {
        for starts_on_canvas in [true, false] {
            let (ctx, mut app) = fixture(true);
            let event = |phase, y| egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                phase,
                delta: Vec2::new(0., y),
                modifiers: Modifiers::NONE,
            };
            let start = if starts_on_canvas { CANVAS } else { SIDE };
            let end = if starts_on_canvas { SIDE } else { CANVAS };
            frame(
                &ctx,
                &mut app,
                3,
                vec![
                    egui::Event::PointerMoved(start),
                    event(egui::TouchPhase::Start, 0.),
                ],
            );
            frame(&ctx, &mut app, 4, vec![event(egui::TouchPhase::Move, -10.)]);
            frame(
                &ctx,
                &mut app,
                5,
                vec![
                    egui::Event::PointerMoved(end),
                    event(egui::TouchPhase::Move, -3.),
                ],
            );
            frame(&ctx, &mut app, 6, vec![event(terminal, 0.)]);
            assert_eq!(app.pan.y, if starts_on_canvas { -13. } else { 0. });
            let pan = app.pan;
            frame(
                &ctx,
                &mut app,
                7,
                vec![egui::Event::PointerMoved(CANVAS), wheel(Vec2::new(0., -2.))],
            );
            assert_eq!(
                app.pan,
                pan + Vec2::new(0., -2.),
                "new stream must get a new owner"
            );
        }
    }
}

#[test]
fn overlay_at_event_position_does_not_scroll_underlying_canvas() {
    let (ctx, mut app) = fixture(true);
    for n in 3..5 {
        ctx.run_ui(
            egui::RawInput {
                time: Some(n as f64 / 60.),
                ..Default::default()
            },
            |ui| {
                app.toolbar(ui);
                app.sidebar(ui);
                app.inspector(ui);
                app.canvas(ui);
                egui::Area::new(egui::Id::new("scroll-test-overlay"))
                    .order(egui::Order::Foreground)
                    .fixed_pos(CANVAS - Vec2::splat(30.))
                    .show(ui.ctx(), |ui| {
                        ui.allocate_exact_size(Vec2::splat(100.), Sense::click());
                    });
            },
        )
        .drop_without_applying_deltas();
    }
    assert_ne!(ctx.layer_id_at(CANVAS), app.viewports.canvas_layer);
    frame(
        &ctx,
        &mut app,
        5,
        vec![
            egui::Event::PointerMoved(CANVAS),
            wheel(Vec2::new(0., -120.)),
        ],
    );
    frame(&ctx, &mut app, 6, vec![]);
    assert_eq!(app.pan, Vec2::ZERO);
}

#[test]
fn command_modifier_on_touch_end_does_not_leave_canvas_ownership_latched() {
    for terminal in [egui::TouchPhase::End, egui::TouchPhase::Cancel] {
        let (ctx, mut app) = fixture(true);
        let phase = |phase, modifiers| egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            phase,
            delta: Vec2::ZERO,
            modifiers,
        };
        frame(
            &ctx,
            &mut app,
            3,
            vec![
                egui::Event::PointerMoved(CANVAS),
                phase(egui::TouchPhase::Start, Modifiers::NONE),
            ],
        );
        frame(&ctx, &mut app, 4, vec![phase(terminal, Modifiers::COMMAND)]);
        frame(
            &ctx,
            &mut app,
            5,
            vec![egui::Event::PointerMoved(SIDE), wheel(Vec2::new(0., -2.))],
        );
        assert_eq!(app.pan, Vec2::ZERO);
    }
}
