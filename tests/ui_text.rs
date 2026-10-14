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
