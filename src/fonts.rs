// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use eframe::egui;
pub(crate) mod interface;
pub use interface::InterfaceRole;

pub fn panel_heading_font() -> egui::FontId {
    egui::FontId::new(13., InterfaceRole::Medium.family())
}
mod project;
pub use project::{MAX_FACE_BYTES, ProjectFont};
pub const REGULAR: &[u8] = include_bytes!("../assets/fonts/comic-neue/ComicNeue-Regular.ttf");
pub const BOLD: &[u8] = include_bytes!("../assets/fonts/comic-neue/ComicNeue-Bold.ttf");
pub const ITALIC: &[u8] = include_bytes!("../assets/fonts/comic-neue/ComicNeue-Italic.ttf");
pub const BOLD_ITALIC: &[u8] =
    include_bytes!("../assets/fonts/comic-neue/ComicNeue-BoldItalic.ttf");

/// New documents retain their font bytes, keeping their appearance stable when
/// opened on another computer or after a later bundled-font update.
pub fn default_project_font() -> std::sync::Arc<ProjectFont> {
    static FONT: std::sync::OnceLock<std::sync::Arc<ProjectFont>> = std::sync::OnceLock::new();
    FONT.get_or_init(|| {
        std::sync::Arc::new(
            ProjectFont::new(
                "Wireframe Draft".into(),
                [
                    include_bytes!("../assets/fonts/wireframe-draft/WireframeDraft-Regular.ttf")
                        .as_slice(),
                    include_bytes!("../assets/fonts/wireframe-draft/WireframeDraft-Bold.ttf")
                        .as_slice(),
                    include_bytes!("../assets/fonts/wireframe-draft/WireframeDraft-Italic.ttf")
                        .as_slice(),
                    include_bytes!("../assets/fonts/wireframe-draft/WireframeDraft-BoldItalic.ttf")
                        .as_slice(),
                ]
                .map(<[u8]>::to_vec),
                concat!(
                    include_str!("../assets/fonts/wireframe-draft/OFL.txt"),
                    "\n",
                    include_str!("../assets/fonts/wireframe-draft/MODIFICATIONS.txt")
                )
                .into(),
            )
            .expect("Bundled project font must be valid"),
        )
    })
    .clone()
}
pub fn family(bold: bool, italic: bool) -> egui::FontFamily {
    egui::FontFamily::Name(
        match (bold, italic) {
            (false, false) => "canvas-regular",
            (true, false) => "canvas-bold",
            (false, true) => "canvas-italic",
            (true, true) => "canvas-bold-italic",
        }
        .into(),
    )
}
pub fn install(context: &egui::Context) {
    install_project(context, None);
}
pub fn install_project(context: &egui::Context, project: Option<&ProjectFont>) {
    let mut fonts = egui::FontDefinitions::default();
    let fallback = fonts.families[&egui::FontFamily::Proportional].clone();
    for (index, bytes) in faces(project).into_iter().enumerate() {
        let bold = index % 2 == 1;
        let italic = index >= 2;
        let family = family(bold, italic);
        let name = family.to_string();
        fonts.font_data.insert(
            name.clone(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes.to_vec())),
        );
        let mut choices = vec![name];
        choices.extend(fallback.clone());
        fonts.families.insert(family, choices);
    }
    for role in [
        InterfaceRole::Regular,
        InterfaceRole::Medium,
        InterfaceRole::Bold,
    ] {
        let name = match role {
            InterfaceRole::Regular => "interface-system",
            InterfaceRole::Medium => "interface-medium",
            InterfaceRole::Bold => "interface-bold",
        };
        let mut choices = fonts.families[&egui::FontFamily::Proportional].clone();
        // The bundled proportional faces omit navigation arrows. Keep the
        // ordinary face first and use the already bundled symbol coverage last.
        if !choices.iter().any(|name| name == "Hack") {
            choices.push("Hack".into());
        }
        if let Some(interface) = interface::system_font(role) {
            let mut data = egui::FontData::from_owned(interface.bytes.as_ref().clone());
            data.index = interface.index;
            data.tweak.coords = egui::epaint::text::VariationCoords::new(interface.axes.clone());
            data.tweak.automatic_sizing = true;
            fonts
                .font_data
                .insert(name.into(), std::sync::Arc::new(data));
            choices.insert(0, name.into());
        }
        fonts.families.insert(role.family(), choices);
    }
    context.set_fonts(fonts);
}

