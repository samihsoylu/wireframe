// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
fn key(key: Key, shift: bool) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: if shift {
            Modifiers::COMMAND | Modifiers::SHIFT
        } else {
            Modifiers::COMMAND
        },
    }
}
fn dispatch(ctx: &egui::Context, app: &mut WireframeApp, events: Vec<egui::Event>) {
    let mut output = ctx.run_ui(
        egui::RawInput {
            events,
            ..Default::default()
        },
        |ui| app.shortcuts(ui.ctx()),
    );
    output.textures_delta.clear();
}

#[test]
fn layering_shortcuts_preserve_geometry_history_and_text_focus() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    for _ in 0..3 {
        app.editor.add(Kind::Button, 20., 20.);
    }
    let first = app.editor.nodes()[0].id;
    app.editor.select(first, false);
    let before = app.editor.document.clone();
    dispatch(&ctx, &mut app, vec![key(Key::ArrowUp, false)]);
    assert_eq!(app.editor.nodes()[1].id, first);
    assert!(
        app.editor
            .nodes()
            .iter()
            .all(|n| n.origin.x == 20. && n.origin.y == 20.)
    );
    dispatch(&ctx, &mut app, vec![key(Key::Z, false)]);
    assert_eq!(app.editor.document, before);
    dispatch(&ctx, &mut app, vec![key(Key::ArrowUp, true)]);
    assert_eq!(app.editor.nodes()[2].id, first);
    dispatch(&ctx, &mut app, vec![key(Key::ArrowDown, true)]);
    assert_eq!(app.editor.document, before);
    let mut field = "Draft".to_owned();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        ui.add(TextInput::singleline(&mut field).autofocus(true));
    });
    output.textures_delta.clear();
    assert!(wireframe::text_entry::is_focused(&ctx));
    dispatch(&ctx, &mut app, vec![key(Key::ArrowUp, true)]);
    assert_eq!(app.editor.document, before);
}
#[test]
fn undo_then_save_in_one_frame_persists_the_undone_state() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.file = Some(dir.path().join("test.wireframe"));
    app.editor.add(Kind::Video, 50., 50.);
    let original = app.editor.document.clone();
    app.editor
        .edit("Change style", |e| e.document.settings.sketch = false);
    dispatch(&ctx, &mut app, vec![key(Key::Z, false), key(Key::S, false)]);
    assert_eq!(app.editor.document, original);
    assert_eq!(
        ProjectStore.load(app.file.as_ref().unwrap()).unwrap(),
        original
    );
    assert!(!app.editor.dirty());
}
#[test]
fn save_then_undo_keeps_the_saved_snapshot_and_marks_the_editor_dirty() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.file = Some(dir.path().join("test.wireframe"));
    app.editor.add(Kind::Video, 50., 50.);
    let original = app.editor.document.clone();
    app.editor
        .edit("Change style", |e| e.document.settings.sketch = false);
    let edited = app.editor.document.clone();
    dispatch(&ctx, &mut app, vec![key(Key::S, false), key(Key::Z, false)]);
    assert_eq!(app.editor.document, original);
    assert_eq!(
        ProjectStore.load(app.file.as_ref().unwrap()).unwrap(),
        edited
    );
    assert!(app.editor.dirty());
}
#[test]
fn repeated_history_events_and_redo_save_preserve_event_order() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.file = Some(dir.path().join("test.wireframe"));
    app.editor.add(Kind::Video, 50., 50.);
    let one = app.editor.document.clone();
    app.editor.add(Kind::Webcam, 400., 50.);
    let two = app.editor.document.clone();
    app.editor.add(Kind::Playback, 50., 300.);
    dispatch(
        &ctx,
        &mut app,
        vec![key(Key::Z, false), key(Key::Z, false), key(Key::S, false)],
    );
    assert_eq!(app.editor.document, one);
    assert_eq!(ProjectStore.load(app.file.as_ref().unwrap()).unwrap(), one);
    dispatch(&ctx, &mut app, vec![key(Key::Z, true), key(Key::S, false)]);
    assert_eq!(app.editor.document, two);
    assert_eq!(ProjectStore.load(app.file.as_ref().unwrap()).unwrap(), two);
}
#[test]
fn new_project_confirmation_blocks_later_save_in_the_same_frame() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    let path = dir.path().join("test.wireframe");
    app.file = Some(path.clone());
    app.editor.add(Kind::Video, 50., 50.);
    let expected = app.editor.document.clone();
    dispatch(&ctx, &mut app, vec![key(Key::N, false), key(Key::S, false)]);
    assert!(app.pending.is_some());
    assert_eq!(app.editor.document, expected);
    assert!(!path.exists());
}
#[test]
fn new_page_shortcut_does_not_trigger_new_project() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Video, 50., 50.);
    let first = app.editor.document.pages[0].clone();
    dispatch(&ctx, &mut app, vec![key(Key::N, true)]);
    assert!(app.pending.is_none());
    assert_eq!(app.editor.document.pages.len(), 2);
    assert_eq!(app.editor.document.pages[0], first);
}
#[test]
fn movement_repeats_and_save_use_the_ordered_input_for_each_platform_modifier() {
    for command in [
        Modifiers::COMMAND,
        Modifiers::CTRL | Modifiers::COMMAND,
        Modifiers::MAC_CMD | Modifiers::COMMAND,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.file = Some(dir.path().join("test.wireframe"));
        app.editor.add(Kind::Video, 50., 50.);
        let arrow = |pressed, repeat| egui::Event::Key {
            key: Key::ArrowRight,
            physical_key: None,
            pressed,
            repeat,
            modifiers: Modifiers::NONE,
        };
        let save = || egui::Event::Key {
            key: Key::S,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: command,
        };
        dispatch(
            &ctx,
            &mut app,
            vec![
                arrow(true, false),
                arrow(true, true),
                arrow(false, false),
                save(),
                arrow(true, true),
            ],
        );
        assert_eq!(app.editor.selected_views()[0].bounds.x, 53.);
        let saved = ProjectStore.load(app.file.as_ref().unwrap()).unwrap();
        assert_eq!(saved.pages[0].content.nodes[0].origin.x, 52.);
        assert!(app.editor.dirty());
    }
}
#[test]
fn duplicate_copy_and_delete_follow_the_input_order() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Video, 50., 50.);
    let original = *app.editor.selection.iter().next().unwrap();
    dispatch(
        &ctx,
        &mut app,
        vec![key(Key::D, false), egui::Event::Cut, key(Key::Z, false)],
    );
    let request = app
        .pending_cut_request()
        .expect("Cut awaits the system clipboard");
    assert_eq!(app.editor.current().controls.len(), 2);
    assert!(app.clipboard.is_empty());
    dispatch(
        &ctx,
        &mut app,
        vec![egui::Event::ClipboardWriteResult {
            request,
            result: Ok(()),
        }],
    );
    assert_eq!(app.editor.current().controls.len(), 2);
    assert_eq!(app.clipboard.len(), 1);
    assert!(
        app.editor
            .current()
            .controls
            .iter()
            .any(|c| c.id == original)
    );
}

