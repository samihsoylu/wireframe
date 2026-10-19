// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::collections::BTreeMap;

fn frame(
    app: &mut WireframeApp,
    ctx: &egui::Context,
    width: f32,
    events: Vec<egui::Event>,
) -> BTreeMap<String, Rect> {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 700.))),
            events,
            ..Default::default()
        },
        |ui| app.toolbar(ui),
    );
    let result = output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter_map(|(_, n)| {
            let label = n.label().or_else(|| n.placeholder())?;
            let b = n.bounds()?;
            Some((
                label.to_owned(),
                Rect::from_min_max(
                    egui::pos2(b.x0 as f32, b.y0 as f32),
                    egui::pos2(b.x1 as f32, b.y1 as f32),
                ),
            ))
        })
        .collect();
    output.textures_delta.clear();
    result
}
fn setup() -> (egui::Context, WireframeApp) {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.canvas_rect = Rect::from_min_size(egui::pos2(200., 150.), Vec2::new(600., 450.));
    app.sync_viewport();
    (ctx, app)
}

#[test]
fn icon_hover_preserves_ink_and_disabled_undo_is_dimmed_once() {
    let (ctx, mut app) = setup();
    frame(&mut app, &ctx, 940., vec![]);
    let bounds = frame(&mut app, &ctx, 940., vec![]);
    let add = bounds["New wireframe"];
    let undo = bounds["Undo"];
    let output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(940., 700.))),
            events: vec![egui::Event::PointerMoved(add.center())],
            ..Default::default()
        },
        |ui| app.toolbar(ui),
    );
    let lines = |bounds: Rect| {
        output
            .shapes
            .iter()
            .filter_map(move |shape| match &shape.shape {
                egui::Shape::LineSegment { points, stroke }
                    if points.iter().all(|p| bounds.contains(*p)) =>
                {
                    Some(stroke.color)
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(lines(add), vec![Color32::WHITE; 2]);
    let disabled: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Path(path)
                if path.closed && undo.contains(Rect::from_points(&path.points).center()) =>
            {
                Some(path.fill)
            }
            _ => None,
        })
        .collect();
    assert_eq!(disabled, vec![Color32::WHITE.gamma_multiply(0.4)]);
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::Shape::Rect(rect) if rect.rect == add && rect.fill != Color32::TRANSPARENT
    )));
}

