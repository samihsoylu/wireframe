// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! A single I/O owner per project. UI callers submit immutable snapshots and
//! consume ordered acknowledgements; filesystem work and timers stay off-thread.
use crate::{
    document::Project,
    save_coordinator::{Revision, SaveCoordinator},
    working_copy::{WorkingCopies, WorkingCopy},
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};
const LOCAL_DELAY: Duration = Duration::from_secs(1);
const LOCAL_RETRY: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestId(u64);
#[derive(Clone, Debug)]
pub struct SaveReceipt {
    pub snapshot: Arc<Project>,
    pub path: PathBuf,
    pub copy_id: String,
    pub preserved: Option<PathBuf>,
    pub warning: Option<String>,
}
#[derive(Debug)]
pub enum SessionEvent {
    Checkpointed {
        snapshot: Arc<Project>,
        copy_id: String,
    },
    Saved {
        snapshot: Arc<Project>,
        path: PathBuf,
    },
    RequestFinished {
        request: RequestId,
        result: Result<SaveReceipt, String>,
    },
    Problem(String),
}
type PendingUpdate = Arc<Mutex<Option<Arc<Project>>>>;

enum Command {
    Update(PendingUpdate),
    Flush(RequestId),
    SaveAs(RequestId, PathBuf),
    Automatic(bool),
    Discard,
}
pub struct ProjectSession {
    commands: Option<mpsc::Sender<Command>>,
    events: mpsc::Receiver<SessionEvent>,
    worker: Option<thread::JoinHandle<Result<(), String>>>,
    observed: Arc<Project>,
    pending_update: Option<PendingUpdate>,
    next_request: u64,
    automatic: bool,
}
impl ProjectSession {
    pub fn start(
        copies: WorkingCopies,
        copy: WorkingCopy,
        automatic: bool,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Self, String> {
        let observed = Arc::new(copy.project().clone());
        let (commands, receiver) = mpsc::channel();
        let (events, event_receiver) = mpsc::channel();
        let actor = Actor::new(copies, copy, automatic, events, wake)?;
        let worker = thread::Builder::new()
            .name("project-save".into())
            .spawn(move || actor.run(receiver))
            .map_err(|e| e.to_string())?;
        Ok(Self {
            commands: Some(commands),
            events: event_receiver,
            worker: Some(worker),
            observed,
            pending_update: None,
            next_request: 0,
            automatic,
        })
    }
    fn send(&self, command: Command) -> Result<(), String> {
        self.commands
            .as_ref()
            .ok_or("Save worker has closed")?
            .send(command)
            .map_err(|_| "Save worker stopped; keep your current document open".into())
    }
    pub fn observe(&mut self, project: &Project) -> Result<(), String> {
        if self.observed.as_ref() != project {
            let snapshot = Arc::new(project.clone());
            let mut coalesced = false;
            if let Some(pending) = &self.pending_update {
                let mut slot = pending.lock().map_err(|_| "Save update queue failed")?;
                if slot.is_some() {
                    *slot = Some(snapshot.clone());
                    coalesced = true;
                }
            }
            if !coalesced {
                let pending = Arc::new(Mutex::new(Some(snapshot.clone())));
                self.send(Command::Update(pending.clone()))?;
                self.pending_update = Some(pending);
            }
            self.observed = snapshot;
        }
        Ok(())
    }
    fn request(&mut self) -> Result<RequestId, String> {
        self.next_request = self
            .next_request
            .checked_add(1)
            .ok_or("Save request sequence exhausted")?;
        Ok(RequestId(self.next_request))
    }
    pub fn flush(&mut self, project: &Project) -> Result<RequestId, String> {
        self.observe(project)?;
        // Later edits must use a new mailbox, after this explicit request.
        self.pending_update = None;
        let id = self.request()?;
        self.send(Command::Flush(id))?;
        Ok(id)
    }
    /// The caller has already obtained the user's Save As destination selection.
    /// This is the only operation that can replace an unrelated selected file.
    pub fn save_as(&mut self, path: PathBuf, project: &Project) -> Result<RequestId, String> {
        self.observe(project)?;
        // Later edits must use a new mailbox, after this explicit request.
        self.pending_update = None;
        let id = self.request()?;
        self.send(Command::SaveAs(id, path))?;
        Ok(id)
    }
    pub fn automatic(&self) -> bool {
        self.automatic
    }
    pub fn set_automatic(&mut self, enabled: bool) -> Result<(), String> {
        self.pending_update = None;
        self.send(Command::Automatic(enabled))?;
        self.automatic = enabled;
        Ok(())
    }
    /// Events are ordered. Apply Saved.snapshot as the saved baseline, never the
    /// UI's current document: the user may have edited after that snapshot.
    pub fn try_event(&self) -> Result<Option<SessionEvent>, String> {
        match self.events.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => {
                Err("Save worker stopped; keep your current document open".into())
            }
        }
    }
    /// Drains queued updates and checkpoints the latest local document before
    /// releasing ownership. Automatic close should await flush first. Manual
    /// application quit can finish locally without requesting a named save.
    pub fn finish(mut self) -> Result<(), String> {
        self.join()
    }
    /// Explicit discard removes the selected local record after queued work.
    /// On failure the retained record can be resumed; the UI must keep its data.
    pub fn discard(mut self) -> Result<(), String> {
        self.send(Command::Discard)?;
        self.join()
    }
    fn join(&mut self) -> Result<(), String> {
        self.commands.take();
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| "Save worker failed while closing".to_owned())?
        } else {
            Ok(())
        }
    }
}
impl Drop for ProjectSession {
    fn drop(&mut self) {
        let _ = self.join();
    }
}
struct Actor {
    copies: WorkingCopies,
    copy: Option<WorkingCopy>,
    coordinator: SaveCoordinator,
    automatic: bool,
    latest: Arc<Project>,
    local: Arc<Project>,
    named: Option<Arc<Project>>,
    retained: Option<(Revision, Arc<Project>)>,
    checkpoint_due: Option<Duration>,
    requests: Vec<(RequestId, Revision)>,
    events: mpsc::Sender<SessionEvent>,
    wake: Arc<dyn Fn() + Send + Sync>,
    origin: Instant,
    last_problem: Option<String>,
}
impl Actor {
    fn new(
        copies: WorkingCopies,
        copy: WorkingCopy,
        automatic: bool,
        events: mpsc::Sender<SessionEvent>,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Self, String> {
        let has_name = copy.named_path().map_err(|e| e.to_string())?.is_some();
        let dirty = copy.has_local_changes().map_err(|e| e.to_string())?;
        let latest = Arc::new(copy.project().clone());
        let mut coordinator = SaveCoordinator::new(automatic && has_name);
        if dirty {
            let revision = coordinator.edited().map_err(|e| format!("{e:?}"))?;
            coordinator
                .local_committed(revision, Duration::ZERO)
                .map_err(|e| format!("{e:?}"))?;
        }
        Ok(Self {
            copies,
            copy: Some(copy),
            coordinator,
            automatic,
            local: latest.clone(),
            named: (!dirty).then(|| latest.clone()),
            latest,
            retained: None,
            checkpoint_due: None,
            requests: vec![],
            events,
            wake,
            origin: Instant::now(),
            last_problem: None,
        })
    }
    fn emit(&self, event: SessionEvent) {
        let _ = self.events.send(event);
        (self.wake)();
    }
    fn problem(&mut self, message: String) {
        if self.last_problem.as_ref() != Some(&message) {
            self.emit(SessionEvent::Problem(message.clone()));
            self.last_problem = Some(message);
        }
    }
    fn update(&mut self, snapshot: Arc<Project>) -> Result<(), String> {
        if self.latest != snapshot {
            self.coordinator.edited().map_err(|e| format!("{e:?}"))?;
            self.latest = snapshot;
            self.checkpoint_due = Some(self.origin.elapsed() + LOCAL_DELAY);
        }
        Ok(())
    }
    fn checkpoint(&mut self) -> Result<(), String> {
        if self.coordinator.is_local_current() {
            return Ok(());
        }
        self.copy
            .as_mut()
            .ok_or("Project closed")?
            .checkpoint(&self.latest)
            .map_err(|e| e.to_string())?;
        self.coordinator
            .local_committed(self.coordinator.current_revision(), self.origin.elapsed())
            .map_err(|e| format!("{e:?}"))?;
        self.local = self.latest.clone();
        self.checkpoint_due = None;
        self.emit(SessionEvent::Checkpointed {
            snapshot: self.local.clone(),
            copy_id: self.copy.as_ref().unwrap().id().into(),
        });
        Ok(())
    }
    fn receipt(&self, snapshot: Arc<Project>) -> Result<SaveReceipt, String> {
        let copy = self.copy.as_ref().ok_or("Project closed")?;
        Ok(SaveReceipt {
            snapshot,
            path: copy
                .named_path()
                .map_err(|e| e.to_string())?
                .ok_or("Choose a saved-file location first")?,
            copy_id: copy.id().into(),
            preserved: copy.preserved_copy().map(|p| p.to_owned()),
            warning: None,
        })
    }
    fn fail_requests(&mut self, error: &str) {
        for (request, _) in std::mem::take(&mut self.requests) {
            self.emit(SessionEvent::RequestFinished {
                request,
                result: Err(error.into()),
            });
        }
        self.coordinator.cancel_flush_request();
    }
    fn complete_requests(&mut self) {
        let requests = std::mem::take(&mut self.requests);
        for (request, target) in requests {
            if self.coordinator.named_revision() >= target {
                let result = self
                    .named
                    .clone()
                    .ok_or("No named snapshot acknowledged".into())
                    .and_then(|p| self.receipt(p));
                self.emit(SessionEvent::RequestFinished { request, result });
            } else {
                self.requests.push((request, target));
            }
        }
    }
    fn flush(&mut self, request: RequestId) {
        let result = (|| {
            if self
                .copy
                .as_ref()
                .unwrap()
                .named_path()
                .map_err(|e| e.to_string())?
                .is_none()
            {
                return Err("Choose a saved-file location first".into());
            }
            self.checkpoint()?;
            // A clean in-memory revision does not prove the file still exists
            // or still belongs to this session's acknowledged baseline.
            if self.coordinator.is_clean() {
                self.copy
                    .as_mut()
                    .unwrap()
                    .save_named(&self.latest)
                    .map_err(|e| e.to_string())?;
            }
            let target = self.coordinator.request_flush();
            self.requests.push((request, target));
            self.complete_requests();
            Ok(())
        })();
        if let Err(error) = result {
            self.emit(SessionEvent::RequestFinished {
                request,
                result: Err(error),
            });
        }
    }
    fn save_as(&mut self, request: RequestId, path: PathBuf) {
        if !self.requests.is_empty() {
            self.emit(SessionEvent::RequestFinished {
                request,
                result: Err("Wait for the current save to finish".into()),
            });
            return;
        }
        let result = (|| {
            self.checkpoint()?;
            let snapshot = self.latest.clone();
            let same = self
                .copy
                .as_ref()
                .unwrap()
                .owns_destination(&path)
                .map_err(|e| e.to_string())?;
            let warning = if same {
                let destination = self.copy.as_mut().unwrap();
                destination
                    .prepare_current_save_as()
                    .map_err(|e| e.to_string())?;
                if destination.save_named(&snapshot).is_err() {
                    thread::sleep(Duration::from_secs(1));
                    destination
                        .save_named(&snapshot)
                        .map_err(|e| e.to_string())?;
                }
                None
            } else {
                let mut destination = self
                    .copies
                    .prepare_save_as(&path, &snapshot)
                    .map_err(|e| e.to_string())?;
                if destination.save_named(&snapshot).is_err() {
                    thread::sleep(Duration::from_secs(1));
                    destination.save_named(&snapshot).map_err(|e| {
                        format!(
                            "{e}. A local copy is retained at {}",
                            destination.local_path().display()
                        )
                    })?;
                }
                // Complete the automatic save boundary before releasing the
                // original. A failed original must not prevent recovery to
                // the successfully written destination or authorize an
                // overwrite of outside changes.
                let mut warnings = Vec::new();
                let source = self.copy.as_mut().unwrap();
                if self.automatic {
                    let result = source.named_path().and_then(|path| {
                        if path.is_some() {
                            source.save_named(&snapshot)
                        } else {
                            Ok(())
                        }
                    });
                    if let Err(error) = result {
                        warnings.push(format!(
                            "The new file is saved. The original file could not be updated: {error}"
                        ));
                    }
                }
                let source = self.copy.replace(destination).unwrap();
                if let Err(error) = source.discard() {
                    warnings.push(format!(
                            "The new file is saved. The previous local copy could not be removed: {error}"
                        ));
                }
                (!warnings.is_empty()).then(|| warnings.join("\n"))
            };
            self.coordinator = SaveCoordinator::new(self.automatic);
            self.retained = None;
            self.checkpoint_due = None;
            self.named = Some(snapshot.clone());
            self.local = snapshot.clone();
            self.last_problem = None;
            let mut receipt = self.receipt(snapshot.clone())?;
            receipt.warning = warning;
            self.emit(SessionEvent::Saved {
                snapshot,
                path: receipt.path.clone(),
            });
            Ok(receipt)
        })();
        self.emit(SessionEvent::RequestFinished { request, result });
    }
    fn service(&mut self) -> Result<(), String> {
        if self
            .checkpoint_due
            .is_some_and(|due| due <= self.origin.elapsed())
            && let Err(error) = self.checkpoint()
        {
            self.checkpoint_due = Some(self.origin.elapsed() + LOCAL_RETRY);
            self.problem(error.clone());
            self.fail_requests(&error);
        }
        if let Some(ticket) = self
            .coordinator
            .poll(self.origin.elapsed())
            .map_err(|e| format!("{e:?}"))?
        {
            let snapshot = if ticket.is_retry() {
                self.retained
                    .as_ref()
                    .filter(|(revision, _)| *revision == ticket.revision())
                    .map(|(_, p)| p.clone())
                    .ok_or("Retry snapshot is unavailable")?
            } else {
                self.local.clone()
            };
            let result = self
                .copy
                .as_mut()
                .unwrap()
                .save_named(&snapshot)
                .map_err(|e| e.to_string());
            let succeeded = result.is_ok();
            self.coordinator
                .complete(ticket.clone(), result, self.origin.elapsed())
                .map_err(|e| format!("{e:?}"))?;
            if succeeded {
                self.retained = None;
                self.named = Some(snapshot.clone());
                self.last_problem = None;
                self.emit(SessionEvent::Saved {
                    snapshot,
                    path: self
                        .copy
                        .as_ref()
                        .unwrap()
                        .named_path()
                        .map_err(|e| e.to_string())?
                        .ok_or("Named write has no location")?,
                });
                self.complete_requests();
            } else if let Some(failure) = self.coordinator.failure() {
                let message = failure.message.clone();
                self.retained = None;
                self.problem(message.clone());
                self.fail_requests(&message);
            } else {
                self.retained = Some((ticket.revision(), snapshot));
            }
        }
        Ok(())
    }
    fn run_loop(&mut self, commands: mpsc::Receiver<Command>) -> Result<(), String> {
        self.emit(SessionEvent::Checkpointed {
            snapshot: self.local.clone(),
            copy_id: self.copy.as_ref().unwrap().id().into(),
        });
        loop {
            let due = [self.checkpoint_due, self.coordinator.next_deadline()]
                .into_iter()
                .flatten()
                .min();
            let incoming = match due {
                Some(due) => commands.recv_timeout(due.saturating_sub(self.origin.elapsed())),
                None => commands
                    .recv()
                    .map_err(|_| mpsc::RecvTimeoutError::Disconnected),
            };
            match incoming {
                Ok(Command::Update(pending)) => {
                    let snapshot = pending
                        .lock()
                        .map_err(|_| "Save update queue failed")?
                        .take();
                    if let Some(snapshot) = snapshot {
                        self.update(snapshot)?;
                    }
                }
                Ok(Command::Flush(id)) => self.flush(id),
                Ok(Command::SaveAs(id, path)) => self.save_as(id, path),
                Ok(Command::Automatic(enabled)) => {
                    self.automatic = enabled;
                    if enabled && let Err(error) = self.checkpoint() {
                        self.problem(error);
                        self.checkpoint_due = Some(self.origin.elapsed() + LOCAL_RETRY);
                    }
                    let named = self
                        .copy
                        .as_ref()
                        .unwrap()
                        .named_path()
                        .map_err(|e| e.to_string())?
                        .is_some();
                    self.coordinator
                        .set_automatic(enabled && named, self.origin.elapsed());
                }
                Ok(Command::Discard) => {
                    let copy = self.copy.take().unwrap();
                    return copy.discard().map_err(|e| e.to_string());
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            self.service()?;
        }
    }
    fn run(mut self, commands: mpsc::Receiver<Command>) -> Result<(), String> {
        let result = self.run_loop(commands);
        if self.copy.is_some() {
            self.checkpoint()?;
        }
        result
    }
}

#[cfg(test)]
mod tests;
