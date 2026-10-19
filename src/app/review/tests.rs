// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::interface::Action;
use super::*;

fn setup() -> (egui::Context, WireframeApp) {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.review.open = true;
    app.canvas_rect = Rect::from_min_size(egui::pos2(200., 100.), Vec2::new(600., 500.));
    app.sync_viewport();
    (ctx, app)
}
fn frame(
    app: &mut WireframeApp,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
) -> BTreeMap<String, Rect> {
    frame_at(app, ctx, events, Vec2::new(1000., 800.))
}
fn frame_at(
    app: &mut WireframeApp,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    size: Vec2,
) -> BTreeMap<String, Rect> {
    let mut input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        events,
        ..Default::default()
    };
    app.order_text_input(ctx, &mut input);
    let mut output = ctx.run_ui(input, |ui| {
        app.sync_reviews();
        app.shortcuts(ui.ctx());
        if app.present {
            app.presentation_toolbar(ui);
        } else {
            app.review_panel(ui);
        }
        app.canvas(ui);
    });
    output.textures_delta.clear();
    output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter_map(|(_, node)| {
            let label = node
                .label()
                .or_else(|| node.placeholder())
                .or_else(|| node.value())?;
            let r = node.bounds()?;
            Some((
                label.into(),
                Rect::from_min_max(
                    egui::pos2(r.x0 as f32, r.y0 as f32),
                    egui::pos2(r.x1 as f32, r.y1 as f32),
                ),
            ))
        })
        .collect()
}
fn click(app: &mut WireframeApp, ctx: &egui::Context, position: Pos2) {
    frame(app, ctx, vec![egui::Event::PointerMoved(position)]);
    frame(
        app,
        ctx,
        vec![egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        }],
    );
    frame(
        app,
        ctx,
        vec![egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    );
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
fn enter_text(app: &mut WireframeApp, text: &str) {
    let composer = app.review_surface().draft.as_mut().unwrap();
    composer.text.value = text.into();
    composer.caret = text.len();
}

#[test]
fn readable_tokens_preserve_unicode_escapes_urls_and_caret() {
    let source = "é *[(1)](callout:7)* [Phone](surface:9) [web](https://example.test/(1)) \\[(1)](callout:90)";
    let text = text::Text::from_source(source).unwrap();
    assert!(!text.value.contains("callout:7"));
    assert!(text.value.starts_with("é *(1)* [Phone]"));
    assert_eq!(text.encode(text.value.len()).unwrap().0, source);
    let pos = text.value.find("(1)").unwrap();
    let (encoded, caret) = text.encode(pos + 1).unwrap();
    assert_eq!(&encoded[caret..caret + 1], "[");
    let mut duplicate = text::Text::from_source("[(1)](callout:7) [(1)](callout:8)").unwrap();
    assert_eq!(duplicate.value, "(1) (1.1)");
    let saved = duplicate.value.clone();
    duplicate.value.clear();
    assert_eq!(duplicate.encode(0).unwrap().0, "");
    duplicate.value = saved;
    assert_eq!(
        duplicate.encode(duplicate.value.len()).unwrap().0,
        "[(1)](callout:7) [(1.1)](callout:8)"
    );
    assert!(duplicate.encode(999).is_err());
}

#[test]
fn composer_posts_last_batched_input_without_drawing_history_and_escape_retains_draft() {
    let (ctx, mut app) = setup();
    frame(&mut app, &ctx, vec![]);
    let nodes = frame(&mut app, &ctx, vec![]);
    click(&mut app, &ctx, nodes["Add a Comment"].center());
    frame(&mut app, &ctx, vec![]);
    assert!(app.review_focused(&ctx));
    let history = app.editor.undo_name().map(str::to_owned);
    frame(
        &mut app,
        &ctx,
        vec![
            egui::Event::Text("Review café 👋".into()),
            key(Key::Enter, Modifiers::COMMAND),
        ],
    );
    assert!(app.editor.document.reviews.is_empty());
    frame(&mut app, &ctx, vec![]);
    let discussion = &app.editor.document.reviews[&app.editor.surface_id()];
    assert_eq!(discussion.comments[&1].body.text, "Review café 👋");
    assert_eq!(app.editor.undo_name(), history.as_deref());
    assert!(app.editor.dirty());
    app.begin_review(None, None);
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![egui::Event::Text("keep draft".into())]);
    frame(&mut app, &ctx, vec![key(Key::Escape, Modifiers::NONE)]);
    assert_eq!(
        app.review_surface().draft.as_ref().unwrap().text.value,
        "keep draft"
    );
    assert!(!app.review_focused(&ctx));
}

