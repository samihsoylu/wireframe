// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

fn fixture() -> (egui::Context, WireframeApp) {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.rename_page(1, "Landing").unwrap();
    app.editor.add(Kind::Button, 20., 30.);
    app.editor.add_page();
    let second = app.editor.page;
    app.editor.rename_page(second, "Contact").unwrap();
    app.editor.switch_page(1);
    app.navigator.selected = HashSet::from([1]);
    (ctx, app)
}

#[test]
fn page_cut_waits_for_publication_and_native_paste_restores_complete_page() {
    let (ctx, mut app) = fixture();
    let before = app.editor.document.clone();
    app.copy_page_resources(&ctx, &[1], true);
    let request = app.pending_cut_request().unwrap();
    assert_eq!(app.editor.document, before);
    app.finish_cut(&ctx, request, &Ok(()));
    assert!(app.editor.document.live_page(1).is_none());
    let current = app.editor.page;
    let text = format!(
        "wireframe-nodes-v2:{}",
        serde_json::to_string(&app.clipboard).unwrap()
    );
    app.pasted(&text);
    assert_eq!(app.editor.document.live_page(1), before.live_page(1));
    assert_eq!(app.editor.page, current);
    assert!(app.error.is_none(), "{:?}", app.error);
}

#[test]
fn failed_or_stale_page_cut_never_removes_pages() {
    for stale in [false, true] {
        let (ctx, mut app) = fixture();
        let before = app.editor.document.clone();
        app.copy_page_resources(&ctx, &[1], true);
        let request = app.pending_cut_request().unwrap();
        if stale {
            app.navigator.selected.clear();
        }
        app.finish_cut(
            &ctx,
            request,
            &if stale {
                Ok(())
            } else {
                Err("Clipboard busy".into())
            },
        );
        assert_eq!(app.editor.document, before);
        assert!(app.pending_cut_request().is_none());
    }
}

#[test]
fn identical_foreign_page_paste_reuses_without_a_dialog_or_repeated_history() {
    let (_, source) = fixture();
    let copy = source.editor.copy_pages(&[1], "other-project").unwrap();
    let (_, mut target) = fixture();
    let pages = target.editor.document.pages.clone();
    target.paste_clipboard(&copy);
    assert!(target.page_paste.is_none());
    assert!(target.error.is_none());
    assert_eq!(target.editor.document.pages, pages);
    let after = target.editor.document.clone();
    let undo = target.editor.undo_name().map(str::to_owned);
    target.paste_clipboard(&copy);
    assert!(target.page_paste.is_none());
    assert_eq!(target.editor.document, after);
    assert_eq!(target.editor.undo_name(), undo.as_deref());
}

fn changed_foreign_source() -> WireframeApp {
    let (_, mut source) = fixture();
    let id = source.editor.current().controls[0].id;
    source
        .editor
        .set_control_property(
            id,
            "text",
            wireframe::document::Value::Text("Incoming change".into()),
        )
        .unwrap();
    source
}

#[test]
fn foreign_page_conflicts_are_modal_cancelable_and_do_not_change_document() {
    let source = changed_foreign_source();
    let copy = source.editor.copy_pages(&[1], "other-project").unwrap();
    let (ctx, mut target) = fixture();
    let before = target.editor.document.clone();
    target.paste_clipboard(&copy);
    assert!(target.page_paste.is_some());
    assert!(target.canvas_input_blocked());
    assert_eq!(target.editor.document, before);
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 750.))),
            events: vec![egui::Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
            ..Default::default()
        },
        |ui| target.page_paste_dialog(ui.ctx()),
    );
    out.textures_delta.clear();
    assert!(target.page_paste.is_none());
    assert_eq!(target.editor.document, before);
}

#[test]
fn foreign_page_keep_both_reuses_shared_review_and_preserves_current_selection() {
    let source = changed_foreign_source();
    let copy = source.editor.copy_pages(&[1], "other-project").unwrap();
    let (_, mut target) = fixture();
    let before = target.editor.document.clone();
    target.paste_clipboard(&copy);
    let mut review = target.page_paste.take().unwrap();
    review.decisions.choose(review::Choice::Both);
    let prepared = review.decisions.take_ready(&target.editor).unwrap();
    let prepared = target
        .editor
        .position_page_clipboard_import(&copy, prepared, &review.selected, &review.collapsed)
        .unwrap();
    let ids = target.editor.apply_page_paste(prepared).unwrap();
    assert_eq!(ids.len(), 1);
    assert_eq!(target.editor.page, 1);
    assert_eq!(target.editor.document.pages.len(), before.pages.len() + 1);
    target.editor.undo();
    assert_eq!(target.editor.document, before);
}

#[test]
fn no_conflict_foreign_paste_applies_without_an_unnecessary_dialog() {
    let (_, source) = fixture();
    let copy = source.editor.copy_pages(&[1], "other-project").unwrap();
    let ctx = egui::Context::default();
    let mut target = WireframeApp::workspace(&ctx, None);
    target.paste_clipboard(&copy);
    assert!(target.page_paste.is_none());
    assert_eq!(target.editor.document.pages.len(), 2);
    assert_eq!(target.editor.document.pages[1].name, "Landing");
    assert!(target.error.is_none(), "{:?}", target.error);
}