pub fn faces(project: Option<&ProjectFont>) -> [&[u8]; 4] {
    project.map_or([REGULAR, BOLD, ITALIC, BOLD_ITALIC], ProjectFont::faces)
}
pub fn export_family(project: Option<&ProjectFont>) -> &'static str {
    if project.is_some() {
        "Project Font"
    } else {
        "Comic Neue"
    }
}
pub fn notices(project: Option<&ProjectFont>) -> &str {
    project.map_or(
        include_str!("../assets/fonts/comic-neue/OFL.txt"),
        ProjectFont::notices,
    )
}
/// Install explicit face roles, independent of imported font metadata or system fonts.
pub fn install_export(db: &mut resvg::usvg::fontdb::Database, project: Option<&ProjectFont>) {
    use resvg::usvg::fontdb::{Database, ID, Language, Style, Weight};
    // Explicit aliases prevent fontdb from choosing another face with a shared
    // family name, and preserve the canvas fallback's style and size.
    for (name, data) in egui::FontDefinitions::default().font_data {
        let mut parsed = Database::new();
        parsed.load_font_data(data.font.to_vec());
        if let Some(face) = parsed.faces().next() {
            let mut face = face.clone();
            face.id = ID::dummy();
            face.families = vec![(
                export_fallback_family(&name),
                Language::English_UnitedStates,
            )];
            face.weight = Weight(400);
            face.style = Style::Normal;
            db.push_face_info(face);
        }
    }
    for (index, bytes) in faces(project).into_iter().enumerate() {
        let mut parsed = Database::new();
        parsed.load_font_data(bytes.to_vec());
        if let Some(face) = parsed.faces().next() {
            let mut face = face.clone();
            face.id = ID::dummy();
            face.families = vec![(
                export_family(project).into(),
                Language::English_UnitedStates,
            )];
            face.weight = Weight(if index % 2 == 1 { 700 } else { 400 });
            face.style = if index >= 2 {
                Style::Italic
            } else {
                Style::Normal
            };
            db.push_face_info(face);
        }
    }
}

pub const FALLBACK_NOTICES: &str = include_str!("../assets/fonts/fallback-notices.txt");
pub fn export_fallback_family(face: &str) -> String {
    format!("Wireframe {face}")
}

/// Keep graphemes together and retain the actual face selected by the canvas.
/// The public character coverage map lists faces in the family's fallback order.
pub fn source_faces(
    context: &egui::Context,
    family: &egui::FontFamily,
    text: &str,
) -> Vec<(std::ops::Range<usize>, String, f32)> {
    use unicode_segmentation::UnicodeSegmentation;
    context.fonts_mut(|fonts| {
        let mut runs: Vec<(std::ops::Range<usize>, String, f32)> = Vec::new();
        {
            let mut font = fonts.fonts.font(family);
            let coverage = font.characters();
            let replacement = coverage
                .get(&'◻')
                .or_else(|| coverage.get(&'?'))
                .and_then(|v| v.first());
            for (start, grapheme) in text.grapheme_indices(true) {
                let first = grapheme.chars().next().unwrap_or(' ');
                let face = coverage
                    .get(&first)
                    .and_then(|v| v.first())
                    .or(replacement)
                    .cloned()
                    .unwrap_or_default();
                if let Some(last) = runs.last_mut()
                    && last.1 == face
                {
                    last.0.end = start + grapheme.len();
                } else {
                    runs.push((start..start + grapheme.len(), face, 1.));
                }
            }
        }
        for (_, face, scale) in &mut runs {
            *scale = fonts
                .definitions()
                .font_data
                .get(face)
                .map_or(1., |data| data.tweak.scale);
        }
        runs
    })
}
