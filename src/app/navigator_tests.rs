// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::cell::Cell;

thread_local! { static CLOCK_READS: Cell<usize> = const { Cell::new(0) }; }
fn clock() -> wireframe::tablet::Clock {
    CLOCK_READS.with(|n| n.set(n.get() + 1));
    wireframe::tablet::Clock::new(9, 52).unwrap()
}
fn fixture() -> (egui::Context, WireframeApp) {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    let mut project = Project::default();
    let mut node = wireframe::document::Node::control(
        2,
        "smartphone",
        wireframe::document::Point { x: 20., y: 20. },
        wireframe::document::Size::default(),
    );
    node.properties.insert(
        "text".into(),
        wireframe::document::Value::Text("Contact".into()),
    );
    project.pages[0].content.nodes.push(node);
    let page = project.pages[0].clone();
    project.pages.clear();
    for index in 0..80 {
        let mut next = page.clone();
        next.id = index * 2 + 1;
        next.name = format!("Contact {}", index + 1);
        next.content.nodes[0].id = index * 2 + 2;
        project.pages.push(next);
    }
    project.next_id = 161;
    project.validate().unwrap();
    app.editor = Editor::with_clock(project, clock);
    CLOCK_READS.with(|n| n.set(0));
    (ctx, app)
}
fn frame(ctx: &egui::Context, app: &mut WireframeApp, time: f64, events: Vec<egui::Event>) {
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
            time: Some(time),
            events,
            ..Default::default()
        },
        |ui| app.sidebar(ui),
    )
    .drop_without_applying_deltas();
}
#[test]
fn offscreen_thumbnails_do_not_capture_dynamic_values_until_visible() {
    let (ctx, mut app) = fixture();
    let document = app.editor.document.clone();
    frame(&ctx, &mut app, 0., vec![]);
    frame(&ctx, &mut app, 0.1, vec![]);
    let initial = CLOCK_READS.with(Cell::get);
    assert!((1..10).contains(&initial), "captured {initial} page clocks");
    frame(
        &ctx,
        &mut app,
        0.2,
        vec![
            egui::Event::PointerMoved(Pos2::new(100., 200.)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                phase: egui::TouchPhase::Move,
                delta: Vec2::new(0., -5000.),
                modifiers: Modifiers::NONE,
            },
        ],
    );
    for i in 3..10 {
        frame(&ctx, &mut app, i as f64 * 0.1, vec![]);
    }
    let after = CLOCK_READS.with(Cell::get);
    assert!(after > initial, "new visible pages must render");
    assert!(
        after < 40,
        "scrolling should not materialize all 80 thumbnails"
    );
    assert_eq!(document, app.editor.document);
}
#[test]
fn list_navigation_does_not_render_dynamic_page_contents() {
    let (ctx, mut app) = fixture();
    let document = app.editor.document.clone();
    app.thumbnails = false;
    frame(&ctx, &mut app, 0., vec![]);
    frame(&ctx, &mut app, 0.1, vec![]);
    assert_eq!(CLOCK_READS.with(Cell::get), 0);
    app.thumbnails = true;
    frame(&ctx, &mut app, 0.2, vec![]);
    assert!((1..10).contains(&CLOCK_READS.with(Cell::get)));
    assert_eq!(document, app.editor.document);
}

#[test]
fn view_switches_and_page_selection_preserve_document_and_history() {
    use std::collections::BTreeMap;
    fn nodes(
        ctx: &egui::Context,
        app: &mut WireframeApp,
        events: Vec<egui::Event>,
    ) -> BTreeMap<String, Rect> {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
                events,
                ..Default::default()
            },
            |ui| app.sidebar(ui),
        );
        let result = output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .filter_map(|(_, n)| {
                let b = n.bounds()?;
                Some((
                    n.label()?.to_owned(),
                    Rect::from_min_max(
                        Pos2::new(b.x0 as f32, b.y0 as f32),
                        Pos2::new(b.x1 as f32, b.y1 as f32),
                    ),
                ))
            })
            .collect();
        output.textures_delta.clear();
        result
    }
    fn click(ctx: &egui::Context, app: &mut WireframeApp, label: &str) {
        nodes(ctx, app, vec![]);
        let available = nodes(ctx, app, vec![]);
        let pos = available
            .get(label)
            .unwrap_or_else(|| panic!("Missing {label}: {:?}", available.keys()))
            .center();
        nodes(ctx, app, vec![egui::Event::PointerMoved(pos)]);
        for pressed in [true, false] {
            nodes(
                ctx,
                app,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Modifiers::NONE,
                    },
                ],
            );
        }
    }
    let (ctx, mut app) = fixture();
    ctx.enable_accesskit();
    let original = app.editor.document.clone();
    let history = app.editor.undo_name().map(str::to_owned);
    nodes(&ctx, &mut app, vec![]);
    let visible = nodes(&ctx, &mut app, vec![]);
    for label in ["Versions", "Components", "Layers"] {
        let bounds = visible[label];
        assert!(
            bounds.top() >= 0. && bounds.bottom() <= 700.,
            "{label} is clipped: {bounds:?}"
        );
    }
    click(&ctx, &mut app, "Layers");
    // Let the expansion animation and measured footer height settle.
    for _ in 0..40 {
        nodes(&ctx, &mut app, vec![]);
    }
    let expanded = nodes(&ctx, &mut app, vec![]);
    let layer = expanded["Smartphone"];
    assert!(
        layer.top() >= 0. && layer.bottom() <= 700.,
        "Expanded layer is clipped: {layer:?}"
    );
    click(&ctx, &mut app, "Smartphone");
    assert_eq!(app.editor.selection, [2].into_iter().collect());
    click(&ctx, &mut app, "List");
    assert!(!app.thumbnails);
    click(&ctx, &mut app, "Contact 2");
    assert_eq!(app.editor.page, 3);
    click(&ctx, &mut app, "Thumbnails");
    assert!(app.thumbnails);
    click(&ctx, &mut app, "Contact 1");
    assert_eq!(app.editor.page, 1);
    assert_eq!(app.editor.document, original);
    assert_eq!(app.editor.undo_name(), history.as_deref());
}

#[test]
fn retained_pages_are_absent_from_both_navigator_views_and_thumbnail_work() {
    let (ctx, mut app) = fixture();
    let mut project = app.editor.document.clone();
    for page in &mut project.pages[..79] {
        page.trashed = true;
    }
    app.editor = Editor::with_clock(project.clone(), clock);
    ctx.enable_accesskit();
    for thumbnails in [true, false] {
        app.thumbnails = thumbnails;
        let mut labels = Vec::new();
        for _ in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
                    ..Default::default()
                },
                |ui| app.sidebar(ui),
            );
            labels.extend(
                output
                    .platform_output
                    .accesskit_update
                    .as_ref()
                    .unwrap()
                    .nodes
                    .iter()
                    .filter_map(|(_, n)| n.label().map(str::to_owned)),
            );
            output.textures_delta.clear();
        }
        assert!(labels.iter().any(|s| s == "Contact 80"));
        assert!(!labels.iter().any(|s| s == "Contact 1" || s == "Contact 79"));
    }
    assert_eq!(CLOCK_READS.with(Cell::get), 1);
    assert_eq!(app.editor.document, project);
}