#[test]
fn pdf_options_block_document_shortcuts_until_the_dialog_is_closed() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 50., 50.);
    let before = app.editor.document.clone();
    app.pdf_options = Some(wireframe::pdf::Options::default());
    dispatch(&ctx, &mut app, vec![key(Key::Z, false), key(Key::N, false)]);
    assert_eq!(app.editor.document, before);
    assert!(app.pending.is_none());
    app.pdf_options = None;
    dispatch(&ctx, &mut app, vec![key(Key::Z, false)]);
    assert_ne!(app.editor.document, before);
}

#[test]
fn save_between_field_edits_preserves_the_snapshot_and_later_input_order() {
    for before_event in [
        egui::Event::Text("before".into()),
        egui::Event::Paste("before".into()),
    ] {
        for save_event in [
            key(Key::S, false),
            egui::Event::AppCommand(menus::Command::Save as u64),
        ] {
            for (later, asynchronous, late) in [
                (egui::Event::Text("after".into()), false, false),
                (key(Key::Z, false), false, false),
                (egui::Event::Text("after".into()), true, false),
                (key(Key::Z, false), true, false),
                (egui::Event::Text("after".into()), true, true),
            ] {
                let dir = tempfile::tempdir().unwrap();
                let ctx = egui::Context::default();
                let mut app = WireframeApp::workspace(&ctx, None);
                app.file = Some(dir.path().join("test.wireframe"));
                app.editor.document.title.clear();
                if asynchronous {
                    ProjectStore
                        .save(app.file.as_ref().unwrap(), &app.editor.document)
                        .unwrap();
                    app.enable_persistence(&ctx, dir.path().join("local"));
                }
                let mut run = |events| {
                    let mut raw = egui::RawInput {
                        events,
                        ..Default::default()
                    };
                    app.order_text_input(&ctx, &mut raw);
                    let mut output = ctx.run_ui(raw, |ui| {
                        // Exercise a disabled busy frame even if this small test save
                        // has already finished on the worker before we poll it.
                        if app.save_busy() {
                            ui.disable();
                        }
                        app.poll_saves(ui.ctx());
                        app.shortcuts(ui.ctx());
                        ui.add(
                            TextInput::singleline(&mut app.editor.document.title).autofocus(true),
                        );
                    });
                    output.textures_delta.clear();
                };
                run(vec![]);
                if late {
                    run(vec![before_event.clone(), save_event.clone()]);
                    run(vec![]); // Dispatch Save with no later input queued yet.
                    run(vec![later.clone()]); // New input arrives during the busy frame.
                } else {
                    run(vec![
                        before_event.clone(),
                        save_event.clone(),
                        later.clone(),
                    ]);
                }
                for _ in 0..50 {
                    run(vec![]);
                    if asynchronous {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                }
                assert_eq!(
                    ProjectStore.load(app.file.as_ref().unwrap()).unwrap().title,
                    "before"
                );
                assert_eq!(
                    app.editor.document.title,
                    if matches!(later, egui::Event::Text(_)) {
                        "beforeafter"
                    } else {
                        ""
                    }
                );
                assert!(app.deferred_text_input.is_empty());
            }
        }
    }
}

#[test]
fn text_after_tab_reaches_the_next_field_even_when_focus_wraps() {
    for reverse in [false, true] {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        let mut values = [String::new(), String::new()];
        let mut run = |events| {
            let mut raw = egui::RawInput {
                events,
                ..Default::default()
            };
            app.order_text_input(&ctx, &mut raw);
            let mut output = ctx.run_ui(raw, |ui| {
                for index in if reverse { [1, 0] } else { [0, 1] } {
                    ui.add(
                        TextInput::singleline(&mut values[index])
                            .id_salt(index)
                            .autofocus(index == 0),
                    );
                }
            });
            output.textures_delta.clear();
        };
        run(vec![]);
        run(vec![
            egui::Event::Text("before".into()),
            egui::Event::Key {
                key: Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
            egui::Event::Text("after".into()),
        ]);
        run(vec![]);
        assert_eq!(values, ["before", "after"], "reverse={reverse}");
    }
}

#[test]
fn text_before_and_after_a_coalesced_click_reaches_each_field_in_order() {
    for reverse in [false, true] {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        let mut values = [String::new(), String::new()];
        let mut run = |events| {
            let mut raw = egui::RawInput {
                events,
                ..Default::default()
            };
            app.order_text_input(&ctx, &mut raw);
            let mut rects = [Rect::NOTHING; 2];
            let mut output = ctx.run_ui(raw, |ui| {
                for index in if reverse { [1, 0] } else { [0, 1] } {
                    rects[index] = ui
                        .add(
                            TextInput::singleline(&mut values[index])
                                .id_salt(index)
                                .autofocus(index == 0),
                        )
                        .rect;
                }
            });
            output.textures_delta.clear();
            rects
        };
        let at = run(vec![])[1].center();
        let mut events = vec![
            egui::Event::Text("before".into()),
            egui::Event::PointerMoved(at),
        ];
        for pressed in [true, false] {
            events.push(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            });
        }
        events.push(egui::Event::Text("after".into()));
        run(events);
        run(vec![]);
        run(vec![]);
        assert_eq!(values, ["before", "after"], "reverse={reverse}");
        assert!(app.deferred_text_input.is_empty());
    }
}

#[test]
fn clicking_save_after_typing_captures_the_updated_field() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.file = Some(dir.path().join("test.wireframe"));
    app.editor.document.title.clear();
    let mut run = |events| {
        let mut raw = egui::RawInput {
            events,
            ..Default::default()
        };
        app.order_text_input(&ctx, &mut raw);
        let mut rect = Rect::NOTHING;
        let mut output = ctx.run_ui(raw, |ui| {
            let response = ui.button("Save");
            rect = response.rect;
            if response.clicked() {
                app.save(false);
            }
            ui.add(TextInput::singleline(&mut app.editor.document.title).autofocus(true));
        });
        output.textures_delta.clear();
        rect
    };
    let pos = run(vec![]).center();
    run(vec![
        egui::Event::Text("complete".into()),
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ]);
    run(vec![]);
    assert_eq!(
        ProjectStore.load(app.file.as_ref().unwrap()).unwrap().title,
        "complete"
    );
}

