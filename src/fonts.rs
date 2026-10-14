// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use eframe::egui;
mod project;
pub use project::{MAX_FACE_BYTES, ProjectFont};
pub const REGULAR: &[u8] = include_bytes!("../assets/fonts/comic-neue/ComicNeue-Regular.ttf");
pub const BOLD: &[u8] = include_bytes!("../assets/fonts/comic-neue/ComicNeue-Bold.ttf");
pub const ITALIC: &[u8] = include_bytes!("../assets/fonts/comic-neue/ComicNeue-Italic.ttf");
pub const BOLD_ITALIC: &[u8] =
    include_bytes!("../assets/fonts/comic-neue/ComicNeue-BoldItalic.ttf");
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
