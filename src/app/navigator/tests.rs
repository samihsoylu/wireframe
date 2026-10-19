// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::collections::BTreeMap;

fn fixture() -> (egui::Context, WireframeApp) {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    let mut p = Project {
        font: None,
        ..Project::default()
    };
    let template = p.pages[0].clone();
    p.pages = [
        (1, "Previous", None),
        (2, "Parent", None),
        (3, "Child", Some(2)),
        (4, "Grandchild", Some(3)),
        (5, "Next", None),
    ]
    .into_iter()
    .map(|(id, name, parent)| wireframe::document::Page {
        id,
        name: name.into(),
        parent,
        ..template.clone()
    })
    .collect();
    p.next_id = 6;
    app.editor = Editor::new(p);
    app.thumbnails = false;
    (ctx, app)
}
fn frame(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    menu: bool,
    mut events: Vec<egui::Event>,
) -> BTreeMap<String, (Rect, bool)> {
    let modifiers = events
        .iter()
        .find_map(|e| {
            if let egui::Event::PointerButton { modifiers, .. } = e {
                Some(*modifiers)
            } else {
                None
            }
        })
        .unwrap_or_default();
    events.insert(0, egui::Event::ModifiersChanged(modifiers));
    let mut out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 850.))),
            events,
            ..Default::default()
        },
        |ui| {
            app.shortcuts(ui.ctx());
            app.sidebar(ui);
            if menu {
                app.page_actions_menu(ui, 2);
            }
        },
    );
    let nodes = out
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
                    Rect::from_min_max(
                        Pos2::new(b.x0 as f32, b.y0 as f32),
                        Pos2::new(b.x1 as f32, b.y1 as f32),
                    ),
                    n.is_disabled(),
                ),
            ))
        })
        .collect();
    out.textures_delta.clear();
    nodes
}
fn click(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    menu: bool,
    label: &str,
    modifiers: Modifiers,
) {
    let mut nodes = BTreeMap::new();
    for _ in 0..3 {
        nodes = frame(ctx, app, menu, vec![]);
    }
    let (r, disabled) = nodes
        .get(label)
        .unwrap_or_else(|| panic!("Missing {label}: {:?}", nodes.keys()));
    assert!(!disabled, "Disabled {label}");
    let pos = r.center();
    for pressed in [true, false] {
        frame(
            ctx,
            app,
            menu,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers,
                },
            ],
        );
    }
}

fn pointer(pos: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        pressed,
        button: egui::PointerButton::Primary,
        modifiers: Modifiers::NONE,
    }
}

fn tab(shift: bool) -> egui::Event {
    egui::Event::Key {
        key: Key::Tab,
        physical_key: Some(Key::Tab),
        pressed: true,
        repeat: false,
        modifiers: if shift {
            Modifiers::SHIFT
        } else {
            Modifiers::NONE
        },
    }
}

#[test]
fn tab_indents_selected_branch_reveals_descendants_and_keeps_keyboard_focus() {
    for thumbnails in [false, true] {
        let (ctx, mut app) = fixture();
        app.thumbnails = thumbnails;
        click(&ctx, &mut app, false, "Collapse Parent", Modifiers::NONE);
        click(&ctx, &mut app, false, "Parent", Modifiers::NONE);
        let before = app.editor.document.clone();
        frame(&ctx, &mut app, false, vec![tab(false)]);
        assert_eq!(app.editor.document.live_page(2).unwrap().parent, Some(1));
        assert_eq!(app.editor.document.live_page(3).unwrap().parent, Some(2));
        assert_eq!(app.editor.document.live_page(4).unwrap().parent, Some(3));
        assert_eq!(app.navigator.selected, HashSet::from([2, 3, 4]));
        assert_eq!(app.editor.page, 2);
        assert!(!app.navigator.collapsed.contains(&2));
        assert!(app.navigator.focused(&ctx));
        assert_eq!(app.editor.undo_name(), Some("Indent wireframes"));
        frame(&ctx, &mut app, false, vec![tab(true)]);
        // Each selected level outdents after its ancestor has moved. This is
        // intentionally different from undoing the preceding indentation.
        assert!(app.editor.document.pages.iter().all(|p| p.parent.is_none()));
        assert!(app.navigator.focused(&ctx));
        assert_eq!(app.editor.undo_name(), Some("Outdent wireframes"));
        app.editor.undo();
        assert_eq!(app.editor.document.live_page(2).unwrap().parent, Some(1));
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        assert!(app.editor.undo_name().is_none());
    }
}