#[test]
fn presentation_forward_restores_route_after_back_link() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    let landing = app.editor.page;
    app.editor.add_page();
    let contact = app.editor.page;
    app.editor.switch_page(landing);
    app.present = true;
    app.navigate_page(contact);
    app.follow_target("back:", &ctx);
    assert_eq!(app.editor.page, landing);
    dispatch(
        &ctx,
        &mut app,
        vec![egui::Event::Key {
            key: Key::ArrowRight,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::ALT,
        }],
    );
    assert_eq!(
        app.editor.page, contact,
        "Forward must retain the route after Back"
    );
}

#[test]
fn presentation_next_restores_remembered_alternate() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    let landing = app.editor.page;
    app.editor.add_page();
    let alternate = app.editor.create_version("Detailed").unwrap();
    app.editor.switch_page(landing);
    app.present = true;
    dispatch(
        &ctx,
        &mut app,
        vec![egui::Event::Key {
            key: Key::ArrowDown,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::ALT,
        }],
    );
    assert_eq!(
        app.editor.surface_id(),
        alternate,
        "Next must use the remembered version"
    );
}

#[test]
fn cut_then_save_waits_for_publication_before_saving_removed_content() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.file = Some(dir.path().join("cut-save.wireframe"));
    app.editor.add(Kind::Button, 40., 50.);
    dispatch(&ctx, &mut app, vec![egui::Event::Cut, key(Key::S, false)]);
    assert_eq!(app.editor.nodes().len(), 1);
    assert!(!app.file.as_ref().unwrap().exists());
    let request = app.pending_cut_request().unwrap();
    dispatch(
        &ctx,
        &mut app,
        vec![egui::Event::ClipboardWriteResult {
            request,
            result: Ok(()),
        }],
    );
    assert!(app.editor.nodes().is_empty());
    assert_eq!(
        ProjectStore.load(app.file.as_ref().unwrap()).unwrap(),
        app.editor.document
    );
    assert!(app.deferred_cut_input.is_empty());
}

