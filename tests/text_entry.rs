// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::text_entry::{Affinity, Caret, Draft};

#[test]
fn range_deletion_preserves_selection_and_grapheme_boundaries() {
    let mut d = Draft::new("one 👩‍💻 two".into());
    let before = d.selection();
    assert!(d.delete_to(Caret::new("one 👩‍💻 ".len())));
    assert_eq!(d.text(), "one 👩‍💻 ");
    assert!(d.undo());
    assert_eq!(d.selection(), before);
    d.select(Caret::new("one 👩‍💻".len()), Caret::new(4));
    let selected = d.selection();
    assert!(d.delete_to(Caret::new(0)));
    assert_eq!(d.text(), "one  two");
    assert!(d.undo());
    assert_eq!(d.selection(), selected);
    d.move_to(Caret::new("one 👩‍💻".len()), false);
    assert!(d.delete_to(Caret::new(6)));
    assert_eq!(d.text(), "one  two");
}

#[test]
fn edits_use_source_bytes_in_mixed_direction_text() {
    let mut d = Draft::new("ABC שלום 123".into());
    let at = d.text().find("123").unwrap();
    d.move_to(Caret::new(at), false);
    assert!(d.insert("|"));
    assert_eq!(d.text(), "ABC שלום |123");
    assert!(d.undo());
    assert_eq!(d.text(), "ABC שלום 123");
    assert_eq!(d.selection().focus.byte, at);
    assert!(d.redo());
    assert_eq!(d.text(), "ABC שלום |123");
}

#[test]
fn backspace_deletes_complete_graphemes() {
    let mut d = Draft::new("a\u{301} 👩‍💻 🇳🇱".into());
    assert_eq!(d.character_count(), 5);
    for expected in ["a\u{301} 👩‍💻 ", "a\u{301} 👩‍💻", "a\u{301} ", "a\u{301}", ""] {
        assert!(d.delete_backward());
        assert_eq!(d.text(), expected);
    }
    assert!(!d.delete_backward());
    for _ in 0..5 {
        assert!(d.undo());
    }
    assert_eq!(d.text(), "a\u{301} 👩‍💻 🇳🇱");
}

#[test]
fn forward_delete_keeps_combining_and_joined_sequences_together() {
    for text in ["a\u{301}\u{327}", "👨‍👩‍👧‍👦", "🇳🇱", "👍🏽", "क्‍ष", "\r\n"]
    {
        let mut d = Draft::new(format!("{text}X"));
        d.move_to(Caret::new(0), false);
        assert!(d.delete_forward());
        assert_eq!(d.text(), "X", "{text:?}");
        assert!(d.undo());
        assert_eq!(d.text(), format!("{text}X"));
    }
}

#[test]
fn invalid_offsets_cannot_split_unicode() {
    let mut d = Draft::new("x👩‍💻y".into());
    for byte in 2..12 {
        d.move_to(Caret::new(byte), false);
        assert_eq!(d.selection().focus.byte, 1);
    }
    d.move_to(Caret::new(usize::MAX), false);
    assert_eq!(d.selection().focus.byte, d.text().len());
    d.select(Caret::new(2), Caret::new(12));
    assert_eq!(d.selected_text(), "👩‍💻");
}

#[test]
fn reversed_selection_copies_source_order_and_undo_restores_direction() {
    let mut d = Draft::new("ABC שלום 123".into());
    d.select(Caret::new(12), Caret::new(4));
    assert_eq!(d.selected_text(), "שלום");
    let before = d.selection();
    d.insert("مرحبا");
    assert_eq!(d.text(), "ABC مرحبا 123");
    d.undo();
    assert_eq!(d.selection(), before);
    assert_eq!(d.selected_text(), "שלום");
}

#[test]
fn line_endings_and_source_normalization_are_preserved() {
    let source = "é\r\na\u{301}\nשלום\rfinal\n";
    let mut d = Draft::new(source.into());
    d.move_to(Caret::new("é\r\n".len()), false);
    d.delete_backward();
    assert_eq!(d.text(), "éa\u{301}\nשלום\rfinal\n");
    d.undo();
    assert_eq!(d.text().as_bytes(), source.as_bytes());
    d.select_all();
    assert_eq!(d.selected_text().as_bytes(), source.as_bytes());
    d.insert("مرحبا\r\n👩‍💻");
    d.undo();
    assert_eq!(d.text().as_bytes(), source.as_bytes());
    d.redo();
    assert_eq!(d.text(), "مرحبا\r\n👩‍💻");
}

#[test]
fn inserting_into_a_joining_sequence_keeps_a_valid_caret() {
    let mut d = Draft::new("a👩💻z".into());
    d.move_to(Caret::new("a👩".len()), false);
    d.insert("\u{200d}");
    assert_eq!(d.text(), "a👩‍💻z");
    assert_eq!(d.selection().focus.byte, "a👩‍💻".len());
    d.undo();
    assert_eq!(d.text(), "a👩💻z");
    assert_eq!(d.selection().focus.byte, "a👩".len());
    d.redo();
    d.delete_backward();
    assert_eq!(d.text(), "az");
}

#[test]
fn affinity_survives_selection_and_undo() {
    let mut d = Draft::new("ABC שלום".into());
    let caret = Caret {
        byte: 4,
        affinity: Affinity::Downstream,
    };
    d.move_to(caret, false);
    d.insert("|");
    d.undo();
    assert_eq!(d.selection().focus, caret);
    assert_eq!(d.selection().anchor, caret);
}

#[test]
fn logical_movement_and_selection_use_graphemes() {
    let mut d = Draft::new("a\u{301}👩‍💻z".into());
    d.move_to(Caret::new(0), false);
    d.move_logical(true, true);
    assert_eq!(d.selected_text(), "a\u{301}");
    d.move_logical(true, true);
    assert_eq!(d.selected_text(), "a\u{301}👩‍💻");
    d.move_logical(false, false);
    assert_eq!(d.selection().focus.byte, 0);
    d.move_logical(true, false);
    assert_eq!(d.selection().focus.byte, "a\u{301}".len());
}

#[test]
fn new_edit_discards_redo_without_recording_noops() {
    let mut d = Draft::new("abc".into());
    assert!(!d.insert(""));
    assert!(!d.can_undo());
    d.insert("d");
    d.undo();
    assert!(d.can_redo());
    d.insert("e");
    assert!(!d.can_redo());
    d.undo();
    assert_eq!(d.text(), "abc");
    assert!(!d.can_undo());
}

#[test]
fn bounded_history_retains_recent_reversible_edits() {
    let mut d = Draft::new(String::new());
    for _ in 0..300 {
        d.insert("👩‍💻");
    }
    let mut count = 0;
    while d.undo() {
        count += 1;
    }
    assert_eq!(count, 256);
    assert_eq!(d.character_count(), 44);
    while d.redo() {}
    assert_eq!(d.character_count(), 300);
}
