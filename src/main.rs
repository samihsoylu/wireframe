// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
#[cfg(any(target_os = "linux", test))]
mod startup_window;
use eframe::egui;
fn main() -> eframe::Result {
    let icon = image::load_from_memory(include_bytes!("../assets/app-icon/wireframe-256.png"))
        .expect("bundled application icon")
        .into_rgba8();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Wireframe")
            .with_icon(egui::IconData {
                width: icon.width(),
                height: icon.height(),
                rgba: icon.into_raw(),
            })
            .with_inner_size([1360., 900.])
            .with_min_inner_size([920., 620.])
            .with_app_id("local.wireframe.desktop"),
        ..Default::default()
    };
    eframe::run_native(
        "Wireframe",
        options,
        Box::new(|cc| {
            #[cfg(target_os = "linux")]
            startup_window::configure(cc);
            Ok(Box::new(app::WireframeApp::new(cc)))
        }),
    )
}
