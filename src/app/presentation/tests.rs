// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::collections::HashMap;

fn fixture() -> (egui::Context, WireframeApp, [Id; 5]) {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    let landing = app.editor.page;
    let focused = app.editor.create_version("Focused").unwrap();
    app.editor.add_page();
    let contact = app.editor.page;
    let detailed = app.editor.create_version("Detailed").unwrap();
    app.editor.add_page();
    let phone = app.editor.page;
    app.editor.switch_page(landing);
    app.start_presentation();
    app.presentation_view.settings_open = false;
    (ctx, app, [landing, focused, contact, detailed, phone])
}
#[test]
fn native_route_contract_preserves_versions_forward_branch_and_document_undo() {
    let (_, mut app, [landing, focused, contact, detailed, phone]) = fixture();
    let document = app.editor.document.clone();
    let undo = app.editor.undo_name().map(str::to_owned);
    app.navigate_adjacent(true, false);
    assert_eq!(app.editor.surface_id(), detailed);
    app.navigate_history(false);
    assert_eq!(app.editor.surface_id(), focused);
    app.navigate_history(true);
    assert_eq!(app.editor.surface_id(), detailed);
    app.navigate_adjacent(false, true);
    assert_eq!(app.editor.surface_id(), contact);
    app.navigate_history(false);
    assert_eq!(app.editor.surface_id(), detailed);
    app.navigate_history(true);
    assert_eq!(app.editor.surface_id(), contact);
    app.navigate_history(false);
    app.navigate_page(phone);
    assert!(app.history_destination(true).is_none());
    app.navigate_history(true);
    assert_eq!(app.editor.page, phone);
    app.navigate_page(phone); // A repeated selection does not add a route entry.
    app.navigate_history(false);
    assert_eq!(app.editor.surface_id(), detailed);
    app.navigate_history(false);
    assert_eq!(app.editor.surface_id(), focused);
    app.navigate_history(false);
    assert_eq!(app.editor.page, landing);
    assert!(app.history_destination(false).is_none());
    assert_eq!(app.editor.document, document);
    assert_eq!(app.editor.undo_name(), undo.as_deref());
    assert!(app.editor.redo_name().is_none());
}
#[test]
fn ordered_versions_cross_page_boundaries_and_do_not_wrap() {
    let (_, mut app, [landing, focused, contact, detailed, phone]) = fixture();
    app.navigate_page(phone);
    for expected in [detailed, contact, focused, landing, landing] {
        app.navigate_adjacent(false, true);
        assert_eq!(app.editor.surface_id(), expected);
    }
    for expected in [focused, contact, detailed, phone, phone] {
        app.navigate_adjacent(true, true);
        assert_eq!(app.editor.surface_id(), expected);
    }
    app.start_presentation();
    assert!(app.history_destination(false).is_none());
    assert!(app.history_destination(true).is_none());
    assert_eq!(app.editor.surface_id(), phone);
}
#[test]
fn history_uses_identity_after_reorder_and_skips_deleted_pages() {
    let (_, mut app, [landing, focused, contact, detailed, phone]) = fixture();
    app.navigate_page(contact);
    app.navigate_page(phone);
    app.editor.document.pages.swap(0, 2);
    app.navigate_history(false);
    assert_eq!(app.editor.surface_id(), detailed);
    app.navigate_history(false);
    assert_eq!(app.editor.surface_id(), focused);
    app.editor.document.pages.retain(|p| p.id != contact);
    app.navigate_history(true);
    assert_eq!(app.editor.page, phone);
    app.navigate_history(false);
    assert_eq!(app.editor.page, landing);
    assert!(app.history_destination(false).is_none());
}
#[test]
fn stale_alternate_falls_back_and_invalid_routes_leave_history_intact() {
    let (ctx, mut app, [landing, focused, contact, _, _]) = fixture();
    app.navigate_page(contact);
    app.editor
        .document
        .pages
        .iter_mut()
        .find(|p| p.id == landing)
        .unwrap()
        .alternates
        .retain(|a| a.id != focused);
    app.navigate_history(false);
    assert_eq!(app.editor.surface_id(), landing);
    let before = app.presentation_history.entries.clone();
    let cursor = app.presentation_history.cursor;
    let document = app.editor.document.clone();
    app.follow_target("page:999999", &ctx);
    app.visit_destination(Destination {
        page: contact,
        alternate: Some(999998),
    });
    assert_eq!(app.presentation_history.entries, before);
    assert_eq!(app.presentation_history.cursor, cursor);
    assert_eq!(app.editor.document, document);
    app.navigate_history(true);
    assert_eq!(app.editor.page, contact);
}
fn key(key: Key, modifiers: Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}
fn toolbar_frame(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    events: Vec<egui::Event>,
) -> HashMap<String, (Pos2, bool)> {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 850.))),
            events,
            ..Default::default()
        },
        |ui| {
            app.shortcuts(ui.ctx());
            app.presentation_toolbar(ui);
        },
    );
    let labels = output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter_map(|(_, n)| {
            let b = n.bounds()?;
            Some((
                n.label()?.to_owned(),
                (
                    Pos2::new(((b.x0 + b.x1) / 2.) as f32, ((b.y0 + b.y1) / 2.) as f32),
                    n.is_disabled(),
                ),
            ))
        })
        .collect();
    output.textures_delta.clear();
    labels
}
fn click(ctx: &egui::Context, app: &mut WireframeApp, label: &str) {
    if matches!(label, "Back" | "Forward") && !egui::Popup::is_any_open(ctx) {
        click(ctx, app, "Versions and navigation");
    }
    let mut labels = HashMap::new();
    for _ in 0..3 {
        labels = toolbar_frame(ctx, app, vec![]);
    }
    let (pos, disabled) = labels
        .get(label)
        .unwrap_or_else(|| panic!("Missing {label}: {:?}", labels.keys()));
    assert!(!disabled, "Disabled {label}");
    for pressed in [true, false] {
        toolbar_frame(
            ctx,
            app,
            vec![
                egui::Event::PointerMoved(*pos),
                egui::Event::PointerButton {
                    pos: *pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
}
#[test]
fn closing_navigator_keeps_presentation_destination_history_and_document() {
    let (ctx, mut app, [_, focused, contact, detailed, _]) = fixture();
    ctx.enable_accesskit();
    app.navigate_page(contact);
    // Hide the other panes to identify the navigator close control unambiguously.
    app.presentation_view.session.toggle(Aid::Notes);
    app.presentation_view.session.toggle(Aid::Comments);
    let document = app.editor.document.clone();
    let undo = app.editor.undo_name().map(str::to_owned);
    let history = app.presentation_history.entries.clone();
    let cursor = app.presentation_history.cursor;
    click(&ctx, &mut app, "Hide Navigator");
    assert!(
        app.present,
        "Closing the navigator must not exit presentation"
    );
    assert!(!app.presentation_view.session.settings().navigator);
    assert_eq!(app.editor.surface_id(), detailed);
    assert_eq!(app.presentation_history.entries, history);
    assert_eq!(app.presentation_history.cursor, cursor);
    assert_eq!(app.editor.document, document);
    assert_eq!(app.editor.undo_name(), undo.as_deref());
    toolbar_frame(&ctx, &mut app, vec![key(Key::J, Modifiers::NONE)]);
    assert!(app.presentation_view.session.settings().navigator);
    toolbar_frame(&ctx, &mut app, vec![key(Key::ArrowLeft, Modifiers::ALT)]);
    assert_eq!(app.editor.surface_id(), focused);
    toolbar_frame(&ctx, &mut app, vec![key(Key::Escape, Modifiers::NONE)]);
    assert!(!app.present);
}

#[test]
fn presentation_thumbnails_are_independent_preserve_document_and_survive_reentry() {
    let (ctx, mut app, [_, focused, contact, detailed, _]) = fixture();
    ctx.enable_accesskit();
    app.thumbnails = false;
    let document = app.editor.document.clone();
    let undo = app.editor.undo_name().map(str::to_owned);
    let settings = app.presentation_view.session.settings().clone();
    click(&ctx, &mut app, "Thumbnails");
    assert!(app.presentation_view.thumbnails);
    assert!(
        !app.thumbnails,
        "Presentation must not change the editor view mode"
    );
    assert_eq!(app.presentation_view.session.settings(), &settings);
    let caption = app.editor.document.live_page(contact).unwrap().name.clone();
    click(&ctx, &mut app, &caption);
    assert_eq!(app.editor.surface_id(), detailed);
    click(&ctx, &mut app, "Back");
    assert_eq!(app.editor.surface_id(), focused);
    toolbar_frame(&ctx, &mut app, vec![key(Key::Escape, Modifiers::NONE)]);
    app.start_presentation();
    app.presentation_view.settings_open = false;
    assert!(
        app.presentation_view.thumbnails,
        "Reentry retains presentation view mode"
    );
    assert!(!app.thumbnails);
    click(&ctx, &mut app, "List");
    assert!(!app.presentation_view.thumbnails);
    assert_eq!(app.editor.document, document);
    assert_eq!(app.editor.undo_name(), undo.as_deref());
}

#[test]
fn toolbar_and_shortcuts_share_route_history() {
    let (ctx, mut app, [_, focused, contact, detailed, phone]) = fixture();
    ctx.enable_accesskit();
    for _ in 0..3 {
        toolbar_frame(&ctx, &mut app, vec![]);
    }
    let labels = toolbar_frame(&ctx, &mut app, vec![]);
    assert!(labels["Prev ↑"].1);
    click(&ctx, &mut app, "Versions and navigation");
    let labels = toolbar_frame(&ctx, &mut app, vec![]);
    assert!(labels["Back"].1 && labels["Forward"].1);
    toolbar_frame(&ctx, &mut app, vec![key(Key::Escape, Modifiers::NONE)]);
    click(&ctx, &mut app, "Next ↓");
    assert_eq!(app.editor.surface_id(), detailed);
    click(&ctx, &mut app, "Back");
    assert_eq!(app.editor.surface_id(), focused);
    toolbar_frame(&ctx, &mut app, vec![key(Key::ArrowRight, Modifiers::ALT)]);
    assert_eq!(app.editor.surface_id(), detailed);
    toolbar_frame(
        &ctx,
        &mut app,
        vec![key(Key::ArrowUp, Modifiers::ALT | Modifiers::SHIFT)],
    );
    assert_eq!(app.editor.surface_id(), contact);
    click(&ctx, &mut app, "Back");
    assert_eq!(app.editor.surface_id(), detailed);
    click(&ctx, &mut app, "Forward");
    assert_eq!(app.editor.surface_id(), contact);
    click(&ctx, &mut app, "Versions and navigation");
    click(&ctx, &mut app, "Detailed");
    assert_eq!(app.editor.surface_id(), detailed);
    toolbar_frame(&ctx, &mut app, vec![key(Key::ArrowDown, Modifiers::ALT)]);
    assert_eq!(app.editor.page, phone);
    toolbar_frame(&ctx, &mut app, vec![key(Key::ArrowLeft, Modifiers::NONE)]);
    assert_eq!(app.editor.surface_id(), detailed);
    toolbar_frame(&ctx, &mut app, vec![key(Key::ArrowRight, Modifiers::NONE)]);
    assert_eq!(app.editor.page, phone);
}
#[test]
fn focused_text_and_modal_inputs_do_not_navigate() {
    let (ctx, mut app, [_, focused, _, _, _]) = fixture();
    let id = egui::Id::new("presentation-input");
    let mut text = "some text".to_owned();
    for _ in 0..2 {
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.add(egui::TextEdit::singleline(&mut text).id(id))
                .request_focus();
        });
        output.textures_delta.clear();
    }
    assert!(ctx.text_edit_focused());
    assert!(!app.presentation_shortcut(&ctx, Key::ArrowDown, Modifiers::ALT));
    assert_eq!(app.editor.surface_id(), focused);
    ctx.memory_mut(|m| m.surrender_focus(id));
    app.pending = Some(Pending::New);
    let mut output = ctx.run_ui(
        egui::RawInput {
            events: vec![key(Key::ArrowDown, Modifiers::ALT)],
            ..Default::default()
        },
        |ui| app.shortcuts(ui.ctx()),
    );
    output.textures_delta.clear();
    assert_eq!(app.editor.surface_id(), focused);
}

#[test]
fn escape_closes_versions_menu_before_leaving_presentation() {
    let (ctx, mut app, _) = fixture();
    ctx.enable_accesskit();
    click(&ctx, &mut app, "Versions and navigation");
    assert!(egui::Popup::is_any_open(&ctx));
    toolbar_frame(&ctx, &mut app, vec![key(Key::Escape, Modifiers::NONE)]);
    assert!(
        app.present,
        "Escape in the open menu must not exit presentation"
    );
    assert!(!egui::Popup::is_any_open(&ctx));
    toolbar_frame(&ctx, &mut app, vec![key(Key::Escape, Modifiers::NONE)]);
    assert!(!app.present);
}

#[test]
fn retained_pages_are_skipped_in_presentation_routes_and_history() {
    let (_, mut app, [landing, _, contact, _, phone]) = fixture();
    app.navigate_page(contact);
    app.navigate_page(phone);
    app.editor
        .document
        .pages
        .iter_mut()
        .find(|p| p.id == contact)
        .unwrap()
        .trashed = true;
    app.editor.document.validate().unwrap();
    let document = app.editor.document.clone();
    app.navigate_history(false);
    assert_eq!(app.editor.page, landing);
    app.navigate_adjacent(true, false);
    assert_eq!(app.editor.page, phone);
    app.navigate_page(contact);
    assert_eq!(app.editor.page, phone);
    assert_eq!(app.editor.document, document);
}

#[test]
fn permanently_removed_link_is_non_navigable_and_does_not_change_route_history() {
    let (ctx, mut app, [landing, _, contact, _, _]) = fixture();
    app.navigate_page(contact);
    app.editor.trash_pages(&[landing]).unwrap();
    app.editor.permanently_remove_pages(&[landing]).unwrap();
    let document = app.editor.document.clone();
    let history = app.presentation_history.entries.clone();
    let cursor = app.presentation_history.cursor;
    app.follow_target(&format!("page:{landing}"), &ctx);
    assert_eq!(app.editor.page, contact);
    assert_eq!(app.editor.document, document);
    assert_eq!(app.presentation_history.entries, history);
    assert_eq!(app.presentation_history.cursor, cursor);
    assert!(!app.status.is_empty());
}

fn interface_frame(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    events: Vec<egui::Event>,
) -> HashMap<String, Pos2> {
    interface_frame_at(ctx, app, None, events)
}
fn interface_frame_at(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    time: Option<f64>,
    events: Vec<egui::Event>,
) -> HashMap<String, Pos2> {
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200., 900.))),
        time,
        events,
        ..Default::default()
    };
    let mut output = interface_output(ctx, app, input);
    let labels = output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter_map(|(_, n)| {
            let b = n.bounds()?;
            Some((
                n.label()?.to_owned(),
                Pos2::new(((b.x0 + b.x1) / 2.) as f32, ((b.y0 + b.y1) / 2.) as f32),
            ))
        })
        .collect();
    output.textures_delta.clear();
    labels
}
fn interface_output(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    mut input: egui::RawInput,
) -> egui::FullOutput {
    app.order_text_input(ctx, &mut input);
    ctx.run_ui(input, |ui| {
        app.prepare_page_notes(ui.ctx());
        app.shortcuts(ui.ctx());
        app.sync_viewport();
        if app.present {
            app.presentation_toolbar(ui);
        }
        app.canvas(ui);
        app.presentation_dialog(ui.ctx());
        app.remember_presentation();
    })
}
fn interface_click(ctx: &egui::Context, app: &mut WireframeApp, label: &str) {
    let mut labels = HashMap::new();
    for _ in 0..3 {
        labels = interface_frame(ctx, app, vec![]);
    }
    let pos = *labels
        .get(label)
        .unwrap_or_else(|| panic!("Missing {label}: {:?}", labels.keys()));
    for pressed in [true, false] {
        interface_frame(
            ctx,
            app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
    interface_frame(ctx, app, vec![]);
}
#[test]
fn entry_modal_presets_shortcuts_and_layered_escape_are_connected() {
    let (ctx, mut app, _) = fixture();
    ctx.enable_accesskit();
    app.start_presentation();
    let before = app.editor.document.clone();
    let undo = app.editor.undo_name().map(str::to_owned);
    assert!(app.presentation_view.settings_open);
    interface_click(&ctx, &mut app, "User Test");
    assert!(!app.presentation_view.session.settings().navigator);
    assert_eq!(
        app.presentation_view.session.settings().zoom,
        Zoom::ActualSize
    );
    interface_click(&ctx, &mut app, "Close Settings");
    assert!(!app.presentation_view.settings_open);
    interface_frame(&ctx, &mut app, vec![key(Key::B, Modifiers::NONE)]);
    assert!(app.presentation_view.session.settings().big_cursor);
    assert_eq!(
        app.presentation_view.session.settings().preset,
        Preset::Custom
    );
    interface_frame(&ctx, &mut app, vec![key(Key::S, Modifiers::NONE)]);
    assert!(app.presentation_view.settings_open);
    interface_frame(&ctx, &mut app, vec![key(Key::Escape, Modifiers::NONE)]);
    assert!(app.present && !app.presentation_view.settings_open);
    interface_frame(&ctx, &mut app, vec![key(Key::Escape, Modifiers::NONE)]);
    assert!(!app.present);
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.undo_name(), undo.as_deref());
}
#[test]
fn presentation_notes_keep_shortcut_text_cancel_and_save_to_original_surface() {
    let (ctx, mut app, [_, focused, contact, _, _]) = fixture();
    ctx.enable_accesskit();
    let before = app.editor.document.clone();
    interface_click(&ctx, &mut app, "Click here to edit notes.");
    interface_frame(&ctx, &mut app, vec![egui::Event::Text("jnclbmzrus".into())]);
    for k in [
        Key::J,
        Key::N,
        Key::C,
        Key::L,
        Key::B,
        Key::M,
        Key::Z,
        Key::R,
        Key::U,
        Key::S,
    ] {
        assert!(!app.presentation_shortcut(&ctx, k, Modifiers::NONE));
    }
    assert_eq!(
        app.presentation_view.session.settings().preset,
        Preset::Review
    );
    interface_click(&ctx, &mut app, "Cancel");
    assert_eq!(app.editor.document, before);
    interface_click(&ctx, &mut app, "Click here to edit notes.");
    interface_frame(
        &ctx,
        &mut app,
        vec![egui::Event::Text("Review notes".into())],
    );
    interface_click(&ctx, &mut app, "Save");
    assert_eq!(app.editor.document.surface_notes(focused), "Review notes");
    app.navigate_page(contact);
    assert_ne!(
        app.editor.document.surface_notes(app.editor.surface_id()),
        "Review notes"
    );
    app.editor.undo();
    assert_eq!(app.editor.document, before);
}
#[test]
fn advanced_cancel_and_accept_persist_without_document_edits() {
    let (ctx, mut app, [landing, _, _, _, _]) = fixture();
    ctx.enable_accesskit();
    let dir = tempfile::tempdir().unwrap();
    app.file = Some(dir.path().join("study.wireframe"));
    app.presentation_view.preferences.load(dir.path()).unwrap();
    app.start_presentation();
    let before = app.editor.document.clone();
    interface_click(&ctx, &mut app, "Advanced…");
    interface_click(&ctx, &mut app, "Select None");
    interface_click(&ctx, &mut app, "Cancel");
    assert!(app.presentation_view.session.settings().excluded.is_empty());
    interface_click(&ctx, &mut app, "Advanced…");
    interface_click(&ctx, &mut app, "Select None");
    interface_click(&ctx, &mut app, "OK");
    assert!(
        app.presentation_view
            .session
            .settings()
            .excluded
            .contains(&landing)
    );
    assert_eq!(
        app.presentation_view.session.settings().preset,
        Preset::Review
    );
    app.present = false;
    app.remember_presentation();
    app.start_presentation();
    assert!(
        app.presentation_view
            .session
            .settings()
            .excluded
            .contains(&landing)
    );
    assert_eq!(app.editor.document, before);
}

#[test]
fn navigator_actions_fit_narrow_panels_at_each_display_scale() {
    for density in [1., 1.25, 1.5, 1.75, 2.] {
        for width in [130_f32, 131., 159., 160., 161., 200., 300.] {
            let (ctx, mut app, _) = fixture();
            ctx.enable_accesskit();
            ctx.set_pixels_per_point(density);
            app.presentation_view
                .session
                .set_panel_sizes(width, 300., 300.)
                .unwrap();
            let before = app.editor.document.clone();
            for _ in 0..4 {
                interface_frame(&ctx, &mut app, vec![]);
            }
            let output = interface_output(
                &ctx,
                &mut app,
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200., 900.))),
                    ..Default::default()
                },
            );
            let nodes = &output
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes;
            let mut end = 0.;
            for name in ["Prev ↑", "Next ↓", "Hide Navigator"] {
                let node = &nodes
                    .iter()
                    .find(|(_, n)| n.label() == Some(name))
                    .unwrap()
                    .1;
                let bounds = node.bounds().unwrap();
                assert!(
                    bounds.x0 >= end,
                    "Actions overlap at {width}/{density}: {name}"
                );
                assert!(
                    bounds.x1 <= f64::from(width) + 0.6,
                    "Action escapes panel at {width}/{density}: {name} {bounds:?}"
                );
                end = bounds.x1;
            }
            let id = egui::Id::new((
                "presentation-navigator",
                &Option::<String>::None,
                width.to_bits(),
            ));
            let actual = egui::containers::panel::PanelState::load(&ctx, id)
                .unwrap()
                .size()
                .x;
            assert!(
                (actual - width).abs() < 1.,
                "Panel expands at {width}/{density}: {actual}"
            );
            assert_eq!(
                app.presentation_view.session.settings().navigator_width,
                width
            );
            assert_eq!(app.editor.document, before);
            output.drop_without_applying_deltas();
        }
    }
}

