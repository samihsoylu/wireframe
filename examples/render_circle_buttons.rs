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
        .unwrap_or_else(|| "evidence/circle-buttons".into());
    fs::create_dir_all(&out)?;
    if out.join("Circle-buttons.wireframe").exists() {
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
    e.document.title = "Circle buttons".into();
    e.document.settings.sketch = false;
    for i in 0..16 {
        e.add(
            Kind::CircleButton,
            50. + (i % 4) as f32 * 180.,
            50. + (i / 4) as f32 * 120.,
        );
        let id = *e.selection.iter().next().unwrap();
        e.begin("Configure circle");
        if i > 0 {
            e.set_control_property(
                id,
                "icon",
                if i == 15 {
                    Value::Null
                } else {
                    Value::Icon(IconSource::Catalog("arrow-right-solid".into()))
                },
            )?;
        }
        if ![0, 10, 11].contains(&i) {
            e.set_control_property(id, "text", Value::Text("Go".into()))?;
        }
        if [2, 3].contains(&i) {
            e.set_control_property(
                id,
                "label_position",
                Value::Choice(if i == 2 { "left" } else { "right" }.into()),
            )?;
        }
        if (4..=6).contains(&i) {
            e.set_control_property(
                id,
                "state",
                Value::Choice(["disabled", "selected", "focused"][i - 4].into()),
            )?;
        }
        if [7, 14].contains(&i) {
            e.set_control_property(id, "fill", Value::Color([0, 0, 0, 255]))?;
        }
        if i == 9 {
            e.set_control_property(id, "border", Value::Choice("none".into()))?;
        }
        if [10, 11].contains(&i) {
            e.set_control_property(id, "icon_size", Value::Choice("48".into()))?;
        }
        if i == 14 {
            e.set_control_property(id, "icon_size", Value::Choice("32".into()))?;
            e.set_control_property(id, "icon_rotation", Value::Choice("90".into()))?;
        }
        e.commit();
        if i == 14 {
            e.import_image_at(
                ImageTarget::Icon(id),
                Prepared::from_bytes("Quadrants.png", &bytes)?,
                ImportChoice::KeepBoth,
            )?;
        }
        if [8, 11, 12, 13, 14].contains(&i) {
            e.begin("Resize circle");
            e.selected_mut(|c| {
                let (w, h) = match i {
                    8 => (96., 48.),
                    11 => (49., 49.),
                    12 => (39., 39.),
                    13 => (40., 40.),
                    _ => (64., 64.),
                };
                c.bounds.w = w;
                c.bounds.h = h;
            });
            e.commit();
        }
    }
    fs::write(out.join("Circle-buttons.wireframe"), e.document.encode()?)?;
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
