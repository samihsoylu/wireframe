// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::sync::Arc;
#[cfg(test)]
use wireframe::model::Kind;
use wireframe::{
    document::{Project, Settings},
    render::Scene,
    workspace::{AssetPreviews, CatalogItem, ComponentPreviews, Editor},
};

pub(super) struct LibraryPreviews {
    settings: Option<Settings>,
    font: Option<std::sync::Arc<wireframe::fonts::ProjectFont>>,
    scenes: Vec<(CatalogItem, Result<Arc<Scene>, String>)>,
    components: ComponentPreviews,
    assets: AssetPreviews,
    clock: fn() -> wireframe::tablet::Clock,
}
impl Default for LibraryPreviews {
    fn default() -> Self {
        Self {
            settings: None,
            font: None,
            scenes: vec![],
            components: ComponentPreviews::default(),
            assets: AssetPreviews::default(),
            clock: wireframe::tablet::Clock::now,
        }
    }
}
impl LibraryPreviews {
    pub(super) fn retained_asset_scene(
        &mut self,
        project: &Project,
        id: wireframe::document::Id,
    ) -> Result<Arc<Scene>, String> {
        self.assets.retained_scene(project, id)
    }
    pub(super) fn item_scene(
        &mut self,
        item: CatalogItem,
        project: &Project,
    ) -> Result<Arc<Scene>, String> {
        match item {
            CatalogItem::Component(id) => self.components.scene_with_clock(project, id, self.clock),
            CatalogItem::Asset(id) => self.assets.scene(project, id),
            CatalogItem::Control(_) | CatalogItem::Preset(_) | CatalogItem::Icon(_) => {
                self.control_scene(item, &project.settings, &project.font)
                    .map_err(str::to_owned)?;
                self.scenes
                    .iter()
                    .find(|(k, _)| *k == item)
                    .unwrap()
                    .1
                    .clone()
            }
        }
    }
    #[cfg(test)]
    pub(super) fn scene(
        &mut self,
        kind: Kind,
        settings: &Settings,
        font: &Option<std::sync::Arc<wireframe::fonts::ProjectFont>>,
    ) -> Result<&Scene, &str> {
        self.control_scene(CatalogItem::Control(kind), settings, font)
    }
    fn control_scene(
        &mut self,
        item: CatalogItem,
        settings: &Settings,
        font: &Option<std::sync::Arc<wireframe::fonts::ProjectFont>>,
    ) -> Result<&Scene, &str> {
        if self.settings.as_ref() != Some(settings) || &self.font != font {
            self.scenes.clear();
            self.settings = Some(settings.clone());
            self.font = font.clone();
        }
        let index = if let Some(index) = self.scenes.iter().position(|(k, _)| *k == item) {
            // Promote the hit so the bounded cache retains visible/held items.
            let entry = self.scenes.remove(index);
            self.scenes.push(entry);
            self.scenes.len() - 1
        } else {
            // Use the insertion path in an isolated document so catalog defaults,
            // automatic sizing and migrated families agree with actual insertion.
            let mut editor = Editor::with_clock(
                Project {
                    settings: settings.clone(),
                    font: font.clone(),
                    ..Project::default()
                },
                self.clock,
            );
            match item {
                CatalogItem::Control(kind) => editor.add(kind, 0., 0.),
                CatalogItem::Preset(preset) => editor.add_preset(preset, 0., 0.),
                CatalogItem::Icon(id) => {
                    if let Err(error) = editor.insert_catalog_icon(id, 0., 0.) {
                        editor.error = Some(error);
                    }
                }
                CatalogItem::Component(_) | CatalogItem::Asset(_) => {
                    unreachable!("project resource previews have their own cache")
                }
            }
            let drawing = if let Some(error) = editor.error.take() {
                Err(error)
            } else {
                editor.rendered_scene(editor.page)
            };
            // Builtins and individual icons share this bounded LRU. Artwork
            // paths are static catalog data; scenes contain only small recipes.
            const MAX_SCENES: usize = 128;
            if self.scenes.len() >= MAX_SCENES {
                let _ = self.scenes.remove(0);
            }
            self.scenes.push((item, drawing.map(Arc::new)));
            self.scenes.len() - 1
        };
        self.scenes[index].1.as_deref().map_err(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_library_preview_matches_new_control_in_both_styles() {
        let mut previews = LibraryPreviews::default();
        for sketch in [false, true] {
            let mut project = Project::default();
            project.settings.sketch = sketch;
            let first = previews
                .item_scene(CatalogItem::Control(Kind::Toggle), &project)
                .unwrap();
            let mut editor = Editor::new(project);
            editor.add(Kind::Toggle, 0., 0.);
            assert_eq!(
                first.svg(),
                editor.rendered_scene(editor.page).unwrap().svg()
            );
            assert_eq!((first.bounds.w, first.bounds.h), (42., 24.));
            assert!(!Kind::Toggle.text_editable());
        }
    }
    #[test]
    fn icon_previews_match_insertion_and_cache_is_bounded_and_refreshes_settings() {
        let mut previews = LibraryPreviews::default();
        let mut project = Project::default();
        let item = CatalogItem::Icon("star-regular");
        for sketch in [false, true] {
            project.settings.sketch = sketch;
            let first = previews.item_scene(item, &project).unwrap();
            let mut editor = Editor::new(project.clone());
            editor.insert_catalog_icon("star-regular", 0., 0.).unwrap();
            assert_eq!(
                first.svg(),
                editor.rendered_scene(editor.page).unwrap().svg()
            );
            assert!(Arc::ptr_eq(
                &first,
                &previews.item_scene(item, &project).unwrap()
            ));
            for icon in wireframe::icons::all().iter().take(150) {
                previews
                    .item_scene(CatalogItem::Icon(&icon.id), &project)
                    .unwrap();
            }
            assert_eq!(previews.scenes.len(), 128);
            assert!(!Arc::ptr_eq(
                &first,
                &previews.item_scene(item, &project).unwrap()
            ));
            let refreshed = previews.item_scene(item, &project).unwrap();
            project.settings.font_size += 1.;
            assert!(!Arc::ptr_eq(
                &refreshed,
                &previews.item_scene(item, &project).unwrap()
            ));
            assert_eq!(previews.scenes.len(), 1);
        }
        assert!(
            previews
                .item_scene(CatalogItem::Icon("invalid"), &project)
                .is_err()
        );
    }

    #[test]
    fn every_library_preview_matches_actual_insertion_in_each_style() {
        fn frozen() -> wireframe::tablet::Clock {
            wireframe::tablet::Clock::new(9, 52).unwrap()
        }
        let mut previews = LibraryPreviews {
            clock: frozen,
            ..Default::default()
        };
        for (font, sketch) in [None, Some(wireframe::fonts::default_project_font())]
            .into_iter()
            .flat_map(|font| [true, false].map(|sketch| (font.clone(), sketch)))
        {
            let settings = Settings {
                palette: Default::default(),
                sketch,
                font_size: 13.,
            };
            for kind in Kind::ALL {
                let mut editor = Editor::with_clock(
                    Project {
                        settings: settings.clone(),
                        font: font.clone(),
                        ..Project::default()
                    },
                    frozen,
                );
                editor.add(kind, 0., 0.);
                let expected = editor.rendered_scene(editor.page).unwrap();
                let actual = previews.scene(kind, &settings, &font).unwrap();
                assert!(
                    actual.svg() == expected.svg(),
                    "{kind:?}, sketch={sketch}, embedded_font={}",
                    font.is_some()
                );
                assert!(actual.bounds.w > 0. && actual.bounds.h > 0.);
            }
        }
    }
    #[test]
    fn preset_previews_match_insertion_and_do_not_share_base_cache_entries() {
        let mut previews = LibraryPreviews::default();
        for sketch in [false, true] {
            let mut project = Project::default();
            project.settings.sketch = sketch;
            for preset in wireframe::workspace::CatalogPreset::ALL {
                let mut editor = Editor::new(project.clone());
                editor.add_preset(preset, 0., 0.);
                let item = CatalogItem::Preset(preset);
                let actual = previews.item_scene(item, &project).unwrap();
                assert_eq!(
                    actual.svg(),
                    editor.rendered_scene(editor.page).unwrap().svg(),
                    "{preset:?}"
                );
                let base = previews
                    .item_scene(CatalogItem::Control(preset.kind()), &project)
                    .unwrap();
                assert!(!Arc::ptr_eq(&actual, &base));
                assert_ne!(
                    actual.svg(),
                    base.svg(),
                    "{preset:?} must change visible output"
                );
                assert!(Arc::ptr_eq(
                    &actual,
                    &previews.item_scene(item, &project).unwrap()
                ));
            }
        }
    }
    #[test]
    fn previews_refresh_for_font_defaults_and_drawing_style() {
        let mut previews = LibraryPreviews::default();
        let mut settings = Settings {
            palette: Default::default(),
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
    #[test]
    fn palette_changes_refresh_cached_library_links() {
        let mut previews = LibraryPreviews::default();
        let mut settings = Project::default().settings;
        let before = previews.scene(Kind::Link, &settings, &None).unwrap().svg();
        settings.palette.link = [139, 0, 139];
        let after = previews.scene(Kind::Link, &settings, &None).unwrap().svg();
        assert_ne!(before, after);
        assert!(after.contains("#8b008b"));
        settings.palette = Default::default();
        assert_eq!(
            previews.scene(Kind::Link, &settings, &None).unwrap().svg(),
            before
        );
    }
}
