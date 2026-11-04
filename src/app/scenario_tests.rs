// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Four bounded journeys through raw_input_hook + the actual application UI.
//! Existing regressions remain intact; these tests add no native-delivery claim.
use super::test_driver::Driver;
use super::*;
use wireframe::document::Value;

#[test]
fn pilot_picker_navigation_selection_cancel_and_input_order() {
    let mut d = Driver::new();
    d.app.editor.add(Kind::TextInput, 100., 150.);
    let id = *d.app.editor.selection.iter().next().unwrap();
    d.app
        .editor
        .set_control_property(id, "text", Value::Text("Your email".into()))
        .unwrap();
    d.idle(3);
    let trigger = d.unique("Left").id;
    d.focus_action(trigger);
    let before = d.app.editor.document.clone();
    d.tap(Key::Space, Modifiers::NONE);
    d.until("picker opened", 8, |d| egui::Popup::is_any_open(&d.ctx));
    d.check(
        d.focused() == Some(trigger),
        "first Down starts on the open picker trigger",
    );
    let focus = d.focused();
    d.release(Key::ArrowDown, Modifiers::NONE);
    d.check(d.focused() == focus, "release alone must not navigate");
    for expected in ["Left", "Center", "Right", "Right"] {
        d.press(Key::ArrowDown, Modifiers::NONE);
        d.check(
            d.focused() != Some(trigger),
            "Down enters an option, not the identically named trigger",
        );
        d.check(
            d.focus_name() == Some(expected),
            "Down remains in the popup",
        );
        d.project(&before, "focus movement is provisional");
        d.release(Key::ArrowDown, Modifiers::NONE);
        d.check(
            d.focus_name() == Some(expected),
            "release does not advance again",
        );
        d.project(&before, "release preserves document");
    }
    d.tap(Key::ArrowUp, Modifiers::NONE);
    d.check(d.focus_name() == Some("Center"), "Up returns to Center");
    d.tap(Key::Enter, Modifiers::NONE);
    d.until("selection returns focus", 8, |d| {
        !egui::Popup::is_any_open(&d.ctx) && d.focused() == Some(trigger)
    });
    let mut expected = before.clone();
    expected.pages[0].content.nodes[0]
        .properties
        .insert("align".into(), Value::Choice("center".into()));
    d.project(&expected, "Enter applies only alignment");
    d.tap(Key::ArrowRight, Modifiers::NONE);
    d.project(&expected, "picker focus must not nudge canvas artwork");
    d.tap(Key::Z, Modifiers::COMMAND);
    d.project(&before, "one Undo restores complete graph");
    let redo = d.app.editor.redo_name().map(str::to_owned);
    d.focus_action(trigger);
    d.tap(Key::Space, Modifiers::NONE);
    d.idle(1);
    d.tap(Key::ArrowDown, Modifiers::NONE);
    d.tap(Key::ArrowDown, Modifiers::NONE);
    d.tap(Key::Escape, Modifiers::NONE);
    d.until("cancel returns picker focus", 8, |d| {
        !egui::Popup::is_any_open(&d.ctx) && d.focused() == Some(trigger)
    });
    d.project(&before, "Escape does not apply provisional choice");
    d.check(
        d.app.editor.redo_name() == redo.as_deref(),
        "cancel retains Redo",
    );
    d.check(
        d.app.editor.selection == HashSet::from([id]),
        "selection identity retained",
    );
    // Preserve modifier/release metadata and order at the app raw-input boundary.
    // A modified arrow may move popup focus; it must not edit the document.
    for modifiers in [Modifiers::SHIFT, Modifiers::ALT, Modifiers::COMMAND] {
        d.focus_action(trigger);
        d.tap(Key::Space, Modifiers::NONE);
        d.idle(1);
        d.input(
            vec![
                Driver::key(Key::ArrowDown, true, modifiers),
                Driver::key(Key::ArrowDown, false, modifiers),
            ],
            modifiers,
            true,
            Vec::new(),
        );
        let edges: Vec<_> = d
            .last_hook_events
            .iter()
            .filter_map(|event| match event {
                egui::Event::Key {
                    key: Key::ArrowDown,
                    pressed,
                    modifiers,
                    ..
                } => Some((*pressed, *modifiers)),
                _ => None,
            })
            .collect();
        assert_eq!(
            edges,
            vec![(true, modifiers), (false, modifiers)],
            "{}",
            d.diagnostics()
        );
        d.project(&before, "modified arrow cannot edit artwork");
        d.tap(Key::Escape, Modifiers::NONE);
        d.idle(1);
        d.check(
            !egui::Popup::is_any_open(&d.ctx),
            "modified route can cancel",
        );
        d.check(
            d.app.editor.redo_name() == redo.as_deref(),
            "modified route preserves Redo",
        );
    }
    d.tap(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
    d.project(&expected, "Redo restores exact selected graph");
}

fn gesture(
    d: &mut Driver,
    point: Pos2,
    pressed: Option<bool>,
    modifiers: Modifiers,
    focused: bool,
) {
    let mut events = vec![egui::Event::PointerMoved(point)];
    if let Some(pressed) = pressed {
        events.push(Driver::pointer(point, pressed, modifiers));
    }
    d.input(events, modifiers, focused, Vec::new());
}

#[test]
fn pilot_interrupted_drag_cancel_and_one_step_history() {
    for duplicate in [false, true] {
        let mut d = Driver::new();
        d.size(Vec2::new(1100., 850.));
        d.app.editor.add(Kind::TextInput, 50., 50.);
        d.app.editor.add(Kind::Button, 350., 100.);
        d.app.editor.add(Kind::Checkbox, 600., 100.);
        let redo = d.app.editor.document.clone();
        d.app.editor.undo(); // Fixture: a genuine pre-existing Redo branch.
        d.app.editor.select_all();
        let before = d.app.editor.document.clone();
        let undo = d.app.editor.undo_name().map(str::to_owned);
        d.idle(3);
        let start = d.app.screen([70., 60.]);
        let middle = start + Vec2::new(60., 30.);
        let end = start + Vec2::new(90., 60.);
        let modifiers = Modifiers {
            alt: duplicate,
            command: true,
            ..Modifiers::NONE
        };
        gesture(&mut d, start, Some(true), modifiers, true);
        gesture(&mut d, middle, None, modifiers, true);
        d.check(
            d.app.drag.is_some() && d.app.editor.document != before,
            "real drag has provisional geometry",
        );
        gesture(&mut d, end, Some(false), modifiers, false);
        d.check(
            d.app.error.is_none() && d.app.drag.is_none(),
            "deactivation ends drag without error",
        );
        d.project(
            &before,
            "same-frame release/deactivation cancels complete graph",
        );
        d.check(
            d.app.editor.undo_name() == undo.as_deref(),
            "cancel does not commit history",
        );
        d.frame(Vec::new());
        d.tap(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
        d.project(&redo, "cancel retains earlier Redo branch");
        d.tap(Key::Z, Modifiers::COMMAND);
        d.project(&before, "return to gesture baseline");
        d.app.editor.select_all();
        gesture(&mut d, start, Some(true), modifiers, true);
        gesture(&mut d, middle, None, modifiers, true);
        d.check(
            d.app.drag.is_some(),
            "inspector does not prematurely commit",
        );
        gesture(&mut d, end, Some(false), modifiers, true);
        d.check(
            d.app.drag.is_none() && d.app.error.is_none(),
            "release finishes one gesture",
        );
        let nodes = d.app.editor.nodes();
        assert_eq!(
            nodes.len(),
            if duplicate { 4 } else { 2 },
            "{}",
            d.diagnostics()
        );
        for original in &before.pages[0].content.nodes {
            let moved = nodes
                .iter()
                .find(|node| {
                    node.content == original.content && (duplicate != (node.id == original.id))
                })
                .unwrap();
            let mut expected = original.clone();
            expected.id = moved.id;
            expected.origin.x += 90.;
            expected.origin.y += 60.;
            assert_eq!(*moved, expected, "{}", d.diagnostics());
        }
        if duplicate {
            assert_eq!(
                &nodes[..2],
                before.pages[0].content.nodes.as_slice(),
                "{}",
                d.diagnostics()
            );
        }
        let moved = d.app.editor.document.clone();
        d.tap(Key::Z, Modifiers::COMMAND);
        d.project(&before, "one Undo restores entire move/copy");
        d.tap(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
        d.project(&moved, "one Redo restores entire move/copy");
        // Existing fault boundary: model edit finishes while input still owns drag.
        let stale_start = {
            let nodes = d.app.editor.nodes();
            let selected = nodes
                .iter()
                .find(|node| d.app.editor.selection.contains(&node.id))
                .unwrap();
            d.app
                .screen([selected.origin.x + 20., selected.origin.y + 10.])
        };
        let stale_middle = stale_start + Vec2::new(60., 30.);
        let stale_end = stale_start + Vec2::new(90., 60.);
        gesture(&mut d, stale_start, Some(true), modifiers, true);
        gesture(&mut d, stale_middle, None, modifiers, true);
        d.check(
            d.app.drag.is_some(),
            "fault fixture has an actual active drag",
        );
        d.app.editor.commit();
        let finished = d.app.editor.document.clone();
        let undo = d.app.editor.undo_name().map(str::to_owned);
        gesture(&mut d, stale_end, None, modifiers, true);
        d.check(
            d.app.error.as_deref() == Some("The move was interrupted. Try dragging again."),
            "stale drag reports interruption",
        );
        d.check(
            d.app.drag.is_none() && d.app.alignment_guides.is_empty(),
            "stale drag and guides clear",
        );
        d.project(&finished, "stale input cannot extend completed edit");
        d.check(
            d.app.editor.undo_name() == undo.as_deref(),
            "stale input does not add history",
        );
        d.tap(Key::Escape, Modifiers::NONE);
        gesture(&mut d, stale_end, Some(false), modifiers, true);
        d.check(
            d.app.error.is_none() && d.app.drag.is_none(),
            "late release cannot restart or repeat error",
        );
        d.project(&finished, "late release preserves finished graph");
    }
}

fn enter_text(d: &mut Driver, target: Id) {
    d.app.editor.select(target, false); // Fixture selection; entry itself uses UI.
    d.idle(3);
    d.click_point(d.app.screen([70., 60.]));
    d.tap(Key::Enter, Modifiers::NONE);
    d.until("text field owns focus", 16, |d| {
        d.app.text_edit.is_some() && wireframe::text_entry::is_focused(&d.ctx)
    });
}

#[test]
fn pilot_text_draft_save_history_and_fresh_reopen() {
    for menu_command in [false, true] {
        for worker in [false, true] {
            let mut d = Driver::new();
            d.size(Vec2::new(1200., 850.));
            d.app.editor.add(Kind::Button, 50., 50.);
            let target = d.app.editor.current().controls[0].id;
            d.app.editor.add(Kind::Button, 50., 250.);
            let other = d.app.editor.current().controls[1].id;
            let original = d.app.editor.document.clone();
            let path = d.path("Original – 世界.wireframe");
            ProjectStore.save(&path, &original).unwrap();
            d.app.file = Some(path.clone());
            d.app.editor.mark_saved();
            if worker {
                d.enable_manual_persistence();
            }
            // Match accept_text_draft's retained transaction exactly. A raw
            // property preview alone does not create a history entry.
            d.app.editor.edit("Edit text", |editor| {
                let mut control = editor
                    .current()
                    .controls
                    .into_iter()
                    .find(|control| control.id == other)
                    .unwrap();
                control.text = "Earlier committed change".into();
                editor.update_control(control);
            });
            let committed = d.app.editor.document.clone();
            d.check(
                committed != original && d.app.editor.dirty(),
                "fixture has earlier committed dirty change",
            );
            d.check(
                d.app.editor.undo_name() == Some("Edit text"),
                "fixture has exactly the intended edit boundary",
            );
            enter_text(&mut d, target);
            d.check(
                ProjectStore.load(&path).unwrap() == original,
                "opening draft does not save",
            );
            let command = if menu_command {
                egui::Event::AppCommand(menus::Command::Save as u64)
            } else {
                Driver::key(Key::S, true, Modifiers::COMMAND)
            };
            d.input(
                vec![egui::Event::Paste("Still provisional".into()), command],
                if menu_command {
                    Modifiers::NONE
                } else {
                    Modifiers::COMMAND
                },
                true,
                Vec::new(),
            );
            if !menu_command {
                d.release(Key::S, Modifiers::COMMAND);
            }
            d.idle(3);
            d.wait_io("Save completion", |d| !d.app.save_busy());
            d.check(
                d.app.text_edit.as_ref().map(|(_, text)| text.as_str())
                    == Some("Still provisional"),
                "Save retains provisional draft",
            );
            d.project(&committed, "Save excludes draft from live document");
            d.project_value(
                &ProjectStore.load(&path).unwrap(),
                &committed,
                "saved graph excludes draft",
            );
            d.check(!d.app.editor.dirty(), "saved committed graph is clean");
            d.frame(vec![egui::Event::Text(" continued".into())]);
            d.check(
                d.app.text_edit.as_ref().unwrap().1 == "Still provisional continued",
                "typing continues in draft after Save",
            );
            d.tap(Key::Escape, Modifiers::NONE);
            d.check(
                d.app.text_edit.is_none() && !d.app.editor.dirty(),
                "Escape ends only draft",
            );
            d.project(&committed, "Escape preserves saved graph");
            d.tap(Key::Z, Modifiers::COMMAND);
            d.project(&original, "Undo reaches pre-commit graph");
            d.tap(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
            d.project(&committed, "Redo reaches exact saved graph");
            d.finish_session("release fixture owner before fresh read");
            let mut reopened = d.fresh();
            if worker {
                reopened.enable_manual_persistence();
            }
            reopened.app.editor.mark_saved();
            reopened.drop_project(path.clone());
            reopened.wait_io("fresh Open completion", |d| {
                d.app.startup_open.is_none() && d.app.file.as_ref() == Some(&path)
            });
            reopened.project(&committed, "fresh app/context reads actual saved file");
            reopened.check(
                reopened.app.error.is_none() && !reopened.app.editor.dirty(),
                "fresh document is clean",
            );
            reopened.check(
                reopened.app.editor.undo_name().is_none()
                    && reopened.app.editor.redo_name().is_none(),
                "fresh history is empty",
            );
            reopened.finish_session("fresh owner finishes normally");
        }
    }
}

fn persistent_original() -> (Driver, PathBuf, Project) {
    let mut d = Driver::new();
    let original = Project {
        title: "Original".into(),
        ..Project::default()
    };
    let path = d.path("Original – 世界.wireframe");
    ProjectStore.save(&path, &original).unwrap();
    d.enable_manual_persistence();
    d.app.editor.mark_saved();
    d.drop_project(path.clone());
    d.wait_io("initial Open", |d| {
        d.app.startup_open.is_none() && d.app.file.as_ref() == Some(&path)
    });
    d.check(d.app.error.is_none(), "initial persistent fixture opens");
    d.project(&original, "initial Open graph");
    (d, path, original)
}

#[test]
fn pilot_async_open_save_failure_and_stale_ownership() {
    for variant in ["missing", "invalid", "locked"] {
        let (mut d, original_path, original) = persistent_original();
        let bytes = std::fs::read(&original_path).unwrap();
        d.app.editor.add(Kind::Label, 20., 30.);
        let changed = d.app.editor.document.clone();
        d.frame(Vec::new());
        let id = d.app.open_source_id();
        let path = d.path(&format!("{variant}.wireframe"));
        let copies = d.app.startup_copies().unwrap();
        let owner = match variant {
            "invalid" => {
                std::fs::write(&path, "invalid").unwrap();
                None
            }
            "locked" => {
                ProjectStore.save(&path, &Project::default()).unwrap();
                Some(copies.open_named(&path).unwrap())
            }
            _ => None,
        };
        d.drop_project(path);
        d.click("Discard changes");
        d.wait_io("failed Open completion", |d| d.app.startup_open.is_none());
        d.check(
            d.app.error.is_some() && matches!(d.app.pending, Some(Pending::Open(_))),
            "failure retains pending Open and error",
        );
        d.check(
            d.app.open_source_id() == id,
            "failed Open keeps original owner",
        );
        d.project(&changed, "failed Open keeps complete edited graph");
        assert_eq!(
            std::fs::read(&original_path).unwrap(),
            bytes,
            "{}",
            d.diagnostics()
        );
        d.check(
            copies.open_named(&original_path).is_err(),
            "live source remains exclusively owned",
        );
        d.tap(Key::Escape, Modifiers::NONE); // top error
        d.tap(Key::Escape, Modifiers::NONE); // pending unsaved review
        d.tap(Key::Z, Modifiers::COMMAND);
        d.project(&original, "failure keeps existing Undo");
        drop(owner);
        d.finish_session("failed-Open fixture finishes normally");
    }
    // Real asynchronous Save As failure via the actual unsaved-review button.
    let (mut d, original_path, _) = persistent_original();
    let bytes = std::fs::read(&original_path).unwrap();
    d.app.editor.add(Kind::Button, 20., 30.);
    let changed = d.app.editor.document.clone();
    d.app.pending = Some(Pending::New); // Establish the existing pending-action fixture.
    d.app.save_dialog_choice = Some(d.path("missing/Copy.wireframe"));
    d.idle(2);
    d.click("Save as…");
    d.wait_io("failed Save As completion", |d| !d.app.save_busy());
    d.check(
        d.app.error.is_some() && matches!(d.app.pending, Some(Pending::New)),
        "save failure retains pending action",
    );
    d.check(
        d.app.file.as_ref() == Some(&original_path),
        "failed Save As retains source file",
    );
    d.project(&changed, "save failure retains edited graph");
    assert_eq!(
        std::fs::read(&original_path).unwrap(),
        bytes,
        "{}",
        d.diagnostics()
    );
    d.tap(Key::Escape, Modifiers::NONE);
    d.tap(Key::Escape, Modifiers::NONE);
    d.finish_session("failed-save fixture finishes normally");

    // Gate only the external completion. Candidate preparation/validation,
    // app cancellation, ownership release and stale rejection remain real.
    let (mut d, original_path, _) = persistent_original();
    d.app.editor.add(Kind::Button, 20., 30.);
    let changed = d.app.editor.document.clone();
    let owner = d.app.open_source_id();
    let target = d.path("Slow.wireframe");
    ProjectStore
        .save(
            &target,
            &Project {
                title: "Slow".into(),
                ..Project::default()
            },
        )
        .unwrap();
    let copies = d.app.startup_copies().unwrap();
    let mut gate = startup::test_support::gated_discard_open(&mut d.app, &d.ctx, target.clone());
    gate.wait_ready();
    d.check(
        copies.open_named(&original_path).is_err() && copies.open_named(&target).is_err(),
        "source and prepared target stay owned",
    );
    d.frame(Vec::new());
    d.tap(Key::N, Modifiers::COMMAND);
    d.project(&changed, "blocked Open refuses other document command");
    d.check(
        d.app.canvas_input_blocked()
            && !d.app.command_enabled(&d.ctx, menus::Command::Save)
            && d.app.command_enabled(&d.ctx, menus::Command::Quit),
        "Open has visible bounded input ownership",
    );
    d.click("Cancel");
    d.check(
        d.app.startup_open.is_none(),
        "Cancel dismisses waiting Open",
    );
    d.project(&changed, "Cancel retains live source graph");
    d.click_point(d.app.screen([40., 40.]));
    d.tap(Key::ArrowRight, Modifiers::NONE);
    let after_cancel = d.app.editor.document.clone();
    d.check(
        after_cancel != changed && d.app.error.is_none(),
        "source remains editable after Cancel",
    );
    gate.release();
    d.wait_io("cancelled candidate releases target lock", |_| {
        copies.open_named(&target).is_ok()
    });
    d.idle(2);
    d.project(
        &after_cancel,
        "late candidate cannot replace post-Cancel edits",
    );
    d.check(
        d.app.open_source_id() == owner,
        "late result cannot transfer source ownership",
    );
    d.finish_session("cancelled-Open source finishes normally");
    d.project_value(
        copies.open_named(&original_path).unwrap().project(),
        &after_cancel,
        "local copy retains post-Cancel edits",
    );
}
