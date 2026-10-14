// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use std::{fs, io::Cursor, path::PathBuf};
use wireframe::{
    document::Value,
    images::{self, Prepared},
    model::Kind,
    workspace::{Editor, ImageTarget, ImportChoice},
};
fn png(image: RgbaImage) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut out, ImageFormat::Png)
        .unwrap();
    out.into_inner()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| "evidence/custom-icons".into());
    fs::create_dir_all(&out)?;
    let colors = [
        [220, 40, 40, 255],
        [30, 180, 70, 255],
        [40, 80, 220, 255],
        [0; 4],
    ];
    let quadrants = png(RgbaImage::from_fn(120, 80, |x, y| {
        Rgba(colors[(x / 60 + y / 40 * 2) as usize])
    }));
    let colors = [
        [220, 40, 40, 255],
        [30, 180, 70, 255],
        [40, 80, 220, 255],
        [0, 0, 0, 255],
        [220, 40, 40, 128],
        [30, 180, 70, 64],
        [40, 80, 220, 192],
        [90, 150, 220, 0],
    ];
    let palette = png(RgbaImage::from_fn(120, 80, |x, y| {
        Rgba(colors[(x / 30 + y / 40 * 4) as usize])
    }));
    fs::write(out.join("Quadrants.png"), &quadrants)?;
    fs::write(out.join("Palette.png"), &palette)?;
    fs::write(
        out.join("Imported.png"),
        png(RgbaImage::from_fn(24, 16, |x, y| {
            if (4..20).contains(&x) && (3..13).contains(&y) {
                Rgba([45, 95, 180, 255])
            } else {
                Rgba([0; 4])
            }
        })),
    )?;
    let mut e = Editor::default();
    e.document.title = "Custom icons".into();
    e.document.settings.sketch = false;
    for i in 0..3 {
        e.add(Kind::List, 50. + i as f32 * 220., 50.);
        let id = *e.selection.iter().next().unwrap();
        e.begin("Configure example");
        for (key, value) in [
            ("text", Value::Text(" ".into())),
            ("font_size", Value::Number(120.)),
            ("row_height", Value::Number(128.)),
            ("border", Value::Choice("none".into())),
            (
                "fill",
                Value::Color(if i == 1 { [0, 0, 0, 255] } else { [255; 4] }),
            ),
        ] {
            e.set_control_property(id, key, value)?;
        }
        e.commit();
        let (name, bytes) = if i == 2 {
            ("Palette.png", &palette)
        } else {
            ("Quadrants.png", &quadrants)
        };
        e.import_image_at(
            ImageTarget::ListRow {
                control: id,
                row: 0,
            },
            Prepared::from_bytes(name, bytes)?,
            ImportChoice::UseExisting,
        )?;
    }
    fs::write(
        out.join("custom-icons.png"),
        e.rendered_scene(e.page)?.png()?,
    )?;
    fs::write(
        out.join("custom-icons.svg"),
        e.rendered_scene(e.page)?.svg(),
    )?;
    for (name, bytes, color) in [
        ("quadrants-gray", &quadrants, [51; 3]),
        ("quadrants-white", &quadrants, [255; 3]),
        ("palette-gray", &palette, [51; 3]),
    ] {
        fs::write(
            out.join(format!("{name}-raw.png")),
            images::decoded_tinted(bytes, color)?.png,
        )?;
    }
    e.add(Kind::List, 50., 250.);
    let id = *e.selection.iter().next().unwrap();
    e.begin("Picker example");
    e.set_control_property(id, "text", Value::Text("Title\nAlpha\nBravo".into()))?;
    e.commit();
    fs::write(out.join("Custom-icons.wireframe"), e.document.encode()?)?;
    Ok(())
}
