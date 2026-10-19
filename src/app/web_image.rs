// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use wireframe::{document::Project, images::web::Preview, workspace::ImportChoice};

pub(super) struct Review {
    ids: Vec<Id>,
    session: String,
    surface: Id,
    context: Vec<Id>,
    document: Project,
    address: String,
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
    }
}
impl Review {
    fn valid(&self, editor: &Editor) -> bool {
        self.session == editor.clipboard_session()
            && self.surface == editor.surface_id()
            && self.context == editor.context
            && self.document == editor.document
    }
    fn fetch(&mut self, ctx: &egui::Context) {
        self.cancel.store(true, Ordering::Relaxed);
        self.cancel = Arc::new(AtomicBool::new(false));
        self.preview = None;
        self.error = None;
        self.conflict = false;
        let (sender, receiver) = mpsc::channel();
        let address = self.address.clone();
        let cancel = self.cancel.clone();
        let ctx = ctx.clone();
        match std::thread::Builder::new()
            .name("image-download".into())
            .spawn(move || {
                let result = wireframe::images::web::fetch(&address, &cancel);
                let _ = sender.send(result);
                ctx.request_repaint();
            }) {
            Ok(_) => self.receiver = Some(receiver),
            Err(_) => self.error = Some("Cannot start the image download".into()),
        }
    }
    fn poll(&mut self) {
        let Some(receiver) = &self.receiver else {
            return;
        };
        match receiver.try_recv() {
            Ok(result) => {
                self.receiver = None;
                match result {
                    Ok(preview) => self.preview = Some(preview),
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
            sketch: false,
            preview: None,
            receiver: None,
            cancel: Arc::new(AtomicBool::new(false)),
            error: None,
            conflict: false,
        });
    }
    pub(super) fn web_image_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut review) = self.web_image.take() else {
            return;
        };
        if !review.valid(&self.editor) {
            self.error = Some("The project changed. Open image import again.".into());
            return;
        }
        review.poll();
        let mut cancel = false;
        let mut apply = None;
        let response = egui::Modal::new(egui::Id::new("web-image-import")).show(ctx, |ui| {
            ui.set_width(460.);
            ui.heading("Import from Web");
            ui.label("Image address");
            let address = ui.add(
                egui::TextEdit::singleline(&mut review.address)
                    .hint_text("https://…")
                    .desired_width(f32::INFINITY),
            );
            if address.changed() {
                review.cancel.store(true, Ordering::Relaxed);
                review.receiver = None;
                review.preview = None;
                review.error = None;
                review.conflict = false;
            }
            if ui
                .add_enabled(
                    review.receiver.is_none() && !review.address.trim().is_empty(),
                    egui::Button::new("Fetch"),
                )
                .clicked()
            {
                review.fetch(ctx);
            }
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
                let ratio = (440. / size[0] as f32).min(260. / size[1] as f32).min(1.);
                ui.add(egui::Image::new((
                    image.texture(ctx),
                    Vec2::new(size[0] as f32, size[1] as f32) * ratio,
                )));
                let natural = preview.original.pixels.size;
                ui.label(format!("{} × {}", natural[0], natural[1]));
                ui.checkbox(&mut review.sketch, "Sketch it");
            }
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
                    .is_some_and(|a| a.bytes != image.bytes)
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