#[test]
fn focused_search_button_owns_return_over_selected_canvas_text() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 50., 50.);
    let document = app.editor.document.clone();
    let mut query = String::new();
    let mut field_focused = false;
    for focus in [true, false] {
        let events = if focus {
            vec![]
        } else {
            vec![egui::Event::Key {
                key: Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }]
        };
        let mut output = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| {
                app.shortcuts(ui.ctx());
                let search = ui.make_persistent_id("quick-add-search");
                let field = super::toolbar::quick_input(ui, &mut query);
                if focus {
                    ui.memory_mut(|m| m.request_focus(search));
                }
                field_focused = field.has_focus();
            },
        );
        output.textures_delta.clear();
    }
    assert!(field_focused, "Return must focus the search field");
    assert!(
        app.text_edit.is_none(),
        "Return must not edit the canvas selection"
    );
    assert_eq!(app.editor.document, document);
}

#[test]
fn canvas_return_still_edits_selected_text_with_or_without_canvas_focus() {
    for focused in [false, true] {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Button, 50., 50.);
        let mut output = ctx.run_ui(Default::default(), |ui| {
            app.canvas(ui);
            if focused {
                ui.memory_mut(|m| m.request_focus(app.canvas_focus_id.unwrap()));
            }
        });
        output.textures_delta.clear();
        dispatch(
            &ctx,
            &mut app,
            vec![egui::Event::Key {
                key: Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
        assert!(app.text_edit.is_some());
    }
}

fn inspector_command_frame(
    app: &mut WireframeApp,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
) -> Option<Rect> {
    let mut raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 800.))),
        events,
        ..Default::default()
    };
    app.order_text_input(ctx, &mut raw);
    let mut output = ctx.run_ui(raw, |ui| {
        app.shortcuts(ui.ctx());
        if !app.present {
            if app.pending.is_some() {
                ui.disable();
            }
            app.inspector(ui);
        }
    });
    output.textures_delta.clear();
    output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find_map(|(_, node)| {
            if node.value() != Some("Button")
                || node.role() != egui::accesskit::Role::MultilineTextInput
            {
                return None;
            }
            let r = node.bounds()?;
            Some(Rect::from_min_max(
                egui::pos2(r.x0 as f32, r.y0 as f32),
                egui::pos2(r.x1 as f32, r.y1 as f32),
            ))
        })
}

