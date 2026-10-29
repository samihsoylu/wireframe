// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! AppKit objects remain on the main thread. Callbacks enqueue input and never
//! borrow the editor; document mutations happen during the normal app update.
use super::{Command, WireframeApp, egui};
use objc2::{
    DefinedClass, MainThreadOnly, define_class, msg_send, rc::Retained, runtime::Sel, sel,
};
use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem, NSView, NSWindow};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSString};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{cell::RefCell, collections::HashMap};

struct Ivars {
    ctx: egui::Context,
    enabled: RefCell<HashMap<u64, bool>>,
}
define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    struct WireframeMenuTarget;
    unsafe impl NSObjectProtocol for WireframeMenuTarget {}
    impl WireframeMenuTarget {
        #[unsafe(method(validateMenuItem:))]
        fn validate(&self, sender: &NSMenuItem) -> bool {
            self.enabled_for(sender)
        }
        #[unsafe(method(performWireframeCommand:))]
        fn command(&self, sender: &NSMenuItem) {
            if let Some(command) = Command::from_id(sender.tag() as u64) {
                if !self.enabled_for(sender) { return; }
                let input = command.native_input();
                let release = match &input {
                    egui_winit::native_input::NativeInput::Event(egui::Event::Key { key, physical_key, modifiers, .. }) => Some(egui::Event::Key {
                        key: *key, physical_key: *physical_key, modifiers: *modifiers, pressed: false, repeat: false,
                    }),
                    _ => None,
                };
                egui_winit::native_input::send(&self.ivars().ctx, egui::ViewportId::ROOT, input);
                if let Some(release) = release {
                    egui_winit::native_input::send(&self.ivars().ctx, egui::ViewportId::ROOT, egui_winit::native_input::NativeInput::Event(release));
                }
            }
        }
    }
);
impl WireframeMenuTarget {
    fn enabled_for(&self, sender: &NSMenuItem) -> bool {
        let Some(command) = Command::from_id(sender.tag() as u64) else {
            return false;
        };
        self.ivars()
            .enabled
            .borrow()
            .get(&(command as u64))
            .copied()
            .unwrap_or(false)
    }
    fn new(mtm: MainThreadMarker, ctx: egui::Context) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars {
            ctx,
            enabled: RefCell::new(HashMap::new()),
        });
        // SAFETY: NSObject initialization of this allocated main-thread instance.
        unsafe { msg_send![super(this), init] }
    }
}

pub(super) struct NativeMenus {
    // Menu items do not retain their target. Drop detaches them before this dies.
    _target: Retained<WireframeMenuTarget>,
    root: Retained<NSMenu>,
    parents: Vec<Retained<NSMenuItem>>,
    items: Vec<(Command, Retained<NSMenuItem>)>,
    app_menu: Retained<NSMenu>,
    app_autoenables: bool,
    replaced: Vec<(Retained<NSMenuItem>, Option<Sel>, String)>,
    window: Retained<NSWindow>,
}
impl NativeMenus {
    pub(super) fn window_is_main(&self) -> bool {
        self.window.isMainWindow()
    }