#[test]
fn resizing_navigator_updates_custom_settings_without_touching_document() {
    let (ctx, mut app, _) = fixture();
    ctx.enable_accesskit();
    for _ in 0..4 {
        interface_frame(&ctx, &mut app, vec![]);
    }
    let before = app.editor.document.clone();
    let old = app.presentation_view.session.settings().navigator_width;
    let panel_id = egui::Id::new((
        "presentation-navigator",
        &Option::<String>::None,
        old.to_bits(),
    ));
    let panel_width = egui::containers::panel::PanelState::load(&ctx, panel_id)
        .unwrap()
        .size()
        .x;
    assert!(
        (panel_width - old).abs() < 1.,
        "Navigator contents must fit the requested width: {panel_width} vs {old}"
    );
    let from = Pos2::new(app.canvas_rect.left() - 1., app.canvas_rect.center().y);
    interface_frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerMoved(from),
            egui::Event::PointerButton {
                pos: from,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    for delta in [30., 60., 90.] {
        interface_frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(from + Vec2::new(delta, 0.))],
        );
    }
    let to = from + Vec2::new(90., 0.);
    interface_frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerButton {
            pos: to,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    );
    interface_frame(&ctx, &mut app, vec![]);
    assert!(app.presentation_view.session.settings().navigator_width > old + 60.);
    assert_eq!(
        app.presentation_view.session.settings().preset,
        Preset::Custom
    );
    assert_eq!(app.editor.document, before);
}

