// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::{collections::HashMap, time::Instant};

fn key(key: Key, modifiers: Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}
fn frame(
    ctx: &egui::Context,
    app: &mut WireframeApp,
    events: Vec<egui::Event>,
) -> HashMap<String, Pos2> {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 850.))),
            events,
            ..Default::default()
        },
        |ui| {
            app.shortcuts(ui.ctx());
            app.versions_panel(ui);
            app.version_dialog(ui.ctx());
        },
    );
    let labels = output
        .platform_output
        .accesskit_update
        .as_ref()
        .map(|update| {
            update
                .nodes
                .iter()
                .filter_map(|(_, n)| Some((n.label()?.to_owned(), n.bounds()?)))
                .map(|(label, b)| {
                    (
                        label,
                        Pos2::new(((b.x0 + b.x1) / 2.) as f32, ((b.y0 + b.y1) / 2.) as f32),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    output.textures_delta.clear();
    labels
}
fn click(ctx: &egui::Context, app: &mut WireframeApp, label: &str) {
    let mut labels = HashMap::new();
    for _ in 0..3 {
        labels = frame(ctx, app, vec![]);
    }
    let pos = *labels
        .get(label)
        .unwrap_or_else(|| panic!("Missing {label}: {:?}", labels.keys()));
    for pressed in [true, false] {
        frame(
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
}
fn drain(app: &mut WireframeApp, ctx: &egui::Context) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while app.save_busy() {
        app.poll_saves(ctx);
        assert!(Instant::now() < deadline, "{:?}", app.error);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(app.error.is_none(), "{:?}", app.error);
}

#[test]
fn visible_version_controls_create_select_and_rename_with_immediate_text_replacement() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor.add(Kind::Button, 50., 60.);
    let official = app.editor.page_data().content.clone();
    click(&ctx, &mut app, "+");
    let alt = app.editor.active_alternate(1).unwrap();
    assert_eq!(app.editor.page_data().alternates[0].content, official);
    click(&ctx, &mut app, "Official version");
    assert_eq!(app.editor.surface_id(), 1);
    click(&ctx, &mut app, "Alternate 1");
    assert_eq!(app.editor.surface_id(), alt);
    click(&ctx, &mut app, "…");
    click(&ctx, &mut app, "Rename…");
    for _ in 0..3 {
        frame(&ctx, &mut app, vec![]);
    }
    let before = app.editor.document.clone();
    assert!(app.canvas_input_blocked());
    frame(&ctx, &mut app, vec![key(Key::D, Modifiers::COMMAND)]);
    assert_eq!(app.editor.document, before);
    frame(&ctx, &mut app, vec![egui::Event::Paste("Compact".into())]);
    assert_eq!(app.versions.rename.as_ref().unwrap().name, "Compact");
    frame(&ctx, &mut app, vec![key(Key::Enter, Modifiers::NONE)]);
    assert!(app.versions.rename.is_none());
    assert_eq!(app.editor.page_data().alternates[0].name, "Compact");
    assert_eq!(app.editor.page_data().content, official);
    app.editor.undo();
    assert_eq!(app.editor.page_data().alternates[0].name, "Alternate 1");
}

#[test]
fn rename_conflict_stays_in_dialog_and_cancel_preserves_both_versions() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    let a = app.editor.create_version("Compact").unwrap();
    app.editor.create_version("Detailed").unwrap();
    app.versions.rename = Some(Rename {
        page: 1,
        alternate: a,
        name: "Compact".into(),
        focus: true,
        error: None,
    });
    for _ in 0..3 {
        frame(&ctx, &mut app, vec![]);
    }
    let before = app.editor.document.clone();
    frame(&ctx, &mut app, vec![egui::Event::Paste("Detailed".into())]);
    frame(&ctx, &mut app, vec![key(Key::Enter, Modifiers::NONE)]);
    assert!(app.versions.rename.as_ref().unwrap().error.is_some());
    assert_eq!(app.editor.document, before);
    click(&ctx, &mut app, "Cancel");
    assert!(app.versions.rename.is_none());
    assert_eq!(app.editor.document, before);
}

#[test]
fn saved_page_and_version_choices_reopen_without_document_changes_and_survive_save_as() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.enable_persistence(&ctx, dir.path().to_owned());
    let landing = app.editor.page;
    let compact = app.editor.create_version("Compact").unwrap();
    app.editor.add_page();
    let contact = app.editor.page;
    let detailed = app.editor.create_version("Detailed").unwrap();
    let path = dir.path().join("Pages.wireframe");
    assert!(app.queue_save(path.clone(), true));
    drain(&mut app, &ctx);
    app.editor.switch_page(landing);
    app.remember_version_view();
    let bytes = std::fs::read(&path).unwrap();
    assert!(!app.editor.dirty());
    assert!(app.finish_session(false));
    let mut reopened = WireframeApp::workspace(&ctx, None);
    reopened.enable_persistence(&ctx, dir.path().to_owned());
    assert!(
        reopened.load_persistent(path.clone()),
        "{:?}",
        reopened.error
    );
    assert_eq!(reopened.editor.page, landing);
    assert_eq!(reopened.editor.surface_id(), compact);
    assert_eq!(reopened.editor.active_surface(contact), detailed);
    assert!(!reopened.editor.dirty());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    reopened.editor.switch_page(contact);
    let copy = dir.path().join("Pages copy.wireframe");
    assert!(reopened.queue_save(copy.clone(), true));
    drain(&mut reopened, &ctx);
    reopened.remember_version_view();
    assert!(reopened.finish_session(false));
    let mut copied = WireframeApp::workspace(&ctx, None);
    copied.enable_persistence(&ctx, dir.path().to_owned());
    assert!(copied.load_persistent(copy));
    assert_eq!(copied.editor.page, contact);
    assert_eq!(copied.editor.surface_id(), detailed);
    assert_eq!(copied.editor.active_surface(landing), compact);
    assert!(copied.finish_session(false));
}

#[test]
fn preferences_isolate_project_local_ids_ignore_stale_choices_and_preserve_unreadable_files() {
    let dir = tempfile::tempdir().unwrap();
    let mut prefs = preferences::Preferences::default();
    prefs.load(dir.path()).unwrap();
    let mut first = Editor::default();
    let alt = first.create_version("Compact").unwrap();
    prefs
        .remember(
            "first",
            &first,
            &[CatalogCategory::All],
            &HashSet::new(),
            true,
        )
        .unwrap();
    let mut second = Editor::new(first.document.clone());
    prefs
        .remember(
            "second",
            &second,
            &[CatalogCategory::All],
            &HashSet::new(),
            true,
        )
        .unwrap();
    let mut restored = preferences::Preferences::default();
    restored.load(dir.path()).unwrap();
    restored.restore("first", &mut second);
    assert_eq!(second.surface_id(), alt);
    restored.restore("second", &mut second);
    assert_eq!(second.surface_id(), 1);
    second.document.pages[0].alternates.clear();
    restored.restore("first", &mut second);
    assert_eq!(second.surface_id(), 1);
    let path = dir.path().join("version-view-settings.json");
    std::fs::write(&path, b"unreadable settings").unwrap();
    assert!(restored.load(dir.path()).is_err());
    assert!(
        restored
            .remember(
                "third",
                &first,
                &[CatalogCategory::All],
                &HashSet::new(),
                true
            )
            .is_err()
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"unreadable settings");
}

fn hierarchical_editor() -> Editor {
    let mut project = Project::default();
    let template = project.pages[0].clone();
    project.pages = [
        (1, "Parent", None),
        (2, "Child", Some(1)),
        (3, "Other", None),
    ]
    .into_iter()
    .map(|(id, name, parent)| wireframe::document::Page {
        id,
        name: name.into(),
        parent,
        ..template.clone()
    })
    .collect();
    project.next_id = 4;
    Editor::new(project)
}

#[test]
fn collapsed_pages_reopen_and_survive_recovery_and_save_as_without_editing_project_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.enable_persistence(&ctx, dir.path().to_owned());
    app.editor = hierarchical_editor();
    let tree = wireframe::page_tree::PageTree::new(&app.editor.document);
    app.navigator.restore(&tree, 1, HashSet::from([1]));
    app.thumbnails = false;
    let path = dir.path().join("Hierarchy.wireframe");
    assert!(app.queue_save(path.clone(), true));
    drain(&mut app, &ctx);
    app.remember_version_view();
    let key = app.version_preferences_key().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert!(!app.editor.dirty());
    assert!(app.finish_session(false));

    let mut reopened = WireframeApp::workspace(&ctx, None);
    reopened.enable_persistence(&ctx, dir.path().to_owned());
    assert!(reopened.load_persistent(path.clone()));
    assert_eq!(reopened.navigator.collapsed, HashSet::from([1]));
    assert!(!reopened.thumbnails);
    assert_eq!(reopened.navigator.selected, HashSet::from([1, 2]));
    assert!(!reopened.editor.dirty());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert!(reopened.finish_session(false));

    let mut recovered = WireframeApp::workspace(&ctx, None);
    recovered.enable_persistence(&ctx, dir.path().to_owned());
    assert!(recovered.restore_copy(key));
    assert_eq!(recovered.navigator.collapsed, HashSet::from([1]));
    assert!(!recovered.thumbnails);
    let copy = dir.path().join("Hierarchy copy.wireframe");
    assert!(recovered.queue_save(copy.clone(), true));
    drain(&mut recovered, &ctx);
    assert!(recovered.finish_session(false));
    let mut copied = WireframeApp::workspace(&ctx, None);
    copied.enable_persistence(&ctx, dir.path().to_owned());
    assert!(copied.load_persistent(copy));
    assert_eq!(copied.navigator.collapsed, HashSet::from([1]));
    assert!(!copied.thumbnails);
    assert_eq!(copied.navigator.selected, HashSet::from([1, 2]));
    let other = dir.path().join("Unrelated.wireframe");
    std::fs::write(&other, &bytes).unwrap();
    assert!(copied.load_persistent(other));
    assert!(copied.navigator.collapsed.is_empty());
    assert!(copied.thumbnails);
    assert_eq!(copied.navigator.selected, HashSet::from([1]));
    assert!(copied.finish_session(false));
}

#[test]
fn navigator_preferences_migrate_old_settings_and_remove_nonparents_and_stale_ids() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("version-view-settings.json");
    std::fs::write(&path, br#"{"old":{"page":1,"versions":{}}}"#).unwrap();
    let mut preferences = preferences::Preferences::default();
    preferences.load(dir.path()).unwrap();
    let mut editor = hierarchical_editor();
    let (_, closed, _) = preferences.restore("old", &mut editor);
    assert!(closed.is_empty());
    assert!(preferences.restore("old", &mut editor).2);
    let before = editor.document.clone();
    preferences
        .remember(
            "new",
            &editor,
            &[CatalogCategory::All],
            &HashSet::from([1, 2, 999]),
            false,
        )
        .unwrap();
    let mut reopened = preferences::Preferences::default();
    reopened.load(dir.path()).unwrap();
    let (_, closed, _) = reopened.restore("new", &mut editor);
    assert_eq!(closed, HashSet::from([1]));
    assert_eq!(editor.document, before);
    editor.document.pages[1].parent = None;
    assert!(reopened.restore("new", &mut editor).1.is_empty());
    let (_, closed, _) = reopened.restore("unrelated", &mut editor);
    assert!(closed.is_empty());
}

#[test]
fn restored_current_descendant_is_revealed_and_previous_project_focus_is_discarded() {
    let editor = hierarchical_editor();
    let tree = wireframe::page_tree::PageTree::new(&editor.document);
    let mut navigator = navigator::Navigator::default();
    navigator.selected = HashSet::from([999]);
    navigator.focus_ids.insert(egui::Id::new("old-project"));
    navigator.restore(&tree, 2, HashSet::from([1, 999]));
    assert!(navigator.collapsed.is_empty());
    assert!(navigator.focus_ids.is_empty());
    assert_eq!(navigator.selected, HashSet::from([2]));
    assert_eq!(navigator.scroll_to, Some(2));
}

#[test]
fn new_project_does_not_inherit_previous_projects_navigator_mode() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    app.editor = hierarchical_editor();
    app.navigator.collapsed.insert(1);
    app.thumbnails = false;
    app.perform(Pending::New, &ctx);
    assert!(app.navigator.collapsed.is_empty());
    assert!(app.thumbnails);
}

#[test]
fn presentation_back_restores_version_identity_and_handles_deleted_versions() {
    let ctx = egui::Context::default();
    let mut app = WireframeApp::workspace(&ctx, None);
    let landing = app.editor.page;
    let compact = app.editor.create_version("Compact").unwrap();
    app.editor.add_page();
    let contact = app.editor.page;
    app.editor.switch_page(landing);
    app.navigate_page(contact);
    app.editor.switch_version(landing, None).unwrap();
    app.editor.switch_page(contact);
    app.follow_target("back:", &ctx);
    assert_eq!(app.editor.surface_id(), compact);
    app.navigate_page(contact);
    app.editor.switch_page(landing);
    app.editor.discard_version(compact).unwrap();
    app.editor.switch_page(contact);
    let before = app.editor.document.clone();
    app.follow_target("back:", &ctx);
    assert_eq!(app.editor.surface_id(), landing);
    assert_eq!(app.editor.document, before);
}

#[test]
fn pdf_current_active_official_and_all_helpers_use_distinct_version_sets() {
    use wireframe::pdf::{Options, Selection};
    fn draw(
        ctx: &egui::Context,
        app: &mut WireframeApp,
        events: Vec<egui::Event>,
    ) -> HashMap<String, Pos2> {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100., 850.))),
                events,
                ..Default::default()
            },
            |ui| app.pdf_export_dialog(ui.ctx()),
        );
        let labels = output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .filter_map(|(_, n)| Some((n.label()?.to_owned(), n.bounds()?)))
            .map(|(label, b)| {
                (
                    label,
                    Pos2::new(((b.x0 + b.x1) / 2.) as f32, ((b.y0 + b.y1) / 2.) as f32),
                )
            })
            .collect();
        output.textures_delta.clear();
        labels
    }
    fn choose(ctx: &egui::Context, app: &mut WireframeApp, label: &str) {
        let mut labels = HashMap::new();
        for _ in 0..3 {
            labels = draw(ctx, app, vec![]);
        }
        let pos = labels[label];
        for pressed in [true, false] {
            draw(
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
    }
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    let first = app.editor.page;
    let alt = app.editor.create_version("Compact").unwrap();
    app.editor.add_page();
    let second = app.editor.page;
    let other = app.editor.create_version("Detailed").unwrap();
    app.editor.switch_version(second, None).unwrap();
    app.editor.switch_page(first);
    let before = app.editor.document.clone();
    app.pdf_options = Some(Options::default());
    choose(&ctx, &mut app, "Current page");
    assert_eq!(
        app.pdf_options.as_ref().unwrap().selection,
        Selection::Current(alt)
    );
    choose(&ctx, &mut app, "Choose pages");
    choose(&ctx, &mut app, "Active");
    assert_eq!(
        app.pdf_options.as_ref().unwrap().selection,
        Selection::Set([alt, second].into())
    );
    choose(&ctx, &mut app, "Official versions");
    assert_eq!(
        app.pdf_options.as_ref().unwrap().selection,
        Selection::Set([first, second].into())
    );
    choose(&ctx, &mut app, "Current");
    assert_eq!(
        app.pdf_options.as_ref().unwrap().selection,
        Selection::Set([alt].into())
    );
    choose(&ctx, &mut app, "All");
    assert_eq!(
        app.pdf_options.as_ref().unwrap().selection,
        Selection::Set([first, alt, second, other].into())
    );
    assert_eq!(app.editor.document, before);
}

#[test]
fn long_version_names_keep_actions_in_the_inspector_and_navigator_on_the_active_version() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut app = WireframeApp::workspace(&ctx, None);
    let name = "Long version שלום ".repeat(40).trim().to_owned();
    app.editor.create_version(&name).unwrap();
    let expected = format!("Wireframe 1 ({name})");
    for width in [860., 1100.] {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 850.))),
                ..Default::default()
            },
            |ui| {
                app.sidebar(ui);
                app.inspector(ui);
            },
        );
        output.textures_delta.clear();
        let nodes = &output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes;
        assert!(
            nodes
                .iter()
                .any(|(_, n)| n.label() == Some(expected.as_str())),
            "Labels: {:?}",
            nodes
                .iter()
                .filter_map(|(_, n)| n.label())
                .collect::<Vec<_>>()
        );
        for label in [name.as_str(), "…"] {
            let bounds = nodes
                .iter()
                .find(|(_, n)| n.label() == Some(label))
                .unwrap()
                .1
                .bounds()
                .unwrap();
            assert!(
                bounds.x0 >= 0. && bounds.x1 <= width as f64,
                "{label} {bounds:?}"
            );
        }
        output.textures_delta.clear();
    }
}
