// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

fn setup() -> (egui::Context, WireframeApp) {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 20., 30.);
    (ctx, app)
}
fn reply(ctx: &egui::Context, app: &mut WireframeApp, request: u64, result: Result<(), String>) {
    let mut out = ctx.run_ui(
        egui::RawInput {
            events: vec![egui::Event::ClipboardWriteResult { request, result }],
            ..Default::default()
        },
        |ui| app.shortcuts(ui.ctx()),
    );
    out.textures_delta.clear();
}
fn request(app: &WireframeApp) -> u64 {
    app.pending_cut.as_ref().unwrap().request
}

#[test]
fn cut_waits_for_matching_native_success_and_is_one_undoable_edit() {
    let (ctx, mut app) = setup();
    let before = app.editor.document.clone();
    let old_clipboard = serde_json::to_value(&app.clipboard).unwrap();
    app.cut_controls(&ctx);
    let id = request(&app);
    assert_eq!(app.editor.document, before);
    assert_eq!(serde_json::to_value(&app.clipboard).unwrap(), old_clipboard);
    assert!(ctx.output(|o| o.commands.iter().any(|c| matches!(c,egui::OutputCommand::CopyTextWithResult {request,text} if *request==id && text.starts_with("wireframe-nodes-v2:")))));
    reply(&ctx, &mut app, id + 1, Ok(()));
    assert_eq!(app.editor.document, before);
    reply(&ctx, &mut app, id, Ok(()));
    assert!(app.editor.nodes().is_empty());
    let after = app.editor.document.clone();
    assert_eq!(app.clipboard.len(), 1);
    app.editor.undo();
    assert_eq!(app.editor.document, before);
    app.editor.redo();
    assert_eq!(app.editor.document, after);
    reply(&ctx, &mut app, id, Ok(()));
    assert_eq!(app.editor.document, after);
}

#[test]
fn failed_cut_preserves_document_clipboard_paste_offset_and_history() {
    let (ctx, mut app) = setup();
    app.copy(&ctx);
    app.reset_paste_offset(37.);
    let before = app.editor.document.clone();
    let clipboard = serde_json::to_value(&app.clipboard).unwrap();
    let undo = app.editor.undo_name().map(str::to_owned);
    app.cut_controls(&ctx);
    let id = request(&app);
    reply(&ctx, &mut app, id, Err("Clipboard unavailable".into()));
    assert_eq!(app.editor.document, before);
    assert_eq!(serde_json::to_value(&app.clipboard).unwrap(), clipboard);
    assert_eq!(app.clipboard_paste.offset, Point::new(37., 37.));
    assert_eq!(app.editor.undo_name(), undo.as_deref());
    assert!(app.pending_cut.is_none());
    assert!(app.error.as_ref().unwrap().contains("Nothing was cut"));
}

#[test]
fn delayed_success_does_not_cut_changed_document_target_or_session() {
    for case in 0..5 {
        let (ctx, mut app) = setup();
        app.cut_controls(&ctx);
        let id = request(&app);
        match case {
            0 => app.editor.add(Kind::TextInput, 90., 40.),
            1 => app.editor.selection.clear(),
            2 => app.editor = Editor::new(app.editor.document.clone()),
            3 => app.pending = Some(Pending::New),
            _ => app.file = Some(PathBuf::from("/tmp/different-cut-origin.wireframe")),
        }
        let changed = app.editor.document.clone();
        reply(&ctx, &mut app, id, Ok(()));
        assert_eq!(app.editor.document, changed, "case {case}");
        assert_eq!(app.clipboard.len(), 1);
        assert!(app.status.contains("nothing was cut"));
    }
}

#[test]
fn later_copy_cancels_pending_cut_and_late_reply_cannot_overwrite_it() {
    let (ctx, mut app) = setup();
    app.cut_controls(&ctx);
    let id = request(&app);
    app.editor.add(Kind::TextInput, 70., 30.);
    app.copy(&ctx);
    let before = app.editor.document.clone();
    let copy = serde_json::to_value(&app.clipboard).unwrap();
    reply(&ctx, &mut app, id, Ok(()));
    assert_eq!(app.editor.document, before);
    assert_eq!(serde_json::to_value(&app.clipboard).unwrap(), copy);
}

#[test]
fn newer_cut_supersedes_older_request() {
    let (ctx, mut app) = setup();
    app.cut_controls(&ctx);
    let first = request(&app);
    app.cut_controls(&ctx);
    let second = request(&app);
    reply(&ctx, &mut app, first, Ok(()));
    assert_eq!(app.editor.nodes().len(), 1);
    reply(&ctx, &mut app, second, Ok(()));
    assert!(app.editor.nodes().is_empty());
}

#[test]
fn source_and_library_cut_require_publication_and_restore_exact_graphs() {
    for whole_library in [false, true] {
        for success in [false, true] {
            let (ctx, mut app) = setup();
            let source = app.editor.create_component("Action".into()).unwrap();
            let library = app.editor.document.component_source(source).unwrap().0.id;
            let before = app.editor.document.clone();
            let copy = if whole_library {
                app.editor
                    .copy_libraries(&[library], &app.clipboard_origin())
                    .unwrap()
            } else {
                app.editor
                    .copy_sources(&[source], &app.clipboard_origin())
                    .unwrap()
            };
            let scope = if whole_library {
                CutScope::Libraries(vec![library])
            } else {
                CutScope::Sources(vec![source])
            };
            app.request_cut(&ctx, copy, scope).unwrap();
            let id = request(&app);
            assert_eq!(app.editor.document, before);
            reply(
                &ctx,
                &mut app,
                id,
                if success {
                    Ok(())
                } else {
                    Err("write failed".into())
                },
            );
            if !success {
                assert_eq!(app.editor.document, before);
                continue;
            }
            assert!(app.editor.document.definitions.is_empty());
            assert_eq!(app.editor.document.libraries.is_empty(), whole_library);
            assert_eq!(app.editor.document.pages, before.pages);
            let after = app.editor.document.clone();
            app.editor.undo();
            assert_eq!(app.editor.document, before);
            app.editor.redo();
            assert_eq!(app.editor.document, after);
        }
    }
}

#[test]
fn held_drag_consumes_reply_without_cutting_or_leaving_pending_work() {
    let (ctx, mut app) = setup();
    app.cut_controls(&ctx);
    let id = request(&app);
    app.library_drag = Some(CatalogItem::Control(Kind::Button));
    let before = app.editor.document.clone();
    reply(&ctx, &mut app, id, Ok(()));
    assert_eq!(app.editor.document, before);
    assert!(app.pending_cut.is_none());
}

#[test]
fn changed_navigator_selection_cancels_delayed_resource_cut() {
    let (ctx, mut app) = setup();
    let source = app.editor.create_component("Action".into()).unwrap();
    let library = app.editor.document.component_source(source).unwrap().0.id;
    let copy = app
        .editor
        .copy_libraries(&[library], &app.clipboard_origin())
        .unwrap();
    app.request_cut(&ctx, copy, CutScope::Libraries(vec![library]))
        .unwrap();
    let id = request(&app);
    app.source_navigation
        .selected
        .insert(source_navigator::ResourceRow::Source(source));
    let before = app.editor.document.clone();
    reply(&ctx, &mut app, id, Ok(()));
    assert_eq!(app.editor.document, before);
    assert!(app.status.contains("nothing was cut"));
}
