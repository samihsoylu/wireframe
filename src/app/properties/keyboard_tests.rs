// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

fn frame(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    sized_frame(ctx, app, events, Vec2::new(1360., 1000.))
}
fn sized_frame(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    events: Vec<egui::Event>,
    size: Vec2,
) -> egui::FullOutput {
    sized_frame_with_focus(ctx, app, events, size).0
}
fn sized_frame_with_focus(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    events: Vec<egui::Event>,
    size: Vec2,
) -> (egui::FullOutput, Option<egui::Response>) {
    let raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        events,
        ..Default::default()
    };
    let (output, focused_response, _) = crate::app::test_driver::application_frame(ctx, app, raw);
    (output, focused_response)
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
fn every_inspector_keyboard_destination_stays_visible_in_short_windows() {
    for size in [Vec2::new(1360., 900.), Vec2::new(1360., 700.)] {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = WireframeApp::workspace(&ctx, None);
        let project =
            serde_json::from_str(include_str!("fixtures/visible_pane_keyboard.json")).unwrap();
        app.editor = Editor::new(project);
        app.editor.selection.insert(10);
        app.show_library = true;
        app.show_navigator = true;
        let document = app.editor.document.clone();
        app.quick_focus = true;
        for _ in 0..3 {
            sized_frame(&ctx, &mut app, vec![], size).drop_without_applying_deltas();
        }
        assert_eq!(ctx.memory(|m| m.focused()), app.quick_input_id);
        let mut reached_canvas = false;
        let mut reached_text = false;
        let mut inspector_left = None;
        let mut left_inspector = false;
        let mut route = Vec::new();
        for _ in 0..64 {
            let mut tab = key(Key::Tab);
            if let egui::Event::Key { modifiers, .. } = &mut tab {
                modifiers.shift = true;
            }
            sized_frame(&ctx, &mut app, vec![tab], size).drop_without_applying_deltas();
            for _ in 0..3 {
                sized_frame(&ctx, &mut app, vec![], size).drop_without_applying_deltas();
            }
            let (out, focused_response) = sized_frame_with_focus(&ctx, &mut app, vec![], size);
            assert_eq!(app.editor.document, document);
            assert!(app.editor.undo_name().is_none());
            assert!(app.editor.redo_name().is_none());
            reached_canvas |= ctx.memory(|m| m.focused()) == app.canvas_focus_id;
            if reached_canvas {
                let id = ctx.memory(|m| m.focused()).expect("reverse destination");
                let response = focused_response.expect("current painted focus response");
                let node = out
                    .platform_output
                    .accesskit_update
                    .as_ref()
                    .unwrap()
                    .nodes
                    .iter()
                    .find_map(|(candidate, node)| (*candidate == id.accesskit_id()).then_some(node))
                    .expect("source destination exposed to accessibility");
                route.push(format!(
                    "{:?}/{:?}/{:?}",
                    node.role(),
                    node.label(),
                    node.value()
                ));
                if Some(id) == app.canvas_focus_id {
                    inspector_left = Some(response.rect.right());
                } else if reached_text && response.rect.center().x < inspector_left.unwrap() {
                    left_inspector = true;
                    out.drop_without_applying_deltas();
                    break;
                }
                assert!(
                    ctx.content_rect().contains_rect(response.rect)
                        && response
                            .interact_rect
                            .contains_rect(response.rect.shrink(1.)),
                    "{size:?}: focused destination must be visible, rect={:?}, interact={:?}, viewport={:?}, route={route:?}",
                    response.rect,
                    response.interact_rect,
                    ctx.content_rect()
                );
                reached_text |= node.role() == egui::accesskit::Role::MultilineTextInput
                    && node.value() == Some("Before");
            }
            out.drop_without_applying_deltas();
        }
        assert!(
            reached_text,
            "{size:?}: selected text never reached; {route:?}"
        );
        assert!(
            left_inspector,
            "{size:?}: reverse route did not leave the inspector; {route:?}"
        );
        eprintln!("{size:?} complete reverse inspector route: {route:?}");
        let mut returned_to_quick_add = false;
        for _ in 0..96 {
            sized_frame(&ctx, &mut app, vec![key(Key::Tab)], size).drop_without_applying_deltas();
            for _ in 0..3 {
                sized_frame(&ctx, &mut app, vec![], size).drop_without_applying_deltas();
            }
            let (out, response) = sized_frame_with_focus(&ctx, &mut app, vec![], size);
            assert_eq!(app.editor.document, document);
            assert!(app.editor.undo_name().is_none());
            assert!(app.editor.redo_name().is_none());
            let response = response.expect("current forward destination");
            assert!(
                ctx.content_rect().contains_rect(response.rect)
                    && response
                        .interact_rect
                        .contains_rect(response.rect.shrink(1.)),
                "{size:?}: forward destination must be visible: {response:?}"
            );
            returned_to_quick_add = Some(response.id) == app.quick_input_id;
            out.drop_without_applying_deltas();
            if returned_to_quick_add {
                break;
            }
        }
        assert!(
            returned_to_quick_add,
            "{size:?}: forward route did not return to Quick Add"
        );
    }
}

