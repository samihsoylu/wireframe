// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use eframe::App;

fn frame(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1360., 1000.))),
        events,
        ..Default::default()
    };
    app.raw_input_hook(ctx, &mut raw);
    ctx.run_ui(raw, |ui| app.ui(ui, &mut eframe::Frame::_new_kittest()))
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
fn labels(out: &egui::FullOutput) -> Vec<(egui::accesskit::NodeId, String)> {
    out.platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter_map(|(id, node)| {
            node.label()
                .or_else(|| node.value())
                .map(|label| (*id, label.to_owned()))
        })
        .collect()
}
fn focus_label(ctx: &egui::Context, out: &egui::FullOutput) -> Option<String> {
    let id = ctx.memory(|m| m.focused())?.accesskit_id();
    labels(out)
        .into_iter()
        .find_map(|(candidate, label)| (candidate == id).then_some(label))
}

#[test]
fn reverse_tab_wrap_exposes_canvas_focus_to_keyboard_users() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.show_library = true;
    app.show_navigator = true;
    let page = app.editor.page;
    app.editor.add_page();
    app.editor.switch_page(page);
    app.editor.add(Kind::Button, 80., 70.);
    let document = app.editor.document.clone();
    app.quick_focus = true;
    for _ in 0..3 {
        frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
    }
    let out = frame(&ctx, &mut app, vec![]);
    assert!(app.quick_input_id.is_some());
    assert_eq!(ctx.memory(|m| m.focused()), app.quick_input_id);
    out.drop_without_applying_deltas();

    for _ in 0..64 {
        let mut tab = key(Key::Tab);
        if let egui::Event::Key { modifiers, .. } = &mut tab {
            modifiers.shift = true;
        }
        frame(&ctx, &mut app, vec![tab]).drop_without_applying_deltas();
        // Reverse traversal schedules its destination for the following frame.
        frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
        let out = frame(&ctx, &mut app, vec![]);
        assert_eq!(app.editor.document, document);
        if ctx.memory(|m| m.focused()) == app.canvas_focus_id {
            assert_eq!(
                focus_label(&ctx, &out).as_deref(),
                Some("Drawing canvas"),
                "the canvas reached by reverse wrap needs an identifiable focus target"
            );
            let canvas_outline = app.canvas_rect.shrink(1.);
            let has_outline = |out: &egui::FullOutput| {
                out.shapes.iter().any(|shape| {
                    matches!(&shape.shape, egui::epaint::Shape::Rect(rect)
                        if rect.rect == canvas_outline
                        && rect.stroke.width > 0.
                        && rect.stroke.color != Color32::TRANSPARENT)
                })
            };
            assert!(has_outline(&out), "keyboard canvas focus must be visible");
            out.drop_without_applying_deltas();
            frame(&ctx, &mut app, vec![key(Key::Tab)]).drop_without_applying_deltas();
            frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
            let out = frame(&ctx, &mut app, vec![]);
            assert_ne!(ctx.memory(|m| m.focused()), app.canvas_focus_id);
            assert!(!has_outline(&out), "the outline must follow actual focus");
            assert_eq!(app.editor.document, document);
            out.drop_without_applying_deltas();
            return;
        }
        out.drop_without_applying_deltas();
    }
    panic!("reverse traversal did not reach the canvas with both panes visible");
}