#[test]
fn page_navigator_does_not_paste_control_payload_into_hidden_canvas_selection() {
    let (_, mut app) = fixture();
    app.editor.select_all();
    let controls = app.editor.copy();
    let text = format!(
        "wireframe-nodes-v2:{}",
        serde_json::to_string(&controls).unwrap()
    );
    let before = app.editor.document.clone();
    app.pasted_page_text(&text);
    assert_eq!(app.editor.document, before);
    assert_eq!(app.status, "Select the canvas to paste controls");
}

#[test]
fn context_paste_uses_the_requested_noncurrent_page_without_changing_selection() {
    let (ctx, mut app) = fixture();
    let copy = app
        .editor
        .copy_pages(&[1], &app.clipboard_origin())
        .unwrap();
    let target = app.editor.document.pages[1].id;
    app.request_page_paste(&ctx, &[target]);
    app.pasted(&format!(
        "wireframe-nodes-v2:{}",
        serde_json::to_string(&copy).unwrap()
    ));
    assert_eq!(app.editor.page, 1);
    assert_eq!(app.editor.document.pages[1].id, target);
    assert_eq!(app.editor.document.pages[2].name, "Landing (1)");
    assert!(app.page_paste_request.is_none());
}

#[test]
fn delayed_context_paste_rejects_a_changed_project() {
    let (ctx, mut app) = fixture();
    let copy = app
        .editor
        .copy_pages(&[1], &app.clipboard_origin())
        .unwrap();
    app.request_page_paste(&ctx, &[1]);
    app.editor.rename_page(1, "Changed").unwrap();
    let before = app.editor.document.clone();
    app.pasted(&format!(
        "wireframe-nodes-v2:{}",
        serde_json::to_string(&copy).unwrap()
    ));
    assert_eq!(app.editor.document, before);
    assert!(app.page_paste_request.is_none());
    assert!(app.error.as_ref().unwrap().contains("target changed"));
}

#[test]
fn foreign_conflict_actions_remain_pointer_accessible_after_summary_reopen() {
    for size in [Vec2::new(1360., 932.), Vec2::new(800., 600.)] {
        let source = changed_foreign_source();
        let copy = source.editor.copy_pages(&[1], "other-project").unwrap();
        let (ctx, mut target) = fixture();
        ctx.enable_accesskit();
        let frame = |target: &mut WireframeApp, events| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                    events,
                    ..Default::default()
                },
                |ui| target.page_paste_dialog(ui.ctx()),
            );
            output.textures_delta.clear();
            output
        };
        target.paste_clipboard(&copy);
        // First show the short summary, then reopen the larger comparison using
        // the same modal identity. A previous height must not clip its actions.
        target
            .page_paste
            .as_mut()
            .unwrap()
            .decisions
            .choose(review::Choice::Existing);
        for _ in 0..4 {
            frame(&mut target, vec![]);
        }
        target.page_paste = None;
        frame(&mut target, vec![]);
        target.paste_clipboard(&copy);
        for _ in 0..4 {
            frame(&mut target, vec![]);
        }
        let output = frame(&mut target, vec![]);
        let bounds = output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find_map(|(_, node)| {
                (node.label() == Some("Keep Both"))
                    .then(|| node.bounds())
                    .flatten()
            })
            .unwrap();
        let pos = egui::pos2(
            ((bounds.x0 + bounds.x1) / 2.) as f32,
            ((bounds.y0 + bounds.y1) / 2.) as f32,
        );
        assert!(Rect::from_min_size(Pos2::ZERO, size).contains(pos));
        frame(
            &mut target,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
        frame(
            &mut target,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            }],
        );
        assert!(
            target
                .page_paste
                .as_ref()
                .is_some_and(|paste| paste.decisions.prepared.is_some()),
            "visible conflict action must accept a real pointer click at {size:?}"
        );
    }
}

#[test]
fn version_fifteen_page_clipboard_reads_and_new_page_transport_is_guarded() {
    let (ctx, mut source) = fixture();
    let copy = source.editor.copy_pages(&[1], "older-source").unwrap();
    let mut json = serde_json::to_value(&copy).unwrap();
    json["pages"]["project"]["format_version"] = 15.into();
    let migrated: Clipboard = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(
        migrated.pages.as_ref().unwrap().project(),
        copy.pages.as_ref().unwrap().project()
    );
    let old_text = format!(
        "wireframe-nodes-v2:{}",
        serde_json::to_string(&json).unwrap()
    );
    let mut target = WireframeApp::workspace(&ctx, None);
    target.pasted(&old_text);
    assert!(target.error.is_none(), "{:?}", target.error);
    assert_eq!(target.editor.document.pages.len(), 2);
    assert!(
        target
            .editor
            .document
            .pages
            .iter()
            .any(|page| page.name == "Landing")
    );
    source.publish_clipboard(&ctx, copy).unwrap();
    let new_text = ctx.output(|output| {
        output
            .commands
            .iter()
            .find_map(|command| {
                if let egui::OutputCommand::CopyText(text) = command {
                    Some(text.clone())
                } else {
                    None
                }
            })
            .unwrap()
    });
    assert!(new_text.starts_with("wireframe-nodes-v3:"));
    let mut fresh = WireframeApp::workspace(&ctx, None);
    fresh.pasted(&new_text);
    assert!(fresh.error.is_none(), "{:?}", fresh.error);
    assert_eq!(fresh.editor.document, target.editor.document);
}
