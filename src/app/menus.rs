// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Shared command semantics for native and in-window menus.
#[cfg(target_os = "macos")]
mod macos;
#[cfg(test)]
mod navigation_tests;
#[cfg(test)]
mod tests;
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u64)]
pub(super) enum Command {
    New = 1,
    Open,
    Save,
    SaveAs,
    Close,
    NewPage,
    AutomaticSave,
    Recover,
    ExportPdf,
    ExportImages,
    ExportSvg,
    Example,
    Sketch,
    Font,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    PasteControls,
    Duplicate,
    SelectAll,
    Delete,
    Group,
    Ungroup,
    Pages,
    Library,
    Inspector,
    Grid,
    Snap,
    ZoomIn,
    ZoomOut,
    Actual,
    Fit,
    AllActual,
    AllFit,
    Present,
    Help,
    About,
    Quit,
    RenameProject,
    Comments,
    SendToBack,
    BringToFront,
    SendBackward,
    BringForward,
    AlignLeft,
    AlignCenter,
    AlignRight,
    AlignTop,
    AlignMiddle,
    AlignBottom,
    DistributeHorizontal,
    DistributeVertical,
    SpaceHorizontal,
    SpaceVertical,
    AutoSize,
    TreatAsMarkup,
    ShowMarkup,
    ExtractText,
}
impl Command {
    pub(super) const ALL: &[Self] = &[
        Self::New,
        Self::Open,
        Self::Save,
        Self::SaveAs,
        Self::Close,
        Self::NewPage,
        Self::AutomaticSave,
        Self::Recover,
        Self::ExportPdf,
        Self::ExportImages,
        Self::ExportSvg,
        Self::Example,
        Self::Sketch,
        Self::Font,
        Self::Undo,
        Self::Redo,
        Self::Cut,
        Self::Copy,
        Self::Paste,
        Self::PasteControls,
        Self::Duplicate,
        Self::SelectAll,
        Self::Delete,
        Self::Group,
        Self::Ungroup,
        Self::Pages,
        Self::Library,
        Self::Inspector,
        Self::Comments,
        Self::Grid,
        Self::Snap,
        Self::ZoomIn,
        Self::ZoomOut,
        Self::Actual,
        Self::Fit,
        Self::AllActual,
        Self::AllFit,
        Self::Present,
        Self::Help,
        Self::About,
        Self::Quit,
        Self::RenameProject,
        Self::SendToBack,
        Self::BringToFront,
        Self::SendBackward,
        Self::BringForward,
        Self::AlignLeft,
        Self::AlignCenter,
        Self::AlignRight,
        Self::AlignTop,
        Self::AlignMiddle,
        Self::AlignBottom,
        Self::DistributeHorizontal,
        Self::DistributeVertical,
        Self::SpaceHorizontal,
        Self::SpaceVertical,
        Self::AutoSize,
        Self::TreatAsMarkup,
        Self::ShowMarkup,
        Self::ExtractText,
    ];
    pub(super) fn from_id(id: u64) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|command| *command as u64 == id)
    }
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::New => "New project",
            Self::Open => "Open…",
            Self::Save => "Save",
            Self::SaveAs => "Save as…",
            Self::Close => "Close project",
            Self::NewPage => "New page",
            Self::AutomaticSave => "Automatically save named projects",
            Self::Recover => "Recover work…",
            Self::ExportPdf => "Export PDF…",
            Self::ExportImages => "Export images…",
            Self::ExportSvg => "Export page as SVG…",
            Self::Example => "Open example project",
            Self::ExtractText => "Extract text from screenshot…",
            Self::Sketch => "Sketch style",
            Self::Font => "Font and colors…",
            Self::Undo => "Undo",
            Self::Redo => "Redo",
            Self::Cut => "Cut",
            Self::Copy => "Copy",
            Self::Paste => "Paste",
            Self::PasteControls => "Paste copied controls",
            Self::Duplicate => "Duplicate",
            Self::SelectAll => "Select all",
            Self::Delete => "Delete selection",
            Self::Group => "Group",
            Self::Ungroup => "Ungroup",
            Self::Pages => "Pages",
            Self::Library => "Library",
            Self::Inspector => "Inspector",
            Self::Comments => "Comments",
            Self::Grid => "Grid",
            Self::Snap => "Snap to 10",
            Self::ZoomIn => "Zoom in",
            Self::ZoomOut => "Zoom out",
            Self::Actual => "Actual size",
            Self::Fit => "Fit page",
            Self::AllActual => "All pages at actual size",
            Self::AllFit => "Fit all pages",
            Self::Present => "Present",
            Self::Help => "Working with Wireframe",
            Self::About => "License and notices",
            Self::Quit => "Quit Wireframe",
            Self::RenameProject => "Rename project…",
            Self::SendToBack => "Send to back",
            Self::BringToFront => "Bring to front",
            Self::SendBackward => "Send backward",
            Self::BringForward => "Bring forward",
            Self::AlignLeft => "Align left",
            Self::AlignCenter => "Align center",
            Self::AlignRight => "Align right",
            Self::AlignTop => "Align top",
            Self::AlignMiddle => "Align middle",
            Self::AlignBottom => "Align bottom",
            Self::DistributeHorizontal => "Distribute centers horizontally",
            Self::DistributeVertical => "Distribute centers vertically",
            Self::SpaceHorizontal => "Space out horizontally",
            Self::SpaceVertical => "Space out vertically",
            Self::AutoSize => "Auto-size",
            Self::TreatAsMarkup => "Treat as Markup",
            Self::ShowMarkup => "Markup",
        }
    }
    pub(super) fn layer_action(self) -> Option<wireframe::workspace::LayerAction> {
        use wireframe::workspace::LayerAction;
        Some(match self {
            Self::SendToBack => LayerAction::Back,
            Self::BringToFront => LayerAction::Front,
            Self::SendBackward => LayerAction::Backward,
            Self::BringForward => LayerAction::Forward,
            _ => return None,
        })
    }
    pub(super) const ALIGNMENT: &[Self] = &[
        Self::AlignLeft,
        Self::AlignCenter,
        Self::AlignRight,
        Self::AlignTop,
        Self::AlignMiddle,
        Self::AlignBottom,
        Self::DistributeHorizontal,
        Self::DistributeVertical,
        Self::SpaceHorizontal,
        Self::SpaceVertical,
    ];
    pub(super) fn alignment_action(self) -> Option<wireframe::workspace::AlignmentAction> {
        use wireframe::workspace::AlignmentAction;
        Some(match self {
            Self::AlignLeft => AlignmentAction::Left,
            Self::AlignCenter => AlignmentAction::Center,
            Self::AlignRight => AlignmentAction::Right,
            Self::AlignTop => AlignmentAction::Top,
            Self::AlignMiddle => AlignmentAction::Middle,
            Self::AlignBottom => AlignmentAction::Bottom,
            Self::DistributeHorizontal => AlignmentAction::DistributeHorizontal,
            Self::DistributeVertical => AlignmentAction::DistributeVertical,
            Self::SpaceHorizontal => AlignmentAction::SpaceHorizontal,
            Self::SpaceVertical => AlignmentAction::SpaceVertical,
            _ => return None,
        })
    }
    fn editing(self) -> bool {
        matches!(
            self,
            Self::Undo
                | Self::Redo
                | Self::Cut
                | Self::Copy
                | Self::Paste
                | Self::SelectAll
                | Self::Delete
        )
    }
    /// Text editing stays in the regular input stream so the focused field owns
    /// local history, selection and clipboard operations, including in dialogs.
    fn editing_event(self) -> Option<egui::Event> {
        Some(match self {
            Self::Copy => egui::Event::Copy,
            Self::Cut => egui::Event::Cut,
            Self::Undo | Self::Redo | Self::SelectAll | Self::Delete => egui::Event::Key {
                key: match self {
                    Self::SelectAll => Key::A,
                    Self::Delete => Key::Backspace,
                    _ => Key::Z,
                },
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: match self {
                    Self::Redo => Modifiers::COMMAND | Modifiers::SHIFT,
                    Self::Delete => Modifiers::NONE,
                    _ => Modifiers::COMMAND,
                },
            },
            _ => return None,
        })
    }
    #[cfg(target_os = "macos")]
    fn native_input(self) -> egui_winit::native_input::NativeInput {
        use egui_winit::native_input::NativeInput;
        if self == Self::Paste {
            NativeInput::Paste
        } else {
            NativeInput::Event(
                self.editing_event()
                    .unwrap_or(egui::Event::AppCommand(self as u64)),
            )
        }
    }
}

