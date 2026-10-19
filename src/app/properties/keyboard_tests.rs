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
