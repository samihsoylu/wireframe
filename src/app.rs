// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

mod components;
mod icon_picker;
mod images;
mod previews;
mod properties;
mod recovery;

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
    storage::{self, ProjectRepository, ProjectStore},
    workspace::{Clipboard, Editor},
};

const ACCENT: Color32 = Color32::from_rgb(49, 94, 145);
enum Pending {
    New,
    Open(PathBuf),
    Quit,
    Example,
}
enum Drag {
    Move {
        start: Pos2,
        original: Vec<(Id, Bounds)>,
    },
    Resize {
        start: Pos2,
        original: Vec<(Id, Bounds)>,
        bounds: Bounds,
        handle: (i8, i8),
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
    editor: Editor,
    store: Box<dyn ProjectRepository>,
    file: Option<PathBuf>,
    quick: String,
    quick_index: usize,
    quick_focus: bool,
    show_library: bool,
    show_navigator: bool,
    show_inspector: bool,
    thumbnails: bool,
    category: String,
    zoom: f32,
    pan: Vec2,
    drag: Option<Drag>,
    grid: bool,
    snap: bool,
    status: String,
    error: Option<String>,
    pending: Option<Pending>,
    image_import: Option<(
        wireframe::workspace::ImageTarget,
        wireframe::images::Prepared,
    )>,
    image_crop: Option<images::CropEdit>,
    quit_allowed: bool,
    text_edit: Option<(Id, String)>,
    clipboard: Clipboard,
    previews: previews::LibraryPreviews,
    recovery: Option<PathBuf>,
    _recovery_lock: Option<std::fs::File>,
    recovery_offer: Option<Project>,
    last_recovery: Instant,
    recovery_doc: Option<Project>,
    help: bool,
    about: bool,
    present: bool,
    presentation_history: Vec<Id>,
    canvas_rect: Rect,
}
impl WireframeApp {
    fn workspace(ctx: &egui::Context, mut recovery: Option<PathBuf>) -> Self {
        wireframe::fonts::install(ctx);
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
        let recovery_offer = recovery.as_ref().and_then(|p| ProjectStore.load(p).ok());
        Self {
            editor: Editor::default(),
            store: Box::new(ProjectStore),
            file: None,
            quick: String::new(),
            quick_index: 0,
            quick_focus: false,
            show_library: true,
            show_navigator: true,
            show_inspector: true,
            thumbnails: true,
            category: "All".into(),
            zoom: 1.,
            pan: Vec2::ZERO,
            drag: None,
            grid: false,
            snap: false,
            status,
            error: None,
            pending: None,
            image_import: None,
            image_crop: None,
            quit_allowed: false,
            text_edit: None,
            clipboard: Clipboard::default(),
            previews: previews::LibraryPreviews::default(),
            recovery,
            _recovery_lock: recovery_lock,
            recovery_offer,
            last_recovery: Instant::now(),
            recovery_doc: None,
            help: false,
            about: false,
            present: false,
            presentation_history: vec![],
            canvas_rect: Rect::NOTHING,
        }
    }
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::workspace(&cc.egui_ctx, storage::recovery_path());
        let args: Vec<_> = std::env::args().skip(1).collect();
        if args.iter().any(|a| a == "--demo") {
            app.editor = Editor::new(Project::try_from(example_document()).expect("valid example"));
        } else if let Some(path) = args.first() {
            app.load(PathBuf::from(path));
        }
        app
    }
    fn load(&mut self, path: PathBuf) -> bool {
        match self.store.load(&path) {
            Ok(doc) => {
                let candidate = Editor::new(doc);
                for page in &candidate.document.pages {
                    if let Err(error) = candidate.layout_page(page.id) {
                        self.error = Some(error);
                        return false;
                    }
                }
                self.editor = candidate;
                self.file = Some(path);
                self.pan = Vec2::ZERO;
                self.zoom = 1.;
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
        self.editor.commit();
        let path = if choose || self.file.is_none() {
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
        } else {
            self.file.clone()
        };
        let Some(mut path) = path else {
            return false;
        };
        if path.extension().is_none() {
            path.set_extension("wireframe");
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
    fn request(&mut self, pending: Pending, ctx: &egui::Context) {
        self.editor.commit();
        if self.editor.dirty() {
            self.pending = Some(pending);
        } else {
            self.perform(pending, ctx);
        }
    }
    fn perform(&mut self, pending: Pending, ctx: &egui::Context) {
        match pending {
            Pending::New => {
                self.editor = Editor::default();
                self.file = None;
                self.pan = Vec2::ZERO;
                self.zoom = 1.;
                self.clear_recovery();
            }
            Pending::Open(p) => {
                if self.load(p) {
                    self.clear_recovery();
                }
            }
            Pending::Quit => {
                self.quit_allowed = true;
                self.clear_recovery();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Pending::Example => {
                self.clear_recovery();
                self.editor =
                    Editor::new(Project::try_from(example_document()).expect("valid example"));
                self.file = None;
                self.zoom = 1.;
                self.pan = Vec2::ZERO;
            }
        }
    }
    fn open(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Wireframe project", &["wireframe"])
            .pick_file()
        {
            self.request(Pending::Open(path), ctx);
        }
    }
    fn export(&mut self, png: bool) {
        let ext = if png { "png" } else { "svg" };
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Wireframe export", &[ext])
            .set_file_name(format!("{}.{}", self.editor.current().name, ext))
            .save_file()
        {
            let bytes = self
                .editor
                .rendered_scene(self.editor.surface_id())
                .and_then(|drawing| {
                    if png {
                        drawing.png()
                    } else {
                        Ok(drawing.svg().into_bytes())
                    }
                });
            match bytes.and_then(|bytes| storage::write_atomic(&path, &bytes)) {
                Ok(()) => self.status = format!("Exported {}", path.display()),
                Err(e) => self.error = Some(e),
            }
        }
    }
    fn add(&mut self, kind: Kind) {
        let n = self.editor.current().controls.len();
        let x = 50. + (n % 8) as f32 * 20.;
        let y = 50. + (n % 8) as f32 * 24.;
        self.editor.add(kind, x, y);
        self.status = format!("Added {}", kind.name());
    }
    fn copy(&mut self, ctx: &egui::Context) {
        self.clipboard = self.editor.copy();
        if let Ok(s) = serde_json::to_string(&self.clipboard) {
            ctx.copy_text(format!("wireframe-nodes-v2:{s}"));
        }
        self.status = format!("Copied {} controls", self.clipboard.len());
    }
    fn pasted(&mut self, s: &str) {
        if let Some(json) = s.strip_prefix("wireframe-nodes-v2:") {
            if json.len() > 4_000_000 {
                self.error = Some("Clipboard content is too large".into());
                return;
            }
            match serde_json::from_str::<Clipboard>(json) {
                Ok(copy) => self.editor.paste(&copy, 20.),
                Err(e) => self.error = Some(format!("Cannot paste controls: {e}")),
            };
            return;
        }
        if let Some(json) = s.strip_prefix("wireframe-controls-v1:") {
            if json.len() > 4_000_000 {
                self.error = Some("Clipboard content is too large".into());
                return;
            }
            if let Ok(controls) = serde_json::from_str::<Vec<Control>>(json) {
                if controls.len() <= 1000
                    && controls.iter().all(|c| {
                        c.bounds.valid()
                            && c.font_size.is_finite()
                            && (6.0..=120.0).contains(&c.font_size)
                            && c.text.len() <= 100_000
                    })
                {
                    self.editor.paste_legacy(&controls, 20.);
                } else {
                    self.error = Some("Clipboard contains invalid controls".into());
                }
            }
        }
    }
    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.pending.is_some()
            || self.text_edit.is_some()
            || self.recovery_offer.is_some()
            || self.image_crop.is_some()
        {
            return;
        }
        let take = |m, k| ctx.input_mut(|i| i.consume_key(m, k));
        if self.present {
            if take(Modifiers::NONE, Key::Escape) {
                self.present = false;
            }
            return;
        }
        if take(Modifiers::COMMAND, Key::S) {
            self.save(false);
        }
        if take(Modifiers::COMMAND, Key::O) {
            self.open(ctx);
        }
        if take(Modifiers::COMMAND, Key::N) {
            self.request(Pending::New, ctx);
        }
        if take(Modifiers::COMMAND | Modifiers::SHIFT, Key::N) {
            self.editor.add_page();
        }
        if take(Modifiers::COMMAND, Key::J) {
            self.show_navigator = !self.show_navigator;
        }
        if take(Modifiers::COMMAND, Key::L) {
            self.show_library = !self.show_library;
        }
        if take(Modifiers::COMMAND, Key::Semicolon) {
            self.show_inspector = !self.show_inspector;
        }
        if take(Modifiers::COMMAND, Key::Num0) {
            self.fit();
        }
        if take(Modifiers::COMMAND, Key::Num1) {
            self.zoom = 1.;
        }
        if take(Modifiers::COMMAND | Modifiers::SHIFT, Key::F) {
            self.editor.commit();
            self.presentation_history.clear();
            self.present = true;
        }
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        if take(Modifiers::NONE, Key::Slash) || take(Modifiers::NONE, Key::Plus) {
            self.quick_focus = true;
        }
        if take(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z) || take(Modifiers::COMMAND, Key::Y) {
            self.editor.redo();
        } else if take(Modifiers::COMMAND, Key::Z) {
            self.editor.undo();
        }
        if take(Modifiers::COMMAND, Key::A) {
            self.editor.select_all();
        }
        if take(Modifiers::COMMAND, Key::D) {
            self.editor.duplicate();
        }
        if take(Modifiers::COMMAND | Modifiers::SHIFT, Key::G) {
            self.editor.ungroup();
        } else if take(Modifiers::COMMAND, Key::G) {
            self.editor.group();
        }
        if take(Modifiers::NONE, Key::Delete) || take(Modifiers::NONE, Key::Backspace) {
            self.editor.delete();
        }
        for (k, dx, dy) in [
            (Key::ArrowLeft, -1., 0.),
            (Key::ArrowRight, 1., 0.),
            (Key::ArrowUp, 0., -1.),
            (Key::ArrowDown, 0., 1.),
        ] {
            if take(Modifiers::SHIFT, k) {
                self.editor.move_by(dx * 10., dy * 10.);
            } else if take(Modifiers::NONE, k) {
                self.editor.move_by(dx, dy);
            }
        }
        for event in ctx.input(|i| i.events.clone()) {
            match event {
                egui::Event::Copy => self.copy(ctx),
                egui::Event::Cut => {
                    self.copy(ctx);
                    self.editor.delete();
                }
                egui::Event::Paste(s) => self.pasted(&s),
                _ => {}
            }
        }
        if take(Modifiers::NONE, Key::Escape) {
            self.present = false;
            self.editor.cancel();
            self.drag = None;
            if !self.editor.leave() {
                self.editor.selection.clear();
            }
        }
        if take(Modifiers::NONE, Key::Enter)
            && let Some(c) = self.editor.selected_views().first()
            && !self.editor.enter(c.id)
            && c.group.is_none()
        {
            self.text_edit = Some((c.id, c.text.clone()));
        }
    }
    fn navigate_page(&mut self, id: Id) {
        if self.editor.document.pages.iter().any(|p| p.id == id) {
            if id != self.editor.page {
                self.presentation_history.push(self.editor.page);
            }
            self.editor.switch_page(id);
        } else {
            self.status = "This page is no longer available".into();
        }
    }
    fn follow_target(&mut self, target: &str, ctx: &egui::Context) {
        if let Some(id) = target
            .strip_prefix("page:")
            .and_then(|s| s.parse::<Id>().ok())
        {
            self.navigate_page(id);
        } else if target == "back:" {
            while let Some(id) = self.presentation_history.pop() {
                if self.editor.document.pages.iter().any(|p| p.id == id) {
                    self.editor.switch_page(id);
                    return;
                }
            }
            self.status = "No previous page in this presentation".into();
        } else if target.starts_with("https://") || target.starts_with("http://") {
            ctx.open_url(egui::OpenUrl::new_tab(target));
        } else {
            self.status = "This link destination is not available".into();
        }
    }
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        if self.present {
            egui::Panel::top("presentation").show(ui, |ui| {
                if self.image_crop.is_some() {
                    ui.disable();
                }
                ui.horizontal(|ui| {
                    if ui.button("Back to editor").clicked() {
                        self.present = false;
                    }
                    ui.separator();
                    ui.strong(&self.editor.document.title);
                    let pages: Vec<_> = self
                        .editor
                        .document
                        .pages
                        .iter()
                        .map(|p| (p.id, p.name.clone()))
                        .collect();
                    for (id, name) in pages {
                        if ui.selectable_label(self.editor.page == id, name).clicked() {
                            self.navigate_page(id);
                        }
                    }
                    if ui.button("Fit").clicked() {
                        self.fit();
                    }
                });
            });
            return;
        }
        let ctx = ui.ctx().clone();
        egui::Panel::top("toolbar")
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgb(56, 61, 66))
                    .inner_margin(8),
            )
            .show(ui, |ui| {
                if self.image_crop.is_some() {
                    ui.disable();
                }
                *ui.visuals_mut() = egui::Visuals::dark();
                ui.visuals_mut().override_text_color = Some(Color32::from_gray(235));
                ui.spacing_mut().button_padding = Vec2::new(8., 4.);
                ui.horizontal(|ui| {
                    ui.add_space(6.);
                    ui.label(
                        egui::RichText::new("Wireframe")
                            .size(15.)
                            .strong()
                            .color(Color32::WHITE),
                    );
                    ui.separator();
                    ui.menu_button("File", |ui| {
                        if ui.button("New project     ⌘/Ctrl N").clicked() {
                            self.request(Pending::New, &ctx);
                            ui.close();
                        }
                        if ui.button("Open…               ⌘/Ctrl O").clicked() {
                            self.open(&ctx);
                            ui.close();
                        }
                        if ui.button("Save                   ⌘/Ctrl S").clicked() {
                            self.save(false);
                            ui.close();
                        }
                        if ui.button("Save as…").clicked() {
                            self.save(true);
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Export page as PNG…").clicked() {
                            self.export(true);
                            ui.close();
                        }
                        if ui.button("Export page as SVG…").clicked() {
                            self.export(false);
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Open example project").clicked() {
                            self.request(Pending::Example, &ctx);
                            ui.close();
                        }
                    });
                    ui.menu_button("Edit", |ui| {
                        if ui
                            .add_enabled(
                                !self.editor.selection.is_empty(),
                                egui::Button::new("Copy"),
                            )
                            .clicked()
                        {
                            self.copy(&ctx);
                            ui.close();
                        }
                        if ui
                            .add_enabled(
                                !self.clipboard.is_empty(),
                                egui::Button::new("Paste copied controls"),
                            )
                            .clicked()
                        {
                            self.editor.paste(&self.clipboard, 20.);
                            ui.close();
                        }
                        if ui.button("Duplicate").clicked() {
                            self.editor.duplicate();
                            ui.close();
                        }
                        if ui.button("Select all").clicked() {
                            self.editor.select_all();
                            ui.close();
                        }
                        if ui.button("Delete selection").clicked() {
                            self.editor.delete();
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Group").clicked() {
                            self.editor.group();
                            ui.close();
                        }
                        if ui.button("Ungroup").clicked() {
                            self.editor.ungroup();
                            ui.close();
                        }
                    });
                    if ui
                        .add_enabled(self.editor.undo_name().is_some(), egui::Button::new("Undo"))
                        .on_hover_text(self.editor.undo_name().unwrap_or("Nothing to undo"))
                        .clicked()
                    {
                        self.editor.undo();
                    }
                    if ui
                        .add_enabled(self.editor.redo_name().is_some(), egui::Button::new("Redo"))
                        .on_hover_text(self.editor.redo_name().unwrap_or("Nothing to redo"))
                        .clicked()
                    {
                        self.editor.redo();
                    }
                    ui.separator();
                    ui.label(format!(
                        "{}{}",
                        self.editor.document.title,
                        if self.editor.dirty() { " •" } else { "" }
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Help").clicked() {
                            self.help = true;
                        }
                        if ui.button("Save").clicked() {
                            self.save(false);
                        }
                        if ui
                            .button(if self.present { "Edit" } else { "Present" })
                            .clicked()
                        {
                            self.presentation_history.clear();
                            self.present = !self.present;
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.add_space(6.);
                    if ui.button("−").clicked() {
                        self.zoom = (self.zoom / 1.2).max(0.2);
                    }
                    ui.label(format!("{:.0}%", self.zoom * 100.));
                    if self.editor.definition.is_some() {
                        ui.strong(format!("Editing component: {}", self.editor.current().name));
                        if ui.button("Back to wireframe").clicked() {
                            self.editor.switch_page(self.editor.page);
                        }
                    }
                    if self.editor.editing_instance() {
                        ui.strong("Editing this instance");
                    }
                    if !self.editor.context.is_empty()
                        && ui
                            .button(if self.editor.editing_instance() {
                                "Exit component level"
                            } else {
                                "Exit group"
                            })
                            .clicked()
                    {
                        self.editor.leave();
                    }
                    if ui.button("+").clicked() {
                        self.zoom = (self.zoom * 1.2).min(4.);
                    }
                    if ui.button("Fit").clicked() {
                        self.fit();
                    }
                    if ui.button("100%").clicked() {
                        self.zoom = 1.;
                    }
                    ui.separator();
                    ui.checkbox(&mut self.grid, "Grid");
                    ui.checkbox(&mut self.snap, "Snap to 10");
                    ui.separator();
                    let mut sketch = self.editor.document.settings.sketch;
                    if ui.checkbox(&mut sketch, "Sketch style").changed() {
                        self.editor
                            .edit("Change style", |e| e.document.settings.sketch = sketch);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.quick_add_ui(ui);
                        if ui
                            .selectable_label(self.show_inspector, "Inspector")
                            .clicked()
                        {
                            self.show_inspector = !self.show_inspector;
                        }
                        if ui.selectable_label(self.show_library, "Library").clicked() {
                            self.show_library = !self.show_library;
                        }
                        if ui.selectable_label(self.show_navigator, "Pages").clicked() {
                            self.show_navigator = !self.show_navigator;
                        }
                    });
                });
            });
        egui::Panel::bottom("status").show(ui, |ui| {
            if self.image_crop.is_some() {
                ui.disable();
            }
            ui.horizontal(|ui| {
                ui.small(&self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.small(format!(
                        "{} controls  ·  {} selected",
                        self.editor.current().controls.len(),
                        self.editor.selection.len()
                    ));
                });
            });
        });
    }
    fn quick_add_ui(&mut self, ui: &mut egui::Ui) {
        let response = ui.add(
            egui::TextEdit::singleline(&mut self.quick)
                .id_salt("quick-add")
                .hint_text("Quick Add  /")
                .desired_width(180.),
        );
        if self.quick_focus {
            response.request_focus();
            self.quick_focus = false;
        }
        if response.changed() {
            self.quick_index = 0;
        }
        let query = self.quick.to_lowercase();
        let mut matches: Vec<_> = Kind::ALL
            .into_iter()
            .filter(|k| k.name().to_lowercase().contains(&query))
            .collect();
        matches.sort_by_key(|k| (k.name().to_lowercase() != query, k.name()));
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
            chosen = matches.get(self.quick_index).copied();
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
                    for (index, kind) in matches.iter().enumerate().take(10) {
                        if ui
                            .selectable_label(index == self.quick_index, kind.name())
                            .clicked()
                        {
                            chosen = Some(*kind);
                        }
                    }
                });
        }
        if let Some(kind) = chosen {
            if ui.input(|i| i.modifiers.shift)
                && let Some(pos) = ui.ctx().pointer_latest_pos()
            {
                let point = self.world(pos);
                self.editor.add(kind, point.x, point.y);
            } else {
                self.add(kind);
            }
            self.quick.clear();
            response.request_focus();
        }
    }
    fn library(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("control-library")
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_gray(235))
                    .inner_margin(6),
            )
            .show(ui, |ui| {
                if self.image_crop.is_some() {
                    ui.disable();
                }
                egui::ScrollArea::horizontal()
                    .id_salt("library-categories")
                    .show(ui, |ui| {
                        if self.image_crop.is_some() {
                            ui.disable();
                        }
                        ui.horizontal(|ui| {
                            for cat in [
                                "All",
                                "Buttons",
                                "Common",
                                "Containers",
                                "Forms",
                                "Layout",
                                "Media",
                                "Icons",
                                "Text",
                            ] {
                                ui.selectable_value(&mut self.category, cat.into(), cat);
                            }
                        });
                    });
                egui::ScrollArea::horizontal()
                    .id_salt("library-previews")
                    .show(ui, |ui| {
                        if self.image_crop.is_some() {
                            ui.disable();
                        }
                        ui.horizontal(|ui| {
                            let mut kinds: Vec<_> = Kind::ALL
                                .into_iter()
                                .filter(|kind| match self.category.as_str() {
                                    "All" => true,
                                    "Buttons" => matches!(
                                        kind,
                                        Kind::Button
                                            | Kind::CircleButton
                                            | Kind::PointedButton
                                            | Kind::MultilineButton
                                            | Kind::ButtonBar
                                            | Kind::Checkbox
                                            | Kind::Radio
                                            | Kind::Toggle
                                    ),
                                    "Common" => matches!(
                                        kind,
                                        Kind::Button
                                            | Kind::Label
                                            | Kind::TextInput
                                            | Kind::Rectangle
                                            | Kind::Image
                                            | Kind::Checkbox
                                            | Kind::Browser
                                    ),
                                    "Containers" => {
                                        matches!(kind, Kind::Browser | Kind::Phone | Kind::Panel)
                                    }
                                    "Media" => matches!(kind, Kind::Image),
                                    cat => kind.category() == cat,
                                })
                                .collect();
                            kinds.sort_by_key(|k| k.name());
                            for kind in kinds {
                                ui.vertical(|ui| {
                                    let (r, response) = ui.allocate_exact_size(
                                        Vec2::new(94., 68.),
                                        Sense::click_and_drag(),
                                    );
                                    if response.hovered() {
                                        ui.painter().rect_filled(r, 3, Color32::from_gray(222));
                                    }
                                    let painter = ui.painter().with_clip_rect(r);
                                    match self.previews.scene(kind, &self.editor.document.settings)
                                    {
                                        Ok(drawing) => {
                                            let b = drawing.bounds;
                                            let scale =
                                                (82. / b.w.max(1.)).min(43. / b.h.max(1.)).min(1.);
                                            let origin = r.min
                                                + Vec2::new(
                                                    (94. - b.w * scale) / 2. - b.x * scale,
                                                    4. - b.y * scale,
                                                );
                                            Self::paint_scene(&painter, drawing, origin, scale);
                                        }
                                        Err(_) => {
                                            painter.text(
                                                r.center_top() + Vec2::new(0., 18.),
                                                Align2::CENTER_TOP,
                                                "Preview unavailable",
                                                FontId::proportional(10.),
                                                Color32::DARK_GRAY,
                                            );
                                        }
                                    }
                                    painter.text(
                                        r.center_bottom() - Vec2::new(0., 4.),
                                        Align2::CENTER_BOTTOM,
                                        kind.name(),
                                        FontId::proportional(11.),
                                        Color32::from_gray(35),
                                    );
                                    response.widget_info(|| {
                                        egui::WidgetInfo::labeled(
                                            egui::WidgetType::Button,
                                            true,
                                            kind.name(),
                                        )
                                    });
                                    if response.double_clicked() {
                                        self.add(kind);
                                    }
                                    if (response.drag_stopped()
                                        || Self::instant_drag(ui, r).is_some())
                                        && let Some(pos) = Self::instant_drag(ui, r)
                                            .map(|(_, end)| end)
                                            .or(ui.ctx().pointer_latest_pos())
                                        && self.canvas_rect.contains(pos)
                                    {
                                        let point = self.world(pos);
                                        self.editor.add(kind, point.x, point.y);
                                    }
                                    response.on_hover_text(format!(
                                        "{} — double-click to add, or drag onto the canvas",
                                        kind.name()
                                    ));
                                });
                            }
                        });
                    });
            });
    }
    fn sidebar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("navigator")
            .resizable(true)
            .default_size(192.)
            .size_range(160.0..=300.0)
            .show(ui, |ui| {
                if self.image_crop.is_some() {
                    ui.disable();
                }
                ui.horizontal(|ui| {
                    ui.strong("Wireframes");
                    if ui
                        .small_button("+")
                        .on_hover_text("New wireframe")
                        .clicked()
                    {
                        self.editor.add_page();
                    }
                    if ui
                        .small_button(if self.thumbnails { "List" } else { "Tiles" })
                        .clicked()
                    {
                        self.thumbnails = !self.thumbnails;
                    }
                });
                ui.separator();
                let pages = self.editor.document.pages.clone();
                egui::ScrollArea::vertical()
                    .id_salt("page-navigator")
                    .max_height((ui.available_height() - 120.).max(140.))
                    .show(ui, |ui| {
                        if self.image_crop.is_some() {
                            ui.disable();
                        }
                        for page in pages {
                            let page = self.editor.rendered_page(page.id).unwrap_or(Page {
                                id: page.id,
                                name: page.name,
                                notes: page.notes,
                                controls: vec![],
                            });
                            let selected =
                                self.editor.definition.is_none() && self.editor.page == page.id;
                            if self.thumbnails {
                                let width = ui.available_width();
                                let (r, response) =
                                    ui.allocate_exact_size(Vec2::new(width, 144.), Sense::click());
                                ui.painter().rect_filled(
                                    r,
                                    0,
                                    if selected {
                                        Color32::from_rgb(65, 155, 221)
                                    } else {
                                        Color32::from_gray(235)
                                    },
                                );
                                let paper = Rect::from_min_max(
                                    r.min + Vec2::new(10., 8.),
                                    r.max - Vec2::new(10., 30.),
                                );
                                ui.painter().rect_filled(paper, 0, Color32::WHITE);
                                let bounds = scene::page_bounds(&page);
                                let scale = ((paper.width() - 12.) / bounds.w.max(1.))
                                    .min((paper.height() - 12.) / bounds.h.max(1.))
                                    .min(1.);
                                let origin = paper.center()
                                    - Vec2::new(bounds.w, bounds.h) * scale / 2.
                                    - Vec2::new(bounds.x, bounds.y) * scale;
                                let painter = ui.painter().with_clip_rect(paper);
                                if let Ok(drawing) = self.editor.rendered_scene(page.id) {
                                    Self::paint_scene(&painter, &drawing, origin, scale);
                                }
                                ui.painter().text(
                                    r.center_bottom() - Vec2::new(0., 9.),
                                    Align2::CENTER_BOTTOM,
                                    &page.name,
                                    FontId::proportional(12.),
                                    if selected {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_gray(40)
                                    },
                                );
                                response.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::SelectableLabel,
                                        true,
                                        &page.name,
                                    )
                                });
                                if response.clicked() {
                                    self.editor.switch_page(page.id);
                                }
                            } else if ui.selectable_label(selected, &page.name).clicked() {
                                self.editor.switch_page(page.id);
                            }
                        }
                    });
                ui.separator();
                self.components_sidebar(ui);
                egui::CollapsingHeader::new("Layers")
                    .default_open(false)
                    .show(ui, |ui| {
                        if self.image_crop.is_some() {
                            ui.disable();
                        }
                        let controls = self.editor.current().controls.clone();
                        for c in controls.iter().rev() {
                            let label = format!(
                                "{}{}",
                                if c.locked { "[locked] " } else { "" },
                                self.editor.selection_label(c.id)
                            );
                            if ui
                                .selectable_label(self.editor.selection.contains(&c.id), label)
                                .clicked()
                            {
                                self.editor.select(c.id, ui.input(|i| i.modifiers.shift));
                            }
                        }
                    });
            });
    }
    fn inspector(&mut self, ui: &mut egui::Ui) {
        egui::Panel::right("inspector").resizable(true).default_size(250.).size_range(220.0..=340.0).show(ui,|ui|{
            ui.add_space(10.);egui::ScrollArea::vertical().show(ui,|ui|{
                let selected=self.editor.selected_views();
                if selected.is_empty() && self.editor.editing_instance() {ui.heading("Component instance");ui.label("Select a child to change this instance. Reset returns a property to its source value.");return;}
                if selected.is_empty() && self.editor.definition.is_some() { self.component_source_inspector(ui); return; }
                if selected.is_empty(){ui.heading("Page");ui.label("Project title");let mut title=self.editor.document.title.clone();let r=ui.text_edit_singleline(&mut title);if r.changed(){self.editor.begin("Rename project");self.editor.document.title=title;}
if r.lost_focus(){self.editor.commit();}
                    ui.label("Page name");let mut name=self.editor.current().name.clone();let r=ui.text_edit_singleline(&mut name);if r.changed(){self.editor.begin("Rename page");self.editor.current_mut().name=name;}
if r.lost_focus(){self.editor.commit();}
                    ui.label("Notes");let mut notes=self.editor.current().notes.clone();let r=ui.add(egui::TextEdit::multiline(&mut notes).desired_rows(8).desired_width(f32::INFINITY).hint_text("Explain what this wireframe is for…"));if r.changed(){self.editor.begin("Edit notes");self.editor.current_mut().notes=notes;}
if r.lost_focus(){self.editor.commit();}
                    ui.add_space(14.);if ui.add_enabled(self.editor.document.pages.len()>1,egui::Button::new("Delete this page")).clicked(){self.editor.delete_page();}ui.separator();ui.small("Select a control on the canvas to change its content and appearance. Double-click it to edit text.");return;
                }
                ui.heading(if selected.len()==1{self.editor.selection_label(selected[0].id)}else{format!("{} controls",selected.len())});
                if selected.len()==1 {let mut c=selected[0].clone();let original=c.clone();
                    ui.add_space(10.);ui.label("Position & size");egui::Grid::new("geometry").num_columns(4).spacing([7.,8.]).show(ui,|ui|{ui.label("X");ui.add(egui::DragValue::new(&mut c.bounds.x).update_while_editing(false).speed(1.).range(-50_000.0..=50_000.0));ui.label("Y");ui.add(egui::DragValue::new(&mut c.bounds.y).update_while_editing(false).speed(1.).range(-50_000.0..=50_000.0));ui.end_row();ui.label("W");ui.add_enabled(!matches!(c.kind,Kind::Icon|Kind::Breadcrumbs|Kind::LinkBar) && !self.editor.missing_component(c.id) && !self.editor.empty_container(c.id),egui::DragValue::new(&mut c.bounds.w).update_while_editing(false).speed(1.).range((if matches!(c.kind,Kind::Breadcrumbs|Kind::LinkBar){1.0}else{8.0})..=10_000.0));ui.label("H");ui.add_enabled(!matches!(c.kind,Kind::Icon|Kind::IconLabel|Kind::PointedButton|Kind::Breadcrumbs|Kind::LinkBar) && !self.editor.missing_component(c.id) && !self.editor.empty_container(c.id),egui::DragValue::new(&mut c.bounds.h).update_while_editing(false).speed(1.).range(8.0..=10_000.0));ui.end_row();});
                    if c.group.is_none() && !self.typed_properties(ui, c.id) {ui.add_space(10.);ui.label("Text");ui.add(egui::TextEdit::multiline(&mut c.text).desired_rows(4).desired_width(f32::INFINITY));if c.kind==Kind::Table {ui.small("Separate columns with commas and rows with new lines.");}
if c.kind==Kind::Tabs {ui.small("Separate tabs with commas.");}
                    ui.horizontal(|ui|{ui.label("Size");ui.add(egui::DragValue::new(&mut c.font_size).range(6.0..=120.0));ui.checkbox(&mut c.bold,"Bold");});
                    ui.horizontal(|ui|{ui.label("Fill");ui.color_edit_button_srgb(&mut c.fill);});if matches!(c.kind,Kind::Checkbox|Kind::Radio|Kind::Toggle){ui.checkbox(&mut c.checked,"Selected state");}
                    }
                    if !self.editor.editing_instance() {ui.checkbox(&mut c.locked,"Lock position and content");}
                    if c.group.is_none() && !matches!(c.kind, Kind::ButtonBar | Kind::Breadcrumbs | Kind::LinkBar | Kind::Tabs | Kind::VerticalTabs) {
                    egui::ComboBox::from_id_salt("page-link").selected_text(c.link.and_then(|id|self.editor.document.pages.iter().find(|p|p.id==id).map(|p|p.name.clone())).unwrap_or("No page link".into())).show_ui(ui,|ui|{ui.selectable_value(&mut c.link,None,"No page link");for p in &self.editor.document.pages {ui.selectable_value(&mut c.link,Some(p.id),&p.name);}});
                    } else if self.editor.is_group(c.id) && ui.button("Edit group contents").clicked(){self.editor.enter(c.id);}
                    if c!=original {self.editor.begin("Edit properties");if original.locked {c.bounds=original.bounds;c.text=original.text;c.font_size=original.font_size;c.bold=original.bold;c.fill=original.fill;c.checked=original.checked;c.link=original.link;}
                        self.editor.update_control(c);}
                    if !ui.ctx().egui_wants_keyboard_input()&&!ui.input(|i|i.pointer.any_down()){self.editor.commit();}
                }else {ui.label("Align");ui.horizontal(|ui|{for(i,name)in ["Left","Center","Right"].iter().enumerate(){if ui.button(*name).clicked(){self.editor.align(0,i);}}});ui.horizontal(|ui|{for(i,name)in ["Top","Middle","Bottom"].iter().enumerate(){if ui.button(*name).clicked(){self.editor.align(1,i);}}});ui.horizontal(|ui|{if ui.button("Spread X").clicked(){self.editor.distribute(0);}
if ui.button("Spread Y").clicked(){self.editor.distribute(1);}});}
                self.component_actions(ui);
                if self.editor.editing_instance() {ui.small("Edit the source to add, remove or rearrange children.");return;}
                ui.add_space(14.);ui.separator();ui.horizontal(|ui|{if ui.button("Group").clicked(){self.editor.group();}
if ui.button("Ungroup").clicked(){self.editor.ungroup();}});ui.horizontal(|ui|{if ui.button("To front").clicked(){self.editor.reorder(true);}
if ui.button("To back").clicked(){self.editor.reorder(false);}});ui.horizontal(|ui|{if ui.button("Duplicate").clicked(){self.editor.duplicate();}
if ui.button("Delete").clicked(){self.editor.delete();}});
                if selected.len()>1 {ui.horizontal(|ui|{if ui.button("Lock").clicked(){self.editor.lock(true);}
if ui.button("Unlock").clicked(){self.editor.lock(false);}});}
            });
        });
    }
    fn fit(&mut self) {
        let b = scene::page_bounds(
            &self
                .editor
                .rendered_page(self.editor.surface_id())
                .unwrap_or_else(|_| self.editor.current()),
        );
        let size = self.canvas_rect.size();
        self.zoom = ((size.x - 100.) / b.w.max(1.))
            .min((size.y - 100.) / b.h.max(1.))
            .clamp(0.2, 2.);
        self.pan = Vec2::new(
            (size.x - b.w * self.zoom) / 2. - b.x * self.zoom,
            (size.y - b.h * self.zoom) / 2. - b.y * self.zoom,
        );
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
        for item in &drawing.items {
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
                Kind::Icon | Kind::Breadcrumbs | Kind::LinkBar
            )
        {
            return vec![];
        }
        let horizontal =
            single && matches!(selected[0].kind, Kind::IconLabel | Kind::PointedButton);
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
        .filter(|(_, y)| !horizontal || *y == 0)
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
                let (response, painter) =
                    ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
                self.canvas_rect = response.rect;
                let ctx = ui.ctx().clone();
                if response.hovered() {
                    let (scroll, zoom_mod) = ui.input(|i| {
                        (
                            i.smooth_scroll_delta,
                            i.modifiers.command || i.modifiers.ctrl,
                        )
                    });
                    if zoom_mod && scroll.y != 0. {
                        let pointer = ui
                            .input(|i| i.pointer.hover_pos())
                            .unwrap_or(response.rect.center());
                        let before = self.world(pointer);
                        self.zoom = (self.zoom * (scroll.y * 0.003).exp()).clamp(0.2, 4.);
                        self.pan =
                            pointer - response.rect.min - Vec2::new(before.x, before.y) * self.zoom;
                    } else {
                        self.pan += scroll;
                    }
                    let pinch = ui.input(|i| i.zoom_delta());
                    if pinch != 1.0 {
                        self.zoom = (self.zoom * pinch).clamp(0.2, 4.);
                    }
                }
                if self.image_crop.is_some() {
                    self.crop_canvas(ui, &response, &painter);
                    return;
                }
                // Native automation and fast mice can deliver a complete drag between
                // paints. Preserve the press origin even after egui releases its state.
                let instant_drag = if self.pending.is_none()
                    && self.text_edit.is_none()
                    && self.error.is_none()
                    && self.recovery_offer.is_none()
                {
                    Self::instant_drag(ui, response.rect)
                } else {
                    None
                };
                let pointer = instant_drag
                    .map(|(_, end)| end)
                    .or(response.interact_pointer_pos());
                if (response.drag_started() || instant_drag.is_some())
                    && let Some(pos) = instant_drag
                        .map(|(start, _)| start)
                        .or(ui.input(|i| i.pointer.press_origin()))
                        .or(pointer)
                {
                    let start = self.world(pos);
                    let space = ui.input(|i| {
                        i.key_down(Key::Space) || i.pointer.button_down(egui::PointerButton::Middle)
                    });
                    let resize = self
                        .editor
                        .bounds()
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
                    } else if let Some((handle, _)) = resize.filter(|_| !self.present) {
                        self.editor.begin("Resize");
                        self.drag = Some(Drag::Resize {
                            start,
                            original: self
                                .editor
                                .selected_views()
                                .iter()
                                .map(|c| (c.id, c.bounds))
                                .collect(),
                            bounds: self.editor.bounds().unwrap(),
                            handle,
                        });
                    } else {
                        let hit = self.editor.hit(start.x, start.y).map(|c| (c.id, c.locked));
                        if let Some((id, locked)) = hit {
                            if !self.editor.selection.contains(&id) {
                                self.editor.select(id, ui.input(|i| i.modifiers.shift));
                            }
                            if !locked && !self.present {
                                self.editor.begin("Move");
                                self.drag = Some(Drag::Move {
                                    start,
                                    original: self
                                        .editor
                                        .selected_views()
                                        .iter()
                                        .map(|c| (c.id, c.bounds))
                                        .collect(),
                                });
                            }
                        } else if !self.present {
                            let previous = if ui.input(|i| i.modifiers.shift) {
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
                if (response.dragged() || response.drag_stopped() || instant_drag.is_some())
                    && let (Some(pos), Some(drag)) = (pointer, self.drag.take())
                {
                    let now = self.world(pos);
                    match drag {
                        Drag::Pan { start, pan } => {
                            self.pan = pan + (pos - start);
                            self.drag = Some(Drag::Pan { start, pan });
                        }
                        Drag::Move { start, original } => {
                            let delta = now - start;
                            let snap = self.snap;
                            self.editor.selected_mut(|c| {
                                if let Some((_, b)) = original.iter().find(|v| v.0 == c.id) {
                                    c.bounds.x = (b.x + delta.x).clamp(-50_000., 50_000.);
                                    c.bounds.y = (b.y + delta.y).clamp(-50_000., 50_000.);
                                    if snap {
                                        c.bounds.x = (c.bounds.x / 10.).round() * 10.;
                                        c.bounds.y = (c.bounds.y / 10.).round() * 10.;
                                    }
                                }
                            });
                            self.drag = Some(Drag::Move { start, original });
                        }
                        Drag::Resize {
                            start,
                            original,
                            bounds,
                            handle,
                        } => {
                            let mut delta = now - start;
                            if self.snap {
                                delta.x = (delta.x / 10.).round() * 10.;
                                delta.y = (delta.y / 10.).round() * 10.;
                            }
                            let resized = bounds.resized(
                                handle,
                                (delta.x, delta.y),
                                ui.input(|i| i.modifiers.shift),
                            );
                            self.editor.selected_mut(|c| {
                                if let Some((_, b)) = original.iter().find(|v| v.0 == c.id) {
                                    c.bounds.x =
                                        resized.x + (b.x - bounds.x) * resized.w / bounds.w;
                                    c.bounds.y =
                                        resized.y + (b.y - bounds.y) * resized.h / bounds.h;
                                    c.bounds.w = (b.w * resized.w / bounds.w).max(1.);
                                    c.bounds.h = (b.h * resized.h / bounds.h).max(1.);
                                }
                            });
                            self.drag = Some(Drag::Resize {
                                start,
                                original,
                                bounds,
                                handle,
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
                }
                if response.clicked()
                    && instant_drag.is_none()
                    && let Some(pos) = pointer
                {
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
                        } else {
                            self.editor.select(c.id, ui.input(|i| i.modifiers.shift));
                        }
                    } else {
                        self.editor.selection.clear();
                    }
                }
                if response.double_clicked()
                    && !self.present
                    && let Some(c) = self.editor.selected_views().first()
                    && !c.locked
                    && !self.editor.enter(c.id)
                    && c.group.is_none()
                {
                    self.text_edit = Some((c.id, c.text.clone()));
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
                match self.editor.rendered_scene(self.editor.surface_id()) {
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
                if !self.present {
                    for c in &self.editor.current().controls {
                        if self.editor.selection.contains(&c.id) {
                            painter.rect_stroke(
                                self.screen_rect(c.bounds).expand(3.),
                                1,
                                Stroke::new(1., ACCENT),
                                StrokeKind::Outside,
                            );
                        }
                    }
                    if let Some(b) = self.editor.bounds() {
                        let r = self.screen_rect(b).expand(4.);
                        painter.rect_stroke(r, 0, Stroke::new(1., ACCENT), StrokeKind::Outside);
                        if self.editor.selected_views().iter().any(|c| !c.locked) {
                            for (_, point) in self.resize_handles(b) {
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
                if self.editor.current().controls.is_empty() {
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
                if !self.present {
                    response.context_menu(|ui| {
                        if ui.button("Duplicate").clicked() {
                            self.editor.duplicate();
                            ui.close();
                        }
                        if ui.button("Edit text").clicked() {
                            if let Some(c) = self.editor.selected_views().first() {
                                self.text_edit = Some((c.id, c.text.clone()));
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
                        if ui.button("Delete").clicked() {
                            self.editor.delete();
                            ui.close();
                        }
                    });
                }
            });
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some((id, mut value)) = self.text_edit.take() {
            let mut keep = true;
            egui::Modal::new(egui::Id::new("edit-text")).show(ctx, |ui| {
                ui.set_width(400.);
                ui.heading("Edit text");
                let r = ui.add(
                    egui::TextEdit::multiline(&mut value)
                        .desired_rows(7)
                        .desired_width(f32::INFINITY),
                );
                if !r.has_focus() {
                    r.request_focus();
                }
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        self.editor.edit("Edit text", |e| {
                            if let Some(mut c) = e
                                .current()
                                .controls
                                .into_iter()
                                .find(|c| c.id == id && !c.locked)
                            {
                                c.text = value.clone();
                                e.update_control(c);
                            }
                        });
                        keep = false;
                    }
                    if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                        keep = false;
                    }
                });
            });
            if keep {
                self.text_edit = Some((id, value));
            }
        }
        if self.pending.is_some() {
            egui::Modal::new(egui::Id::new("unsaved")).show(ctx, |ui| {
                ui.heading("Save your changes?");
                ui.label("This project has changes that haven't been saved.");
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
                        self.perform(p, ctx);
                    }
                    if ui.button("Cancel").clicked() {
                        self.pending = None;
                    }
                });
            });
        }
        if self.recovery_offer.is_some() {
            egui::Modal::new(egui::Id::new("recover")).show(ctx, |ui| {
                ui.heading("An unsaved project is available");
                ui.label("Restore the last local recovery copy as a new project.");
                ui.horizontal(|ui| {
                    if ui.button("Restore").clicked() {
                        let doc = self.recovery_offer.take().unwrap();
                        self.editor = Editor::default();
                        self.editor.edit("Restore recovery", |e| {
                            e.page = doc.pages[0].id;
                            e.document = doc;
                        });
                        self.file = None;
                    }
                    if ui.button("Start fresh").clicked() {
                        self.recovery_offer = None;
                        self.clear_recovery();
                    }
                });
            });
        }
        if let Some(error) = self.error.clone() {
            egui::Modal::new(egui::Id::new("error")).show(ctx, |ui| {
                ui.heading("Couldn't complete that action");
                ui.label(error);
                if ui.button("OK").clicked() {
                    self.error = None;
                }
            });
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
        egui::Window::new("Working with Wireframe").open(&mut self.help).default_width(470.).show(ctx,|ui|{ui.label("Copyright 2026 Wireframe contributors · GNU GPL v3.0 only · No warranty.");if ui.button("View license").clicked(){self.about=true;}ui.separator();ui.label("Everything stays on your computer. Save editable projects as .wireframe files; export the current page as PNG or SVG.");ui.separator();for (keys,what)in [("Click / Shift-click","Select one or several controls"),("Drag empty canvas","Select an area"),("Drag a selection","Move controls"),("Eight resize handles","Resize; hold Shift for aspect ratio"),("Arrow / Shift+Arrow","Move by 1 / 10 units"),("Double-click / Enter","Edit selected control text"),("⌘/Ctrl Z · Shift Z","Undo · Redo"),("⌘/Ctrl D · G · Shift G","Duplicate · Group · Ungroup"),("Space + drag","Pan the canvas"),("⌘/Ctrl + scroll","Zoom around the pointer")]{ui.horizontal(|ui|{ui.strong(keys);ui.label(what);});}ui.separator();ui.label("Groups move and resize together. Double-click a group to edit its children; Escape or Exit group returns to its parent. Images are embedded in the project. Rotate or crop them from the inspector. Click page links in Present mode.");});
    }
}
impl eframe::App for WireframeApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.sync_recovery(true);
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.image_import_dialog(ui.ctx());
        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.viewport().close_requested()) && !self.quit_allowed {
            self.editor.commit();
            if self.editor.dirty() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.pending = Some(Pending::Quit);
            } else if self.recovery_offer.is_none() {
                self.clear_recovery();
            }
        }
        if self.pending.is_none()
            && self.text_edit.is_none()
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
        self.shortcuts(&ctx);
        let cropping = self.image_crop.is_some();
        self.toolbar(ui);
        if !self.present {
            if self.show_library {
                self.library(ui);
            }
            if self.show_navigator {
                self.sidebar(ui);
            }
            if cropping {
                self.crop_controls(ui);
            } else if self.show_inspector {
                self.inspector(ui);
            }
        }
        self.canvas(ui);
        if let Some(error) = self.editor.error.take() {
            self.error = Some(error);
        }
        self.dialogs(&ctx);
        self.sync_recovery(false);
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
        assert!(app.presentation_history.is_empty());
        app.follow_target("back:", &ctx);
        assert_eq!(app.editor.page, first);
        app.follow_target("page:999999", &ctx);
        assert_eq!(app.editor.page, first);
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
}