fn inspector_command_setup() -> (egui::Context, WireframeApp) {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 40., 50.);
    inspector_command_frame(&mut app, &ctx, vec![]);
    let position = inspector_command_frame(&mut app, &ctx, vec![])
        .expect("Inspector text field")
        .center();
    for pressed in [true, false] {
        inspector_command_frame(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
    assert!(wireframe::text_entry::is_focused(&ctx));
    (ctx, app)
}

#[test]
fn inspector_text_preceding_page_or_presentation_commands_is_not_lost() {
    for command in [Key::N, Key::F] {
        let (ctx, mut app) = inspector_command_setup();
        let page = app.editor.page;
        inspector_command_frame(
            &mut app,
            &ctx,
            vec![
                key(Key::A, false),
                egui::Event::Text("Complete before command".into()),
                key(command, true),
            ],
        );
        for _ in 0..4 {
            inspector_command_frame(&mut app, &ctx, vec![]);
        }
        if command == Key::N {
            assert_eq!(app.editor.document.pages.len(), 2);
            assert_ne!(app.editor.page, page);
            app.editor.switch_page(page);
        } else {
            assert!(app.present);
        }
        assert_eq!(
            app.editor.current().controls[0].text,
            "Complete before command",
            "command={command:?}"
        );
        assert!(app.deferred_text_input.is_empty());
        app.editor.commit();
        let expected = app.editor.document.clone();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ordered.wireframe");
        ProjectStore.save(&path, &expected).unwrap();
        assert!(ProjectStore.load(&path).unwrap() == expected);
        if command == Key::N {
            app.editor.undo();
            assert_eq!(app.editor.document.pages.len(), 1);
            assert_eq!(
                app.editor.current().controls[0].text,
                "Complete before command"
            );
        }
        app.editor.undo();
        assert_eq!(app.editor.current().controls[0].text, "Button");
        app.editor.redo();
        if command == Key::N {
            app.editor.redo();
        }
        assert!(app.editor.document == expected);
    }
}

#[test]
fn inspector_input_before_new_project_enters_unsaved_confirmation() {
    for input in [
        egui::Event::Text("Retain this edit".into()),
        egui::Event::Paste("Retain this edit".into()),
        egui::Event::Ime(egui::ImeEvent::Commit("Retain this edit".into())),
    ] {
        let (ctx, mut app) = inspector_command_setup();
        app.editor.commit();
        app.editor.mark_saved();
        let original = app.editor.document.clone();
        inspector_command_frame(
            &mut app,
            &ctx,
            vec![key(Key::A, false), input, key(Key::N, false)],
        );
        for _ in 0..3 {
            inspector_command_frame(&mut app, &ctx, vec![]);
        }
        assert!(
            matches!(app.pending, Some(Pending::New)),
            "New must not discard a clean document's incoming edit"
        );
        assert_eq!(app.editor.current().controls[0].text, "Retain this edit");
        assert!(app.editor.dirty());
        app.pending = None; // Cancel the confirmation: retain the edited document.
        app.editor.commit();
        app.editor.undo();
        assert!(app.editor.document == original);
        app.editor.redo();
        assert_eq!(app.editor.current().controls[0].text, "Retain this edit");
    }
}

#[test]
fn inspector_caption_and_inline_links_preserve_panel_width() {
    for (requested_width, density) in [
        (220., 1.),
        (300., 1.),
        (400., 1.),
        (220., 2.),
        (300., 2.),
        (400., 2.),
    ] {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Arrow, 50., 50.);
        let id = *app.editor.selection.iter().next().unwrap();
        ctx.data_mut(|d| {
            d.insert_persisted(
                egui::Id::new("inspector"),
                egui::containers::panel::PanelState {
                    outer_rect: Rect::from_min_size(Pos2::ZERO, Vec2::new(requested_width, 700.)),
                },
            )
        });
        let mut time = 0.;
        let mut frame = |app: &mut WireframeApp, events| {
            time += 0.05;
            let mut width = 0.;
            let mut raw = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1360., 700.))),
                time: Some(time),
                events,
                ..Default::default()
            };
            raw.viewports
                .get_mut(&egui::ViewportId::ROOT)
                .unwrap()
                .native_pixels_per_point = Some(density);
            let mut output = ctx.run_ui(raw, |ui| {
                let right = ui.available_rect_before_wrap().right();
                app.inspector(ui);
                width = right - ui.available_rect_before_wrap().right();
            });
            output.textures_delta.clear();
            (width, output)
        };
        let mut last = frame(&mut app, vec![]);
        for _ in 0..8 {
            last = frame(&mut app, vec![]);
        }
        let baseline = last.0;
        assert!(
            (baseline - requested_width).abs() < 0.1,
            "requested {requested_width}, got {baseline}"
        );
        let nodes = &last
            .1
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes;
        let slider = nodes
            .iter()
            .find(|(_, n)| {
                n.role() == egui::accesskit::Role::Slider
                    && n.labelled_by().iter().any(|id| {
                        nodes.iter().any(|(other, n)| {
                            other == id && n.value().or_else(|| n.label()) == Some("Opacity")
                        })
                    })
            })
            .unwrap();
        let r = slider.1.bounds().unwrap();
        let at = Pos2::new((r.x0 + r.x1) as f32 / 2., (r.y0 + r.y1) as f32 / 2.);
        for pressed in [true, false] {
            frame(
                &mut app,
                vec![
                    egui::Event::PointerMoved(at),
                    egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Modifiers::NONE,
                    },
                ],
            );
        }
        for _ in 0..12 {
            last = frame(&mut app, vec![]);
        }
        assert!(
            (last.0 - baseline).abs() < 0.1,
            "initial pointer edit grew panel {baseline} -> {}",
            last.0
        );
        app.editor.begin("Opacity");
        app.editor
            .set_control_property(
                id,
                "background_opacity",
                wireframe::document::Value::Number(0.49),
            )
            .unwrap();
        app.editor.commit();
        for _ in 0..12 {
            last = frame(&mut app, vec![]);
        }
        println!("fractional opacity width {}", last.0);
        assert!(
            (last.0 - baseline).abs() < 1.,
            "fractional opacity grew panel {} -> {}",
            baseline,
            last.0
        );
        println!("baseline width {baseline}");
        let nodes = &last
            .1
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes;
        let r = nodes
            .iter()
            .find(|(_, n)| n.label() == Some("Edit caption"))
            .unwrap()
            .1
            .bounds()
            .unwrap();
        let at = Pos2::new((r.x0 + r.x1) as f32 / 2., (r.y0 + r.y1) as f32 / 2.);
        for pressed in [true, false] {
            frame(
                &mut app,
                vec![
                    egui::Event::PointerMoved(at),
                    egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Modifiers::NONE,
                    },
                ],
            );
        }
        for _ in 0..12 {
            last = frame(&mut app, vec![]);
        }
        println!("expanded caption width {}", last.0);
        assert!(
            last.1
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .any(|(_, n)| n.role() == egui::accesskit::Role::MultilineTextInput),
            "caption actually expanded"
        );
        assert!(
            (last.0 - baseline).abs() < 1.,
            "caption expansion grew panel {} -> {}",
            baseline,
            last.0
        );
        app.editor.begin("Caption");
        app.editor
            .set_control_property(
                id,
                "text",
                wireframe::document::Value::Text("Café → [Go](page:1)".into()),
            )
            .unwrap();
        app.editor.commit();
        for _ in 0..12 {
            last = frame(&mut app, vec![]);
        }
        println!(
            "width {requested_width} density {density}: inline link width {}",
            last.0
        );
        assert!(
            (last.0 - baseline).abs() < 1.,
            "inline link grew panel {} -> {}",
            baseline,
            last.0
        );
    }
}

