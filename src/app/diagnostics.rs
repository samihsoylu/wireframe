// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Bounded owned trace. A UI snapshot is provisional; only the backend queries final focus.
use eframe::egui::{self, Id, ViewportId};
use egui_winit::diagnostics::{Observer, Receipt};
use serde_json::{Value, json};
use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::Mutex,
};
const MAX_RECORDS: usize = 4096;
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Snapshot {
    pub graph: String,
    pub dirty: bool,
    pub undo: bool,
    pub redo: bool,
    pub jobs: usize,
    pub page: u64,
    pub selection: Vec<u64>,
    pub blocked: bool,
    pub text_edit: bool,
    pub drag: bool,
    pub pending: bool,
    pub deferred: usize,
}
struct State {
    writer: BufWriter<File>,
    count: usize,
    invalid: bool,
    ready: bool,
    closed: bool,
    close_requested: bool,
    close_accepted: bool,
    stable_target: Option<Id>,
    rendered_target: Option<Id>,
    completed_target: Option<(u64, Id)>,
    in_target_pass: bool,
    source: Option<u64>,
    baseline: Option<String>,
    snapshot: Option<Snapshot>,
}
pub(crate) struct Trace {
    run: String,
    state: Mutex<State>,
}
impl Trace {
    pub(crate) fn new(run: String, file: File) -> Self {
        Self {
            run,
            state: Mutex::new(State {
                writer: BufWriter::new(file),
                count: 0,
                invalid: false,
                ready: false,
                closed: false,
                close_requested: false,
                close_accepted: false,
                stable_target: None,
                rendered_target: None,
                completed_target: None,
                in_target_pass: false,
                source: None,
                baseline: None,
                snapshot: None,
            }),
        }
    }
    fn append(&self, s: &mut State, stage: &str, data: Value) {
        if s.count >= MAX_RECORDS {
            s.invalid = true;
            return;
        }
        let value = json!({"run":self.run,"record":s.count+1,"stage":stage,"data":data});
        let bytes = match serde_json::to_vec(&value) {
            Ok(b) if b.len() <= 2048 => b,
            _ => {
                s.invalid = true;
                return;
            }
        };
        if s.writer
            .write_all(&bytes)
            .and_then(|_| s.writer.write_all(b"\n"))
            .is_err()
        {
            s.invalid = true;
        }
        s.count += 1;
    }
    pub(crate) fn record(&self, stage: &str, data: Value) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if stage == "close_or_request_received" {
            s.close_requested = true;
        }
        if matches!(stage, "close_sent" | "native_termination_approved") {
            s.close_accepted = true;
        }
        self.append(&mut s, stage, data);
    }
    pub(crate) fn invalid(&self, reason: &'static str) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.invalid = true;
        self.append(&mut s, "invalid", json!({"reason":reason}));
    }
    pub(crate) fn flush(&self) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.writer.flush().is_err() {
            s.invalid = true;
        }
    }
    pub(super) fn baseline(&self, source: u64, graph: String) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.source = Some(source);
        s.baseline = Some(graph);
    }
    fn begin_target_pass(&self) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.rendered_target = None;
        s.completed_target = None;
        s.snapshot = None;
        s.in_target_pass = true;
    }
    fn complete_target_pass(&self, ctx: &egui::Context) {
        // output_hook runs after Memory::end_pass and the counter increment.
        // This is an observed completed-pass stamp, never a UI counter offset.
        let completed_pass = ctx.cumulative_pass_nr_for(ViewportId::ROOT);
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.in_target_pass
            && s.source.is_some_and(|source| {
                s.snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.selection.as_slice() == [source])
            })
        {
            s.completed_target = s.rendered_target.map(|target| (completed_pass, target));
        }
        s.in_target_pass = false;
    }
    pub(super) fn target_for_source(&self, ctx: &egui::Context, source: u64, target: Id) {
        let root = ctx.viewport_id() == ViewportId::ROOT;
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !root
            || !s.in_target_pass
            || s.source != Some(source)
            || s.stable_target.is_some_and(|old| old != target)
        {
            s.invalid = true;
            return;
        }
        s.stable_target = Some(target);
        s.rendered_target = Some(target);
    }
    pub(super) fn snapshot(&self, ctx: &egui::Context, snapshot: Snapshot) {
        let batch = egui_winit::diagnostics::batch(ctx, ctx.viewport_id());
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !s.in_target_pass {
            s.invalid = true;
            return;
        }
        if s.snapshot.as_ref() != Some(&snapshot) || batch.is_some_and(|b| b.event_count > 0) {
            self.append(&mut s,"ui_return_provisional",json!({"viewport":format!("{:?}",ctx.viewport_id()),"frame":ctx.cumulative_frame_nr(),"pass":ctx.cumulative_pass_nr(),"batch":batch.map(|b|b.id),"focus":ctx.memory(|m|m.focused()).map(|id|format!("{id:?}")),"graph":snapshot.graph,"dirty":snapshot.dirty,"undo":snapshot.undo,"redo":snapshot.redo,"jobs":snapshot.jobs,"page":snapshot.page,"selection":snapshot.selection,"blocked":snapshot.blocked,"text_edit":snapshot.text_edit,"drag":snapshot.drag,"pending":snapshot.pending,"deferred":snapshot.deferred}));
        }
        s.snapshot = Some(snapshot);
    }
    pub(crate) fn healthy(&self) -> bool {
        !self.state.lock().unwrap_or_else(|e| e.into_inner()).invalid
    }
    pub(crate) fn complete(&self) -> bool {
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.ready && s.closed && !s.invalid
    }
    pub(crate) fn finish(&self) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !s.close_requested || !s.close_accepted {
            s.invalid = true;
            self.append(
                &mut s,
                "invalid",
                json!({"reason":"missing_close_acknowledgement"}),
            );
        }
        let valid = s.ready && !s.invalid;
        self.append(&mut s, "on_exit_complete", json!({"valid":valid}));
        if s.writer.flush().is_err() || s.writer.get_ref().sync_all().is_err() {
            s.invalid = true;
        }
        s.closed = valid && !s.invalid;
    }
    pub(super) fn is_baseline(&self, graph: &str) -> bool {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .baseline
            .as_deref()
            == Some(graph)
    }
    #[cfg(test)]
    pub(super) fn valid(&self) -> bool {
        !self.state.lock().unwrap().invalid
    }
}
impl Observer for Trace {
    fn target(&self, ctx: &egui::Context, viewport: ViewportId) -> Option<Id> {
        if viewport != ViewportId::ROOT || ctx.viewport_id() != viewport {
            return None;
        }
        let pass = ctx.cumulative_pass_nr_for(viewport);
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.in_target_pass {
            return None;
        }
        s.completed_target
            .filter(|(completed, _)| *completed == pass)
            .map(|(_, target)| target)
    }
    fn record(&self, r: Receipt) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if r.stage == "window_received" && r.pressed.is_none() {
            return;
        }
        if r.stage == "input_batch" && r.count == 0 {
            return;
        }
        let target_current = r.stage == "output_complete"
            && r.completed_passes > 0
            && r.target
                .is_some_and(|target| s.completed_target == Some((r.pass, target)))
            && s.source.is_some_and(|source| {
                s.snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.selection.as_slice() == [source])
            });
        // Every completed output is an acknowledgement, including a new batch
        // with unchanged focus. The hard record cap fails closed on overflow.
        let target_pass = s.completed_target.map(|(pass, _)| pass);
        self.append(&mut s,r.stage,json!({"viewport":format!("{:?}",r.viewport),"event":r.seq,"first_event":r.first_event,"command":r.command,"batch":r.batch,"key":r.key.map(|k|format!("{k:?}")),"pressed":r.pressed,"repeat":r.repeat,"physical_key":r.physical_key.map(|k|format!("{k:?}")),"modifiers":format!("{:?}",r.modifiers),"count":r.count,"frame":r.frame,"pass":r.pass,"completed_passes":r.completed_passes,"discarded_passes":r.discarded_passes,"target_pass":target_pass,"target_current":target_current,"target":r.target.map(|id|format!("{id:?}")),"focus":r.focus.map(|id|format!("{id:?}")),"target_open":r.target_open}));
        if r.stage == "output_complete" {
            if !target_current {
                s.invalid = true;
                self.append(
                    &mut s,
                    "invalid",
                    json!({"reason":"missing_or_stale_target"}),
                );
            }
            if !s.ready
                && !s.invalid
                && s.snapshot.as_ref().is_some_and(|a| {
                    !a.dirty
                        && !a.undo
                        && !a.redo
                        && a.jobs == 0
                        && !a.blocked
                        && !a.text_edit
                        && !a.drag
                        && !a.pending
                        && a.deferred == 0
                        && Some(&a.graph) == s.baseline.as_ref()
                })
            {
                s.ready = true;
                self.append(
                    &mut s,
                    "ready",
                    json!({"keyboard_only":true,"drag_and_services_outside_boundary":true}),
                );
            }
            if s.writer.flush().is_err() {
                s.invalid = true;
            }
        }
    }
}

