// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Opt-in, immutable keyboard diagnostic bridge. Native drag/Services are outside it.
use egui::{Context, Id, Key, Modifiers, ViewportId};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Debug)]
pub struct Receipt {
    pub stage: &'static str,
    pub viewport: ViewportId,
    pub seq: u64,
    pub first_event: u64,
    pub command: Option<u64>,
    pub batch: u64,
    pub key: Option<Key>,
    pub pressed: Option<bool>,
    pub repeat: Option<bool>,
    pub physical_key: Option<Key>,
    pub modifiers: Modifiers,
    pub count: usize,
    pub frame: u64,
    pub pass: u64,
    pub completed_passes: usize,
    pub discarded_passes: usize,
    pub target: Option<Id>,
    pub focus: Option<Id>,
    pub target_open: Option<bool>,
}
impl Receipt {
    pub fn new(stage: &'static str, viewport: ViewportId) -> Self {
        Self {
            stage,
            viewport,
            seq: 0,
            first_event: 0,
            command: None,
            batch: 0,
            key: None,
            pressed: None,
            repeat: None,
            physical_key: None,
            modifiers: Modifiers::NONE,
            count: 0,
            frame: 0,
            pass: 0,
            completed_passes: 0,
            discarded_passes: 0,
            target: None,
            focus: None,
            target_open: None,
        }
    }
}
pub trait Observer: Send + Sync {
    fn record(&self, receipt: Receipt);
    fn target(&self, ctx: &Context, viewport: ViewportId) -> Option<Id>;
}
struct Inner {
    observer: Arc<dyn Observer>,
    seq: AtomicU64,
    batch: AtomicU64,
}
#[derive(Clone)]
pub struct Bridge(Arc<Inner>);
#[derive(Clone, Copy, Debug, Default)]
pub struct Batch {
    pub id: u64,
    pub first_event: u64,
    pub last_event: u64,
    pub event_count: usize,
}
fn key(viewport: ViewportId) -> Id {
    Id::new(("wireframe-diagnostic-batch", viewport))
}
fn bridge_key(viewport: ViewportId) -> Id {
    Id::new(("wireframe-diagnostic-bridge", viewport))
}
impl Bridge {
    pub fn new(observer: Arc<dyn Observer>) -> Self {
        Self(Arc::new(Inner {
            observer,
            seq: AtomicU64::new(0),
            batch: AtomicU64::new(0),
        }))
    }
    pub fn next_event(&self) -> u64 {
        self.0.seq.fetch_add(1, Ordering::Relaxed) + 1
    }
    pub fn record(&self, receipt: Receipt) {
        self.0.observer.record(receipt);
    }
    pub(crate) fn attach(&self, ctx: &Context, viewport: ViewportId) {
        ctx.data_mut(|d| d.insert_temp(bridge_key(viewport), self.clone()));
    }
    pub fn for_context(ctx: &Context, viewport: ViewportId) -> Option<Self> {
        ctx.data(|d| d.get_temp(bridge_key(viewport)))
    }
    pub fn input(&self, stage: &'static str, viewport: ViewportId, seq: u64, event: &egui::Event) {
        let mut r = Receipt::new(stage, viewport);
        r.seq = seq;
        r.count = 1;
        if let egui::Event::Key {
            key,
            pressed,
            modifiers,
            repeat,
            physical_key,
        } = event
        {
            r.key = Some(*key);
            r.pressed = Some(*pressed);
            r.modifiers = *modifiers;
            r.repeat = Some(*repeat);
            r.physical_key = *physical_key;
        }
        if let egui::Event::AppCommand(command) = event {
            r.command = Some(*command);
        }
        self.record(r);
    }
    pub(crate) fn begin_batch(
        &self,
        ctx: &Context,
        viewport: ViewportId,
        previous: u64,
        last: u64,
        count: usize,
    ) {
        let batch = Batch {
            id: self.0.batch.fetch_add(1, Ordering::Relaxed) + 1,
            first_event: if last > previous { previous + 1 } else { 0 },
            last_event: last,
            event_count: count,
        };
        ctx.data_mut(|d| d.insert_temp(key(viewport), batch));
        let mut r = Receipt::new("input_batch", viewport);
        r.batch = batch.id;
        r.seq = last;
        r.first_event = batch.first_event;
        r.count = count;
        self.record(r);
    }
    /// Called only after Context::run has returned, never from the UI/end-pass callback.
    pub fn output_completed(
        &self,
        ctx: &Context,
        viewport: ViewportId,
        completed: usize,
        discarded: usize,
    ) {
        let mut r = Receipt::new("output_complete", viewport);
        if let Some(batch) = batch(ctx, viewport) {
            r.batch = batch.id;
            r.seq = batch.last_event;
            r.first_event = batch.first_event;
            r.count = batch.event_count;
        }
        r.frame = ctx.cumulative_frame_nr_for(viewport);
        r.pass = ctx.cumulative_pass_nr_for(viewport);
        r.completed_passes = completed;
        r.discarded_passes = discarded;
        r.target = (completed > 0).then(|| self.0.observer.target(ctx, viewport)).flatten();
        // Fresh post-finalization queries: never reuse the earlier app snapshot.
        r.focus = ctx.memory(|m| m.focused());
        r.target_open = r.target.map(|id| egui::ComboBox::is_open(ctx, id));
        self.record(r);
    }
}
pub fn batch(ctx: &Context, viewport: ViewportId) -> Option<Batch> {
    ctx.data(|d| d.get_temp(key(viewport)))
}
#[derive(Default)]
struct Gate {
    started: bool,
    bridge: Option<Bridge>,
}
impl Gate {
    fn install(&mut self, bridge: Bridge) -> Result<(), &'static str> {
        if self.started || self.bridge.is_some() {
            return Err("diagnostics must be installed exactly once before any backend");
        }
        self.bridge = Some(bridge);
        Ok(())
    }
    fn begin(&mut self) -> Option<Bridge> {
        self.started = true;
        self.bridge.clone()
    }
}
static GATE: Mutex<Gate> = Mutex::new(Gate {
    started: false,
    bridge: None,
});
pub fn install(bridge: Bridge) -> Result<(), &'static str> {
    GATE.lock()
        .unwrap_or_else(|e| e.into_inner())
        .install(bridge)
}
pub(crate) fn backend_configuration() -> Option<Bridge> {
    GATE.lock().unwrap_or_else(|e| e.into_inner()).begin()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Sink(Mutex<Vec<Receipt>>, Mutex<Option<Id>>);
    impl Observer for Sink {
        fn record(&self, r: Receipt) {
            self.0.lock().unwrap().push(r);
        }
        fn target(&self, _: &Context, _: ViewportId) -> Option<Id> {
            *self.1.lock().unwrap()
        }
    }
    #[test]
    fn policy_gate_is_immutable_and_instance_scoped() {
        let bridge = Bridge::new(Arc::new(Sink::default()));
        let mut gate = Gate::default();
        assert!(gate.install(bridge.clone()).is_ok());
        assert!(gate.install(bridge.clone()).is_err());
        assert!(gate.begin().is_some());
        let mut normal = Gate::default();
        assert!(normal.begin().is_none());
        assert!(normal.install(bridge).is_err());
    }
    #[test]
    fn output_receipt_reads_finalized_spatial_focus_not_ui_snapshot() {
        let ctx = Context::default();
        let sink = Arc::new(Sink::default());
        let bridge = Bridge::new(sink.clone());
        let mut first = Id::NULL;
        let mut second = Id::NULL;
        ctx.run_ui(Default::default(), |ui| {
            first = ui.button("first").id;
            second = ui.button("second").id;
            ui.memory_mut(|m| m.request_focus(first));
        })
        .drop_without_applying_deltas();
        let mut provisional = None;
        let out = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: Key::ArrowDown,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ui| {
                assert_eq!(ui.button("first").id, first);
                assert_eq!(ui.button("second").id, second);
                provisional = ui.memory(|m| m.focused());
            },
        );
        assert_eq!(provisional, Some(first));
        assert_eq!(ctx.memory(|m| m.focused()), Some(second));
        bridge.output_completed(
            &ctx,
            ViewportId::ROOT,
            out.platform_output.num_completed_passes,
            out.platform_output.request_discard_reasons.len(),
        );
        let rows = sink.0.lock().unwrap();
        assert_eq!(rows.last().unwrap().focus, Some(second));
        assert_ne!(rows.last().unwrap().focus, provisional);
        out.drop_without_applying_deltas();
    }
    #[test]
    fn repeated_pass_receipt_keeps_batch_and_queries_exact_popup_afresh() {
        let ctx = Context::default();
        let sink = Arc::new(Sink::default());
        let target = Id::new("exact-combo");
        *sink.1.lock().unwrap() = Some(target);
        let bridge = Bridge::new(sink.clone());
        bridge.begin_batch(&ctx, ViewportId::ROOT, 2, 7, 3);
        let mut passes = 0;
        let out = ctx.run_ui(Default::default(), |ui| {
            passes += 1;
            egui::Popup::open_id(ui.ctx(),target.with("popup"));
            if passes == 1 { ui.ctx().request_discard("diagnostic ordering fixture"); }
        });
        assert_eq!(passes, 2);
        assert_eq!(out.platform_output.num_completed_passes, 2);
        assert!(egui::ComboBox::is_open(&ctx,target));
        // Change finalized state after the provisional UI snapshot.
        egui::Popup::close_id(&ctx,target.with("popup"));
        bridge.output_completed(&ctx,ViewportId::ROOT,out.platform_output.num_completed_passes,
            out.platform_output.num_completed_passes.saturating_sub(1));
        let rows = sink.0.lock().unwrap();
        let r = rows.last().unwrap();
        assert_eq!(r.batch,1);
        assert_eq!(r.seq,7);
        assert_eq!(r.target,Some(target));
        assert_eq!(r.target_open,Some(false));
        assert_eq!(r.completed_passes,2);
        assert_eq!(r.discarded_passes,1);
        assert_eq!(r.frame,ctx.cumulative_frame_nr_for(ViewportId::ROOT));
        assert_eq!(r.pass,ctx.cumulative_pass_nr_for(ViewportId::ROOT));
        out.drop_without_applying_deltas();
    }

}
