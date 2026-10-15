// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;
use wireframe::document::Palette;

#[derive(Default)]
pub(super) struct PaletteDefaults {
    pub value: Palette,
    path: Option<PathBuf>,
}
impl PaletteDefaults {
    fn load(&mut self, root: &std::path::Path) -> Result<(), String> {
        let path = root.join("appearance-settings.json");
        self.path = Some(path.clone());
        match std::fs::metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.to_string()),
            Ok(m) if m.len() > 4096 => return Err("Color settings exceed the size limit".into()),
            Ok(_) => {}
        }
        self.value = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    fn save(&mut self, value: Palette) -> Result<(), String> {
        self.value = value;
        if let Some(path) = &self.path {
            storage::write_atomic(
                path,
                &serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
            )?;
        }
        Ok(())
    }
}

pub(super) struct ColorDraft {
    rgb: [u8; 3],
    hex: String,
}
impl ColorDraft {
    pub fn new(rgb: [u8; 3]) -> Self {
        Self {
            rgb,
            hex: format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]),
        }
    }
    pub fn value(&self) -> Option<[u8; 3]> {
        let text = self.hex.trim().strip_prefix('#').unwrap_or(self.hex.trim());
        if text.len() != 6 || !text.bytes().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let n = u32::from_str_radix(text, 16).ok()?;
        Some([(n >> 16) as u8, (n >> 8) as u8, n as u8])
    }
    pub fn show(&mut self, ui: &mut egui::Ui, label: &str) {
        ui.push_id(label, |ui| {
            ui.horizontal(|ui| {
                ui.label(label);
                if ui.color_edit_button_srgb(&mut self.rgb).changed() {
                    self.hex =
                        format!("#{:02X}{:02X}{:02X}", self.rgb[0], self.rgb[1], self.rgb[2]);
                }
                if ui
                    .add(TextInput::singleline(&mut self.hex).desired_width(110.))
                    .changed()
                    && let Some(value) = self.value()
                {
                    self.rgb = value;
                }
            })
        });
    }
}

impl WireframeApp {
    pub(super) fn load_palette_defaults(&mut self, root: &std::path::Path) {
        if let Err(error) = self.palette_defaults.load(root) {
            self.error = Some(format!(
                "New-project colors couldn't be read. The settings file is unchanged.\n{error}"
            ));
        }
    }
    pub(super) fn new_editor(&self) -> Editor {
        let mut document = Project::default();
        document.settings.palette = self.palette_defaults.value;
        Editor::new(document)
    }
    pub(super) fn apply_project_palette(&mut self, palette: Palette) {
        self.editor.edit("Change project colors", |e| {
            e.document.settings.palette = palette
        });
        self.status = "Colors applied".into();
        if let Err(error) = self.palette_defaults.save(palette) {
            self.error = Some(format!(
                "Project colors changed, but defaults for new projects couldn't be saved.\n{error}"
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn custom() -> Palette {
        Palette {
            selection: [32; 3],
            link: [139, 0, 139],
        }
    }
    #[test]
    fn palette_history_and_new_project_defaults_have_separate_lifetimes() {
        let root = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.load_palette_defaults(root.path());
        let original = app.editor.document.clone();
        app.apply_project_palette(custom());
        assert!(app.editor.dirty());
        assert_eq!(app.new_editor().document.settings.palette, custom());
        assert!(!app.new_editor().dirty());
        app.editor.undo();
        assert_eq!(app.editor.document, original);
        assert!(!app.editor.dirty());
        assert_eq!(app.new_editor().document.settings.palette, custom());
        app.editor.redo();
        assert_eq!(app.editor.document.settings.palette, custom());
        let mut reopened = WireframeApp::workspace(&ctx, None);
        reopened.load_palette_defaults(root.path());
        assert_eq!(reopened.new_editor().document.settings.palette, custom());
        let legacy = root.path().join("legacy.wireframe");
        ProjectStore.save(&legacy, &original).unwrap();
        reopened.font_import = Some(fonts::FontImport::default());
        assert!(reopened.load(legacy));
        assert!(reopened.font_import.is_none());
        assert_eq!(reopened.editor.document, original);
        assert_eq!(reopened.new_editor().document.settings.palette, custom());
    }
    #[test]
    fn defaults_write_failure_keeps_project_change_and_invalid_settings_are_preserved() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("appearance-settings.json");
        std::fs::write(&path, b"broken settings").unwrap();
        let ctx = egui::Context::default();
        let mut app = WireframeApp::workspace(&ctx, None);
        app.load_palette_defaults(root.path());
        assert!(app.error.as_deref().unwrap().contains("couldn't be read"));
        assert_eq!(std::fs::read(&path).unwrap(), b"broken settings");
        assert_eq!(app.palette_defaults.value, Palette::default());
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        app.apply_project_palette(custom());
        assert_eq!(app.editor.document.settings.palette, custom());
        assert!(app.error.as_deref().unwrap().contains("couldn't be saved"));
        app.editor.undo();
        assert_eq!(app.editor.document.settings.palette, Palette::default());
    }
    #[test]
    fn color_drafts_validate_without_mutating_the_document() {
        let mut draft = ColorDraft::new([149, 188, 242]);
        assert_eq!(draft.value(), Some([149, 188, 242]));
        for (text, expected) in [
            (" #20aBcD ", Some([32, 171, 205])),
            ("202020", Some([32; 3])),
            ("#fff", None),
            ("#GG0022", None),
            ("#1234567", None),
            ("日本", None),
        ] {
            draft.hex = text.into();
            assert_eq!(draft.value(), expected);
        }
    }
}