struct TargetLifecycle(std::sync::Weak<Trace>);
impl egui::plugin::Plugin for TargetLifecycle {
    fn debug_name(&self) -> &'static str {
        "wireframe-diagnostic-target-lifecycle"
    }
    fn on_begin_pass(&mut self, ui: &mut egui::Ui) {
        if let Some(trace) = self.0.upgrade() {
            if ui.ctx().viewport_id() == ViewportId::ROOT {
                trace.begin_target_pass();
            } else {
                trace.invalid("unsupported_target_viewport");
            }
        }
    }
    fn output_hook(&mut self, ctx: &egui::Context, _: &mut egui::FullOutput) {
        if ctx.viewport_id() == ViewportId::ROOT
            && let Some(trace) = self.0.upgrade()
        {
            trace.complete_target_pass(ctx);
        }
    }
}

impl super::WireframeApp {
    pub(super) fn workspace_in(
        ctx: &egui::Context,
        environment: super::environment::AppEnvironment,
        normal_recovery: impl FnOnce() -> Option<std::path::PathBuf>,
    ) -> Self {
        let recovery = environment.normal_only(normal_recovery).flatten();
        let mut app = Self::workspace(ctx, recovery);
        app.environment = environment;
        if let Some(trace) = app.environment.trace() {
            ctx.data_mut(|d| {
                d.insert_temp(egui::Id::new("wireframe-diagnostic-trace"), trace.clone())
            });
            ctx.add_plugin(TargetLifecycle(std::sync::Arc::downgrade(trace)));
            app.seed_diagnostic_fixture();
        }
        app
    }
    pub(super) fn seed_diagnostic_fixture(&mut self) {
        use sha2::{Digest, Sha256};
        use wireframe::{document::Value, model::Kind, workspace::Editor};
        let mut editor = Editor::default();
        editor.add(Kind::TextArea, 100., 150.);
        let source = *editor.selection.iter().next().unwrap();
        editor
            .set_control_property(
                source,
                "text",
                Value::Text("Diagnostic whole-control link".into()),
            )
            .unwrap();
        editor.add(Kind::Button, 450., 150.);
        let project = editor.document.clone();
        self.editor = Editor::new(project.clone());
        self.editor.select(source, false);
        self.show_library = false;
        self.show_navigator = false;
        if let Some(trace) = self.environment.trace() {
            let hash = Sha256::digest(project.encode().unwrap())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            trace.baseline(source, hash);
        }
    }
    pub(super) fn diagnostic_jobs(&self) -> usize {
        [
            self.startup_open.is_some(),
            self.persistence.is_some(),
            self.pdf_job.is_some(),
            self.image_export.job.is_some(),
            self.web_image.is_some(),
            self.screenshot_text.is_some(),
            self.more_controls.is_some(),
            self.image_source_request.is_some(),
        ]
        .into_iter()
        .filter(|b| *b)
        .count()
    }
    pub(super) fn diagnostic_command(
        &self,
        ctx: &egui::Context,
        stage: &'static str,
        command: u64,
        outcome: &'static str,
    ) {
        if let Some(trace) = self.environment.trace() {
            trace.record(stage,json!({"command":command,"outcome":outcome,"durability_ack":false,"viewport":format!("{:?}",ctx.viewport_id()),"batch":egui_winit::diagnostics::batch(ctx,ctx.viewport_id()).map(|b|b.id),"frame":ctx.cumulative_frame_nr(),"pass":ctx.cumulative_pass_nr(),"dirty":self.editor.dirty(),"jobs":self.diagnostic_jobs()}));
        }
    }
    pub(super) fn diagnostic_stage(&self, stage: &'static str) {
        if let Some(trace) = self.environment.trace() {
            trace.record(
                stage,
                json!({"dirty":self.editor.dirty(),"jobs":self.diagnostic_jobs()}),
            );
        }
    }
    pub(super) fn diagnostic_snapshot(&self, ctx: &egui::Context) {
        use sha2::{Digest, Sha256};
        if let Some(trace) = self.environment.trace() {
            let bytes = match self.editor.document.encode() {
                Ok(b) => b,
                Err(_) => {
                    trace.invalid("invalid_project");
                    return;
                }
            };
            let hash = Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            trace.snapshot(
                ctx,
                Snapshot {
                    graph: hash,
                    dirty: self.editor.dirty(),
                    undo: self.editor.undo_name().is_some(),
                    redo: self.editor.redo_name().is_some(),
                    jobs: self.diagnostic_jobs(),
                    page: self.editor.page,
                    selection: self.editor.selection.iter().copied().collect(),
                    blocked: self.canvas_input_blocked(),
                    text_edit: self.text_edit.is_some() || ctx.text_edit_focused(),
                    drag: self.drag.is_some() || self.library_drag.is_some(),
                    pending: self.pending.is_some()
                        || self.pending_cut.is_some()
                        || self.menus.has_pending_text_command(),
                    deferred: self.deferred_text_input.len()
                        + self.deferred_cut_input.len()
                        + usize::from(self.deferred_text_focus.is_some()),
                },
            );
        }
    }
    pub(super) fn diagnostic_input(
        &self,
        ctx: &egui::Context,
        raw: &egui::RawInput,
        stage: &'static str,
    ) {
        if let Some(trace) = self.environment.trace()
            && !raw.events.is_empty()
        {
            let keys: Vec<_> = raw.events.iter().filter_map(|event| {
                if let egui::Event::Key {key, pressed, modifiers, repeat, physical_key} = event {
                    Some(json!({"key":format!("{key:?}"),"pressed":pressed,"modifiers":format!("{modifiers:?}"),"repeat":repeat,"physical_key":physical_key.map(|k|format!("{k:?}"))}))
                } else { None }
            }).collect();
            trace.record(stage,json!({"viewport":format!("{:?}",ctx.viewport_id()),"batch":egui_winit::diagnostics::batch(ctx,ctx.viewport_id()).map(|b|b.id),"keys":keys,"event_count":raw.events.len(),"deferred":self.deferred_text_input.len(),"frame":ctx.cumulative_frame_nr(),"pass":ctx.cumulative_pass_nr()}));
        }
    }
}

#[cfg(all(test, unix))]
mod tests;
