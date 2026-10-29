// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::collections::BTreeMap;
use wireframe::presentation::{Aid, Preferences, Settings};

const NOTICE: &str =
    "Couldn't save presentation settings. Your choices still work for this session.";
struct Fixture {
    dir: tempfile::TempDir,
    ctx: egui::Context,
    app: WireframeApp,
    baseline: serde_json::Value,
    redo: Project,
    second: Settings,
}
fn snapshot(app: &WireframeApp) -> serde_json::Value {
    let mut selection: Vec<_> = app.editor.selection.iter().copied().collect();
    selection.sort_unstable();
    serde_json::json!({"project":app.editor.document,"selection":selection,
        "page":app.editor.page,"versions":app.editor.version_selections(),
        "undo":app.editor.undo_name(),"redo":app.editor.redo_name()})
}
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Rectangle, 40., 50.);
    app.file = Some(dir.path().join("study.wireframe"));
    std::fs::write(
        app.file.as_ref().unwrap(),
        app.editor.document.encode().unwrap(),
    )
    .unwrap();
    app.presentation_view.preferences.load(dir.path()).unwrap();
    let second = Settings {
        notes: false,
        side_width: 333.,
        ..Settings::default()
    };
    app.presentation_view
        .preferences
        .remember("second-project", &second)
        .unwrap();
    app.start_presentation();
    app.remember_presentation();
    app.editor
        .rename_page(app.editor.page, "Actual pending Redo")
        .unwrap();
    let redo = app.editor.document.clone();
    app.editor.undo();
    let baseline = snapshot(&app);
    std::fs::rename(
        dir.path().join("presentation-settings.json"),
        dir.path().join("baseline.json"),
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("presentation-settings.json")).unwrap();
    Fixture {
        dir,
        ctx,
        app,
        baseline,
        redo,
        second,
    }
}
struct Render {
    labels: BTreeMap<String, Rect>,
    painted: Vec<(String, Rect, Rect)>,
}
fn collect_shapes(shape: &egui::epaint::Shape, clip: Rect, out: &mut Vec<(String, Rect, Rect)>) {
    match shape {
        egui::epaint::Shape::Text(text) => out.push((
            text.galley.job.text.clone(),
            text.visual_bounding_rect(),
            clip,
        )),
        egui::epaint::Shape::Vec(shapes) => {
            for shape in shapes {
                collect_shapes(shape, clip, out);
            }
        }
        _ => {}
    }
}
fn frame(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    size: Vec2,
    events: Vec<egui::Event>,
) -> Render {
    use eframe::App;
    let mut input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        events,
        ..Default::default()
    };
    app.raw_input_hook(ctx, &mut input);
    let output = ctx.run_ui(input, |ui| {
        app.prepare_page_notes(ui.ctx());
        app.shortcuts(ui.ctx());
        app.sync_viewport();
        app.toolbar(ui); // Actual editor/presentation dispatch, not its presentation-only callee.
        app.canvas(ui);
        app.presentation_dialog(ui.ctx());
        app.dialogs(ui.ctx());
        app.remember_presentation(); // Same order as the production UI.
    });
    let labels = output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter_map(|(_, n)| {
            let name = n.label().or_else(|| n.value())?;
            let b = n.bounds()?;
            Some((
                name.to_owned(),
                Rect::from_min_max(
                    egui::pos2(b.x0 as f32, b.y0 as f32),
                    egui::pos2(b.x1 as f32, b.y1 as f32),
                ),
            ))
        })
        .collect();
    let mut painted = vec![];
    for shape in &output.shapes {
        collect_shapes(&shape.shape, shape.clip_rect, &mut painted);
    }
    output.drop_without_applying_deltas();
    Render { labels, painted }
}
fn settle(f: &mut Fixture, size: Vec2) -> Render {
    // Cover egui's existing disclosure/layout animation before asserting complete paint.
    let frames = (f.ctx.global_style().animation_time * 60.).ceil() as usize + 4;
    for _ in 0..frames {
        frame(&f.ctx, &mut f.app, size, vec![]);
    }
    frame(&f.ctx, &mut f.app, size, vec![])
}
fn click(f: &mut Fixture, size: Vec2, name: &str) {
    let render = settle(f, size);
    let at = render
        .labels
        .get(name)
        .unwrap_or_else(|| panic!("Missing {name}: {:?}", render.labels.keys()))
        .center();
    for pressed in [true, false] {
        frame(
            &f.ctx,
            &mut f.app,
            size,
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
}
fn assert_notice(render: &Render, size: Vec2) {
    let viewport = Rect::from_min_size(Pos2::ZERO, size);
    for name in [NOTICE, "Details"] {
        let bounds = render
            .labels
            .get(name)
            .unwrap_or_else(|| panic!("Missing visible {name}; {:?}", render.labels.keys()));
        assert!(
            bounds.is_positive() && viewport.contains_rect(*bounds),
            "{name} outside {size:?}: {bounds:?}"
        );
        assert!(
            render.painted.iter().any(|(text, rect, clip)| text == name
                && rect.is_positive()
                && viewport.contains_rect(*rect)
                && clip.contains_rect(*rect)),
            "No clipped text paint for {name}: {:?}",
            render.painted
        );
    }
    eprintln!(
        "PREFERENCE-FEEDBACK {size:?} notice={:?} details={:?}",
        render.labels[NOTICE], render.labels["Details"]
    );
}
fn assert_integrity(f: &mut Fixture) {
    assert_eq!(snapshot(&f.app), f.baseline);
    f.app.editor.redo();
    assert_eq!(f.app.editor.document, f.redo, "Actual Redo payload changed");
    f.app.editor.undo();
    assert_eq!(snapshot(&f.app), f.baseline);
    let saved: BTreeMap<String, Settings> =
        serde_json::from_slice(&std::fs::read(f.dir.path().join("baseline.json")).unwrap())
            .unwrap();
    assert_eq!(saved["second-project"], f.second);
}
fn visible_case(mode: &str, size: Vec2) {
    let mut f = fixture();
    if mode == "closed" {
        click(&mut f, size, "User Test");
        click(&mut f, size, "Close Settings");
        for aid in [
            Aid::Navigator,
            Aid::Notes,
            Aid::Comments,
            Aid::LinkHints,
            Aid::BigCursor,
            Aid::Markup,
        ] {
            assert!(!f.app.presentation_view.session.settings().aid(aid));
        }
    } else {
        click(&mut f, size, "Big Arrow Cursor");
        if mode == "advanced" {
            click(&mut f, size, "Advanced…");
        }
    }
    assert!(f.dir.path().join("presentation-settings.json").is_dir());
    let collapsed = settle(&mut f, size);
    assert_notice(&collapsed, size);
    record_render(&collapsed, mode, size, false);
    click(&mut f, size, "Details");
    let details = settle(&mut f, size);
    let raw: Vec<_> = details
        .labels
        .iter()
        .filter(|(s, _)| s.contains("persist"))
        .collect();
    assert!(
        !raw.is_empty(),
        "Original cause unavailable: {:?}",
        details.labels.keys()
    );
    for (cause, bounds) in raw {
        assert!(
            Rect::from_min_size(Pos2::ZERO, size).contains_rect(*bounds),
            "Cause clipped: {bounds:?}"
        );
        assert!(
            details
                .painted
                .iter()
                .any(|(text, rect, clip)| text == cause
                    && rect.is_positive()
                    && Rect::from_min_size(Pos2::ZERO, size).contains_rect(*rect)
                    && clip.contains_rect(*rect)),
            "Original cause paint clipped: {:?}",
            details.painted
        );
        eprintln!("DETAILS {mode} {size:?} {cause} {bounds:?}");
    }
    record_render(&details, mode, size, true);
    assert!(f.app.present);
    assert_integrity(&mut f);
    // The notice and Details must not take over the existing layered Escape contract.
    let steps = if mode == "advanced" {
        3
    } else if mode == "main" {
        2
    } else {
        1
    };
    for index in 0..steps {
        frame(
            &f.ctx,
            &mut f.app,
            size,
            vec![egui::Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
        assert_eq!(f.app.present, index + 1 < steps);
    }
    assert_integrity(&mut f);
}
fn record_render(render: &Render, mode: &str, size: Vec2, expanded: bool) {
    let rect = |r: &Rect| [r.min.x, r.min.y, r.max.x, r.max.y];
    let labels: BTreeMap<_, _> = render.labels.iter().map(|(k, v)| (k, rect(v))).collect();
    let paints: Vec<_> = render.painted.iter().filter(|(text, _, _)| text == NOTICE || text == "Details" || text.contains("persist"))
        .map(|(text, bounds, clip)| serde_json::json!({"text":text,"bounds":rect(bounds),"clip":rect(clip)})).collect();
    eprintln!(
        "RENDER_EVIDENCE {}",
        serde_json::json!({"mode":mode,"viewport":[size.x,size.y],"details_expanded":expanded,"labels":labels,"paints":paints})
    );
}
macro_rules! visibility_case {
    ($name:ident,$mode:literal,$w:literal,$h:literal) => {
        #[test]
        fn $name() {
            visible_case($mode, egui::vec2($w, $h));
        }
    };
}
visibility_case!(preference_feedback_main_minimum, "main", 920., 620.);
visibility_case!(preference_feedback_main_normal, "main", 1360., 900.);
visibility_case!(preference_feedback_advanced_minimum, "advanced", 920., 620.);
visibility_case!(preference_feedback_advanced_normal, "advanced", 1360., 900.);
visibility_case!(preference_feedback_aidless_minimum, "closed", 920., 620.);
visibility_case!(preference_feedback_aidless_normal, "closed", 1360., 900.);
#[test]
fn preference_feedback_healthy_settings_have_no_notice_and_keep_real_redo() {
    let mut f = fixture();
    let size = egui::vec2(920., 620.);
    let render = settle(&mut f, size);
    assert!(!render.labels.contains_key(NOTICE));
    assert!(!render.labels.contains_key("Details"));
    assert_integrity(&mut f);
}
#[test]
fn preference_feedback_unchanged_retry_clears_notice_and_retains_full_map_history() {
    let size = egui::vec2(920., 620.);
    let mut f = fixture();
    click(&mut f, size, "Big Arrow Cursor");
    settle(&mut f, size);
    let expected = f.app.presentation_view.session.settings().clone();
    std::fs::remove_dir(f.dir.path().join("presentation-settings.json")).unwrap();
    let render = settle(&mut f, size); // No second settings change.
    assert!(!render.labels.contains_key(NOTICE));
    let actual: BTreeMap<String, Settings> = serde_json::from_slice(
        &std::fs::read(f.dir.path().join("presentation-settings.json")).unwrap(),
    )
    .unwrap();
    let mut expected_map: BTreeMap<String, Settings> =
        serde_json::from_slice(&std::fs::read(f.dir.path().join("baseline.json")).unwrap())
            .unwrap();
    let key = expected_map
        .keys()
        .find(|k| k.as_str() != "second-project")
        .unwrap()
        .clone();
    expected_map.insert(key, expected.clone());
    assert_eq!(actual, expected_map);
    f.app.present = false;
    let editor = settle(&mut f, size);
    assert!(
        !editor
            .labels
            .keys()
            .any(|label| label.contains("retained for this session")),
        "Stale editor status: {:?}",
        editor.labels.keys()
    );
    assert!(!f.app.status.contains("retained for this session"));
    let mut prefs = Preferences::default();
    prefs.load(f.dir.path()).unwrap();
    assert_eq!(prefs.get("second-project"), f.second);
    let fresh_ctx = egui::Context::default();
    let mut fresh = WireframeApp::workspace(&fresh_ctx, None);
    fresh.editor = Editor::new(f.app.editor.document.clone());
    fresh.file = f.app.file.clone();
    fresh.presentation_view.preferences = prefs;
    fresh.start_presentation();
    assert_eq!(fresh.presentation_view.session.settings(), &expected);
    assert_eq!(fresh.editor.document, f.app.editor.document);
    assert!(fresh.editor.undo_name().is_none() && fresh.editor.redo_name().is_none());
    assert_integrity(&mut f);
}
#[test]
fn preference_feedback_does_not_overwrite_unrelated_status_on_failure_or_retry() {
    let mut f = fixture();
    let size = egui::vec2(1360., 900.);
    f.app.status = "Unrelated operation completed".into();
    click(&mut f, size, "Big Arrow Cursor");
    settle(&mut f, size);
    assert_eq!(f.app.status, "Unrelated operation completed");
    assert_eq!(f.app.status_text(), NOTICE);
    std::fs::remove_dir(f.dir.path().join("presentation-settings.json")).unwrap();
    settle(&mut f, size);
    assert_eq!(f.app.status_text(), "Unrelated operation completed");
    assert_integrity(&mut f);
}
#[test]
fn preference_feedback_supported_pane_resize_reaches_failure_notice_without_drawing_edit() {
    let mut f = fixture();
    let size = egui::vec2(1360., 900.);
    click(&mut f, size, "Close Settings");
    settle(&mut f, size);
    let old = f.app.presentation_view.session.settings().navigator_width;
    let from = egui::pos2(f.app.canvas_rect.left() - 1., f.app.canvas_rect.center().y);
    frame(
        &f.ctx,
        &mut f.app,
        size,
        vec![
            egui::Event::PointerMoved(from),
            egui::Event::PointerButton {
                pos: from,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    for dx in [30., 60., 90.] {
        frame(
            &f.ctx,
            &mut f.app,
            size,
            vec![egui::Event::PointerMoved(from + egui::vec2(dx, 0.))],
        );
    }
    let to = from + egui::vec2(90., 0.);
    frame(
        &f.ctx,
        &mut f.app,
        size,
        vec![egui::Event::PointerButton {
            pos: to,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert!(f.app.presentation_view.session.settings().navigator_width > old + 60.);
    assert_notice(&settle(&mut f, size), size);
    assert_integrity(&mut f);
}

#[test]
fn preference_feedback_editor_return_still_shows_current_failure_without_status_clobber() {
    let mut f = fixture();
    let size = egui::vec2(920., 620.);
    f.app.status = "Unrelated operation completed".into();
    click(&mut f, size, "Big Arrow Cursor");
    click(&mut f, size, "Close Settings");
    frame(
        &f.ctx,
        &mut f.app,
        size,
        vec![egui::Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert!(!f.app.present);
    let render = settle(&mut f, size);
    let bounds = render
        .labels
        .get(NOTICE)
        .expect("Current preference failure must remain visible in editor");
    assert!(Rect::from_min_size(Pos2::ZERO, size).contains_rect(*bounds));
    assert!(
        !render.labels.contains_key("Details"),
        "No added editor controls"
    );
    assert_eq!(f.app.status, "Unrelated operation completed");
    assert!(f.dir.path().join("presentation-settings.json").is_dir());
    assert_integrity(&mut f);
    std::fs::remove_dir(f.dir.path().join("presentation-settings.json")).unwrap();
    let recovered = settle(&mut f, size);
    assert!(!recovered.labels.contains_key(NOTICE));
    assert!(
        recovered
            .labels
            .contains_key("Unrelated operation completed")
    );
    assert_integrity(&mut f);
}
