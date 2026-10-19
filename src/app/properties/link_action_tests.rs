// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::collections::BTreeMap;

fn frame(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    events: Vec<egui::Event>,
) -> BTreeMap<String, Pos2> {
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 1000.))),
            events,
            time: Some(ctx.cumulative_frame_nr() as f64 * 0.25),
            ..Default::default()
        },
        |ui| app.inspector(ui),
    );
    let result = out
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .rev()
        .filter_map(|(_, n)| {
            let label = n.label().or_else(|| n.value())?;
            let b = n.bounds()?;
            Some((
                label.to_owned(),
                Pos2::new(((b.x0 + b.x1) / 2.) as f32, ((b.y0 + b.y1) / 2.) as f32),
            ))
        })
        // Repeated row labels select the topmost row, independently of
        // accessibility node IDs and allocation order.
        .fold(
            BTreeMap::<String, Pos2>::new(),
            |mut labels, (label, pos)| {
                labels
                    .entry(label)
                    .and_modify(|old| {
                        if pos.y < old.y {
                            *old = pos;
                        }
                    })
                    .or_insert(pos);
                labels
            },
        );
    out.textures_delta.clear();
    result
}
fn click(ctx: &egui::Context, app: &mut WireframeApp, label: &str) {
    frame(ctx, app, vec![]);
    let mut labels = frame(ctx, app, vec![]);
    let mut p = *labels
        .get(label)
        .unwrap_or_else(|| panic!("Missing {label}: {:?}", labels.keys()));
    // Property sets can exceed the viewport. Exercise their real scroll area
    // before clicking, rather than sending a pointer outside the window.
    for _ in 0..8 {
        if (8.0..=980.0).contains(&p.y) {
            break;
        }
        frame(
            ctx,
            app,
            vec![
                egui::Event::PointerMoved(Pos2::new(p.x, 500.)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    phase: egui::TouchPhase::Move,
                    delta: Vec2::new(0., if p.y > 980. { -250. } else { 250. }),
                    modifiers: Modifiers::NONE,
                },
            ],
        );
        labels = frame(ctx, app, vec![]);
        p = *labels.get(label).unwrap();
    }
    assert!(
        (8.0..=980.0).contains(&p.y),
        "{label} is offscreen at {p:?}"
    );
    for pressed in [true, false] {
        frame(
            ctx,
            app,
            vec![
                egui::Event::PointerMoved(p),
                egui::Event::PointerButton {
                    pos: p,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
}
#[test]
fn actual_link_menu_creates_undoable_destination_and_notice_opens_it() {
    for kind in [Kind::Button, Kind::TextInput] {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(kind, 100., 100.);
        let id = *app.editor.selection.iter().next().unwrap();
        let source = app.editor.page;
        let before = app.editor.document.clone();
        click(&ctx, &mut app, "No link");
        click(&ctx, &mut app, "Link to new wireframe");
        let Some(Link::Page(dest)) = app.editor.node_link(id) else {
            panic!("No destination")
        };
        assert_eq!(app.editor.page, source);
        assert_eq!(app.editor.document.pages.len(), 2);
        let created = app.editor.document.clone();
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        app.editor.redo();
        assert_eq!(app.editor.document, created);
        click(&ctx, &mut app, "Open wireframe");
        assert_eq!(app.editor.page, dest);
    }
}
#[test]
fn actual_mixed_picker_can_apply_the_first_existing_target_to_every_control() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 20., 20.);
    let first = *app.editor.selection.iter().next().unwrap();
    app.editor.set_node_link(first, Some(Link::Back)).unwrap();
    app.editor.add(Kind::Button, 180., 20.);
    let second = *app.editor.selection.iter().next().unwrap();
    app.editor.select_all();
    let before = app.editor.document.clone();
    click(&ctx, &mut app, "Mixed links");
    click(&ctx, &mut app, "Go back");
    assert_eq!(app.editor.node_link(first), Some(Link::Back));
    assert_eq!(app.editor.node_link(second), Some(Link::Back));
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    click(&ctx, &mut app, "Mixed links");
    click(&ctx, &mut app, "Link to duplicate of this wireframe");
    assert_eq!(app.editor.document.pages.len(), 2);
    assert_eq!(
        app.editor.document.pages[1].content,
        before.pages[0].content
    );
    assert_eq!(app.editor.node_link(first), app.editor.node_link(second));
    app.editor.undo();
    assert_eq!(app.editor.document, before);
}
#[test]
fn actual_row_picker_creates_destination_without_overwriting_other_slots() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::ButtonBar, 20., 20.);
    let id = *app.editor.selection.iter().next().unwrap();
    app.editor
        .set_control_property(id, "text", Value::Text("Contact".into()))
        .unwrap();
    let before = app.editor.document.clone();
    click(&ctx, &mut app, "Edit segment links");
    click(&ctx, &mut app, "No link");
    click(&ctx, &mut app, "Link to new wireframe");
    let Some(Link::Page(dest)) = app.editor.link_target(LinkTarget::row(id, 0)) else {
        panic!("No row destination")
    };
    assert_eq!(app.editor.document.live_page(dest).unwrap().name, "Contact");
    app.editor.undo();
    assert_eq!(app.editor.document, before);
}

