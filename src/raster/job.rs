// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! One background export with bounded progress state and explicit conflict replies.
use super::{
    Geometry, archive,
    names::{FileName, Format},
    plan::{Prepared, Request},
    publication::{Attempt, PendingOutput, SavedOutput},
};
use crate::workspace::ExportSnapshot;
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};
const CANCELLED: &str = "Export cancelled";
#[derive(Clone, Debug)]
pub enum Destination {
    /// One page produces PNG; multiple pages produce a ZIP.
    File(PathBuf),
    Folder(PathBuf),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Replace,
    Skip,
    KeepBoth,
    Cancel,
}
#[derive(Clone, Copy, Debug)]
pub struct Decision {
    pub choice: Choice,
    pub apply_to_all: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Running,
    Completed,
    Cancelled,
    Failed(String),
}
#[derive(Clone, Debug)]
pub struct ReducedPage {
    pub name: String,
    pub width: u32,
    pub height: u32,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub total_pages: usize,
    pub exported_pages: usize,
    pub skipped_pages: usize,
    pub files: Vec<SavedOutput>,
    pub reduced_pages: Vec<ReducedPage>,
    pub outcome: Outcome,
}
impl Default for Report {
    fn default() -> Self {
        Self {
            total_pages: 0,
            exported_pages: 0,
            skipped_pages: 0,
            files: vec![],
            reduced_pages: vec![],
            outcome: Outcome::Running,
        }
    }
}
impl Report {
    pub fn remaining_pages(&self) -> usize {
        self.total_pages
            .saturating_sub(self.exported_pages + self.skipped_pages)
    }
}
#[derive(Clone, Debug)]
pub enum Phase {
    Preparing,
    Rendering {
        page: usize,
        name: String,
        rows: u32,
        total_rows: u32,
    },
    Publishing(PathBuf),
    Conflict {
        id: u64,
        path: PathBuf,
    },
    Finished,
}
#[derive(Clone, Debug)]
pub struct Status {
    pub phase: Phase,
    pub total_pages: usize,
    pub exported_pages: usize,
    pub skipped_pages: usize,
    pub output_files: usize,
    pub cancel_requested: bool,
}
struct State {
    phase: Phase,
    report: Report,
    reply: Option<Decision>,
    next_conflict: u64,
}
struct Shared {
    state: Mutex<State>,
    changed: Condvar,
    cancel: AtomicBool,
}
impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
    fn check(&self) -> Result<(), String> {
        if self.cancel.load(Ordering::Acquire) {
            Err(CANCELLED.into())
        } else {
            Ok(())
        }
    }
    fn phase(&self, phase: Phase) {
        self.lock().phase = phase;
    }
    fn cancel(&self) {
        // Coordinate with the condition-variable predicate to avoid a lost wake.
        let _guard = self.lock();
        self.cancel.store(true, Ordering::Release);
        self.changed.notify_all();
    }
    fn conflict(&self, path: PathBuf) -> Result<Decision, String> {
        let mut state = self.lock();
        state.next_conflict += 1;
        let id = state.next_conflict;
        state.reply = None;
        state.phase = Phase::Conflict {
            id,
            path: path.clone(),
        };
        loop {
            self.check()?;
            if let Some(reply) = state.reply.take() {
                state.phase = Phase::Publishing(path);
                return Ok(reply);
            }
            state = self.changed.wait(state).unwrap_or_else(|e| e.into_inner());
        }
    }
}
pub struct ExportJob {
    shared: Arc<Shared>,
    handle: Option<JoinHandle<()>>,
}
impl ExportJob {
    pub fn start(
        snapshot: ExportSnapshot,
        request: Request,
        destination: Destination,
    ) -> Result<Self, String> {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                phase: Phase::Preparing,
                report: Report::default(),
                reply: None,
                next_conflict: 0,
            }),
            changed: Condvar::new(),
            cancel: AtomicBool::new(false),
        });
        let worker = shared.clone();
        let handle = std::thread::Builder::new()
            .name("image-export".into())
            .spawn(move || {
                let result = run(&worker, snapshot, request, destination);
                let outcome = match result {
                    Ok(()) => Outcome::Completed,
                    Err(error) if error == CANCELLED && worker.cancel.load(Ordering::Acquire) => {
                        Outcome::Cancelled
                    }
                    Err(error) => Outcome::Failed(error),
                };
                let mut state = worker.lock();
                state.report.outcome = outcome;
                state.phase = Phase::Finished;
            })
            .map_err(|e| format!("Cannot start image export: {e}"))?;
        Ok(Self {
            shared,
            handle: Some(handle),
        })
    }
    pub fn status(&self) -> Status {
        let state = self.shared.lock();
        Status {
            phase: state.phase.clone(),
            total_pages: state.report.total_pages,
            exported_pages: state.report.exported_pages,
            skipped_pages: state.report.skipped_pages,
            output_files: state.report.files.len(),
            cancel_requested: self.shared.cancel.load(Ordering::Acquire),
        }
    }
    pub fn cancel(&self) {
        self.shared.cancel();
    }
    /// Stale or duplicate replies cannot authorize a later destination.
    pub fn decide(&self, id: u64, decision: Decision) -> bool {
        let mut state = self.shared.lock();
        if self.shared.cancel.load(Ordering::Acquire)
            || state.reply.is_some()
            || !matches!(state.phase,Phase::Conflict {id:current,..} if current==id)
        {
            return false;
        }
        state.reply = Some(decision);
        self.shared.changed.notify_all();
        true
    }
    pub fn is_finished(&self) -> bool {
        self.handle.as_ref().is_none_or(JoinHandle::is_finished)
    }
    pub fn finish(mut self) -> Report {
        if self
            .handle
            .take()
            .is_some_and(|handle| handle.join().is_err())
        {
            let mut state = self.shared.lock();
            state.report.outcome = Outcome::Failed("Image export stopped unexpectedly".into());
            state.phase = Phase::Finished;
        }
        self.shared.lock().report.clone()
    }
}
impl Drop for ExportJob {
    fn drop(&mut self) {
        if self.handle.is_some() {
            self.shared.cancel();
        }
    }
}
fn rendering(
    shared: &Shared,
    plan: &Prepared,
    index: usize,
    rows: u32,
    total_rows: u32,
) -> Result<(), String> {
    shared.check()?;
    shared.phase(Phase::Rendering {
        page: index,
        name: plan.entries()[index].display_name(),
        rows,
        total_rows,
    });
    Ok(())
}
fn run(
    shared: &Shared,
    snapshot: ExportSnapshot,
    request: Request,
    destination: Destination,
) -> Result<(), String> {
    shared.check()?;
    let plan = Prepared::from_snapshot(snapshot, &request)?;
    shared.lock().report.total_pages = plan.entries().len();
    shared.check()?;
    let mut policy = None;
    let mut reserved: HashSet<_> = plan.file_names().iter().map(FileName::key).collect();
    match destination {
        Destination::File(path) if plan.entries().len() > 1 => {
            let base = FileName::new(
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or("Choose a Unicode archive filename")?,
                Format::Zip,
            );
            reserved.insert(base.key());
            let mut geometry = vec![];
            let pending = PendingOutput::prepare(path, |file| {
                geometry = archive::write_zip(&plan, file, |index, row, total| {
                    rendering(shared, &plan, index, row, total)
                })?;
                Ok(())
            })?;
            publish(
                shared,
                &plan,
                pending,
                &base,
                &reserved,
                &mut policy,
                &geometry.into_iter().enumerate().collect::<Vec<_>>(),
            )?;
        }
        Destination::File(path) => {
            let base = FileName::new(
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or("Choose a Unicode image filename")?,
                Format::Png,
            );
            reserved.insert(base.key());
            let (pending, geometry) = encode_page(shared, &plan, 0, path)?;
            publish(
                shared,
                &plan,
                pending,
                &base,
                &reserved,
                &mut policy,
                &[(0, geometry)],
            )?;
        }
        Destination::Folder(directory) => {
            if !directory.is_dir() {
                return Err("Choose an existing export folder".into());
            }
            for index in 0..plan.entries().len() {
                let base = &plan.file_names()[index];
                let (pending, geometry) =
                    encode_page(shared, &plan, index, directory.join(base.text()))?;
                publish(
                    shared,
                    &plan,
                    pending,
                    base,
                    &reserved,
                    &mut policy,
                    &[(index, geometry)],
                )?;
            }
        }
    }
    Ok(())
}
fn encode_page(
    shared: &Shared,
    plan: &Prepared,
    index: usize,
    path: PathBuf,
) -> Result<(PendingOutput, Geometry), String> {
    rendering(shared, plan, index, 0, 0)?;
    let scene = plan.scene(index)?;
    let geometry = Geometry::new(scene.bounds, plan.options())?;
    let pending = PendingOutput::prepare(path, |file| {
        super::write_png(&scene, plan.options(), file, |row, total| {
            rendering(shared, plan, index, row, total)
        })
        .map(|_| ())
    })?;
    Ok((pending, geometry))
}
fn publish(
    shared: &Shared,
    plan: &Prepared,
    pending: PendingOutput,
    base: &FileName,
    reserved: &HashSet<String>,
    policy: &mut Option<Choice>,
    geometry: &[(usize, Geometry)],
) -> Result<(), String> {
    shared.check()?;
    shared.phase(Phase::Publishing(pending.target().to_owned()));
    let output = match pending.publish_new()? {
        Attempt::Saved(output) => Some(output),
        Attempt::Conflict(pending) => {
            let choice = if let Some(choice) = policy {
                *choice
            } else {
                let decision = shared.conflict(pending.target().to_owned())?;
                if decision.apply_to_all {
                    *policy = Some(decision.choice);
                }
                decision.choice
            };
            shared.check()?;
            shared.phase(Phase::Publishing(pending.target().to_owned()));
            match choice {
                Choice::Replace => Some(pending.replace()?),
                Choice::KeepBoth => Some(pending.keep_both(base, reserved, || shared.check())?),
                Choice::Skip => {
                    pending.discard()?;
                    None
                }
                Choice::Cancel => {
                    shared.cancel();
                    return Err(CANCELLED.into());
                }
            }
        }
    };
    let mut state = shared.lock();
    if let Some(output) = output {
        state.report.files.push(output);
        state.report.exported_pages += geometry.len();
        for &(index, size) in geometry {
            if size.reduced {
                state.report.reduced_pages.push(ReducedPage {
                    name: plan.entries()[index].display_name(),
                    width: size.width,
                    height: size.height,
                });
            }
        }
    } else {
        state.report.skipped_pages += geometry.len();
    }
    Ok(())
}
