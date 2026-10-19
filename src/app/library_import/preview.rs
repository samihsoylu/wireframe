// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::{render::Scene, workspace::LibraryImportEntry};

pub(super) struct Pair {
    pub(super) existing: Result<Scene, String>,
    pub(super) incoming: Result<Scene, String>,
}
impl Review {
    pub(super) fn prepare_preview(&mut self, destination: &Editor, entry: &LibraryImportEntry) {
        if self
            .preview
            .as_ref()
            .is_some_and(|(id, _)| *id == entry.incoming)
        {
            return;
        }
        let incoming = self.preview_editor.get_or_insert_with(|| {
            let mut project = self.clipboard.library_resources()?.preview_project()?;
            project.settings = destination.document.settings.clone();
            project.font = destination.document.font.clone();
            Ok(Editor::new(project))
        });
        let incoming = match incoming {
            Ok(editor) => editor.rendered_scene(entry.incoming),
            Err(error) => Err(error.clone()),
        };
        let existing = entry
            .existing
            .ok_or_else(|| "Existing library is unavailable".into())
            .and_then(|id| destination.rendered_scene(id));
        self.preview = Some((entry.incoming, Pair { existing, incoming }));
    }
}
impl Pair {
    pub(super) fn show(&self, ui: &mut egui::Ui) {
        let width = ((ui.available_width() - ui.spacing().item_spacing.x) / 2.).max(1.);
        let height = (ui.ctx().content_rect().height() * 0.38).clamp(100., 330.);
        ui.horizontal_top(|ui| {
            for (title, preview) in [("Existing", &self.existing), ("New", &self.incoming)] {
                ui.vertical(|ui| {
                    ui.set_width(width);
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Image,
                            true,
                            format!("{title} library preview"),
                        )
                    });
                    ui.painter().rect_filled(rect, 0, Color32::WHITE);
                    ui.painter().rect_stroke(
                        rect,
                        0,
                        Stroke::new(1., Color32::from_gray(180)),
                        egui::StrokeKind::Inside,
                    );
                    let content = rect.shrink(12.);
                    match preview {
                        Ok(scene) if !scene.items.is_empty() => {
                            let b = scene.bounds;
                            let scale =
                                (content.width() / b.w.max(1.)).min(content.height() / b.h.max(1.));
                            let origin = content.center()
                                - Vec2::new(b.w, b.h) * scale / 2.
                                - Vec2::new(b.x, b.y) * scale;
                            WireframeApp::paint_scene(
                                &ui.painter().with_clip_rect(content),
                                scene,
                                origin,
                                scale,
                            );
                        }
                        Ok(_) => {
                            ui.painter().text(
                                content.center(),
                                egui::Align2::CENTER_CENTER,
                                "Empty library",
                                egui::FontId::proportional(14.),
                                Color32::GRAY,
                            );
                        }
                        Err(error) => {
                            ui.painter().text(
                                content.center(),
                                egui::Align2::CENTER_CENTER,
                                "Preview unavailable",
                                egui::FontId::proportional(14.),
                                Color32::DARK_GRAY,
                            );
                            response.on_hover_text(error);
                        }
                    }
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        ui.label(title);
                    });
                });
            }
        });
    }
}
