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
        .unwrap_or_else(|| "evidence/standalone-icons".into());
    fs::create_dir_all(&out)?;
    let colors = [
        [220, 40, 40, 255],
        [30, 180, 70, 255],
        [40, 80, 220, 255],
        [0; 4],
    ];
    let image = image::RgbaImage::from_fn(120, 80, |x, y| {
        image::Rgba(colors[(x / 60 + y / 40 * 2) as usize])
    });
    let mut data = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image).write_to(&mut data, image::ImageFormat::Png)?;
    let bytes = data.into_inner();
    fs::write(out.join("Quadrants.png"), &bytes)?;
    let mut e = Editor::default();
    e.document.title = "Standalone icons".into();
    e.document.settings.sketch = false;
    for i in 0..16 {
        e.add(
            if i < 4 { Kind::Icon } else { Kind::IconLabel },
            50. + (i % 4) as f32 * 220.,
            50. + (i / 4) as f32 * 140.,
        );
        let id = *e.selection.iter().next().unwrap();
        e.begin("Configure icon");
        if i > 4 {
            e.set_control_property(
                id,
                "text",
                Value::Text(
                    if i == 11 {
                        ""
                    } else if i == 15 {
                        "*Bold* and _under_"
                    } else {
                        "Go"
                    }
                    .into(),
                ),
            )?;
        }
        if i != 0 && i != 4 {
            e.set_control_property(
                id,
                "icon",
                Value::Icon(IconSource::Catalog("arrow-right-solid".into())),
            )?;
            let size = match i {
                1 => "24",
                2 => "64",
                15 => "16",
                _ => "48",
            };
            e.set_control_property(id, "icon_size", Value::Choice(size.into()))?;
        }
        if [2, 3, 9, 13].contains(&i) {
            e.set_control_property(id, "border", Value::Choice("solid".into()))?;
        }
        if [6, 10, 13].contains(&i) {
            e.set_control_property(id, "label_position", Value::Choice("left".into()))?;
        }
        if [7, 8, 14].contains(&i) {
            e.set_control_property(id, "label_position", Value::Choice("right".into()))?;
        }
        if i == 8 {
            e.set_control_property(id, "state", Value::Choice("disabled".into()))?;
        }
        if [3, 10].contains(&i) {
            e.set_control_property(id, "icon_rotation", Value::Choice("90".into()))?;
        }
        if i == 15 {
            e.set_control_property(id, "font_size", Value::Number(28.))?;
        }
        e.commit();
        if [3, 10].contains(&i) {
            e.import_image_at(
                ImageTarget::Icon(id),
                Prepared::from_bytes("Quadrants.png", &bytes)?,
                ImportChoice::UseExisting,
            )?;
        }
        if i >= 12 {
            e.begin("Fixed width");
            e.selected_mut(|c| {
                c.bounds.w = match i {
                    12 => 47.,
                    13 => 48.,
                    14 => 35.,
                    _ => 160.,
                }
            });
            e.commit();
        }
    }
    fs::write(out.join("Standalone-icons.wireframe"), e.document.encode()?)?;
    let s = e.rendered_scene(e.page)?;
    fs::write(out.join("clean.png"), s.png()?)?;
    fs::write(out.join("clean.svg"), s.svg())?;
    e.document.settings.sketch = true;
    fs::write(out.join("sketch.png"), e.rendered_scene(e.page)?.png()?)?;
    println!("{}", out.display());
    Ok(())
}
