// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use unicode_segmentation::UnicodeSegmentation;

pub(super) struct TextEditView {
    document: Project,
    session: String,
    page: Id,
    alternate: Option<Id>,
    context: Vec<Id>,
    definition: Option<Id>,
    library: Option<Id>,
    position: Pos2,
    width: f32,
    multiline: bool,
    select_all: bool,
    last_frame: Option<u64>,
    field_id: Option<egui::Id>,
    menu_active: bool,
    pub(super) preview: Option<wireframe::render::Scene>,
}

impl TextEditView {
    fn matches(&self, editor: &Editor) -> bool {
        self.session == editor.clipboard_session()
            && self.document == editor.document
            && self.page == editor.page
            && self.alternate == editor.active_alternate(editor.page)
            && self.context == editor.context
            && self.definition == editor.definition
            && self.library == editor.library
    }
}

impl WireframeApp {
    pub(super) fn canvas_text_menu_owner(&self) -> Option<egui::Id> {
        let view = self.text_edit_view.as_ref()?;
        (self.text_edit.is_some() && view.matches(&self.editor))
            .then_some(view.field_id)
            .flatten()
    }

    pub(super) fn text_session_exit_key(
        &self,
        ctx: &egui::Context,
        key: Key,
        modifiers: Modifiers,
    ) -> bool {
        let Some(view) = &self.text_edit_view else {
            return false;
        };
        if view.field_id.is_none() || view.field_id != ctx.memory(|m| m.focused()) {
            return false;
        }
        (key == Key::Escape && modifiers.matches_logically(Modifiers::NONE))
            || (!view.multiline && key == Key::Tab && modifiers.matches_logically(Modifiers::NONE))
            || (key == Key::Enter
                && (modifiers.matches_logically(Modifiers::COMMAND)
                    || (!view.multiline && modifiers.matches_logically(Modifiers::ALT))
                    || (!view.multiline && modifiers.matches_logically(Modifiers::NONE))))
    }

    fn accept_text_draft(&mut self, id: Id, text: &str) {
        match self.editor.control_text_from_edit(id, text) {
            Ok(text) => self.editor.edit("Edit text", |editor| {
                if let Some(mut control) =
                    editor.current().controls.into_iter().find(|c| c.id == id)
                {
                    control.text = text;
                    editor.update_control(control);
                }
            }),
            Err(error) => self.editor.error = Some(error),
        }
    }

    pub(super) fn commit_text_session(&mut self, ctx: &egui::Context) {
        if let Some(view) = self.text_edit_view.take() {
            if let Some((id, text)) = self.text_edit.take()
                && view.matches(&self.editor)
            {
                self.accept_text_draft(id, &text);
            }
            if let Some(id) = view.field_id {
                wireframe::text_entry::forget(ctx, id);
                ctx.memory_mut(|m| m.surrender_focus(id));
            }
        }
        self.text_edit_return_to_quick = false;
    }

    pub(super) fn begin_text_edit(&mut self, id: Id) {
        self.menus.cancel_text_command();
        self.end_pointer_gestures();
        self.text_edit_return_to_quick = false;
        self.text_edit_view = None;
        match self.editor.prepare_text_edit(id) {
            Ok(draft) => self.text_edit = draft,
            Err(error) => {
                self.editor.error = Some(error);
                self.text_edit = None;
            }
        }
        if self.text_edit.is_none() {
            return;
        }
        let Some(control) = self
            .editor
            .current()
            .controls
            .into_iter()
            .find(|c| c.id == id)
        else {
            return;
        };
        self.text_edit_view = Some(TextEditView {
            document: self.editor.document.clone(),
            session: self.editor.clipboard_session().into(),
            page: self.editor.page,
            alternate: self.editor.active_alternate(self.editor.page),
            context: self.editor.context.clone(),
            definition: self.editor.definition,
            library: self.editor.library,
            position: self.screen([control.bounds.x, control.bounds.y]),
            width: 750.,
            multiline: control.kind.multiline_text(),
            select_all: true,
            last_frame: None,
            field_id: None,
            menu_active: false,
            preview: None,
        });
    }