#[test]
fn focus_reveal_respects_pointer_scroll_outside_focus_and_default_behavior() {
    for reveal in [false, true] {
        let ctx = egui::Context::default();
        // AccessKit is deliberately disabled: focus visibility must not depend
        // on an assistive-technology tree being enabled.
        let draw = |events: Vec<egui::Event>, request: Option<bool>| {
            let raw = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(300., 220.))),
                events,
                ..Default::default()
            };
            let mut result = None;
            ctx.run_ui(raw, |ui| {
                let outside = ui.button("Outside scroll area");
                if request == Some(true) {
                    outside.request_focus();
                }
                let mut bottom = None;
                let scroll = egui::ScrollArea::vertical()
                    .max_height(150.)
                    .scroll_to_focused(reveal)
                    .show(ui, |ui| {
                        for index in 0..30 {
                            let response = ui.button(format!("Control {index}"));
                            if index == 29 {
                                if request == Some(false) {
                                    response.request_focus();
                                }
                                bottom = Some(response);
                            }
                        }
                    });
                result = Some((scroll.state.offset.y, scroll.inner_rect, bottom.unwrap()));
            })
            .drop_without_applying_deltas();
            result.unwrap()
        };
        draw(vec![], Some(false));
        for _ in 0..3 {
            draw(vec![], None);
        }
        let (offset, viewport, bottom) = draw(vec![], None);
        assert_eq!(ctx.memory(|m| m.focused()), Some(bottom.id));
        if !reveal {
            assert_eq!(offset, 0.);
            assert!(bottom.rect.top() > viewport.bottom());
            continue;
        }
        assert!(offset > 0.);
        assert!(viewport.contains_rect(bottom.rect));
        draw(
            vec![
                egui::Event::PointerMoved(Pos2::new(bottom.rect.center().x, viewport.center().y)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: Vec2::new(0., 150.),
                    modifiers: Modifiers::NONE,
                    phase: egui::TouchPhase::Move,
                },
            ],
            None,
        );
        for _ in 0..30 {
            draw(vec![], None);
        }
        let (scrolled_offset, viewport, bottom_after_scroll) = draw(vec![], None);
        assert_eq!(ctx.memory(|m| m.focused()), Some(bottom.id));
        assert!(
            scrolled_offset < offset - 100.,
            "pointer scroll must remain free: before={offset}, after={scrolled_offset}, viewport={viewport:?}"
        );
        assert!(bottom_after_scroll.rect.top() > viewport.bottom());
        draw(vec![], Some(true));
        for _ in 0..3 {
            draw(vec![], None);
        }
        let (outside_offset, _, _) = draw(vec![], None);
        assert_ne!(ctx.memory(|m| m.focused()), Some(bottom.id));
        assert!(
            (outside_offset - scrolled_offset).abs() < 0.1,
            "outside focus must not scroll content even when its rectangle overlaps scrolled content"
        );
    }
}

#[test]
fn selected_list_nested_focus_reveal_preserves_graph_history_and_pointer_scroll() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let size = Vec2::new(1360., 700.);
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::List, 80., 70.);
    let selected = *app.editor.selection.iter().next().unwrap();
    app.editor
        .set_control_property(
            selected,
            "text",
            Value::Text(
                (0..30)
                    .map(|i| format!("Row {i:02}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        )
        .unwrap();
    let document = app.editor.document.clone();
    app.editor = Editor::new(document.clone());
    app.editor.selection.insert(selected);
    for _ in 0..3 {
        sized_frame(&ctx, &mut app, vec![], size).drop_without_applying_deltas();
    }
    let out = sized_frame(&ctx, &mut app, vec![], size);
    let header = out
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find_map(|(id, node)| (node.label() == Some("Edit row links")).then_some(*id))
        .expect("actual selected List row-links disclosure");
    out.drop_without_applying_deltas();
    let focus = egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
        action: egui::accesskit::Action::Focus,
        target_node: header,
        target_tree: egui::accesskit::TreeId::ROOT,
        data: None,
    });
    sized_frame(&ctx, &mut app, vec![focus], size).drop_without_applying_deltas();
    for _ in 0..4 {
        sized_frame(&ctx, &mut app, vec![], size).drop_without_applying_deltas();
    }
    assert_eq!(ctx.memory(|m| m.focused()).unwrap().accesskit_id(), header);
    sized_frame(&ctx, &mut app, vec![key(Key::Space)], size).drop_without_applying_deltas();
    let mut rows = 0;
    let mut last = None;
    for _ in 0..96 {
        sized_frame(&ctx, &mut app, vec![key(Key::Tab)], size).drop_without_applying_deltas();
        for _ in 0..8 {
            sized_frame(&ctx, &mut app, vec![], size).drop_without_applying_deltas();
        }
        let (out, response) = sized_frame_with_focus(&ctx, &mut app, vec![], size);
        assert_eq!(app.editor.document, document);
        assert!(app.editor.undo_name().is_none() && app.editor.redo_name().is_none());
        let response = response.expect("current focused inspector row");
        let node = out
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find_map(|(id, node)| (*id == response.id.accesskit_id()).then_some(node))
            .expect("actual focused source node");
        if node.role() == egui::accesskit::Role::ComboBox && node.value() == Some("No link") {
            rows += 1;
            assert!(
                ctx.content_rect().contains_rect(response.rect)
                    && response
                        .interact_rect
                        .contains_rect(response.rect.shrink(1.)),
                "selected List row {rows} must be visible through both inspector clips: {response:?}"
            );
            last = Some(response);
        }
        out.drop_without_applying_deltas();
        if rows == 30 {
            break;
        }
    }
    assert_eq!(rows, 30, "all actual row-link destinations must be reached");
    let last = last.unwrap();
    sized_frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerMoved(last.rect.center()),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                phase: egui::TouchPhase::Move,
                delta: Vec2::new(0., 180.),
                modifiers: Modifiers::NONE,
            },
        ],
        size,
    )
    .drop_without_applying_deltas();
    for _ in 0..30 {
        sized_frame(&ctx, &mut app, vec![], size).drop_without_applying_deltas();
    }
    let (out, response) = sized_frame_with_focus(&ctx, &mut app, vec![], size);
    let response = response.unwrap();
    assert_eq!(response.id, last.id);
    assert!(
        response.rect.top() > last.rect.top() + 100.,
        "nested pointer scroll must stay free"
    );
    assert!(
        !response
            .interact_rect
            .contains_rect(response.rect.shrink(1.))
    );
    assert_eq!(app.editor.document, document);
    assert!(app.editor.undo_name().is_none() && app.editor.redo_name().is_none());
    out.drop_without_applying_deltas();
}