#[test]
fn drafts_are_surface_scoped_and_replacements_keep_text_but_block_posting() {
    let (_, mut app) = setup();
    let first = app.editor.surface_id();
    app.begin_review(None, None);
    enter_text(&mut app, "first draft");
    app.editor.add_page();
    let second = app.editor.surface_id();
    app.begin_review(None, None);
    enter_text(&mut app, "second draft");
    app.editor.switch_version(first, None).unwrap();
    app.sync_reviews();
    assert_eq!(
        app.review_surface().draft.as_ref().unwrap().text.value,
        "first draft"
    );
    app.editor
        .change_review(first, |d| {
            d.post(
                Author {
                    id: "other".into(),
                    name: "Other".into(),
                },
                None,
                wireframe::review::Body {
                    text: "External feedback".into(),
                    callouts: vec![],
                },
                1,
            )
        })
        .unwrap();
    app.sync_reviews();
    assert!(app.review_surface().draft.as_ref().unwrap().stale);
    app.post_review();
    assert_eq!(app.editor.document.reviews[&first].comments.len(), 1);
    app.editor.switch_version(second, None).unwrap();
    app.sync_reviews();
    assert!(!app.review_surface().draft.as_ref().unwrap().stale);
    app.post_review();
    assert_eq!(
        app.editor.document.reviews[&second].comments[&1].body.text,
        "second draft"
    );
    app.review.reset();
    assert!(app.review.surfaces.is_empty());
}

#[test]
fn review_actions_keep_attribution_thread_order_and_session_deletion_boundaries() {
    let (_, mut app) = setup();
    app.begin_review(None, None);
    enter_text(&mut app, "Parent");
    app.post_review();
    app.review_action(Action::Reply(1));
    enter_text(&mut app, "Reply");
    app.post_review();
    app.review_action(Action::Like(2, true));
    app.review_action(Action::Read(2, false));
    let actor = app.review.identity.author.id.clone();
    assert!(
        app.review_surface().observed.comments[&2]
            .liked_by
            .contains(&actor)
    );
    assert!(
        !app.review_surface().observed.comments[&2]
            .read_by
            .contains(&actor)
    );
    app.review_action(Action::Delete(1));
    assert!(
        app.review_surface().observed.comments[&1]
            .body
            .text
            .is_empty()
    );
    assert_eq!(
        app.review_surface()
            .observed
            .rows(&BTreeSet::new())
            .unwrap()
            .len(),
        2
    );
    app.review_action(Action::Restore(1));
    assert_eq!(
        app.review_surface().observed.comments[&1].body.text,
        "Parent"
    );
    app.review_action(Action::Edit(2));
    enter_text(&mut app, "Changed reply");
    app.post_review();
    assert_eq!(
        app.review_surface().observed.comments[&2].body.text,
        "Changed reply"
    );
    app.review_action(Action::Delete(2));
    app.editor
        .document
        .reviews
        .get_mut(&app.editor.surface_id())
        .unwrap()
        .comments
        .get_mut(&1)
        .unwrap()
        .imported = true;
    app.sync_reviews();
    assert!(app.review_surface().deleted.is_empty());
    app.review_action(Action::Edit(1));
    assert!(app.review_surface().draft.is_none());
}

#[test]
fn local_identity_reopens_and_corrupt_settings_are_preserved() {
    let root = tempfile::tempdir().unwrap();
    let mut identity = identity::Identity::default();
    identity.load(root.path());
    identity.name = "Reviewer É".into();
    let saved = identity.save().unwrap();
    let mut reopened = identity::Identity::default();
    reopened.load(root.path());
    assert_eq!(saved, reopened.save().unwrap());
    let path = root.path().join("reviewer.json");
    std::fs::write(&path, b"broken settings").unwrap();
    reopened.load(root.path());
    assert!(reopened.save().is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"broken settings");
}

