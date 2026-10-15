// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

fn sec(n: u64) -> Duration {
    Duration::from_secs(n)
}
fn edit(c: &mut SaveCoordinator, at: u64) -> Revision {
    let r = c.edited().unwrap();
    c.local_committed(r, sec(at)).unwrap();
    r
}
fn write(c: &mut SaveCoordinator, at: u64) -> WriteTicket {
    c.poll(sec(at)).unwrap().expect("write must be due")
}
fn finish(c: &mut SaveCoordinator, ticket: WriteTicket, at: u64) {
    c.complete(ticket, Ok(()), sec(at)).unwrap();
}

#[test]
fn debounce_starts_at_local_durability_and_duplicate_ack_does_not_reset_it() {
    let mut c = SaveCoordinator::new(true);
    let r = c.edited().unwrap();
    assert!(!c.is_local_current());
    assert!(c.poll(sec(30)).unwrap().is_none());
    c.local_committed(r, sec(30)).unwrap();
    c.local_committed(r, sec(34)).unwrap();
    assert_eq!(c.next_deadline(), Some(sec(35)));
    assert!(c.poll(sec(34)).unwrap().is_none());
    let t = write(&mut c, 35);
    assert_eq!(t.revision(), r);
    assert!(!c.is_clean());
    finish(&mut c, t, 36);
    assert!(c.is_clean());
    assert_eq!(c.next_deadline(), None);
}

#[test]
fn newer_local_checkpoint_restarts_delay_and_skips_intermediate_snapshots() {
    let mut c = SaveCoordinator::new(true);
    edit(&mut c, 0);
    let latest = edit(&mut c, 4);
    assert!(c.poll(sec(5)).unwrap().is_none());
    assert_eq!(c.next_deadline(), Some(sec(9)));
    let t = write(&mut c, 9);
    assert_eq!(t.revision(), latest);
    finish(&mut c, t, 9);
    assert!(c.is_clean());
}

#[test]
fn older_completion_keeps_newer_edits_dirty_until_their_own_deadline() {
    let mut c = SaveCoordinator::new(true);
    let first = edit(&mut c, 0);
    let t = write(&mut c, 5);
    let latest = edit(&mut c, 6);
    finish(&mut c, t, 7);
    assert_eq!(c.named_revision(), first);
    assert!(!c.is_clean());
    assert!(c.poll(sec(10)).unwrap().is_none());
    let t = write(&mut c, 11);
    assert_eq!(t.revision(), latest);
    finish(&mut c, t, 11);
    assert!(c.is_clean());
}

#[test]
fn slow_write_is_serial_and_overdue_changes_follow_immediately() {
    let mut c = SaveCoordinator::new(true);
    edit(&mut c, 0);
    let first = write(&mut c, 5);
    let latest = edit(&mut c, 6);
    assert!(c.poll(sec(20)).unwrap().is_none());
    assert_eq!(c.next_deadline(), None);
    finish(&mut c, first, 20);
    let second = write(&mut c, 20);
    assert_eq!(second.revision(), latest);
    finish(&mut c, second, 20);
    assert!(c.is_clean());
}

#[test]
fn explicit_flush_waits_for_local_checkpoint_and_requested_revision() {
    let mut c = SaveCoordinator::new(false);
    let first = edit(&mut c, 0);
    c.request_flush();
    let t = write(&mut c, 0);
    let second = c.edited().unwrap();
    assert_eq!(c.request_flush(), second);
    finish(&mut c, t, 1);
    assert_eq!(c.named_revision(), first);
    assert_eq!(c.requested_revision(), Some(second));
    assert!(c.poll(sec(100)).unwrap().is_none());
    c.local_committed(second, sec(100)).unwrap();
    let t = write(&mut c, 100);
    finish(&mut c, t, 101);
    assert_eq!(c.named_revision(), second);
    assert_eq!(c.requested_revision(), None);
}

#[test]
fn later_edit_does_not_retroactively_expand_an_explicit_request() {
    let mut c = SaveCoordinator::new(false);
    let first = edit(&mut c, 0);
    assert_eq!(c.request_flush(), first);
    let t = write(&mut c, 0);
    edit(&mut c, 1);
    finish(&mut c, t, 2);
    assert_eq!(c.requested_revision(), None);
    assert_eq!(c.named_revision(), first);
    assert!(!c.is_clean());
    assert!(c.poll(sec(100)).unwrap().is_none());
    c.request_flush();
    let t = write(&mut c, 100);
    finish(&mut c, t, 100);
    assert!(c.is_clean());
}