#[test]
fn tab_on_expanded_parent_keeps_unselected_children_at_their_depth_and_limits_are_noops() {
    let (ctx, mut app) = fixture();
    click(&ctx, &mut app, false, "Parent", Modifiers::NONE);
    frame(&ctx, &mut app, false, vec![tab(false)]);
    assert_eq!(app.editor.document.live_page(2).unwrap().parent, Some(1));
    assert_eq!(app.editor.document.live_page(3).unwrap().parent, Some(1));
    assert_eq!(app.editor.document.live_page(4).unwrap().parent, Some(3));
    assert_eq!(app.navigator.selected, HashSet::from([2]));
    let after = app.editor.document.clone();
    frame(&ctx, &mut app, false, vec![tab(false)]);
    assert_eq!(app.editor.document, after);
    app.editor.undo();
    assert!(app.editor.undo_name().is_none());
    click(&ctx, &mut app, false, "Previous", Modifiers::NONE);
    frame(&ctx, &mut app, false, vec![tab(true)]);
    assert!(app.editor.undo_name().is_none());
    assert!(app.navigator.focused(&ctx));
}

#[test]
fn tab_does_not_mutate_pages_without_navigator_focus_or_under_rename_modal() {
    let (ctx, mut app) = fixture();
    click(&ctx, &mut app, false, "Parent", Modifiers::NONE);
    let before = app.editor.document.clone();
    ctx.memory_mut(|m| m.request_focus(egui::Id::new("another-widget")));
    frame(&ctx, &mut app, false, vec![tab(false)]);
    assert_eq!(app.editor.document, before);
    click(&ctx, &mut app, false, "Parent", Modifiers::NONE);
    app.begin_page_rename(&ctx, 2);
    frame(&ctx, &mut app, false, vec![tab(false)]);
    assert_eq!(app.editor.document, before);
    assert!(app.page_rename.is_some());
    assert!(app.editor.undo_name().is_none());
}

#[test]
fn deleting_focused_page_keeps_successor_navigation_owned_by_pages() {
    let (ctx, mut app) = fixture();
    click(&ctx, &mut app, false, "Parent", Modifiers::NONE);
    frame(
        &ctx,
        &mut app,
        false,
        vec![egui::Event::Key {
            key: Key::Delete,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert_eq!(app.editor.page, 3);
    assert!(
        app.navigator.focused(&ctx),
        "successor must retain page command ownership"
    );
    frame(&ctx, &mut app, false, vec![tab(true)]);
    assert_eq!(app.editor.document.live_page(3).unwrap().parent, None);
    assert_eq!(app.editor.undo_name(), Some("Outdent wireframes"));
}

#[test]
fn page_duplicate_undo_redo_follow_focus_without_stealing_other_widgets() {
    let (ctx, mut app) = fixture();
    click(&ctx, &mut app, false, "Parent", Modifiers::NONE);
    for key in [Key::D, Key::Z, Key::Y] {
        frame(
            &ctx,
            &mut app,
            false,
            vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::COMMAND,
            }],
        );
        assert!(app.navigator.focused(&ctx));
        assert_eq!(app.navigator.selected, HashSet::from([app.editor.page]));
    }
    ctx.memory_mut(|m| m.request_focus(egui::Id::new("another-widget")));
    app.editor.switch_page(5);
    frame(&ctx, &mut app, false, vec![]);
    assert!(!app.navigator.focused(&ctx));
}

#[test]
fn text_field_tab_does_not_indent_the_current_page() {
    let (ctx, mut app) = fixture();
    click(&ctx, &mut app, false, "Parent", Modifiers::NONE);
    let before = app.editor.document.clone();
    let mut text = String::from("A field owns this key");
    for step in 0..2 {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 850.))),
                events: if step == 1 { vec![tab(false)] } else { vec![] },
                ..Default::default()
            },
            |ui| {
                app.shortcuts(ui.ctx());
                app.sidebar(ui);
                let response =
                    ui.add(egui::TextEdit::singleline(&mut text).id(egui::Id::new("tab-owner")));
                if step == 0 {
                    response.request_focus();
                }
            },
        );
        out.textures_delta.clear();
    }
    assert_eq!(app.editor.document, before);
    assert!(app.editor.undo_name().is_none());
    assert_eq!(text, "A field owns this key");
}