#[test]
fn nested_focus_reveal_keeps_the_destination_inside_both_scroll_viewports() {
    for gap in [0., 350.] {
        let ctx = egui::Context::default();
        scroll_focus_fixture_frame(&ctx, true, true, false, gap);
        for _ in 0..8 {
            scroll_focus_fixture_frame(&ctx, false, true, false, gap);
        }
        let (_, bottom, _) = scroll_focus_fixture_frame(&ctx, false, true, false, gap);
        assert_eq!(ctx.memory(|m| m.focused()), Some(bottom.id));
        assert!(
            ctx.content_rect().contains_rect(bottom.rect)
                && bottom.interact_rect.contains_rect(bottom.rect.shrink(1.)),
            "nested destination must be visible through both clips, gap={gap}: {bottom:?}"
        );
    }
}

#[test]
fn focus_reveal_preserves_an_explicit_scroll_destination() {
    for (nested, gap) in [(false, 0.), (true, 350.)] {
        let ctx = egui::Context::default();
        scroll_focus_fixture_frame(&ctx, true, nested, true, gap);
        for _ in 0..8 {
            scroll_focus_fixture_frame(&ctx, false, nested, true, gap);
        }
        let (top, _, offset) = scroll_focus_fixture_frame(&ctx, false, nested, true, gap);
        assert_eq!(
            offset, 0.,
            "the explicit destination must take precedence, nested={nested}"
        );
        assert!(top.interact_rect.contains_rect(top.rect.shrink(1.)));
    }
}

fn scroll_focus_fixture_frame(
    ctx: &egui::Context,
    request: bool,
    nested: bool,
    explicit: bool,
    gap: f32,
) -> (egui::Response, egui::Response, f32) {
    let mut result = None;
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(300., 250.))),
            ..Default::default()
        },
        |ui| {
            let outer = egui::ScrollArea::vertical()
                .max_height(210.)
                .scroll_to_focused(true)
                .show(ui, |ui| {
                    let parent_top =
                        (nested && explicit).then(|| ui.button("Explicit parent destination"));
                    if request && let Some(top) = &parent_top {
                        ui.scroll_to_rect(top.rect, Some(egui::Align::TOP));
                    }
                    ui.add_space(gap);
                    let controls = |ui: &mut egui::Ui| {
                        let top = ui.button("Explicit top destination");
                        for index in 0..30 {
                            let _ = ui.button(format!("Row {index}"));
                        }
                        let bottom = ui.button("Focused destination");
                        if request {
                            bottom.request_focus();
                            if explicit && !nested {
                                ui.scroll_to_rect(top.rect, Some(egui::Align::TOP));
                            }
                        }
                        (top, bottom)
                    };
                    let responses = if nested {
                        egui::ScrollArea::vertical()
                            .id_salt("nested")
                            .max_height(90.)
                            .scroll_to_focused(true)
                            .show(ui, controls)
                            .inner
                    } else {
                        controls(ui)
                    };
                    ui.add_space(200.);
                    (parent_top.unwrap_or(responses.0), responses.1)
                });
            result = Some((outer.inner.0, outer.inner.1, outer.state.offset.y));
        },
    )
    .drop_without_applying_deltas();
    result.unwrap()
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