#[test]
fn pressed_icon_lights_only_inside_and_release_outside_does_not_activate() {
    let (ctx, mut app) = setup();
    frame(&mut app, &ctx, 940., vec![]);
    let add = frame(&mut app, &ctx, 940., vec![])["New wireframe"];
    let before = app.editor.document.clone();
    frame(
        &mut app,
        &ctx,
        940.,
        vec![egui::Event::PointerMoved(add.center())],
    );
    for (position, events, expected) in [
        (
            add.center(),
            vec![egui::Event::PointerButton {
                pos: add.center(),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            }],
            chrome::toolbar_palette().pressed,
        ),
        (Pos2::new(500., 500.), vec![], Color32::WHITE),
    ] {
        let mut events = events;
        events.insert(0, egui::Event::PointerMoved(position));
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(940., 700.))),
                events,
                ..Default::default()
            },
            |ui| app.toolbar(ui),
        );
        let colors: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::LineSegment { points, stroke }
                    if points.iter().all(|p| add.contains(*p)) =>
                {
                    Some(stroke.color)
                }
                _ => None,
            })
            .collect();
        assert_eq!(colors, vec![expected; 2]);
    }
    frame(
        &mut app,
        &ctx,
        940.,
        vec![egui::Event::PointerButton {
            pos: Pos2::new(500., 500.),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert_eq!(app.editor.document, before);
}
fn click(app: &mut WireframeApp, ctx: &egui::Context, label: &str) {
    frame(app, ctx, 940., vec![]);
    let nodes = frame(app, ctx, 940., vec![]);
    let pos = nodes
        .get(label)
        .unwrap_or_else(|| panic!("Missing {label}; {:?}", nodes.keys()))
        .center();
    frame(app, ctx, 940., vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        frame(
            app,
            ctx,
            940.,
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
#[test]
fn panel_icons_fill_when_open_and_return_to_outlines_when_closed() {
    for label in ["Pages", "Library", "Comments", "Inspector"] {
        let (ctx, mut app) = setup();
        app.show_navigator = false;
        app.show_library = false;
        app.show_inspector = false;
        app.review.open = false;
        let before = app.editor.document.clone();
        for selected in [false, true, false] {
            frame(&mut app, &ctx, 940., vec![]);
            let bounds = frame(&mut app, &ctx, 940., vec![])[label];
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(940., 700.))),
                    events: vec![egui::Event::PointerMoved(Pos2::new(500., 500.))],
                    ..Default::default()
                },
                |ui| app.toolbar(ui),
            );
            let has_filled_body = output.shapes.iter().any(|shape| {
                let (fill, body) = match &shape.shape {
                    egui::Shape::Rect(rect) => (rect.fill, rect.rect),
                    egui::Shape::Circle(circle) => (
                        circle.fill,
                        Rect::from_center_size(circle.center, Vec2::splat(circle.radius * 2.)),
                    ),
                    _ => return false,
                };
                fill == chrome::toolbar_palette().selected
                    && bounds.contains(body.center())
                    && body.area() > 100.
            });
            assert_eq!(has_filled_body, selected, "{label}: open={selected}");
            assert_eq!(app.editor.document, before);
            click(&mut app, &ctx, label);
        }
    }
}
#[test]
fn toolbar_actions_do_not_overlap_at_supported_widths() {
    let (ctx, mut app) = setup();
    app.editor.trash_pages(&[1]).unwrap();
    app.editor.document.title = "A long project 👩‍💻 שלום ".repeat(100);
    let names = [
        "Pages",
        "New wireframe",
        "Trash",
        "Undo",
        "Redo",
        "Zoom out",
        "Zoom in",
        "Actual size",
        "Fit page",
        "Focus Quick Add",
        "Quick Add",
        "Library",
        "Comments",
        "Inspector",
        "Present",
    ];
    for width in [860., 940., 1360., 1920.] {
        let nodes = frame(&mut app, &ctx, width, vec![]);
        for (i, name) in names.iter().enumerate() {
            let a = nodes[*name];
            assert!(a.min.x >= 0. && a.max.x <= width, "{name} {a:?}");
            for other in names.iter().skip(i + 1) {
                assert!(
                    a.intersect(nodes[*other]).width() <= 0.
                        || a.intersect(nodes[*other]).height() <= 0.,
                    "{name} overlaps {other} at {width}"
                );
            }
        }
        assert!(nodes["Actual size"].min.x < nodes["Fit page"].min.x);
        assert!(nodes["Pages"].min.x < nodes["Undo"].min.x);
        assert!(nodes["Fit page"].max.x < nodes["Trash"].min.x);
        assert!(nodes["Trash"].max.x < nodes["Library"].min.x);
        assert!(!nodes.contains_key("Sketch style"));
        assert!(!nodes.contains_key("Snap to 10"));
    }
}

#[test]
fn action_groups_keep_compact_frames_and_measured_spacing() {
    for width in [860., 940., 1360., 1920.] {
        for trash in [false, true] {
            let (ctx, mut app) = setup();
            if trash {
                app.editor.trash_pages(&[1]).unwrap();
            }
            frame(&mut app, &ctx, width, vec![]);
            let nodes = frame(&mut app, &ctx, width, vec![]);
            for name in [
                "Pages",
                "New wireframe",
                "Undo",
                "Redo",
                "Zoom out",
                "Zoom in",
                "Actual size",
                "Fit page",
                "Library",
                "Comments",
                "Inspector",
                "Present",
            ] {
                assert_eq!(nodes[name].size(), Vec2::splat(20.), "{name}");
            }
            for (a, b) in [
                ("Undo", "Redo"),
                ("Zoom out", "Zoom in"),
                ("Zoom in", "Actual size"),
                ("Actual size", "Fit page"),
                ("Library", "Comments"),
                ("Comments", "Inspector"),
            ] {
                assert_eq!(nodes[b].center().x - nodes[a].center().x, 30., "{a}/{b}");
            }
            for (a, b) in [
                ("Pages", "New wireframe"),
                ("Redo", "Zoom out"),
                ("Inspector", "Present"),
            ] {
                assert_eq!(nodes[b].center().x - nodes[a].center().x, 50., "{a}/{b}");
            }
            let end = if nodes.contains_key("Trash") {
                "Trash"
            } else {
                "Fit page"
            };
            let group = nodes["Undo"].union(nodes[end]);
            assert!((group.center().x - width * 0.5).abs() < 0.01);
            if end == "Trash" {
                assert_eq!(nodes[end].size(), Vec2::new(24., 25.));
            }
        }
    }
}
#[test]
fn source_navigation_uses_the_main_action_row_without_overlap() {
    let (ctx, mut app) = setup();
    let library = app.editor.create_library("Shared controls").unwrap();
    let source = app
        .editor
        .create_empty_source(
            library,
            "Long component name ".repeat(40).as_str(),
            wireframe::document::Point { x: 6., y: 6. },
        )
        .unwrap();
    app.editor.trash_pages(&[1]).unwrap();
    for width in [860., 940., 1360., 1920.] {
        for source_selected in [false, true] {
            if source_selected {
                app.editor.edit_library_source(source).unwrap();
            } else {
                app.editor.edit_library(library).unwrap();
            }
            frame(&mut app, &ctx, width, vec![]);
            let nodes = frame(&mut app, &ctx, width, vec![]);
            let names = [
                "Back to Wireframes",
                "New component library",
                "Trash",
                "Undo",
                "Zoom out",
                "Fit page",
            ];
            for (i, name) in names.iter().enumerate() {
                let bounds = nodes[*name];
                assert!((bounds.center().y - nodes["Undo"].center().y).abs() < 1.);
                for other in names.iter().skip(i + 1) {
                    assert!(
                        !bounds.intersects(nodes[*other]),
                        "{name} overlaps {other} at {width}"
                    );
                }
            }
            assert!(!nodes.contains_key("Pages"));
            assert!(!nodes.contains_key("Present"));
            assert!(nodes["Fit page"].max.x < nodes["Trash"].min.x);
            assert!(!app.command_enabled(&ctx, menus::Command::Present));
            app.start_presentation();
            assert!(!app.present);
            assert!(app.editor.in_source_workspace());
            assert!(
                !nodes
                    .keys()
                    .any(|label| label.starts_with("Editing library:")
                        || label.starts_with("Editing component:"))
            );
        }
    }
    let before = app.editor.document.clone();
    click(&mut app, &ctx, "New component library");
    assert_eq!(
        app.editor.document.libraries.len(),
        before.libraries.len() + 1
    );
    click(&mut app, &ctx, "Undo");
    assert_eq!(app.editor.document, before);
    let page = app.editor.page;
    click(&mut app, &ctx, "Back to Wireframes");
    assert!(!app.editor.in_source_workspace());
    assert_eq!(app.editor.page, page);
    assert_eq!(app.editor.document, before);
}

#[test]
fn toolbar_icons_route_zoom_panels_and_page_history() {
    let (ctx, mut app) = setup();
    app.editor.add(Kind::Button, 100., 100.);
    let source = serde_json::to_value(&app.editor.document).unwrap();
    click(&mut app, &ctx, "Zoom in");
    assert_eq!(app.zoom, 1.5);
    click(&mut app, &ctx, "Zoom out");
    assert_eq!(app.zoom, 1.);
    click(&mut app, &ctx, "Fit page");
    assert!(app.zoom.is_finite());
    click(&mut app, &ctx, "Actual size");
    assert_eq!(app.zoom, 1.);
    for (label, previous) in [
        ("Pages", app.show_navigator),
        ("Library", app.show_library),
        ("Inspector", app.show_inspector),
    ] {
        click(&mut app, &ctx, label);
        let current = match label {
            "Pages" => app.show_navigator,
            "Library" => app.show_library,
            _ => app.show_inspector,
        };
        assert_ne!(current, previous);
    }
    assert_eq!(serde_json::to_value(&app.editor.document).unwrap(), source);
    let count = app.editor.document.pages.len();
    click(&mut app, &ctx, "New wireframe");
    assert_eq!(app.editor.document.pages.len(), count + 1);
    click(&mut app, &ctx, "Undo");
    assert_eq!(app.editor.document.pages.len(), count);
    click(&mut app, &ctx, "Redo");
    assert_eq!(app.editor.document.pages.len(), count + 1);
}
#[test]
fn relocated_settings_remain_reachable_and_undoable() {
    let (ctx, mut app) = setup();
    let initial = app.editor.document.settings.sketch;
    click(&mut app, &ctx, "File");
    let nodes = frame(&mut app, &ctx, 940., vec![]);
    for name in [
        "Save                   ⌘/Ctrl S",
        "Font and colors…",
        "Sketch style",
    ] {
        assert!(nodes.contains_key(name), "{name}");
    }
    click(&mut app, &ctx, "Sketch style");
    assert_ne!(app.editor.document.settings.sketch, initial);
    frame(
        &mut app,
        &ctx,
        940.,
        vec![egui::Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    click(&mut app, &ctx, "Undo");
    assert_eq!(app.editor.document.settings.sketch, initial);
    click(&mut app, &ctx, "View");
    click(&mut app, &ctx, "Grid");
    assert!(app.grid);
    click(&mut app, &ctx, "View");
    click(&mut app, &ctx, "Snap to 10");
    assert!(app.snap);
}

#[test]
fn quick_add_search_button_focuses_the_field_and_preserves_insertion_history() {
    let (ctx, mut app) = setup();
    let before = app.editor.document.clone();
    click(&mut app, &ctx, "Focus Quick Add");
    assert!(wireframe::text_entry::is_focused(&ctx));
    frame(
        &mut app,
        &ctx,
        940.,
        vec![egui::Event::Text("Button".into())],
    );
    assert_eq!(app.quick, "Button");
    frame(
        &mut app,
        &ctx,
        940.,
        vec![egui::Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert_eq!(app.editor.current().controls.len(), 1);
    assert_eq!(app.editor.current().controls[0].kind, Kind::Button);
    assert!(app.quick.is_empty());
    click(&mut app, &ctx, "Undo");
    assert_eq!(app.editor.document, before);
}