fn start_drag(ctx: &egui::Context, app: &mut WireframeApp, label: &str) {
    let mut nodes = BTreeMap::new();
    for _ in 0..3 {
        nodes = frame(ctx, app, false, vec![]);
    }
    let point = nodes[label].0.center();
    frame(
        ctx,
        app,
        false,
        vec![egui::Event::PointerMoved(point), pointer(point, true)],
    );
    frame(
        ctx,
        app,
        false,
        vec![egui::Event::PointerMoved(point + Vec2::new(12., 0.))],
    );
    assert!(
        app.navigator.drag.is_some(),
        "drag must exist before mouse release"
    );
}

#[test]
fn page_drag_coalesced_movement_and_release_keeps_branch_and_atomic_history() {
    for thumbnails in [false, true] {
        for one_frame in [false, true] {
            let (ctx, mut app) = fixture();
            app.thumbnails = thumbnails;
            click(&ctx, &mut app, false, "Collapse Parent", Modifiers::NONE);
            let before = app.editor.document.clone();
            let nodes = frame(&ctx, &mut app, false, vec![]);
            let start = nodes["Parent"].0.center();
            let end = nodes["Next"].0.center_bottom() - Vec2::new(0., 2.);
            let mut events = vec![egui::Event::PointerMoved(start), pointer(start, true)];
            if !one_frame {
                frame(&ctx, &mut app, false, std::mem::take(&mut events));
            }
            events.extend([egui::Event::PointerMoved(end), pointer(end, false)]);
            frame(&ctx, &mut app, false, events);
            assert_eq!(
                app.editor
                    .document
                    .pages
                    .iter()
                    .map(|p| p.id)
                    .collect::<Vec<_>>(),
                [1, 5, 2, 3, 4],
                "thumbnails={thumbnails}, one_frame={one_frame}"
            );
            assert_eq!(app.editor.document.live_page(3).unwrap().parent, Some(2));
            assert_eq!(app.editor.document.live_page(4).unwrap().parent, Some(3));
            assert_eq!(app.editor.page, 1);
            assert!(app.navigator.drag.is_none());
            let moved = app.editor.document.clone();
            app.editor.undo();
            assert_eq!(app.editor.document, before);
            assert!(app.editor.undo_name().is_none());
            app.editor.redo();
            assert_eq!(app.editor.document, moved);
        }
    }
}