#[test]
fn presentation_link_activates_on_press_and_release_does_not_add_a_second_route() {
    let (ctx, mut app, [_, _, contact, _, _]) = fixture();
    ctx.enable_accesskit();
    app.editor.add(Kind::Button, 20., 20.);
    let id = *app.editor.selection.iter().next().unwrap();
    app.editor
        .set_control_property(
            id,
            "link",
            wireframe::document::Value::Link(wireframe::document::Link::Page(contact)),
        )
        .unwrap();
    for _ in 0..3 {
        interface_frame(&ctx, &mut app, vec![]);
    }
    let before = app.editor.document.clone();
    let point = app.screen([30., 30.]);
    interface_frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerMoved(point),
            egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    assert_eq!(app.editor.page, contact);
    let entries = app.presentation_history.entries.clone();
    interface_frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert_eq!(app.presentation_history.entries, entries);
    assert_eq!(app.editor.document, before);
    app.navigate_history(false);
    for _ in 0..2 {
        interface_frame(&ctx, &mut app, vec![]);
    }
    let point = app.screen([30., 30.]);
    interface_frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerMoved(point),
            egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    assert_eq!(
        app.editor.page, contact,
        "coalesced press/release must still activate the link"
    );
}

fn pointer_button(pos: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn presentation_pan_waits_for_hold_uses_integer_delta_and_stops_on_release() {
    let (ctx, mut app, _) = fixture();
    ctx.enable_accesskit();
    for time in [0., 0.01, 0.02] {
        interface_frame_at(&ctx, &mut app, Some(time), vec![]);
    }
    let document = app.editor.document.clone();
    let undo = app.editor.undo_name().map(str::to_owned);
    let start = app.canvas_rect.center();
    let before = app.pan;
    interface_frame_at(
        &ctx,
        &mut app,
        Some(1.),
        vec![
            egui::Event::PointerMoved(start),
            pointer_button(start, true),
        ],
    );
    let moved = start + Vec2::new(20.9, -15.9);
    interface_frame_at(
        &ctx,
        &mut app,
        Some(1.14),
        vec![egui::Event::PointerMoved(moved)],
    );
    assert_eq!(app.pan, before, "movement before 150 ms must not pan");
    interface_frame_at(&ctx, &mut app, Some(1.151), vec![]);
    assert_eq!(app.pan, before + Vec2::new(20., -15.));
    let moved = start + Vec2::new(40.9, -30.9);
    interface_frame_at(
        &ctx,
        &mut app,
        Some(1.2),
        vec![egui::Event::PointerMoved(moved)],
    );
    assert_eq!(app.pan, before + Vec2::new(40., -30.));
    interface_frame_at(
        &ctx,
        &mut app,
        Some(1.21),
        vec![pointer_button(moved, false)],
    );
    interface_frame_at(
        &ctx,
        &mut app,
        Some(1.5),
        vec![egui::Event::PointerMoved(start)],
    );
    assert_eq!(app.pan, before + Vec2::new(40., -30.));
    assert_eq!(app.editor.document, document);
    assert_eq!(app.editor.undo_name(), undo.as_deref());
}

#[test]
fn presentation_pan_does_not_rearm_after_settings_interrupts_a_held_press() {
    let (ctx, mut app, _) = fixture();
    ctx.enable_accesskit();
    for time in [0., 0.01, 0.02] {
        interface_frame_at(&ctx, &mut app, Some(time), vec![]);
    }
    let start = app.canvas_rect.center();
    interface_frame_at(
        &ctx,
        &mut app,
        Some(1.),
        vec![
            egui::Event::PointerMoved(start),
            pointer_button(start, true),
        ],
    );
    app.presentation_view.settings_open = true;
    interface_frame_at(&ctx, &mut app, Some(1.1), vec![]);
    app.presentation_view.settings_open = false;
    interface_frame_at(&ctx, &mut app, Some(1.2), vec![]);
    let before = app.pan;
    interface_frame_at(
        &ctx,
        &mut app,
        Some(1.5),
        vec![egui::Event::PointerMoved(start + Vec2::new(40., 20.))],
    );
    assert_eq!(
        app.pan, before,
        "closing settings must not invent another mouse-down"
    );
}

#[test]
fn presentation_pan_does_not_carry_its_origin_to_another_page() {
    let (ctx, mut app, [_, _, contact, _, _]) = fixture();
    ctx.enable_accesskit();
    for time in [0., 0.01, 0.02] {
        interface_frame_at(&ctx, &mut app, Some(time), vec![]);
    }
    let start = app.canvas_rect.center();
    interface_frame_at(
        &ctx,
        &mut app,
        Some(1.),
        vec![
            egui::Event::PointerMoved(start),
            pointer_button(start, true),
        ],
    );
    interface_frame_at(
        &ctx,
        &mut app,
        Some(1.2),
        vec![egui::Event::PointerMoved(start + Vec2::new(30., 10.))],
    );
    app.navigate_page(contact);
    interface_frame_at(&ctx, &mut app, Some(1.3), vec![]);
    let centered = app.pan;
    interface_frame_at(
        &ctx,
        &mut app,
        Some(1.5),
        vec![egui::Event::PointerMoved(start + Vec2::new(60., 20.))],
    );
    assert_eq!(
        app.pan, centered,
        "a held press from the previous page must end at navigation"
    );
}

fn hint_frame(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    time: f64,
    alt: bool,
    mut events: Vec<egui::Event>,
) -> Vec<Color32> {
    events.insert(
        0,
        egui::Event::ModifiersChanged(Modifiers {
            alt,
            ..Modifiers::NONE
        }),
    );
    let mut output = interface_output(
        ctx,
        app,
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200., 900.))),
            time: Some(time),
            events,
            ..Default::default()
        },
    );
    let mut colors = Vec::new();
    fn collect(shape: &egui::Shape, colors: &mut Vec<Color32>) {
        match shape {
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, colors);
                }
            }
            egui::Shape::Rect(rect) if rect.fill.a() > 0 && rect.fill.a() < 255 => {
                colors.push(rect.fill)
            }
            _ => {}
        }
    }
    for shape in &output.shapes {
        collect(&shape.shape, &mut colors);
    }
    output.textures_delta.clear();
    colors
}
fn link_fixture() -> (egui::Context, WireframeApp, Id) {
    let (ctx, mut app, [_, _, contact, _, _]) = fixture();
    app.presentation_view.session.set_aid(Aid::BigCursor, false);
    for (x, target) in [(20., contact), (220., 999999)] {
        app.editor.add(Kind::Button, x, 20.);
        let id = *app.editor.selection.iter().next().unwrap();
        app.editor
            .set_control_property(
                id,
                "link",
                wireframe::document::Value::Link(wireframe::document::Link::Page(target)),
            )
            .unwrap();
    }
    for time in [0., 0.01, 0.02] {
        hint_frame(&ctx, &mut app, time, false, vec![]);
    }
    (ctx, app, contact)
}
#[test]
fn presentation_link_hints_are_transient_fills_and_distinguish_missing_pages() {
    let (ctx, mut app, _) = link_fixture();
    let before = app.editor.document.clone();
    let undo = app.editor.undo_name().map(str::to_owned);
    let blue = Color32::from_rgba_unmultiplied(61, 155, 232, 204);
    let red = Color32::from_rgba_unmultiplied(224, 102, 102, 204);
    assert!(!hint_frame(&ctx, &mut app, 1., false, vec![]).contains(&blue));
    hint_frame(&ctx, &mut app, 2., true, vec![]);
    let colors = hint_frame(&ctx, &mut app, 2.1, true, vec![]);
    assert_eq!(colors.iter().filter(|c| **c == blue).count(), 1);
    assert_eq!(colors.iter().filter(|c| **c == red).count(), 1);
    assert!(hint_frame(&ctx, &mut app, 8., true, vec![]).contains(&blue));
    hint_frame(&ctx, &mut app, 9., false, vec![]);
    assert!(
        hint_frame(&ctx, &mut app, 9.2, false, vec![])
            .contains(&Color32::from_rgba_unmultiplied(61, 155, 232, 102))
    );
    assert!(!hint_frame(&ctx, &mut app, 9.5, false, vec![]).contains(&blue));
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.undo_name(), undo.as_deref());
}

