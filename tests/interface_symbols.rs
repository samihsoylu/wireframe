// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use eframe::egui::{self, Color32, FontId};
use wireframe::fonts::{self, InterfaceRole};

#[test]
fn navigation_symbols_have_real_glyphs_without_a_host_font() {
    let installed = egui::Context::default();
    fonts::install(&installed);
    let mut definitions = None;
    let mut output = installed.run_ui(Default::default(), |ui| {
        definitions = Some(ui.ctx().fonts(|f| f.definitions().clone()));
    });
    output.textures_delta.clear();
    let mut definitions = definitions.unwrap();
    // Exercise the actual installed fallback order even on a machine with a
    // native interface face. Linux uses these families without the host face.
    for choices in definitions.families.values_mut() {
        choices.retain(|name| {
            !matches!(
                name.as_str(),
                "interface-system" | "interface-medium" | "interface-bold"
            )
        });
    }
    let baseline = egui::Context::default();
    let portable = egui::Context::default();
    portable.set_fonts(definitions);
    let mut regular = Vec::new();
    let mut output = baseline.run_ui(Default::default(), |ui| {
        for label in ["Prev", "Next", "Quick Add", "Wireframes"] {
            regular.push(ui.fonts_mut(|f| {
                f.layout_no_wrap(label.into(), FontId::proportional(13.), Color32::BLACK)
                    .size()
            }));
        }
    });
    output.textures_delta.clear();
    let mut output = portable.run_ui(Default::default(), |ui| {
        for role in [
            InterfaceRole::Regular,
            InterfaceRole::Medium,
            InterfaceRole::Bold,
        ] {
            ui.fonts_mut(|f| {
                let mut face = f.fonts.font(&role.family());
                let coverage = face.characters();
                for ch in ['↑', '↓'] {
                    assert!(coverage.contains_key(&ch), "missing {ch} in {role:?}");
                }
            });
            for (i, label) in ["Prev", "Next", "Quick Add", "Wireframes"]
                .iter()
                .enumerate()
            {
                let size = ui.fonts_mut(|f| {
                    f.layout_no_wrap(
                        (*label).into(),
                        FontId::new(13., role.family()),
                        Color32::BLACK,
                    )
                    .size()
                });
                assert_eq!(size, regular[i], "ordinary {role:?} label changed");
            }
        }
    });
    output.textures_delta.clear();
}
