// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

mod activation;
mod arrangement;
mod arrows;
#[cfg(test)]
mod canvas_action_tests;
mod canvas_click;
mod canvas_scroll;
mod chrome;
mod clipboard;
#[cfg(test)]
mod close_tests;
mod closing;
mod components;
#[cfg(test)]
mod creation_tests;
mod crop;
#[cfg(test)]
mod export_tests;
mod fonts;
mod geometry;
mod icon_dialog;
mod icon_picker;
mod image_export;
mod images;
mod insertion;
mod layering;
mod library;
mod library_drag;
mod library_import;
mod menus;
mod more_controls;
#[cfg(target_os = "macos")]
mod native_termination;
mod navigator;
#[cfg(test)]
mod navigator_tests;
#[cfg(all(test, target_os = "macos"))]
mod offscreen_tests;
mod page_clipboard;
mod page_inspector;
mod pages;
mod palette;
mod pdf_export;
mod persistence;
mod presentation;
mod previews;
mod properties;
mod property_sections;
mod quick_draw;
mod recovery;
mod review;
mod selection;
mod selection_properties;
#[cfg(test)]
mod shortcut_tests;
mod shortcuts;
mod source_navigator;
mod startup;
mod text_session;
mod toolbar;
mod trash;
#[cfg(target_os = "macos")]
mod update_resume;
#[cfg(target_os = "macos")]
mod updates;
mod versions;
mod viewports;
mod web_image;

use eframe::egui::{
    self, Align2, Color32, FontId, Key, Modifiers, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2,
};
use std::{
    collections::HashSet,
    path::PathBuf,
    time::{Duration, Instant},
};
use wireframe::{
    document::Project,
    model::*,
    scene::{self, Primitive},
    snapping::{self, Constraint, Motion, Targets},
    storage::{self, ProjectRepository, ProjectStore},
    text_entry::TextInput,
    ui_text,
    workspace::{CatalogCategory, CatalogItem, Clipboard, Editor},
};

