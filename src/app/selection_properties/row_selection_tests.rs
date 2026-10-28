// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use egui::accesskit::{Node, Role, Toggled};
use wireframe::document::{Node as DocumentNode, Point, Project, Size};

fn fixture(indices: [i32; 2], reverse: bool, short_second: bool) -> (egui::Context, WireframeApp) {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    let mut project = Project {
        next_id: 4,
        ..Project::default()
    };
    project.pages[0].content.nodes = indices
        .into_iter()
        .enumerate()
        .map(|(i, index)| {
            let mut node = DocumentNode::control(
                i as u64 + 2,
                "tabs",
                Point::new(50. + i as f32 * 450., 50.),
                Size::fixed(420., 160.),
            );
            let text = if i == 1 && short_second {
                "Inbox, Profile"
            } else {
                ["Summary, Details, Activity", "Inbox, Profile, Logs"][i]
            };
            node.properties
                .insert("text".into(), Value::Text(text.into()));
            node.properties
                .insert("selected_index".into(), Value::Number(f64::from(index)));
            node
        })
        .collect();
    app.editor = wireframe::workspace::Editor::new(project);
    for id in if reverse { [3, 2] } else { [2, 3] } {
        app.editor.select(id, !app.editor.selection.is_empty());
    }
    app.editor.begin("Unrelated formatting");
    app.editor
        .set_control_property(2, "bold", Value::Bool(true))
        .unwrap();
    app.editor.commit();
    app.editor.undo();
    assert!(app.editor.redo_name().is_some());
    (ctx, app)
}

fn frame(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(320., 480.))),
            time: Some(ctx.cumulative_frame_nr() as f64 * 0.25),
            events,
            ..Default::default()
        },
        |ui| app.shared_row_selection(ui),
    );
    out.textures_delta.clear();
    out
}

fn nodes(out: &egui::FullOutput) -> impl Iterator<Item = &Node> {
    out.platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .map(|(_, node)| node)
}

fn center(node: &Node) -> Pos2 {
    let bounds = node.bounds().unwrap();
    Pos2::new(
        ((bounds.x0 + bounds.x1) / 2.) as f32,
        ((bounds.y0 + bounds.y1) / 2.) as f32,
    )
}

fn click(ctx: &egui::Context, app: &mut WireframeApp, at: Pos2) -> egui::FullOutput {
    let pointer = |pressed| {
        vec![
            egui::Event::PointerMoved(at),
            egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            },
        ]
    };
    frame(ctx, app, pointer(true));
    frame(ctx, app, pointer(false))
}

fn open(ctx: &egui::Context, app: &mut WireframeApp) -> egui::FullOutput {
    frame(ctx, app, vec![]);
    let out = frame(ctx, app, vec![]);
    let at = center(
        nodes(&out)
            .find(|node| node.role() == Role::ComboBox)
            .unwrap(),
    );
    click(ctx, app, at);
    frame(ctx, app, vec![])
}

#[test]
fn shared_row_caption_reports_mixed_indices_and_dismissal_preserves_redo() {
    for indices in [[2, 0], [-1, 0], [0, -1], [2, 2], [-1, -1]] {
        for reverse in [false, true] {
            let (ctx, mut app) = fixture(indices, reverse, false);
            let before = app.editor.document.clone();
            let redo = app.editor.redo_name().map(str::to_owned);
            let rows = app.editor.row_selection().unwrap();
            let captions = rows.captions();
            let expected = if rows.mixed {
                "Mixed selection"
            } else if rows.value < 0 {
                "None"
            } else {
                &captions[rows.value as usize]
            };
            frame(&ctx, &mut app, vec![]);
            let out = frame(&ctx, &mut app, vec![]);
            let combo = nodes(&out)
                .find(|node| node.role() == Role::ComboBox)
                .unwrap();
            assert_eq!(combo.value(), Some(expected), "{indices:?}/{reverse}");
            open(&ctx, &mut app);
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::Key {
                    key: Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
            );
            assert!(app.editor.document == before);
            assert_eq!(app.editor.redo_name(), redo.as_deref());
            let mut expected_redo = before.clone();
            expected_redo.pages[0].content.nodes[0]
                .properties
                .insert("bold".into(), Value::Bool(true));
            app.editor.redo();
            assert!(app.editor.document == expected_redo);
            app.editor.undo();
            assert!(app.editor.document == before);
        }
    }
}

#[test]
fn mixed_row_popup_selects_nothing_until_a_choice_updates_both_controls_atomically() {
    for indices in [[2, 0], [-1, 0], [0, -1]] {
        for reverse in [false, true] {
            let (ctx, mut app) = fixture(indices, reverse, false);
            let before = app.editor.document.clone();
            let rows = app.editor.row_selection().unwrap();
            assert!(rows.mixed);
            let captions = rows.captions();
            let chosen = rows.value;
            let caption = if chosen < 0 {
                "None"
            } else {
                &captions[chosen as usize]
            };
            let out = open(&ctx, &mut app);
            for label in std::iter::once("None").chain(captions.iter().map(String::as_str)) {
                let node = nodes(&out)
                    .find(|node| node.role() == Role::Button && node.label() == Some(label))
                    .unwrap();
                assert_eq!(
                    node.toggled(),
                    Some(Toggled::False),
                    "{indices:?}/{reverse}: {label}"
                );
            }
            let at = center(
                nodes(&out)
                    .find(|node| node.label() == Some(caption))
                    .unwrap(),
            );
            click(&ctx, &mut app, at);
            let mut expected = before.clone();
            for node in &mut expected.pages[0].content.nodes {
                let value = Value::Number(f64::from(chosen));
                if node.properties.get("selected_index") == Some(&value) {
                    continue;
                }
                // Changed default values become sparse; already-current
                // explicit defaults are retained by the selection operation.
                if chosen == -1 {
                    node.properties.remove("selected_index");
                } else {
                    node.properties.insert("selected_index".into(), value);
                }
            }
            assert!(app.editor.document == expected, "{indices:?}/{reverse}");
            assert!(!app.editor.row_selection().unwrap().mixed);
            let out = open(&ctx, &mut app);
            let selected: Vec<_> = nodes(&out)
                .filter(|node| node.toggled() == Some(Toggled::True))
                .filter_map(Node::label)
                .collect();
            assert_eq!(selected, [caption]);
            app.editor.undo();
            assert!(app.editor.document == before);
            app.editor.redo();
            assert!(app.editor.document == expected);
        }
    }
}

#[test]
fn unavailable_row_keeps_mixed_feedback_and_noop_choice_preserves_redo() {
    let (ctx, mut app) = fixture([2, 0], false, true);
    let before = app.editor.document.clone();
    let redo = app.editor.redo_name().map(str::to_owned);
    let out = open(&ctx, &mut app);
    let row = nodes(&out)
        .find(|node| node.label() == Some("Activity / -"))
        .unwrap();
    click(&ctx, &mut app, center(row));
    assert!(app.editor.document == before);
    assert_eq!(app.editor.redo_name(), redo.as_deref());
    assert!(app.editor.row_selection().unwrap().mixed);
    let out = frame(&ctx, &mut app, vec![]);
    let combo = nodes(&out)
        .find(|node| node.role() == Role::ComboBox)
        .unwrap();
    assert_eq!(combo.value(), Some("Mixed selection"));
}
