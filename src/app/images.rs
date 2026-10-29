// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::{
    document::{Crop, Properties, Value},
    images::Prepared,
    workspace::{ImageImportTarget, ImageTarget, ImportChoice},
};

use super::crop::{CropEdit, CropPreview};

impl WireframeApp {
    pub(super) fn start_image_split(&mut self, id: Id) -> Result<(), String> {
        let edit = (|| {
            let (full, _, crop) = self.editor.image_split_geometry(id)?;
            Ok::<_, String>(CropEdit {
                id,
                full,
                original_full: full,
                crop: crop.unwrap_or(wireframe::document::Crop::ALL),
                preview: CropPreview::Group(self.editor.image_split_preview(id)?),
                background: self.editor.crop_background(id)?,
                drag: None,
                split: Some(super::crop::SplitPreview {
                    fraction: self.image_split_fraction,
                    grab: None,
                    session: self.editor.clipboard_session().into(),
                    surface: self.editor.surface_id(),
                    context: self.editor.context.clone(),
                    original: self
                        .editor
                        .nodes()
                        .iter()
                        .find(|n| n.id == id)
                        .cloned()
                        .ok_or("Image no longer exists")?,
                }),
            })
        })()?;
        self.end_pointer_gestures();
        self.image_crop = Some(edit);
        Ok(())
    }

