// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use std::{fs, io::Cursor, path::PathBuf};
use wireframe::{
    document::{IconSource, Value},
    images::Prepared,
    model::Kind,
    workspace::{Editor, ImageTarget, ImportChoice},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| "evidence/multiline-buttons".into());
    fs::create_dir_all(&out)?;
    if out.join("Multiline-buttons.wireframe").exists() {
        return Err("Choose a fresh directory to preserve existing fixtures".into());
    }
    let colors = [
        [220, 40, 40, 255],
        [30, 180, 70, 255],
        [40, 80, 220, 255],
        [0; 4],
    ];
    let im = image::RgbaImage::from_fn(120, 80, |x, y| {
        image::Rgba(colors[(x / 60 + y / 40 * 2) as usize])
    });
    let mut data = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(im).write_to(&mut data, image::ImageFormat::Png)?;
    let bytes = data.into_inner();
    fs::write(out.join("Quadrants.png"), &bytes)?;
    let mut e = Editor::default();
    e.document.title = "Multiline buttons".into();
    e.document.settings.sketch = false;
    for i in 0..16 {
        e.add(
            Kind::MultilineButton,
            50. + (i % 4) as f32 * 210.,
            50. + (i / 4) as f32 * 160.,
        );
        let id = *e.selection.iter().next().unwrap();
        e.begin("Configure multiline button");
        let text = match i {
            0 => "Multiline Button\nSecond line of text",
            1 => "Go\nA longer second line wraps",
            2 => "Go",
            3 => "",
            6 => "[Go](https://example.org)\n[More](https://example.com)",
            7 => "Go\nSecond\nThird",
            11 => "Long title\nA description on several lines",
            14 => "*Go*\n[Details](https://example.org/details)\nMore information",
            15 => "Go\n",
            _ => "Go\nSecond",
        };
        e.set_control_property(id, "text", Value::Text(text.into()))?;
        if [4, 8, 10, 12, 13].contains(&i) {
            e.set_control_property(
                id,
                "icon",
                Value::Icon(IconSource::Catalog("arrow-right-solid".into())),
            )?;
            e.set_control_property(
                id,
                "icon_size",
                Value::Choice(if [4, 8, 10].contains(&i) { "48" } else { "32" }.into()),
            )?;
        }
        if [5, 9, 12, 13, 14].contains(&i) {
            e.set_control_property(id, "fill", Value::Color([0, 0, 0, 255]))?;
        }
        if [5, 14].contains(&i) {
            e.set_control_property(id, "background_opacity", Value::Number(0.5))?;
        }
        if i == 9 {
            e.set_control_property(id, "background_opacity", Value::Number(0.75))?;
        }
        if i == 15 {
            e.set_control_property(id, "background_opacity", Value::Number(0.))?;
        }
        if i == 12 {
            e.set_control_property(id, "icon_rotation", Value::Choice("90".into()))?;
        }
        if i == 7 {
            e.set_control_property(id, "font_size", Value::Number(28.))?;
        }
        if i == 14 {
            e.set_control_property(id, "font_size", Value::Number(18.))?;
        }
        e.commit();
        if [6, 7, 8, 10, 11, 12, 13, 14].contains(&i) {
            e.begin("Resize multiline button");
            e.selected_mut(|c| {
                let (w, h) = match i {
                    6 => (130., 80.),
                    7 => (120., 100.),
                    8 => (100., 73.),
                    10 => (100., 48.),
                    11 => (140., 100.),
                    _ => (160., 80.),
                };
                c.bounds.w = w;
                c.bounds.h = h;
            });
            e.commit();
        }
        if i == 12 {
            e.import_image_at(
                ImageTarget::Icon(id),
                Prepared::from_bytes("Quadrants.png", &bytes)?,
                ImportChoice::KeepBoth,
            )?;
        }
    }
    fs::write(
        out.join("Multiline-buttons.wireframe"),
        e.document.encode()?,
    )?;
    let s = e.rendered_scene(e.page)?;
    fs::write(out.join("clean.png"), s.png()?)?;
    fs::write(out.join("clean.svg"), s.svg())?;
    e.document.settings.sketch = true;
    let s = e.rendered_scene(e.page)?;
    fs::write(out.join("sketch.png"), s.png()?)?;
    fs::write(out.join("sketch.svg"), s.svg())?;
    println!("{}", out.display());
    Ok(())
}
