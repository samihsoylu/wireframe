// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use eframe::egui;
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
    let mut fonts = egui::FontDefinitions::default();
    let fallback = fonts.families[&egui::FontFamily::Proportional].clone();
    for (bold, italic, bytes) in [
        (false, false, REGULAR),
        (true, false, BOLD),
        (false, true, ITALIC),
        (true, true, BOLD_ITALIC),
    ] {
        let family = family(bold, italic);
        let name = family.to_string();
        fonts.font_data.insert(
            name.clone(),
            std::sync::Arc::new(egui::FontData::from_static(bytes)),
        );
        let mut choices = vec![name];
        choices.extend(fallback.clone());
        fonts.families.insert(family, choices);
    }
    context.set_fonts(fonts);
}