    pub(super) fn text_session_ui(&mut self, ctx: &egui::Context) {
        let Some(mut view) = self.text_edit_view.take() else {
            return;
        };
        if view.last_frame == Some(ctx.cumulative_frame_nr()) {
            self.text_edit_view = Some(view);
            return;
        }
        let Some((id, mut value)) = self.text_edit.take() else {
            return;
        };
        if !view.matches(&self.editor) {
            if let Some(id) = view.field_id {
                wireframe::text_entry::forget(ctx, id);
            }
            self.text_edit_return_to_quick = false;
            return;
        }
        let fresh = view.last_frame.is_none();
        view.last_frame = Some(ctx.cumulative_frame_nr());
        // Application menus temporarily own input without accepting this draft.
        // An ordinary click elsewhere still commits before changing context.
        let menu_active = self.save_busy()
            || self.menus.in_window_open(ctx)
            || ctx
                .input(|i| i.events.clone())
                .iter()
                .any(|event| self.menus.owns_in_window_event(ctx, event));
        if let Some(id) = view.field_id {
            if menu_active {
                ctx.memory_mut(|m| m.surrender_focus(id));
            } else if view.menu_active {
                wireframe::text_entry::restore_focus(ctx, id);
            }
        }
        let returning_from_menu = view.menu_active && !menu_active;
        view.menu_active = menu_active;
        let viewport = ctx.content_rect().shrink(8.);
        let width = view.width.min(viewport.width()).max(100.);
        let height = if view.multiline { 350. } else { 54. };
        let position = Pos2::new(
            view.position.x.clamp(
                viewport.left(),
                (viewport.right() - width).max(viewport.left()),
            ),
            view.position.y.clamp(
                viewport.top(),
                (viewport.bottom() - height).max(viewport.top()),
            ),
        );
        let before = value.clone();
        let mut submit = false;
        let mut transform = false;
        let mut cancel = false;
        let popup = egui::Area::new(egui::Id::new("canvas-text-session"))
            .order(egui::Order::Foreground)
            .fixed_pos(position)
            .movable(false)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_width((width - 14.).max(80.));
                    let field = if view.multiline {
                        TextInput::multiline(&mut value).desired_rows(17)
                    } else {
                        TextInput::singleline(&mut value)
                    };
                    let response = ui.add(
                        field
                            .id_salt("canvas-text-session-field")
                            .desired_width(f32::INFINITY)
                            .autofocus(true)
                            .select_all_on_open(view.select_all)
                            .submit_on_command_enter(true)
                            .capture_tab(true),
                    );
                    view.field_id = Some(response.id);
                    if returning_from_menu {
                        // The previous save modal can still disable this pass.
                        // Retry until the source field actually receives focus.
                        view.menu_active = !response.has_focus();
                        ctx.request_repaint();
                    }
                    submit = !menu_active
                        && (response.has_focus() || response.lost_focus())
                        && ui.input_mut(|i| {
                            let modified = i.consume_key(Modifiers::COMMAND, Key::Enter)
                                || (!view.multiline && i.consume_key(Modifiers::ALT, Key::Enter));
                            transform = !view.multiline && modified;
                            modified
                                || (!view.multiline
                                    && (i.consume_key(Modifiers::NONE, Key::Enter)
                                        || i.consume_key(Modifiers::NONE, Key::Tab)))
                        });
                    cancel = !menu_active
                        && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.small(value.graphemes(true).count().to_string());
                    });
                });
            });
        let outside = !fresh
            && ctx
                .input(|i| i.events.clone())
                .iter()
                .any(|event| match event {
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        ..
                    } => {
                        !popup.response.rect.contains(*pos)
                            && !self.menus.owns_in_window_event(ctx, event)
                    }
                    // Accessible activation does not emit pointer events. Commit
                    // before the target widget can switch the page or edit context.
                    egui::Event::AccessKitActionRequest(request)
                        if request.action == egui::accesskit::Action::Click =>
                    {
                        !self.menus.owns_in_window_event(ctx, event)
                            && view
                                .field_id
                                .is_some_and(|id| request.target_node != id.accesskit_id())
                    }
                    _ => false,
                });
        if cancel || submit || outside {
            if transform && !self.editor.can_transform_text(id) {
                wireframe::text_entry::restore_focus(ctx, view.field_id.unwrap());
                self.text_edit = Some((id, value));
                self.text_edit_view = Some(view);
                ctx.request_repaint();
                return;
            }
            if !cancel {
                self.accept_text_draft(id, &value);
            }
            if let Some(id) = view.field_id {
                wireframe::text_entry::forget(ctx, id);
            }
            if transform {
                match self.editor.transform_text_control(id) {
                    Ok(Some(new_id)) => {
                        let quick = self.text_edit_return_to_quick;
                        self.begin_text_edit(new_id);
                        self.text_edit_return_to_quick = quick;
                        if let Some((_, text)) = &mut self.text_edit {
                            text.push('\n');
                        }
                        if let Some(view) = &mut self.text_edit_view {
                            view.select_all = false;
                        }
                        ctx.request_repaint();
                        return;
                    }
                    Ok(None) => {}
                    Err(error) => self.editor.error = Some(error),
                }
            }
            let quick = std::mem::take(&mut self.text_edit_return_to_quick);
            if !outside {
                if quick {
                    self.quick_focus = true;
                } else if let Some(id) = self.canvas_focus_id {
                    ctx.memory_mut(|m| m.request_focus(id));
                }
            }
            ctx.request_repaint();
        } else {
            if value != before {
                match self.editor.text_preview_scene(id, &value) {
                    Ok(scene) => view.preview = Some(scene),
                    Err(error) => self.editor.error = Some(error),
                }
                ctx.request_repaint();
            }
            self.text_edit = Some((id, value));
            self.text_edit_view = Some(view);
        }
    }
}

