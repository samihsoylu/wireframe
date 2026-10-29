// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
};
use wireframe::{
    document::{ImageAttribution, Project},
    images::{
        search::{Hit, Results},
        web::Preview,
    },
    workspace::ImportChoice,
};

const MAX_NETWORK_WORKERS: usize = 4;
static NETWORK_WORKERS: AtomicUsize = AtomicUsize::new(0);
struct WorkerSlot(&'static AtomicUsize);
impl WorkerSlot {
    fn acquire(counter: &'static AtomicUsize) -> Option<Self> {
        let mut active = counter.load(Ordering::Relaxed);
        loop {
            if active >= MAX_NETWORK_WORKERS {
                return None;
            }
            match counter.compare_exchange_weak(
                active,
                active + 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some(Self(counter)),
                Err(current) => active = current,
            }
        }
    }
}
impl Drop for WorkerSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}
fn network_available() -> bool {
    NETWORK_WORKERS.load(Ordering::Relaxed) < MAX_NETWORK_WORKERS
}

pub(super) struct Review {
    ids: Vec<Id>,
    session: String,
    surface: Id,
    context: Vec<Id>,
    document: Project,
    address: String,
    search_mode: bool,
    query: String,
    hits: Vec<Hit>,
    thumbnails: Vec<Option<wireframe::images::Decoded>>,
    thumbnail_receiver: Option<mpsc::Receiver<(usize, Result<wireframe::images::Decoded, String>)>>,
    next: Option<u64>,
    searched: bool,
    search_receiver: Option<mpsc::Receiver<Result<Results, String>>>,
    search_cancel: Arc<AtomicBool>,
    attribution: Option<ImageAttribution>,
    sketch: bool,
    preview: Option<Preview>,
    receiver: Option<mpsc::Receiver<Result<Preview, String>>>,
    cancel: Arc<AtomicBool>,
    error: Option<String>,
    conflict: bool,
}

