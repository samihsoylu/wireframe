// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::fonts::ProjectFont;

#[derive(Default)]
pub(super) struct FontImport {
    name: String,
    faces: [Option<Vec<u8>>; 4],
    filenames: [String; 4],
    notices: String,
    error: Option<String>,
}

impl WireframeApp {
    pub(super) fn font_dialog(&mut self, ctx: &egui::Context) {
        let Some(draft) = &mut self.font_import else {
            return;
        };
        let mut open = true;
        let mut apply = None;
        let mut reset = false;
        let mut cancel = false;
        egui::Window::new("Project font")
            .open(&mut open)
            .resizable(false)
            .default_width(440.)
            .show(ctx, |ui| {
                ui.label(format!(
                    "Current: {}",
                    self.editor
                        .document
                        .font
                        .as_ref()
                        .map_or("Comic Neue", |f| f.name())
                ));
                ui.label(
                    "Imported fonts are saved in this project and used for its canvas and exports.",
                );
                if self.editor.document.font.is_some()
                    && ui.button("Use bundled Comic Neue").clicked()
                {
                    reset = true;
                }
                ui.separator();
                ui.label("Choose the four font faces (TTF or OTF, up to 4 MB each).");
                for (i, label) in ["Regular", "Bold", "Italic", "Bold italic"]
                    .into_iter()
                    .enumerate()
                {
                    ui.horizontal(|ui| {
                        if ui.button(format!("Choose {label}…")).clicked()
                            && let Some(path) = rfd::FileDialog::new()
                                .set_title(format!("Choose {label} font"))
                                .add_filter("Font face", &["ttf", "otf"])
                                .pick_file()
                        {
                            match read_face(&path) {
                                Ok((name, bytes)) => {
                                    if i == 0 {
                                        draft.name = name;
                                    }
                                    draft.faces[i] = Some(bytes);
                                    draft.filenames[i] = path
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .into_owned();
                                    draft.error = None;
                                }
                                Err(error) => draft.error = Some(error),
                            }
                        }
                        ui.label(if draft.filenames[i].is_empty() {
                            "No file chosen"
                        } else {
                            &draft.filenames[i]
                        });
                    });
                }
                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.add(TextInput::singleline(&mut draft.name));
                });
                ui.collapsing("Font notices", |ui| {
                    ui.label("Optional notices to retain with the project and SVG exports.");
                    ui.add(TextInput::multiline(&mut draft.notices).desired_rows(4));
                });
                if let Some(error) = &draft.error {
                    ui.colored_label(Color32::DARK_RED, error);
                }
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            draft.faces.iter().all(Option::is_some),
                            egui::Button::new("Apply font"),
                        )
                        .clicked()
                    {
                        let faces = draft.faces.each_ref().map(|f| f.as_ref().unwrap().clone());
                        match ProjectFont::new(draft.name.clone(), faces, draft.notices.clone()) {
                            Ok(font) => apply = Some(std::sync::Arc::new(font)),
                            Err(error) => draft.error = Some(error),
                        }
                    }
                    cancel = ui.button("Cancel").clicked();
                });
            });
        if apply.is_some() || reset {
            self.editor
                .edit("Change project font", |e| e.document.font = apply);
            self.status = "Project font changed".into();
            open = false;
        }
        if !open || cancel {
            self.font_import = None;
        }
    }
}

fn read_face(path: &std::path::Path) -> Result<(String, Vec<u8>), String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(wireframe::fonts::MAX_FACE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let name = ProjectFont::inspect_face(&bytes)?;
    Ok((name, bytes))
}
