// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::time::{Duration, Instant};

pub(in crate::app::persistence) fn drain_recoveries(app: &mut WireframeApp) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.persistence.as_ref().unwrap().discovery.is_some() {
        app.poll_recoveries();
        assert!(Instant::now() < deadline, "recovery scan did not finish");
        std::thread::sleep(Duration::from_millis(2));
    }
}
fn setup() -> (tempfile::TempDir, egui::Context, WireframeApp) {
    let dir = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.enable_persistence(&ctx, dir.path().to_owned());
    drain_recoveries(&mut app);
    (dir, ctx, app)
}
fn offer(id: &str) -> CopySummary {
    CopySummary {
        id: id.into(),
        title: "Recovered".into(),
        named_path: None,
        has_local_changes: true,
    }
}

#[test]
fn automatic_recovery_preserves_quick_add_focus_and_the_first_pointer_action() {
    use eframe::App;
    use egui::{Event, Modifiers, PointerButton, Pos2, Rect, Vec2};
    fn frame(app: &mut WireframeApp, ctx: &egui::Context, events: Vec<Event>) -> Pos2 {
        let mut raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200., 850.))),
            events,
            ..Default::default()
        };
        app.raw_input_hook(ctx, &mut raw);
        let mut out = ctx.run_ui(raw, |ui| app.ui(ui, &mut eframe::Frame::_new_kittest()));
        let point = out
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find_map(|(_, n)| {
                (n.placeholder() == Some("Quick Add"))
                    .then(|| n.bounds())
                    .flatten()
            })
            .map(|r| egui::pos2(((r.x0 + r.x1) / 2.) as f32, ((r.y0 + r.y1) / 2.) as f32));
        out.textures_delta.clear();
        point.unwrap_or_default()
    }
    for first_click in [false, true] {
        let (_dir, ctx, mut app) = setup();
        frame(&mut app, &ctx, vec![]);
        let point = frame(&mut app, &ctx, vec![]);
        assert_ne!(point, Pos2::ZERO);
        let click = || {
            vec![
                Event::PointerMoved(point),
                Event::PointerButton {
                    pos: point,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
                Event::PointerButton {
                    pos: point,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::NONE,
                },
            ]
        };
        if !first_click {
            frame(&mut app, &ctx, click());
            frame(&mut app, &ctx, vec![Event::Text("Button".into())]);
            assert_eq!(app.quick, "Button");
            assert!(wireframe::text_entry::is_focused(&ctx));
        }
        let before = app.editor.document.clone();
        let (sender, receiver) = mpsc::channel();
        sender
            .send(Found {
                offers: vec![offer("retained")],
                errors: vec![],
            })
            .unwrap();
        app.persistence.as_mut().unwrap().discovery = Some(Scan {
            receiver,
            show: true,
            again: false,
        });
        frame(&mut app, &ctx, if first_click { click() } else { vec![] });
        assert!(!app.recovery_chooser_open(), "first click: {first_click}");
        assert_eq!(app.persistence.as_ref().unwrap().offers.len(), 1);
        assert!(wireframe::text_entry::is_focused(&ctx));
        frame(&mut app, &ctx, vec![Event::Text(" cursor".into())]);
        assert_eq!(
            app.quick,
            if first_click {
                " cursor"
            } else {
                "Button cursor"
            }
        );
        assert_eq!(app.editor.document, before);
        app.show_recoveries();
        assert!(
            app.recovery_chooser_open(),
            "explicit recovery stays available"
        );
        drain_recoveries(&mut app);
    }
}