#[test]
fn retry_uses_failed_snapshot_and_preserves_later_flush_target() {
    let mut c = SaveCoordinator::new(false);
    let first = edit(&mut c, 0);
    c.request_flush();
    let t = write(&mut c, 0);
    c.complete(t.clone(), Err("temporarily unavailable".into()), sec(1))
        .unwrap();
    let latest = edit(&mut c, 1);
    c.request_flush();
    assert!(c.poll(sec(1)).unwrap().is_none());
    assert_eq!(c.next_deadline(), Some(sec(2)));
    let retry = write(&mut c, 2);
    assert!(retry.is_retry());
    assert_eq!(retry.revision(), first);
    assert_ne!(retry, t);
    finish(&mut c, retry, 2);
    assert_eq!(c.requested_revision(), Some(latest));
    let followup = write(&mut c, 2);
    assert_eq!(followup.revision(), latest);
    finish(&mut c, followup, 2);
    assert!(c.is_clean());
}

#[test]
fn two_failures_stop_until_an_explicit_trigger_and_do_not_claim_success() {
    let mut c = SaveCoordinator::new(true);
    let r = edit(&mut c, 0);
    c.request_flush();
    let t = write(&mut c, 0);
    c.complete(t, Err("first".into()), sec(0)).unwrap();
    let t = write(&mut c, 1);
    c.complete(t, Err("second".into()), sec(1)).unwrap();
    assert_eq!(
        c.failure(),
        Some(&SaveFailure {
            revision: r,
            message: "second".into()
        })
    );
    assert_eq!(c.requested_revision(), Some(r));
    assert!(!c.is_clean());
    assert!(c.poll(sec(100)).unwrap().is_none());
    c.local_committed(r, sec(101)).unwrap();
    assert!(c.failure().is_some());
    c.request_flush();
    assert!(c.failure().is_none());
    let t = write(&mut c, 101);
    assert!(!t.is_retry());
    finish(&mut c, t, 101);
    assert!(c.is_clean());
}

#[test]
fn next_edit_rearms_automatic_saving_after_failure_without_a_retry_loop() {
    let mut c = SaveCoordinator::new(true);
    edit(&mut c, 0);
    let t = write(&mut c, 5);
    c.complete(t, Err("one".into()), sec(5)).unwrap();
    let t = write(&mut c, 6);
    c.complete(t, Err("two".into()), sec(6)).unwrap();
    edit(&mut c, 20);
    assert!(c.failure().is_none());
    assert!(c.poll(sec(24)).unwrap().is_none());
    let t = write(&mut c, 25);
    finish(&mut c, t, 25);
    assert!(c.is_clean());
}

#[test]
fn disabling_cancels_automatic_timer_but_keeps_active_retry_and_explicit_save() {
    let mut c = SaveCoordinator::new(true);
    edit(&mut c, 0);
    c.set_automatic(false, sec(1));
    assert!(!c.automatic());
    assert_eq!(c.next_deadline(), None);
    assert!(c.poll(sec(20)).unwrap().is_none());
    c.set_automatic(true, sec(20));
    let t = write(&mut c, 20);
    c.set_automatic(false, sec(20));
    c.complete(t, Err("retry".into()), sec(20)).unwrap();
    let t = write(&mut c, 21);
    finish(&mut c, t, 21);
    edit(&mut c, 22);
    assert!(c.poll(sec(100)).unwrap().is_none());
    c.request_flush();
    let t = write(&mut c, 100);
    finish(&mut c, t, 100);
    assert!(c.is_clean());
}