#[cfg(test)]
mod retention_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readable_inline_drafts_keep_ids_for_submit_cancel_and_save_handoff() {
        for mode in [0, 1, 2] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            let source = app.editor.page;
            app.editor.add_page();
            let destination = app.editor.page;
            app.editor.rename_page(destination, "Contact").unwrap();
            app.editor.switch_page(source);
            app.editor.add(Kind::Paragraph, 50., 50.);
            let id = *app.editor.selection.iter().next().unwrap();
            app.editor
                .set_control_property(
                    id,
                    "text",
                    wireframe::document::Value::Text(format!("[Go](page:{destination})")),
                )
                .unwrap();
            frame(&ctx, &mut app, vec![]);
            let before = app.editor.document.clone();
            app.begin_text_edit(id);
            assert_eq!(app.text_edit.as_ref().unwrap().1, "[Go](Contact)");
            for _ in 0..3 {
                frame(&ctx, &mut app, vec![]);
            }
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::Paste(
                    "Edited [Go](Contact) [Home](Wireframe 1)".into(),
                )],
            );
            assert_eq!(app.editor.document, before);
            assert!(
                app.text_edit_view
                    .as_ref()
                    .unwrap()
                    .preview
                    .as_ref()
                    .unwrap()
                    .links
                    .iter()
                    .any(|l| l.target == format!("page:{destination}"))
            );
            if mode == 2 {
                app.commit_text_session(&ctx);
            } else {
                frame(
                    &ctx,
                    &mut app,
                    vec![egui::Event::Key {
                        key: if mode == 0 { Key::Escape } else { Key::Enter },
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: if mode == 0 {
                            Modifiers::NONE
                        } else {
                            Modifiers::COMMAND
                        },
                    }],
                );
            }
            assert!(app.text_edit.is_none());
            if mode == 0 {
                assert_eq!(app.editor.document, before);
            } else {
                assert_eq!(
                    app.editor.current().controls[0].text,
                    format!("Edited [Go](page:{destination}) [Home](page:{source})")
                );
                let after = app.editor.document.clone();
                app.editor.undo();
                assert_eq!(app.editor.document, before);
                app.editor.redo();
                assert_eq!(app.editor.document, after);
            }
        }
    }

    fn application_frame(
        ctx: &egui::Context,
        app: &mut WireframeApp,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        use eframe::App;
        let mut raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200., 850.))),
            events,
            ..Default::default()
        };
        app.raw_input_hook(ctx, &mut raw);
        ctx.run_ui(raw, |ui| app.ui(ui, &mut eframe::Frame::_new_kittest()))
    }

    #[test]
    fn accessible_page_activation_commits_the_text_draft_before_navigation() {
        for thumbnails in [false, true] {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.thumbnails = thumbnails;
            let first = app.editor.page;
            app.editor.add(Kind::Label, 50., 50.);
            let target = app.editor.current().controls[0].id;
            app.editor.add_page();
            let second = app.editor.page;
            app.editor.rename_page(second, "Destination").unwrap();
            app.editor.switch_page(first);
            for _ in 0..3 {
                application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
            }
            let original = app.editor.document.clone();
            app.begin_text_edit(target);
            for _ in 0..3 {
                application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
            }
            let out = application_frame(&ctx, &mut app, vec![]);
            let destination = out
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .find_map(|(id, node)| (node.label() == Some("Destination")).then_some(*id))
                .expect("Destination navigator entry");
            out.drop_without_applying_deltas();
            // The last text input and accessible activation can arrive in one frame.
            application_frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::Paste("Final headline".into()),
                    egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                        action: egui::accesskit::Action::Click,
                        target_tree: egui::accesskit::TreeId::ROOT,
                        target_node: destination,
                        data: None,
                    }),
                ],
            )
            .drop_without_applying_deltas();
            assert_eq!(app.editor.page, second);
            assert!(app.text_edit.is_none());
            app.editor.switch_page(first);
            assert_eq!(app.editor.current().controls[0].text, "Final headline");
            assert_eq!(app.editor.undo_name(), Some("Edit text"));
            app.editor.undo();
            assert_eq!(app.editor.document, original);
        }
    }

    #[test]
    fn accessible_activation_inside_the_editor_keeps_the_draft_open() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let (mut app, _, _) = setup(&ctx);
        let original = app.editor.document.clone();
        let field = app.text_edit_view.as_ref().unwrap().field_id.unwrap();
        frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::Paste("Still editing".into()),
                egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                    action: egui::accesskit::Action::Click,
                    target_tree: egui::accesskit::TreeId::ROOT,
                    target_node: field.accesskit_id(),
                    data: None,
                }),
            ],
        );
        assert_eq!(app.text_edit.as_ref().unwrap().1, "Still editing");
        assert_eq!(app.editor.document, original);
    }

    fn frame(ctx: &egui::Context, app: &mut WireframeApp, events: Vec<egui::Event>) {
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
                time: Some(ctx.cumulative_frame_nr() as f64 * 0.1),
                events,
                ..Default::default()
            },
            |ui| {
                app.text_session_ui(ui.ctx());
                app.canvas(ui);
                app.dialogs(ui.ctx());
            },
        )
        .drop_without_applying_deltas();
    }

    fn setup(ctx: &egui::Context) -> (WireframeApp, Id, Id) {
        let mut app = WireframeApp::workspace(ctx, None);
        app.editor.add(Kind::Button, 50., 50.);
        let target = app.editor.current().controls[0].id;
        app.editor.add(Kind::Button, 50., 250.);
        let other = app.editor.current().controls[1].id;
        app.editor.select(target, false);
        frame(ctx, &mut app, vec![]);
        app.begin_text_edit(target);
        for _ in 0..3 {
            frame(ctx, &mut app, vec![]);
        }
        (app, target, other)
    }

    #[test]
    fn fallback_edit_menu_keeps_text_history_local() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let (mut app, _, _) = setup(&ctx);
        let original = app.editor.document.clone();
        application_frame(
            &ctx,
            &mut app,
            vec![egui::Event::Paste("Provisional".into())],
        )
        .drop_without_applying_deltas();
        let click = |app: &mut WireframeApp, label: &str| {
            let out = application_frame(&ctx, app, vec![]);
            let node = &out
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .filter(|(_, node)| node.label() == Some(label))
                .max_by(|(_, a), (_, b)| a.bounds().unwrap().x0.total_cmp(&b.bounds().unwrap().x0))
                .unwrap_or_else(|| panic!("Missing {label}"))
                .1;
            let b = node.bounds().unwrap();
            let pos = egui::pos2(((b.x0 + b.x1) / 2.) as f32, ((b.y0 + b.y1) / 2.) as f32);
            out.drop_without_applying_deltas();
            for pressed in [true, false] {
                application_frame(
                    &ctx,
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
                )
                .drop_without_applying_deltas();
            }
        };
        click(&mut app, "Edit");
        assert_eq!(
            app.editor.document, original,
            "opening Edit must not commit the canvas draft"
        );
        assert_eq!(app.text_edit.as_ref().unwrap().1, "Provisional");
        click(&mut app, "Select all");
        for _ in 0..3 {
            application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
        }
        application_frame(
            &ctx,
            &mut app,
            vec![egui::Event::Paste("Replacement".into())],
        )
        .drop_without_applying_deltas();
        assert_eq!(app.text_edit.as_ref().unwrap().1, "Replacement");
        click(&mut app, "Edit");
        click(&mut app, "Undo");
        for _ in 0..3 {
            application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
        }
        assert_eq!(app.text_edit.as_ref().unwrap().1, "Provisional");
        assert_eq!(
            app.editor.document, original,
            "menu Undo must stay in draft history"
        );
    }

    #[test]
    fn queued_menu_edit_cannot_reach_a_replacement_draft_or_modal() {
        for replace in [false, true] {
            let ctx = egui::Context::default();
            let (mut app, _, other) = setup(&ctx);
            let original = app.editor.document.clone();
            app.run_command(&ctx, menus::Command::Cut);
            if replace {
                app.begin_text_edit(other);
            } else {
                app.error = Some("An intervening failure".into());
            }
            application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
            assert_eq!(app.editor.document, original);
            assert_eq!(app.text_edit.as_ref().unwrap().1, "Button");
        }
    }

    #[test]
    fn in_window_file_menu_keeps_the_draft_provisional_and_restores_focus() {
        for (menu, save, accessible, worker) in [
            ("File", false, false, false),
            ("File", true, false, false),
            ("File", false, true, false),
            ("File", true, true, false),
            ("File", true, false, true),
            ("File", true, true, true),
            ("Edit", false, false, false),
            ("Edit", false, true, false),
            ("View", false, false, false),
            ("View", false, true, false),
            ("Help", false, false, false),
            ("Help", false, true, false),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            let (mut app, target, other) = setup(&ctx);
            app.commit_text_session(&ctx);
            let baseline = app.editor.document.clone();
            let path = dir.path().join("Menu.wireframe");
            ProjectStore.save(&path, &baseline).unwrap();
            app.file = Some(path.clone());
            app.editor.mark_saved();
            if worker {
                app.enable_persistence(&ctx, dir.path().join("working-copies"));
                app.set_save_mode(false);
            }
            app.accept_text_draft(other, "Earlier change");
            let committed = app.editor.document.clone();
            application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
            app.begin_text_edit(target);
            for _ in 0..3 {
                application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
            }
            application_frame(
                &ctx,
                &mut app,
                vec![egui::Event::Paste("Provisional".into())],
            )
            .drop_without_applying_deltas();
            let point = |out: &egui::FullOutput, label: &str| {
                let (id, node) = out
                    .platform_output
                    .accesskit_update
                    .as_ref()
                    .unwrap()
                    .nodes
                    .iter()
                    .find(|(_, node)| {
                        node.label()
                            .is_some_and(|s| s == label || s.starts_with(&format!("{label}  ")))
                    })
                    .unwrap();
                let bounds = node.bounds().unwrap();
                (
                    *id,
                    egui::pos2(
                        ((bounds.x0 + bounds.x1) / 2.) as f32,
                        ((bounds.y0 + bounds.y1) / 2.) as f32,
                    ),
                )
            };
            let out = application_frame(&ctx, &mut app, vec![]);
            let file = point(&out, menu);
            out.drop_without_applying_deltas();
            let click = |app: &mut WireframeApp, (id, pos): (egui::accesskit::NodeId, Pos2)| {
                if accessible {
                    application_frame(
                        &ctx,
                        app,
                        vec![egui::Event::AccessKitActionRequest(
                            egui::accesskit::ActionRequest {
                                action: egui::accesskit::Action::Click,
                                target_tree: egui::accesskit::TreeId::ROOT,
                                target_node: id,
                                data: None,
                            },
                        )],
                    )
                    .drop_without_applying_deltas();
                    return;
                }
                for pressed in [true, false] {
                    application_frame(
                        &ctx,
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
                    )
                    .drop_without_applying_deltas();
                }
            };
            click(&mut app, file);
            assert_eq!(
                app.text_edit.as_ref().map(|(_, s)| s.as_str()),
                Some("Provisional")
            );
            assert!(app.editor.document == committed);
            assert!(ProjectStore.load(&path).unwrap() == baseline);
            if save {
                let out = application_frame(&ctx, &mut app, vec![]);
                let save = point(&out, "Save");
                out.drop_without_applying_deltas();
                click(&mut app, save);
                let deadline = Instant::now() + Duration::from_secs(10);
                while app.save_busy() {
                    assert!(Instant::now() < deadline, "Save timed out: {:?}", app.error);
                    std::thread::sleep(Duration::from_millis(5));
                    application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
                }
                assert!(ProjectStore.load(&path).unwrap() == committed);
            } else {
                application_frame(
                    &ctx,
                    &mut app,
                    vec![egui::Event::Key {
                        key: Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::NONE,
                    }],
                )
                .drop_without_applying_deltas();
                assert!(ProjectStore.load(&path).unwrap() == baseline);
            }
            for _ in 0..3 {
                application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
            }
            assert_eq!(
                app.text_edit.as_ref().map(|(_, s)| s.as_str()),
                Some("Provisional")
            );
            assert!(!egui::Popup::is_any_open(&ctx));
            application_frame(&ctx, &mut app, vec![egui::Event::Text(" continued".into())])
                .drop_without_applying_deltas();
            assert_eq!(app.text_edit.as_ref().unwrap().1, "Provisional continued");
            assert!(app.editor.document == committed);
        }
    }

    #[test]
    fn batched_canvas_text_exit_routes_following_undo_and_save_to_document() {
        for multiline in [false, true] {
            for cancel in [true, false] {
                for batch_typing in [false, true] {
                    let dir = tempfile::tempdir().unwrap();
                    let ctx = egui::Context::default();
                    let mut app = WireframeApp::workspace(&ctx, None);
                    app.editor.add(
                        if multiline {
                            Kind::Paragraph
                        } else {
                            Kind::Button
                        },
                        50.,
                        50.,
                    );
                    let target = app.editor.current().controls[0].id;
                    app.editor.add(Kind::Button, 50., 250.);
                    let other = app.editor.current().controls[1].id;
                    let original = app.editor.document.clone();
                    let path = dir.path().join("Ordered.wireframe");
                    ProjectStore.save(&path, &original).unwrap();
                    app.file = Some(path.clone());
                    app.editor.mark_saved();
                    app.accept_text_draft(other, "Earlier committed change");
                    let committed = app.editor.document.clone();
                    for _ in 0..3 {
                        application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
                    }
                    app.begin_text_edit(target);
                    for _ in 0..3 {
                        application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
                    }
                    let key = |key, modifiers| egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers,
                    };
                    let mut events = vec![egui::Event::Paste("Latest draft".into())];
                    if !batch_typing {
                        application_frame(&ctx, &mut app, std::mem::take(&mut events))
                            .drop_without_applying_deltas();
                    }
                    events.extend([
                        key(
                            if cancel { Key::Escape } else { Key::Enter },
                            if !cancel && multiline {
                                Modifiers::COMMAND
                            } else {
                                Modifiers::NONE
                            },
                        ),
                        key(Key::Z, Modifiers::COMMAND),
                        key(Key::S, Modifiers::COMMAND),
                    ]);
                    application_frame(&ctx, &mut app, events).drop_without_applying_deltas();
                    for _ in 0..8 {
                        application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
                    }
                    assert!(app.text_edit.is_none());
                    let expected = if cancel { &original } else { &committed };
                    assert_eq!(
                        &app.editor.document, expected,
                        "multiline={multiline}, cancel={cancel}, batch_typing={batch_typing}"
                    );
                    assert_eq!(&ProjectStore.load(&path).unwrap(), expected);
                    assert!(!app.editor.dirty());
                    app.editor.redo();
                    if cancel {
                        assert_eq!(app.editor.document, committed);
                    } else {
                        assert_eq!(app.editor.current().controls[0].text, "Latest draft");
                        app.editor.undo();
                        assert_eq!(app.editor.document, committed);
                    }
                }
            }
        }
    }

    #[test]
    fn multiline_return_keeps_following_text_in_the_provisional_draft() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Paragraph, 50., 50.);
        let target = app.editor.current().controls[0].id;
        let original = app.editor.document.clone();
        let path = dir.path().join("Multiline.wireframe");
        ProjectStore.save(&path, &original).unwrap();
        app.file = Some(path.clone());
        app.editor.mark_saved();
        for _ in 0..3 {
            application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
        }
        app.begin_text_edit(target);
        for _ in 0..3 {
            application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
        }
        let key = |key, modifiers| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        application_frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::Paste("First".into()),
                key(Key::Enter, Modifiers::NONE),
                egui::Event::Text("Second".into()),
                key(Key::S, Modifiers::COMMAND),
            ],
        )
        .drop_without_applying_deltas();
        for _ in 0..4 {
            application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
        }
        assert_eq!(app.text_edit.as_ref().unwrap().1, "First\nSecond");
        assert_eq!(app.editor.document, original);
        assert_eq!(ProjectStore.load(&path).unwrap(), original);
        application_frame(&ctx, &mut app, vec![key(Key::Z, Modifiers::COMMAND)])
            .drop_without_applying_deltas();
        assert!(app.text_edit.is_some());
        assert_ne!(app.text_edit.as_ref().unwrap().1, "First\nSecond");
        assert_eq!(app.editor.document, original);
    }

    #[test]
    fn ordinary_save_keeps_canvas_text_draft_provisional() {
        for (native_command, worker) in [(false, false), (true, false), (false, true), (true, true)]
        {
            let dir = tempfile::tempdir().unwrap();
            let ctx = egui::Context::default();
            let (mut app, target, other) = setup(&ctx);
            let original = app.editor.document.clone();
            let path = dir.path().join("Original.wireframe");
            ProjectStore.save(&path, &original).unwrap();
            app.file = Some(path.clone());
            app.editor.mark_saved();
            if worker {
                app.enable_persistence(&ctx, dir.path().join("working-copies"));
                app.set_save_mode(false);
            }
            app.commit_text_session(&ctx);
            app.accept_text_draft(other, "Earlier committed change");
            let committed = app.editor.document.clone();
            assert_ne!(committed, original);
            assert!(app.editor.dirty());
            application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
            app.begin_text_edit(target);
            for _ in 0..3 {
                application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
            }
            assert_eq!(ProjectStore.load(&path).unwrap(), original);
            let command = if native_command {
                egui::Event::AppCommand(menus::Command::Save as u64)
            } else {
                egui::Event::Key {
                    key: Key::S,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::COMMAND,
                }
            };
            application_frame(
                &ctx,
                &mut app,
                vec![egui::Event::Paste("Still provisional".into()), command],
            )
            .drop_without_applying_deltas();
            for _ in 0..3 {
                application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
            }
            let deadline = Instant::now() + Duration::from_secs(10);
            while app.save_busy() {
                assert!(Instant::now() < deadline, "Save timed out: {:?}", app.error);
                std::thread::sleep(Duration::from_millis(5));
                application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
            }
            assert_eq!(app.text_edit.as_ref().unwrap().1, "Still provisional");
            assert_eq!(app.editor.document, committed);
            assert_eq!(ProjectStore.load(&path).unwrap(), committed);
            assert!(!app.editor.dirty());
            application_frame(&ctx, &mut app, vec![egui::Event::Text(" continued".into())])
                .drop_without_applying_deltas();
            assert_eq!(
                app.text_edit.as_ref().unwrap().1,
                "Still provisional continued"
            );
            application_frame(
                &ctx,
                &mut app,
                vec![egui::Event::Key {
                    key: Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
            )
            .drop_without_applying_deltas();
            assert!(app.text_edit.is_none());
            assert!(app.editor.document == committed);
            assert!(!app.editor.dirty());
            app.editor.undo();
            assert!(app.editor.document == original);
            app.editor.redo();
            assert!(app.editor.document == committed);
        }
    }

    #[test]
    fn save_as_accepts_the_ordered_text_draft_before_chooser_cancel_or_save() {
        for native_command in [false, true] {
            for accept in [false, true] {
                let dir = tempfile::tempdir().unwrap();
                let ctx = egui::Context::default();
                let (mut app, target, _) = setup(&ctx);
                let original = app.editor.document.clone();
                let original_path = dir.path().join("Original.wireframe");
                ProjectStore.save(&original_path, &original).unwrap();
                app.file = Some(original_path.clone());
                let destination = dir.path().join("Copy – 世界.wireframe");
                app.save_dialog_choice = accept.then(|| destination.clone());
                let command = if native_command {
                    egui::Event::AppCommand(menus::Command::SaveAs as u64)
                } else {
                    egui::Event::Key {
                        key: Key::S,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::COMMAND | Modifiers::SHIFT,
                    }
                };
                application_frame(
                    &ctx,
                    &mut app,
                    vec![egui::Event::Paste("Focus boundary".into()), command],
                )
                .drop_without_applying_deltas();
                for _ in 0..3 {
                    application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
                }
                assert!(app.text_edit.is_none(), "Save As must accept the draft");
                assert_eq!(app.editor.current().controls[0].text, "Focus boundary");
                assert_eq!(app.editor.current().controls[0].id, target);
                assert_eq!(app.editor.undo_name(), Some("Edit text"));
                assert_eq!(ProjectStore.load(&original_path).unwrap(), original);
                if accept {
                    assert_eq!(app.file.as_ref(), Some(&destination));
                    assert_eq!(
                        ProjectStore.load(&destination).unwrap(),
                        app.editor.document
                    );
                } else {
                    assert_eq!(app.file.as_ref(), Some(&original_path));
                    assert!(!destination.exists());
                }
                let edited = app.editor.document.clone();
                app.editor.undo();
                assert_eq!(app.editor.document, original);
                app.editor.redo();
                assert_eq!(app.editor.document, edited);
            }
        }
    }

    #[test]
    fn preview_is_visible_but_not_in_saved_document_and_escape_preserves_redo() {
        let ctx = egui::Context::default();
        let (mut app, target, _) = setup(&ctx);
        // Prepare a valid Redo before starting a new session.
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
        app.editor.add(Kind::Label, 500., 500.);
        app.editor.undo();
        app.begin_text_edit(target);
        for _ in 0..3 {
            frame(&ctx, &mut app, vec![]);
        }
        let original = app.editor.document.clone();
        let redo = app.editor.redo_name().map(str::to_owned);
        frame(&ctx, &mut app, vec![egui::Event::Paste("Gamma".into())]);
        let scene = app
            .text_edit_view
            .as_ref()
            .unwrap()
            .preview
            .as_ref()
            .unwrap();
        let text: String = scene
            .items
            .iter()
            .filter_map(|item| match &item.drawing {
                wireframe::render::Drawing::Glyph(g) => Some(g.text.as_str()),
                _ => None,
            })
            .collect();
        assert!(text.contains("Gamma"), "{text}");
        assert_eq!(app.editor.document, original);
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
        assert!(app.text_edit.is_none());
        assert!(app.text_edit_view.is_none());
        assert_eq!(app.editor.document, original);
        assert_eq!(app.editor.redo_name(), redo.as_deref());
    }

    #[test]
    fn aligned_text_preview_and_commit_match_without_saving_the_draft() {
        for align in ["center", "right"] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(Kind::Label, 300., 200.);
            let target = *app.editor.selection.iter().next().unwrap();
            app.editor
                .set_control_property(
                    target,
                    "align",
                    wireframe::document::Value::Choice(align.into()),
                )
                .unwrap();
            frame(&ctx, &mut app, vec![]);
            let original = app.editor.document.clone();
            app.begin_text_edit(target);
            for _ in 0..3 {
                frame(&ctx, &mut app, vec![]);
            }
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::Paste("Longer label text".into())],
            );
            let glyphs = |scene: &wireframe::render::Scene| {
                scene
                    .items
                    .iter()
                    .filter_map(|item| match &item.drawing {
                        wireframe::render::Drawing::Glyph(g) => {
                            Some((g.text.clone(), g.x, g.baseline, g.advance))
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            };
            let preview = glyphs(
                app.text_edit_view
                    .as_ref()
                    .unwrap()
                    .preview
                    .as_ref()
                    .unwrap(),
            );
            assert!(!preview.is_empty());
            assert_eq!(app.editor.document, original);
            frame(
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
            assert!(app.text_edit.is_none());
            assert_eq!(preview, glyphs(&app.editor.editing_scene().unwrap()));
            assert!(app.editor.current().controls[0].bounds.x < 300.);
            app.editor.undo();
            assert_eq!(app.editor.document, original);
        }
    }

    #[test]
    fn structured_text_popup_keeps_row_metadata_and_history_together() {
        use wireframe::document::{IconSource, Link, Value};
        for (new_text, selected, metadata_row) in [
            ("Beta\nGamma\nAlpha", 0., Some(0)),
            ("Alpha\nRenamed\nGamma", 1., Some(1)),
            ("Alpha\nGamma", -1., None),
        ] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(Kind::List, 50., 200.);
            let target = *app.editor.selection.iter().next().unwrap();
            let destination = app.editor.page;
            let camera = IconSource::Catalog("camera-solid".into());
            for (key, value) in [
                ("text", Value::Text("Alpha\nBeta\nGamma".into())),
                ("selected_index", Value::Number(1.)),
                (
                    "row_links",
                    Value::Links(vec![None, Some(Link::Page(destination)), None]),
                ),
                (
                    "row_icons",
                    Value::Icons(vec![None, Some(camera.clone()), None]),
                ),
            ] {
                app.editor.set_control_property(target, key, value).unwrap();
            }
            frame(&ctx, &mut app, vec![]);
            let original = app.editor.document.clone();
            for cancel in [true, false] {
                app.begin_text_edit(target);
                for _ in 0..3 {
                    frame(&ctx, &mut app, vec![]);
                }
                frame(&ctx, &mut app, vec![egui::Event::Paste(new_text.into())]);
                assert_eq!(app.editor.document, original);
                frame(
                    &ctx,
                    &mut app,
                    vec![egui::Event::Key {
                        key: if cancel { Key::Escape } else { Key::Enter },
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: if cancel {
                            Modifiers::NONE
                        } else {
                            Modifiers::COMMAND
                        },
                    }],
                );
                assert!(app.text_edit.is_none());
                if cancel {
                    assert_eq!(app.editor.document, original);
                }
            }
            let properties = app.editor.control_properties(target).unwrap().1;
            let count = new_text.lines().count();
            let mut links = vec![None; count];
            let mut icons = vec![None; count];
            if let Some(row) = metadata_row {
                links[row] = Some(Link::Page(destination));
                icons[row] = Some(camera);
            }
            assert_eq!(properties["text"], Value::Text(new_text.into()));
            assert_eq!(properties["selected_index"], Value::Number(selected));
            assert_eq!(properties["row_links"], Value::Links(links));
            assert_eq!(properties["row_icons"], Value::Icons(icons));
            let edited = app.editor.document.clone();
            app.editor.undo();
            assert_eq!(app.editor.document, original);
            app.editor.redo();
            assert_eq!(app.editor.document, edited);
        }
    }

    #[test]
    fn nudge_after_text_dismissal_keeps_canvas_focus_for_history_and_delete() {
        for cancel in [false, true] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(Kind::List, 50., 200.);
            let target = *app.editor.selection.iter().next().unwrap();
            let input = |key, modifiers| {
                vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                }]
            };
            for _ in 0..3 {
                application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
            }
            app.begin_text_edit(target);
            for _ in 0..3 {
                application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
            }
            application_frame(
                &ctx,
                &mut app,
                input(
                    if cancel { Key::Escape } else { Key::Enter },
                    if cancel {
                        Modifiers::NONE
                    } else {
                        Modifiers::COMMAND
                    },
                ),
            )
            .drop_without_applying_deltas();
            assert!(app.text_edit.is_none());
            application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
            assert_eq!(ctx.memory(|m| m.focused()), app.canvas_focus_id);
            let original = app.editor.document.clone();
            application_frame(&ctx, &mut app, input(Key::ArrowRight, Modifiers::NONE))
                .drop_without_applying_deltas();
            assert_eq!(app.editor.current().controls[0].bounds.x, 51.);
            assert_eq!(
                ctx.memory(|m| m.focused()),
                app.canvas_focus_id,
                "A consumed canvas nudge must not also navigate interface focus"
            );
            application_frame(&ctx, &mut app, input(Key::Z, Modifiers::COMMAND))
                .drop_without_applying_deltas();
            assert_eq!(app.editor.document, original);
            application_frame(&ctx, &mut app, input(Key::Backspace, Modifiers::NONE))
                .drop_without_applying_deltas();
            assert!(app.editor.current().controls.is_empty());
            application_frame(&ctx, &mut app, input(Key::Z, Modifiers::COMMAND))
                .drop_without_applying_deltas();
            assert_eq!(app.editor.document, original);
            application_frame(&ctx, &mut app, input(Key::Enter, Modifiers::NONE))
                .drop_without_applying_deltas();
            assert!(app.text_edit.is_some());
        }
    }

    #[test]
    fn outside_click_commits_then_selects_the_clicked_control_in_one_action() {
        let ctx = egui::Context::default();
        let (mut app, target, other) = setup(&ctx);
        let original = app.editor.document.clone();
        frame(&ctx, &mut app, vec![egui::Event::Paste("Gamma".into())]);
        let pos = app.screen([70., 265.]);
        for pressed in [true, false] {
            frame(
                &ctx,
                &mut app,
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
        assert!(app.text_edit.is_none());
        assert_eq!(
            app.editor
                .current()
                .controls
                .iter()
                .find(|c| c.id == target)
                .unwrap()
                .text,
            "Gamma"
        );
        assert_eq!(app.editor.selection, [other].into_iter().collect());
        assert_eq!(app.editor.undo_name(), Some("Edit text"));
        app.editor.undo();
        assert_eq!(app.editor.document, original);
    }

    #[test]
    fn replacement_editor_session_cannot_receive_a_stale_draft() {
        let ctx = egui::Context::default();
        let (mut app, _, _) = setup(&ctx);
        frame(&ctx, &mut app, vec![egui::Event::Paste("Gamma".into())]);
        let original = app.editor.document.clone();
        app.editor = Editor::new(original.clone());
        frame(&ctx, &mut app, vec![]);
        assert!(app.text_edit.is_none());
        assert!(app.text_edit_view.is_none());
        assert_eq!(app.editor.document, original);
    }
    fn press(key: Key, modifiers: Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn single_line_conversion_preserves_position_and_separate_text_history() {
        for (kind, target, width, height) in [
            (Kind::Label, Kind::Paragraph, 275., 80.),
            (Kind::TextInput, Kind::TextArea, 200., 140.),
        ] {
            for modifiers in [Modifiers::COMMAND, Modifiers::ALT] {
                for cancel in [true, false] {
                    let ctx = egui::Context::default();
                    let mut app = WireframeApp::workspace(&ctx, None);
                    app.editor.add(kind, 400., 300.);
                    let old_id = *app.editor.selection.iter().next().unwrap();
                    let original = app.editor.document.clone();
                    app.begin_text_edit(old_id);
                    for _ in 0..3 {
                        frame(&ctx, &mut app, vec![]);
                    }
                    frame(
                        &ctx,
                        &mut app,
                        vec![egui::Event::Paste("Draft café".into())],
                    );
                    assert_eq!(app.editor.document, original);
                    frame(&ctx, &mut app, vec![press(Key::Enter, modifiers)]);
                    let converted = app.editor.document.clone();
                    let view = app.editor.selected_views()[0].clone();
                    assert_eq!(view.kind, target);
                    assert_ne!(view.id, old_id);
                    assert_eq!(view.bounds, Bounds::new(400., 300., width, height));
                    assert_eq!(view.text, "Draft café");
                    assert_eq!(app.text_edit.as_ref().unwrap().1, "Draft café\n");
                    for _ in 0..3 {
                        frame(&ctx, &mut app, vec![]);
                    }
                    frame(&ctx, &mut app, vec![egui::Event::Text("Tail".into())]);
                    assert_eq!(app.text_edit.as_ref().unwrap().1, "Draft café\nTail");
                    frame(
                        &ctx,
                        &mut app,
                        vec![press(
                            if cancel { Key::Escape } else { Key::Enter },
                            if cancel {
                                Modifiers::NONE
                            } else {
                                Modifiers::COMMAND
                            },
                        )],
                    );
                    assert!(app.text_edit.is_none());
                    if cancel {
                        assert_eq!(app.editor.document, converted);
                    } else {
                        assert_eq!(app.editor.selected_views()[0].text, "Draft café\nTail");
                        app.editor.undo();
                        assert_eq!(app.editor.document, converted);
                    }
                    app.editor.undo();
                    assert_eq!(app.editor.selected_views()[0].id, old_id);
                    assert_eq!(app.editor.selected_views()[0].kind, kind);
                    assert_eq!(app.editor.selected_views()[0].text, "Draft café");
                    app.editor.undo();
                    assert_eq!(app.editor.document, original);
                    app.editor.redo();
                    app.editor.redo();
                    assert_eq!(app.editor.document, converted);
                }
            }
        }
    }

    #[test]
    fn single_line_tab_and_modified_return_accept_nonconvertible_text() {
        for (key, modifiers) in [
            (Key::Tab, Modifiers::NONE),
            (Key::Enter, Modifiers::ALT),
            (Key::Enter, Modifiers::COMMAND),
        ] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(Kind::Button, 200., 150.);
            let id = *app.editor.selection.iter().next().unwrap();
            let original = app.editor.document.clone();
            app.begin_text_edit(id);
            for _ in 0..3 {
                frame(&ctx, &mut app, vec![]);
            }
            frame(&ctx, &mut app, vec![egui::Event::Paste("Accepted".into())]);
            frame(&ctx, &mut app, vec![press(key, modifiers)]);
            assert!(app.text_edit.is_none(), "{key:?} {modifiers:?}");
            assert_eq!(app.editor.selected_views()[0].text, "Accepted");
            assert_eq!(app.editor.selected_views()[0].kind, Kind::Button);
            app.editor.undo();
            assert_eq!(app.editor.document, original);
        }
    }

    #[test]
    fn blocked_conversion_retains_the_provisional_draft_and_redo() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Label, 200., 150.);
        let id = *app.editor.selection.iter().next().unwrap();
        app.editor.lock(true);
        app.editor.lock(false);
        app.editor.undo();
        let original = app.editor.document.clone();
        let undo = app.editor.undo_name().map(str::to_owned);
        let redo = app.editor.redo_name().map(str::to_owned);
        assert!(redo.is_some());
        app.begin_text_edit(id);
        for _ in 0..3 {
            frame(&ctx, &mut app, vec![]);
        }
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::Paste("Provisional".into())],
        );
        frame(&ctx, &mut app, vec![press(Key::Enter, Modifiers::COMMAND)]);
        assert_eq!(app.text_edit.as_ref().unwrap().1, "Provisional");
        assert_eq!(app.editor.document, original);
        assert_eq!(app.editor.undo_name(), undo.as_deref());
        frame(&ctx, &mut app, vec![press(Key::Escape, Modifiers::NONE)]);
        assert_eq!(app.editor.document, original);
        assert_eq!(app.editor.redo_name(), redo.as_deref());
    }
    #[test]
    fn multiline_tab_inserts_text_without_leaving_the_editor() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::TextArea, 200., 150.);
        let id = *app.editor.selection.iter().next().unwrap();
        let original = app.editor.document.clone();
        app.begin_text_edit(id);
        for _ in 0..3 {
            frame(&ctx, &mut app, vec![]);
        }
        frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::Text("Before".into()),
                press(Key::Tab, Modifiers::NONE),
                egui::Event::Text("After".into()),
            ],
        );
        assert_eq!(app.text_edit.as_ref().unwrap().1, "Before\tAfter");
        assert_eq!(app.editor.document, original);
        frame(&ctx, &mut app, vec![press(Key::Enter, Modifiers::COMMAND)]);
        assert_eq!(app.editor.selected_views()[0].text, "Before\tAfter");
        app.editor.undo();
        assert_eq!(app.editor.document, original);
    }
    #[test]
    fn ordered_conversion_reopens_before_following_text_and_acceptance() {
        for modifiers in [Modifiers::ALT, Modifiers::COMMAND] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(Kind::Label, 400., 300.);
            let id = *app.editor.selection.iter().next().unwrap();
            app.begin_text_edit(id);
            for _ in 0..3 {
                application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
            }
            application_frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::Paste("First".into()),
                    press(Key::Enter, modifiers),
                    egui::Event::Text("Second".into()),
                    press(Key::Enter, Modifiers::COMMAND),
                ],
            )
            .drop_without_applying_deltas();
            for _ in 0..10 {
                application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
            }
            assert!(app.text_edit.is_none());
            assert_eq!(app.editor.selected_views()[0].kind, Kind::Paragraph);
            assert_eq!(app.editor.selected_views()[0].text, "First\nSecond");
            app.editor.undo();
            assert_eq!(app.editor.selected_views()[0].text, "First");
            app.editor.undo();
            assert_eq!(app.editor.selected_views()[0].id, id);
            assert_eq!(app.editor.selected_views()[0].kind, Kind::Label);
            assert_eq!(app.editor.selected_views()[0].text, "First");
        }
    }
    #[test]
    fn single_line_tab_returns_focus_to_canvas_for_following_commands() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Button, 200., 150.);
        let id = *app.editor.selection.iter().next().unwrap();
        app.begin_text_edit(id);
        for _ in 0..3 {
            application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
        }
        application_frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::Text("Tab text".into()),
                press(Key::Tab, Modifiers::NONE),
            ],
        )
        .drop_without_applying_deltas();
        for _ in 0..4 {
            application_frame(&ctx, &mut app, vec![]).drop_without_applying_deltas();
        }
        assert!(app.text_edit.is_none());
        assert_eq!(ctx.memory(|m| m.focused()), app.canvas_focus_id);
        assert_eq!(app.editor.selected_views()[0].text, "Tab text");
    }
}