#[test]
fn page_drag_release_position_and_click_threshold_own_the_operation() {
    for mode in [
        "click_then_move",
        "outside_then_inside",
        "drop_then_outside",
        "secondary",
    ] {
        let (ctx, mut app) = fixture();
        click(&ctx, &mut app, false, "Collapse Parent", Modifiers::NONE);
        let before = app.editor.document.clone();
        let nodes = frame(&ctx, &mut app, false, vec![]);
        let start = nodes["Parent"].0.center();
        let target = nodes["Next"].0.center_bottom() - Vec2::new(0., 2.);
        let outside = Pos2::new(700., 500.);
        let (release, after) = match mode {
            "click_then_move" => (start + Vec2::new(2., 0.), target),
            "outside_then_inside" => (outside, target),
            _ => (target, outside),
        };
        let mut events = vec![
            egui::Event::PointerMoved(start),
            pointer(start, true),
            egui::Event::PointerMoved(release),
            pointer(release, false),
            egui::Event::PointerMoved(after),
        ];
        if mode == "secondary" {
            for event in &mut events {
                if let egui::Event::PointerButton { button, .. } = event {
                    *button = egui::PointerButton::Secondary;
                }
            }
        }
        frame(&ctx, &mut app, false, events);
        if mode == "drop_then_outside" {
            assert_eq!(
                app.editor
                    .document
                    .pages
                    .iter()
                    .map(|p| p.id)
                    .collect::<Vec<_>>(),
                [1, 5, 2, 3, 4]
            );
            app.editor.undo();
        }
        assert_eq!(app.editor.document, before, "{mode}");
        assert!(
            app.navigator.drag.is_none() && app.navigator.press.is_none(),
            "{mode}"
        );
        assert!(app.editor.undo_name().is_none(), "{mode}");
    }
}

#[test]
fn pending_page_drag_escape_and_stale_context_cannot_reactivate_on_release() {
    for mode in ["escape", "tree", "session", "focus", "hidden", "modal"] {
        let (ctx, mut app) = fixture();
        let nodes = frame(&ctx, &mut app, false, vec![]);
        let start = nodes["Parent"].0.center();
        let end = nodes["Next"].0.center_bottom() - Vec2::new(0., 2.);
        frame(
            &ctx,
            &mut app,
            false,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
        );
        assert!(app.navigator.press.is_some());
        match mode {
            "escape" => {
                frame(
                    &ctx,
                    &mut app,
                    false,
                    vec![egui::Event::Key {
                        key: Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::NONE,
                    }],
                );
            }
            "tree" => app.editor.document.pages.swap(0, 1),
            "modal" => app.error = Some("An operation needs attention".into()),
            "session" => app.editor = Editor::new(app.editor.document.clone()),
            "focus" => {
                let mut out = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 850.))),
                        focused: false,
                        ..Default::default()
                    },
                    |ui| {
                        app.shortcuts(ui.ctx());
                        app.sidebar(ui);
                    },
                );
                out.textures_delta.clear();
                assert!(app.navigator.press.is_none());
            }
            "hidden" => {
                app.show_navigator = false;
                frame(&ctx, &mut app, false, vec![]);
            }
            _ => unreachable!(),
        }
        let before = app.editor.document.clone();
        frame(
            &ctx,
            &mut app,
            false,
            vec![egui::Event::PointerMoved(end), pointer(end, false)],
        );
        assert_eq!(app.editor.document, before, "{mode}");
        assert!(
            app.navigator.drag.is_none() && app.navigator.press.is_none(),
            "{mode}"
        );
    }
}

#[test]
fn page_drag_previews_without_mutation_then_moves_closed_branch_as_one_history_step() {
    for thumbnails in [false, true] {
        let (ctx, mut app) = fixture();
        app.thumbnails = thumbnails;
        click(&ctx, &mut app, false, "Collapse Parent", Modifiers::NONE);
        let before = app.editor.document.clone();
        start_drag(&ctx, &mut app, "Parent");
        assert_eq!(app.editor.document, before);
        assert!(app.editor.undo_name().is_none());
        let nodes = frame(&ctx, &mut app, false, vec![]);
        let target = nodes["Next"].0.center_bottom() - Vec2::new(0., 2.);
        frame(
            &ctx,
            &mut app,
            false,
            vec![egui::Event::PointerMoved(target)],
        );
        assert_eq!(app.editor.document, before);
        frame(&ctx, &mut app, false, vec![pointer(target, false)]);
        assert!(app.navigator.drag.is_none());
        assert_eq!(
            app.editor
                .document
                .pages
                .iter()
                .map(|p| p.id)
                .collect::<Vec<_>>(),
            [1, 5, 2, 3, 4]
        );
        assert_eq!(app.editor.document.live_page(3).unwrap().parent, Some(2));
        assert_eq!(app.editor.document.live_page(4).unwrap().parent, Some(3));
        assert_eq!(app.editor.page, 1);
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        assert!(app.editor.undo_name().is_none());
    }
}

