// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use eframe::egui::{
    self, Event, ImeEvent, Key, Modifiers, Pos2, Rect, Vec2,
    accesskit::{Action, ActionData, ActionRequest, Role, TextPosition, TextSelection, TreeId},
};
use std::sync::Mutex;
use unicode_segmentation::UnicodeSegmentation;
static SERIAL: Mutex<()> = Mutex::new(());

fn frame(
    ctx: &egui::Context,
    value: &mut String,
    events: Vec<Event>,
) -> (egui::Id, egui::FullOutput) {
    let mut id = egui::Id::NULL;
    let mut output = ctx.run_ui(
        egui::RawInput {
            events,
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(500., 300.))),
            ..Default::default()
        },
        |ui| {
            ui.set_width(400.);
            id = wireframe::text_entry::multiline(ui, value, 4, true).id;
        },
    );
    output.textures_delta.clear();
    (id, output)
}
fn key(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

#[test]
fn accessibility_retains_long_graphemes_without_invalid_lengths() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut value = format!("a{}\r\nשלום", "\u{301}".repeat(300));
    let original = value.clone();
    let (id, output) = frame(&ctx, &mut value, vec![]);
    let nodes = &output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes;
    let parent = &nodes
        .iter()
        .find(|(n, _)| *n == id.accesskit_id())
        .unwrap()
        .1;
    let mut rebuilt = String::new();
    for child in parent.children() {
        let node = &nodes.iter().find(|(n, _)| n == child).unwrap().1;
        let text = node.value().unwrap();
        let mut offset = 0;
        for &length in node.character_lengths() {
            assert!(length > 0);
            offset += usize::from(length);
            assert!(text.is_char_boundary(offset));
        }
        assert_eq!(offset, text.len());
        rebuilt.push_str(text);
    }
    assert_eq!(rebuilt, original);
    assert_eq!(value, original);
}

#[test]
fn accessibility_exposes_source_and_accepts_source_selection() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut value = "ABC שלום 123 👩‍💻\r\n".to_owned();
    let (id, output) = frame(&ctx, &mut value, vec![]);
    let nodes = &output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes;
    let parent = &nodes
        .iter()
        .find(|(n, _)| *n == id.accesskit_id())
        .unwrap()
        .1;
    assert_eq!(parent.role(), Role::MultilineTextInput);
    assert_eq!(parent.value(), Some(value.as_str()));
    let child_text: String = parent
        .children()
        .iter()
        .map(|id| {
            nodes
                .iter()
                .find(|(n, _)| n == id)
                .unwrap()
                .1
                .value()
                .unwrap_or("")
        })
        .collect();
    assert_eq!(child_text, value);
    let number = nodes
        .iter()
        .find(|(_, n)| n.role() == Role::TextRun && n.value().is_some_and(|v| v.contains("123")))
        .unwrap()
        .0;
    let number_value = nodes
        .iter()
        .find(|(n, _)| *n == number)
        .unwrap()
        .1
        .value()
        .unwrap();
    let character_index = number_value[..number_value.find("123").unwrap()]
        .graphemes(true)
        .count();
    let position = TextPosition {
        node: number,
        character_index,
    };
    frame(
        &ctx,
        &mut value,
        vec![
            Event::AccessKitActionRequest(ActionRequest {
                action: Action::SetTextSelection,
                target_tree: TreeId::ROOT,
                target_node: id.accesskit_id(),
                data: Some(ActionData::SetTextSelection(TextSelection {
                    anchor: position,
                    focus: position,
                })),
            }),
            Event::Text("|".into()),
        ],
    );
    assert_eq!(value, "ABC שלום |123 👩‍💻\r\n");
}

#[test]
fn pointer_position_and_unicode_backspace_reach_the_source_model() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut value = "ABC שלום 123".to_owned();
    let (_, output) = frame(&ctx, &mut value, vec![]);
    let node = &output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, n)| n.role() == Role::TextRun && n.value().is_some_and(|v| v.contains("123")))
        .unwrap()
        .1;
    let bounds = node.bounds().unwrap();
    assert!(bounds.width() > 0.);
    let text = node.value().unwrap();
    let character = text[..text.find("123").unwrap()].graphemes(true).count();
    let pos = Pos2::new(
        (bounds.x0 + node.character_positions().unwrap()[character] as f64 + 0.1) as f32,
        ((bounds.y0 + bounds.y1) / 2.) as f32,
    );
    for pressed in [true, false] {
        frame(
            &ctx,
            &mut value,
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
    frame(&ctx, &mut value, vec![Event::Text("|".into())]);
    assert_eq!(value, "ABC שלום |123");
    frame(
        &ctx,
        &mut value,
        vec![
            key(Key::A, Modifiers::COMMAND),
            Event::Paste("á 👩‍💻 🇳🇱".into()),
        ],
    );
    for expected in ["á 👩‍💻 ", "á 👩‍💻", "á ", "á", ""] {
        frame(&ctx, &mut value, vec![key(Key::Backspace, Modifiers::NONE)]);
        assert_eq!(value, expected);
    }
    for _ in 0..5 {
        frame(&ctx, &mut value, vec![key(Key::Z, Modifiers::COMMAND)]);
    }
    assert_eq!(value, "á 👩‍💻 🇳🇱");
}

#[test]
fn ime_preview_is_transient_and_commit_is_one_undo_step() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut value = "Hello ".to_owned();
    frame(&ctx, &mut value, vec![]);
    let (_, preview) = frame(
        &ctx,
        &mut value,
        vec![Event::Ime(ImeEvent::Preedit {
            text: "に".into(),
            active_range_chars: Some(0..1),
        })],
    );
    assert_eq!(value, "Hello ");
    assert!(preview.platform_output.ime.is_some());
    frame(
        &ctx,
        &mut value,
        vec![Event::Ime(ImeEvent::Preedit {
            text: "日本".into(),
            active_range_chars: Some(1..2),
        })],
    );
    frame(
        &ctx,
        &mut value,
        vec![Event::Ime(ImeEvent::Commit("日本".into()))],
    );
    assert_eq!(value, "Hello 日本");
    frame(&ctx, &mut value, vec![key(Key::Z, Modifiers::COMMAND)]);
    assert_eq!(value, "Hello ");
    frame(
        &ctx,
        &mut value,
        vec![
            Event::Ime(ImeEvent::Preedit {
                text: "未確定".into(),
                active_range_chars: None,
            }),
            key(Key::Escape, Modifiers::NONE),
        ],
    );
    assert_eq!(value, "Hello ");
    frame(
        &ctx,
        &mut value,
        vec![
            key(Key::A, Modifiers::COMMAND),
            Event::Paste("áZ".into()),
            key(Key::Home, Modifiers::NONE),
        ],
    );
    frame(
        &ctx,
        &mut value,
        vec![Event::Ime(ImeEvent::DeleteSurrounding {
            before_chars: 0,
            after_chars: 1,
        })],
    );
    assert_eq!(value, "Z");
}