#[test]
fn rotated_presentation_hint_tracks_framed_text_without_changing_document() {
    use wireframe::document::Value;
    for orientation in ["left", "right", "diagonal"] {
        let (ctx, mut app, _) = fixture();
        app.editor.add(Kind::Label, 50., 50.);
        let id = *app.editor.selection.iter().next().unwrap();
        app.editor
            .set_control_property(
                id,
                "text",
                Value::Text("[Order target](https://example.com)".into()),
            )
            .unwrap();
        app.editor
            .set_control_property(id, "text_orientation", Value::Choice(orientation.into()))
            .unwrap();
        app.presentation_view.session.set_aid(Aid::BigCursor, false);
        let before = app.editor.document.clone();
        let undo = app.editor.undo_name().map(str::to_owned);
        for time in [0., 0.01, 0.02, 1.] {
            hint_frame(&ctx, &mut app, time, true, vec![]);
        }
        let output = interface_output(
            &ctx,
            &mut app,
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200., 900.))),
                time: Some(1.1),
                events: vec![egui::Event::ModifiersChanged(Modifiers {
                    alt: true,
                    ..Modifiers::NONE
                })],
                ..Default::default()
            },
        );
        let color = Color32::from_rgba_unmultiplied(61, 155, 232, 204);
        fn paths(shape: &egui::Shape, color: Color32, out: &mut Vec<Vec<Pos2>>) {
            match shape {
                egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| paths(s, color, out)),
                egui::Shape::Path(path) if path.fill == color => out.push(path.points.clone()),
                _ => {}
            }
        }
        let mut hints = Vec::new();
        for shape in &output.shapes {
            paths(&shape.shape, color, &mut hints);
        }
        let scene = app.editor.rendered_scene(app.editor.surface_id()).unwrap();
        let expected: Vec<_> = scene.links[0].quad.unwrap().map(|p| app.screen(p)).to_vec();
        assert_eq!(hints, vec![expected], "{orientation}");
        assert_eq!(app.editor.document, before);
        assert_eq!(app.editor.undo_name(), undo.as_deref());
    }
}
#[test]
fn presentation_blank_click_flashes_but_disabled_hints_do_not_block_links() {
    let (ctx, mut app, contact) = link_fixture();
    let blank = app.canvas_rect.right_bottom() - Vec2::splat(15.);
    hint_frame(
        &ctx,
        &mut app,
        1.,
        false,
        vec![
            egui::Event::PointerMoved(blank),
            pointer_button(blank, true),
            pointer_button(blank, false),
        ],
    );
    let colors = hint_frame(&ctx, &mut app, 1.2, false, vec![]);
    assert!(colors.contains(&Color32::from_rgba_unmultiplied(61, 155, 232, 102)));
    assert!(
        !hint_frame(&ctx, &mut app, 1.5, false, vec![])
            .contains(&Color32::from_rgba_unmultiplied(61, 155, 232, 204))
    );
    app.presentation_view.session.set_aid(Aid::LinkHints, false);
    hint_frame(&ctx, &mut app, 2., true, vec![]);
    assert!(
        !hint_frame(&ctx, &mut app, 2.2, true, vec![])
            .contains(&Color32::from_rgba_unmultiplied(61, 155, 232, 204))
    );
    let point = app.screen([30., 30.]);
    hint_frame(
        &ctx,
        &mut app,
        3.,
        false,
        vec![
            egui::Event::PointerMoved(point),
            pointer_button(point, true),
        ],
    );
    assert_eq!(app.editor.page, contact);
}
#[test]
fn presentation_hint_flash_is_cancelled_by_settings_and_page_navigation() {
    let (ctx, mut app, contact) = link_fixture();
    let blank = app.canvas_rect.right_bottom() - Vec2::splat(15.);
    hint_frame(
        &ctx,
        &mut app,
        1.,
        false,
        vec![
            egui::Event::PointerMoved(blank),
            pointer_button(blank, true),
            pointer_button(blank, false),
        ],
    );
    app.presentation_view.settings_open = true;
    hint_frame(&ctx, &mut app, 1.1, false, vec![]);
    app.presentation_view.settings_open = false;
    assert!(
        !hint_frame(&ctx, &mut app, 1.2, false, vec![])
            .contains(&Color32::from_rgba_unmultiplied(61, 155, 232, 102))
    );
    hint_frame(
        &ctx,
        &mut app,
        2.,
        false,
        vec![pointer_button(blank, true), pointer_button(blank, false)],
    );
    app.navigate_page(contact);
    hint_frame(&ctx, &mut app, 2.1, false, vec![]);
    app.navigate_history(false);
    assert!(
        !hint_frame(&ctx, &mut app, 2.2, false, vec![])
            .contains(&Color32::from_rgba_unmultiplied(61, 155, 232, 102))
    );
}

