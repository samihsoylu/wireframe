// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use wireframe::{
    document::{Project, Settings},
    model::Kind,
    render::Scene,
    workspace::Editor,
};

#[derive(Default)]
pub(super) struct LibraryPreviews {
    settings: Option<Settings>,
    font: Option<std::sync::Arc<wireframe::fonts::ProjectFont>>,
    scenes: Vec<(Kind, Result<Scene, String>)>,
}
impl LibraryPreviews {
    pub(super) fn scene(
        &mut self,
        kind: Kind,
        settings: &Settings,
        font: &Option<std::sync::Arc<wireframe::fonts::ProjectFont>>,
    ) -> Result<&Scene, &str> {
        if self.settings.as_ref() != Some(settings) || &self.font != font {
            self.scenes.clear();
            self.settings = Some(settings.clone());
            self.font = font.clone();
        }
        let index = if let Some(index) = self.scenes.iter().position(|(k, _)| *k == kind) {
            index
        } else {
            // Use the insertion path in an isolated document so catalog defaults,
            // automatic sizing and migrated families agree with actual insertion.
            let mut editor = Editor::new(Project {
                settings: settings.clone(),
                font: font.clone(),
                ..Project::default()
            });
            editor.add(kind, 0., 0.);
            let drawing = if let Some(error) = editor.error.take() {
                Err(error)
            } else {
                editor.rendered_scene(editor.page)
            };
            self.scenes.push((kind, drawing));
            self.scenes.len() - 1
        };
        self.scenes[index].1.as_ref().map_err(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_library_preview_matches_actual_insertion_in_each_style() {
        let mut previews = LibraryPreviews::default();
        for sketch in [true, false] {
            let settings = Settings {
                sketch,
                font_size: 13.,
            };
            for kind in Kind::ALL {
                let mut editor = Editor::new(Project {
                    settings: settings.clone(),
                    ..Project::default()
                });
                editor.add(kind, 0., 0.);
                let expected = editor.rendered_scene(editor.page).unwrap();
                let actual = previews.scene(kind, &settings, &None).unwrap();
                assert_eq!(actual.svg(), expected.svg(), "{kind:?}, sketch={sketch}");
                assert!(actual.bounds.w > 0. && actual.bounds.h > 0.);
            }
        }
    }
    #[test]
    fn previews_refresh_for_font_defaults_and_drawing_style() {
        let mut previews = LibraryPreviews::default();
        let mut settings = Settings {
            sketch: true,
            font_size: 13.,
        };
        assert_eq!(
            previews
                .scene(Kind::Checkbox, &settings, &None)
                .unwrap()
                .bounds
                .h,
            23.
        );
        settings.font_size = 24.;
        assert_eq!(
            previews
                .scene(Kind::Checkbox, &settings, &None)
                .unwrap()
                .bounds
                .h,
            34.
        );
        settings.font_size = 13.;
        let sketch = previews
            .scene(Kind::Button, &settings, &None)
            .unwrap()
            .svg();
        settings.sketch = false;
        let clean = previews
            .scene(Kind::Button, &settings, &None)
            .unwrap()
            .svg();
        assert_ne!(sketch, clean);
        assert_eq!(
            previews
                .scene(Kind::Button, &settings, &None)
                .unwrap()
                .bounds
                .h,
            24.
        );
        settings.sketch = true;
        assert_eq!(
            previews
                .scene(Kind::Button, &settings, &None)
                .unwrap()
                .svg(),
            sketch
        );
    }
}