#[test]
fn whole_field_accessibility_replacement_and_clipboard_keep_unicode() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut value = "old".to_owned();
    let (id, _) = frame(&ctx, &mut value, vec![]);
    frame(
        &ctx,
        &mut value,
        vec![Event::AccessKitActionRequest(ActionRequest {
            action: Action::SetValue,
            target_tree: TreeId::ROOT,
            target_node: id.accesskit_id(),
            data: Some(ActionData::Value("مرحبا\n👩‍💻".into())),
        })],
    );
    let (_, output) = frame(
        &ctx,
        &mut value,
        vec![key(Key::A, Modifiers::COMMAND), Event::Copy],
    );
    assert!(
        output
            .platform_output
            .commands
            .iter()
            .any(|c| matches!(c,egui::OutputCommand::CopyText(s) if s=="مرحبا\n👩‍💻"))
    );
    frame(&ctx, &mut value, vec![Event::Cut]);
    assert_eq!(value, "");
    frame(&ctx, &mut value, vec![key(Key::Z, Modifiers::COMMAND)]);
    assert_eq!(value, "مرحبا\n👩‍💻");
}

#[test]
fn singleline_retains_source_scrolls_and_leaves_form_keys_available() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut value = "ABC שלום 123 👩‍💻 ".repeat(15);
    let original = value.clone();
    let mut run = |events| {
        let mut id = egui::Id::NULL;
        let mut rect = Rect::NOTHING;
        let mut keys = (false, false);
        let mut output = ctx.run_ui(
            egui::RawInput {
                events,
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(500., 300.))),
                ..Default::default()
            },
            |ui| {
                let response = ui.add(
                    wireframe::text_entry::TextInput::singleline(&mut value)
                        .desired_width(140.)
                        .autofocus(true)
                        .hint_text("Search"),
                );
                id = response.id;
                rect = response.rect;
                keys = ui.input(|i| (i.key_pressed(Key::Enter), i.key_pressed(Key::ArrowDown)));
            },
        );
        output.textures_delta.clear();
        (id, rect, keys, output)
    };
    let (id, rect, _, output) = run(vec![]);
    assert!(wireframe::text_entry::is_focused(&ctx));
    let caret = output.platform_output.ime.unwrap().cursor_rect;
    assert!(caret.left() >= rect.left() && caret.right() <= rect.right());
    let nodes = &output.platform_output.accesskit_update.unwrap().nodes;
    let parent = &nodes
        .iter()
        .find(|(n, _)| *n == id.accesskit_id())
        .unwrap()
        .1;
    assert_eq!(parent.role(), Role::TextInput);
    assert_eq!(parent.value(), Some(original.as_str()));
    assert_eq!(parent.placeholder(), Some("Search"));
    let (_, _, keys, _) = run(vec![key(Key::ArrowDown, Modifiers::NONE)]);
    assert!(keys.1);
    assert!(wireframe::text_entry::is_focused(&ctx));
    let (_, _, keys, _) = run(vec![key(Key::Enter, Modifiers::NONE)]);
    assert!(keys.0);
    assert!(!wireframe::text_entry::is_focused(&ctx));
    assert_eq!(value, original);
}

#[test]
fn singleline_paste_keeps_line_endings_and_delete_keeps_graphemes() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let ctx = egui::Context::default();
    let mut value = String::new();
    let mut run = |events| {
        let mut out = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| {
                ui.add(wireframe::text_entry::TextInput::singleline(&mut value).autofocus(true));
            },
        );
        out.textures_delta.clear();
    };
    run(vec![]);
    run(vec![Event::Paste("שלום\r\n👩‍💻".into())]);
    run(vec![key(Key::Backspace, Modifiers::NONE)]);
    assert_eq!(value, "שלום\r\n");
}

#[test]
fn composition_keys_do_not_submit_a_singleline_form() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let ctx = egui::Context::default();
    let mut value = String::new();
    for events in [
        vec![],
        vec![
            Event::Ime(ImeEvent::Preedit {
                text: "に".into(),
                active_range_chars: None,
            }),
            key(Key::Enter, Modifiers::NONE),
            key(Key::ArrowDown, Modifiers::NONE),
        ],
    ] {
        let mut output = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| {
                ui.add(wireframe::text_entry::TextInput::singleline(&mut value).autofocus(true));
                assert!(!ui.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::ArrowDown)));
            },
        );
        output.textures_delta.clear();
    }
    assert!(value.is_empty());
    assert!(wireframe::text_entry::is_focused(&ctx));
}