#[test]
fn recovery_escape_defers_only_the_top_dialog_and_keeps_late_results() {
    let (_dir, ctx, mut app) = setup();
    let (sender, receiver) = mpsc::channel();
    let p = app.persistence.as_mut().unwrap();
    p.offers = vec![offer("retained")];
    p.chooser = true;
    p.discovery = Some(Scan {
        receiver,
        show: true,
        again: false,
    });
    app.error = Some("Temporary error above recovery".into());
    let before = app.editor.document.clone();
    let draw = |app: &mut WireframeApp, escape| {
        let output = ctx.run_ui(
            egui::RawInput {
                events: if escape {
                    vec![egui::Event::Key {
                        key: egui::Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }]
                } else {
                    vec![]
                },
                ..Default::default()
            },
            |_| app.dialogs(&ctx),
        );
        output.drop_without_applying_deltas();
    };
    for _ in 0..3 {
        draw(&mut app, false);
    }
    draw(&mut app, true);
    assert!(app.error.is_none());
    assert!(app.recovery_chooser_open());
    draw(&mut app, false);
    draw(&mut app, true);
    assert!(!app.recovery_chooser_open());
    sender
        .send(Found {
            offers: vec![offer("retained")],
            errors: vec![],
        })
        .unwrap();
    drain_recoveries(&mut app);
    assert!(!app.recovery_chooser_open());
    assert_eq!(app.persistence.as_ref().unwrap().offers.len(), 1);
    assert_eq!(app.editor.document, before);
}
#[test]
fn later_keeps_the_ui_usable_and_a_late_scan_does_not_reopen_the_picker() {
    let (_dir, ctx, mut app) = setup();
    let (release, wait) = mpsc::channel();
    let (started, ready) = mpsc::channel();
    app.persistence.as_mut().unwrap().discovery = Some(
        Scan::start(ctx.clone(), true, move || {
            started.send(()).unwrap();
            wait.recv().unwrap();
            Found {
                offers: vec![offer("retained")],
                errors: vec![],
            }
        })
        .unwrap(),
    );
    app.persistence.as_mut().unwrap().chooser = true;
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    let mut later = None;
    for _ in 0..2 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(940., 620.),
                )),
                ..Default::default()
            },
            |_| {
                app.poll_recoveries();
                app.persistence_dialogs(&ctx);
            },
        );
        later = output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find_map(|(_, n)| {
                if n.label() != Some("Later") {
                    return None;
                }
                let b = n.bounds()?;
                Some(egui::pos2(
                    ((b.x0 + b.x1) / 2.) as f32,
                    ((b.y0 + b.y1) / 2.) as f32,
                ))
            });
        output.textures_delta.clear();
    }
    let pos = later.expect("Later remains reachable while scan is blocked");
    for event in [
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        },
    ] {
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(940., 620.),
                )),
                events: vec![event],
                ..Default::default()
            },
            |_| app.persistence_dialogs(&ctx),
        );
        output.drop_without_applying_deltas();
    }
    assert!(!app.recovery_chooser_open());
    app.editor.add(wireframe::model::Kind::Button, 10., 20.);
    let project = app.editor.document.clone();
    release.send(()).unwrap();
    drain_recoveries(&mut app);
    assert!(!app.recovery_chooser_open());
    assert!(app.editor.document == project);
    assert_eq!(app.persistence.as_ref().unwrap().offers.len(), 1);
}
#[test]
fn refreshes_coalesce_and_stale_results_cannot_offer_a_replaced_owner() {
    let (_dir, ctx, mut app) = setup();
    let (release, wait) = mpsc::channel();
    app.persistence.as_mut().unwrap().discovery = Some(
        Scan::start(ctx, true, move || {
            wait.recv().unwrap();
            Found {
                offers: vec![offer("stale")],
                errors: vec![],
            }
        })
        .unwrap(),
    );
    for _ in 0..10 {
        app.refresh_recoveries(true);
    }
    assert!(
        app.persistence
            .as_ref()
            .unwrap()
            .discovery
            .as_ref()
            .unwrap()
            .again
    );
    release.send(()).unwrap();
    drain_recoveries(&mut app);
    assert!(app.persistence.as_ref().unwrap().offers.is_empty());
    assert!(!app.recovery_chooser_open());
    let (sender, receiver) = mpsc::channel();
    sender
        .send(Found {
            offers: vec![offer("current")],
            errors: vec![],
        })
        .unwrap();
    let p = app.persistence.as_mut().unwrap();
    p.id = Some("current".into());
    p.discovery = Some(Scan {
        receiver,
        show: true,
        again: false,
    });
    app.poll_recoveries();
    assert!(app.persistence.as_ref().unwrap().offers.is_empty());
}
#[test]
fn missing_named_files_and_corrupt_records_are_reported_without_modification() {
    let (dir, ctx, mut app) = setup();
    let copies = app.persistence.as_ref().unwrap().copies.clone();
    let named = dir.path().join("Missing.wireframe");
    let project = wireframe::document::Project::default();
    ProjectStore.save(&named, &project).unwrap();
    drop(copies.open_named(&named).unwrap());
    std::fs::remove_file(&named).unwrap();
    let copy = copies.create_unnamed(&project).unwrap();
    let damaged = copy.local_path().to_owned();
    drop(copy);
    std::fs::write(&damaged, b"broken").unwrap();
    app.show_recoveries();
    drain_recoveries(&mut app);
    let p = app.persistence.as_ref().unwrap();
    assert_eq!(p.offers.len(), 1);
    assert_eq!(p.discovery_errors.len(), 1);
    assert!(p.chooser);
    assert_eq!(std::fs::read(&damaged).unwrap(), b"broken");
    let output = ctx.run_ui(egui::RawInput::default(), |_| app.persistence_dialogs(&ctx));
    output.drop_without_applying_deltas();
}
#[test]
fn a_document_action_suppresses_automatic_offers_but_keeps_the_results() {
    let (_dir, ctx, mut app) = setup();
    let (sender, receiver) = mpsc::channel();
    app.persistence.as_mut().unwrap().discovery = Some(Scan {
        receiver,
        show: true,
        again: false,
    });
    app.request(super::super::super::Pending::New, &ctx);
    sender
        .send(Found {
            offers: vec![offer("other")],
            errors: vec![],
        })
        .unwrap();
    app.poll_recoveries();
    assert!(!app.recovery_chooser_open());
    assert_eq!(app.persistence.as_ref().unwrap().offers.len(), 1);
}