#[test]
fn reverse_tab_after_canvas_reaches_an_identifiable_visible_target() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.show_library = true;
    app.show_navigator = true;
    let page = app.editor.page;
    app.editor.add_page();
    app.editor.switch_page(page);
    app.editor.add(Kind::Button, 80., 70.);
    let document = app.editor.document.clone();
    app.quick_focus = true;
    for _ in 0..3 {
        frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
    }
    let mut reached_canvas = false;
    for _ in 0..64 {
        let mut tab = key(Key::Tab);
        if let egui::Event::Key { modifiers, .. } = &mut tab {
            modifiers.shift = true;
        }
        frame(&ctx, &mut app, vec![tab]).drop_without_applying_deltas();
        frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
        let out = frame(&ctx, &mut app, vec![]);
        assert_eq!(app.editor.document, document);
        if reached_canvas {
            let id = ctx.memory(|m| m.focused()).expect("a focus destination");
            assert_eq!(id, egui::Id::new("inspector").with("__resize"));
            let response = ctx.read_response(id).expect("a live focus destination");
            let node = out
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .find_map(|(candidate, node)| (*candidate == id.accesskit_id()).then_some(node));
            eprintln!(
                "After canvas: name={:?}, role={:?}, rect={:?}, viewport={:?}",
                focus_label(&ctx, &out),
                node.map(|node| node.role()),
                response.rect,
                ctx.content_rect()
            );
            let node = node.expect("keyboard focus must be exposed to accessibility");
            assert_eq!(node.role(), egui::accesskit::Role::Splitter);
            assert_eq!(node.label(), Some("Resize panel width"));
            assert!(
                ctx.content_rect().contains_rect(response.rect),
                "keyboard focus must remain visible in the window"
            );
            let has_focus_line = |out: &egui::FullOutput| {
                out.shapes.iter().any(|shape| {
                    matches!(&shape.shape, egui::epaint::Shape::LineSegment { points, stroke }
                        if response.rect.contains(points[0])
                        && response.rect.contains(points[1])
                        && shape.clip_rect.contains(points[0])
                        && shape.clip_rect.contains(points[1])
                        && stroke.width == 2.
                        && stroke.color == ctx.style_of(ctx.theme()).visuals.selection.stroke.color)
                })
            };
            assert!(
                has_focus_line(&out),
                "keyboard divider focus must be visible"
            );
            out.drop_without_applying_deltas();
            frame(&ctx, &mut app, vec![key(Key::Tab)]).drop_without_applying_deltas();
            frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
            let out = frame(&ctx, &mut app, vec![]);
            assert_ne!(ctx.memory(|m| m.focused()), Some(id));
            assert!(!has_focus_line(&out), "the divider cue must follow focus");
            assert_eq!(app.editor.document, document);
            out.drop_without_applying_deltas();
            return;
        }
        reached_canvas = ctx.memory(|m| m.focused()) == app.canvas_focus_id;
        out.drop_without_applying_deltas();
    }
    panic!("reverse traversal did not pass the canvas with both panes visible");
}

#[test]
fn visible_panes_reverse_to_selected_text_and_forward_to_quick_add() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.show_library = true;
    app.show_navigator = true;
    let page = app.editor.page;
    app.editor.add_page();
    app.editor.switch_page(page);
    app.editor.add(Kind::Button, 80., 70.);
    let document = app.editor.document.clone();
    app.quick_focus = true;
    for _ in 0..3 {
        frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
    }
    let mut reached_canvas = false;
    let mut reached_text = false;
    let mut route = Vec::new();
    for _ in 0..64 {
        let mut tab = key(Key::Tab);
        if let egui::Event::Key { modifiers, .. } = &mut tab {
            modifiers.shift = true;
        }
        frame(&ctx, &mut app, vec![tab]).drop_without_applying_deltas();
        frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
        let out = frame(&ctx, &mut app, vec![]);
        assert_eq!(app.editor.document, document);
        reached_canvas |= ctx.memory(|m| m.focused()) == app.canvas_focus_id;
        if reached_canvas {
            let id = ctx
                .memory(|m| m.focused())
                .expect("live reverse destination");
            let node = out
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .find_map(|(candidate, node)| (*candidate == id.accesskit_id()).then_some(node))
                .expect("reverse destination exposed to accessibility");
            route.push(format!(
                "{:?}/{:?}/{:?}",
                node.role(),
                node.label(),
                node.value()
            ));
            if node.role() == egui::accesskit::Role::MultilineTextInput
                && node.value() == Some("Button")
            {
                let response = ctx.read_response(id).unwrap();
                assert!(
                    ctx.content_rect().contains_rect(response.rect),
                    "text field is visible"
                );
                reached_text = true;
            }
        }
        out.drop_without_applying_deltas();
        if reached_text {
            break;
        }
    }
    assert!(
        reached_text,
        "reverse route did not reach selected Button text: {route:?}"
    );
    eprintln!("Reverse route after canvas: {route:?}");
    for _ in 0..64 {
        frame(&ctx, &mut app, vec![key(Key::Tab)]).drop_without_applying_deltas();
        frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
        let out = frame(&ctx, &mut app, vec![]);
        assert_eq!(app.editor.document, document);
        let reached_quick_add = ctx.memory(|m| m.focused()) == app.quick_input_id;
        out.drop_without_applying_deltas();
        if reached_quick_add {
            return;
        }
    }
    panic!("forward route from selected text did not return to Quick Add");
}

