// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
#[cfg(any(target_os = "linux", test))]
mod startup_window;
use eframe::egui;
fn main() -> eframe::Result {
    let bundle = app::environment::bundle_identity();
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let request = app::environment::parse(
        &args,
        cfg!(debug_assertions),
        &bundle,
        std::env::var_os("WIREFRAME_INPUT_TRACE").is_some(),
    )
    .map_err(|e| {
        eframe::Error::AppCreation(std::io::Error::new(std::io::ErrorKind::InvalidInput, e).into())
    })?;
    let environment = match request {
        Some(r) => app::environment::open(r),
        None => Ok(app::environment::AppEnvironment::Normal),
    }
    .map_err(|e| {
        eframe::Error::AppCreation(std::io::Error::new(std::io::ErrorKind::InvalidInput, e).into())
    })?;
    environment
        .install_backend()
        .map_err(|e| eframe::Error::AppCreation(std::io::Error::other(e).into()))?;

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
    let final_trace = environment.trace().cloned();
    let result = eframe::run_native(
        "Wireframe",
        options,
        Box::new(move |cc| {
            #[cfg(target_os = "linux")]
            startup_window::configure(cc);
            if environment.diagnostic() {
                Ok(Box::new(app::WireframeApp::new_with_environment(
                    cc,
                    environment,
                )))
            } else {
                Ok(Box::new(app::WireframeApp::new(cc)))
            }
        }),
    );
    result?;
    if final_trace.is_some_and(|trace| !trace.complete()) {
        return Err(eframe::Error::AppCreation(
            std::io::Error::other("Diagnostic acknowledgements are incomplete or invalid").into(),
        ));
    }
    Ok(())
}