#[test]
fn editing_suppresses_automatic_offers_and_disconnection_retains_error_feedback() {
    let (_dir, _ctx, mut app) = setup();
    let (sender, receiver) = mpsc::channel();
    app.persistence.as_mut().unwrap().discovery = Some(Scan {
        receiver,
        show: true,
        again: false,
    });
    app.editor.add(wireframe::model::Kind::Label, 10., 20.);
    sender
        .send(Found {
            offers: vec![offer("retained")],
            errors: vec![],
        })
        .unwrap();
    app.poll_recoveries();
    assert!(!app.recovery_chooser_open());
    assert_eq!(app.persistence.as_ref().unwrap().offers.len(), 1);
    let (sender, receiver) = mpsc::channel();
    drop(sender);
    app.persistence.as_mut().unwrap().discovery = Some(Scan {
        receiver,
        show: true,
        again: false,
    });
    app.poll_recoveries();
    assert!(app.persistence.as_ref().unwrap().discovery_errors[0].contains("stopped unexpectedly"));
    assert!(!app.recovery_chooser_open());
    assert!(app.status.contains("Recovery check needs attention"));
}

#[test]
fn many_errors_remain_scrollable_with_later_visible_at_minimum_window_size() {
    let (_dir, ctx, mut app) = setup();
    let p = app.persistence.as_mut().unwrap();
    p.chooser = true;
    p.discovery_errors = (0..100)
        .map(|i| {
            format!(
                "Record {i} could not be read. Its original bytes remain available for recovery."
            )
        })
        .collect();
    let view = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(940., 620.));
    let mut later = None;
    for _ in 0..3 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(view),
                ..Default::default()
            },
            |_| app.persistence_dialogs(&ctx),
        );
        later = output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find_map(|(_, n)| {
                if n.label() != Some("Later") {
                    return None;
                }
                let b = n.bounds()?;
                Some(egui::Rect::from_min_max(
                    egui::pos2(b.x0 as f32, b.y0 as f32),
                    egui::pos2(b.x1 as f32, b.y1 as f32),
                ))
            });
        output.textures_delta.clear();
    }
    assert!(view.contains_rect(later.unwrap()));
}
