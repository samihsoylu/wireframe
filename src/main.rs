// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
use eframe::egui;
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Wireframe")
            .with_inner_size([1360., 900.])
            .with_min_inner_size([940., 620.])
            .with_app_id("local.wireframe.desktop"),
        ..Default::default()
    };
    eframe::run_native(
        "Wireframe",
        options,
        Box::new(|cc| Ok(Box::new(app::WireframeApp::new(cc)))),
    )
}