    pub(super) fn new(cc: &eframe::CreationContext<'_>) -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        if !egui_winit::native_input::is_available(&cc.egui_ctx, egui::ViewportId::ROOT) {
            return None;
        }
        let handle = cc.window_handle().ok()?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return None;
        };
        // SAFETY: CreationContext lends its live NSView on the main thread. We
        // retain its owning window, not an unrelated key window or save panel.
        let view = unsafe { &*handle.ns_view.as_ptr().cast::<NSView>() };
        let window = view.window()?;
        let application = NSApplication::sharedApplication(mtm);
        let root = application.mainMenu()?;
        let app_menu = root.itemAtIndex(0)?.submenu()?;
        let target = WireframeMenuTarget::new(mtm, cc.egui_ctx.clone());
        let mut menus = Self {
            _target: target,
            root,
            parents: vec![],
            items: vec![],
            app_autoenables: app_menu.autoenablesItems(),
            app_menu,
            replaced: vec![],
            window,
        };
        // Preserve the platform application menu and its Hide/Services actions.
        // Quit must go through the editor's pending-save/export state machine.
        for item in menus.app_menu.itemArray().iter() {
            let command = if item.action() == Some(sel!(terminate:)) {
                Some(Command::Quit)
            } else if item.action() == Some(sel!(orderFrontStandardAboutPanel:)) {
                Some(Command::About)
            } else {
                None
            };
            if let Some(command) = command {
                menus
                    .replaced
                    .push((item.clone(), item.action(), item.title().to_string()));
                menus.bind(command, &item);
                if command == Command::About {
                    item.setTitle(&NSString::from_str("About Wireframe"));
                } else {
                    item.setTitle(&NSString::from_str(command.label()));
                }
                menus.items.push((command, item));
            }
        }
        menus.app_menu.setAutoenablesItems(true);
        use Command::*;
        menus.menu(
            mtm,
            "File",
            &[
                &[NewPage, New, Open],
                &[Close, Save, SaveAs, RenameProject, AutomaticSave, Recover],
                &[ExportPdf, ExportImages, ExportSvg],
                &[ExtractText],
                &[Example, Sketch, Font],
            ],
        );
        let edit_menu = menus.menu(
            mtm,
            "Edit",
            &[
                &[Undo, Redo],
                &[Cut, Copy, Paste, PasteControls, Delete],
                &[Duplicate, SelectAll],
                &[Group, Ungroup, AutoSize, TreatAsMarkup],
                &[SendToBack, BringToFront, SendBackward, BringForward],
            ],
        );
        let alignment = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str("Align"));
        alignment.setAutoenablesItems(true);
        let parent = NSMenuItem::new(mtm);
        parent.setTitle(&NSString::from_str("Align"));
        parent.setSubmenu(Some(&alignment));
        for &command in Command::ALIGNMENT {
            menus.add(mtm, &alignment, command);
        }
        edit_menu.addItem(&parent);
        let view_menu = menus.menu(
            mtm,
            "View",
            &[
                &[Pages, Library, Comments, Inspector, ShowMarkup],
                &[Grid, Snap],
            ],
        );
        let zoom = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str("Zoom"));
        zoom.setAutoenablesItems(true);
        let parent = NSMenuItem::new(mtm);
        parent.setTitle(&NSString::from_str("Zoom"));
        parent.setSubmenu(Some(&zoom));
        for command in [ZoomIn, ZoomOut, Actual, Fit, AllActual, AllFit] {
            menus.add(mtm, &zoom, command);
        }
        view_menu.addItem(&parent);
        menus.add(mtm, &view_menu, Present);
        let window_menu = menus.menu(mtm, "Window", &[]);
        window_menu.setAutoenablesItems(true);
        for (title, action, key, flags) in [
            (
                "Minimize",
                sel!(performMiniaturize:),
                "m",
                NSEventModifierFlags::Command,
            ),
            (
                "Zoom",
                sel!(performZoom:),
                "",
                NSEventModifierFlags::empty(),
            ),
            (
                "Enter Full Screen",
                sel!(toggleFullScreen:),
                "f",
                NSEventModifierFlags::Command | NSEventModifierFlags::Control,
            ),
        ] {
            // SAFETY: Standard AppKit actions are resolved through the responder chain.
            let item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    NSMenuItem::alloc(mtm),
                    &NSString::from_str(title),
                    Some(action),
                    &NSString::from_str(key),
                )
            };
            item.setKeyEquivalentModifierMask(flags);
            window_menu.addItem(&item);
        }
        application.setWindowsMenu(Some(&window_menu));
        menus.menu(mtm, "Help", &[&[Help, About]]);
        Some(menus)
    }
    fn bind(&self, command: Command, item: &NSMenuItem) {
        // SAFETY: This target implements the selector and is retained until all
        // bound items are detached in Drop.
        unsafe {
            item.setTarget(Some(&self._target));
            item.setAction(Some(sel!(performWireframeCommand:)));
        }
        item.setTag(command as isize);
    }
    fn add(&mut self, mtm: MainThreadMarker, menu: &NSMenu, command: Command) {
        let (key, shift) = shortcut(command);
        // SAFETY: Action is installed with a retained target immediately below.
        let item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(command.label()),
                None,
                &NSString::from_str(key),
            )
        };
        item.setKeyEquivalentModifierMask(if key.is_empty() {
            NSEventModifierFlags::empty()
        } else if matches!(
            command,
            Command::AutoSize
                | Command::AlignLeft
                | Command::AlignCenter
                | Command::AlignRight
                | Command::AlignTop
                | Command::AlignMiddle
                | Command::AlignBottom
        ) {
            NSEventModifierFlags::Command | NSEventModifierFlags::Option
        } else if shift {
            NSEventModifierFlags::Command | NSEventModifierFlags::Shift
        } else {
            NSEventModifierFlags::Command
        });
        self.bind(command, &item);
        menu.addItem(&item);
        self.items.push((command, item));
    }
    fn menu(
        &mut self,
        mtm: MainThreadMarker,
        title: &str,
        groups: &[&[Command]],
    ) -> Retained<NSMenu> {
        let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(title));
        menu.setAutoenablesItems(true);
        let parent = NSMenuItem::new(mtm);
        parent.setTitle(&NSString::from_str(title));
        parent.setSubmenu(Some(&menu));
        self.root.addItem(&parent);
        self.parents.push(parent);
        for (index, group) in groups.iter().enumerate() {
            if index > 0 {
                menu.addItem(&NSMenuItem::separatorItem(mtm));
            }
            for &command in *group {
                self.add(mtm, &menu, command);
            }
        }
        menu
    }
    pub(super) fn dialog_scope(&self) -> DialogMenus {
        let mut items = Vec::new();
        for (command, item) in &self.items {
            if let Some(action) = text_action(*command) {
                // SAFETY: Native modal panels require standard responder-chain
                // targets. AppKit disables app-specific targets during runModal.
                unsafe {
                    item.setTarget(None);
                    item.setAction(Some(action));
                }
                items.push(item.clone());
            }
        }
        DialogMenus {
            items,
            target: self._target.clone(),
        }
    }
    pub(super) fn update(&self, app: &WireframeApp, ctx: &egui::Context) {
        let text = wireframe::text_entry::is_focused(ctx) || ctx.text_edit_focused();
        for (command, item) in &self.items {
            let enabled = app.command_enabled(ctx, *command);
            self._target
                .ivars()
                .enabled
                .borrow_mut()
                .insert(*command as u64, enabled);
            item.setEnabled(enabled);
            item.setState(isize::from(app.command_checked(*command).unwrap_or(false)));
            if *command == Command::TreatAsMarkup {
                item.setTitle(&NSString::from_str(app.command_label(*command)));
            }
            if matches!(command, Command::Undo | Command::Redo) {
                let name = if text {
                    None
                } else if *command == Command::Undo {
                    app.editor.undo_name()
                } else {
                    app.editor.redo_name()
                };
                item.setTitle(&NSString::from_str(&name.map_or_else(
                    || command.label().to_owned(),
                    |name| format!("{} {name}", command.label()),
                )));
            }
        }
        self.window.setDocumentEdited(app.editor.dirty());
        self.window.setRepresentedFilename(&NSString::from_str(
            &app.file
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
        ));
    }
}
impl Drop for NativeMenus {
    fn drop(&mut self) {
        for (_, item) in &self.items {
            // SAFETY: Detach unretained targets before dropping the owning object.
            unsafe {
                item.setTarget(None);
                item.setAction(None);
            }
        }
        for (item, action, title) in &self.replaced {
            unsafe {
                item.setAction(*action);
            }
            item.setTitle(&NSString::from_str(title));
        }
        self.app_menu.setAutoenablesItems(self.app_autoenables);
        for parent in &self.parents {
            self.root.removeItem(parent);
        }
        if let Some(mtm) = MainThreadMarker::new() {
            NSApplication::sharedApplication(mtm).setWindowsMenu(None);
        }
    }
}
fn shortcut(command: Command) -> (&'static str, bool) {
    use Command::*;
    match command {
        New => ("n", false),
        NewPage => ("n", true),
        Open => ("o", false),
        Save => ("s", false),
        SaveAs => ("s", true),
        Close => ("w", false),
        Undo => ("z", false),
        Redo => ("z", true),
        Cut => ("x", false),
        Copy => ("c", false),
        Paste => ("v", false),
        SelectAll => ("a", false),
        Duplicate => ("d", false),
        Group => ("g", false),
        Ungroup => ("g", true),
        SendToBack => ("\u{f701}", true),
        BringToFront => ("\u{f700}", true),
        SendBackward => ("\u{f701}", false),
        BringForward => ("\u{f700}", false),
        AutoSize => ("0", false),
        TreatAsMarkup => ("k", true),
        ShowMarkup => ("k", false),
        AlignLeft => ("1", false),
        AlignCenter => ("2", false),
        AlignRight => ("3", false),
        AlignTop => ("4", false),
        AlignMiddle => ("5", false),
        AlignBottom => ("6", false),
        Pages => ("j", false),
        Library => ("l", false),
        Inspector => (";", false),
        ZoomIn => ("+", false),
        ZoomOut => ("-", false),
        Actual => ("1", false),
        Fit => ("0", false),
        AllActual => ("1", true),
        AllFit => ("0", true),
        Present => ("f", true),
        _ => ("", false),
    }
}

fn text_action(command: Command) -> Option<Sel> {
    use Command::*;
    Some(match command {
        Undo => sel!(undo:),
        Redo => sel!(redo:),
        Cut => sel!(cut:),
        Copy => sel!(copy:),
        Paste => sel!(paste:),
        SelectAll => sel!(selectAll:),
        Delete => sel!(deleteBackward:),
        _ => return None,
    })
}

pub(super) struct DialogMenus {
    items: Vec<Retained<NSMenuItem>>,
    target: Retained<WireframeMenuTarget>,
}
impl Drop for DialogMenus {
    fn drop(&mut self) {
        for item in &self.items {
            // SAFETY: The target remains retained by NativeMenus after this guard.
            unsafe {
                item.setTarget(Some(&self.target));
                item.setAction(Some(sel!(performWireframeCommand:)));
            }
        }
    }
}
