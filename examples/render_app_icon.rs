// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Render application artwork at native icon sizes using the pinned SVG engine.
use std::{fs, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("Expected output directory")?,
    );
    fs::create_dir_all(&out)?;
    let tree = resvg::usvg::Tree::from_str(
        include_str!("../assets/app-icon/wireframe.svg"),
        &resvg::usvg::Options::default(),
    )?;
    let small = resvg::usvg::Tree::from_str(
        include_str!("../assets/app-icon/wireframe-small.svg"),
        &resvg::usvg::Options::default(),
    )?;
    for size in [16, 32, 64, 128, 256, 512, 1024] {
        let tree = if size <= 32 { &small } else { &tree };
        let mut pixels =
            resvg::tiny_skia::Pixmap::new(size, size).ok_or("Icon allocation failed")?;
        let scale = size as f32 / tree.size().width();
        resvg::render(
            tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixels.as_mut(),
        );
        pixels.save_png(out.join(format!("{size}.png")))?;
    }
    Ok(())
}