#[test]
fn duplicate_and_foreign_completions_cannot_acknowledge_another_write() {
    let mut a = SaveCoordinator::new(true);
    let mut b = SaveCoordinator::new(true);
    edit(&mut a, 0);
    edit(&mut b, 0);
    let ta = write(&mut a, 5);
    let tb = write(&mut b, 5);
    assert_eq!(
        a.complete(tb.clone(), Ok(()), sec(6)),
        Err(ProtocolError::UnexpectedCompletion)
    );
    assert_eq!(a.active_write(), Some(&ta));
    finish(&mut a, ta.clone(), 6);
    edit(&mut a, 6);
    let next = write(&mut a, 11);
    assert_eq!(
        a.complete(ta, Ok(()), sec(12)),
        Err(ProtocolError::UnexpectedCompletion)
    );
    assert_eq!(a.active_write(), Some(&next));
    assert!(!a.is_clean());
    finish(&mut a, next, 12);
    finish(&mut b, tb, 12);
    assert!(a.is_clean() && b.is_clean());
}

#[test]
fn invalid_checkpoint_order_is_rejected_without_rewinding_durability() {
    let mut c = SaveCoordinator::new(true);
    let first = c.edited().unwrap();
    let second = c.edited().unwrap();
    c.local_committed(second, sec(1)).unwrap();
    assert_eq!(
        c.local_committed(first, sec(2)),
        Err(ProtocolError::OutOfOrderCheckpoint)
    );
    assert_eq!(
        c.local_committed(Revision(3), sec(2)),
        Err(ProtocolError::UnknownRevision)
    );
    assert_eq!(c.local_revision(), second);
    assert_eq!(c.next_deadline(), Some(sec(6)));
}

#[test]
fn clock_and_sequence_limits_do_not_wrap_into_a_false_acknowledgement() {
    let mut c = SaveCoordinator::new(true);
    let r = c.edited().unwrap();
    c.local_committed(r, Duration::MAX).unwrap();
    assert_eq!(c.next_deadline(), Some(Duration::MAX));
    c.next_ticket = u64::MAX;
    assert_eq!(c.poll(Duration::MAX), Err(ProtocolError::TicketExhausted));
    assert!(c.active_write().is_none());
    c.current = Revision(u64::MAX);
    assert_eq!(c.edited(), Err(ProtocolError::RevisionExhausted));
    assert!(!c.is_clean());
}

#[test]
fn automatic_saving_is_default_but_a_clean_project_never_needs_a_write() {
    let mut c = SaveCoordinator::default();
    assert!(c.automatic());
    assert!(c.is_clean() && c.is_local_current());
    c.request_flush();
    c.set_automatic(false, sec(0));
    c.set_automatic(true, sec(1));
    assert!(c.poll(sec(100)).unwrap().is_none());
    assert_eq!(c.next_deadline(), None);
    edit(&mut c, 100);
    assert!(c.poll(sec(104)).unwrap().is_none());
    let t = write(&mut c, 105);
    finish(&mut c, t, 105);
    assert!(c.is_clean());
}

#[test]
fn a_future_debounce_for_a_newer_edit_survives_older_write_failure() {
    let mut c = SaveCoordinator::new(true);
    edit(&mut c, 0);
    let t = write(&mut c, 5);
    c.complete(t, Err("first".into()), sec(5)).unwrap();
    let retry = write(&mut c, 6);
    let latest = edit(&mut c, 7);
    c.complete(retry, Err("second".into()), sec(8)).unwrap();
    assert!(c.failure().is_some());
    assert_eq!(c.next_deadline(), Some(sec(12)));
    assert!(c.poll(sec(11)).unwrap().is_none());
    let t = write(&mut c, 12);
    assert_eq!(t.revision(), latest);
    assert!(!t.is_retry());
    assert!(c.failure().is_none());
    finish(&mut c, t, 12);
    assert!(c.is_clean());
}

#[test]
fn an_expired_debounce_does_not_start_another_write_after_terminal_failure() {
    let mut c = SaveCoordinator::new(true);
    edit(&mut c, 0);
    let t = write(&mut c, 5);
    c.complete(t, Err("first".into()), sec(5)).unwrap();
    let retry = write(&mut c, 6);
    edit(&mut c, 7);
    c.complete(retry, Err("second".into()), sec(13)).unwrap();
    assert!(c.failure().is_some());
    assert_eq!(c.next_deadline(), None);
    assert!(c.poll(sec(100)).unwrap().is_none());
    c.request_flush();
    let t = write(&mut c, 100);
    finish(&mut c, t, 100);
    assert!(c.is_clean());
}