#[test]
fn inspector_popup_arrows_stay_above_background_controls_and_commit_one_edit() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.show_library = false;
    app.show_navigator = false;
    app.editor.add(Kind::TextInput, 100., 150.);
    let id = *app.editor.selection.iter().next().unwrap();
    app.editor
        .set_control_property(id, "text", Value::Text("Your email".into()))
        .unwrap();
    for _ in 0..3 {
        frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
    }
    let out = frame(&ctx, &mut app, vec![]);
    let combo = labels(&out)
        .into_iter()
        .find_map(|(id, label)| (label == "Left").then_some(id))
        .expect("alignment picker");
    out.drop_without_applying_deltas();
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Focus,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: combo,
                data: None,
            },
        )],
    )
    .drop_without_applying_deltas();
    let before = app.editor.document.clone();
    frame(&ctx, &mut app, vec![key(Key::Space)]).drop_without_applying_deltas();
    frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
    for expected in ["Left", "Center", "Right", "Right"] {
        let out = frame(&ctx, &mut app, vec![key(Key::ArrowDown)]);
        assert_eq!(
            focus_label(&ctx, &out).as_deref(),
            Some(expected),
            "arrow focus must remain in the picker"
        );
        out.drop_without_applying_deltas();
        assert_eq!(app.editor.document, before, "moving focus is provisional");
    }
    let out = frame(&ctx, &mut app, vec![key(Key::ArrowUp)]);
    assert_eq!(focus_label(&ctx, &out).as_deref(), Some("Center"));
    out.drop_without_applying_deltas();
    frame(&ctx, &mut app, vec![key(Key::Enter)]).drop_without_applying_deltas();
    frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
    let mut expected = before.clone();
    expected.pages[0].content.nodes[0]
        .properties
        .insert("align".into(), Value::Choice("center".into()));
    assert_eq!(app.editor.document, expected);
    assert!(!egui::Popup::is_any_open(&ctx));
    frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
    assert_eq!(
        ctx.memory(|m| m.focused()).map(|id| id.accesskit_id()),
        Some(combo),
        "keyboard selection must return to its picker"
    );
    frame(&ctx, &mut app, vec![key(Key::ArrowRight)]).drop_without_applying_deltas();
    assert_eq!(
        app.editor.document, expected,
        "leaving the picker must not nudge canvas artwork"
    );
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    let redo = app.editor.redo_name().map(str::to_owned);
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Focus,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: combo,
                data: None,
            },
        )],
    )
    .drop_without_applying_deltas();
    frame(&ctx, &mut app, vec![key(Key::Space)]).drop_without_applying_deltas();
    frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
    frame(&ctx, &mut app, vec![key(Key::ArrowDown)]).drop_without_applying_deltas();
    frame(&ctx, &mut app, vec![key(Key::ArrowDown)]).drop_without_applying_deltas();
    frame(&ctx, &mut app, vec![key(Key::Escape)]).drop_without_applying_deltas();
    frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
    assert!(!egui::Popup::is_any_open(&ctx));
    assert_eq!(
        ctx.memory(|m| m.focused()).map(|id| id.accesskit_id()),
        Some(combo)
    );
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.redo_name(), redo.as_deref());
    assert_eq!(
        app.editor.selection.iter().copied().collect::<Vec<_>>(),
        vec![id]
    );
    app.editor.redo();
    assert_eq!(app.editor.document, expected);
}
