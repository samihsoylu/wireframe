// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::{
    document::{Crop, Properties, Value},
    images::Prepared,
    workspace::{ImageTarget, ImportChoice},
};

pub(super) struct CropEdit {
    id: Id,
    full: Bounds,
    crop: Crop,
    image: wireframe::images::Decoded,
    drag: Option<((i8, i8), Pos2, Crop)>,
}

impl WireframeApp {
    pub(super) fn image_properties(
        &mut self,
        ui: &mut egui::Ui,
        id: Id,
        props: &Properties,
        standalone: bool,
    ) {
        let mut selected = match props.get("image") {
            Some(Value::Asset(id)) => Some(*id),
            _ => None,
        };
        let before = selected;
        let locked = self
            .editor
            .current()
            .controls
            .iter()
            .find(|c| c.id == id)
            .is_none_or(|c| c.locked);
        ui.add_space(10.);
        ui.add_enabled_ui(!locked, |ui| {
            ui.label("Image source");
            let name = selected
                .and_then(|id| self.editor.document.assets.iter().find(|a| a.id == id))
                .map(|a| a.name.as_str())
                .unwrap_or("Default placeholder");
            egui::ComboBox::from_id_salt("image-source")
                .selected_text(name)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected, None, "Default placeholder");
                    for a in &self.editor.document.assets {
                        if wireframe::images::dimensions(&a.bytes).is_ok() {
                            ui.selectable_value(&mut selected, Some(a.id), &a.name);
                        }
                    }
                });
            if selected != before
                && let Err(e) = self.editor.assign_image(id, selected)
            {
                self.error = Some(e);
            }
            if ui.button("Import image…").clicked() {
                self.start_image_import(ImageTarget::Control(id));
            }
            if selected.is_some()
                && ui.button("Remove image").clicked()
                && let Err(e) = self.editor.assign_image(id, None)
            {
                self.error = Some(e);
            }
            if selected.is_some() {
                if ui.button("Rotate 90°").clicked()
                    && let Err(e) = self.editor.rotate_image(id)
                {
                    self.error = Some(e);
                }
                if standalone && ui.button("Crop image").clicked() {
                    let result = (|| {
                        let (full, _, crop) = self.editor.image_geometry(id)?;
                        let asset = self
                            .editor
                            .document
                            .assets
                            .iter()
                            .find(|a| Some(a.id) == selected)
                            .ok_or("Image unavailable")?;
                        let image = wireframe::images::decoded_styled(
                            &asset.bytes,
                            wireframe::images::quarter_turns(props),
                            props.get("sketch_filter") == Some(&Value::Bool(true)),
                        )?;
                        Ok::<_, String>(CropEdit {
                            id,
                            full,
                            crop: crop.unwrap_or(Crop {
                                x: 0.,
                                y: 0.,
                                width: 0.8,
                                height: 0.8,
                            }),
                            image,
                            drag: None,
                        })
                    })();
                    match result {
                        Ok(edit) => {
                            self.drag = None;
                            self.image_crop = Some(edit);
                        }
                        Err(e) => self.error = Some(e),
                    }
                }
                if standalone
                    && self
                        .editor
                        .nodes()
                        .iter()
                        .any(|n| n.id == id && n.crop.is_some())
                    && ui.button("Remove crop").clicked()
                    && let Err(e) = self.editor.crop_image(id, None)
                {
                    self.error = Some(e);
                }
            }
            if standalone && ui.button("Auto-size").clicked() {
                self.editor.begin("Auto-size image");
                if let Err(e) = self.editor.auto_size_control(id) {
                    self.error = Some(e);
                    self.editor.cancel();
                } else {
                    self.editor.commit();
                }
            }
            if self.editor.editing_instance()
                && self
                    .editor
                    .explicit_control_properties(id)
                    .contains_key("image")
                && ui.button("Reset image to source").clicked()
            {
                self.editor.begin("Reset image source");
                if let Err(e) = self.editor.reset_control_property(id, "image") {
                    self.error = Some(e);
                    self.editor.cancel();
                } else {
                    self.editor.commit();
                }
            }
            ui.small("Images are stored in this project.");
        });
    }
    pub(super) fn crop_controls(&mut self, ui: &mut egui::Ui) {
        let mut apply = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        let mut cancel = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        egui::Panel::right("crop-inspector")
            .default_size(300.)
            .show(ui, |ui| {
                ui.heading("Crop image");
                ui.label("Drag the handles to choose the visible part of the image.");
                ui.horizontal(|ui| {
                    apply |= ui.button("Apply crop").clicked();
                    cancel |= ui.button("Cancel crop").clicked();
                });
                ui.small("Enter applies · Escape cancels");
                if let Some(edit) = &self.image_crop {
                    ui.label(format!(
                        "Visible size: {:.0} × {:.0}",
                        edit.full.w * edit.crop.width,
                        edit.full.h * edit.crop.height
                    ));
                }
            });
        if cancel {
            self.image_crop = None;
        } else if apply
            && let Some(edit) = self.image_crop.take()
            && let Err(e) = self.editor.crop_image(edit.id, Some(edit.crop))
        {
            self.error = Some(e);
        }
    }
    pub(super) fn crop_canvas(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        painter: &egui::Painter,
    ) {
        if let Ok(scene) = self.editor.rendered_scene(self.editor.surface_id()) {
            Self::paint_scene(painter, &scene, self.canvas_rect.min + self.pan, self.zoom);
        }
        painter.rect_filled(response.rect, 0, Color32::from_black_alpha(95));
        let mut edit = self.image_crop.take().unwrap();
        let full = self.screen_rect(edit.full);
        painter.rect_filled(full, 0, Color32::WHITE);
        painter.image(
            edit.image.texture(ui.ctx()),
            full,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
            Color32::WHITE,
        );
        let bounds = |c: Crop| {
            Bounds::new(
                edit.full.x + c.x * edit.full.w,
                edit.full.y + c.y * edit.full.h,
                c.width * edit.full.w,
                c.height * edit.full.h,
            )
        };
        let instant = Self::instant_drag(ui, response.rect);
        let start = instant.map(|(start, _)| start).or_else(|| {
            if response.drag_started() {
                ui.input(|i| i.pointer.press_origin())
            } else {
                None
            }
        });
        if let Some(start) = start {
            if let Some((handle, _)) = self
                .resize_handles(bounds(edit.crop))
                .into_iter()
                .find(|(_, p)| p.distance(start) < 12.)
            {
                edit.drag = Some((handle, start, edit.crop));
            } else if self.screen_rect(bounds(edit.crop)).contains(start) {
                edit.drag = Some(((0, 0), start, edit.crop));
            }
        }
        if let Some((handle, start, initial)) = edit.drag
            && let Some(end) = instant
                .map(|(_, end)| end)
                .or_else(|| ui.input(|i| i.pointer.interact_pos()))
        {
            let delta = (end - start) / self.zoom;
            let dx = (delta.x.round() / edit.full.w).clamp(-1., 1.);
            let dy = (delta.y.round() / edit.full.h).clamp(-1., 1.);
            let mut left = initial.x;
            let mut top = initial.y;
            let mut right = left + initial.width;
            let mut bottom = top + initial.height;
            let min_w = (1. / edit.full.w).min(1.);
            let min_h = (1. / edit.full.h).min(1.);
            if handle == (0, 0) {
                left = (left + dx).clamp(0., 1. - initial.width);
                top = (top + dy).clamp(0., 1. - initial.height);
                right = left + initial.width;
                bottom = top + initial.height;
            } else {
                if handle.0 < 0 {
                    left = (left + dx).clamp(0., (right - min_w).max(0.));
                }
                if handle.0 > 0 {
                    right = (right + dx).clamp((left + min_w).min(1.), 1.);
                }
                if handle.1 < 0 {
                    top = (top + dy).clamp(0., (bottom - min_h).max(0.));
                }
                if handle.1 > 0 {
                    bottom = (bottom + dy).clamp((top + min_h).min(1.), 1.);
                }
            }
            edit.crop = Crop {
                x: left,
                y: top,
                width: right - left,
                height: bottom - top,
            };
        }
        if instant.is_some() || response.drag_stopped() {
            edit.drag = None;
        }
        let visible = self.screen_rect(bounds(edit.crop));
        for rect in [
            Rect::from_min_max(full.min, Pos2::new(full.max.x, visible.min.y)),
            Rect::from_min_max(Pos2::new(full.min.x, visible.max.y), full.max),
            Rect::from_min_max(
                Pos2::new(full.min.x, visible.min.y),
                Pos2::new(visible.min.x, visible.max.y),
            ),
            Rect::from_min_max(
                Pos2::new(visible.max.x, visible.min.y),
                Pos2::new(full.max.x, visible.max.y),
            ),
        ] {
            painter.rect_filled(rect, 0, Color32::from_black_alpha(100));
        }
        painter.rect_stroke(visible, 0, Stroke::new(1., ACCENT), StrokeKind::Inside);
        for (_, point) in self.resize_handles(bounds(edit.crop)) {
            painter.rect(
                Rect::from_center_size(point, Vec2::splat(7.)),
                0,
                Color32::WHITE,
                Stroke::new(1., ACCENT),
                StrokeKind::Inside,
            );
        }
        self.image_crop = Some(edit);
    }
    pub(super) fn start_image_import(&mut self, target: ImageTarget) {
        self.editor.commit();
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "gif"])
            .pick_file()
        {
            match Prepared::read(&path) {
                Ok(image) => {
                    let conflict = self
                        .editor
                        .image_conflict(&image)
                        .and_then(|id| self.editor.document.assets.iter().find(|a| a.id == id));
                    if conflict.is_some_and(|a| a.bytes != image.bytes) {
                        self.image_import = Some((target, image));
                    } else if let Err(e) =
                        self.editor
                            .import_image_at(target, image, ImportChoice::UseExisting)
                    {
                        self.error = Some(e);
                    }
                }
                Err(e) => self.error = Some(e),
            }
        }
    }
    pub(super) fn image_import_dialog(&mut self, ctx: &egui::Context) {
        let Some((_, image)) = &self.image_import else {
            return;
        };
        let name = image.name.clone();
        let mut choice = None;
        let mut cancel = false;
        egui::Modal::new(egui::Id::new("image-import-conflict")).show(ctx, |ui| {
            ui.heading("An image with this name already exists");
            ui.label(name);
            ui.label("Use new replaces the shared image everywhere in this project. Keep both adds a separate image.");
            ui.horizontal(|ui| {
                if ui.button("Use new").clicked() { choice = Some(ImportChoice::Replace); }
                if ui.button("Use existing").clicked() { choice = Some(ImportChoice::UseExisting); }
                if ui.button("Keep both").clicked() { choice = Some(ImportChoice::KeepBoth); }
                if ui.button("Cancel").clicked() { cancel = true; }
            });
        });
        if let Some(choice) = choice {
            let (target, image) = self.image_import.take().unwrap();
            if let Err(e) = self.editor.import_image_at(target, image, choice) {
                self.error = Some(e);
            }
        } else if cancel {
            self.image_import = None;
        }
    }
}
