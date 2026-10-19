// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::document::{Crop, Point};

pub(super) enum CropPreview {
    Image(wireframe::images::Decoded),
    Group(wireframe::render::Scene),
}
#[derive(Clone, Copy)]
enum Gesture {
    Resize(i8, i8),
    Region,
    Content,
}
#[derive(Clone, Copy)]
pub(super) struct CropDrag {
    gesture: Gesture,
    start: Pos2,
    crop: Crop,
    full: Bounds,
}
pub(super) struct CropEdit {
    pub(super) id: Id,
    pub(super) full: Bounds,
    pub(super) original_full: Bounds,
    pub(super) crop: Crop,
    pub(super) preview: CropPreview,
    pub(super) background: wireframe::render::Scene,
    pub(super) drag: Option<CropDrag>,
    pub(super) split: Option<SplitPreview>,
}
pub(super) struct SplitPreview {
    pub(super) fraction: f32,
    pub(super) grab: Option<f32>,
    pub(super) session: String,
    pub(super) surface: Id,
    pub(super) context: Vec<Id>,
    pub(super) original: wireframe::document::Node,
}
impl SplitPreview {
    fn valid(&self, editor: &Editor) -> bool {
        self.session == editor.clipboard_session()
            && self.surface == editor.surface_id()
            && self.context == editor.context
            && editor.nodes().iter().any(|n| n == &self.original)
    }
}
impl CropEdit {
    fn bounds(&self) -> Bounds {
        Bounds::new(
            self.full.x + self.crop.x * self.full.w,
            self.full.y + self.crop.y * self.full.h,
            self.crop.width * self.full.w,
            self.crop.height * self.full.h,
        )
    }
    fn update_drag(&mut self, end: Pos2, zoom: f32) {
        let Some(drag) = self.drag else {
            return;
        };
        let delta = (end - drag.start) / zoom;
        let dx = delta.x.round() / drag.full.w;
        let dy = delta.y.round() / drag.full.h;
        self.full = drag.full;
        self.crop = drag.crop;
        let mut left = drag.crop.x;
        let mut top = drag.crop.y;
        let mut right = left + drag.crop.width;
        let mut bottom = top + drag.crop.height;
        let minimum = if matches!(self.preview, CropPreview::Group(_)) {
            10.
        } else {
            1.
        };
        let min_w = (minimum / self.full.w).min(1.);
        let min_h = (minimum / self.full.h).min(1.);
        match drag.gesture {
            Gesture::Content => {
                // Keep the frame fixed while shifting the full artwork behind it.
                let x = dx.clamp(right - 1., left);
                let y = dy.clamp(bottom - 1., top);
                self.full.x += x * self.full.w;
                self.full.y += y * self.full.h;
                left -= x;
                right -= x;
                top -= y;
                bottom -= y;
            }
            Gesture::Region => {
                left = (left + dx).clamp(0., 1. - drag.crop.width);
                top = (top + dy).clamp(0., 1. - drag.crop.height);
                right = left + drag.crop.width;
                bottom = top + drag.crop.height;
            }
            Gesture::Resize(x, y) => {
                if x < 0 {
                    left = (left + dx).clamp(0., (right - min_w).max(0.));
                }
                if x > 0 {
                    right = (right + dx).clamp((left + min_w).min(1.), 1.);
                }
                if y < 0 {
                    top = (top + dy).clamp(0., (bottom - min_h).max(0.));
                }
                if y > 0 {
                    bottom = (bottom + dy).clamp((top + min_h).min(1.), 1.);
                }
            }
        }
        self.crop = Crop {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        };
    }
    fn apply(self, editor: &mut Editor) -> Result<(), String> {
        if let Some(split) = self.split {
            if !split.valid(editor) {
                return Err("The image changed while splitting. Start the split again.".into());
            }
            return editor.split_image(self.id, split.fraction).map(|_| ());
        }
        match self.preview {
            CropPreview::Image(_) => editor.crop_image(self.id, Some(self.crop)),
            CropPreview::Group(_) if !editor.markup_visible() => {
                let bounds = self.bounds();
                let crop = Crop {
                    x: (self.crop.x * 1000.).round() / 1000.,
                    y: (self.crop.y * 1000.).round() / 1000.,
                    width: (self.crop.width * 1000.).round().max(1.) / 1000.,
                    height: (self.crop.height * 1000.).round().max(1.) / 1000.,
                };
                editor.crop_group_at(self.id, crop, Point::new(bounds.x, bounds.y))
            }
            CropPreview::Group(_) => editor.crop_group(
                self.id,
                Some(self.crop),
                Point::new(
                    self.full.x - self.original_full.x,
                    self.full.y - self.original_full.y,
                ),
            ),
        }
    }
}
impl WireframeApp {
    fn start_group_crop(&mut self, id: Id) -> Result<(), String> {
        self.end_pointer_gestures();
        let (full, visible, crop) = self.editor.group_crop_edit_geometry(id)?;
        let crop = crop.map(|crop| {
            if self.editor.markup_visible() {
                crop
            } else {
                Crop {
                    x: (visible.x - full.x) / full.w,
                    y: (visible.y - full.y) / full.h,
                    width: visible.w / full.w,
                    height: visible.h / full.h,
                }
            }
        });
        let preview = self.editor.group_crop_preview(id)?;
        self.image_crop = Some(CropEdit {
            id,
            full,
            original_full: full,
            crop: crop.unwrap_or(Crop {
                x: 0.,
                y: 0.,
                width: (full.w * 0.8).round().max(1.).min(full.w) / full.w,
                height: (full.h * 0.8).round().max(1.).min(full.h) / full.h,
            }),
            preview: CropPreview::Group(preview),
            background: self.editor.crop_background(id)?,
            drag: None,
            split: None,
        });
        Ok(())
    }
    pub(super) fn group_crop_properties(&mut self, ui: &mut egui::Ui, id: Id) {
        if !self.editor.is_group(id) && self.editor.instance_definition(id).is_none() {
            return;
        }
        let geometry = self.editor.group_crop_geometry(id);
        ui.add_space(10.);
        ui.add_enabled_ui(geometry.is_ok(), |ui| {
            if ui.button("Crop selection").clicked()
                && let Err(e) = self.start_group_crop(id)
            {
                self.error = Some(e);
            }
            if geometry.as_ref().is_ok_and(|(_, _, crop)| crop.is_some())
                && ui.button("Remove crop").clicked()
                && let Err(e) = self.editor.crop_group(id, None, Point::default())
            {
                self.error = Some(e);
            }
            if self.editor.has_local_crop(id)
                && ui.button("Reset crop to source").clicked()
                && let Err(e) = self.editor.reset_local_crop(id)
            {
                self.error = Some(e);
            }
        });
    }
    pub(super) fn crop_controls(&mut self, ui: &mut egui::Ui) {
        let splitting = self.image_crop.as_ref().is_some_and(|e| e.split.is_some());
        let mut apply = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        let mut cancel = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        chrome::prepare_panel_resize(
            ui,
            egui::Id::new("crop-inspector"),
            chrome::PanelEdge::Right,
            96. ..=f32::INFINITY,
            true,
        );
        egui::Panel::right("crop-inspector")
            .default_size(300.)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.heading(if splitting {
                    "Split image"
                } else {
                    "Crop selection"
                });
                ui.label(if splitting {
                    "Drag the line or click inside the image to set the split point."
                } else {
                    "Drag inside the frame to move it. Drag a handle to resize it."
                });
                if splitting {
                    let edit = self.image_crop.as_mut().unwrap();
                    let height = edit.bounds().h;
                    let split = edit.split.as_mut().unwrap();
                    let minimum = 10_f32.min(height / 2.);
                    let mut top = split.fraction * height;
                    if ui
                        .add(
                            egui::DragValue::new(&mut top)
                                .range(minimum..=height - minimum)
                                .prefix("Top height: ")
                                .suffix(" px"),
                        )
                        .changed()
                    {
                        split.fraction = top / height;
                    }
                    ui.small("The two pieces share the image, with a 20 px gap.");
                }
                if !splitting
                    && self
                        .image_crop
                        .as_ref()
                        .is_some_and(|e| matches!(e.preview, CropPreview::Group(_)))
                {
                    ui.label("Drag outside the frame to move the content behind it.");
                }
                ui.horizontal(|ui| {
                    apply |= ui
                        .button(if splitting {
                            "Apply split"
                        } else {
                            "Apply crop"
                        })
                        .clicked();
                    cancel |= ui
                        .button(if splitting {
                            "Cancel split"
                        } else {
                            "Cancel crop"
                        })
                        .clicked();
                });
                ui.small("Enter applies · Escape cancels");
                if let Some(edit) = &self.image_crop {
                    ui.label(format!(
                        "Visible size: {:.0} × {:.0}",
                        edit.bounds().w,
                        edit.bounds().h
                    ));
                }
            });
        if let Some(split) = self.image_crop.as_ref().and_then(|e| e.split.as_ref()) {
            self.image_split_fraction = split.fraction;
        }
        if cancel {
            self.image_crop = None;
        } else if apply
            && let Some(edit) = self.image_crop.take()
            && let Err(e) = edit.apply(&mut self.editor)
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
        let mut edit = self.image_crop.take().unwrap();
        if edit.split.as_ref().is_some_and(|s| !s.valid(&self.editor)) {
            self.error = Some("The image changed while splitting. Start the split again.".into());
            return;
        }
        let instant = Self::instant_drag(ui, response.rect);
        // Native input may coalesce press + movement, then movement + release.
        // Retain the press immediately; egui's drag_started may never be observable.
        let start = ui
            .input(|i| {
                i.events.iter().find_map(|event| match event {
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        ..
                    } => Some(*pos),
                    _ => None,
                })
            })
            .filter(|p| {
                ui.is_enabled()
                    && response.rect.contains(*p)
                    && ui.ctx().layer_id_at(*p) == Some(ui.layer_id())
            });
        if !ui.is_enabled() || !ui.input(|i| i.focused) {
            edit.drag = None;
        }
        let visible = self.screen_rect(edit.bounds());
        if let Some(split) = edit.split.as_mut() {
            let line = visible.top() + visible.height() * split.fraction;
            if !ui.is_enabled() || !ui.input(|i| i.focused) {
                split.grab = None;
            } else {
                if let Some(start) = start {
                    split.grab = (visible.contains(start) && (start.y - line).abs() <= 8.)
                        .then_some(start.y - line);
                }
                if let Some(point) = instant
                    .map(|(_, end)| end)
                    .or_else(|| ui.input(|i| i.pointer.interact_pos()))
                {
                    let offset = split
                        .grab
                        .or_else(|| (response.clicked() && visible.contains(point)).then_some(0.));
                    if let Some(offset) = offset {
                        let minimum = (10. * self.zoom).min(visible.height() / 2.);
                        split.fraction = ((point.y - offset - visible.top())
                            .clamp(minimum, visible.height() - minimum)
                            / visible.height())
                        .clamp(f32::EPSILON, 1. - f32::EPSILON);
                    }
                }
                if ui.input(|i| i.pointer.primary_released()) {
                    split.grab = None;
                }
            }
        } else if let Some(start) = start {
            let gesture = self
                .resize_handles(edit.bounds())
                .into_iter()
                .find(|(_, p)| p.distance(start) < 12.)
                .map(|((x, y), _)| Gesture::Resize(x, y))
                .or_else(|| {
                    self.screen_rect(edit.bounds())
                        .contains(start)
                        .then_some(Gesture::Region)
                })
                .or_else(|| {
                    matches!(edit.preview, CropPreview::Group(_)).then_some(Gesture::Content)
                });
            edit.drag = gesture.map(|gesture| CropDrag {
                gesture,
                start,
                crop: edit.crop,
                full: edit.full,
            });
        }
        if edit.split.is_none()
            && let Some(end) = instant
                .map(|(_, end)| end)
                .or_else(|| ui.input(|i| i.pointer.interact_pos()))
        {
            edit.update_drag(end, self.zoom);
        }
        if instant.is_some()
            || response.drag_stopped()
            || ui.input(|i| i.pointer.primary_released())
        {
            edit.drag = None;
        }
        Self::paint_scene(
            painter,
            &edit.background,
            self.canvas_rect.min + self.pan,
            self.zoom,
        );
        painter.rect_filled(response.rect, 0, Color32::from_black_alpha(95));
        let full = self.screen_rect(edit.full);
        painter.rect_filled(full, 0, Color32::WHITE);
        match &edit.preview {
            CropPreview::Image(image) => {
                painter.image(
                    image.texture(ui.ctx()),
                    full,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
                    Color32::WHITE,
                );
            }
            CropPreview::Group(scene) => {
                let delta = Vec2::new(
                    edit.full.x - edit.original_full.x,
                    edit.full.y - edit.original_full.y,
                );
                Self::paint_scene(
                    painter,
                    scene,
                    self.canvas_rect.min + self.pan + delta * self.zoom,
                    self.zoom,
                );
            }
        }
        let visible = self.screen_rect(edit.bounds());
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
        if let Some(split) = &edit.split {
            let y = visible.top() + visible.height() * split.fraction;
            painter.line_segment(
                [Pos2::new(visible.left(), y), Pos2::new(visible.right(), y)],
                Stroke::new(3., Color32::WHITE),
            );
            painter.line_segment(
                [Pos2::new(visible.left(), y), Pos2::new(visible.right(), y)],
                Stroke::new(1., Color32::BLACK),
            );
            painter.rect(
                Rect::from_center_size(Pos2::new(visible.center().x, y), Vec2::new(24., 8.)),
                2.,
                Color32::WHITE,
                Stroke::new(1., Color32::DARK_GRAY),
                StrokeKind::Inside,
            );
            if response
                .hover_pos()
                .is_some_and(|p| visible.contains(p) && (p.y - y).abs() < 8.)
                || split.grab.is_some()
            {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
            }
        } else {
            for (_, point) in self.resize_handles(edit.bounds()) {
                painter.rect(
                    Rect::from_center_size(point, Vec2::splat(7.)),
                    0,
                    Color32::WHITE,
                    Stroke::new(1., ACCENT),
                    StrokeKind::Inside,
                );
            }
        }
        if let Some(split) = &edit.split {
            self.image_split_fraction = split.fraction;
        }
        if response.double_clicked() && edit.split.is_none() {
            if let Err(e) = edit.apply(&mut self.editor) {
                self.error = Some(e);
            }
        } else {
            self.image_crop = Some(edit);
        }
    }
}

#[cfg(test)]
mod tests;