#[test]
fn example_project_opens_four_editable_pages_as_a_fresh_unsaved_session() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 50., 50.);
    let old_session = app.editor.clipboard_session().to_owned();
    app.perform(Pending::Example, &ctx);
    assert!(app.file.is_none());
    assert_ne!(app.editor.clipboard_session(), old_session);
    assert_eq!(app.editor.document.title, "Northstar Studio");
    assert_eq!(
        app.editor
            .document
            .pages
            .iter()
            .map(|page| (page.id, page.name.as_str(), page.content.nodes.len()))
            .collect::<Vec<_>>(),
        vec![
            (1, "Landing", 7),
            (9, "Desktop contact", 11),
            (21, "Phone contact", 11),
            (33, "Tablet order", 11)
        ]
    );
    assert!(app.editor.document.assets.is_empty());
    assert!(app.editor.undo_name().is_none());
    assert!(app.editor.redo_name().is_none());
    assert!(app.editor.document.encode().is_ok());
}

#[test]
fn unnamed_project_status_does_not_claim_the_previous_file_was_saved() {
    for (action, expected) in [
        (Pending::New, "New project · not saved to a file"),
        (Pending::Example, "Example project · not saved to a file"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("previous.wireframe");
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Button, 50., 50.);
        app.file = Some(path.clone());
        assert!(app.save(false));
        let saved = std::fs::read(&path).unwrap();
        assert_eq!(app.status, "Saved to your computer");

        app.perform(action, &ctx);

        assert!(app.file.is_none());
        assert_eq!(app.status, expected);
        assert_eq!(std::fs::read(path).unwrap(), saved);
    }
}