#[test]
fn panel_resize_preserves_released_size_with_batched_input() {
    for batched in [0, 1, 2] {
        let (ctx, mut app, _) = fixture();
        ctx.enable_accesskit();
        for _ in 0..3 {
            interface_frame(&ctx, &mut app, vec![]);
        }
        let before = app.presentation_view.session.settings().side_width;
        let start = Pos2::new(app.canvas_rect.max.x + 1., 100.);
        let end = start - Vec2::new(100., 0.);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        interface_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(start)]);
        if batched == 1 {
            interface_frame(
                &ctx,
                &mut app,
                vec![
                    button(start, true),
                    egui::Event::PointerMoved(end),
                    button(end, false),
                ],
            );
        } else if batched == 2 {
            interface_frame(&ctx, &mut app, vec![button(start, true)]);
            interface_frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(end), button(end, false)],
            );
        } else {
            interface_frame(&ctx, &mut app, vec![button(start, true)]);
            interface_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(end)]);
            interface_frame(&ctx, &mut app, vec![button(end, false)]);
        }
        for _ in 0..3 {
            interface_frame(&ctx, &mut app, vec![]);
        }
        assert!(
            (app.presentation_view.session.settings().side_width - (before + 99.)).abs() < 2.,
            "batched={batched}: before={before}, after={}, start={start:?}, canvas={:?}",
            app.presentation_view.session.settings().side_width,
            app.canvas_rect
        );
    }
}

