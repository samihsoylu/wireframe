// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use std::sync::{LazyLock, Mutex};
use unicode_segmentation::UnicodeSegmentation;
use wireframe::{text_entry::layout::Engine, ui_text};
static ENGINE: LazyLock<Mutex<Engine>> = LazyLock::new(|| Mutex::new(Engine::default()));

#[test]
fn captions_shape_complete_source_and_truncate_only_at_graphemes() {
    let mut engine = ENGINE.lock().unwrap_or_else(|e| e.into_inner());
    for source in ["ABC שלום 👩‍💻", "واجهة 🇳🇱", "á 🇳🇱 👩‍💻 office ffi", ""]
    {
        let full = ui_text::prepare(&mut engine, source, 13., 400., 2., Color32::BLACK).unwrap();
        assert!(!full.truncated);
        assert_eq!(full.displayed, source);
        if !source.is_empty() {
            assert!(full.image.pixels.iter().any(|p| p.a() > 0));
        }
        for width in [1., 20., 40., 80.] {
            let small =
                ui_text::prepare(&mut engine, source, 13., width, 2., Color32::BLACK).unwrap();
            assert!(small.ink_width <= width);
            if small.truncated {
                let prefix = small.displayed.strip_suffix('…').unwrap();
                assert!(source.starts_with(prefix));
                assert!(
                    prefix.is_empty()
                        || source
                            .grapheme_indices(true)
                            .any(|(i, g)| i + g.len() == prefix.len())
                );
            }
        }
    }
}

#[test]
fn captions_bound_long_input_and_keep_line_breaks_out_of_layout() {
    let mut engine = ENGINE.lock().unwrap_or_else(|e| e.into_inner());
    let s = "👩‍💻".repeat(10000);
    let prepared = ui_text::prepare(&mut engine, &s, 13., 150., 2., Color32::BLACK).unwrap();
    assert!(prepared.truncated);
    assert!(prepared.displayed.len() < 1000);
    let lines = ui_text::prepare(
        &mut engine,
        "one\r\ntwo\nthree",
        13.,
        400.,
        2.,
        Color32::BLACK,
    )
    .unwrap();
    assert_eq!(lines.displayed, "one two three");
}

#[test]
fn accessibility_keeps_full_names_and_noneditable_roles() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let source = "ABC שלום 👩‍💻 ".repeat(40);
    let mut ids = vec![];
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(500., 300.))),
            ..Default::default()
        },
        |ui| {
            ids.push(
                ui_text::label_styled(ui, &source, 100., ui_text::CaptionStyle::panel_heading())
                    .id
                    .accesskit_id(),
            );
            ids.push(ui_text::label(ui, &source, 100.).id.accesskit_id());
            ids.push(
                ui_text::selectable(ui, true, &source, 100.)
                    .id
                    .accesskit_id(),
            );
        },
    );
    let nodes = &output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes;
    for id in ids {
        let node = &nodes.iter().find(|(i, _)| *i == id).unwrap().1;
        assert!(!matches!(
            node.role(),
            egui::accesskit::Role::TextInput | egui::accesskit::Role::MultilineTextInput
        ));
        assert_eq!(node.label().or(node.value()), Some(source.as_str()));
        assert!(node.bounds().unwrap().width() <= 100.);
    }
    output.textures_delta.clear();
}

#[test]
fn shaped_combo_retains_value_role_popup_selection_and_escape() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let names = ["ABC שלום 👩‍💻", "واجهة 🇳🇱"];
    let mut selected = 0usize;
    let mut run = |events| {
        let mut id = egui::Id::NULL;
        let mut rect = Rect::NOTHING;
        let mut output = ctx.run_ui(
            egui::RawInput {
                events,
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(600., 400.))),
                ..Default::default()
            },
            |ui| {
                ui.set_width(200.);
                let current = names[selected];
                let response = ui_text::combo_box(ui, "test", current, |ui| {
                    for (index, name) in names.iter().enumerate() {
                        if ui_text::selectable_value(ui, &mut selected, index, name).clicked() {
                            ui.close();
                        }
                    }
                })
                .response;
                id = response.id;
                rect = response.rect;
            },
        );
        output.textures_delta.clear();
        (id, rect, output)
    };
    let pointer = |p: Pos2, pressed| {
        vec![
            egui::Event::PointerMoved(p),
            egui::Event::PointerButton {
                pos: p,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    };
    let (id, rect, output) = run(vec![]);
    let nodes = &output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes;
    let node = &nodes
        .iter()
        .find(|(n, _)| *n == id.accesskit_id())
        .unwrap()
        .1;
    assert_eq!(node.role(), egui::accesskit::Role::ComboBox);
    assert_eq!(node.value(), Some(names[0]));
    assert!(rect.width() <= 200.);
    run(pointer(rect.center(), true));
    run(pointer(rect.center(), false));
    assert!(egui::ComboBox::is_open(&ctx, id));
    // Popup areas use their first frame to measure, then settle their position.
    let (_, _, output) = run(vec![]);
    let nodes = &output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes;
    let option = &nodes
        .iter()
        .find(|(_, n)| n.label() == Some(names[1]))
        .unwrap()
        .1;
    let b = option.bounds().unwrap();
    let p = Pos2::new(((b.x0 + b.x1) / 2.) as f32, ((b.y0 + b.y1) / 2.) as f32);
    run(pointer(p, true));
    run(pointer(p, false));
    let (_, _, output) = run(vec![]);
    let node = &output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|(n, _)| *n == id.accesskit_id())
        .unwrap()
        .1;
    assert_eq!(node.value(), Some(names[1]));
    run(pointer(rect.center(), true));
    run(pointer(rect.center(), false));
    run(vec![egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }]);
    assert!(!egui::ComboBox::is_open(&ctx, id));
    ctx.memory_mut(|m| m.request_focus(id));
    run(vec![egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }]);
    assert!(egui::ComboBox::is_open(&ctx, id));
    let (_, _, output) = run(vec![]);
    let first_id = output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some(names[0]))
        .unwrap()
        .0;
    run(vec![egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target_tree: egui::accesskit::TreeId::ROOT,
            target_node: first_id,
            data: None,
        },
    )]);
    assert!(!egui::ComboBox::is_open(&ctx, id));
    assert_eq!(selected, 0);
}

#[test]
fn styled_headings_keep_grapheme_truncation_and_regular_caption_ink() {
    let mut engine = ENGINE.lock().unwrap_or_else(|e| e.into_inner());
    let source = "Components café 👩‍💻 שלום";
    let regular = ui_text::prepare(&mut engine, source, 13., 400., 2., Color32::BLACK).unwrap();
    for style in [
        ui_text::CaptionStyle::panel_heading(),
        ui_text::CaptionStyle::selected_page(),
    ] {
        for width in [1., 20., 80., 400.] {
            let heading =
                ui_text::prepare_styled(&mut engine, source, style, width, 2., Color32::BLACK)
                    .unwrap();
            assert!(heading.ink_width <= width);
            if heading.truncated {
                let prefix = heading.displayed.strip_suffix('…').unwrap();
                assert!(source.starts_with(prefix));
                assert!(
                    prefix.is_empty()
                        || source
                            .grapheme_indices(true)
                            .any(|(i, g)| i + g.len() == prefix.len())
                );
            } else {
                assert_eq!(heading.displayed, source);
            }
        }
    }
    assert_eq!(
        ui_text::prepare(&mut engine, source, 13., 400., 2., Color32::BLACK)
            .unwrap()
            .image
            .pixels,
        regular.image.pixels
    );
}