#[cfg(test)]
mod tests;
impl Drop for Review {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.search_cancel.store(true, Ordering::Relaxed);
    }
}
impl Review {
    fn valid(&self, editor: &Editor) -> bool {
        self.session == editor.clipboard_session()
            && self.surface == editor.surface_id()
            && self.context == editor.context
            && self.document == editor.document
    }
    fn reset_preview(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.receiver = None;
        self.preview = None;
        self.attribution = None;
        self.error = None;
        self.conflict = false;
    }
    fn search(&mut self, ctx: &egui::Context, offset: Option<u64>) {
        self.reset_preview();
        self.search_cancel.store(true, Ordering::Relaxed);
        self.search_cancel = Arc::new(AtomicBool::new(false));
        self.hits.clear();
        self.thumbnails.clear();
        self.thumbnail_receiver = None;
        self.next = None;
        self.searched = false;
        let Some(slot) = WorkerSlot::acquire(&NETWORK_WORKERS) else {
            self.error =
                Some("Previous image requests are finishing. Try again in a moment.".into());
            return;
        };
        let (sender, receiver) = mpsc::channel();
        let query = self.query.clone();
        let cancel = self.search_cancel.clone();
        let ctx = ctx.clone();
        match std::thread::Builder::new()
            .name("image-search".into())
            .spawn(move || {
                let _slot = slot;
                let result = wireframe::images::search::search(&query, offset, &cancel);
                let _ = sender.send(result);
                ctx.request_repaint();
            }) {
            Ok(_) => self.search_receiver = Some(receiver),
            Err(_) => self.error = Some("Cannot start image search".into()),
        }
    }
    fn fetch(&mut self, ctx: &egui::Context) {
        self.cancel.store(true, Ordering::Relaxed);
        self.cancel = Arc::new(AtomicBool::new(false));
        self.preview = None;
        self.error = None;
        self.conflict = false;
        self.receiver = None;
        let Some(slot) = WorkerSlot::acquire(&NETWORK_WORKERS) else {
            self.error =
                Some("Previous image requests are finishing. Try again in a moment.".into());
            return;
        };
        let (sender, receiver) = mpsc::channel();
        let address = self.address.clone();
        let cancel = self.cancel.clone();
        let ctx = ctx.clone();
        match std::thread::Builder::new()
            .name("image-download".into())
            .spawn(move || {
                let _slot = slot;
                let result = wireframe::images::web::fetch(&address, &cancel);
                let _ = sender.send(result);
                ctx.request_repaint();
            }) {
            Ok(_) => self.receiver = Some(receiver),
            Err(_) => self.error = Some("Cannot start the image download".into()),
        }
    }
    fn search_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.label("Search Wikimedia Commons");
        ui.small("Your search is sent to Wikimedia. No project or image is uploaded.");
        let query = ui.add(
            egui::TextEdit::singleline(&mut self.query)
                .hint_text("For example: Amsterdam canal")
                .desired_width(f32::INFINITY),
        );
        if query.changed() {
            self.search_cancel.store(true, Ordering::Relaxed);
            self.search_receiver = None;
            self.hits.clear();
            self.thumbnails.clear();
            self.thumbnail_receiver = None;
            self.next = None;
            self.searched = false;
            self.reset_preview();
        }
        let enter = query.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        if (ui
            .add_enabled(
                network_available()
                    && self.search_receiver.is_none()
                    && !self.query.trim().is_empty(),
                egui::Button::new("Search"),
            )
            .clicked()
            || enter)
            && !self.query.trim().is_empty()
        {
            self.search(ctx, None);
        }
        if self.search_receiver.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Searching…");
            });
        }
        if self.searched && self.hits.is_empty() && self.error.is_none() {
            ui.label("No supported images on this page. Try another search or the next results.");
        }
        if !self.hits.is_empty() {
            ui.small("Select an image to preview it.");
        }
        let mut selected = None;
        egui::ScrollArea::vertical()
            .id_salt("image-search-results")
            .max_height(190.)
            .show(ui, |ui| {
                let columns = if ui.available_width() >= 460. { 2 } else { 1 };
                let card_width = ui.available_width().min(225.);
                egui::Grid::new("search-image-grid")
                    .num_columns(columns)
                    .spacing([12., 10.])
                    .show(ui, |ui| {
                        for (index, hit) in self.hits.iter().enumerate() {
                            ui.vertical(|ui| {
                                ui.set_width(card_width);
                                if let Some(Some(image)) = self.thumbnails.get(index) {
                                    let size = image.pixels.size;
                                    let ratio = (card_width.min(220.) / size[0] as f32)
                                        .min(110. / size[1] as f32)
                                        .min(1.);
                                    if ui
                                        .add(
                                            egui::Image::new((
                                                image.texture(ctx),
                                                Vec2::new(size[0] as f32, size[1] as f32) * ratio,
                                            ))
                                            .sense(egui::Sense::click()),
                                        )
                                        .clicked()
                                    {
                                        selected = Some(index);
                                    }
                                } else {
                                    ui.allocate_ui(Vec2::new(card_width.min(220.), 60.), |ui| {
                                        ui.weak("Image preview");
                                    });
                                }
                                let title: String =
                                    hit.attribution.title.chars().take(75).collect();
                                if ui
                                    .selectable_label(
                                        self.attribution.as_ref() == Some(&hit.attribution),
                                        title,
                                    )
                                    .on_hover_text(&hit.attribution.title)
                                    .clicked()
                                {
                                    selected = Some(index);
                                }
                                ui.small(format!(
                                    "{} × {} · {}",
                                    hit.width,
                                    hit.height,
                                    hit.attribution.license.chars().take(75).collect::<String>()
                                ));
                            });
                            if index % columns == columns - 1 {
                                ui.end_row();
                            }
                        }
                    });
            });
        if let Some(index) = selected {
            let hit = self.hits[index].clone();
            self.address = hit.address;
            self.attribution = Some(hit.attribution);
            self.fetch(ctx);
        }
        if let Some(next) = self.next
            && ui
                .add_enabled(
                    network_available() && self.search_receiver.is_none(),
                    egui::Button::new("Next results"),
                )
                .clicked()
        {
            self.search(ctx, Some(next));
        }
    }
    fn address_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.label("Image address");
        let address = ui.add(
            egui::TextEdit::singleline(&mut self.address)
                .hint_text("https://…")
                .desired_width(f32::INFINITY),
        );
        if address.changed() {
            self.reset_preview();
        }
        if ui
            .add_enabled(
                network_available() && self.receiver.is_none() && !self.address.trim().is_empty(),
                egui::Button::new("Fetch"),
            )
            .clicked()
        {
            self.fetch(ctx);
        }
    }
    fn start_thumbnails(&mut self, ctx: &egui::Context) {
        self.thumbnails = (0..self.hits.len()).map(|_| None).collect();
        let (sender, receiver) = mpsc::channel();
        for group in 0..2 {
            let jobs: Vec<_> = self
                .hits
                .iter()
                .enumerate()
                .filter(|(index, _)| index % 2 == group)
                .filter_map(|(index, hit)| hit.thumbnail.clone().map(|url| (index, url)))
                .collect();
            if jobs.is_empty() {
                continue;
            }
            let Some(slot) = WorkerSlot::acquire(&NETWORK_WORKERS) else {
                continue;
            };
            let sender = sender.clone();
            let cancel = self.search_cancel.clone();
            let ctx = ctx.clone();
            let _ = std::thread::Builder::new()
                .name("search-thumbnails".into())
                .spawn(move || {
                    let _slot = slot;
                    for (index, address) in jobs {
                        if cancel.load(Ordering::Relaxed) {
                            break;
                        }
                        let result = wireframe::images::search::thumbnail(&address, &cancel);
                        if sender.send((index, result)).is_err() {
                            break;
                        }
                        ctx.request_repaint();
                    }
                });
        }
        self.thumbnail_receiver = Some(receiver);
    }
    fn poll(&mut self, ctx: &egui::Context) {
        if let Some(receiver) = &self.search_receiver {
            match receiver.try_recv() {
                Ok(result) => {
                    self.search_receiver = None;
                    self.searched = true;
                    match result {
                        Ok(results) => {
                            self.hits = results.hits;
                            self.next = results.next;
                            self.start_thumbnails(ctx);
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.search_receiver = None;
                    self.error = Some("Image search stopped. Try Search again.".into());
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if let Some(receiver) = &self.thumbnail_receiver {
            loop {
                match receiver.try_recv() {
                    Ok((index, image)) => {
                        if let Some(thumbnail) = self.thumbnails.get_mut(index) {
                            *thumbnail = image.ok();
                        }
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.thumbnail_receiver = None;
                        break;
                    }
                }
            }
        }
        let Some(receiver) = &self.receiver else {
            return;
        };
        match receiver.try_recv() {
            Ok(result) => {
                self.receiver = None;
                match result {
                    Ok(mut preview) => {
                        preview.image.attribution = self.attribution.clone();
                        self.preview = Some(preview);
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.receiver = None;
                self.error = Some("The image download stopped. Try Fetch again.".into());
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
}
impl WireframeApp {
    pub(super) fn start_web_image(&mut self, ids: Vec<Id>) {
        self.end_pointer_gestures();
        self.editor.commit();
        self.web_image = Some(Review {
            ids,
            session: self.editor.clipboard_session().into(),
            surface: self.editor.surface_id(),
            context: self.editor.context.clone(),
            document: self.editor.document.clone(),
            address: String::new(),
            search_mode: false,
            query: String::new(),
            hits: Vec::new(),
            thumbnails: Vec::new(),
            thumbnail_receiver: None,
            next: None,
            searched: false,
            search_receiver: None,
            search_cancel: Arc::new(AtomicBool::new(false)),
            attribution: None,
            sketch: false,
            preview: None,
            receiver: None,
            cancel: Arc::new(AtomicBool::new(false)),
            error: None,
            conflict: false,
        });
    }
    pub(super) fn start_image_search(&mut self, ids: Vec<Id>) {
        self.start_web_image(ids);
        self.web_image.as_mut().unwrap().search_mode = true;
    }
    pub(super) fn web_image_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut review) = self.web_image.take() else {
            return;
        };
        if !review.valid(&self.editor) {
            self.error = Some("The project changed. Open image import again.".into());
            return;
        }
        review.poll(ctx);
        let mut cancel = false;
        let mut apply = None;
        let response = egui::Modal::new(egui::Id::new("web-image-import")).show(ctx, |ui| {
            ui.set_width(500_f32.min((ctx.viewport_rect().width() - 60.).max(240.)));
            egui::ScrollArea::vertical().id_salt("web-import-body").max_height((ctx.viewport_rect().height() - 190.).max(140.)).show(ui, |ui| {
            ui.heading("Import from Web");
            let old_mode = review.search_mode;
            ui.horizontal(|ui| {
                ui.selectable_value(&mut review.search_mode, false, "Image address");
                ui.selectable_value(&mut review.search_mode, true, "Search images");
            });
            if !network_available() {
                ui.small("Image requests are finishing. Please wait…");
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            if old_mode != review.search_mode {
                review.reset_preview();
                review.search_cancel.store(true, Ordering::Relaxed);
                review.search_receiver = None;
                review.thumbnail_receiver = None;
            }
            if review.search_mode { review.search_ui(ui, ctx); }
            else { review.address_ui(ui, ctx); }
            if review.receiver.is_some() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Fetching image…");
                });
            }
            if let Some(preview) = &review.preview {
                let image = if review.sketch {
                    &preview.sketch
                } else {
                    &preview.original
                };
                let size = image.pixels.size;
                let ratio = (ui.available_width().min(480.) / size[0] as f32).min(if review.search_mode { 170. } else { 260. } / size[1] as f32).min(1.);
                ui.add(egui::Image::new((
                    image.texture(ctx),
                    Vec2::new(size[0] as f32, size[1] as f32) * ratio,
                )));
                let natural = preview.original.pixels.size;
                ui.label(format!("{} × {}", natural[0], natural[1]));
                ui.checkbox(&mut review.sketch, "Sketch it");
                if let Some(credit) = &review.attribution {
                    ui.label(format!("{} · {}", if credit.creator.is_empty() { "See source for creator".into() } else { credit.creator.chars().take(160).collect::<String>() }, credit.license.chars().take(75).collect::<String>()));
                    ui.horizontal(|ui| {
                        ui.hyperlink_to("Source & usage terms", &credit.source_url);
                        if ui.button("Copy image credit").clicked() { ui.ctx().copy_text(credit.credit()); }
                    });
                    ui.small("Credit is kept with this image in your project. Include it when sharing exports.");
                }
            }
            });
            if let Some(error) = &review.error {
                ui.colored_label(Color32::DARK_RED, error);
            }
            if review.conflict {
                ui.label("An image with this name already exists.");
                ui.label("Use new replaces it everywhere in this project.");
                ui.horizontal(|ui| {
                    if ui.button("Use new").clicked() {
                        apply = Some(ImportChoice::Replace);
                    }
                    if ui.button("Use existing").clicked() {
                        apply = Some(ImportChoice::UseExisting);
                    }
                    if ui.button("Keep both").clicked() {
                        apply = Some(ImportChoice::KeepBoth);
                    }
                });
            } else if ui
                .add_enabled(review.preview.is_some(), egui::Button::new("Load"))
                .clicked()
            {
                let image = &review.preview.as_ref().unwrap().image;
                if self
                    .editor
                    .image_conflict(image)
                    .and_then(|id| self.editor.document.live_asset(id))
                    .is_some_and(|a| image.conflicts_with(a))
                {
                    review.conflict = true;
                } else {
                    apply = Some(ImportChoice::UseExisting);
                }
            }
            cancel = ui.button("Cancel").clicked();
        });
        cancel |= response.is_top_modal
            && !response.any_popup_open
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        if cancel {
            return;
        }
        if let Some(choice) = apply {
            let image = review.preview.take().unwrap().image;
            if let Err(error) =
                self.editor
                    .import_web_image(review.ids.clone(), image, choice, review.sketch)
            {
                self.error = Some(error);
            }
        } else {
            self.web_image = Some(review);
        }
    }
}