#[test]
fn batched_panel_resizes_keep_document_and_respect_input_boundaries() {
    let (ctx, mut app, _) = fixture();
    ctx.enable_accesskit();
    for _ in 0..3 {
        interface_frame(&ctx, &mut app, vec![]);
    }
    let document = app.editor.document.clone();
    let undo = app.editor.undo_name().map(str::to_owned);
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    let start = Pos2::new(app.canvas_rect.max.x + 1., 100.);
    let end = start - Vec2::new(100., 0.);
    let before = app.presentation_view.session.settings().clone();
    // Release-only, sub-threshold, off-handle, cancelled and unfocused events
    // must not resize or accidentally persist a panel preference.
    for events in [
        vec![egui::Event::PointerMoved(end), button(end, false)],
        vec![
            button(start, true),
            button(start - Vec2::new(1., 0.), false),
        ],
        vec![button(start - Vec2::new(40., 0.), true), button(end, false)],
        vec![
            button(start, true),
            egui::Event::PointerGone,
            button(end, false),
        ],
        vec![
            button(start, true),
            key(Key::Escape, Modifiers::NONE),
            button(end, false),
        ],
    ] {
        interface_frame(&ctx, &mut app, events);
        assert_eq!(app.presentation_view.session.settings(), &before);
    }
    // Escape exits presentation; re-enter before testing its modal boundary.
    app.start_presentation();
    interface_frame(&ctx, &mut app, vec![]);
    interface_frame(
        &ctx,
        &mut app,
        vec![button(start, true), button(end, false)],
    );
    assert_eq!(app.presentation_view.session.settings(), &before);
    app.presentation_view.settings_open = false;
    interface_frame(&ctx, &mut app, vec![]);
    let output = interface_output(
        &ctx,
        &mut app,
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200., 900.))),
            focused: false,
            events: vec![button(start, true), button(end, false)],
            ..Default::default()
        },
    );
    drop(output);
    assert_eq!(app.presentation_view.session.settings(), &before);
    for _ in 0..3 {
        interface_frame(&ctx, &mut app, vec![]);
    }
    // Both axes and the opposite fixed edge use the released coordinate,
    // ignoring hover movement that comes after release in the same batch.
    for (start, end, navigator) in [
        (Pos2::new(199., 100.), Pos2::new(260., 100.), true),
        (Pos2::new(1100., 299.), Pos2::new(1100., 400.), false),
    ] {
        interface_frame(
            &ctx,
            &mut app,
            vec![
                button(start, true),
                egui::Event::PointerMoved(end),
                button(end, false),
                egui::Event::PointerMoved(Pos2::new(600., 600.)),
            ],
        );
        for _ in 0..3 {
            interface_frame(&ctx, &mut app, vec![]);
        }
        let value = if navigator {
            app.presentation_view.session.settings().navigator_width
        } else {
            app.presentation_view.session.settings().notes_height
        };
        assert!(
            (value - if navigator { 260. } else { 400. }).abs() < 2.,
            "navigator={navigator}, value={value}"
        );
    }
    assert_eq!(app.editor.document, document);
    assert_eq!(app.editor.undo_name(), undo.as_deref());
}
