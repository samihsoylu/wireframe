use super::*;
use crate::app::{Pending, WireframeApp, environment, test_driver};
use eframe::App;
use wireframe::{document::Project, storage::ProjectRepository};
struct ForbiddenStore;
impl ProjectRepository for ForbiddenStore {
    fn load(&self, _: &std::path::Path) -> Result<Project, String> {
        panic!("normal project read")
    }
    fn save(&self, _: &std::path::Path, _: &Project) -> Result<(), String> {
        panic!("normal project write")
    }
}
fn fixture() -> (tempfile::TempDir, egui::Context, WireframeApp, Arc<Trace>) {
    let (root, env) = environment::diagnostic_fixture();
    let trace = env.trace().unwrap().clone();
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace_in(&ctx, env, || panic!("normal recovery resolver"));
    app.store = Box::new(ForbiddenStore);
    (root, ctx, app, trace)
}
use std::sync::Arc;
fn frame(ctx: &egui::Context, app: &mut WireframeApp, events: Vec<egui::Event>) {
    test_driver::application_frame(
        ctx,
        app,
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1360., 1000.),
            )),
            events,
            ..Default::default()
        },
    )
    .0
    .drop_without_applying_deltas();
}
#[test]
fn memory_startup_denies_effect_leaves_without_normal_data_or_jobs() {
    let (root, ctx, mut app, trace) = fixture();
    let before = app.editor.document.clone();
    assert!(app.persistence.is_none() && app.recovery.is_none() && app.file.is_none());
    assert!(!app.editor.dirty());
    assert!(app.editor.undo_name().is_none());
    assert!(
        app.environment
            .normal_only(|| panic!("normal settings/updater factory"))
            .is_none()
    );
    app.enable_persistence(&ctx, root.path().join("forbidden-state"));
    app.set_save_mode(true);
    for command in [
        super::super::menus::Command::Save,
        super::super::menus::Command::SaveAs,
        super::super::menus::Command::New,
        super::super::menus::Command::ExportPdf,
    ] {
        app.run_command(&ctx, command);
    }
    app.start_startup_open(root.path().join("missing.wireframe"), &ctx);
    assert!(!app.save(false));
    assert!(app.choose_save_path().is_none());
    app.open(&ctx);
    app.export(false);
    app.start_image_import(wireframe::workspace::ImageImportTarget::Controls(vec![]));
    app.start_web_image(vec![]);
    app.start_image_search(vec![]);
    app.choose_screenshot_text(&ctx);
    app.extract_asset_text(&ctx, 999);
    app.open_more_controls();
    app.open_image_export();
    app.request(Pending::Open(root.path().join("missing")), &ctx);
    app.follow_target("https://invalid.example/", &ctx);
    assert!(ctx.output_mut(|o| o.commands.is_empty()));
    assert_eq!(app.editor.document, before);
    assert_eq!(app.diagnostic_jobs(), 0);
    assert!(!root.path().join("forbidden-state").exists());
    for _ in 0..3 {
        frame(&ctx, &mut app, vec![]);
    }
    let bridge = egui_winit::diagnostics::Bridge::new(trace.clone());
    bridge.output_completed(&ctx, egui::ViewportId::ROOT, 1, 0);
    assert!(trace.valid());
    app.request(Pending::Quit, &ctx);
    assert!(app.quit_allowed);
    app.on_exit(None);
    assert!(trace.complete());
    let names: Vec<_> = std::fs::read_dir(root.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names.len(), 2);
    let log = std::fs::read_to_string(root.path().join("trace.jsonl")).unwrap();
    assert!(
        log.contains("ready") && log.contains("on_exit_complete") && log.contains("effect_denied")
    );
}
#[test]
fn unexpected_dirty_exit_is_not_reset_or_accepted() {
    let (_root, ctx, mut app, trace) = fixture();
    frame(&ctx, &mut app, vec![]);
    egui_winit::diagnostics::Bridge::new(trace.clone()).output_completed(
        &ctx,
        egui::ViewportId::ROOT,
        1,
        0,
    );
    app.editor.add(wireframe::model::Kind::Label, 30., 30.);
    assert!(app.editor.dirty());
    app.on_exit(None);
    assert!(app.editor.dirty());
    assert!(!trace.complete());
}
#[test]
fn diagnostics_bound_output_and_never_log_input_text() {
    let (root, ctx, app, trace) = fixture();
    let secret = "not-a-trace-payload";
    let raw = egui::RawInput {
        events: vec![
            egui::Event::Text(secret.into()),
            egui::Event::Paste(secret.into()),
        ],
        ..Default::default()
    };
    app.diagnostic_input(&ctx, &raw, "hook_enter");
    trace.flush();
    assert!(
        !std::fs::read_to_string(root.path().join("trace.jsonl"))
            .unwrap()
            .contains(secret)
    );
    for _ in 0..=MAX_RECORDS {
        trace.record("bounded", json!({}));
    }
    trace.flush();
    assert!(!trace.valid());
    assert!(
        std::fs::metadata(root.path().join("trace.jsonl"))
            .unwrap()
            .len()
            < MAX_RECORDS as u64 * 2048
    );
}

