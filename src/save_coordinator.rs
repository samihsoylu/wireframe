// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Scheduling for one named project. The caller owns immutable revision snapshots,
//! local checkpoints and file I/O; this type owns ordering and acknowledgement.
use std::{sync::Arc, time::Duration};

pub const AUTOMATIC_SAVE_DELAY: Duration = Duration::from_secs(5);
pub const SAVE_RETRY_DELAY: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Revision(u64);

/// An issued write must use the exact immutable snapshot identified by revision.
/// Retrying issues a new ticket for the same snapshot; stale completions are rejected.
#[derive(Clone, Debug)]
pub struct WriteTicket {
    owner: Arc<()>,
    id: u64,
    revision: Revision,
    attempt: u8,
}
impl PartialEq for WriteTicket {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
            && self.id == other.id
            && self.revision == other.revision
            && self.attempt == other.attempt
    }
}
impl Eq for WriteTicket {}
impl WriteTicket {
    pub fn revision(&self) -> Revision {
        self.revision
    }
    pub fn is_retry(&self) -> bool {
        self.attempt != 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveFailure {
    pub revision: Revision,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    RevisionExhausted,
    UnknownRevision,
    OutOfOrderCheckpoint,
    UnexpectedCompletion,
    TicketExhausted,
}

#[derive(Clone, Copy, Debug)]
struct Retry {
    revision: Revision,
    due: Duration,
}

/// Construct a new coordinator when opening or switching a named project.
/// Revision zero denotes the initial document already durable in both files.
/// Clock arguments must use one monotonic origin for the coordinator's lifetime.
#[derive(Debug)]
pub struct SaveCoordinator {
    owner: Arc<()>,
    automatic: bool,
    current: Revision,
    local: Revision,
    named: Revision,
    requested: Option<Revision>,
    automatic_due: Option<Duration>,
    active: Option<WriteTicket>,
    retry: Option<Retry>,
    failure: Option<SaveFailure>,
    next_ticket: u64,
}
impl Default for SaveCoordinator {
    fn default() -> Self {
        Self::new(true)
    }
}

impl SaveCoordinator {
    pub fn new(automatic: bool) -> Self {
        Self {
            owner: Arc::new(()),
            automatic,
            current: Revision(0),
            local: Revision(0),
            named: Revision(0),
            requested: None,
            automatic_due: None,
            active: None,
            retry: None,
            failure: None,
            next_ticket: 0,
        }
    }
    pub fn current_revision(&self) -> Revision {
        self.current
    }
    pub fn local_revision(&self) -> Revision {
        self.local
    }
    pub fn named_revision(&self) -> Revision {
        self.named
    }
    pub fn is_clean(&self) -> bool {
        self.named == self.current
    }
    pub fn is_local_current(&self) -> bool {
        self.local == self.current
    }
    pub fn automatic(&self) -> bool {
        self.automatic
    }
    pub fn failure(&self) -> Option<&SaveFailure> {
        self.failure.as_ref()
    }
    pub fn active_write(&self) -> Option<&WriteTicket> {
        self.active.as_ref()
    }
    pub fn requested_revision(&self) -> Option<Revision> {
        self.requested
    }
    /// Call once for each changed document, including Undo/Redo. Selection and
    /// viewport changes do not create document revisions.
    pub fn edited(&mut self) -> Result<Revision, ProtocolError> {
        self.current = Revision(
            self.current
                .0
                .checked_add(1)
                .ok_or(ProtocolError::RevisionExhausted)?,
        );
        Ok(self.current)
    }
    /// Acknowledge only after the corresponding local snapshot is durable.
    /// Coalesced checkpoints may skip revisions, but cannot move backward.
    pub fn local_committed(
        &mut self,
        revision: Revision,
        now: Duration,
    ) -> Result<(), ProtocolError> {
        if revision > self.current {
            return Err(ProtocolError::UnknownRevision);
        }
        if revision < self.local {
            return Err(ProtocolError::OutOfOrderCheckpoint);
        }
        if revision == self.local {
            return Ok(());
        }
        self.local = revision;
        self.failure = None;
        if self.automatic {
            self.automatic_due = Some(now.saturating_add(AUTOMATIC_SAVE_DELAY));
        }
        Ok(())
    }
    /// Capture the revision Save or close needs. The caller must checkpoint it
    /// before a write can start, then wait until named_revision >= this token.
    /// Later edits need their own request if they must be included in a close.
    pub fn request_flush(&mut self) -> Revision {
        let revision = self.current;
        self.requested = (revision > self.named).then_some(revision);
        self.failure = None;
        revision
    }
    pub fn set_automatic(&mut self, enabled: bool, now: Duration) {
        if enabled == self.automatic {
            return;
        }
        self.automatic = enabled;
        self.automatic_due = enabled.then_some(now);
        if enabled {
            self.failure = None;
        }
        // An explicit Save and an already started write/retry survive disabling
        // automatic saving. Disabling only cancels future automatic work.
    }
    /// Earliest useful poll time. An absent deadline can mean a local checkpoint
    /// or active write must finish first. Such completions should trigger polling.
    pub fn next_deadline(&self) -> Option<Duration> {
        if self.active.is_some() {
            return None;
        }
        if self.failure.is_some() {
            return self.automatic_due.filter(|_| self.local > self.named);
        }
        if let Some(retry) = self.retry {
            return Some(retry.due);
        }
        if self.requested.is_some_and(|r| r <= self.local) {
            return Some(Duration::ZERO);
        }
        if self.local > self.named {
            self.automatic_due
        } else {
            None
        }
    }
    /// At most one write is active. The caller must keep its snapshot alive
    /// through completion and the optional retry, even if newer edits arrive.
    pub fn poll(&mut self, now: Duration) -> Result<Option<WriteTicket>, ProtocolError> {
        if self.active.is_some()
            || (self.failure.is_some() && !self.automatic_due.is_some_and(|due| due <= now))
        {
            return Ok(None);
        }
        let (revision, attempt) = if let Some(retry) = self.retry {
            if now < retry.due {
                return Ok(None);
            }
            (retry.revision, 1)
        } else {
            let requested = self.requested.is_some_and(|r| r <= self.local);
            let automatic = self.automatic_due.is_some_and(|due| due <= now);
            if self.local <= self.named || !(requested || automatic) {
                return Ok(None);
            }
            (self.local, 0)
        };
        let id = self
            .next_ticket
            .checked_add(1)
            .ok_or(ProtocolError::TicketExhausted)?;
        let ticket = WriteTicket {
            owner: self.owner.clone(),
            id,
            revision,
            attempt,
        };
        self.next_ticket = id;
        self.active = Some(ticket.clone());
        self.failure = None;
        if attempt == 1 {
            self.retry = None;
        } else {
            self.automatic_due = None;
        }
        Ok(Some(ticket))
    }
    /// Report the result of writing exactly the issued ticket's snapshot.
    pub fn complete(
        &mut self,
        ticket: WriteTicket,
        result: Result<(), String>,
        now: Duration,
    ) -> Result<(), ProtocolError> {
        if self.active.as_ref() != Some(&ticket) {
            return Err(ProtocolError::UnexpectedCompletion);
        }
        self.active = None;
        match result {
            Ok(()) => {
                self.named = ticket.revision;
                if self.requested.is_some_and(|r| r <= self.named) {
                    self.requested = None;
                }
                if self.local == self.named {
                    self.automatic_due = None;
                }
            }
            Err(_) if ticket.attempt == 0 => {
                self.retry = Some(Retry {
                    revision: ticket.revision,
                    due: now.saturating_add(SAVE_RETRY_DELAY),
                });
            }
            Err(message) => {
                // A later edit's future deadline remains a real trigger. An
                // already elapsed deadline must not create an endless retry loop.
                if self.automatic_due.is_some_and(|due| due <= now) {
                    self.automatic_due = None;
                }
                self.failure = Some(SaveFailure {
                    revision: ticket.revision,
                    message,
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