#[test]
fn callout_drag_stays_in_draft_cancel_restores_and_post_preserves_position() {
    let (ctx, mut app) = setup();
    app.begin_review(None, None);
    enter_text(&mut app, "Check this");
    frame(&mut app, &ctx, vec![]);
    let nodes = frame(&mut app, &ctx, vec![]);
    click(&mut app, &ctx, nodes["Add Callout"].center());
    let marker = app
        .review_surface()
        .draft
        .as_ref()
        .unwrap()
        .draft
        .body
        .callouts[0]
        .clone();
    let start = app.screen([marker.position.x + 12.5, marker.position.y]);
    let end = start + Vec2::new(-80., -30.);
    frame(&mut app, &ctx, vec![egui::Event::PointerMoved(start)]);
    frame(
        &mut app,
        &ctx,
        vec![
            egui::Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
            egui::Event::PointerMoved(end),
            egui::Event::PointerButton {
                pos: end,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    let moved = app
        .review_surface()
        .draft
        .as_ref()
        .unwrap()
        .draft
        .body
        .callouts[0]
        .clone();
    assert!((moved.position.x - marker.position.x + 80. / app.zoom).abs() < 0.01);
    assert!(app.editor.document.reviews.is_empty());
    assert!(app.editor.current().controls.is_empty());
    app.post_review();
    let id = app.editor.surface_id();
    assert_eq!(
        app.editor.document.reviews[&id].comments[&1].body.callouts[0].position,
        moved.position
    );
    app.begin_review(None, Some(1));
    app.review_surface()
        .draft
        .as_mut()
        .unwrap()
        .draft
        .body
        .callouts[0]
        .position
        .x += 123.;
    app.cancel_review();
    assert_eq!(
        app.editor.document.reviews[&id].comments[&1].body.callouts[0].position,
        moved.position
    );
}

#[test]
fn formatted_surface_labels_round_trip_and_duplicate_callouts_have_surface_wide_names() {
    let source = "[*Phone*](surface:3) [_Phone_](surface:3) [A\\] B](surface:4)";
    let text = text::Text::from_source(source).unwrap();
    assert_eq!(text.encode(text.value.len()).unwrap().0, source);
    let mut discussion = Discussion::default();
    let author = Author {
        id: "reviewer".into(),
        name: "Reviewer".into(),
    };
    for id in [1, 2] {
        let marker = wireframe::review::Callout {
            id,
            label: 1,
            position: wireframe::document::Point { x: 0., y: 0. },
            tip: Default::default(),
            color: Default::default(),
        };
        discussion
            .post(
                author.clone(),
                None,
                wireframe::review::Body {
                    text: marker.markup(),
                    callouts: vec![marker],
                },
                id,
            )
            .unwrap();
    }
    let state = Surface {
        observed: discussion,
        ..Default::default()
    };
    assert_eq!(
        state.marker_names(),
        BTreeMap::from([(1, "1.1".into()), (2, "1.2".into())])
    );
}

#[test]
fn presentation_composer_keeps_character_shortcuts_and_preserves_view_after_posting() {
    let (ctx, mut app) = setup();
    app.start_presentation();
    app.presentation_view.settings_open = false;
    frame(&mut app, &ctx, vec![]);
    frame(&mut app, &ctx, vec![]);
    app.begin_review(None, None);
    frame(&mut app, &ctx, vec![]);
    assert!(app.review_focused(&ctx));
    let settings = app.presentation_view.session.settings().clone();
    app.pan += Vec2::new(47., 25.);
    let pan = app.pan;
    frame(
        &mut app,
        &ctx,
        vec![
            key(Key::C, Modifiers::NONE),
            egui::Event::Text("comments stay open".into()),
            key(Key::B, Modifiers::NONE),
        ],
    );
    assert_eq!(*app.presentation_view.session.settings(), settings);
    app.post_review();
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.pan, pan);
    assert_eq!(
        app.editor.document.reviews[&app.editor.surface_id()].comments[&1]
            .body
            .text,
        "comments stay open"
    );
}

#[test]
fn alternate_links_visit_exact_surface_and_missing_links_preserve_current_page() {
    let (_, mut app) = setup();
    let page = app.editor.page;
    let alternate = app.editor.create_version("Phone").unwrap();
    app.editor.add_page();
    let other = app.editor.page;
    app.review_link(&format!("surface:{alternate}"));
    assert_eq!(app.editor.page, page);
    assert_eq!(app.editor.active_alternate(page), Some(alternate));
    app.review_link(&format!("surface:{page}"));
    assert_eq!(app.editor.active_alternate(page), None);
    app.review_link("surface:99999999");
    assert_eq!(app.editor.page, page);
    app.review_link(&format!("surface:{other}"));
    assert_eq!(app.editor.page, other);
}

#[test]
fn new_composer_has_its_own_text_history_and_inspector_returns_without_discarding_draft() {
    let (ctx, mut app) = setup();
    app.begin_review(None, None);
    frame(&mut app, &ctx, vec![]);
    let first = app.review_input_id();
    frame(
        &mut app,
        &ctx,
        vec![egui::Event::Text("Prior comment".into())],
    );
    app.post_review();
    app.begin_review(None, None);
    frame(&mut app, &ctx, vec![]);
    assert_ne!(app.review_input_id(), first);
    frame(&mut app, &ctx, vec![key(Key::Z, Modifiers::COMMAND)]);
    assert!(
        app.review_surface()
            .draft
            .as_ref()
            .unwrap()
            .text
            .value
            .is_empty()
    );
    enter_text(&mut app, "New draft");
    app.run_command(&ctx, menus::Command::Inspector);
    assert!(!app.review.open);
    assert!(app.show_inspector);
    app.run_command(&ctx, menus::Command::Comments);
    assert!(app.review.open);
    assert_eq!(
        app.review_surface().draft.as_ref().unwrap().text.value,
        "New draft"
    );
}

#[test]
#[cfg(target_os = "macos")]
#[ignore = "Opt-in windowless rendering; requires WIREFRAME_RENDER_DIR"]
fn render_review_states_without_windows() {
    let root = PathBuf::from(
        std::env::var_os("WIREFRAME_RENDER_DIR").expect("Set snapshot output directory"),
    );
    std::fs::create_dir_all(&root).unwrap();
    let mut gpu = crate::app::offscreen_tests::Offscreen::new();
    let mut evidence = vec![];
    for (name, size, density, present, long) in [
        ("editor-1x", Vec2::new(1280., 800.), 1., false, false),
        ("editor-2x", Vec2::new(1280., 800.), 2., false, false),
        ("minimum-long", Vec2::new(940., 620.), 1., false, true),
        (
            "presentation-minimum-long",
            Vec2::new(940., 620.),
            1.,
            true,
            true,
        ),
        ("presentation-2x", Vec2::new(1280., 800.), 2., true, false),
        ("presentation-narrow", Vec2::new(940., 620.), 1., true, true),
        (
            "presentation-callout-settings",
            Vec2::new(940., 620.),
            1.,
            true,
            true,
        ),
    ] {
        let (ctx, mut app) = setup();

        app.editor.document.title = "Contact page review".into();
        for (kind, y, text) in [
            (Kind::Heading, 70., "Contact us"),
            (Kind::Button, 150., "Send message"),
        ] {
            app.editor.add(kind, 70., y);
            let mut control = app.editor.selected_views()[0].clone();
            control.text = text.into();
            app.editor.begin("Fixture text");
            app.editor.update_control(control);
            app.editor.commit();
        }
        app.editor.selection.clear();
        app.review.identity.name = "Alex".into();
        app.begin_review(None, None);
        enter_text(&mut app, "Please make the primary action easier to find.");
        app.post_review();
        app.review.identity.name = "Sam".into();
        app.begin_review(Some(1), None);
        enter_text(
            &mut app,
            "I moved it below the contact details. Please check the new position.",
        );
        app.post_review();
        app.begin_review(None, None);
        let discussion = app.review_surface().observed.clone();
        let composer = app.review_surface().draft.as_mut().unwrap();
        composer.draft.body.text = "Check this location ".into();
        let caret = composer.draft.body.text.len();
        composer
            .draft
            .add_callout(
                &discussion,
                wireframe::document::Point { x: 360., y: 220. },
                caret,
            )
            .unwrap();
        composer
            .text
            .show_source(&composer.draft.body.text)
            .unwrap();
        composer.caret = composer.text.value.len();
        if long {
            composer.text.value = format!(
                "{}\n{}",
                composer.text.value,
                "A long draft with detail, links and multiple lines.\n".repeat(30)
            );
            composer.caret = composer.text.value.len();
        }
        app.review_surface().selected = Some(1);
        if name == "presentation-callout-settings" {
            app.review_surface()
                .draft
                .as_mut()
                .unwrap()
                .callout_settings = true;
        }
        if present {
            app.start_presentation();
            app.presentation_view.settings_open = false;
            if name == "presentation-narrow" || name == "presentation-callout-settings" {
                app.presentation_view
                    .session
                    .set_panel_sizes(200., 180., 700.)
                    .unwrap();
            }
        }
        let mut bounds = vec![];
        for frame in 0..4 {
            let mut input = egui::RawInput {
                time: Some(frame as f64 * 0.1),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                max_texture_side: Some(gpu.max_texture_side()),
                ..Default::default()
            };
            input
                .viewports
                .get_mut(&egui::ViewportId::ROOT)
                .unwrap()
                .native_pixels_per_point = Some(density);
            let mut output = ctx.run_ui(input, |ui| {
                app.sync_reviews();
                app.sync_viewport();
                app.toolbar(ui);
                if !app.present {
                    app.sidebar(ui);
                    app.review_panel(ui);
                }
                app.canvas(ui);
            });
            bounds = output
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .filter_map(|(_, node)| {
                    let label = node
                        .label()
                        .or_else(|| node.placeholder())
                        .or_else(|| node.value())?;
                    let b = node.bounds()?;
                    Some(serde_json::json!({"label":label,"bounds":[b.x0,b.y0,b.x1,b.y1]}))
                })
                .collect();
            gpu.draw(&ctx, &mut output, size);
        }
        gpu.save(&root.join(format!("{name}.png")));
        evidence.push(serde_json::json!({"name":name,"logical_size":[size.x,size.y],"density":density,"bounds":bounds}));
    }
    std::fs::write(root.join("render.json"),serde_json::to_vec_pretty(&serde_json::json!({"renderer":gpu.renderer(),"kind":"Windowless application widgets using production egui OpenGL renderer; no native window or input integration; opacity callbacks excluded","states":evidence})).unwrap()).unwrap();
}

#[test]
fn minimum_presentation_keeps_long_draft_caret_and_submission_in_view() {
    for width in [180., 300.] {
        let (ctx, mut app) = setup();
        app.begin_review(None, None);
        enter_text(&mut app, "Existing discussion");
        app.post_review();
        app.start_presentation();
        app.presentation_view.settings_open = false;
        app.presentation_view
            .session
            .set_panel_sizes(200., width, 700.)
            .unwrap();
        app.begin_review(None, None);
        enter_text(
            &mut app,
            &"Long draft line with detail and a Unicode word café.\n".repeat(30),
        );
        let size = Vec2::new(940., 620.);
        let mut bounds = BTreeMap::new();
        for _ in 0..6 {
            bounds = frame_at(&mut app, &ctx, vec![], size);
        }
        let view = Rect::from_min_size(Pos2::ZERO, size);
        assert!(
            view.contains_rect(bounds["Post"]),
            "Post {:?}",
            bounds["Post"]
        );
        assert!(view.contains_rect(bounds["Cancel"]));
        assert!(app.review_surface().draft.as_ref().unwrap().caret_visible);
        frame_at(
            &mut app,
            &ctx,
            vec![
                egui::Event::Text("Last words".into()),
                key(Key::Enter, Modifiers::COMMAND),
            ],
            size,
        );
        frame_at(&mut app, &ctx, vec![], size);
        assert!(
            app.editor.document.reviews[&app.editor.surface_id()].comments[&2]
                .body
                .text
                .ends_with("Last words")
        );
    }
}

#[test]
fn callout_settings_remain_editable_from_the_pinned_composer_menu() {
    let (ctx, mut app) = setup();
    app.begin_review(None, None);
    frame(&mut app, &ctx, vec![]);
    let bounds = frame(&mut app, &ctx, vec![]);
    click(&mut app, &ctx, bounds["Add Callout"].center());
    let bounds = frame(&mut app, &ctx, vec![]);
    click(&mut app, &ctx, bounds["Callout settings"].center());
    let bounds = frame(&mut app, &ctx, vec![]);
    click(&mut app, &ctx, bounds["Blue"].center());
    let bounds = frame(&mut app, &ctx, vec![]);
    click(&mut app, &ctx, bounds["Green"].center());
    assert_eq!(
        app.review_surface()
            .draft
            .as_ref()
            .unwrap()
            .draft
            .body
            .callouts[0]
            .color,
        wireframe::review::Color::Green
    );
    assert!(app.editor.document.reviews.is_empty());
    app.post_review();
    assert_eq!(
        app.editor.document.reviews[&app.editor.surface_id()].comments[&1]
            .body
            .callouts[0]
            .color,
        wireframe::review::Color::Green
    );
}

#[test]
fn reviewer_settings_keep_text_focus_until_saved() {
    let (ctx, mut app) = setup();
    let root = tempfile::tempdir().unwrap();
    app.review.identity.load(root.path());
    frame(&mut app, &ctx, vec![]);
    let bounds = frame(&mut app, &ctx, vec![]);
    click(&mut app, &ctx, bounds["Reviewer settings"].center());
    let bounds = frame(&mut app, &ctx, vec![]);
    click(&mut app, &ctx, bounds["Reviewer"].center());
    frame(&mut app, &ctx, vec![key(Key::A, Modifiers::COMMAND)]);
    frame(&mut app, &ctx, vec![egui::Event::Text("Reviewer É".into())]);
    let bounds = frame(&mut app, &ctx, vec![]);
    assert_eq!(app.review.identity.name, "Reviewer É");
    click(&mut app, &ctx, bounds["Save name"].center());
    let mut reopened = identity::Identity::default();
    reopened.load(root.path());
    assert_eq!(reopened.author, app.review.identity.author);
    assert_eq!(reopened.name, "Reviewer É");
    assert!(!egui::Popup::is_any_open(&ctx));
}