    pub(super) fn shared_image_properties(&mut self, ui: &mut egui::Ui) {
        let Some(ids) = self.editor.selected_image_ids() else {
            return;
        };
        if self.editor.editing_instance()
            && self.component_section_header(ui, &ids, wireframe::workspace::PropertySection::Image)
        {
            return;
        }
        let sources: Vec<_> = ids
            .iter()
            .map(|id| {
                self.editor
                    .control_properties(*id)
                    .and_then(|(_, p)| match p.get("image") {
                        Some(Value::Asset(id)) => Some(*id),
                        _ => None,
                    })
            })
            .collect();
        let labels: Vec<_> = sources
            .iter()
            .map(|source| {
                source.map_or("Default placeholder".into(), |id| {
                    self.editor
                        .document
                        .live_asset(id)
                        .map_or("Unavailable image".into(), |a| a.name.clone())
                })
            })
            .collect();
        let caption = if labels.iter().all(|label| label == &labels[0]) {
            labels[0].clone()
        } else {
            labels.join(" / ")
        };
        ui.label("Image source");
        let mut chosen = None;
        ui_text::combo_box(ui, "shared-image-source", &caption, |ui| {
            if ui_text::selectable(
                ui,
                sources.iter().all(Option::is_none),
                "Default placeholder",
                ui.available_width(),
            )
            .clicked()
            {
                chosen = Some(None);
                ui.close();
            }
            for asset in self.editor.document.live_assets() {
                if wireframe::images::dimensions(&asset.bytes).is_ok()
                    && ui_text::selectable(
                        ui,
                        sources.iter().all(|source| *source == Some(asset.id)),
                        &asset.name,
                        ui.available_width(),
                    )
                    .clicked()
                {
                    chosen = Some(Some(asset.id));
                    ui.close();
                }
            }
        });
        if let Some(source) = chosen
            && let Err(error) = self.editor.assign_images(&ids, source)
        {
            self.error = Some(error);
        }
        if ui.button("Import image…").clicked() {
            self.start_image_import(ImageImportTarget::Controls(ids.clone()));
        }
        if ui.button("Import from Web…").clicked() {
            self.start_web_image(ids.clone());
        }
        if ui.button("Search images…").clicked() {
            self.start_image_search(ids.clone());
        }
        for source in sources
            .iter()
            .flatten()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
        {
            if let Some(credit) = self
                .editor
                .document
                .live_asset(source)
                .and_then(|a| a.attribution.as_ref())
            {
                ui.collapsing("Image credit", |ui| {
                    ui.label(credit.credit());
                    ui.hyperlink_to("Source & usage terms", &credit.source_url);
                    if ui.button("Copy image credit").clicked() {
                        ui.ctx().copy_text(credit.credit());
                    }
                });
            }
        }
        if sources.iter().any(Option::is_some) {
            ui.horizontal(|ui| {
                if ui.button("Rotate images 90°").clicked()
                    && let Err(error) = self.editor.rotate_images(&ids)
                {
                    self.error = Some(error);
                }
                if ui.button("Remove images").clicked()
                    && let Err(error) = self.editor.assign_images(&ids, None)
                {
                    self.error = Some(error);
                }
            });
        }
    }
    pub(super) fn image_properties(
        &mut self,
        ui: &mut egui::Ui,
        id: Id,
        props: &Properties,
        standalone: bool,
    ) {
        if self.editor.editing_instance()
            && self.component_section_header(
                ui,
                &[id],
                wireframe::workspace::PropertySection::Image,
            )
        {
            return;
        }
        let mut selected = match props.get("image") {
            Some(Value::Asset(id)) => Some(*id),
            _ => None,
        };
        let before = selected;

        ui.add_space(10.);
        ui.scope(|ui| {
            ui.label("Image source");
            let name = selected
                .and_then(|id| self.editor.document.live_asset(id))
                .map(|a| a.name.as_str())
                .unwrap_or(if selected.is_some() {
                    "Unavailable image"
                } else {
                    "Default placeholder"
                });
            ui_text::combo_box(ui, "image-source", name, |ui| {
                if ui_text::selectable_value(ui, &mut selected, None, "Default placeholder")
                    .clicked()
                {
                    ui.close();
                }
                for a in self.editor.document.live_assets() {
                    if wireframe::images::dimensions(&a.bytes).is_ok()
                        && ui_text::selectable_value(ui, &mut selected, Some(a.id), &a.name)
                            .clicked()
                    {
                        ui.close();
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
            if ui.button("Import from Web…").clicked() {
                self.start_web_image(vec![id]);
            }
            if ui.button("Search images…").clicked() {
                self.start_image_search(vec![id]);
            }
            if let Some(credit) = selected
                .and_then(|asset| self.editor.document.live_asset(asset))
                .and_then(|a| a.attribution.as_ref())
            {
                ui.collapsing("Image credit", |ui| {
                    ui.label(credit.credit());
                    ui.hyperlink_to("Source & usage terms", &credit.source_url);
                    if ui.button("Copy image credit").clicked() {
                        ui.ctx().copy_text(credit.credit());
                    }
                });
            }
            if selected.is_some()
                && ui.button("Remove image").clicked()
                && let Err(e) = self.editor.assign_image(id, None)
            {
                self.error = Some(e);
            }
            if standalone
                && ui
                    .add_enabled(
                        self.editor.image_split_geometry(id).is_ok(),
                        egui::Button::new("Split image"),
                    )
                    .clicked()
                && let Err(error) = self.start_image_split(id)
            {
                self.error = Some(error);
            }
            if selected.is_some() {
                if ui.button("Rotate 90°").clicked()
                    && let Err(e) = self.editor.rotate_image(id)
                {
                    self.error = Some(e);
                }
                if standalone
                    && ui
                        .add_enabled(!self.editor.own_locked(id), egui::Button::new("Crop image"))
                        .clicked()
                {
                    let result = (|| {
                        let (full, _, crop) = self.editor.image_geometry(id)?;
                        let asset = self
                            .editor
                            .document
                            .live_assets()
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
                            original_full: full,
                            preview: CropPreview::Image(image),
                            background: self.editor.crop_background(id)?,
                            drag: None,
                            split: None,
                        })
                    })();
                    match result {
                        Ok(edit) => {
                            self.end_pointer_gestures();
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
                    && ui
                        .add_enabled(
                            !self.editor.own_locked(id),
                            egui::Button::new("Remove crop"),
                        )
                        .clicked()
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
            ui.small("Images are stored in this project.");
        });
    }
    pub(super) fn start_image_import(&mut self, target: impl Into<ImageImportTarget>) {
        let target = target.into();
        self.editor.commit();
        if let Some(path) = self.menus.with_dialog(|| {
            rfd::FileDialog::new()
                .add_filter("Images", &["png", "jpg", "jpeg", "gif"])
                .pick_file()
        }) {
            self.import_image_file(target, &path);
        }
    }
    fn import_image_file(&mut self, target: ImageImportTarget, path: &std::path::Path) {
        match Prepared::read(path) {
            Ok(image) => {
                let conflict = self
                    .editor
                    .image_conflict(&image)
                    .and_then(|id| self.editor.document.live_asset(id));
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
    pub(super) fn image_import_dialog(&mut self, ctx: &egui::Context) {
        self.web_image_dialog(ctx);
        if let Some(id) = self.image_source_request.take() {
            self.start_image_import(ImageTarget::Control(id));
        }
        let Some((_, image)) = &self.image_import else {
            return;
        };
        let name = image.name.clone();
        let mut choice = None;
        let mut cancel = false;
        let response = egui::Modal::new(egui::Id::new("image-import-conflict")).show(ctx, |ui| {
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
        if response.is_top_modal
            && !response.any_popup_open
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            cancel = true;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use wireframe::document::{Node, Point, Size};
    fn picture(color: u8) -> Prepared {
        let image = image::RgbaImage::from_pixel(40, 20, image::Rgba([color, 30, 90, 180]));
        let mut bytes = Cursor::new(vec![]);
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        Prepared::from_bytes("picture.png", &bytes.into_inner()).unwrap()
    }
    fn app(ctx: &egui::Context) -> WireframeApp {
        let mut app = WireframeApp::workspace(ctx, None);
        app.editor.current_mut().content.nodes = ["image", "image", "cover_flow"]
            .into_iter()
            .enumerate()
            .map(|(i, kind)| {
                Node::control(
                    i as u64 + 2,
                    kind,
                    Point::new(50. + i as f32 * 200., 50.),
                    Size::fixed(160., 100.),
                )
            })
            .collect();
        app.editor.document.next_id = 5;
        app.editor.select_all();
        app.editor
            .import_image_at(
                ImageImportTarget::Controls(vec![2, 3, 4]),
                picture(20),
                ImportChoice::KeepBoth,
            )
            .unwrap();
        app
    }
    #[test]
    fn unreadable_and_corrupt_local_images_preserve_work_and_actual_redo() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("Missing.png");
        let corrupt = dir.path().join("Broken.png");
        // A recognized PNG signature with an incomplete body exercises codec
        // diagnostics rather than the actionable unsupported-format branch.
        let broken = b"\x89PNG\r\n\x1a\n";
        std::fs::write(&corrupt, broken).unwrap();
        for path in [&missing, &corrupt] {
            let raw = if path == &missing {
                std::fs::read(path).unwrap_err().to_string()
            } else {
                Prepared::from_bytes("Broken.png", broken).unwrap_err()
            };
            let ctx = egui::Context::default();
            let mut app = app(&ctx);
            app.editor.move_by(20., 0.);
            let moved = app.editor.document.clone();
            app.editor.undo();
            let before = app.editor.document.clone();
            let redo = app.editor.redo_name().map(str::to_owned);
            app.import_image_file(ImageTarget::Control(2).into(), path);
            let error = app.error.as_deref().unwrap();
            let (primary, details) = error.split_once("\n\nDetails: ").unwrap();
            assert!(primary.contains("Couldn't import this image"));
            assert!(primary.contains("choose another image"));
            assert_eq!(details, raw);
            assert_eq!(app.editor.document, before);
            assert_eq!(app.editor.redo_name(), redo.as_deref());
            assert!(app.image_import.is_none());
            app.editor.redo();
            assert_eq!(app.editor.document, moved);
        }
        assert_eq!(std::fs::read(corrupt).unwrap(), broken);
        assert!(!missing.exists());
    }
    #[test]
    fn valid_local_image_file_still_imports_as_one_reversible_action() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Local new image.png");
        let image = picture(140);
        std::fs::write(&path, &image.bytes).unwrap();
        let ctx = egui::Context::default();
        let mut app = app(&ctx);
        let before = app.editor.document.clone();
        app.import_image_file(ImageTarget::Control(2).into(), &path);
        assert!(app.error.is_none() && app.image_import.is_none());
        assert_eq!(app.editor.document.assets.len(), before.assets.len() + 1);
        assert_eq!(
            app.editor.document.assets.last().unwrap().bytes,
            image.bytes
        );
        assert_eq!(app.editor.undo_name(), Some("Import image"));
        let after = app.editor.document.clone();
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        app.editor.redo();
        assert_eq!(app.editor.document, after);
    }
    fn frame(
        app: &mut WireframeApp,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(420., 900.))),
                events,
                ..Default::default()
            },
            |ui| {
                app.shortcuts(ui.ctx());
                app.selection_properties(ui);
                app.image_import_dialog(ui.ctx());
            },
        );
        out.textures_delta.clear();
        out
    }
    fn click(app: &mut WireframeApp, ctx: &egui::Context, out: &egui::FullOutput, label: &str) {
        let r = out
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some(label) && n.role() == egui::accesskit::Role::Button)
            .unwrap_or_else(|| panic!("missing {label}"))
            .1
            .bounds()
            .unwrap();
        let at = Pos2::new((r.x0 + r.x1) as f32 / 2., (r.y0 + r.y1) as f32 / 2.);
        for pressed in [true, false] {
            frame(
                app,
                ctx,
                vec![
                    egui::Event::PointerMoved(at),
                    egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Modifiers::NONE,
                    },
                ],
            );
        }
    }
    #[test]
    fn image_source_names_stay_inside_the_inspector_and_keep_full_accessible_values() {
        for shared in [false, true] {
            for density in [1., 2.] {
                let ctx = egui::Context::default();
                ctx.enable_accesskit();
                ctx.set_pixels_per_point(density);
                let mut app = app(&ctx);
                let name = "واجهة שלום 👩‍💻 long-image-name ".repeat(20);
                app.editor.document.assets[0].name = name.clone();
                let before = app.editor.document.clone();
                let props = app.editor.control_properties(2).unwrap().1.clone();
                for _ in 0..3 {
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(Rect::from_min_size(
                                Pos2::ZERO,
                                Vec2::new(300., 900.),
                            )),
                            ..Default::default()
                        },
                        |ui| {
                            if shared {
                                app.shared_image_properties(ui);
                            } else {
                                app.image_properties(ui, 2, &props, true);
                            }
                        },
                    );
                    output.textures_delta.clear();
                    let nodes = &output
                        .platform_output
                        .accesskit_update
                        .as_ref()
                        .unwrap()
                        .nodes;
                    let combo = nodes
                        .iter()
                        .find(|(_, n)| n.role() == egui::accesskit::Role::ComboBox)
                        .unwrap();
                    assert_eq!(combo.1.value(), Some(name.as_str()));
                    let bounds = combo.1.bounds().unwrap();
                    assert!(
                        bounds.x0 >= 0. && bounds.x1 <= 300. * density as f64 + 1.,
                        "shared={shared} density={density} picker={bounds:?}"
                    );
                    output.textures_delta.clear();
                }
                assert_eq!(app.editor.document, before);
            }
        }
    }
    #[test]
    fn shared_source_choice_closes_popup_and_keeps_noop_redo() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = app(&ctx);
        app.editor.assign_images(&[2, 3, 4], None).unwrap();
        let removed = app.editor.document.clone();
        app.editor.undo();
        let before = app.editor.document.clone();
        frame(&mut app, &ctx, vec![]);
        for choose in [false, true] {
            let out = frame(&mut app, &ctx, vec![]);
            let nodes = &out.platform_output.accesskit_update.as_ref().unwrap().nodes;
            let node = nodes
                .iter()
                .find(|(_, n)| {
                    if choose {
                        n.label() == Some("picture.png")
                            && n.role() == egui::accesskit::Role::Button
                    } else {
                        n.role() == egui::accesskit::Role::ComboBox
                    }
                })
                .unwrap_or_else(|| {
                    panic!(
                        "missing choice={choose}: {:?}",
                        nodes
                            .iter()
                            .map(|(_, n)| (n.role(), n.label(), n.value()))
                            .collect::<Vec<_>>()
                    )
                });
            let r = node.1.bounds().unwrap();
            let at = Pos2::new((r.x0 + r.x1) as f32 / 2., (r.y0 + r.y1) as f32 / 2.);
            for pressed in [true, false] {
                frame(
                    &mut app,
                    &ctx,
                    vec![
                        egui::Event::PointerMoved(at),
                        egui::Event::PointerButton {
                            pos: at,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Modifiers::NONE,
                        },
                    ],
                );
            }
            assert_eq!(egui::Popup::is_any_open(&ctx), !choose);
        }
        assert_eq!(app.editor.document, before);
        app.editor.redo();
        assert_eq!(app.editor.document, removed);
    }
    #[test]
    fn shared_image_buttons_rotate_and_remove_all_with_exact_undo() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = app(&ctx);
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        let before = app.editor.document.clone();
        click(&mut app, &ctx, &out, "Rotate images 90°");
        for id in [2, 3, 4] {
            assert_eq!(
                app.editor.control_properties(id).unwrap().1["rotation"],
                Value::Number(270.)
            );
        }
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        let out = frame(&mut app, &ctx, vec![]);
        click(&mut app, &ctx, &out, "Remove images");
        for id in [2, 3, 4] {
            assert_eq!(
                app.editor.control_properties(id).unwrap().1["image"],
                Value::Null
            );
        }
        assert_eq!(app.editor.document.assets, before.assets);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(
            !out.platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .any(|(_, n)| matches!(
                    n.label(),
                    Some("Rotate images 90°" | "Remove images" | "Sketch filter")
                ))
        );
        app.editor.undo();
        assert_eq!(app.editor.document, before);
    }
    #[test]
    fn shared_import_conflict_cancel_preserves_redo_and_accepts_one_asset_atomically() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = app(&ctx);
        app.editor.assign_images(&[2, 3, 4], None).unwrap();
        let removed = app.editor.document.clone();
        app.editor.undo();
        let before = app.editor.document.clone();
        app.image_import = Some((ImageImportTarget::Controls(vec![2, 3, 4]), picture(160)));
        assert!(app.canvas_input_blocked());
        frame(&mut app, &ctx, vec![]);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
        assert!(app.image_import.is_none());
        assert_eq!(app.editor.ordered_selection(), [2, 3, 4]);
        assert_eq!(app.editor.document, before);
        app.editor.redo();
        assert_eq!(app.editor.document, removed);
        app.editor.undo();
        app.image_import = Some((ImageImportTarget::Controls(vec![2, 3, 4]), picture(160)));
        frame(&mut app, &ctx, vec![]);
        let out = frame(&mut app, &ctx, vec![]);
        click(&mut app, &ctx, &out, "Keep both");
        assert!(app.image_import.is_none());
        assert_eq!(app.editor.document.assets.len(), 2);
        let new = app.editor.document.assets.last().unwrap().id;
        for id in [2, 3, 4] {
            assert_eq!(
                app.editor.control_properties(id).unwrap().1["image"],
                Value::Asset(new)
            );
        }
        let after = app.editor.document.clone();
        app.editor.undo();
        assert_eq!(app.editor.document, before);
        app.editor.redo();
        assert_eq!(app.editor.document, after);
    }
}