const ACCENT: Color32 = Color32::from_rgb(49, 94, 145);
enum Pending {
    New,
    Open(PathBuf),
    Quit,
    Example,
    Restore(String),
}
enum Drag {
    Arrow {
        id: Id,
        handle: wireframe::arrows::Handle,
        origin: Pos2,
        offset: wireframe::document::Point,
    },
    Move {
        start: Pos2,
        original: Vec<(Id, Bounds)>,
        targets: Targets,
        duplicate_targets: Targets,
        motion: Motion,
    },
    Resize {
        start: Pos2,
        original: Vec<(Id, Bounds)>,
        bounds: Bounds,
        handle: (i8, i8),
        targets: Targets,
    },
    Marquee {
        start: Pos2,
        now: Pos2,
        previous: HashSet<Id>,
    },
    Pan {
        start: Pos2,
        pan: Vec2,
    },
}
pub struct WireframeApp {
    menus: menus::Menus,
    compositor: Option<std::sync::Arc<wireframe::render::compositor::Compositor>>,
    editor: Editor,
    installed_font: Option<std::sync::Arc<wireframe::fonts::ProjectFont>>,
    font_import: Option<fonts::FontImport>,
    palette_defaults: palette::PaletteDefaults,
    image_export: image_export::ImageExport,
    versions: versions::Versions,
    page_rename: Option<pages::Rename>,
    page_inspector: page_inspector::PageInspector,
    review: review::Review,
    source_navigation: source_navigator::SourceNavigation,
    library_import: Option<library_import::Review>,
    more_controls: Option<more_controls::Browser>,
    page_paste: Option<page_clipboard::Review>,
    page_paste_request: Option<page_clipboard::Request>,
    trash: Option<trash::Trash>,
    navigator: navigator::Navigator,
    viewports: viewports::Viewports,
    pdf_options: Option<wireframe::pdf::Options>,
    pdf_job: Option<wireframe::pdf::ExportJob>,
    quit_after_pdf: bool,
    store: Box<dyn ProjectRepository>,
    persistence: Option<persistence::Persistence>,
    file: Option<PathBuf>,
    // Unit frames supply the chooser outcome; native dialogs are exercised in the app.
    #[cfg(test)]
    save_dialog_choice: Option<PathBuf>,
    deferred_text_input: Vec<egui::Event>,
    deferred_text_focus: Option<egui::Id>,
    save_navigator_focus: Option<egui::Id>,
    quick: String,
    quick_index: usize,
    quick_focus: bool,
    quick_input_id: Option<egui::Id>,
    show_library: bool,
    show_navigator: bool,
    show_inspector: bool,
    thumbnails: bool,
    categories: Vec<CatalogCategory>,
    zoom: f32,
    pan: Vec2,
    drag: Option<Drag>,
    quick_draw: quick_draw::State,
    drag_scroll: Option<wireframe::viewport::DragScroll>,
    library_drag: Option<CatalogItem>,
    library_press: Option<library_drag::LibraryPress>,
    library_anchor: Vec2,
    alignment_guides: Vec<snapping::Guide>,
    canvas_press: Option<(Pos2, Modifiers)>,
    canvas_clicks: canvas_click::CanvasClicks,
    unlock_press: Option<selection::UnlockPress>,
    grid: bool,
    snap: bool,
    status: String,
    error: Option<String>,
    pending: Option<Pending>,
    startup_open: Option<startup::Opening>,
    image_import: Option<(
        wireframe::workspace::ImageImportTarget,
        wireframe::images::Prepared,
    )>,
    image_crop: Option<crop::CropEdit>,
    web_image: Option<web_image::Review>,
    image_split_fraction: f32,
    quit_allowed: bool,
    window_close_pending: bool,
    #[cfg(target_os = "macos")]
    native_termination: Option<native_termination::NativeTermination>,
    #[cfg(target_os = "macos")]
    updates: Option<updates::Updates>,
    icon_dialog: Option<icon_dialog::IconDialog>,
    text_edit: Option<(Id, String)>,
    image_source_request: Option<Id>,
    text_edit_view: Option<text_session::TextEditView>,
    text_edit_return_to_quick: bool,
    clipboard: Clipboard,
    pending_cut: Option<clipboard::PendingCut>,
    deferred_cut_input: Vec<egui::Event>,
    clipboard_paste: clipboard::PasteState,
    previews: previews::LibraryPreviews,
    recovery: Option<PathBuf>,
    _recovery_lock: Option<storage::ExclusiveFileLock>,
    recovery_offer: Option<Project>,
    last_recovery: Instant,
    recovery_doc: Option<Project>,
    help: bool,
    about: bool,
    present: bool,
    presentation_history: presentation::History,
    presentation_view: presentation::State,
    canvas_rect: Rect,
    canvas_focus_id: Option<egui::Id>,
}
impl WireframeApp {
    /// End transient pointer state at document/action boundaries. Commit the
    /// last visible canvas edit before a save captures its immutable document.
    fn end_pointer_gestures(&mut self) {
        self.cancel_quick_draw();
        self.editor.commit();
        self.drag = None;
        self.drag_scroll = None;
        self.library_drag = None;
        self.library_press = None;
        self.source_navigation.drag = None;
        self.canvas_press = None;
        self.canvas_clicks = canvas_click::CanvasClicks::default();
        if let Some(press) = &mut self.unlock_press {
            press.cancelled = true;
        }
        self.alignment_guides.clear();
    }
    fn canvas_input_blocked(&self) -> bool {
        self.startup_open.is_some()
            || self.pdf_options.is_some()
            || self.image_export.blocks_input()
            || self.versions.rename.is_some()
            || self.page_rename.is_some()
            || self.trash.is_some()
            || self.library_import.is_some()
            || self.more_controls.is_some()
            || self.page_paste.is_some()
            || self.save_busy()
            || self.recovery_chooser_open()
            || self.pending.is_some()
            || self.icon_dialog.is_some()
            || self.image_import.is_some()
            || self.web_image.is_some()
            || self.image_source_request.is_some()
            || self.error.is_some()
            || self.recovery_offer.is_some()
    }
    fn workspace(ctx: &egui::Context, mut recovery: Option<PathBuf>) -> Self {
        wireframe::fonts::install(ctx);
        ctx.options_mut(|options| options.input_options.zoom_modifier = Modifiers::COMMAND);
        ctx.set_theme(egui::Theme::Light);
        ctx.set_visuals(egui::Visuals::light());
        ctx.global_style_mut(|s| {
            s.spacing.item_spacing = Vec2::new(8., 8.);
            s.spacing.button_padding = Vec2::new(10., 6.);
            s.visuals.selection.bg_fill = Color32::from_rgb(218, 231, 245);
            s.visuals.selection.stroke = Stroke::new(1., ACCENT);
            s.visuals.panel_fill = Color32::from_rgb(248, 249, 251);
            s.visuals.window_fill = Color32::WHITE;
        });
        let mut status = "Local workspace · no account needed".to_owned();
        let recovery_lock = recovery
            .as_ref()
            .and_then(|p| match storage::acquire_recovery(p) {
                Ok(lock) => Some(lock),
                Err(e) => {
                    status = e;
                    None
                }
            });
        if recovery_lock.is_none() {
            recovery = None;
        }
        let mut recovery_error = None;
        let mut recovery_offer = None;
        if let Some(path) = recovery.clone() {
            match ProjectStore.load(&path) {
                Ok(project) => recovery_offer = Some(project),
                Err(error) if !matches!(std::fs::symlink_metadata(&path), Err(e) if e.kind() == std::io::ErrorKind::NotFound) => {
                    match storage::preserve_unreadable_recovery(&path) {
                        Ok(preserved) => {
                            status = "An unreadable recovery copy was preserved".into();
                            recovery_error = Some(format!(
                                "The previous recovery copy couldn't be opened. Its original bytes are preserved at:\n{}\n\nNew work will have its own recovery copy.\n\nDetails: {error}",
                                preserved.display()
                            ));
                        }
                        Err(preserve_error) => {
                            recovery = None;
                            status = "Automatic recovery is unavailable. Save your work manually."
                                .into();
                            recovery_error = Some(format!(
                                "The previous recovery copy couldn't be opened or moved aside. It remains unchanged at:\n{}\n\nAutomatic recovery is unavailable for this session. Save your work manually.\n\nDetails: {error}\n{preserve_error}",
                                path.display()
                            ));
                        }
                    }
                }
                Err(_) => {}
            }
        }
        Self {
            menus: menus::Menus::default(),
            compositor: None,
            editor: Editor::default(),
            installed_font: None,
            font_import: None,
            palette_defaults: Default::default(),
            image_export: Default::default(),
            versions: Default::default(),
            page_rename: None,
            page_inspector: Default::default(),
            review: Default::default(),
            source_navigation: Default::default(),
            library_import: None,
            more_controls: None,
            page_paste: None,
            page_paste_request: None,
            trash: None,
            navigator: Default::default(),
            viewports: Default::default(),
            pdf_options: None,
            pdf_job: None,
            quit_after_pdf: false,
            store: Box::new(ProjectStore),
            persistence: None,
            file: None,
            #[cfg(test)]
            save_dialog_choice: None,
            deferred_text_input: Vec::new(),
            deferred_text_focus: None,
            save_navigator_focus: None,
            quick: String::new(),
            quick_index: 0,
            quick_focus: false,
            quick_input_id: None,
            show_library: true,
            show_navigator: true,
            show_inspector: true,
            thumbnails: true,
            categories: vec![CatalogCategory::All],
            zoom: 1.,
            pan: Vec2::ZERO,
            drag: None,
            quick_draw: quick_draw::State::default(),
            drag_scroll: None,
            library_drag: None,
            library_press: None,
            library_anchor: Vec2::ZERO,
            alignment_guides: Vec::new(),
            canvas_press: None,
            canvas_clicks: canvas_click::CanvasClicks::default(),
            unlock_press: None,
            grid: false,
            snap: false,
            status,
            error: recovery_error,
            pending: None,
            startup_open: None,
            image_import: None,
            web_image: None,
            image_crop: None,
            image_split_fraction: 0.3,
            quit_allowed: false,
            window_close_pending: false,
            #[cfg(target_os = "macos")]
            native_termination: None,
            #[cfg(target_os = "macos")]
            updates: None,
            icon_dialog: None,
            text_edit: None,
            image_source_request: None,
            text_edit_view: None,
            text_edit_return_to_quick: false,
            clipboard: Clipboard::default(),
            pending_cut: None,
            deferred_cut_input: Vec::new(),
            clipboard_paste: Default::default(),
            previews: previews::LibraryPreviews::default(),
            recovery,
            _recovery_lock: recovery_lock,
            recovery_offer,
            last_recovery: Instant::now(),
            recovery_doc: None,
            help: false,
            about: false,
            present: false,
            presentation_history: presentation::History::default(),
            presentation_view: presentation::State::default(),
            canvas_rect: Rect::NOTHING,
            canvas_focus_id: None,
        }
    }
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::workspace(&cc.egui_ctx, storage::recovery_path());
        if let Some(gl) = cc.gl.as_deref() {
            match wireframe::render::compositor::Compositor::install(&cc.egui_ctx, gl) {
                Ok(compositor) => app.compositor = Some(compositor),
                Err(error) => {
                    app.error = Some(format!(
                        "Control fading is reduced on this graphics device. Editing and saving remain available.\n\n{error}"
                    ))
                }
            }
        }
        if let Some(path) = storage::recovery_path().and_then(|p| p.parent().map(|p| p.to_owned()))
        {
            app.load_palette_defaults(&path);
            app.load_reviewer(&path);
            app.load_image_export_preferences(&path);
            app.editor = app.new_editor();
            app.enable_persistence(&cc.egui_ctx, path);
        }
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        #[cfg(target_os = "macos")]
        let update_project = updates::take_reopened_project(!args.is_empty());
        if args.iter().any(|a| a == "--demo") {
            app.editor = Editor::new(Project::try_from(example_document()).expect("valid example"));
        } else if let Some(path) = args.first() {
            app.start_startup_open(PathBuf::from(path), &cc.egui_ctx);
        }
        #[cfg(target_os = "macos")]
        if let Some(path) = update_project {
            app.start_startup_open(path, &cc.egui_ctx);
        }
        app.init_menus(cc);
        #[cfg(target_os = "macos")]
        {
            app.native_termination = native_termination::NativeTermination::new(&cc.egui_ctx);
            match updates::Updates::start(app.native_termination.is_some()) {
                Ok(updates) => app.updates = updates,
                Err(error) => app.error = Some(format!("Updates are unavailable. {error}")),
            }
        }
        app
    }
    #[cfg(test)]
    fn load(&mut self, path: PathBuf) -> bool {
        self.remember_version_view();
        self.remember_viewports(true);
        self.remember_presentation();
        self.font_import = None;
        if self.persistence.is_some() {
            return self.load_persistent(path);
        }
        match self.store.load(&path) {
            Ok(doc) => {
                let candidate = Editor::new(doc);
                for page in candidate.document.live_pages() {
                    if let Err(error) = candidate.layout_page(page.id) {
                        self.error = Some(error);
                        return false;
                    }
                }
                self.review.reset();
                self.editor = candidate;
                self.file = Some(path);
                self.restore_version_view();
                self.status = "Project opened".into();
                true
            }
            Err(e) => {
                self.error = Some(e);
                false
            }
        }
    }
    fn save(&mut self, choose: bool) -> bool {
        if self.save_busy() || self.startup_open.is_some() {
            return false;
        }
        if !self.finish_page_notes() {
            return false;
        }
        self.remember_viewports(true);
        self.editor.commit();
        let path = if choose || self.file.is_none() {
            self.choose_save_path()
        } else {
            self.file.clone()
        };
        let Some(mut path) = path else {
            return false;
        };
        if path.extension().is_none() {
            path.set_extension("wireframe");
        }
        if self.persistence.is_some() {
            self.queue_save(path, choose);
            return false; // Completion arrives from the worker, never this frame.
        }
        match self.store.save(&path, &self.editor.document) {
            Ok(()) => {
                self.file = Some(path);
                self.editor.mark_saved();
                self.status = "Saved to your computer".into();
                self.clear_recovery();
                true
            }
            Err(e) => {
                self.error = Some(e);
                false
            }
        }
    }
    fn choose_save_path(&self) -> Option<PathBuf> {
        #[cfg(test)]
        {
            self.save_dialog_choice.clone()
        }
        #[cfg(not(test))]
        self.menus.with_dialog(|| {
            rfd::FileDialog::new()
                .add_filter("Wireframe project", &["wireframe"])
                .set_file_name(
                    self.file
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("Untitled.wireframe"),
                )
                .save_file()
        })
    }
    fn request(&mut self, pending: Pending, ctx: &egui::Context) {
        self.dismiss_recoveries();
        if self.startup_open.is_some() {
            if matches!(pending, Pending::Quit) {
                if !self.cancel_open_for_quit() {
                    return;
                }
            } else {
                return;
            }
        }
        if self.save_busy() {
            return;
        }
        if !self.finish_page_notes() {
            return;
        }
        if matches!(pending, Pending::Quit) && self.image_export.job.is_some() {
            self.image_export.quit_after = true;
            self.status = "Finishing image export before closing…".into();
            return;
        }
        if matches!(pending, Pending::Quit) && self.pdf_job.is_some() {
            self.quit_after_pdf = true;
            self.status = "Finishing PDF export before closing…".into();
            return;
        }
        self.pdf_options = None;
        self.image_export.draft = None;
        self.versions.rename = None;
        self.page_rename = None;
        self.page_inspector.clear();
        self.source_navigation = Default::default();
        self.library_import = None;
        self.more_controls = None;
        self.page_paste = None;
        self.page_paste_request = None;
        self.trash = None;
        self.font_import = None;
        self.end_pointer_gestures();
        if self.automatic_named() {
            self.pending = Some(pending);
            self.queue_save(self.file.clone().unwrap(), false);
        } else if self.editor.dirty() {
            self.pending = Some(pending);
        } else {
            self.perform(pending, ctx);
        }
    }
    fn perform(&mut self, pending: Pending, ctx: &egui::Context) {
        if let Pending::Open(path) = pending {
            self.start_menu_open(path, false, ctx);
            return;
        }
        self.remember_version_view();
        self.remember_viewports(true);
        self.remember_presentation();
        if self.persistence.is_some() && !self.finish_session(false) {
            return;
        }
        match pending {
            Pending::New => {
                self.reset_session_identity();
                self.editor = self.new_editor();
                self.categories = library::default_categories();
                self.font_import = None;
                self.file = None;
                self.status = "New project · not saved to a file".into();
                self.restore_viewports();
                self.clear_recovery();
            }
            Pending::Open(_) => unreachable!("Open is prepared before session handoff"),
            Pending::Quit => {
                #[cfg(target_os = "macos")]
                if let Some(updates) = &self.updates
                    && let Err(error) = updates.prepare_relaunch(self.file.as_deref())
                {
                    self.error = Some(format!(
                        "Could not prepare project reopening after the update. The app remains open.\n\n{error}"
                    ));
                    return;
                }
                self.quit_allowed = true;
                self.clear_recovery();
                #[cfg(target_os = "macos")]
                if self
                    .native_termination
                    .as_ref()
                    .is_some_and(|gate| gate.approve())
                {
                    return;
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Pending::Restore(id) => {
                self.restore_copy(id);
            }
            Pending::Example => {
                self.reset_session_identity();
                self.categories = library::default_categories();
                self.clear_recovery();
                self.editor =
                    Editor::new(Project::try_from(example_document()).expect("valid example"));
                self.file = None;
                self.status = "Example project · not saved to a file".into();
                self.restore_viewports();
            }
        }
    }
    fn open(&mut self, ctx: &egui::Context) {
        if let Some(path) = self.menus.with_dialog(|| {
            rfd::FileDialog::new()
                .add_filter("Wireframe project", &["wireframe"])
                .pick_file()
        }) {
            self.request(Pending::Open(path), ctx);
        }
    }
    fn export(&mut self, png: bool) {
        if !self.finish_page_notes() {
            return;
        }
        let ext = if png { "png" } else { "svg" };
        if let Some(path) = self.menus.with_dialog(|| {
            rfd::FileDialog::new()
                .add_filter("Wireframe export", &[ext])
                .set_file_name(format!("{}.{}", self.editor.current().name, ext))
                .save_file()
        }) {
            match self.write_page_export(&path, png) {
                Ok(()) => self.status = format!("Exported {}", path.display()),
                Err(e) => self.error = Some(e),
            }
        }
    }
    fn write_page_export(&self, path: &std::path::Path, png: bool) -> Result<(), String> {
        self.ensure_export_destination(path)?;
        let drawing = self.editor.export_scene(
            self.editor.surface_id(),
            &wireframe::workspace::SceneScope::Whole,
        )?;
        let bytes = if png {
            drawing.png()?
        } else {
            drawing.svg().into_bytes()
        };
        storage::write_atomic(path, &bytes)
    }
    fn ensure_export_destination(&self, path: &std::path::Path) -> Result<(), String> {
        let same_project = self.file.as_ref().is_some_and(|project| {
            project == path
                || project
                    .canonicalize()
                    .ok()
                    .zip(path.canonicalize().ok())
                    .is_some_and(|(a, b)| a == b)
        });
        if same_project {
            Err("Choose an export destination other than the open project file".into())
        } else {
            Ok(())
        }
    }
    fn quick_add_ui(&mut self, ui: &mut egui::Ui) {
        let response = toolbar::quick_input(ui, &mut self.quick);
        self.quick_input_id = Some(response.id);
        if self.quick_focus {
            response.request_focus();
            self.quick_focus = false;
        }
        if response.changed() {
            self.quick_index = 0;
        }
        let matches = self.editor.search_catalog(&self.quick);
        self.quick_index = self.quick_index.min(matches.len().saturating_sub(1));
        if response.has_focus() && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown))
        {
            self.quick_index = (self.quick_index + 1).min(matches.len().saturating_sub(1));
        }
        if response.has_focus() && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp)) {
            self.quick_index = self.quick_index.saturating_sub(1);
        }
        let mut chosen = None;
        if (response.has_focus() || response.lost_focus())
            && !self.quick.is_empty()
            && ui.input(|i| i.key_pressed(Key::Enter))
        {
            chosen = matches.get(self.quick_index).map(|entry| entry.item);
        }
        if response.has_focus() && !self.quick.is_empty() {
            let mut open = true;
            egui::Popup::from_response(&response)
                .open_bool(&mut open)
                .width(230.)
                .show(|ui| {
                    if matches.is_empty() {
                        ui.label("No matching controls");
                    }
                    for (index, entry) in matches.iter().enumerate().take(10) {
                        if ui
                            .selectable_label(index == self.quick_index, &entry.name)
                            .clicked()
                        {
                            chosen = Some(entry.item);
                        }
                    }
                });
        }
        if let Some(item) = chosen {
            let point = if ui.input(|i| i.modifiers.shift) {
                ui.ctx().pointer_latest_pos().map(|pos| self.world(pos))
            } else {
                None
            };
            self.add_catalog_item(item, point);
            self.quick.clear();
            response.request_focus();
        }
    }
    fn inspector(&mut self, ui: &mut egui::Ui) {
        let page = self.editor.selection.is_empty() && !self.editor.editing_instance();
        let frame = if page {
            egui::Frame::NONE.fill(Color32::from_gray(238))
        } else {
            egui::Frame::side_top_panel(ui.style())
        };
        chrome::prepare_panel_resize(
            ui,
            egui::Id::new("inspector"),
            chrome::PanelEdge::Right,
            220. ..=400.,
            !self.canvas_input_blocked(),
        );
        egui::Panel::right("inspector").resizable(true).default_size(300.).size_range(220.0..=400.0).frame(frame).show(ui,|ui|{
            if self.linked_destination_notice(ui) { return; }
            if page { self.page_inspector(ui); return; }
            ui.add_space(10.);egui::ScrollArea::vertical().show(ui,|ui|{
                let selected=self.editor.selected_views();
                if selected.is_empty() && self.editor.editing_instance() {ui.heading("Component instance");ui.label("Select a child to change this instance. Reset returns a property to its source value.");return;}
                ui.heading(if selected.len()==1{self.editor.selection_label(selected[0].id)}else{format!("{} controls",selected.len())});
                self.arrangement_properties(ui);
                self.geometry_properties(ui);
                self.auto_size_properties(ui);
                let selected=self.editor.selected_views();
                let movable = selected.iter().any(|c| !c.locked);
                if selected.len()==1 {let mut c=selected[0].clone();let original=c.clone();
                    self.layering_properties(ui);
                    if c.group.is_none() && !self.typed_properties(ui, c.id) {ui.add_space(10.);ui.label("Text");wireframe::text_entry::multiline(ui, &mut c.text, 4, false);if c.kind==Kind::Table {ui.small("Separate columns with commas and rows with new lines.");}
if c.kind==Kind::Tabs {ui.small("Separate tabs with commas.");}
                    ui.horizontal(|ui|{ui.label("Size");ui.add(egui::DragValue::new(&mut c.font_size).range(6.0..=120.0));ui.checkbox(&mut c.bold,"Bold");});
                    ui.horizontal(|ui|{ui.label("Fill");ui.color_edit_button_srgb(&mut c.fill);});if matches!(c.kind,Kind::Checkbox|Kind::Radio|Kind::Toggle){ui.checkbox(&mut c.checked,"Selected state");}
                    }
                    let mut own_lock=self.editor.own_locked(c.id);if ui.checkbox(&mut own_lock,"Lock position").changed(){self.editor.lock(own_lock);c.locked=self.editor.position_locked(c.id);}
                    if self.whole_link_available(&c) {
                    let typed_link = self.editor.control_properties(c.id).is_some_and(|(spec,_)| spec.property("link").is_some_and(|field| field.visible));
                    if c.group.is_some() || !typed_link { self.whole_link_properties(ui,c.id); }
                    }
                    if self.editor.is_group(c.id) && ui.button("Edit group contents").clicked(){self.editor.enter(c.id);}
                    if c!=original {if c.link == original.link {c.link=self.editor.node_link(c.id).and_then(|link|match link {wireframe::document::Link::Page(id)=>Some(id),_=>None});}self.editor.begin("Edit properties");if original.locked {c.bounds=original.bounds;}
                        self.editor.update_control(c);}
                    if !wireframe::text_entry::is_focused(ui.ctx()) && !ui.ctx().text_edit_focused()&&!ui.input(|i|i.pointer.any_down()){self.editor.commit();}
                }else {self.layering_properties(ui);self.selection_properties(ui);}
                self.inline_link_properties(ui, &selected.iter().map(|c| c.id).collect::<Vec<_>>());
                if selected.len() == 1 { self.group_crop_properties(ui, selected[0].id); }
                self.component_actions(ui);
                if selected.len()>1 {ui.horizontal(|ui|{if ui.button("Lock").clicked(){self.editor.lock(true);}
if ui.button("Unlock").clicked(){self.editor.lock(false);}});}
                if self.editor.editing_instance() {ui.small("Edit the source to add, remove or rearrange children.");return;}
                ui.add_space(14.);ui.separator();ui.horizontal(|ui|{if ui.add_enabled(movable, egui::Button::new("Group")).clicked(){self.editor.group();}
if ui.add_enabled(movable, egui::Button::new("Ungroup")).clicked(){self.editor.ungroup();}});ui.horizontal(|ui|{if ui.button("Duplicate").clicked(){self.editor.duplicate();}
if ui.add_enabled(movable, egui::Button::new("Delete")).clicked(){self.editor.delete();}});

            });
        });
    }
    fn screen(&self, p: [f32; 2]) -> Pos2 {
        self.canvas_rect.min + self.pan + Vec2::new(p[0], p[1]) * self.zoom
    }
    fn world(&self, p: Pos2) -> Pos2 {
        let v = (p - self.canvas_rect.min - self.pan) / self.zoom;
        Pos2::new(v.x, v.y)
    }
    fn screen_rect(&self, b: Bounds) -> Rect {
        Rect::from_min_size(self.screen([b.x, b.y]), Vec2::new(b.w, b.h) * self.zoom)
    }
    fn paint_scene(
        painter: &egui::Painter,
        drawing: &wireframe::render::Scene,
        origin: Pos2,
        scale: f32,
    ) {
        let scene_start = painter.ctx().graphics(|g| {
            g.get(painter.layer_id())
                .map_or(0, |list| list.next_idx().0)
        });
        let mut masks = Vec::new();
        let mut opacity_scopes = Vec::new();
        for item in &drawing.items {
            match &item.drawing {
                wireframe::render::Drawing::BeginOpacity(opacity) => {
                    let start = painter.ctx().graphics(|g| {
                        g.get(painter.layer_id())
                            .map_or(0, |list| list.next_idx().0)
                    });
                    opacity_scopes.push((start, *opacity));
                    continue;
                }
                wireframe::render::Drawing::EndOpacity => {
                    if let Some((start, opacity)) = opacity_scopes.pop() {
                        wireframe::render::compositor::Compositor::finish(painter, start, opacity);
                    }
                    continue;
                }
                wireframe::render::Drawing::BeginMask(mask) => {
                    let start = painter.ctx().graphics(|g| {
                        g.get(painter.layer_id())
                            .map_or(0, |list| list.next_idx().0)
                    });
                    masks.push((start, mask));
                    continue;
                }
                wireframe::render::Drawing::EndMask => {
                    if let Some((start, mask)) = masks.pop() {
                        wireframe::render::mask::finish(painter, start, mask, origin, scale);
                    }
                    continue;
                }
                _ => {}
            }
            let mut clipped = item.clip.map_or_else(
                || painter.clone(),
                |b| {
                    painter.with_clip_rect(
                        Rect::from_min_size(
                            origin + Vec2::new(b.x, b.y) * scale,
                            Vec2::new(b.w, b.h) * scale,
                        )
                        .intersect(painter.clip_rect()),
                    )
                },
            );
            clipped.multiply_opacity(item.opacity);
            if !clipped.clip_rect().is_positive() {
                continue;
            }
            match &item.drawing {
                wireframe::render::Drawing::BeginMask(_)
                | wireframe::render::Drawing::EndMask
                | wireframe::render::Drawing::BeginOpacity(_)
                | wireframe::render::Drawing::EndOpacity => {
                    unreachable!()
                }
                wireframe::render::Drawing::Icon {
                    bounds: b,
                    id,
                    color,
                    turns,
                } => {
                    let rect = Rect::from_min_size(
                        origin + Vec2::new(b.x, b.y) * scale,
                        Vec2::new(b.w, b.h) * scale,
                    );
                    if let Some(icon) = wireframe::icons::get(id) {
                        icon.paint_rotated(
                            &clipped,
                            rect,
                            Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]),
                            *turns,
                        );
                    } else {
                        clipped.rect_stroke(
                            rect,
                            0.,
                            egui::Stroke::new(
                                scale,
                                Color32::from_rgb(color[0], color[1], color[2]),
                            ),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
                wireframe::render::Drawing::Image { bounds: b, data } => {
                    let rect = Rect::from_min_size(
                        origin + Vec2::new(b.x, b.y) * scale,
                        Vec2::new(b.w, b.h) * scale,
                    );
                    clipped.image(
                        data.texture(clipped.ctx()),
                        rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
                        Color32::WHITE,
                    );
                }
                wireframe::render::Drawing::Primitive(p) => {
                    Self::paint_at(&clipped, p.clone(), origin, scale)
                }
                wireframe::render::Drawing::Glyph(g) => {
                    wireframe::render::paint_glyph(&clipped, g, origin, scale)
                }
            }
        }
        wireframe::render::compositor::Compositor::resolve(painter, scene_start);
    }
    fn paint_at(painter: &egui::Painter, p: Primitive, origin: Pos2, scale: f32) {
        let screen = |p: [f32; 2]| origin + Vec2::new(p[0], p[1]) * scale;
        let screen_rect =
            |b: Bounds| Rect::from_min_size(screen([b.x, b.y]), Vec2::new(b.w, b.h) * scale);
        match p {
            Primitive::Rect {
                b,
                fill,
                stroke,
                radius,
            } => {
                painter.rect(
                    screen_rect(b),
                    (radius * scale).clamp(0., 255.) as u8,
                    Color32::from_rgb(fill[0], fill[1], fill[2]),
                    if stroke {
                        Stroke::new(1.5 * scale, Color32::from_rgb(44, 48, 55))
                    } else {
                        Stroke::NONE
                    },
                    StrokeKind::Inside,
                );
            }
            Primitive::StyledEllipse {
                b,
                fill,
                stroke,
                width,
            } => {
                let r = screen_rect(b);
                painter.add(egui::epaint::EllipseShape::filled(
                    r.center(),
                    r.size() / 2.,
                    Color32::from_rgb(fill[0], fill[1], fill[2]),
                ));
                if width > 0. {
                    painter.add(egui::epaint::EllipseShape::stroke(
                        r.center(),
                        r.size() / 2.,
                        Stroke::new(
                            width * scale,
                            Color32::from_rgb(stroke[0], stroke[1], stroke[2]),
                        ),
                    ));
                }
            }
            Primitive::Ellipse { b, fill } => {
                let r = screen_rect(b);
                painter.add(egui::epaint::EllipseShape::filled(
                    r.center(),
                    r.size() / 2.,
                    Color32::from_rgb(fill[0], fill[1], fill[2]),
                ));
                painter.add(egui::epaint::EllipseShape::stroke(
                    r.center(),
                    r.size() / 2.,
                    Stroke::new(1.5 * scale, Color32::from_rgb(44, 48, 55)),
                ));
            }
            Primitive::Mesh {
                points,
                indices,
                fill,
            } => {
                if wireframe::render::coverage::paint(
                    painter, &points, &indices, fill, origin, scale,
                )
                .is_ok()
                {
                    return;
                }
                // Preserve the previous drawing path if invalid or out-of-range
                // geometry cannot be represented by the coverage renderer.
                let mut mesh = egui::Mesh::default();
                let color = Color32::from_rgb(fill[0], fill[1], fill[2]);
                for point in points {
                    mesh.colored_vertex(screen(point), color);
                }
                mesh.indices = indices;
                painter.add(egui::Shape::mesh(mesh));
            }
            Primitive::Polygon { points, fill } => {
                painter.add(egui::Shape::convex_polygon(
                    points.into_iter().map(screen).collect(),
                    Color32::from_rgb(fill[0], fill[1], fill[2]),
                    Stroke::NONE,
                ));
            }
            Primitive::Line {
                points,
                color,
                width,
            } => {
                painter.add(egui::Shape::line(
                    points.into_iter().map(screen).collect(),
                    Stroke::new(
                        width * scale,
                        Color32::from_rgb(color[0], color[1], color[2]),
                    ),
                ));
            }
            Primitive::Text {
                x,
                y,
                text,
                size,
                bold,
                color,
                center,
            } => {
                let pos = screen([x, y - size * 0.85]);
                let font = FontId::proportional(size * scale);
                let anchor = if center {
                    Align2::CENTER_TOP
                } else {
                    Align2::LEFT_TOP
                };
                let col = Color32::from_rgb(color[0], color[1], color[2]);
                painter.text(pos, anchor, &text, font.clone(), col);
                if bold {
                    painter.text(pos + Vec2::new(0.35 * scale, 0.), anchor, text, font, col);
                }
            }
        }
    }
    fn resize_handles(&self, bounds: Bounds) -> Vec<((i8, i8), Pos2)> {
        let selected = self.editor.selected_views();
        let single = selected.len() == 1 && selected[0].group.is_none();
        if single
            && matches!(
                selected[0].kind,
                Kind::Arrow
                    | Kind::SiteMap
                    | Kind::ColorPicker
                    | Kind::HelpButton
                    | Kind::Icon
                    | Kind::Breadcrumbs
                    | Kind::LinkBar
                    | Kind::Playback
            )
        {
            return vec![];
        }
        let axis = if single {
            self.editor.linear_axis(selected[0].id)
        } else {
            None
        };
        let horizontal = single
            && (matches!(
                selected[0].kind,
                Kind::IconLabel
                    | Kind::PointedButton
                    | Kind::Heading
                    | Kind::Subtitle
                    | Kind::NumericStepper
            ) || axis.is_some_and(|a| !a.vertical));
        let vertical = axis.is_some_and(|a| a.vertical);
        let r = self.screen_rect(bounds).expand(4.);
        [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ]
        .into_iter()
        .filter(|(x, y)| {
            (!horizontal || *y == 0)
                && (!vertical || *x == 0)
                && (bounds.w > 0. || *x == 0)
                && (bounds.h > 0. || *y == 0)
        })
        .map(|(x, y)| {
            (
                (x, y),
                Pos2::new(
                    r.center().x + x as f32 * r.width() / 2.,
                    r.center().y + y as f32 * r.height() / 2.,
                ),
            )
        })
        .collect()
    }
    fn instant_drag(ui: &egui::Ui, rect: Rect) -> Option<(Pos2, Pos2)> {
        let result = ui.input(|i| {
            let mut press = None;
            let mut result = None;
            for event in &i.events {
                if let egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    ..
                } = event
                {
                    if *pressed {
                        press = Some(*pos);
                    } else if let Some(start) = press.take()
                        && rect.contains(start)
                        && start.distance(*pos) > 6.
                    {
                        result = Some((start, *pos));
                    }
                }
            }
            result
        });
        result.filter(|(start, _)| ui.ctx().layer_id_at(*start) == Some(ui.layer_id()))
    }
    fn canvas(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::WHITE))
            .show(ui, |ui| {
                if self.canvas_input_blocked() || chrome::panel_owns_pointer(ui.ctx()) {
                    self.end_pointer_gestures();
                    ui.disable();
                }
                let (response, painter) =
                    ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
                self.canvas_rect = response.rect;
                self.canvas_focus_id = Some(response.id);
                let ctx = ui.ctx().clone();
                self.sync_viewport();
                self.viewports.canvas_layer = Some(response.layer_id);
                if self.present {
                    self.presentation_canvas(ui, &response, &painter);
                    return;
                }
                if !self.canvas_input_blocked()
                    && self.text_edit.is_none()
                    && ui.input(|i| i.focused)
                {
                    self.viewport_input(&ctx);
                } else {
                    self.reset_canvas_scroll();
                }
                self.library_drag_update(&ctx);
                if !ui.input(|i| i.focused) {
                    if self.drag.is_some() {
                        self.editor.cancel();
                    }
                    self.drag = None;
                    self.canvas_press = None;
                    self.alignment_guides.clear();
                }
                if self.image_crop.is_some() {
                    self.drag_scroll = None;
                    self.crop_canvas(ui, &response, &painter);
                    return;
                }
                if self.review_pointer(ui) {
                    if let Ok(scene) = self.editor.editing_scene() {
                        Self::paint_scene(
                            &painter,
                            &scene,
                            self.canvas_rect.min + self.pan,
                            self.zoom,
                        );
                    }
                    self.paint_review_markers(&painter);
                    return;
                }
                let quick_draw_used =
                    self.unlock_press.is_none() && self.quick_draw_input(ui, &response);
                let unlock_used = !quick_draw_used && self.unlock_badge_input(ui, &response);
                if !quick_draw_used && !unlock_used {
                    // Native automation and fast mice can deliver a complete drag between
                    // paints. Preserve the press origin even after egui releases its state.
                    let instant_drag = if ui.input(|i| i.focused && !i.key_pressed(Key::Escape))
                        && !self.save_busy()
                        && !self.recovery_chooser_open()
                        && self.pending.is_none()
                        && self.text_edit.is_none()
                        && self.icon_dialog.is_none()
                        && self.error.is_none()
                        && self.recovery_offer.is_none()
                    {
                        let mut completed = None;
                        for event in ui.input(|i| i.events.clone()) {
                            if let egui::Event::PointerButton {
                                pos,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers,
                            } = event
                            {
                                if pressed {
                                    self.canvas_press = (response.rect.contains(pos)
                                        && ctx.layer_id_at(pos) == Some(ui.layer_id()))
                                    .then_some((pos, modifiers));
                                } else if let Some((start, modifiers)) = self.canvas_press.take()
                                    && start.distance(pos) > 6.
                                {
                                    completed = Some((start, pos, modifiers));
                                }
                            }
                        }
                        completed
                    } else {
                        self.canvas_press = None;
                        None
                    };
                    let pointer = instant_drag
                        .map(|(_, end, _)| end)
                        .or(response.interact_pointer_pos());
                    if self.drag.is_none()
                        && (response.drag_started() || instant_drag.is_some())
                        && let Some(pos) = instant_drag
                            .map(|(start, _, _)| start)
                            .or(ui.input(|i| i.pointer.press_origin()))
                            .or(pointer)
                    {
                        let start = self.world(pos);
                        let press_modifiers = instant_drag
                            .map(|(_, _, modifiers)| modifiers)
                            .or(self.canvas_press.map(|(_, modifiers)| modifiers))
                            .unwrap_or_else(|| ui.input(|i| i.modifiers));
                        let space = ui.input(|i| {
                            i.key_down(Key::Space)
                                || i.pointer.button_down(egui::PointerButton::Middle)
                        });
                        let resize = self
                            .editor
                            .movable_bounds()
                            .and_then(|b| {
                                self.resize_handles(b)
                                    .into_iter()
                                    .find(|(_, point)| point.distance(pos) < 7.)
                            })
                            .filter(|_| self.editor.selected_views().iter().any(|c| !c.locked));
                        if space {
                            self.drag = Some(Drag::Pan {
                                start: pos,
                                pan: self.pan,
                            });
                        } else if let Some(handle) = self
                            .arrow_handles()
                            .into_iter()
                            .find(|h| h.position.distance(pos) < 7.)
                        {
                            self.editor.begin("Move arrow handle");
                            self.drag = Some(Drag::Arrow {
                                id: handle.id,
                                handle: handle.kind,
                                origin: handle.origin,
                                offset: handle.offset,
                            });
                        } else if let Some((handle, _)) = resize.filter(|_| !self.present) {
                            self.editor.begin("Resize");
                            self.drag = Some(Drag::Resize {
                                start,
                                original: self
                                    .editor
                                    .selected_views()
                                    .iter()
                                    .filter(|c| !c.locked)
                                    .map(|c| (c.id, c.bounds))
                                    .collect(),
                                bounds: self.editor.movable_bounds().unwrap(),
                                handle,
                                targets: self
                                    .editor
                                    .alignment_targets(5.)
                                    .unwrap_or_else(|_| Targets::new(5.)),
                            });
                        } else {
                            let hit = self
                                .editor
                                .hit(start.x, start.y)
                                .filter(|_| !press_modifiers.shift)
                                .map(|c| (c.id, c.locked));
                            if let Some((id, locked)) = hit {
                                if !self.editor.selection.contains(&id)
                                    && (!self.editor.own_locked(id)
                                        || ui.input(|i| i.modifiers.command))
                                {
                                    self.editor.select(id, ui.input(|i| i.modifiers.shift));
                                }
                                if !locked && !self.present {
                                    self.editor.begin("Move");
                                    self.drag = Some(Drag::Move {
                                        start,
                                        targets: self
                                            .editor
                                            .alignment_targets(5.)
                                            .unwrap_or_else(|_| Targets::new(5.)),
                                        duplicate_targets: self
                                            .editor
                                            .alignment_targets_with_selection(5., true)
                                            .unwrap_or_else(|_| Targets::new(5.)),
                                        motion: Motion::default(),
                                        original: self
                                            .editor
                                            .selected_views()
                                            .iter()
                                            .filter(|c| !c.locked)
                                            .map(|c| (c.id, c.bounds))
                                            .collect(),
                                    });
                                }
                            } else if !self.present {
                                let previous = if press_modifiers.shift {
                                    self.editor.selection.clone()
                                } else {
                                    HashSet::new()
                                };
                                self.editor.selection = previous.clone();
                                self.drag = Some(Drag::Marquee {
                                    start,
                                    now: start,
                                    previous,
                                });
                            }
                        }
                    }
                    let scrolled = self.scroll_during_drag(&ctx);
                    if (scrolled
                        || response.dragged()
                        || (matches!(self.drag, Some(Drag::Move { .. }))
                            && ui.input(|i| {
                                i.events
                                    .iter()
                                    .any(|e| matches!(e, egui::Event::ModifiersChanged(_)))
                            }))
                        || response.drag_stopped()
                        || instant_drag.is_some())
                        && let (Some(pos), Some(drag)) = (pointer, self.drag.take())
                    {
                        let now = self.world(pos);
                        match drag {
                            Drag::Arrow {
                                id,
                                handle,
                                origin,
                                offset,
                            } => {
                                self.editor.reset_preview();
                                let target = wireframe::document::Point::new(
                                    now.x - origin.x + offset.x,
                                    now.y - origin.y + offset.y,
                                );
                                if let Err(error) = self.editor.move_arrow_handle(
                                    id,
                                    handle,
                                    target,
                                    ui.input(|i| i.modifiers.shift),
                                ) {
                                    self.error = Some(error);
                                }
                                self.drag = Some(Drag::Arrow {
                                    id,
                                    handle,
                                    origin,
                                    offset,
                                });
                            }
                            Drag::Pan { start, pan } => {
                                self.pan = pan + (pos - start);
                                self.drag = Some(Drag::Pan { start, pan });
                            }
                            Drag::Move {
                                start,
                                original,
                                targets,
                                duplicate_targets,
                                mut motion,
                            } => {
                                let mut delta = now - start;
                                let modifiers = ui.input(|i| {
                                    i.events
                                        .iter()
                                        .rev()
                                        .find_map(|e| {
                                            if let egui::Event::PointerButton {
                                                button: egui::PointerButton::Primary,
                                                pressed: false,
                                                modifiers,
                                                ..
                                            } = e
                                            {
                                                Some(*modifiers)
                                            } else {
                                                None
                                            }
                                        })
                                        .unwrap_or(i.modifiers)
                                });
                                let duplicate = modifiers.alt && self.editor.can_duplicate_move();
                                let constraint = if modifiers.shift {
                                    if delta.x.abs() > delta.y.abs() {
                                        delta.y = 0.;
                                        Constraint::Horizontal
                                    } else {
                                        delta.x = 0.;
                                        Constraint::Vertical
                                    }
                                } else {
                                    Constraint::Free
                                };
                                let mut fast = motion.too_fast();
                                let events = ui.input(|i| i.events.clone());
                                // A coalesced native frame may contain pointer travel before
                                // the press. Only movement belonging to this gesture counts.
                                let first = events
                                    .iter()
                                    .rposition(|event| {
                                        matches!(
                                            event,
                                            egui::Event::PointerButton {
                                                button: egui::PointerButton::Primary,
                                                pressed: true,
                                                ..
                                            }
                                        )
                                    })
                                    .map_or(0, |index| index + 1);
                                for event in events.into_iter().skip(first) {
                                    if let egui::Event::PointerMoved(p) = event {
                                        fast = motion.too_fast();
                                        let d = self.world(p) - start;
                                        motion.push([d.x, d.y]);
                                    }
                                }
                                if response.drag_stopped() || instant_drag.is_some() {
                                    fast = motion.too_fast();
                                }
                                self.alignment_guides.clear();
                                let bypass = modifiers.command;
                                if !bypass
                                    && !self.snap
                                    && !fast
                                    && let Some(bounds) =
                                        original.iter().map(|(_, b)| *b).reduce(Bounds::union)
                                {
                                    let wanted = Bounds::new(
                                        bounds.x + delta.x,
                                        bounds.y + delta.y,
                                        bounds.w,
                                        bounds.h,
                                    );
                                    let result = if duplicate {
                                        &duplicate_targets
                                    } else {
                                        &targets
                                    }
                                    .moved(wanted, constraint);
                                    if result.bounds.x >= 0. && result.bounds.y >= 0. {
                                        delta = Vec2::new(
                                            result.bounds.x - bounds.x,
                                            result.bounds.y - bounds.y,
                                        );
                                        self.alignment_guides = result.guides;
                                    }
                                }
                                let grid = self.snap && !bypass;
                                let offsets = original
                                    .iter()
                                    .map(|(id, b)| {
                                        let mut x = (b.x + delta.x).clamp(-50_000., 50_000.);
                                        let mut y = (b.y + delta.y).clamp(-50_000., 50_000.);
                                        if grid {
                                            x = (x / 10.).round() * 10.;
                                            y = (y / 10.).round() * 10.;
                                        }
                                        (*id, wireframe::document::Point::new(x - b.x, y - b.y))
                                    })
                                    .collect::<Vec<_>>();
                                if let Err(error) = self.editor.preview_move(&offsets, duplicate) {
                                    self.error = Some(error);
                                }
                                self.drag = Some(Drag::Move {
                                    start,
                                    original,
                                    targets,
                                    duplicate_targets,
                                    motion,
                                });
                            }
                            Drag::Resize {
                                start,
                                original,
                                bounds,
                                handle,
                                targets,
                            } => {
                                self.editor.reset_preview();
                                let mut delta = now - start;
                                let modifiers = ui.input(|i| i.modifiers);
                                let bypass = modifiers.command || modifiers.alt;
                                if self.snap && !bypass {
                                    delta.x = (delta.x / 10.).round() * 10.;
                                    delta.y = (delta.y / 10.).round() * 10.;
                                }
                                let selected = self.editor.selected_views();
                                let minimum = self.editor.resize_minimum(
                                    &original.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
                                );
                                let axis = (selected.len() == 1 && original.len() == 1)
                                    .then(|| selected[0].id)
                                    .and_then(|id| self.editor.linear_axis(id));
                                let scalable_media = selected.len() == 1
                                    && original.len() == 1
                                    && matches!(
                                        selected[0].kind,
                                        Kind::Popover
                                            | Kind::StreetMap
                                            | Kind::ModalOverlay
                                            | Kind::Webcam
                                            | Kind::Video
                                            | Kind::CoverFlow
                                            | Kind::ColumnChart
                                            | Kind::BarChart
                                            | Kind::LineChart
                                            | Kind::PieChart
                                            | Kind::HorizontalBrace
                                            | Kind::VerticalBrace
                                            | Kind::RedCross
                                            | Kind::ScratchOut
                                            | Kind::TextBlock
                                            | Kind::TextLine
                                            | Kind::TagCloud
                                            | Kind::Heading
                                            | Kind::Subtitle
                                            | Kind::NumericStepper
                                            | Kind::Callout
                                            | Kind::StickyNote
                                            | Kind::Tooltip
                                    );
                                let aspect = modifiers.shift
                                    && axis.is_none()
                                    && !(selected.len() == 1
                                        && matches!(
                                            selected[0].kind,
                                            Kind::Heading | Kind::Subtitle | Kind::NumericStepper
                                        ));
                                let mut resized = if let Some(axis) = axis {
                                    axis.resize(
                                        bounds,
                                        if axis.vertical { handle.1 } else { handle.0 },
                                        if axis.vertical { delta.y } else { delta.x },
                                    )
                                } else if scalable_media {
                                    wireframe::media_controls::resize_placeholder(
                                        bounds,
                                        handle,
                                        (delta.x, delta.y),
                                        aspect,
                                    )
                                } else {
                                    bounds.resized_with_minimum_size(
                                        handle,
                                        (delta.x, delta.y),
                                        aspect,
                                        minimum,
                                    )
                                };
                                self.alignment_guides.clear();
                                if !self.snap && !bypass {
                                    let result = targets.resized(
                                        resized,
                                        handle,
                                        aspect.then_some(bounds.w / bounds.h),
                                    );
                                    if (!scalable_media
                                        || (result.bounds.w >= 10. && result.bounds.h >= 10.))
                                        && (handle.0 == 0 || result.bounds.w >= minimum.0)
                                        && (handle.1 == 0 || result.bounds.h >= minimum.1)
                                        && axis.is_none_or(|a| {
                                            if a.vertical {
                                                result.bounds.h >= 10.
                                            } else {
                                                result.bounds.w >= 10.
                                            }
                                        })
                                    {
                                        resized = result.bounds;
                                        self.alignment_guides = result.guides;
                                    }
                                }
                                self.editor.update_selection_bounds(resized);
                                self.drag = Some(Drag::Resize {
                                    start,
                                    original,
                                    bounds,
                                    handle,
                                    targets,
                                });
                            }
                            Drag::Marquee {
                                start, previous, ..
                            } => {
                                let r = Bounds::new(
                                    start.x.min(now.x),
                                    start.y.min(now.y),
                                    (start.x - now.x).abs(),
                                    (start.y - now.y).abs(),
                                );
                                self.editor.select_area(r, &previous);
                                self.drag = Some(Drag::Marquee {
                                    start,
                                    now,
                                    previous,
                                });
                            }
                        }
                    }
                    if response.drag_stopped() || instant_drag.is_some() {
                        self.editor.commit();
                        self.drag = None;
                        self.drag_scroll = None;
                        self.alignment_guides.clear();
                    }
                    let double_click = self.canvas_double_click(ui, &response);
                    let arrow_handle =
                        double_click
                            .map(|(pos, _)| pos)
                            .or(pointer)
                            .and_then(|pos| {
                                self.arrow_handles()
                                    .into_iter()
                                    .find(|h| h.position.distance(pos) < 7.)
                            });
                    if double_click.is_some()
                        && let Some(handle) = arrow_handle
                            .as_ref()
                            .filter(|h| h.kind == wireframe::arrows::Handle::Middle)
                    {
                        self.editor.begin("Reset arrow bend");
                        if let Err(error) = self.editor.snap_arrow_middle(handle.id) {
                            self.error = Some(error);
                        }
                        self.editor.commit();
                    }
                    if response.clicked()
                        && arrow_handle.is_none()
                        && instant_drag.is_none()
                        && let Some(pos) = pointer
                    {
                        // A modifier can be released after the click but before this paint.
                        // Use the click's snapshot rather than the frame's final key state.
                        let modifiers = ui.input(|i| {
                            i.events
                                .iter()
                                .rev()
                                .find_map(|event| match event {
                                    egui::Event::PointerButton {
                                        button: egui::PointerButton::Primary,
                                        pressed: false,
                                        modifiers,
                                        ..
                                    } => Some(*modifiers),
                                    _ => None,
                                })
                                .unwrap_or(i.modifiers)
                        });
                        if let Some(id) = ctx.memory(|m| m.focused()) {
                            ctx.memory_mut(|m| m.surrender_focus(id));
                        }
                        let p = self.world(pos);
                        let hit = if self.present {
                            self.editor
                                .layers(self.editor.surface_id())
                                .ok()
                                .and_then(|layers| {
                                    layers
                                        .into_iter()
                                        .rev()
                                        .find(|l| {
                                            l.control.bounds.contains(p.x, p.y)
                                                && l.clip.is_none_or(|b| b.contains(p.x, p.y))
                                        })
                                        .map(|l| l.control)
                                })
                        } else {
                            self.editor.hit(p.x, p.y)
                        };
                        let inline = self
                            .present
                            .then(|| {
                                self.editor
                                    .rendered_scene(self.editor.surface_id())
                                    .ok()
                                    .and_then(|s| s.hit_link(p.x, p.y).map(str::to_owned))
                            })
                            .flatten();
                        if let Some(target) = inline {
                            self.follow_target(&target, &ctx);
                        } else if let Some(c) = hit {
                            if self.present {
                                if let Some(id) = c.link {
                                    self.navigate_page(id);
                                }
                            } else if !self.editor.own_locked(c.id)
                                || self.editor.selection.contains(&c.id)
                                || modifiers.command
                            {
                                self.editor.select(c.id, modifiers.shift);
                            }
                        } else {
                            self.editor.selection.clear();
                        }
                    }
                    let exit_group = !self.present
                        && !self.editor.context.is_empty()
                        && double_click.is_some_and(|(pointer, _)| {
                            let point = self.world(pointer);
                            self.editor.hit(point.x, point.y).is_none()
                        });
                    if let Some((pointer, modifiers)) = double_click
                        && arrow_handle.is_none()
                        && !self.present
                        && let point = self.world(pointer)
                        && let Some(c) = self.editor.hit(point.x, point.y)
                        && self.editor.selection.contains(&c.id)
                        && !self.editor.enter(c.id)
                        && c.group.is_none()
                    {
                        let command = modifiers.command;
                        if matches!(c.kind, Kind::Image | Kind::CoverFlow) {
                            self.image_source_request = Some(c.id);
                            ctx.request_repaint();
                        } else if c.kind == Kind::Icon || (c.kind == Kind::IconLabel && command) {
                            self.begin_icon_dialog(c.id);
                        } else if c.kind.text_editable() {
                            self.begin_text_edit(c.id);
                        }
                    }
                    if exit_group {
                        self.editor.leave();
                        self.editor.selection.clear();
                    }
                }
                if self.grid && !self.present && self.zoom > 0.35 {
                    let step = 20. * self.zoom;
                    let start = self.screen([0., 0.]);
                    let min = response.rect.min;
                    let x0 = min.x + (start.x - min.x).rem_euclid(step);
                    let y0 = min.y + (start.y - min.y).rem_euclid(step);
                    for ix in 0..((response.rect.width() / step) as usize + 1) {
                        for iy in 0..((response.rect.height() / step) as usize + 1) {
                            painter.circle_filled(
                                Pos2::new(x0 + ix as f32 * step, y0 + iy as f32 * step),
                                0.75,
                                Color32::from_gray(224),
                            );
                        }
                    }
                }
                let drawing = if self.present {
                    self.editor.rendered_scene(self.editor.surface_id())
                } else {
                    self.text_edit_view
                        .as_ref()
                        .and_then(|view| view.preview.clone())
                        .map_or_else(|| self.editor.editing_scene(), Ok)
                };
                match drawing {
                    Ok(drawing) => Self::paint_scene(
                        &painter,
                        &drawing,
                        self.canvas_rect.min + self.pan,
                        self.zoom,
                    ),
                    Err(error) => {
                        painter.text(
                            response.rect.min + Vec2::splat(20.),
                            Align2::LEFT_TOP,
                            error,
                            FontId::proportional(14.),
                            Color32::DARK_RED,
                        );
                    }
                }
                self.paint_quick_draw(ui, &painter);
                if !self.present {
                    for guide in &self.alignment_guides {
                        let color = match guide.kind {
                            snapping::Kind::Edge => Color32::from_rgb(204, 204, 204),
                            snapping::Kind::Spacing => Color32::from_rgb(217, 211, 164),
                            snapping::Kind::Center => Color32::from_rgb(137, 201, 222),
                        };
                        let points = if guide.axis == snapping::Axis::X {
                            let x = self.screen([guide.position, 0.]).x;
                            [
                                Pos2::new(x, response.rect.top()),
                                Pos2::new(x, response.rect.bottom()),
                            ]
                        } else {
                            let y = self.screen([0., guide.position]).y;
                            [
                                Pos2::new(response.rect.left(), y),
                                Pos2::new(response.rect.right(), y),
                            ]
                        };
                        painter.line_segment(points, Stroke::new(1., color));
                    }
                    if !self.quick_draw.is_active() {
                        let arrow_selection = self.paint_arrow_selection(&painter);
                        for c in &self.editor.current().controls {
                            if self.editor.selection.contains(&c.id) && c.locked {
                                self.paint_locked_selection(&painter, c.bounds);
                            } else if self.editor.selection.contains(&c.id) && !arrow_selection {
                                painter.rect_stroke(
                                    self.screen_rect(c.bounds).expand(3.),
                                    1,
                                    Stroke::new(1., ACCENT),
                                    StrokeKind::Outside,
                                );
                            }
                        }
                        if let Some(b) = self.editor.movable_bounds()
                            && !arrow_selection
                        {
                            let r = self.screen_rect(b).expand(4.);
                            painter.rect_stroke(r, 0, Stroke::new(1., ACCENT), StrokeKind::Outside);
                            if self.editor.selected_views().iter().any(|c| !c.locked) {
                                for (_, point) in
                                    self.resize_handles(self.editor.movable_bounds().unwrap_or(b))
                                {
                                    painter.rect(
                                        Rect::from_center_size(point, Vec2::splat(6.)),
                                        0,
                                        Color32::WHITE,
                                        Stroke::new(1., Color32::from_gray(65)),
                                        StrokeKind::Inside,
                                    );
                                }
                            }
                        }
                        for handle in self.arrow_handles() {
                            painter.rect(
                                Rect::from_center_size(
                                    handle.position,
                                    Vec2::splat(
                                        if handle.kind == wireframe::arrows::Handle::Middle {
                                            7.
                                        } else {
                                            6.
                                        },
                                    ),
                                ),
                                1,
                                Color32::WHITE,
                                Stroke::new(1., Color32::from_gray(65)),
                                StrokeKind::Inside,
                            );
                        }
                        if let Some(Drag::Marquee { start, now, .. }) = &self.drag {
                            let r = Rect::from_two_pos(
                                self.screen([start.x, start.y]),
                                self.screen([now.x, now.y]),
                            );
                            painter.rect(
                                r,
                                0,
                                Color32::from_rgba_unmultiplied(49, 94, 145, 22),
                                Stroke::new(1., ACCENT),
                                StrokeKind::Inside,
                            );
                        }
                    }
                }
                if self.editor.nodes().is_empty() && !self.quick_draw.is_active() {
                    painter.text(
                        response.rect.center() - Vec2::new(0., 24.),
                        Align2::CENTER_CENTER,
                        "Start with a rough idea.",
                        FontId::proportional(26.),
                        Color32::from_gray(125),
                    );
                    painter.text(
                        response.rect.center() + Vec2::new(0., 16.),
                        Align2::CENTER_CENTER,
                        "Choose a control from the library. Make it yours.",
                        FontId::proportional(15.),
                        Color32::from_gray(140),
                    );
                }
                self.paint_review_markers(&painter);
                if !self.present {
                    response.context_menu(|ui| {
                        if ui.button("Duplicate").clicked() {
                            self.editor.duplicate();
                            ui.close();
                        }
                        let text_editable = self
                            .editor
                            .selected_views()
                            .first()
                            .is_some_and(|c| c.kind.text_editable());
                        if ui
                            .add_enabled(text_editable, egui::Button::new("Edit text"))
                            .clicked()
                        {
                            if let Some(c) = self.editor.selected_views().first() {
                                self.begin_text_edit(c.id);
                            }
                            ui.close();
                        }
                        if ui.button("Group").clicked() {
                            self.editor.group();
                            ui.close();
                        }
                        if ui.button("Ungroup").clicked() {
                            self.editor.ungroup();
                            ui.close();
                        }
                        self.menu_item(ui, menus::Command::TreatAsMarkup);
                        if ui.button("Delete").clicked() {
                            self.editor.delete();
                            ui.close();
                        }
                    });
                }
            });
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.startup_open.is_some() {
            self.startup_open_dialog(ctx);
            return;
        }
        self.text_session_ui(ctx);
        // Newly shown modals have no defined relative stacking order. Keep
        // the review pending while an error is presented on its own.
        let showing_save_review = self.pending.is_some() && self.error.is_none();
        if showing_save_review {
            let response = egui::Modal::new(egui::Id::new("unsaved")).show(ctx, |ui| {
                ui.heading("Save your changes?");
                ui.label("Save this project before continuing, or discard its local changes.");
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked()
                        && self.save(false)
                        && let Some(p) = self.pending.take()
                    {
                        self.perform(p, ctx);
                    }
                    if ui.button("Discard changes").clicked()
                        && let Some(p) = self.pending.take()
                    {
                        self.discard_then(p, ctx);
                    }
                    if self.persistence.is_some() && ui.button("Save as…").clicked() {
                        self.save(true);
                    }
                    if ui.button("Cancel").clicked() {
                        self.pending = None;
                    }
                });
            });
            // Escape is Cancel, never Discard. Only the visible modal owns it;
            // an error above this review must be dismissed independently.
            if response.is_top_modal
                && !response.any_popup_open
                && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
            {
                self.pending = None;
            }
        }
        if self.recovery_offer.is_some() {
            egui::Modal::new(egui::Id::new("recover")).show(ctx, |ui| {
                ui.heading("An unsaved project is available");
                ui.label("Restore the last local recovery copy as a new project.");
                ui.horizontal(|ui| {
                    if ui.button("Restore").clicked() {
                        self.restore_legacy();
                    }
                    if ui.button("Start fresh").clicked() {
                        self.recovery_offer = None;
                        self.clear_recovery();
                    }
                });
            });
        }
        self.persistence_dialogs(ctx);
        if showing_save_review && self.error.is_some() {
            // Save can fail synchronously inside the review. Present its
            // error next frame, when the review is no longer drawn.
            ctx.request_repaint();
        } else if let Some(error) = self.error.clone() {
            let response = egui::Modal::new(egui::Id::new("error")).show(ctx, |ui| {
                ui.heading("Couldn't complete that action");
                ui.label(error);
                if ui.button("OK").clicked() {
                    self.error = None;
                }
            });
            if response.is_top_modal
                && !response.any_popup_open
                && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
            {
                self.error = None;
            }
        }
        egui::Window::new("Wireframe license")
            .open(&mut self.about)
            .default_size([650., 500.])
            .show(ctx, |ui| {
                ui.label("Copyright 2026 Wireframe contributors. GNU GPL v3.0 only.");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(include_str!("../LICENSE"));
                });
            });
        egui::Window::new("Working with Wireframe").open(&mut self.help).default_width(470.).show(ctx,|ui|{ui.label("Copyright 2026 Wireframe contributors · GNU GPL v3.0 only · No warranty.");if ui.button("View license").clicked(){self.about=true;}ui.separator();ui.label("Everything stays on your computer. Save editable projects as .wireframe files; export pages as PDF or PNG images, or the current page as SVG.");ui.separator();for (keys,what)in [("Click / Shift-click","Select one or several controls"),("Drag empty canvas","Select an area"),("Drag a selection","Move controls"),("Eight resize handles","Resize; hold Shift for aspect ratio"),("Arrow / Shift+Arrow","Move by 1 / 10 units"),("Double-click / Enter","Edit selected control text"),("⌘/Ctrl Z · Shift Z","Undo · Redo"),("⌘/Ctrl D · G · Shift G","Duplicate · Group · Ungroup"),("Space + drag","Pan the canvas"),("⌘/Ctrl + scroll","Zoom around the pointer")]{ui.horizontal(|ui|{ui.strong(keys);ui.label(what);});}ui.separator();ui.label("Groups move and resize together. Double-click a group to edit its children; Escape or Exit group returns to its parent. Images are embedded in the project. Rotate or crop them from the inspector. Click page links in Present mode.");});
    }
}
impl eframe::App for WireframeApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // A minimized window may receive a logic-only pass before its next UI pass.
        self.capture_window_close(ctx);
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.deliver_menu_text_command(ctx, raw);
        self.order_text_input(ctx, raw);
        self.prepare_viewport_input(ctx, raw);
    }

    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        self.startup_open = None;
        if let (Some(compositor), Some(gl)) = (self.compositor.take(), gl) {
            compositor.destroy(gl);
        }
        if let Some(job) = self.pdf_job.take() {
            let _ = job.finish();
        }
        if let Some(job) = self.image_export.job.take() {
            job.cancel();
            let _ = job.finish();
        }
        self.remember_version_view();
        self.remember_viewports(true);
        self.remember_presentation();
        self.shutdown_persistence();
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.image_import_dialog(ui.ctx());
        let ctx = ui.ctx().clone();
        self.poll_startup_open();
        self.poll_recoveries();
        self.sync_reviews();
        self.prepare_page_notes(&ctx);
        self.poll_saves(&ctx);
        self.preserve_save_navigator_focus(&ctx);
        self.poll_pdf_export(&ctx);
        self.poll_image_export(&ctx);
        self.capture_window_close(&ctx);
        if !(self.startup_open.is_some()
            || self.save_busy()
            || self.present && self.presentation_view.settings_open)
            && !self.recovery_chooser_open()
            && self.pdf_options.is_none()
            && !self.image_export.blocks_input()
            && self.versions.rename.is_none()
            && self.page_rename.is_none()
            && self.trash.is_none()
            && self.pending.is_none()
            && self.text_edit.is_none()
            && self.icon_dialog.is_none()
            && self.recovery_offer.is_none()
            && self.image_crop.is_none()
            && let Some(path) =
                ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()))
        {
            self.request(Pending::Open(path), &ctx);
        }
        #[cfg(debug_assertions)]
        if let Ok(path) = std::env::var("WIREFRAME_INPUT_TRACE") {
            let pointer_events = ctx.input(|i| {
                i.events
                    .iter()
                    .filter(|e| {
                        matches!(
                            e,
                            egui::Event::PointerMoved(_) | egui::Event::PointerButton { .. }
                        )
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            });
            if !pointer_events.is_empty() {
                use std::io::Write;
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                {
                    let _ = writeln!(file, "{pointer_events:?}");
                }
            }
        }
        if self.startup_open.is_some()
            || self.save_busy()
            || self.recovery_chooser_open()
            || self.trash.is_some()
        {
            ui.disable();
        }
        self.sync_viewport();
        self.shortcuts(&ctx);
        self.text_session_ui(&ctx);
        self.sync_viewport();
        let cropping = self.image_crop.is_some();
        self.update_menus(&ctx);
        self.toolbar(ui);
        self.font_dialog(&ctx);
        if self.installed_font != self.editor.document.font {
            wireframe::fonts::install_project(&ctx, self.editor.document.font.as_deref());
            self.installed_font = self.editor.document.font.clone();
            ctx.request_discard("Project font changed");
            return;
        }
        if !self.present {
            if self.show_library {
                self.library(ui);
            }
            if self.show_navigator {
                self.sidebar(ui);
            }
            if cropping {
                self.crop_controls(ui);
            } else if self.review.open && !self.editor.in_source_workspace() {
                self.review_panel(ui);
            } else if self.show_inspector {
                self.inspector(ui);
            }
        }
        self.canvas(ui);
        self.library_drag_overlay(ui.ctx());
        self.presentation_dialog(&ctx);
        if let Some(error) = self.editor.error.take() {
            self.error = Some(error);
        }
        self.pdf_export_dialog(&ctx);
        self.image_export_dialogs(&ctx);
        self.version_dialog(&ctx);
        self.page_rename_dialog(&ctx);
        self.trash_dialog(&ctx);
        self.library_import_dialog(&ctx);
        self.more_controls_dialog(&ctx);
        self.page_paste_dialog(&ctx);
        self.icon_dialog_ui(&ctx);
        self.dialogs(&ctx);
        self.finish_window_close(&ctx);
        self.settle_native_close();
        self.observe_project();
        self.remember_version_view();
        self.remember_viewports(false);
        self.remember_presentation();
        self.sync_recovery(false);
        self.update_menus(&ctx);
        ctx.request_repaint_after(Duration::from_secs(2));
    }
}
fn example_document() -> Document {
    let mut doc = Document {
        title: "A simple sign-in flow".into(),
        ..Document::default()
    };
    doc.pages[0].name = "Sign in".into();
    doc.pages[0].notes =
        "A starting point. Select any control to change it, or create a new project from File."
            .into();
    for (kind, x, y, w, h, text) in [
        (
            Kind::Browser,
            20.,
            20.,
            760.,
            570.,
            "https://your-product.local",
        ),
        (Kind::Heading, 90., 110., 580., 50., "Welcome back"),
        (
            Kind::Paragraph,
            90.,
            164.,
            510.,
            55.,
            "A few rough shapes. A clearer conversation.",
        ),
        (Kind::Label, 90., 240., 130., 28., "Email address"),
        (Kind::TextInput, 90., 275., 360., 42., "you@example.com"),
        (Kind::Label, 90., 337., 130., 28., "Password"),
        (Kind::TextInput, 90., 372., 360., 42., "••••••••"),
        (Kind::Checkbox, 90., 440., 230., 28., "Remember me"),
        (Kind::Button, 90., 500., 180., 42., "Sign in"),
        (Kind::Link, 300., 508., 180., 28., "Forgot password?"),
    ] {
        let id = doc.allocate_id();
        let mut c = Control::new(id, kind, x, y);
        c.bounds.w = w;
        c.bounds.h = h;
        c.text = text.into();
        doc.pages[0].controls.push(c);
    }
    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_fill_survives_canvas_tessellation_at_small_sizes_and_zoom() {
        use wireframe::document::{NodeContent, Point, Project, Size, Value};
        let color = Color32::from_rgb(255, 220, 168);
        for (w, h) in [(8., 8.), (20., 20.), (70., 40.), (180., 40.), (70., 120.)] {
            let mut project = Project::default();
            let id = project.allocate_id().unwrap();
            let mut node = wireframe::catalog::get("rectangle")
                .unwrap()
                .create(id, Point::new(20., 20.));
            node.content = NodeContent::Control {
                kind: "rectangle".into(),
                size: Size::fixed(w, h),
            };
            node.properties
                .insert("border".into(), Value::Choice("rounded".into()));
            node.properties
                .insert("fill".into(), Value::Color([255, 220, 168, 255]));
            project.pages[0].content.nodes.push(node);
            let editor = wireframe::workspace::Editor::new(project);
            let scene = editor.rendered_scene(editor.page).unwrap();
            for scale in [0.72, 1., 1.5] {
                let ctx = egui::Context::default();
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(500., 400.))),
                        ..Default::default()
                    },
                    |ui| WireframeApp::paint_scene(ui.painter(), &scene, Pos2::ZERO, scale),
                );
                let jobs =
                    ctx.tessellate(std::mem::take(&mut output.shapes), output.pixels_per_point);
                output.textures_delta.clear();
                for fx in [0.35, 0.5, 0.65] {
                    for fy in [0.35, 0.5, 0.65] {
                        let p = Pos2::new((20. + w * fx) * scale, (20. + h * fy) * scale);
                        let covered = jobs.iter().any(|job| {
                            let egui::epaint::Primitive::Mesh(mesh) = &job.primitive else {
                                return false;
                            };
                            mesh.indices.as_chunks::<3>().0.iter().any(|tri| {
                                let vertices = tri.map(|i| mesh.vertices[i as usize]);
                                if vertices.iter().any(|v| v.color != color) {
                                    return false;
                                }
                                let [a, b, c] = vertices.map(|v| v.pos);
                                let cross = |a: Pos2, b: Pos2, c: Pos2| {
                                    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
                                };
                                let area = cross(a, b, c);
                                if area.abs() < 0.00001 {
                                    return false;
                                }
                                let sign = area.signum();
                                [cross(a, b, p), cross(b, c, p), cross(c, a, p)]
                                    .iter()
                                    .all(|v| *v * sign >= -0.0001)
                            })
                        });
                        assert!(
                            covered,
                            "Rounded {w} x {h} at scale {scale} lost its fill at {fx}, {fy}"
                        );
                    }
                }
            }
        }
    }

    fn frame(ctx: &egui::Context, app: &mut WireframeApp, events: Vec<egui::Event>) {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
                events,
                ..Default::default()
            },
            |ui| app.canvas(ui),
        );
        output.textures_delta.clear();
    }
    #[test]
    fn text_undo_stays_local_but_button_focus_allows_document_undo() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Rectangle, 50., 50.);
        let mut value = "hello".to_owned();
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::COMMAND,
        };
        for (events, button) in [
            (vec![], false),
            (vec![egui::Event::Text("X".into())], false),
            (vec![key(Key::Z)], false),
            (vec![], true),
            (vec![key(Key::Z)], true),
        ] {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    app.shortcuts(ui.ctx());
                    if button {
                        ui.button("Action").request_focus();
                    } else {
                        ui.add(TextInput::singleline(&mut value).autofocus(true));
                    }
                },
            );
            output.textures_delta.clear();
            if !button {
                assert_eq!(app.editor.current().controls.len(), 1);
            }
        }
        assert_eq!(value, "hello");
        assert_eq!(app.editor.current().controls.len(), 0);
    }

    #[test]
    fn quick_add_keeps_arrow_selection_and_enter_submission() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.quick_focus = true;
        app.quick = "button".into();
        let event = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        for events in [vec![], vec![event(Key::ArrowDown)], vec![event(Key::Enter)]] {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| app.quick_add_ui(ui),
            );
            output.textures_delta.clear();
        }
        assert_eq!(app.editor.current().controls.len(), 1);
        assert!(app.quick.is_empty());
        assert_eq!(app.editor.current().controls[0].kind, Kind::ButtonBar);
    }

    #[test]
    fn long_project_titles_leave_toolbar_actions_inside_the_window() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.document.title = "ABC שלום 👩‍💻 ".repeat(100);
        for width in [860., 1000., 1600.] {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 700.))),
                    ..Default::default()
                },
                |ui| app.toolbar(ui),
            );
            let nodes = &output
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes;
            for label in [
                "Help",
                "File",
                "Edit",
                "View",
                "Present",
                "Zoom out",
                "Zoom in",
                "Actual size",
                "Fit page",
                "Pages",
                "Library",
                "Inspector",
                "New wireframe",
            ] {
                let node = &nodes
                    .iter()
                    .find(|(_, n)| n.label() == Some(label))
                    .unwrap()
                    .1;
                let b = node.bounds().unwrap();
                assert!(
                    b.x0 >= 0. && b.x1 <= width as f64,
                    "{label} {b:?} width{width}"
                );
            }
            output.textures_delta.clear();
        }
    }

    fn pointer(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }
    #[test]
    fn presentation_follows_inline_page_link_at_zoomed_glyph_position() {
        use wireframe::document::{Content, NodeContent, Page, Point, Project, Size, Value};
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        let mut p = Project {
            next_id: 4,
            ..Project::default()
        };
        let mut node = wireframe::catalog::get("paragraph")
            .unwrap()
            .create(2, Point::new(50., 50.));
        node.content = NodeContent::Control {
            kind: "paragraph".into(),
            size: Size::fixed(250., 80.),
        };
        node.properties
            .insert("text".into(), Value::Text("é [Continue](page:3)".into()));
        p.pages[0].content.nodes.push(node);
        p.pages.push(Page {
            trashed: false,
            id: 3,
            name: "Destination".into(),
            notes: String::new(),
            parent: None,
            content: Content::default(),
            alternates: vec![],
        });
        app.editor = Editor::new(p);
        app.present = true;
        app.zoom = 1.5;
        frame(&ctx, &mut app, vec![]);
        let b = app.editor.rendered_scene(1).unwrap().links[0].bounds;
        let position = app.screen([b.x + b.w / 2., b.y + b.h / 2.]);
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(position), pointer(position, true)],
        );
        frame(&ctx, &mut app, vec![pointer(position, false)]);
        assert_eq!(app.editor.page, 3);
        assert!(!app.editor.dirty());
    }
    #[test]
    fn presentation_row_links_follow_background_and_back_history() {
        use wireframe::document::{Link, Value};
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        let first = app.editor.page;
        app.editor.add_page();
        let second = app.editor.page;
        app.editor.add(Kind::List, 50., 50.);
        let back = *app.editor.selection.iter().next().unwrap();
        app.editor.begin("Back row");
        app.editor
            .set_control_property(back, "row_links", Value::Links(vec![Some(Link::Back)]))
            .unwrap();
        app.editor.commit();
        app.editor.switch_page(first);
        app.editor.add(Kind::List, 50., 50.);
        let forward = *app.editor.selection.iter().next().unwrap();
        app.editor.begin("Forward row");
        app.editor
            .set_control_property(
                forward,
                "row_links",
                Value::Links(vec![Some(Link::Page(second))]),
            )
            .unwrap();
        app.editor.commit();
        let saved = app.editor.document.clone();
        app.present = true;
        app.zoom = 1.5;
        frame(&ctx, &mut app, vec![]);
        for expected in [second, first] {
            let b = app.editor.rendered_scene(app.editor.page).unwrap().links[0].bounds;
            let pos = app.screen([b.x + b.w - 3., b.y + b.h / 2.]);
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(pos), pointer(pos, true)],
            );
            frame(&ctx, &mut app, vec![pointer(pos, false)]);
            assert_eq!(app.editor.page, expected);
        }
        assert_eq!(app.editor.document, saved);
        assert!(app.history_destination(false).is_none());
        app.follow_target("back:", &ctx);
        assert_eq!(app.editor.page, first);
        app.follow_target("page:999999", &ctx);
        assert_eq!(app.editor.page, first);
    }
    #[test]
    fn presentation_group_link_owns_gap_and_child_and_preserves_back_history() {
        use wireframe::document::{Link, Node, Point, Size, Value};
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        let first = app.editor.page;
        app.editor.add_page();
        let contact = app.editor.page;
        app.editor.add_page();
        let order = app.editor.page;
        app.editor.switch_page(first);
        let a = Node::control(100, "button", Point::default(), Size::fixed(100., 40.));
        let mut b = Node::control(101, "button", Point::new(100., 80.), Size::fixed(100., 40.));
        b.properties
            .insert("link".into(), Value::Link(Link::Page(order)));
        let mut group = Node::group(102, Point::new(100., 100.), vec![a, b]);
        group
            .properties
            .insert("link".into(), Value::Link(Link::Page(contact)));
        app.editor.document.next_id = 103;
        app.editor.document.pages[0].content.nodes = vec![group];
        let saved = app.editor.document.clone();
        app.present = true;
        app.zoom = 1.5;
        for point in [[150., 160.], [250., 200.]] {
            frame(&ctx, &mut app, vec![]);
            let pos = app.screen(point);
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(pos), pointer(pos, true)],
            );
            frame(&ctx, &mut app, vec![pointer(pos, false)]);
            assert_eq!(app.editor.page, contact);
            app.follow_target("back:", &ctx);
            assert_eq!(app.editor.page, first);
            assert!(app.history_destination(false).is_none());
        }
        assert_eq!(app.editor.document, saved);
    }
    #[test]
    fn actual_canvas_drag_resize_and_undo() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Button, 50., 50.);
        frame(&ctx, &mut app, vec![]);
        let before = app.editor.current().controls[0].bounds;
        let start = app.screen([80., 65.]);
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
        );
        let end = start + Vec2::new(80., 40.);
        frame(&ctx, &mut app, vec![egui::Event::PointerMoved(end)]);
        frame(&ctx, &mut app, vec![pointer(end, false)]);
        let moved = app.editor.current().controls[0].bounds;
        assert_eq!(moved.x, before.x + 80.);
        assert_eq!(moved.y, before.y + 40.);
        app.editor.undo();
        assert_eq!(app.editor.current().controls[0].bounds, before);
        frame(&ctx, &mut app, vec![]);
        let start = app.screen_rect(before).right_bottom() + Vec2::splat(4.);
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
        );
        let end = start + Vec2::new(40., 20.);
        frame(&ctx, &mut app, vec![egui::Event::PointerMoved(end)]);
        frame(&ctx, &mut app, vec![pointer(end, false)]);
        let resized = app.editor.current().controls[0].bounds;
        assert_eq!(resized.w, before.w + 40.);
        assert_eq!(resized.h, before.h + 20.);
        app.editor.undo();
        assert_eq!(app.editor.current().controls[0].bounds, before);
    }

    #[test]
    fn modified_click_uses_click_time_modifiers_after_key_release() {
        for release_modifier_in_frame in [false, true] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(Kind::TextInput, 50., 50.);
            let locked = *app.editor.selection.iter().next().unwrap();
            app.editor.lock(true);
            app.editor.selection.clear();
            let before = app.editor.document.clone();
            frame(&ctx, &mut app, vec![]);
            let pos = app.screen([70., 60.]);
            let event = |pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Modifiers::COMMAND,
            };
            frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::ModifiersChanged(Modifiers::COMMAND),
                    egui::Event::PointerMoved(pos),
                    event(true),
                ],
            );
            let mut events = vec![event(false)];
            if release_modifier_in_frame {
                events.push(egui::Event::ModifiersChanged(Modifiers::NONE));
            }
            frame(&ctx, &mut app, events);
            assert!(
                app.editor.selection.contains(&locked),
                "modifier released in click frame: {release_modifier_in_frame}"
            );
            assert_eq!(app.editor.document, before);
            assert!(app.text_edit.is_none());
        }
    }

    #[test]
    fn double_click_edits_only_the_explicitly_selected_locked_target() {
        for explicit in [false, true] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(Kind::TextInput, 50., 50.);
            let locked = *app.editor.selection.iter().next().unwrap();
            app.editor.lock(true);
            app.editor.add(Kind::Button, 50., 150.);
            let other = *app.editor.selection.iter().next().unwrap();
            if explicit {
                app.editor.select(locked, false);
            }
            let before = app.editor.document.clone();
            frame(&ctx, &mut app, vec![]);
            let pos = app.screen([70., 60.]);
            for _ in 0..2 {
                frame(
                    &ctx,
                    &mut app,
                    vec![egui::Event::PointerMoved(pos), pointer(pos, true)],
                );
                frame(&ctx, &mut app, vec![pointer(pos, false)]);
            }
            assert_eq!(
                app.text_edit.as_ref().map(|(id, _)| *id),
                explicit.then_some(locked)
            );
            assert!(
                app.editor
                    .selection
                    .contains(&if explicit { locked } else { other })
            );
            assert_eq!(app.editor.document, before);
        }
    }
    #[test]
    fn cropped_group_pointer_minimum_includes_fixed_height_descendants() {
        use wireframe::document::{
            Content, Crop, Definition, Node, NodeContent, Point, Project, Size,
        };
        for instance in [false, true] {
            for top in [false, true] {
                let ctx = egui::Context::default();
                let mut app = WireframeApp::workspace(&ctx, None);
                let mut label =
                    Node::control(4, "label", Point::new(0., 130.), Size::fixed(100., 20.));
                label.locked = true;
                let mut inner = Node::group(
                    3,
                    Point::default(),
                    vec![
                        Node::control(5, "rectangle", Point::default(), Size::fixed(200., 200.)),
                        label,
                    ],
                );
                inner.crop = Some(Crop {
                    x: 0.25,
                    y: 0.25,
                    width: 0.5,
                    height: 0.5,
                });
                let mut root = Node::group(6, Point::new(100., 100.), vec![inner]);
                let mut project = Project {
                    next_id: 100,
                    ..Project::default()
                };
                if instance {
                    root.origin = Point::default();
                    project.definitions.push(Definition {
                        id: 20,
                        name: "Panel".into(),
                        content: Content {
                            nodes: vec![root],
                            paint_order: vec![],
                        },
                    });
                    root = Node::group(21, Point::new(100., 100.), vec![]);
                    root.content = NodeContent::Instance {
                        definition: 20,
                        overrides: vec![],
                    };
                }
                let id = root.id;
                project.pages[0].content.nodes = vec![root];
                app.editor = {
                    project.organize_components().unwrap();
                    Editor::new(project)
                };
                app.editor.select(id, false);
                frame(&ctx, &mut app, vec![]);
                let before = app.editor.document.clone();
                let bounds = app.editor.current().controls[0].bounds;
                let handle = (0, if top { -1 } else { 1 });
                let start = app
                    .resize_handles(bounds)
                    .into_iter()
                    .find(|(h, _)| *h == handle)
                    .unwrap()
                    .1;
                let end = start + Vec2::new(0., if top { 90. } else { -90. });
                alignment_frame(
                    &ctx,
                    &mut app,
                    vec![egui::Event::PointerMoved(start), pointer(start, true)],
                    Modifiers::COMMAND,
                    true,
                );
                alignment_frame(
                    &ctx,
                    &mut app,
                    vec![egui::Event::PointerMoved(end)],
                    Modifiers::COMMAND,
                    true,
                );
                alignment_frame(
                    &ctx,
                    &mut app,
                    vec![pointer(end, false)],
                    Modifiers::COMMAND,
                    true,
                );
                let after = app.editor.current().controls[0].bounds;
                assert_eq!(after.h, 20., "instance={instance}, top={top}");
                assert_eq!(after.w, bounds.w);
                assert_eq!(
                    after.y,
                    if top {
                        bounds.y + bounds.h - 20.
                    } else {
                        bounds.y
                    }
                );
                assert_eq!(app.editor.document.definitions, before.definitions);
                app.editor.undo();
                assert_eq!(app.editor.document, before);
            }
        }
    }

    #[test]
    fn horizontal_rule_handles_follow_style_and_keep_height_during_pointer_resize() {
        for style in [
            "solid",
            "dashed",
            "dotted",
            "breakline_top",
            "breakline_bottom",
        ] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(Kind::Line, 200., 200.);
            let id = *app.editor.selection.iter().next().unwrap();
            app.editor
                .set_control_property(
                    id,
                    "stroke",
                    wireframe::document::Value::Choice(style.into()),
                )
                .unwrap();
            frame(&ctx, &mut app, vec![]);
            let before = app.editor.document.clone();
            let bounds = app.editor.selected_views()[0].bounds;
            let handles = app.resize_handles(bounds);
            assert_eq!(handles.len(), 2);
            assert!(handles.iter().all(|((_, y), _)| *y == 0));
            let start = handles[1].1;
            let end = start + Vec2::new(70., 40.);
            alignment_frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(start), pointer(start, true)],
                Modifiers::NONE,
                true,
            );
            alignment_frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(end)],
                Modifiers::SHIFT,
                true,
            );
            alignment_frame(
                &ctx,
                &mut app,
                vec![pointer(end, false)],
                Modifiers::SHIFT,
                true,
            );
            assert_eq!(
                app.editor.selected_views()[0].bounds,
                Bounds::new(200., 200., 170., bounds.h)
            );
            app.editor.undo();
            assert_eq!(app.editor.document, before);
        }
    }

    #[test]
    fn linear_resize_handles_preserve_thickness_minimum_anchor_and_history() {
        for kind in [
            Kind::HorizontalSlider,
            Kind::VerticalSlider,
            Kind::FormattingToolbar,
            Kind::VerticalRule,
            Kind::Line,
            Kind::HorizontalScrollbar,
            Kind::VerticalScrollbar,
            Kind::HorizontalSplitter,
            Kind::VerticalSplitter,
            Kind::VolumeSlider,
        ] {
            for end_index in [0, 1] {
                for shrink in [false, true] {
                    let ctx = egui::Context::default();
                    let mut app = WireframeApp::workspace(&ctx, None);
                    app.editor.add(kind, 200., 200.);
                    frame(&ctx, &mut app, vec![]);
                    let before = app.editor.current().controls[0].bounds;
                    let axis = wireframe::linear_controls::Axis::for_kind(kind).unwrap();
                    let handles = app.resize_handles(before);
                    assert_eq!(handles.len(), 2);
                    assert!(handles.iter().all(|((x, y), _)| if axis.vertical {
                        *x == 0
                    } else {
                        *y == 0
                    }));
                    let start = handles[end_index].1;
                    let old = if axis.vertical { before.h } else { before.w };
                    let amount = if shrink { 5. - old } else { 70. };
                    let direction = if end_index == 0 { -1. } else { 1. };
                    let end = start
                        + if axis.vertical {
                            Vec2::new(40., amount * direction)
                        } else {
                            Vec2::new(amount * direction, 40.)
                        };
                    alignment_frame(
                        &ctx,
                        &mut app,
                        vec![egui::Event::PointerMoved(start), pointer(start, true)],
                        Modifiers::NONE,
                        true,
                    );
                    alignment_frame(
                        &ctx,
                        &mut app,
                        vec![egui::Event::PointerMoved(end)],
                        Modifiers::SHIFT,
                        true,
                    );
                    alignment_frame(
                        &ctx,
                        &mut app,
                        vec![pointer(end, false)],
                        Modifiers::SHIFT,
                        true,
                    );
                    let after = app.editor.current().controls[0].bounds;
                    let length = if shrink { 10. } else { old + 70. };
                    assert_eq!(
                        (after.w, after.h),
                        if axis.vertical {
                            (axis.thickness, length)
                        } else {
                            (length, axis.thickness)
                        }
                    );
                    if end_index == 0 {
                        assert_eq!(
                            if axis.vertical {
                                after.y + after.h
                            } else {
                                after.x + after.w
                            },
                            if axis.vertical {
                                before.y + before.h
                            } else {
                                before.x + before.w
                            }
                        );
                    } else {
                        assert_eq!((after.x, after.y), (before.x, before.y));
                    }
                    app.editor.undo();
                    assert_eq!(app.editor.current().controls[0].bounds, before);
                    app.editor.redo();
                    assert_eq!(app.editor.current().controls[0].bounds, after);
                }
            }
        }
    }
    #[test]
    fn nested_resize_preview_is_independent_of_pointer_frames() {
        use wireframe::document::{Content, Definition, Node, NodeContent, Point, Project, Size};
        for instance in [false, true] {
            let mut outcomes = vec![];
            for steps in [vec![13.], vec![6., 9., 13.], vec![20., -10., 13.]] {
                let ctx = egui::Context::default();
                let mut app = WireframeApp::workspace(&ctx, None);
                let leaf = |id, x, y, w, h| {
                    Node::control(id, "rectangle", Point::new(x, y), Size::fixed(w, h))
                };
                let mut group = Node::group(
                    2,
                    Point::new(100., 100.),
                    vec![
                        leaf(3, 0., 0., 40., 30.),
                        Node::group(
                            4,
                            Point::new(75., 20.),
                            vec![leaf(5, 0., 0., 35., 25.), leaf(6, 80., 45., 45., 25.)],
                        ),
                    ],
                );
                let mut project = Project {
                    next_id: 30,
                    ..Project::default()
                };
                let selected = if instance {
                    group.origin = Point::default();
                    project.definitions.push(Definition {
                        id: 20,
                        name: "Panel".into(),
                        content: Content {
                            nodes: vec![group],
                            paint_order: vec![],
                        },
                    });
                    let mut node = Node::group(21, Point::new(100., 100.), vec![]);
                    node.content = NodeContent::Instance {
                        definition: 20,
                        overrides: vec![],
                    };
                    project.pages[0].content.nodes.push(node);
                    21
                } else {
                    project.pages[0].content.nodes.push(group);
                    2
                };
                app.editor = {
                    project.organize_components().unwrap();
                    Editor::new(project)
                };
                app.editor.select(selected, false);
                frame(&ctx, &mut app, vec![]);
                let before = app.editor.document.clone();
                let bounds = app.editor.selected_views()[0].bounds;
                let start = app
                    .resize_handles(bounds)
                    .into_iter()
                    .find(|h| h.0 == (1, 1))
                    .unwrap()
                    .1;
                alignment_frame(
                    &ctx,
                    &mut app,
                    vec![egui::Event::PointerMoved(start), pointer(start, true)],
                    Modifiers::COMMAND,
                    true,
                );
                for step in steps {
                    alignment_frame(
                        &ctx,
                        &mut app,
                        vec![egui::Event::PointerMoved(start + Vec2::new(step, 0.))],
                        Modifiers::COMMAND,
                        true,
                    );
                }
                assert!(matches!(app.drag, Some(Drag::Resize { .. })));
                let after = app.editor.document.clone();
                alignment_frame(
                    &ctx,
                    &mut app,
                    vec![pointer(start + Vec2::new(13., 0.), false)],
                    Modifiers::COMMAND,
                    true,
                );
                assert_eq!(app.editor.document, after);
                assert_eq!(app.editor.undo_name(), Some("Resize"));
                app.editor.undo();
                assert_eq!(app.editor.document, before);
                assert_eq!(app.editor.undo_name(), None);
                app.editor.redo();
                assert_eq!(app.editor.document, after);
                outcomes.push(after);
                app.editor.undo();
                for cancel in [false, true] {
                    alignment_frame(
                        &ctx,
                        &mut app,
                        vec![egui::Event::PointerMoved(start), pointer(start, true)],
                        Modifiers::COMMAND,
                        true,
                    );
                    alignment_frame(
                        &ctx,
                        &mut app,
                        vec![egui::Event::PointerMoved(start + Vec2::new(20., 0.))],
                        Modifiers::COMMAND,
                        true,
                    );
                    assert_ne!(app.editor.document, before);
                    if cancel {
                        alignment_frame(&ctx, &mut app, vec![], Modifiers::COMMAND, false);
                    } else {
                        alignment_frame(
                            &ctx,
                            &mut app,
                            vec![egui::Event::PointerMoved(start), pointer(start, false)],
                            Modifiers::COMMAND,
                            true,
                        );
                    }
                    assert_eq!(app.editor.document, before);
                    assert_eq!(app.editor.undo_name(), None);
                    assert!(app.drag.is_none());
                }
            }
            assert_eq!(
                outcomes[0], outcomes[1],
                "intermediate frames changed final geometry"
            );
            assert_eq!(
                outcomes[0], outcomes[2],
                "pointer detour changed final geometry"
            );
        }
    }
    #[test]
    fn zero_crop_selection_has_no_resize_handles_and_preserves_content() {
        use wireframe::document::{Crop, Node, Point, Size};
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        let mut group = Node::group(
            2,
            Point::new(100., 100.),
            vec![Node::control(
                3,
                "rectangle",
                Point::default(),
                Size::fixed(100., 100.),
            )],
        );
        group.crop = Some(Crop {
            x: 0.25,
            y: 0.25,
            width: 0.001,
            height: 0.001,
        });
        app.editor.document.pages[0].content.nodes = vec![group];
        app.editor.document.next_id = 4;
        app.editor.select(2, false);
        let saved = app.editor.document.clone();
        frame(&ctx, &mut app, vec![]);
        assert!(
            app.resize_handles(app.editor.selected_views()[0].bounds)
                .is_empty()
        );
        assert_eq!(app.editor.document, saved);
        app.editor.crop_group(2, None, Point::default()).unwrap();
        assert!(
            !app.resize_handles(app.editor.selected_views()[0].bounds)
                .is_empty()
        );
        app.editor.undo();
        assert_eq!(app.editor.document, saved);
    }
    #[test]
    fn selecting_subminimum_controls_does_not_rewrite_saved_sizes() {
        for kind in [
            Kind::Rectangle,
            Kind::Arrow,
            Kind::Button,
            Kind::FormattingToolbar,
            Kind::VerticalRule,
            Kind::StreetMap,
            Kind::ModalOverlay,
            Kind::HorizontalSlider,
            Kind::VerticalSlider,
            Kind::HorizontalScrollbar,
            Kind::VerticalScrollbar,
            Kind::HorizontalSplitter,
            Kind::VerticalSplitter,
            Kind::VolumeSlider,
            Kind::Webcam,
            Kind::Video,
            Kind::CoverFlow,
            Kind::ColumnChart,
            Kind::BarChart,
            Kind::LineChart,
            Kind::PieChart,
            Kind::HorizontalBrace,
            Kind::VerticalBrace,
            Kind::RedCross,
            Kind::ScratchOut,
            Kind::Heading,
            Kind::Subtitle,
            Kind::TextBlock,
            Kind::TextLine,
            Kind::TagCloud,
            Kind::Callout,
            Kind::StickyNote,
            Kind::Tooltip,
        ] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(kind, 100., 100.);
            // Model a previously saved file, without invoking the resize command.
            let wireframe::document::NodeContent::Control { size, .. } =
                &mut app.editor.document.pages[0].content.nodes[0].content
            else {
                panic!("Expected a control");
            };
            *size = wireframe::document::Size::fixed(1., 1.);
            let saved = app.editor.document.encode().unwrap();
            frame(&ctx, &mut app, vec![]);
            assert_eq!(app.editor.document.encode().unwrap(), saved);
        }
    }
    #[test]
    fn webcam_pointer_resize_clamps_both_axes_and_playback_has_no_handles() {
        for (kind, corner) in [
            Kind::Rectangle,
            Kind::StreetMap,
            Kind::ModalOverlay,
            Kind::Webcam,
            Kind::Video,
            Kind::CoverFlow,
            Kind::ColumnChart,
            Kind::BarChart,
            Kind::LineChart,
            Kind::PieChart,
            Kind::HorizontalBrace,
            Kind::VerticalBrace,
            Kind::RedCross,
            Kind::ScratchOut,
            Kind::TextBlock,
            Kind::TextLine,
            Kind::TagCloud,
            Kind::Callout,
            Kind::StickyNote,
            Kind::Tooltip,
        ]
        .into_iter()
        .flat_map(|k| [(-1, -1), (1, 1)].map(|c| (k, c)))
        {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(kind, 200., 200.);
            frame(&ctx, &mut app, vec![]);
            let before = app.editor.current().controls[0].bounds;
            let start = app
                .resize_handles(before)
                .into_iter()
                .find(|h| h.0 == corner)
                .unwrap()
                .1;
            let end = start
                + Vec2::new(
                    (5. - before.w) * corner.0 as f32,
                    (5. - before.h) * corner.1 as f32,
                );
            alignment_frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(start), pointer(start, true)],
                Modifiers::NONE,
                true,
            );
            alignment_frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(end)],
                Modifiers::NONE,
                true,
            );
            alignment_frame(
                &ctx,
                &mut app,
                vec![pointer(end, false)],
                Modifiers::NONE,
                true,
            );
            let after = app.editor.current().controls[0].bounds;
            assert_eq!((after.w, after.h), (10., 10.));
            if corner.0 < 0 {
                assert_eq!(after.x + after.w, before.x + before.w);
                assert_eq!(after.y + after.h, before.y + before.h);
            }
            app.editor.undo();
            assert_eq!(app.editor.current().controls[0].bounds, before);
            app.editor.redo();
            assert_eq!(app.editor.current().controls[0].bounds, after);
            app.editor.add(Kind::Playback, 400., 200.);
            let b = app.editor.selected_views()[0].bounds;
            assert!(app.resize_handles(b).is_empty());
        }
    }
    #[test]
    fn titles_only_offer_horizontal_resize_handles() {
        for kind in [
            Kind::FormattingToolbar,
            Kind::Heading,
            Kind::Subtitle,
            Kind::NumericStepper,
        ] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(kind, 100., 100.);
            frame(&ctx, &mut app, vec![]);
            let b = app.editor.selected_views()[0].bounds;
            let handles = app.resize_handles(b);
            assert_eq!(handles.len(), 2);
            assert!(handles.iter().all(|((_, y), _)| *y == 0));
        }
    }
    #[test]
    fn compact_form_handles_and_keyboard_obey_sizing_contract() {
        for kind in [Kind::ColorPicker, Kind::HelpButton] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(kind, 200., 200.);
            frame(&ctx, &mut app, vec![]);
            let b = app.editor.selected_views()[0].bounds;
            assert!(app.resize_handles(b).is_empty());
            alignment_frame(
                &ctx,
                &mut app,
                vec![egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
                Modifiers::NONE,
                true,
            );
            assert!(app.text_edit.is_none());
            let id = *app.editor.selection.iter().next().unwrap();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                assert!(app.typed_properties(ui, id))
            });
            output.textures_delta.clear();
        }
        for side in [-1, 1] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(Kind::NumericStepper, 200., 200.);
            frame(&ctx, &mut app, vec![]);
            let before = app.editor.selected_views()[0].bounds;
            let start = app
                .resize_handles(before)
                .into_iter()
                .find(|h| h.0 == (side, 0))
                .unwrap()
                .1;
            let end = start + Vec2::new((5. - before.w) * side as f32, 40.);
            alignment_frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(start), pointer(start, true)],
                Modifiers::NONE,
                true,
            );
            alignment_frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(end)],
                Modifiers::SHIFT,
                true,
            );
            alignment_frame(
                &ctx,
                &mut app,
                vec![pointer(end, false)],
                Modifiers::SHIFT,
                true,
            );
            let after = app.editor.selected_views()[0].bounds;
            assert_eq!(after.w, 10.);
            assert_eq!(after.h, before.h);
            assert_eq!(after.y, before.y);
            app.editor.undo();
            assert_eq!(app.editor.selected_views()[0].bounds, before);
        }
    }
    #[test]
    fn placeholder_enter_conversion_survives_escape_and_undo_restores_it() {
        for (kind, target) in [
            (Kind::TextBlock, Kind::Paragraph),
            (Kind::TextLine, Kind::Subtitle),
        ] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(kind, 100., 100.);
            let before = app.editor.document.clone();
            let event = |key| egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            };
            ctx.run_ui(
                egui::RawInput {
                    events: vec![event(Key::Enter)],
                    ..Default::default()
                },
                |ui| app.shortcuts(ui.ctx()),
            )
            .drop_without_applying_deltas();
            assert!(app.text_edit.is_some());
            assert_eq!(app.editor.selected_views()[0].kind, target);
            ctx.run_ui(
                egui::RawInput {
                    events: vec![event(Key::Escape)],
                    ..Default::default()
                },
                |ui| app.dialogs(ui.ctx()),
            )
            .drop_without_applying_deltas();
            assert!(app.text_edit.is_none());
            assert_eq!(app.editor.selected_views()[0].kind, target);
            app.editor.undo();
            assert_eq!(app.editor.document, before);
        }
    }
    #[test]
    fn static_media_controls_do_not_offer_keyboard_text_editing() {
        for kind in [
            Kind::Button,
            Kind::VolumeSlider,
            Kind::Playback,
            Kind::Webcam,
            Kind::Video,
            Kind::CoverFlow,
            Kind::ColumnChart,
            Kind::BarChart,
            Kind::LineChart,
            Kind::PieChart,
            Kind::RedCross,
            Kind::ScratchOut,
        ] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(kind, 100., 100.);
            let output = ctx.run_ui(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key: Key::Enter,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::NONE,
                    }],
                    ..Default::default()
                },
                |ui| app.shortcuts(ui.ctx()),
            );
            output.drop_without_applying_deltas();
            assert_eq!(app.text_edit.is_some(), kind == Kind::Button, "{kind:?}");
        }
    }
    #[test]
    fn native_drag_delivered_in_one_frame_is_not_lost() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Button, 50., 50.);
        frame(&ctx, &mut app, vec![]);
        let start = app.screen([80., 65.]);
        let end = start + Vec2::new(60., 30.);
        frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(start),
                pointer(start, true),
                egui::Event::PointerMoved(end),
                pointer(end, false),
            ],
        );
        let bounds = app.editor.current().controls[0].bounds;
        assert_eq!((bounds.x, bounds.y), (110., 80.));
        app.editor.undo();
        assert_eq!(app.editor.current().controls[0].bounds.x, 50.);
    }
    #[test]
    fn dirty_project_requires_decision_and_invalid_open_preserves_work() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Heading, 20., 20.);
        let expected = app.editor.document.clone();
        app.request(Pending::New, &ctx);
        assert!(app.pending.is_some());
        assert_eq!(app.editor.document, expected);
        assert!(!app.load(PathBuf::from("not-a-real-project.wireframe")));
        assert_eq!(app.editor.document, expected);
    }
    #[test]
    fn failed_recursive_save_retains_dirty_state_and_document() {
        struct FailingStore;
        impl ProjectRepository for FailingStore {
            fn load(&self, _: &std::path::Path) -> Result<Project, String> {
                Err("Unavailable".into())
            }
            fn save(&self, _: &std::path::Path, _: &Project) -> Result<(), String> {
                Err("Disk full".into())
            }
        }
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Button, 10., 20.);
        app.editor.add(Kind::Label, 60., 70.);
        app.editor.select_all();
        app.editor.group();
        let expected = app.editor.document.clone();
        app.file = Some(PathBuf::from("unused-test-path.wireframe"));
        app.store = Box::new(FailingStore);
        assert!(!app.save(false));
        assert_eq!(app.editor.document, expected);
        assert!(app.editor.dirty());
        assert_eq!(app.error.as_deref(), Some("Disk full"));
    }

    fn alignment_frame(
        ctx: &egui::Context,
        app: &mut WireframeApp,
        mut events: Vec<egui::Event>,
        modifiers: Modifiers,
        focused: bool,
    ) {
        for event in &mut events {
            if let egui::Event::PointerButton {
                modifiers: value, ..
            } = event
            {
                *value = modifiers;
            }
        }
        events.insert(0, egui::Event::ModifiersChanged(modifiers));
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
                events,
                focused,
                ..Default::default()
            },
            |ui| {
                app.shortcuts(ui.ctx());
                app.canvas(ui);
            },
        );
        output.textures_delta.clear();
    }
    fn alignment_fixture(ctx: &egui::Context) -> WireframeApp {
        let mut app = WireframeApp::workspace(ctx, None);
        app.editor.add(Kind::CircleButton, 50., 50.);
        app.editor.add(Kind::CircleButton, 201., 100.);
        frame(ctx, &mut app, vec![]);
        app
    }
    #[test]
    fn shift_at_canvas_press_selects_instead_of_moving_even_after_key_release() {
        for (coalesced, held) in [(false, false), (false, true), (true, false), (true, true)] {
            let ctx = egui::Context::default();
            let mut app = alignment_fixture(&ctx);
            let before = app.editor.document.clone();
            let undo = app.editor.undo_name().map(str::to_owned);
            let start = app.screen([225., 124.]);
            let end = start + Vec2::new(80., 45.);
            let press = egui::Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::SHIFT,
            };
            let mut events = vec![
                egui::Event::ModifiersChanged(Modifiers::SHIFT),
                egui::Event::PointerMoved(start),
                press,
            ];
            if !coalesced {
                frame(&ctx, &mut app, std::mem::take(&mut events));
            }
            let release_modifiers = if held {
                Modifiers::SHIFT
            } else {
                Modifiers::NONE
            };
            events.extend([
                egui::Event::ModifiersChanged(release_modifiers),
                egui::Event::PointerMoved(end),
                egui::Event::PointerButton {
                    pos: end,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: release_modifiers,
                },
            ]);
            frame(&ctx, &mut app, events);
            assert!(
                app.editor.document == before,
                "coalesced={coalesced}, held={held}"
            );
            assert_eq!(app.editor.undo_name(), undo.as_deref());
            assert!(app.drag.is_none());
            assert!(app.text_edit.is_none());
        }
    }
    #[test]
    fn option_drag_stationary_modifier_changes_release_and_cancel_preserve_history() {
        for ending in ["copy", "move", "escape", "focus"] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.editor.add(Kind::Button, 50., 50.);
            app.editor.add(Kind::Checkbox, 300., 50.);
            let redo = app.editor.document.clone();
            app.editor.undo();
            frame(&ctx, &mut app, vec![]);
            let before = app.editor.document.clone();
            let start = app.screen([80., 65.]);
            let end = start + Vec2::new(90., 60.);
            alignment_frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(start), pointer(start, true)],
                Modifiers::NONE,
                true,
            );
            alignment_frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(end)],
                Modifiers::NONE,
                true,
            );
            assert_eq!(app.editor.nodes().len(), 1);
            alignment_frame(&ctx, &mut app, vec![], Modifiers::ALT, true);
            assert_eq!(app.editor.nodes().len(), 2, "{ending}");
            assert_eq!(app.editor.nodes()[0], before.pages[0].content.nodes[0]);
            let copy_id = app.editor.nodes()[1].id;
            assert!(app.editor.redo_name().is_some());
            alignment_frame(&ctx, &mut app, vec![], Modifiers::NONE, true);
            assert_eq!(app.editor.nodes().len(), 1);
            alignment_frame(&ctx, &mut app, vec![], Modifiers::ALT, true);
            assert_eq!(app.editor.nodes()[1].id, copy_id);
            if ending == "escape" || ending == "focus" {
                let events = if ending == "escape" {
                    vec![egui::Event::Key {
                        key: Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::ALT,
                    }]
                } else {
                    vec![]
                };
                alignment_frame(&ctx, &mut app, events, Modifiers::ALT, ending != "focus");
                assert!(app.drag.is_none());
                assert_eq!(app.editor.document, before);
                app.editor.redo();
                assert_eq!(app.editor.document, redo);
            } else {
                alignment_frame(
                    &ctx,
                    &mut app,
                    vec![pointer(end, false)],
                    if ending == "copy" {
                        Modifiers::ALT
                    } else {
                        Modifiers::NONE
                    },
                    true,
                );
                assert_eq!(
                    app.editor.nodes().len(),
                    if ending == "copy" { 2 } else { 1 }
                );
                assert!(!app.editor.redo_name().is_some());
                app.editor.undo();
                assert_eq!(app.editor.document, before);
            }
        }
    }
    #[test]
    fn option_drag_release_modifiers_win_over_later_events_and_keep_grid_constraints() {
        for (shift, command, expected) in [
            (false, false, (140., 110.)),
            (true, false, (140., 50.)),
            (false, true, (143., 113.)),
        ] {
            let ctx = egui::Context::default();
            let mut app = WireframeApp::workspace(&ctx, None);
            app.snap = true;
            app.editor.add(Kind::Button, 50., 50.);
            frame(&ctx, &mut app, vec![]);
            let start = app.screen([80., 65.]);
            let end = start + Vec2::new(93., 63.);
            let modifiers = Modifiers {
                alt: true,
                shift,
                command,
                ..Modifiers::NONE
            };
            let mut events = vec![
                egui::Event::ModifiersChanged(Modifiers::NONE),
                egui::Event::PointerMoved(start),
                pointer(start, true),
                egui::Event::ModifiersChanged(modifiers),
                egui::Event::PointerMoved(end),
                pointer(end, false),
                egui::Event::ModifiersChanged(Modifiers::NONE),
            ];
            for event in &mut events {
                if let egui::Event::PointerButton {
                    pressed: false,
                    modifiers: m,
                    ..
                } = event
                {
                    *m = modifiers;
                }
            }
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000., 700.))),
                    events,
                    ..Default::default()
                },
                |ui| {
                    app.shortcuts(ui.ctx());
                    app.canvas(ui);
                },
            );
            output.textures_delta.clear();
            let controls = app.editor.current().controls;
            assert_eq!(controls.len(), 2);
            assert_eq!((controls[1].bounds.x, controls[1].bounds.y), expected);
            assert_eq!((controls[0].bounds.x, controls[0].bounds.y), (50., 50.));
        }
    }
    #[test]
    fn option_drag_duplicates_without_moving_original_and_undo_is_atomic() {
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.editor.add(Kind::Button, 50., 50.);
        frame(&ctx, &mut app, vec![]);
        let before = app.editor.document.clone();
        let original = app.editor.current().controls[0].id;
        let start = app.screen([80., 65.]);
        let end = start + Vec2::new(90., 60.);
        alignment_frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(start),
                pointer(start, true),
                egui::Event::PointerMoved(end),
                pointer(end, false),
            ],
            Modifiers::ALT,
            true,
        );
        let controls = app.editor.current().controls;
        assert_eq!(controls.len(), 2);
        assert_eq!(
            controls.iter().find(|c| c.id == original).unwrap().bounds.x,
            50.
        );
        let copy = controls.iter().find(|c| c.id != original).unwrap();
        assert_eq!((copy.bounds.x, copy.bounds.y), (140., 110.));
        assert_eq!(app.editor.selection, HashSet::from([copy.id]));
        let after = app.editor.document.clone();
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        app.editor.redo();
        assert_eq!(app.editor.document, after);
    }
    #[test]
    fn canvas_alignment_drag_shows_guides_at_each_zoom_and_undo_restores_start() {
        for zoom in [0.75, 1., 2.] {
            let ctx = egui::Context::default();
            let mut app = alignment_fixture(&ctx);
            app.zoom = zoom;
            let before = app.editor.document.clone();
            let start = app.screen([225., 124.]);
            let end = app.screen([75., 124.]);
            alignment_frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(start), pointer(start, true)],
                Modifiers::NONE,
                true,
            );
            alignment_frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(end)],
                Modifiers::NONE,
                true,
            );
            assert_eq!(
                app.editor.current().controls[1].bounds,
                Bounds::new(50., 98., 48., 48.)
            );
            assert!(!app.alignment_guides.is_empty());
            alignment_frame(
                &ctx,
                &mut app,
                vec![pointer(end, false)],
                Modifiers::NONE,
                true,
            );
            assert!(app.alignment_guides.is_empty());
            app.editor.undo();
            assert_eq!(app.editor.document, before);
        }
    }
    #[test]
    fn coalesced_alignment_drag_ignores_pointer_travel_before_press() {
        let ctx = egui::Context::default();
        let mut app = alignment_fixture(&ctx);
        let before = app.editor.document.clone();
        let start = app.screen([225., 124.]);
        let end = app.screen([75., 124.]);
        alignment_frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(start),
                pointer(start, true),
                egui::Event::PointerMoved(end),
                egui::Event::PointerMoved(end),
                pointer(end, false),
            ],
            Modifiers::NONE,
            true,
        );
        assert_eq!(
            app.editor.current().controls[1].bounds,
            Bounds::new(50., 98., 48., 48.)
        );
        assert!(app.drag.is_none());
        app.editor.undo();
        assert_eq!(app.editor.document, before);
    }
    #[test]
    fn alignment_drag_with_press_in_previous_frame_is_not_lost() {
        let ctx = egui::Context::default();
        let mut app = alignment_fixture(&ctx);
        let before = app.editor.document.clone();
        let start = app.screen([225., 124.]);
        let end = app.screen([75., 124.]);
        alignment_frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
            Modifiers::NONE,
            true,
        );
        alignment_frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(end), pointer(end, false)],
            Modifiers::NONE,
            true,
        );
        assert_eq!(
            app.editor.current().controls[1].bounds,
            Bounds::new(50., 98., 48., 48.)
        );
        assert!(app.drag.is_none());
        app.editor.undo();
        assert_eq!(app.editor.document, before);
    }
    #[test]
    fn canvas_alignment_modifier_bypass_and_axis_constraint_are_live() {
        let ctx = egui::Context::default();
        let mut app = alignment_fixture(&ctx);
        let start = app.screen([225., 124.]);
        let end = app.screen([75., 141.]);
        alignment_frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
            Modifiers::NONE,
            true,
        );
        alignment_frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(end)],
            Modifiers::SHIFT,
            true,
        );
        assert_eq!(
            app.editor.current().controls[1].bounds,
            Bounds::new(50., 100., 48., 48.)
        );
        alignment_frame(&ctx, &mut app, vec![], Modifiers::COMMAND, true);
        assert_eq!(
            app.editor.current().controls[1].bounds,
            Bounds::new(51., 117., 48., 48.)
        );
        assert!(app.alignment_guides.is_empty());
        alignment_frame(&ctx, &mut app, vec![], Modifiers::NONE, true);
        assert!(!app.alignment_guides.is_empty());
    }
    #[test]
    fn canvas_resize_snaps_active_edge_and_focus_loss_cancels_preview() {
        let ctx = egui::Context::default();
        let mut app = alignment_fixture(&ctx);
        let before = app.editor.document.clone();
        let start = app.screen([201., 124.]);
        let end = app.screen([101., 124.]);
        alignment_frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
            Modifiers::NONE,
            true,
        );
        alignment_frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(end)],
            Modifiers::NONE,
            true,
        );
        assert_eq!(
            app.editor.current().controls[1].bounds,
            Bounds::new(98., 100., 151., 48.)
        );
        assert!(!app.alignment_guides.is_empty());
        alignment_frame(&ctx, &mut app, vec![], Modifiers::NONE, false);
        assert_eq!(app.editor.document, before);
        assert!(app.alignment_guides.is_empty());
        assert!(app.drag.is_none());
    }
    #[test]
    fn multi_selection_alignment_keeps_internal_spacing_and_one_undo() {
        let ctx = egui::Context::default();
        let mut app = alignment_fixture(&ctx);
        let second = app.editor.current().controls[1].id;
        app.editor.add(Kind::CircleButton, 301., 200.);
        app.editor.select(second, true);
        let before = app.editor.document.clone();
        let start = app.screen([225., 124.]);
        let end = app.screen([75., 124.]);
        alignment_frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
            Modifiers::NONE,
            true,
        );
        alignment_frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(end)],
            Modifiers::NONE,
            true,
        );
        alignment_frame(
            &ctx,
            &mut app,
            vec![pointer(end, false)],
            Modifiers::NONE,
            true,
        );
        let c = app.editor.current().controls;
        assert_eq!((c[1].bounds.x, c[1].bounds.y), (50., 98.));
        assert_eq!((c[2].bounds.x, c[2].bounds.y), (150., 198.));
        app.editor.undo();
        assert_eq!(app.editor.document, before);
    }
}