#[test]
fn markup_shortcut_is_ordered_atomic_and_leaves_focused_text_unchanged() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 40., 50.);
    app.editor.add(Kind::Callout, 180., 50.);
    app.editor.select_all();
    let before = app.editor.document.clone();
    dispatch(&ctx, &mut app, vec![key(Key::K, true), key(Key::Z, false)]);
    assert_eq!(app.editor.document, before);
    app.editor.redo();
    assert!(app.editor.nodes().iter().all(|n| n.markup));
    app.editor.undo();
    let mut field = "Draft".to_owned();
    let output = ctx.run_ui(egui::RawInput::default(), |ui| {
        ui.add(TextInput::singleline(&mut field)).request_focus();
    });
    output.drop_without_applying_deltas();
    dispatch(&ctx, &mut app, vec![key(Key::K, true)]);
    assert_eq!(app.editor.document, before);
    assert_eq!(field, "Draft");
}

#[test]
fn markup_visibility_shortcut_is_distinct_from_classification_and_yields_to_text() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 40., 50.);
    let original = app.editor.document.clone();
    dispatch(&ctx, &mut app, vec![key(Key::K, false)]);
    assert!(!app.editor.markup_visible());
    assert_eq!(app.editor.document, original);
    dispatch(&ctx, &mut app, vec![key(Key::K, true)]);
    assert!(!app.editor.markup_visible());
    assert!(app.editor.nodes()[0].markup);
    assert!(app.editor.selection.is_empty());
    dispatch(&ctx, &mut app, vec![key(Key::K, false)]);
    assert!(app.editor.markup_visible());
    let mut field = "Draft".to_owned();
    ctx.run_ui(egui::RawInput::default(), |ui| {
        ui.add(TextInput::singleline(&mut field)).request_focus();
    })
    .drop_without_applying_deltas();
    dispatch(&ctx, &mut app, vec![key(Key::K, false)]);
    assert!(app.editor.markup_visible());
    assert_eq!(field, "Draft");
}