#[test]
fn common_row_picker_uses_shared_rows_and_isolates_created_notice_between_projects() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    let mut ids = vec![];
    for caption in ["Contact,Order", "Support"] {
        app.editor.add(Kind::ButtonBar, 20., 20.);
        let id = *app.editor.selection.iter().next().unwrap();
        app.editor
            .set_control_property(id, "text", Value::Text(caption.into()))
            .unwrap();
        ids.push(id);
    }
    app.editor.select(ids[1], false);
    app.editor.select(ids[0], true);
    let before = app.editor.document.clone();
    click(&ctx, &mut app, "Edit row links");
    let labels = frame(&ctx, &mut app, vec![]);
    assert!(labels.contains_key("Support / Contact"));
    assert!(labels.contains_key("- / Order"));
    click(&ctx, &mut app, "No link / No link");
    click(&ctx, &mut app, "Link to new wireframe");
    let link = app.editor.link_target(LinkTarget::row(ids[0], 0));
    let Some(Link::Page(page)) = link else {
        panic!("No shared destination")
    };
    assert_eq!(app.editor.document.live_page(page).unwrap().name, "Support");
    assert_eq!(app.editor.link_target(LinkTarget::row(ids[1], 0)), link);
    assert_eq!(app.editor.link_target(LinkTarget::row(ids[0], 1)), None);
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    let labels = frame(&ctx, &mut app, vec![]);
    assert!(!labels.contains_key("Open wireframe"));
    app.editor = wireframe::workspace::Editor::default();
    app.editor.add_page();
    let labels = frame(&ctx, &mut app, vec![]);
    assert!(!labels.contains_key("Open wireframe"));
}

#[test]
fn actual_inline_picker_creates_destination_preserving_the_text_field() {
    for kind in [Kind::Paragraph, Kind::Table] {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(kind, 20., 20.);
        let id = *app.editor.selection.iter().next().unwrap();
        app.editor
            .set_control_property(id, "text", Value::Text("[*Contact*](back:)".into()))
            .unwrap();
        let before = app.editor.document.clone();
        if kind == Kind::Table {
            frame(&ctx, &mut app, vec![]);
            frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::PointerMoved(Pos2::new(1080., 500.)),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        phase: egui::TouchPhase::Move,
                        delta: Vec2::new(0., -5000.),
                        modifiers: Modifiers::NONE,
                    },
                ],
            );
            for _ in 0..5 {
                frame(&ctx, &mut app, vec![]);
            }
        }
        click(&ctx, &mut app, "Go back");
        click(&ctx, &mut app, "Link to new wireframe");
        let Some(Link::Page(dest)) = app.editor.link_target(LinkTarget::text(id, 0)) else {
            panic!("No inline target")
        };
        assert_eq!(
            app.editor.document.live_page(dest).unwrap().name,
            "*Contact*"
        );
        assert_eq!(
            app.editor.control_properties(id).unwrap().1.get("text"),
            Some(&Value::Text(format!("[*Contact*](page:{dest})")))
        );
        assert_eq!(app.editor.node_link(id), None);
        app.editor.undo();
        assert_eq!(app.editor.document, before);
    }
}
#[test]
fn actual_common_inline_picker_sets_only_matching_text_positions() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    let mut ids = vec![];
    for source in ["[One](back:)", "[Two]"] {
        app.editor.add(Kind::Paragraph, 20., 20.);
        let id = *app.editor.selection.iter().next().unwrap();
        app.editor
            .set_control_property(id, "text", Value::Text(source.into()))
            .unwrap();
        ids.push(id);
    }
    app.editor.select_all();
    let before = app.editor.document.clone();
    click(&ctx, &mut app, "Mixed links");
    click(&ctx, &mut app, "Go back");
    for id in ids {
        assert_eq!(
            app.editor.link_target(LinkTarget::text(id, 0)),
            Some(Link::Back)
        );
        assert_eq!(app.editor.node_link(id), None);
    }
    app.editor.undo();
    assert_eq!(app.editor.document, before);
}

#[test]
fn inspector_edits_readable_names_without_storing_them_or_changing_idle_history() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    let source = app.editor.page;
    app.editor.add_page();
    let destination = app.editor.page;
    app.editor.rename_page(destination, "Contact").unwrap();
    app.editor.switch_page(source);
    app.editor.add(Kind::Paragraph, 20., 20.);
    let id = *app.editor.selection.iter().next().unwrap();
    app.editor
        .set_control_property(id, "text", Value::Text(format!("[Go](page:{destination})")))
        .unwrap();
    let before = app.editor.document.clone();
    for _ in 0..3 {
        frame(&ctx, &mut app, vec![]);
    }
    assert_eq!(app.editor.document, before);
    click(&ctx, &mut app, "[Go](Contact)");
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::Key {
            key: Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::COMMAND,
        }],
    );
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::Paste(
            "Edited [Go](Contact) [Home](Wireframe 1)".into(),
        )],
    );
    click(&ctx, &mut app, "13");
    assert_eq!(
        app.editor.current().controls[0].text,
        format!("Edited [Go](page:{destination}) [Home](page:{source})")
    );
    let after = app.editor.document.clone();
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    app.editor.redo();
    assert_eq!(app.editor.document, after);
}