#[test]
fn missing_close_receipt_cannot_acknowledge_exit() {
    let (_root, ctx, mut app, trace) = fixture();
    frame(&ctx, &mut app, vec![]);
    egui_winit::diagnostics::Bridge::new(trace.clone()).output_completed(
        &ctx,
        egui::ViewportId::ROOT,
        1,
        0,
    );
    assert!(trace.valid());
    app.on_exit(None);
    assert!(!trace.complete());
}

#[test]
fn normal_workspace_retains_normal_resolver_and_does_not_seed_diagnostic_content() {
    let ctx = egui::Context::default();
    let mut calls = 0;
    let app = WireframeApp::workspace_in(&ctx, environment::AppEnvironment::Normal, || {
        calls += 1;
        None
    });
    assert_eq!(calls, 1);
    assert!(!app.environment.diagnostic());
    assert!(app.editor.selection.is_empty());
    assert!(app.editor.undo_name().is_none());
}

#[test]
fn unknown_target_and_trace_flush_failure_cannot_acknowledge_readiness() {
    let (root, ctx, _app, trace) = fixture();
    egui_winit::diagnostics::Bridge::new(trace.clone()).output_completed(
        &ctx,
        egui::ViewportId::ROOT,
        1,
        0,
    );
    assert!(!trace.valid());
    assert!(!trace.complete());
    let readonly = std::fs::File::open(root.path().join("trace.jsonl")).unwrap();
    let broken = Trace::new("readonly-fixture".into(), readonly);
    broken.record("bounded", json!({}));
    broken.flush();
    assert!(!broken.healthy());
    assert!(!broken.complete());
}

#[test]
fn ready_waits_for_blocking_owner_even_when_backend_focus_stamp_is_unchanged() {
    let (root, ctx, mut app, trace) = fixture();
    frame(&ctx, &mut app, vec![]);
    let bridge = egui_winit::diagnostics::Bridge::new(trace.clone());
    assert!(Observer::target(trace.as_ref(), &ctx, egui::ViewportId::ROOT).is_some());
    assert!(
        Observer::target(
            trace.as_ref(),
            &ctx,
            egui::ViewportId::from_hash_of("unsupported-child")
        )
        .is_none()
    );
    app.error = Some("owned fixture error".into());
    frame(&ctx, &mut app, vec![]);
    bridge.output_completed(&ctx, egui::ViewportId::ROOT, 1, 0);
    trace.flush();
    assert!(trace.valid());
    let ready = || {
        std::fs::read_to_string(root.path().join("trace.jsonl"))
            .unwrap()
            .contains("\"stage\":\"ready\"")
    };
    assert!(!ready());
    app.error = None;
    frame(&ctx, &mut app, vec![]);
    bridge.output_completed(&ctx, egui::ViewportId::ROOT, 1, 0);
    trace.flush();
    assert!(ready());
    let rows: Vec<serde_json::Value> = std::fs::read_to_string(root.path().join("trace.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        rows.iter()
            .filter(|row| row["stage"] == "output_complete")
            .count(),
        2
    );
}

fn output_rows(root: &std::path::Path) -> Vec<serde_json::Value> {
    std::fs::read_to_string(root.join("trace.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .filter(|row| row["stage"] == "output_complete")
        .collect()
}
fn complete_frame(ctx: &egui::Context, trace: &Arc<Trace>) {
    egui_winit::diagnostics::Bridge::new(trace.clone()).output_completed(
        ctx,
        egui::ViewportId::ROOT,
        1,
        0,
    );
}
#[test]
fn completed_output_rejects_target_after_inspector_is_hidden() {
    for acknowledge_first in [false, true] {
        let (root, ctx, mut app, trace) = fixture();
        frame(&ctx, &mut app, vec![]);
        if acknowledge_first {
            complete_frame(&ctx, &trace);
            assert!(trace.valid());
        }
        let graph = app.editor.document.clone();
        app.show_inspector = false;
        frame(&ctx, &mut app, vec![]);
        assert_eq!(app.editor.document, graph);
        complete_frame(&ctx, &trace);
        assert!(
            !trace.valid(),
            "hidden target must invalidate even when graph is unchanged"
        );
        assert!(output_rows(root.path()).last().unwrap()["data"]["target"].is_null());
        let log = std::fs::read_to_string(root.path().join("trace.jsonl")).unwrap();
        assert_eq!(
            log.lines()
                .filter(|line| line.contains("\"stage\":\"ready\""))
                .count(),
            usize::from(acknowledge_first)
        );
    }
}
#[test]
fn completed_output_rejects_target_after_expected_source_is_deselected() {
    let (root, ctx, mut app, trace) = fixture();
    frame(&ctx, &mut app, vec![]);
    let graph = app.editor.document.clone();
    app.editor.selection.clear();
    frame(&ctx, &mut app, vec![]);
    assert_eq!(app.editor.document, graph);
    complete_frame(&ctx, &trace);
    assert!(
        !trace.valid(),
        "an empty selection cannot acknowledge the fixture's target"
    );
    assert!(output_rows(root.path()).last().unwrap()["data"]["target"].is_null());
    assert!(
        !std::fs::read_to_string(root.path().join("trace.jsonl"))
            .unwrap()
            .contains("\"stage\":\"ready\"")
    );
}
fn two_passes(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    final_visible: bool,
) -> egui::FullOutput {
    frame(ctx, app, vec![]); // Mount once before testing a genuine discarded/final distinction.
    let mut raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1360., 1000.),
        )),
        ..Default::default()
    };
    app.raw_input_hook(ctx, &mut raw);
    let mut passes = 0;
    let out = ctx.run_ui(raw, |ui| {
        passes += 1;
        app.show_inspector = passes == 1 || final_visible;
        app.ui(ui, &mut eframe::Frame::_new_kittest());
        let mounted = app
            .environment
            .trace()
            .unwrap()
            .state
            .lock()
            .unwrap()
            .rendered_target;
        assert_eq!(
            mounted.is_some(),
            passes == 1 || final_visible,
            "the actual property UI must mount only in the expected pass"
        );
        if passes == 1 {
            ctx.request_discard("target provenance fixture");
        }
    });
    assert_eq!(passes, 2);
    assert_eq!(out.platform_output.num_completed_passes, 2);
    out
}
#[test]
fn discarded_pass_target_cannot_acknowledge_a_final_pass_without_it() {
    let (root, ctx, mut app, trace) = fixture();
    let out = two_passes(&ctx, &mut app, false);
    egui_winit::diagnostics::Bridge::new(trace.clone()).output_completed(
        &ctx,
        egui::ViewportId::ROOT,
        out.platform_output.num_completed_passes,
        out.platform_output.num_completed_passes - 1,
    );
    assert!(
        !trace.valid(),
        "the discarded pass's mounted target cannot survive the final pass"
    );
    assert!(output_rows(root.path()).last().unwrap()["data"]["target"].is_null());
    out.drop_without_applying_deltas();
}
#[test]
fn final_pass_rendered_target_preserves_valid_multipass_acknowledgement() {
    let (root, ctx, mut app, trace) = fixture();
    let out = two_passes(&ctx, &mut app, true);
    egui_winit::diagnostics::Bridge::new(trace.clone()).output_completed(
        &ctx,
        egui::ViewportId::ROOT,
        out.platform_output.num_completed_passes,
        out.platform_output.num_completed_passes - 1,
    );
    assert!(trace.valid());
    let rows = output_rows(root.path());
    let data = &rows.last().unwrap()["data"];
    assert!(!data["target"].is_null());
    assert_eq!(data["completed_passes"], 2);
    assert_eq!(data["discarded_passes"], 1);
    assert!(
        std::fs::read_to_string(root.path().join("trace.jsonl"))
            .unwrap()
            .contains("\"stage\":\"ready\"")
    );
    out.drop_without_applying_deltas();
}