#[derive(Default)]
pub(super) struct Menus {
    #[cfg(target_os = "macos")]
    native: Option<macos::NativeMenus>,
    title: String,
    in_window: Vec<(egui::Id, Rect, Option<Rect>)>,
    in_window_items: Vec<egui::Id>,
    pending_text_command: Option<PendingTextCommand>,
    text_owner: Option<egui::Id>,
    navigator_owner: Option<NavigatorMenuOwner>,
    pub(super) drawing_popup: bool,
}
struct NavigatorMenuOwner {
    id: egui::Id,
    session: String,
    page: Id,
    surface: Option<source_navigator::ResourceRow>,
}
struct PendingTextCommand {
    owner: egui::Id,
    command: Command,
    canvas: bool,
    session: String,
}
impl Menus {
    pub(super) fn cancel_text_command(&mut self) {
        self.pending_text_command = None;
    }

    pub(super) fn begin_in_window(&mut self, ctx: &egui::Context) {
        if !self.in_window_open(ctx) {
            self.text_owner = (wireframe::text_entry::is_focused(ctx) || ctx.text_edit_focused())
                .then(|| ctx.memory(|m| m.focused()))
                .flatten();
        }
        self.in_window.clear();
        self.in_window_items.clear();
    }

    fn text_owner(&self, ctx: &egui::Context) -> Option<egui::Id> {
        (self.drawing_popup || self.in_window_open(ctx))
            .then_some(self.text_owner)
            .flatten()
    }

