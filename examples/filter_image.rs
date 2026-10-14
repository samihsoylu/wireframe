// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Write a filtered image for renderer inspection: INPUT.png OUTPUT.png.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("Usage: filter_image INPUT OUTPUT".into());
    }
    let bytes = std::fs::read(&args[0])?;
    let image = wireframe::images::decoded_styled(&bytes, 0, true)?;
    std::fs::write(&args[1], image.png)?;
    Ok(())
}