#[test]
fn previously_blocked_ready_cannot_accept_a_now_hidden_target() {
    let (root, ctx, mut app, trace) = fixture();
    app.error = Some("blocking fixture owner".into());
    frame(&ctx, &mut app, vec![]);
    complete_frame(&ctx, &trace);
    assert!(trace.valid());
    assert!(
        !std::fs::read_to_string(root.path().join("trace.jsonl"))
            .unwrap()
            .contains("\"stage\":\"ready\"")
    );
    app.error = None;
    app.show_inspector = false;
    frame(&ctx, &mut app, vec![]);
    complete_frame(&ctx, &trace);
    assert!(!trace.valid());
    assert!(output_rows(root.path()).last().unwrap()["data"]["target"].is_null());
    assert!(
        !std::fs::read_to_string(root.path().join("trace.jsonl"))
            .unwrap()
            .contains("\"stage\":\"ready\"")
    );
}
#[test]
fn replaced_source_and_logic_only_output_cannot_reuse_the_fixture_target() {
    let (root, ctx, mut app, trace) = fixture();
    frame(&ctx, &mut app, vec![]);
    let source = *app.editor.selection.iter().next().unwrap();
    let other = app
        .editor
        .current()
        .controls
        .iter()
        .find(|control| control.id != source)
        .unwrap()
        .id;
    app.editor.select(other, false);
    frame(&ctx, &mut app, vec![]);
    complete_frame(&ctx, &trace);
    assert!(!trace.valid());
    assert!(output_rows(root.path()).last().unwrap()["data"]["target"].is_null());
    let (root, ctx, mut app, trace) = fixture();
    frame(&ctx, &mut app, vec![]);
    let out = ctx.run_logic(&egui::RawInput::default(), |_| {});
    assert_eq!(out.platform_output.num_completed_passes, 0);
    egui_winit::diagnostics::Bridge::new(trace.clone()).output_completed(
        &ctx,
        egui::ViewportId::ROOT,
        out.platform_output.num_completed_passes,
        0,
    );
    assert!(!trace.valid());
    assert!(output_rows(root.path()).last().unwrap()["data"]["target"].is_null());
}