    pub(super) fn record_in_window(&mut self, button: &egui::Response, popup: Option<Rect>) {
        self.in_window.push((button.id, button.rect, popup));
    }

    pub(super) fn in_window_open(&self, ctx: &egui::Context) -> bool {
        self.in_window
            .iter()
            .any(|(id, _, _)| egui::Popup::is_id_open(ctx, id.with("popup")))
    }

    pub(super) fn owns_in_window_event(&self, ctx: &egui::Context, event: &egui::Event) -> bool {
        match event {
            egui::Event::PointerButton { pos, .. } => {
                self.in_window.iter().any(|(id, button, popup)| {
                    button.contains(*pos)
                        || (egui::Popup::is_id_open(ctx, id.with("popup"))
                            && popup.is_some_and(|rect| rect.contains(*pos)))
                })
            }
            egui::Event::AccessKitActionRequest(request) => {
                self.in_window
                    .iter()
                    .any(|(id, _, _)| request.target_node == id.accesskit_id())
                    || (self.in_window_open(ctx)
                        && self
                            .in_window_items
                            .iter()
                            .any(|id| request.target_node == id.accesskit_id()))
            }
            _ => false,
        }
    }

    pub(super) fn window_is_main(&self) -> Option<bool> {
        #[cfg(target_os = "macos")]
        {
            self.native.as_ref().map(macos::NativeMenus::window_is_main)
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    pub(super) fn with_dialog<T>(&self, action: impl FnOnce() -> T) -> T {
        #[cfg(target_os = "macos")]
        let _scope = self.native.as_ref().map(macos::NativeMenus::dialog_scope);
        action()
    }

    pub(super) fn native(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.native.is_some()
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }
}
impl WireframeApp {
    pub(super) fn begin_in_window_menus(&mut self, ctx: &egui::Context) {
        if !self.menus.in_window_open(ctx) {
            // The popup takes widget focus; retain its original resource owner
            // without letting that owner survive a project or workspace change.
            self.menus.navigator_owner = (self.show_navigator
                && (self.source_navigation_focused(ctx)
                    || (!self.editor.in_source_workspace() && self.navigator.focused(ctx))))
            .then(|| ctx.memory(|m| m.focused()))
            .flatten()
            .map(|id| NavigatorMenuOwner {
                id,
                session: self.editor.clipboard_session().into(),
                page: self.editor.page,
                surface: self.active_resource_row(),
            });
        }
        self.menus.begin_in_window(ctx);
    }

    pub(super) fn command_navigation_focus(&self, ctx: &egui::Context) -> Option<egui::Id> {
        if self.menus.drawing_popup || self.menus.in_window_open(ctx) {
            let owner = self.menus.navigator_owner.as_ref()?;
            return (self.show_navigator
                && owner.session == self.editor.clipboard_session()
                && owner.page == self.editor.page
                && owner.surface == self.active_resource_row())
            .then_some(owner.id);
        }
        ctx.memory(|m| m.focused())
    }

    pub(super) fn deliver_menu_text_command(
        &mut self,
        ctx: &egui::Context,
        raw: &mut egui::RawInput,
    ) {
        let Some(pending) = self.menus.pending_text_command.take() else {
            return;
        };
        if self.menu_blocked()
            || pending.session != self.editor.clipboard_session()
            || (pending.canvas && self.canvas_text_menu_owner() != Some(pending.owner))
        {
            return;
        }
        // Canvas text is processed before the toolbar, and other fields may
        // still be disabled by the closing popup. Deliver next frame, before
        // newer keyboard input, to the original text owner.
        wireframe::text_entry::restore_focus(ctx, pending.owner);
        if let Some(event) = pending.command.editing_event() {
            raw.events.insert(0, event);
        } else if pending.command == Command::Paste {
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
        }
    }

    pub(super) fn init_menus(&mut self, cc: &eframe::CreationContext<'_>) {
        #[cfg(target_os = "macos")]
        {
            self.menus.native = macos::NativeMenus::new(cc);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = cc;
    }
    fn menu_blocked(&self) -> bool {
        self.canvas_input_blocked()
            || (self.present && self.presentation_view.settings_open)
            || self.font_import.is_some()
            || self.image_crop.is_some()
            || self.image_import.is_some()
            || self.library_drag.is_some()
            || self.library_press.is_some()
            || self.drag.is_some()
    }
    pub(super) fn command_enabled(&self, ctx: &egui::Context, command: Command) -> bool {
        if self.startup_open.is_some() {
            return matches!(command, Command::Quit | Command::Close);
        }
        let text = wireframe::text_entry::is_focused(ctx)
            || ctx.text_edit_focused()
            || (!self.menu_blocked() && self.menus.text_owner(ctx).is_some())
            || (command.editing() && !self.menu_blocked() && self.text_edit.is_some());
        if command.editing() && text {
            return !self.save_busy()
                && self.library_drag.is_none()
                && self.library_press.is_none();
        }
        if self.menu_blocked()
            || (self.text_edit.is_some() && !matches!(command, Command::Save | Command::SaveAs))
        {
            return false;
        }
        if self.present {
            return matches!(command, Command::Present | Command::Close | Command::Quit);
        }
        if let Some(enabled) = self.source_command_enabled(ctx, command) {
            return enabled;
        }
        if let Some(enabled) = self.page_command_enabled(ctx, command) {
            return enabled;
        }
        use Command::*;
        match command {
            Present | Comments => !self.editor.in_source_workspace(),
            Undo => self.editor.undo_name().is_some(),
            Redo => self.editor.redo_name().is_some(),
            Cut | Delete | Group | Ungroup => {
                self.editor.selected_views().iter().any(|c| !c.locked)
            }
            AlignLeft | AlignCenter | AlignRight | AlignTop | AlignMiddle | AlignBottom
            | DistributeHorizontal | DistributeVertical | SpaceHorizontal | SpaceVertical => {
                !text && self.editor.can_align_selection()
            }
            AutoSize => !text && self.editor.can_auto_size_selection(false),
            TreatAsMarkup => !text && self.editor.selection_markup_target().is_some(),
            ShowMarkup => !text,
            Copy | Duplicate => !self.editor.selection.is_empty(),
            PasteControls => !self.clipboard.is_empty(),
            AutomaticSave | Recover => self.persistence.is_some(),
            ExportPdf => self.pdf_job.is_none(),
            ExportImages => self.image_export.job.is_none(),
            ExtractText => self.can_extract_screenshot(),
            ZoomIn => wireframe::viewport::next_zoom(self.zoom, true).is_some(),
            ZoomOut => wireframe::viewport::next_zoom(self.zoom, false).is_some(),
            SendToBack | BringToFront | SendBackward | BringForward => {
                !text && self.editor.can_arrange(command.layer_action().unwrap())
            }
            _ => true,
        }
    }
    pub(super) fn command_checked(&self, command: Command) -> Option<bool> {
        use Command::*;
        match command {
            AutomaticSave => self.automatic_save_setting(),
            Sketch => Some(self.editor.document.settings.sketch),
            Pages => Some(self.show_navigator),
            Library => Some(self.show_library),
            Inspector => Some(self.show_inspector && !self.review.open),
            Comments => Some(self.review.open),
            Grid => Some(self.grid),
            ShowMarkup => Some(self.editor.markup_visible()),
            Snap => Some(self.snap),
            Present => Some(self.present),
            _ => None,
        }
    }
    pub(super) fn run_command(&mut self, ctx: &egui::Context, command: Command) {
        // Cached native enabled states are only presentation. Recheck after every
        // command, since earlier commands in this frame can establish a modal.
        if !self.command_enabled(ctx, command) {
            return;
        }
        if command.editing() {
            let canvas_owner = self.canvas_text_menu_owner();
            // Menu enablement is cheap; validate the full draft only when an
            // action is chosen. A stale draft must never fall through to the
            // drawing's history or selection.
            if self.text_edit.is_some() && canvas_owner.is_none() {
                return;
            }
            if let Some(owner) = canvas_owner.or_else(|| self.menus.text_owner(ctx)) {
                self.menus.pending_text_command = Some(PendingTextCommand {
                    owner,
                    command,
                    canvas: canvas_owner.is_some(),
                    session: self.editor.clipboard_session().into(),
                });
                ctx.request_repaint();
                return;
            }
        }
        if command.editing() && (wireframe::text_entry::is_focused(ctx) || ctx.text_edit_focused())
        {
            if let Some(event) = command.editing_event() {
                if !self.quick_add_shortcut(ctx, &event) {
                    ctx.input_mut(|i| i.events.push(event));
                }
            } else if command == Command::Paste {
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
            }
            return;
        }
        if !self.finish_page_notes() {
            return;
        }
        if self.run_source_command(ctx, command) {
            return;
        }
        if self.run_page_command(ctx, command) {
            return;
        }
        use Command::*;
        match command {
            RenameProject => self.begin_project_rename(),
            New => self.request(Pending::New, ctx),
            Open => self.open(ctx),
            Save => {
                let owner = self.command_navigation_focus(ctx).filter(|id| {
                    if self.editor.in_source_workspace() {
                        self.source_navigation_focused(ctx)
                    } else {
                        self.navigator.focus_ids.contains(id)
                    }
                });
                self.save(false);
                if self.save_busy()
                    && let Some(id) = owner
                {
                    // Capture before the closing popup and saving modal disable rows.
                    self.save_navigator_focus = Some(id);
                    ctx.memory_mut(|m| m.request_focus(id));
                }
            }
            SaveAs => {
                // Choosing a destination accepts the canvas draft even when the
                // subsequent native chooser is cancelled. Ordinary Save leaves
                // that draft provisional.
                self.commit_text_session(ctx);
                self.save(true);
            }
            Close | Quit => self.request(Pending::Quit, ctx),
            NewPage => {
                self.editor.add_page();
            }
            AutomaticSave => {
                if let Some(enabled) = self.automatic_save_setting() {
                    self.set_save_mode(!enabled);
                }
            }
            Recover => self.show_recoveries(),
            ExportPdf => {
                self.end_pointer_gestures();
                self.pdf_options = Some(wireframe::pdf::Options {
                    markup_visible: self.editor.markup_visible(),
                    ..Default::default()
                });
            }
            ExportImages => self.open_image_export(),
            ExportSvg => self.export(false),
            ExtractText => self.choose_screenshot_text(ctx),
            Example => self.request(Pending::Example, ctx),
            Sketch => self.editor.edit("Change style", |e| {
                e.document.settings.sketch = !e.document.settings.sketch
            }),
            Font => self.font_import = Some(fonts::FontImport::default()),
            Undo => self.editor.undo(),
            Redo => self.editor.redo(),
            Copy => {
                self.copy(ctx);
            }
            Cut => {
                self.cut_controls(ctx);
            }
            Paste => ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste),
            PasteControls => self.paste_clipboard(&self.clipboard.clone()),
            Duplicate => self.editor.duplicate(),
            SelectAll => self.editor.select_all(),
            Delete => self.editor.delete(),
            Group => self.editor.group(),
            Ungroup => self.editor.ungroup(),
            SendToBack | BringToFront | SendBackward | BringForward => {
                self.editor.arrange(command.layer_action().unwrap());
            }
            AlignLeft | AlignCenter | AlignRight | AlignTop | AlignMiddle | AlignBottom
            | DistributeHorizontal | DistributeVertical | SpaceHorizontal | SpaceVertical => self
                .editor
                .align_selection(command.alignment_action().unwrap()),
            AutoSize => {
                self.editor.commit();
                if let Err(error) = self.editor.auto_size_selection() {
                    self.error = Some(error);
                }
            }
            Pages => self.show_navigator = !self.show_navigator,
            Library => self.show_library = !self.show_library,
            Inspector => self.toggle_review_inspector(),
            ShowMarkup => {
                self.editor
                    .set_markup_visible(!self.editor.markup_visible());
                self.remember_viewports(true);
            }
            TreatAsMarkup => {
                if let Err(error) = self.editor.toggle_selection_markup() {
                    self.error = Some(error);
                }
            }
            Comments => self.review.open = !self.review.open,
            Grid => self.grid = !self.grid,
            Snap => self.snap = !self.snap,
            ZoomIn => self.zoom_step(true),
            ZoomOut => self.zoom_step(false),
            Actual => self.actual_size(),
            Fit => self.fit(),
            AllActual => self.zoom_all(false),
            AllFit => self.zoom_all(true),
            Present => {
                if self.present {
                    self.present = false;
                } else {
                    self.start_presentation();
                }
            }
            Help => self.help = true,
            About => self.about = true,
        }
    }
    pub(super) fn update_menus(&mut self, ctx: &egui::Context) {
        let title = format!("{} — Wireframe", self.editor.document.title);
        if self.menus.title != title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.menus.title = title;
        }
        #[cfg(target_os = "macos")]
        if let Some(native) = &self.menus.native {
            native.update(self, ctx);
        }
    }
    pub(super) fn command_label(&self, command: Command) -> &'static str {
        if command == Command::TreatAsMarkup && self.editor.selection_markup_target() == Some(false)
        {
            "Do not Treat as Markup"
        } else {
            command.label()
        }
    }
    pub(super) fn menu_item(&mut self, ui: &mut egui::Ui, command: Command) {
        let checked = self.command_checked(command);
        let label = match command {
            Command::New => "New project     ⌘/Ctrl N",
            Command::Open => "Open…               ⌘/Ctrl O",
            Command::Save => "Save                   ⌘/Ctrl S",
            _ => self.command_label(command),
        };
        let button = egui::Button::new(label);
        let button = if let Some(value) = checked {
            button.selected(value)
        } else {
            button
        };
        let response = ui.add_enabled(self.command_enabled(ui.ctx(), command), button);
        self.menus.in_window_items.push(response.id);
        if response.clicked() {
            self.run_command(ui.ctx(), command);
            ui.close();
        }
    }
}