#[test]
fn page_drop_after_closed_branch_skips_hidden_descendants() {
    let (ctx, mut app) = fixture();
    click(&ctx, &mut app, false, "Collapse Parent", Modifiers::NONE);
    start_drag(&ctx, &mut app, "Next");
    let nodes = frame(&ctx, &mut app, false, vec![]);
    let target = nodes["Parent"].0.center_bottom() - Vec2::new(0., 2.);
    frame(
        &ctx,
        &mut app,
        false,
        vec![egui::Event::PointerMoved(target), pointer(target, false)],
    );
    assert_eq!(
        app.editor
            .document
            .pages
            .iter()
            .map(|p| p.id)
            .collect::<Vec<_>>(),
        [1, 2, 3, 4, 5]
    );
    assert_eq!(app.editor.document.live_page(5).unwrap().parent, None);
    assert!(app.editor.undo_name().is_none());
}

#[test]
fn dragging_selected_closed_branch_preserves_selection_and_collapse() {
    let (ctx, mut app) = fixture();
    click(&ctx, &mut app, false, "Collapse Parent", Modifiers::NONE);
    click(&ctx, &mut app, false, "Parent", Modifiers::NONE);
    assert_eq!(app.navigator.selected, HashSet::from([2, 3, 4]));
    start_drag(&ctx, &mut app, "Parent");
    let nodes = frame(&ctx, &mut app, false, vec![]);
    let target = nodes["Next"].0.center_bottom() - Vec2::new(0., 2.);
    frame(
        &ctx,
        &mut app,
        false,
        vec![egui::Event::PointerMoved(target), pointer(target, false)],
    );
    frame(&ctx, &mut app, false, vec![]);
    assert_eq!(
        app.editor
            .document
            .pages
            .iter()
            .map(|p| p.id)
            .collect::<Vec<_>>(),
        [1, 5, 2, 3, 4]
    );
    assert_eq!(app.navigator.selected, HashSet::from([2, 3, 4]));
    assert!(app.navigator.collapsed.contains(&2));
    assert_eq!(app.editor.page, 2);
}

#[test]
fn page_drag_escape_outside_and_stale_source_cancel_without_editing() {
    for cancel in ["escape", "outside", "stale", "session", "focus"] {
        let (ctx, mut app) = fixture();
        start_drag(&ctx, &mut app, "Parent");
        if cancel == "stale" {
            app.editor.add_page();
        }
        if cancel == "session" {
            app.editor = Editor::new(app.editor.document.clone());
        }
        let before = app.editor.document.clone();
        let undo = app.editor.undo_name().map(str::to_owned);
        let target = Pos2::new(700., 700.);
        if cancel == "escape" {
            frame(
                &ctx,
                &mut app,
                false,
                vec![egui::Event::Key {
                    key: Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
            );
        }
        if cancel == "focus" {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 850.))),
                    focused: false,
                    ..Default::default()
                },
                |ui| {
                    app.shortcuts(ui.ctx());
                    app.sidebar(ui);
                },
            );
            out.textures_delta.clear();
            assert!(app.navigator.drag.is_none());
        }
        frame(
            &ctx,
            &mut app,
            false,
            vec![egui::Event::PointerMoved(target), pointer(target, false)],
        );
        assert!(app.navigator.drag.is_none());
        assert_eq!(app.editor.document, before);
        assert_eq!(app.editor.undo_name(), undo.as_deref());
    }
}

