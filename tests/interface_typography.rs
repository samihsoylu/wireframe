// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
#![cfg(target_os = "macos")]
use eframe::egui::{self, Color32, FontId};
use wireframe::{
    fonts::{self, InterfaceRole},
    text_entry::layout::{Engine, Layout},
};

fn native_width(text: &str, size: f32, role: InterfaceRole) -> f32 {
    use core_foundation::{
        attributed_string::CFMutableAttributedString,
        base::{CFRange, TCFType},
        string::CFString,
    };
    use core_text::{
        font::{kCTFontSystemFontType, new_ui_font_for_language},
        line::CTLine,
        string_attributes::kCTFontAttributeName,
    };
    let font = match role {
        InterfaceRole::Regular => {
            new_ui_font_for_language(kCTFontSystemFontType, f64::from(size), None)
        }
        InterfaceRole::Medium | InterfaceRole::Bold => {
            let native = objc2_app_kit::NSFont::systemFontOfSize_weight(f64::from(size), unsafe {
                if role == InterfaceRole::Bold {
                    objc2_app_kit::NSFontWeightBold
                } else {
                    objc2_app_kit::NSFontWeightMedium
                }
            });
            unsafe {
                core_text::font::CTFont::wrap_under_get_rule(
                    objc2::rc::Retained::as_ptr(&native).cast(),
                )
            }
        }
    };
    let mut string = CFMutableAttributedString::new();
    string.replace_str(&CFString::new(text), CFRange::init(0, 0));
    // The range is in UTF-16 code units; both values remain alive during layout.
    unsafe {
        string.set_attribute(
            CFRange::init(0, text.encode_utf16().count() as isize),
            kCTFontAttributeName,
            &font,
        );
    }
    CTLine::new_with_attributed_string(string.as_concrete_TypeRef())
        .get_typographic_bounds()
        .width as f32
}

#[test]
fn interface_labels_and_fields_follow_native_sizing_across_display_scales() {
    let ctx = egui::Context::default();
    fonts::install(&ctx);
    let mut engine = Engine::default();
    let mut count = 0;
    for scale in [1., 1.25, 2., 3.] {
        let mut raw = egui::RawInput::default();
        raw.viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .native_pixels_per_point = Some(scale);
        let mut output = ctx.run_ui(raw, |ui| {
            let ctx = ui.ctx();
            for role in [
                InterfaceRole::Regular,
                InterfaceRole::Medium,
                InterfaceRole::Bold,
            ] {
                for size in [10., 11., 12., 13., 14., 18., 24., 36.] {
                    for text in [
                        "Quick Add",
                        "Components",
                        "Email address",
                        "Order total €123.45",
                        "AV To office",
                        "İstanbul café ﬁ",
                        "cafe\u{301}",
                    ] {
                        let native = native_width(text, size, role);
                        let layout =
                            Layout::with_style(&mut engine, text, size, 2000., scale, false, role)
                                .unwrap();
                        let shaped = layout.rows[0].rect.width();
                        let galley = ctx.fonts_mut(|f| {
                            f.layout_no_wrap(
                                text.into(),
                                FontId::new(size, role.family()),
                                Color32::BLACK,
                            )
                        });
                        let widget = galley.rows[0].size.x;
                        assert!(
                            (shaped - native).abs() < 0.1,
                            "input {role:?} {size} {scale} {text}: {shaped} != {native}"
                        );
                        assert!(
                            (widget - native).abs() < 0.1,
                            "widget {role:?} {size} {scale} {text}: {widget} != {native}"
                        );
                        count += 1;
                    }
                }
            }
        });
        output.textures_delta.clear();
    }
    assert_eq!(count, 672);
}

#[test]
fn installing_interface_fonts_preserves_canvas_families_and_project_bytes() {
    let project = wireframe::document::Project::default();
    let before = project.encode().unwrap();
    let mut baseline = egui::FontDefinitions::default();
    let fallback = baseline.families[&egui::FontFamily::Proportional].clone();
    for (index, bytes) in fonts::faces(project.font.as_deref())
        .into_iter()
        .enumerate()
    {
        let family = fonts::family(index % 2 == 1, index >= 2);
        let name = family.to_string();
        baseline.font_data.insert(
            name.clone(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes.to_vec())),
        );
        let mut choices = vec![name];
        choices.extend(fallback.clone());
        baseline.families.insert(family, choices);
    }
    let old = egui::Context::default();
    old.set_fonts(baseline);
    let new = egui::Context::default();
    fonts::install_project(&new, project.font.as_deref());
    let measure = |ctx: &egui::Context| {
        let mut result = Vec::new();
        let mut output = ctx.run_ui(Default::default(), |ui| {
            let ctx = ui.ctx();
            for bold in [false, true] {
                for italic in [false, true] {
                    for size in [10., 13., 17., 36.] {
                        let g = ctx.fonts_mut(|f| {
                            f.layout_no_wrap(
                                "Contact us • café ☎".into(),
                                FontId::new(size, fonts::family(bold, italic)),
                                Color32::BLACK,
                            )
                        });
                        result.push(g.size());
                    }
                }
            }
        });
        output.textures_delta.clear();
        result
    };
    assert_eq!(measure(&old), measure(&new));
    assert_eq!(project.encode().unwrap(), before);
}

#[test]
fn heading_weight_changes_ink_without_mutating_existing_regular_layouts() {
    use egui::{Pos2, Rect, Vec2};
    let mut engine = Engine::default();
    let viewport = Rect::from_min_size(Pos2::ZERO, Vec2::new(240., 30.));
    let mut regular = Layout::with_style(
        &mut engine,
        "Components Wireframes",
        13.,
        240.,
        2.,
        false,
        InterfaceRole::Regular,
    )
    .unwrap();
    let before = regular
        .raster(&mut engine, viewport, Color32::BLACK)
        .unwrap();
    let mut medium = Layout::with_style(
        &mut engine,
        "Components Wireframes",
        13.,
        240.,
        2.,
        false,
        InterfaceRole::Medium,
    )
    .unwrap();
    let heading = medium
        .raster(&mut engine, viewport, Color32::BLACK)
        .unwrap();
    assert_ne!(before.pixels, heading.pixels);
    let ink = |image: &egui::ColorImage| image.pixels.iter().map(|p| u64::from(p.a())).sum::<u64>();
    assert!(ink(&heading) > ink(&before));
    let mut bold = Layout::with_style(
        &mut engine,
        "Components Wireframes",
        13.,
        240.,
        2.,
        false,
        InterfaceRole::Bold,
    )
    .unwrap();
    let selected = bold.raster(&mut engine, viewport, Color32::BLACK).unwrap();
    assert!(ink(&selected) > ink(&heading));
    assert_eq!(
        regular
            .raster(&mut engine, viewport, Color32::BLACK)
            .unwrap()
            .pixels,
        before.pixels
    );
    let mut fresh_engine = Engine::default();
    let mut fresh = Layout::with_style(
        &mut fresh_engine,
        "Components Wireframes",
        13.,
        240.,
        2.,
        false,
        InterfaceRole::Medium,
    )
    .unwrap();
    assert_eq!(
        fresh
            .raster(&mut fresh_engine, viewport, Color32::BLACK)
            .unwrap()
            .pixels,
        heading.pixels
    );
}