#[test]
fn page_rename_shortcuts_use_current_page_and_held_drag_blocks_commands() {
    for (key, modifiers) in [
        (Key::F2, Modifiers::NONE),
        (Key::R, Modifiers::COMMAND | Modifiers::ALT),
    ] {
        let (ctx, mut app) = fixture();
        click(&ctx, &mut app, false, "Parent", Modifiers::NONE);
        let before = app.editor.document.clone();
        frame(
            &ctx,
            &mut app,
            false,
            vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
        );
        assert!(app.page_rename.is_some());
        assert_eq!(app.editor.document, before);
        assert_eq!(app.editor.page, 2);
    }
    let (ctx, mut app) = fixture();
    let before = app.editor.document.clone();
    start_drag(&ctx, &mut app, "Parent");
    frame(
        &ctx,
        &mut app,
        false,
        vec![
            egui::Event::Key {
                key: Key::Delete,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
            egui::Event::Key {
                key: Key::F2,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    assert!(app.navigator.drag.is_some());
    assert!(app.page_rename.is_none());
    assert_eq!(app.editor.document, before);
}

#[test]
fn disclosures_hide_descendants_and_select_active_parent_without_editing() {
    let (ctx, mut app) = fixture();
    let before = app.editor.document.clone();
    click(&ctx, &mut app, false, "Collapse Parent", Modifiers::NONE);
    assert_eq!(app.editor.page, 1);
    assert!(app.navigator.collapsed.contains(&2));
    let nodes = frame(&ctx, &mut app, false, vec![]);
    assert!(!nodes.contains_key("Child"));
    assert!(nodes.contains_key("Next"));
    click(&ctx, &mut app, false, "Thumbnails", Modifiers::NONE);
    assert!(!frame(&ctx, &mut app, false, vec![]).contains_key("Child"));
    click(&ctx, &mut app, false, "Expand Parent", Modifiers::NONE);
    click(&ctx, &mut app, false, "Child", Modifiers::NONE);
    assert_eq!(app.editor.page, 3);
    click(&ctx, &mut app, false, "Collapse Parent", Modifiers::NONE);
    assert_eq!(app.editor.page, 2);
    assert_eq!(app.navigator.selected, HashSet::from([2, 3, 4]));
    assert_eq!(app.editor.document, before);
    assert!(app.editor.undo_name().is_none());
}

#[test]
fn modified_range_and_context_scope_include_hidden_descendants() {
    let (ctx, mut app) = fixture();
    click(&ctx, &mut app, false, "Collapse Parent", Modifiers::NONE);
    click(&ctx, &mut app, false, "Next", Modifiers::SHIFT);
    assert_eq!(app.editor.page, 1);
    assert_eq!(app.navigator.selected, HashSet::from([1, 2, 3, 4, 5]));
    let nodes = frame(&ctx, &mut app, true, vec![]);
    assert!(nodes.contains_key("Duplicate 5 items"));
    assert!(nodes["Rename…"].1);
    assert!(nodes["Create alternate version"].1);
    click(&ctx, &mut app, false, "Parent", Modifiers::COMMAND);
    assert_eq!(app.navigator.selected, HashSet::from([1, 5]));
    assert_eq!(app.editor.page, 1);
    let nodes = frame(&ctx, &mut app, true, vec![]);
    assert!(nodes.contains_key("Move 3 items to Trash"));
}

#[test]
fn collapsed_context_duplicate_and_trash_use_one_complete_batch() {
    let (ctx, mut app) = fixture();
    let before = app.editor.document.clone();
    click(&ctx, &mut app, false, "Collapse Parent", Modifiers::NONE);
    click(&ctx, &mut app, true, "Duplicate 3 items", Modifiers::NONE);
    frame(&ctx, &mut app, false, vec![]);
    let copies = app.editor.duplicated_page_selection();
    assert_eq!(copies.len(), 3);
    assert_eq!(app.navigator.selected, copies.iter().copied().collect());
    assert_eq!(app.editor.page, copies[0]);
    app.editor.undo();
    frame(&ctx, &mut app, false, vec![]);
    assert_eq!(app.editor.document, before);
    assert_eq!(app.editor.page, 4);
    assert!(!app.navigator.collapsed.contains(&2));
    app.editor.redo();
    frame(&ctx, &mut app, false, vec![]);
    assert_eq!(app.navigator.selected, copies.iter().copied().collect());
    click(&ctx, &mut app, false, "Collapse Parent", Modifiers::NONE);
    let before_trash = app.editor.document.clone();
    click(
        &ctx,
        &mut app,
        true,
        "Move 3 items to Trash",
        Modifiers::NONE,
    );
    assert!(
        app.editor
            .document
            .pages
            .iter()
            .filter(|p| [2, 3, 4].contains(&p.id))
            .all(|p| p.trashed)
    );
    assert!(
        copies
            .iter()
            .all(|id| app.editor.document.live_page(*id).is_some())
    );
    app.editor.undo();
    assert_eq!(app.editor.document, before_trash);
}

#[test]
fn focused_page_commands_use_pages_and_cut_failure_leaves_canvas_unchanged() {
    let (ctx, mut app) = fixture();
    app.editor.add(Kind::Button, 10., 20.);
    let before = app.editor.document.clone();
    click(&ctx, &mut app, false, "Previous", Modifiers::NONE);
    assert!(app.navigator.focused(&ctx));
    assert!(app.command_enabled(&ctx, menus::Command::Duplicate));
    assert!(app.command_enabled(&ctx, menus::Command::Copy));
    frame(
        &ctx,
        &mut app,
        false,
        vec![egui::Event::Key {
            key: Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::COMMAND,
        }],
    );
    assert_eq!(app.navigator.selected.len(), 5);
    assert!(app.editor.selection.is_empty());
    frame(
        &ctx,
        &mut app,
        false,
        vec![egui::Event::AppCommand(menus::Command::Duplicate as u64)],
    );
    assert_eq!(app.editor.document.live_pages().count(), 10);
    assert_eq!(app.navigator.selected.len(), 5);
    app.editor.undo();
    frame(&ctx, &mut app, false, vec![]);
    assert_eq!(app.editor.document, before);
    click(&ctx, &mut app, false, "Previous", Modifiers::NONE);
    frame(
        &ctx,
        &mut app,
        false,
        vec![egui::Event::Cut, egui::Event::Paste("unrelated".into())],
    );
    assert_eq!(app.editor.document, before);
    let request = app.pending_cut_request().unwrap();
    frame(
        &ctx,
        &mut app,
        false,
        vec![egui::Event::ClipboardWriteResult {
            request,
            result: Err("Clipboard unavailable".into()),
        }],
    );
    app.error = None;
    frame(
        &ctx,
        &mut app,
        false,
        vec![egui::Event::Key {
            key: Key::Backspace,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert!(
        app.editor
            .document
            .pages
            .iter()
            .find(|p| p.id == 1)
            .unwrap()
            .trashed
    );
    app.editor.undo();
    assert_eq!(app.editor.document, before);
}

#[test]
fn escape_dismisses_page_context_without_clearing_page_selection() {
    let (ctx, mut app) = fixture();
    click(&ctx, &mut app, false, "Collapse Parent", Modifiers::NONE);
    let nodes = frame(&ctx, &mut app, false, vec![]);
    let pos = nodes["Parent"].0.center();
    for pressed in [true, false] {
        frame(
            &ctx,
            &mut app,
            false,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Secondary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
        );
    }
    assert!(egui::Popup::is_any_open(&ctx));
    let before = app.editor.document.clone();
    let selection = app.navigator.selected.clone();
    frame(
        &ctx,
        &mut app,
        false,
        vec![egui::Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert!(!egui::Popup::is_any_open(&ctx));
    assert_eq!(app.navigator.selected, selection);
    assert_eq!(app.editor.document, before);
    nav_key(&ctx, &mut app, Key::ArrowDown, Modifiers::NONE);
    assert_eq!(
        app.editor.page, 2,
        "Keyboard ownership survives context dismissal"
    );
}

fn nav_key(ctx: &egui::Context, app: &mut WireframeApp, key: Key, modifiers: Modifiers) {
    frame(
        ctx,
        app,
        false,
        vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }],
    );
}

#[test]
fn keyboard_traversal_skips_closed_children_and_extends_visible_ranges() {
    let (ctx, mut app) = fixture();
    let before = app.editor.document.clone();
    click(&ctx, &mut app, false, "Previous", Modifiers::NONE);
    nav_key(&ctx, &mut app, Key::ArrowDown, Modifiers::NONE);
    assert_eq!(app.editor.page, 2);
    nav_key(&ctx, &mut app, Key::ArrowLeft, Modifiers::NONE);
    assert!(app.navigator.collapsed.contains(&2));
    assert_eq!(app.navigator.selected, HashSet::from([2, 3, 4]));
    nav_key(&ctx, &mut app, Key::ArrowUp, Modifiers::SHIFT);
    assert_eq!(app.editor.page, 2);
    assert_eq!(app.navigator.selected, HashSet::from([1, 2, 3, 4]));
    nav_key(&ctx, &mut app, Key::ArrowDown, Modifiers::SHIFT);
    assert_eq!(app.navigator.selected, HashSet::from([2, 3, 4]));
    nav_key(&ctx, &mut app, Key::ArrowDown, Modifiers::NONE);
    assert_eq!(app.editor.page, 5);
    nav_key(&ctx, &mut app, Key::ArrowDown, Modifiers::NONE);
    assert_eq!(app.editor.page, 5);
    nav_key(&ctx, &mut app, Key::PageUp, Modifiers::NONE);
    assert_eq!(app.editor.page, 2);
    nav_key(&ctx, &mut app, Key::ArrowRight, Modifiers::NONE);
    assert!(!app.navigator.collapsed.contains(&2));
    assert_eq!(app.navigator.selected, HashSet::from([2]));
    nav_key(&ctx, &mut app, Key::ArrowRight, Modifiers::NONE);
    assert_eq!(app.editor.page, 2);
    nav_key(&ctx, &mut app, Key::PageDown, Modifiers::NONE);
    assert_eq!(app.editor.page, 3);
    nav_key(&ctx, &mut app, Key::ArrowDown, Modifiers::NONE);
    assert_eq!(app.editor.page, 4);
    nav_key(&ctx, &mut app, Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(app.editor.page, 4);
    assert_eq!(app.editor.document, before);
    assert!(app.editor.undo_name().is_none());
}

#[test]
fn view_mode_change_reveals_current_thumbnail_without_document_edits() {
    let (ctx, mut app) = fixture();
    let template = app.editor.document.pages[0].clone();
    for id in 6..=15 {
        app.editor.document.pages.push(wireframe::document::Page {
            id,
            name: format!("Page {id}"),
            ..template.clone()
        });
    }
    app.editor.document.next_id = 16;
    let before = app.editor.document.clone();
    click(&ctx, &mut app, false, "Page 15", Modifiers::NONE);
    click(&ctx, &mut app, false, "Thumbnails", Modifiers::NONE);
    let mut nodes = BTreeMap::new();
    for _ in 0..30 {
        nodes = frame(&ctx, &mut app, false, vec![]);
    }
    let last = nodes["Page 15"].0;
    assert!(last.top() >= nodes["Thumbnails"].0.bottom(), "{last:?}");
    assert!(last.bottom() <= nodes["Versions"].0.top(), "{last:?}");
    assert_eq!(app.editor.page, 15);
    assert_eq!(app.editor.document, before);
    assert!(app.editor.undo_name().is_none());
}
